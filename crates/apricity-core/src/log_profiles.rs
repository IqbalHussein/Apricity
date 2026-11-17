//! Camera log decoding curves and native-gamut matrices.
//!
//! Each profile decodes a **log-encoded code value in [0, 1]** to
//! **scene-linear reflectance** (0.18 = mid grey) in the camera's native
//! gamut, then converts to **scene-linear Rec.2020** via CIE XYZ (all gamuts
//! here are D65, so no chromatic adaptation is needed).
//!
//! Sources (manufacturer white papers):
//! - ARRI, "ALEXA Log C Curve — Usage in VFX" (LogC3, EI 800).
//! - Sony, "Technical Summary for S-Gamut3.Cine/S-Log3 and S-Gamut3/S-Log3".
//! - Panasonic, "V-Log/V-Gamut Reference Manual".
//! - Apple, "Apple Log Profile" white paper (2023).

use glam::{Mat3, Vec3};

use crate::color_space::ColorSpace;
use crate::grade_parameters::LogProfile;

// Published reference matrices are transcribed verbatim.
#[allow(clippy::excessive_precision)]
const AWG3_TO_XYZ: [[f32; 3]; 3] = [
    [0.638_008, 0.214_704, 0.097_744],
    [0.291_954, 0.823_841, -0.115_795],
    [0.002_798, -0.067_034, 1.153_294],
];

#[allow(clippy::excessive_precision)]
const SGAMUT3_CINE_TO_XYZ: [[f32; 3]; 3] = [
    [0.599_083_9, 0.248_925_5, 0.102_446_5],
    [0.215_075_8, 0.885_068_5, -0.100_144_3],
    [-0.032_065_8, -0.027_658_4, 1.148_782_0],
];

#[allow(clippy::excessive_precision)]
const VGAMUT_TO_XYZ: [[f32; 3]; 3] = [
    [0.679_644, 0.152_211, 0.118_600],
    [0.260_686, 0.774_894, -0.035_580],
    [-0.009_310, -0.004_612, 1.102_980],
];

fn mat_rows(rows: [[f32; 3]; 3]) -> Mat3 {
    Mat3::from_cols(
        Vec3::new(rows[0][0], rows[1][0], rows[2][0]),
        Vec3::new(rows[0][1], rows[1][1], rows[2][1]),
        Vec3::new(rows[0][2], rows[1][2], rows[2][2]),
    )
}

/// ARRI LogC3 (EI 800) code value → scene-linear.
pub fn logc3_decode(t: f32) -> f32 {
    const CUT: f32 = 0.010_591;
    const A: f32 = 5.555_556;
    const B: f32 = 0.052_272;
    const C: f32 = 0.247_190;
    const D: f32 = 0.385_537;
    const E: f32 = 5.367_655;
    const F: f32 = 0.092_809;
    if t > E * CUT + F {
        (10f32.powf((t - D) / C) - B) / A
    } else {
        (t - F) / E
    }
}

/// Sony S-Log3 code value → scene-linear.
pub fn slog3_decode(t: f32) -> f32 {
    let cv = t * 1023.0;
    if cv >= 171.210_3 {
        10f32.powf((cv - 420.0) / 261.5) * (0.18 + 0.01) - 0.01
    } else {
        (cv - 95.0) * 0.01125 / (171.210_3 - 95.0)
    }
}

/// Panasonic V-Log code value → scene-linear.
pub fn vlog_decode(t: f32) -> f32 {
    const CUT2: f32 = 0.181;
    const B: f32 = 0.008_73;
    const C: f32 = 0.241_514;
    const D: f32 = 0.598_206;
    if t < CUT2 {
        (t - 0.125) / 5.6
    } else {
        10f32.powf((t - D) / C) - B
    }
}

/// Apple Log code value → scene-linear.
pub fn apple_log_decode(p: f32) -> f32 {
    const R0: f32 = -0.056_410_88;
    const RT: f32 = 0.01;
    const C: f32 = 47.287_112;
    const BETA: f32 = 0.009_640_52;
    const GAMMA: f32 = 0.085_504_79;
    const DELTA: f32 = 0.693_369_45;
    let pt = C * (RT - R0) * (RT - R0);
    if p >= pt {
        2f32.powf((p - DELTA) / GAMMA) - BETA
    } else if p >= 0.0 {
        (p / C).sqrt() + R0
    } else {
        R0
    }
}

impl LogProfile {
    /// Decode one channel's code value to scene-linear in the native gamut.
    /// For [`LogProfile::None`] this is the sRGB EOTF.
    #[inline]
    pub fn decode_channel(self, v: f32) -> f32 {
        match self {
            LogProfile::None => crate::transfer::srgb_decode(v),
            LogProfile::ArriLogC3 => logc3_decode(v),
            LogProfile::SonySLog3 => slog3_decode(v),
            LogProfile::PanasonicVLog => vlog_decode(v),
            LogProfile::AppleLog => apple_log_decode(v),
        }
    }

    /// Matrix from the profile's native **linear** gamut to linear Rec.2020.
    pub fn to_rec2020_matrix(self) -> Mat3 {
        let xyz_to_2020 = ColorSpace::LinearRec2020.xyz_to_rgb();
        match self {
            LogProfile::None => xyz_to_2020 * ColorSpace::LinearSrgb.rgb_to_xyz(),
            LogProfile::ArriLogC3 => xyz_to_2020 * mat_rows(AWG3_TO_XYZ),
            LogProfile::SonySLog3 => xyz_to_2020 * mat_rows(SGAMUT3_CINE_TO_XYZ),
            LogProfile::PanasonicVLog => xyz_to_2020 * mat_rows(VGAMUT_TO_XYZ),
            LogProfile::AppleLog => Mat3::IDENTITY,
        }
    }

    /// Whether this is a camera log encoding (scene-referred source).
    pub fn is_log(self) -> bool {
        !matches!(self, LogProfile::None)
    }
}

/// Filmic display-rendering curve used by Stage 1's log-to-display step:
/// maps scene-linear (0.18 mid grey) to display-referred linear, with a
/// soft shoulder so highlights above diffuse white roll off instead of
/// clipping. Mid grey is preserved (0.18 → ≈0.18).
///
/// Narkowicz's fitted ACES approximation with a 0.72 input scale.
/// **Mirrored in `stage1.wgsl`.**
#[inline]
pub fn display_render(x: f32) -> f32 {
    let x = x.max(0.0) * 0.72;
    let num = x * (2.51 * x + 0.03);
    let den = x * (2.43 * x + 0.59) + 0.14;
    (num / den).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn mid_grey_anchors() {
        // Published 18% grey code values.
        assert!(approx(logc3_decode(0.391_007), 0.18, 2e-3));
        assert!(approx(slog3_decode(420.0 / 1023.0), 0.18, 1e-3));
        assert!(approx(vlog_decode(0.423_311), 0.18, 2e-3));
        assert!(approx(apple_log_decode(0.487_9), 0.18, 5e-3));
    }

    #[test]
    fn decode_is_monotone() {
        for profile in LogProfile::ALL {
            let mut prev = f32::NEG_INFINITY;
            for i in 0..=100 {
                let v = profile.decode_channel(i as f32 / 100.0);
                assert!(v >= prev - 1e-6, "{profile:?} non-monotone at {i}");
                prev = v;
            }
        }
    }

    #[test]
    fn gamut_matrices_preserve_white() {
        for profile in LogProfile::ALL {
            let w = profile.to_rec2020_matrix() * Vec3::ONE;
            assert!(
                approx(w.x, w.y, 2e-2) && approx(w.y, w.z, 2e-2),
                "{profile:?} white → {w:?}"
            );
        }
    }

    #[test]
    fn display_render_preserves_mid_grey_and_saturates() {
        assert!(approx(display_render(0.18), 0.18, 0.01));
        assert!(display_render(100.0) <= 1.0);
        assert_eq!(display_render(-1.0), 0.0);
    }
}
