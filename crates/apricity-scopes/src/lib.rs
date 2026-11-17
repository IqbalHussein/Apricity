//! Broadcast-style scopes: waveform, RGB parade, vectorscope, histogram,
//! and broadcast-legal checks.
//!
//! **Input contract:** every function takes `&[[f32; 3]]` of **output-encoded
//! RGB** (display-referred, e.g. sRGB-encoded, unit-interval nominal range,
//! possibly unclamped: values below 0 or above 1 are legal input and are
//! reported, not rejected). Luma is Rec.709 `Y' = 0.2126R' + 0.7152G' +
//! 0.0722B'` computed on the *encoded* values.
//!
//! **Output contract:** raster scopes return a [`ScopeImage`] (RGBA8,
//! row-major, top row first, alpha 0 where empty) for the host to composite
//! over its own background.
//!
//! The functions are deterministic and allocate only a fixed number of
//! buffers per call, independent of pixel count.

#![forbid(unsafe_code)]

mod graticule;
mod histogram;
mod legal;
mod raster;
mod vectorscope;
mod waveform;

pub use graticule::{vectorscope_graticule, Graticule, VectorTarget};
pub use histogram::{histogram, Histogram};
pub use legal::{broadcast_legal, LegalReport};
pub use raster::ScopeImage;
pub use vectorscope::vectorscope;
pub use waveform::{rgb_parade, waveform_luma};

/// Rec.709 luma weights applied to output-encoded RGB.
pub(crate) const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// Rec.709 luma `Y'` of an output-encoded RGB triple, unit interval nominal.
#[inline]
pub(crate) fn luma(p: &[f32; 3]) -> f32 {
    LUMA[0] * p[0] + LUMA[1] * p[1] + LUMA[2] * p[2]
}
