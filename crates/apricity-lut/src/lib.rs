//! `.cube` 3D LUT parsing, writing, and trilinear sampling.
//!
//! Pure Rust, no GPU. The GPU sampling kernel for production rendering lives in
//! `apricity-render` and consumes the 3D table this crate produces.

#![forbid(unsafe_code)]
