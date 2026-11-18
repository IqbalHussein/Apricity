//! Rendering pipeline for Apricity.
//!
//! Stage 1 (technical correction) → scene-linear Rec.2020 → Stage 2
//! (creative grade) → output transform.
//!
//! - [`uniforms`] derives the render-ready [`uniforms::GradeUniforms`] from
//!   `GradeParameters` once per parameter change.
//! - [`cpu`] is the reference implementation and the specification of the
//!   pipeline (pure Rust, always built).
//! - [`gpu`] (feature `gpu`, default) runs the same math in WGSL via `wgpu`
//!   on WebGPU, Vulkan, Metal and DX12. Parity tests hold the two within
//!   1 LSB at 8-bit.
//!
//! Does **not** own: parameter schema or color math primitives
//! (`apricity-core`), scopes (`apricity-scopes`), LUT file I/O
//! (`apricity-lut`), image decode/encode (`apricity-io`, host shells).

#![forbid(unsafe_code)]

pub mod cpu;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod lut_bake;
pub mod uniforms;

#[cfg(feature = "golden-images")]
pub mod golden;

pub use lut_bake::{bake_lut, BakeScope};
pub use uniforms::{prepare, GradeUniforms, Overlay, RenderContext};
