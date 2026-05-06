//! GPU rendering pipeline for Apricity.
//!
//! Stage 1 (technical correction) → scene-linear Rec.2020 → Stage 2
//! (creative grade) → output transform. Same WGSL source compiles to Metal,
//! Vulkan, DX12, and WebGPU via `wgpu`.
//!
//! M0 stub: no shaders yet. Real pipeline lands in M2; CPU reference impl is
//! kept in step for parity testing.

#![forbid(unsafe_code)]

#[cfg(feature = "golden-images")]
pub mod golden;

/// Placeholder for the wgpu-backed pipeline. Filled in M2.
pub struct Pipeline;
