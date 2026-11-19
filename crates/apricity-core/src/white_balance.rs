//! White-balance model shared by Stage 1 correction and the Stage 2
//! temperature/tint sliders.
//!
//! **Temperature** is expressed in **mireds relative to D65**. For a shift
//! `m`, let `T = 1e6 / (M_D65 + |m|)` kelvin (a Planckian illuminant warmer
//! than D65). Positive `m` multiplies the image by `white(T) / white(D65)`
//! in **scene-linear Rec.2020** (warmer); negative `m` multiplies by the
//! exact reciprocal (cooler). So `gains(-m)` undoes `gains(m)`, and
//! correcting a scene lit at CCT `K` takes `m = M_D65 − 1e6/K`.
//!
//! **Tint** scales the green channel by `2^(−tint / 400)` (positive =
//! magenta).
//!
//! Both are then renormalized so the gains have unit Rec.2020 luminance —
//! white balance never changes exposure.
//!
//! The Planckian locus uses the Kim et al. (2002) cubic-spline
//! approximation, valid 1667 K – 25000 K.

use glam::Vec3;

use crate::color_space::ColorSpace;

/// Mired value of the D65 reference (6504 K).
pub const D65_MIRED: f32 = 1.0e6 / 6504.0;

/// Rec.2020 luminance coefficients (Y row of the RGB→XYZ matrix).
pub const REC2020_LUMA: Vec3 = Vec3::new(0.262_700_2, 0.677_998_1, 0.059_301_7);

/// CIE 1931 xy chromaticity of the Planckian radiator at `kelvin`
/// (clamped to 1667–25000 K).
pub fn planckian_xy(kelvin: f32) -> (f32, f32) {
    let t = kelvin.clamp(1667.0, 25_000.0) as f64;
    let (t2, t3) = (t * t, t * t * t);
    let x = if t <= 4000.0 {
        -0.266_123_9e9 / t3 - 0.234_358_9e6 / t2 + 0.877_695_6e3 / t + 0.179_910
    } else {
        -3.025_846_9e9 / t3 + 2.107_037_9e6 / t2 + 0.222_634_7e3 / t + 0.240_390
    };
    let (x2, x3) = (x * x, x * x * x);
    let y = if t <= 2222.0 {
        -1.106_381_4 * x3 - 1.348_110_20 * x2 + 2.185_558_32 * x - 0.202_196_83
    } else if t <= 4000.0 {
        -0.954_947_6 * x3 - 1.374_185_93 * x2 + 2.091_370_15 * x - 0.167_488_67
    } else {
        3.081_758_0 * x3 - 5.873_386_70 * x2 + 3.751_129_97 * x - 0.370_014_83
    };
    (x as f32, y as f32)
}

/// Linear Rec.2020 RGB of a white at chromaticity (x, y) with Y = 1.
fn white_rgb(x: f32, y: f32) -> Vec3 {
    let xyz = Vec3::new(x / y, 1.0, (1.0 - x - y) / y);
    ColorSpace::LinearRec2020.xyz_to_rgb() * xyz
}

/// Convert a mired shift (relative to D65) to kelvin.
pub fn mired_shift_to_kelvin(mired_shift: f32) -> f32 {
    1.0e6 / (D65_MIRED + mired_shift).max(40.0)
}

/// Per-channel gains in scene-linear Rec.2020 for a temperature shift in
/// mireds and a tint in tint units. Unit-luminance; `(0, 0)` is exactly
/// `(1, 1, 1)`.
pub fn gains(temperature_mireds: f32, tint: f32) -> Vec3 {
    if temperature_mireds == 0.0 && tint == 0.0 {
        return Vec3::ONE;
    }
    let reference = {
        let (x, y) = planckian_xy(6504.0);
        white_rgb(x, y)
    };
    let target = {
        let (x, y) = planckian_xy(mired_shift_to_kelvin(temperature_mireds.abs()));
        white_rgb(x, y)
    };
    let mut g = if temperature_mireds >= 0.0 {
        target / reference
    } else {
        reference / target
    };
    g.y *= 2f32.powf(-tint / 400.0);
    g / g.dot(REC2020_LUMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_identity() {
        assert_eq!(gains(0.0, 0.0), Vec3::ONE);
    }

    #[test]
    fn positive_temperature_is_warmer() {
        let g = gains(50.0, 0.0);
        assert!(g.x > 1.0 && g.z < 1.0, "{g:?}");
        let g = gains(-50.0, 0.0);
        assert!(g.x < 1.0 && g.z > 1.0, "{g:?}");
    }

    #[test]
    fn negative_temperature_inverts_positive() {
        let p = gains(120.0, 0.0);
        let n = gains(-120.0, 0.0);
        let prod = p * n;
        assert!((prod.x / prod.y - 1.0).abs() < 1e-4 && (prod.z / prod.y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn positive_tint_is_magenta() {
        let g = gains(0.0, 50.0);
        assert!(g.y < g.x && g.y < g.z, "{g:?}");
    }

    #[test]
    fn gains_preserve_luminance() {
        for (m, t) in [(80.0, 0.0), (-120.0, 30.0), (10.0, -90.0)] {
            assert!((gains(m, t).dot(REC2020_LUMA) - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn planckian_near_known_values() {
        // 6504 K Planckian ≈ (0.3135, 0.3236); D65 itself is slightly off-locus.
        let (x, y) = planckian_xy(6504.0);
        assert!(
            (x - 0.3135).abs() < 2e-3 && (y - 0.3236).abs() < 2e-3,
            "{x},{y}"
        );
        let (x, _) = planckian_xy(2856.0); // Illuminant A ≈ 0.4476
        assert!((x - 0.4476).abs() < 2e-3);
    }
}
