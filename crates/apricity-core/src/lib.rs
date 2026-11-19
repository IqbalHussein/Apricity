//! Pure-Rust color science core for Apricity.
//!
//! Owns the single source of truth for [`GradeParameters`] and its
//! [`param_spec`] table, the [`ColorSpace`] enum, camera log decoding,
//! the white-balance model, tone-curve baking, Stage 1 auto-analysis, and
//! parameter validation. No GPU, no ML, no I/O — those live in sibling
//! crates that depend on this one.
//!
//! Internal pipeline color space is **32-bit float scene-linear Rec.2020**.
//! Every numeric parameter in this crate documents its unit and color space.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod color_space;
pub mod curve;
pub mod grade_parameters;
pub mod log_profiles;
pub mod param_spec;
pub mod transfer;
pub mod validate;
pub mod white_balance;

pub use color_space::{convert, ColorSpace};
pub use grade_parameters::{
    ColorWheel, CorrectionParameters, CreativeParameters, CurvePoint, GradeParameters, Grain,
    HslBand, HslBands, LogProfile, Rgb, SplitTone, ToneCurve, Vignette, SCHEMA_VERSION,
};

/// Exponent of the **working encoding**: `w = sign(x)·|x|^(1/2.4)`.
pub const WORKING_GAMMA: f32 = 2.4;

/// Scene-linear → working encoding (sign-preserving `x^(1/2.4)`). Tone and
/// wheel operations in Stage 2 run in this perceptually spaced domain.
/// Mid grey 0.18 maps to ≈0.489.
#[inline]
pub fn working_encode(x: f32) -> f32 {
    x.signum() * x.abs().powf(1.0 / WORKING_GAMMA)
}

/// Working encoding → scene-linear. Inverse of [`working_encode`].
#[inline]
pub fn working_decode(w: f32) -> f32 {
    w.signum() * w.abs().powf(WORKING_GAMMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn working_encoding_round_trips() {
        for x in [-0.5f32, 0.0, 0.18, 1.0, 4.0] {
            assert!((working_decode(working_encode(x)) - x).abs() < 1e-5);
        }
        assert!((working_encode(0.18) - 0.489).abs() < 1e-3);
    }
}
