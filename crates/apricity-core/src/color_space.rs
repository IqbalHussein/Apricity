//! Color space identifiers, primaries, RGB↔XYZ matrices, and the high-level
//! [`convert`] entry point.
//!
//! The pipeline interior is always [`ColorSpace::LinearRec2020`]. Input and
//! output transforms convert at the seams. All v1.0 spaces share the **D65
//! whitepoint**, so RGB↔XYZ→RGB chains require no chromatic adaptation; the
//! inter-primary transform is pure matrix math on linear-light values.
//!
//! Matrices are the canonical Bruce Lindbloom / ITU values for the named
//! primaries with D65 (CIE 1931 2°). They are reproducible from chromaticity
//! coordinates and a 3×3 inverse, but the rounding error of an `f32`
//! recomputation would offset us from the published reference, so the
//! published values are used directly.

use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

use crate::transfer;

/// A named color space. Variants identify both the primaries and the transfer
/// function; "Linear" prefix means a linear transfer function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorSpace {
    /// sRGB primaries, sRGB (IEC 61966-2-1) transfer.
    Srgb,
    /// Rec.709 primaries (= sRGB primaries), BT.1886 transfer (gamma 2.4).
    Rec709,
    /// Rec.2020 primaries, BT.2020 transfer (gamma 2.4 in v1.0).
    Rec2020,
    /// Rec.2020 primaries, linear transfer. The pipeline interior space.
    LinearRec2020,
    /// sRGB primaries, linear transfer.
    LinearSrgb,
}

// The matrix literals below intentionally reproduce the published f64
// reference values from Bruce Lindbloom / ITU-R verbatim. f32 will round
// some entries; the round is the same on every host, which is what we care
// about. `clippy::excessive_precision` flags this faithful transcription;
// silence it on the constants only.

/// Bruce Lindbloom: sRGB / Rec.709 D65 → CIE XYZ.
/// <http://brucelindbloom.com/Eqn_RGB_XYZ_Matrix.html>
#[allow(clippy::excessive_precision)]
const SRGB_TO_XYZ: [[f32; 3]; 3] = [
    [0.412_456_4, 0.357_576_1, 0.180_437_5],
    [0.212_672_9, 0.715_152_2, 0.072_175_0],
    [0.019_333_9, 0.119_192_0, 0.950_304_1],
];

/// ITU-R BT.2020 D65 RGB → CIE XYZ.
#[allow(clippy::excessive_precision)]
const REC2020_TO_XYZ: [[f32; 3]; 3] = [
    [0.636_958_0, 0.144_616_9, 0.168_881_0],
    [0.262_700_2, 0.677_998_1, 0.059_301_7],
    [0.000_000_0, 0.028_072_7, 1.060_985_1],
];

impl ColorSpace {
    /// Matrix taking **linear-light** RGB in this space's primaries to CIE
    /// XYZ. Independent of the transfer function — callers must
    /// [`Self::decode`] encoded inputs first.
    pub fn rgb_to_xyz(self) -> Mat3 {
        match self {
            ColorSpace::Srgb | ColorSpace::Rec709 | ColorSpace::LinearSrgb => mat_from(SRGB_TO_XYZ),
            ColorSpace::Rec2020 | ColorSpace::LinearRec2020 => mat_from(REC2020_TO_XYZ),
        }
    }

    /// Inverse of [`Self::rgb_to_xyz`].
    pub fn xyz_to_rgb(self) -> Mat3 {
        self.rgb_to_xyz().inverse()
    }

    /// Decode one channel from this space's encoded representation to
    /// linear-light. For "Linear" variants this is the identity.
    #[inline]
    pub fn decode(self, channel: f32) -> f32 {
        match self {
            ColorSpace::Srgb => transfer::srgb_decode(channel),
            ColorSpace::Rec709 => transfer::bt1886_decode(channel),
            ColorSpace::Rec2020 => transfer::bt2020_decode(channel),
            ColorSpace::LinearSrgb | ColorSpace::LinearRec2020 => channel,
        }
    }

    /// Encode one channel from linear-light to this space's encoded
    /// representation. For "Linear" variants this is the identity.
    #[inline]
    pub fn encode(self, channel: f32) -> f32 {
        match self {
            ColorSpace::Srgb => transfer::srgb_encode(channel),
            ColorSpace::Rec709 => transfer::bt1886_encode(channel),
            ColorSpace::Rec2020 => transfer::bt2020_encode(channel),
            ColorSpace::LinearSrgb | ColorSpace::LinearRec2020 => channel,
        }
    }

    fn decode_rgb(self, rgb: Vec3) -> Vec3 {
        Vec3::new(self.decode(rgb.x), self.decode(rgb.y), self.decode(rgb.z))
    }

    fn encode_rgb(self, rgb: Vec3) -> Vec3 {
        Vec3::new(self.encode(rgb.x), self.encode(rgb.y), self.encode(rgb.z))
    }
}

#[inline]
fn mat_from(rows: [[f32; 3]; 3]) -> Mat3 {
    // Literals are written row-major (matches the published reference);
    // glam's `Mat3::from_cols` takes column vectors, so transpose.
    Mat3::from_cols(
        Vec3::new(rows[0][0], rows[1][0], rows[2][0]),
        Vec3::new(rows[0][1], rows[1][1], rows[2][1]),
        Vec3::new(rows[0][2], rows[1][2], rows[2][2]),
    )
}

/// Convert an RGB triplet from one color space to another.
///
/// The conversion path:
/// 1. Decode `from`'s transfer function → linear-light RGB in `from`'s primaries
/// 2. Linear matrix `from.rgb_to_xyz()` → CIE XYZ
/// 3. Linear matrix `to.xyz_to_rgb()` → linear-light RGB in `to`'s primaries
/// 4. Encode with `to`'s transfer function
///
/// Same-space conversions short-circuit to the identity. Same-primaries
/// conversions skip the XYZ matrices.
///
/// **Units:** input and output are in the encoded representation of their
/// respective spaces; for "Linear" variants this is linear-light, for
/// non-linear spaces this is the display-referred encoded value.
pub fn convert(rgb: Vec3, from: ColorSpace, to: ColorSpace) -> Vec3 {
    if from == to {
        return rgb;
    }
    let linear_from = from.decode_rgb(rgb);
    let linear_to = if same_primaries(from, to) {
        linear_from
    } else {
        let xyz = from.rgb_to_xyz() * linear_from;
        to.xyz_to_rgb() * xyz
    };
    to.encode_rgb(linear_to)
}

fn same_primaries(a: ColorSpace, b: ColorSpace) -> bool {
    use ColorSpace::*;
    matches!(
        (a, b),
        (Srgb | Rec709 | LinearSrgb, Srgb | Rec709 | LinearSrgb)
            | (Rec2020 | LinearRec2020, Rec2020 | LinearRec2020)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn approx_eq_vec3(a: Vec3, b: Vec3, eps: f32) -> bool {
        (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps && (a.z - b.z).abs() < eps
    }

    #[test]
    fn identity_same_space_is_passthrough() {
        let v = Vec3::new(0.3, 0.6, 0.1);
        for cs in [
            ColorSpace::Srgb,
            ColorSpace::Rec709,
            ColorSpace::Rec2020,
            ColorSpace::LinearRec2020,
            ColorSpace::LinearSrgb,
        ] {
            assert_eq!(convert(v, cs, cs), v);
        }
    }

    #[test]
    fn srgb_to_linear_srgb_known_anchor() {
        // Encoded 0.5 in sRGB ≈ linear 0.21404114.
        let v = convert(Vec3::splat(0.5), ColorSpace::Srgb, ColorSpace::LinearSrgb);
        assert!(approx_eq_vec3(v, Vec3::splat(0.214_041), 1e-4));
    }

    #[test]
    fn rgb_xyz_matrix_inverse_is_identity() {
        for cs in [
            ColorSpace::Srgb,
            ColorSpace::Rec2020,
            ColorSpace::LinearSrgb,
            ColorSpace::LinearRec2020,
        ] {
            let prod = cs.rgb_to_xyz() * cs.xyz_to_rgb();
            assert!(
                approx_eq_vec3(prod.x_axis, Vec3::X, 1e-5)
                    && approx_eq_vec3(prod.y_axis, Vec3::Y, 1e-5)
                    && approx_eq_vec3(prod.z_axis, Vec3::Z, 1e-5),
                "{cs:?} matrix*inverse not identity: {prod:?}"
            );
        }
    }

    fn any_color_space() -> impl Strategy<Value = ColorSpace> {
        prop_oneof![
            Just(ColorSpace::Srgb),
            Just(ColorSpace::Rec709),
            Just(ColorSpace::Rec2020),
            Just(ColorSpace::LinearRec2020),
            Just(ColorSpace::LinearSrgb),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        // Encoded → linear → encoded round-trip through every (from, to) pair
        // that shares primaries (transfer-function round-trip only).
        #[test]
        fn round_trip_same_primaries(
            r in 0.0_f32..1.0,
            g in 0.0_f32..1.0,
            b in 0.0_f32..1.0,
            from in any_color_space(),
            to in any_color_space(),
        ) {
            prop_assume!(same_primaries(from, to));
            let v = Vec3::new(r, g, b);
            let there = convert(v, from, to);
            let back = convert(there, to, from);
            prop_assert!(approx_eq_vec3(v, back, 1e-4),
                "{from:?}→{to:?}→{from:?} drift: {v:?} vs {back:?}");
        }

        // Cross-primaries round-trip. f32 matrix×inverse compounds error;
        // larger eps. Constrained to a region well inside the simultaneous
        // gamut of both Rec.709 and Rec.2020 to avoid the failure modes of
        // out-of-gamut linear values pushed back through a non-linear OETF.
        #[test]
        fn round_trip_cross_primaries(
            r in 0.2_f32..0.8,
            g in 0.2_f32..0.8,
            b in 0.2_f32..0.8,
            from in any_color_space(),
            to in any_color_space(),
        ) {
            prop_assume!(!same_primaries(from, to));
            let v = Vec3::new(r, g, b);
            let there = convert(v, from, to);
            let back = convert(there, to, from);
            prop_assert!(approx_eq_vec3(v, back, 1e-3),
                "{from:?}→{to:?}→{from:?} drift: {v:?} vs {back:?}");
        }
    }
}
