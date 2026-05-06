//! Pure-Rust color science core for Apricity.
//!
//! Owns the single source of truth for [`GradeParameters`], the [`ColorSpace`]
//! enum, and scene-linear math. No GPU, no ML, no I/O — those live in sibling
//! crates that depend on this one.
//!
//! Internal pipeline color space is **32-bit float scene-linear Rec.2020**.
//! Every numeric parameter in this crate documents its unit and color space.

#![forbid(unsafe_code)]

pub mod color_space;
pub mod grade_parameters;
pub mod transfer;

pub use color_space::{convert, ColorSpace};
pub use grade_parameters::GradeParameters;
