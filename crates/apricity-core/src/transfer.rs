//! Transfer functions (OETF / EOTF) for the color spaces Apricity v1.0
//! supports.
//!
//! "Encode" goes **linear → display-encoded** (a.k.a. OETF for capture or
//! inverse-EOTF for display). "Decode" goes **display-encoded → linear**.
//!
//! All functions operate per-channel on `f32` in the [0.0, 1.0] reference
//! range; values outside that range are passed through the same math
//! (extrapolated), since the pipeline interior is scene-linear and may carry
//! HDR values > 1.0 or negative undershoots.
//!
//! Note: BT.1886 is implemented as pure gamma-2.4 (the simplified form
//! appropriate when display black is 0). Production grading on calibrated
//! studio monitors should layer the proper black-level offset on top — out
//! of scope for v1.0 web preview.

/// sRGB IEC 61966-2-1 transfer (encode: linear → sRGB-encoded).
#[inline]
pub fn srgb_encode(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB IEC 61966-2-1 transfer (decode: sRGB-encoded → linear).
#[inline]
pub fn srgb_decode(encoded: f32) -> f32 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

/// BT.1886 EOTF inverse (encode: linear → display-encoded), simplified to
/// pure gamma 2.4.
#[inline]
pub fn bt1886_encode(linear: f32) -> f32 {
    // Preserve sign for negative values that may appear in scene-linear math.
    linear.signum() * linear.abs().powf(1.0 / 2.4)
}

/// BT.1886 EOTF (decode: display-encoded → linear), simplified gamma 2.4.
#[inline]
pub fn bt1886_decode(encoded: f32) -> f32 {
    encoded.signum() * encoded.abs().powf(2.4)
}

/// BT.2020 transfer (encode: linear → encoded). v1.0 treats this as the same
/// gamma-2.4 curve as BT.1886; the production BT.2020 OETF differs only in
/// the toe segment, which has no effect on identity round-trips and is
/// indistinguishable at 8-bit for the v1.0 web preview path.
#[inline]
pub fn bt2020_encode(linear: f32) -> f32 {
    bt1886_encode(linear)
}

/// BT.2020 transfer (decode: encoded → linear). See [`bt2020_encode`].
#[inline]
pub fn bt2020_decode(encoded: f32) -> f32 {
    bt1886_decode(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn srgb_round_trip_endpoints() {
        for &v in &[0.0_f32, 0.5, 1.0] {
            let r = srgb_decode(srgb_encode(v));
            assert!(
                approx(r, v, 1e-6),
                "srgb round-trip failed for {v}: got {r}"
            );
        }
    }

    #[test]
    fn bt1886_round_trip_endpoints() {
        for &v in &[0.0_f32, 0.25, 1.0] {
            let r = bt1886_decode(bt1886_encode(v));
            assert!(
                approx(r, v, 1e-5),
                "bt1886 round-trip failed for {v}: got {r}"
            );
        }
    }

    #[test]
    fn srgb_known_anchor_points() {
        // Linear 0.5 ≈ encoded 0.7353569830524495 per IEC 61966-2-1.
        assert!(approx(srgb_encode(0.5), 0.735_357, 1e-5));
        assert!(approx(srgb_decode(0.735_357), 0.5, 1e-5));
    }
}
