//! Neural Grading Agent (NGA).
//!
//! Translates a natural-language prompt into Stage 2 [`GradeParameters`]
//! changes and explains every change. v1 ships a deterministic, on-device
//! [`LexiconAgent`] behind the [`GradeAgent`] trait (ADR-0006); the ONNX model
//! of ADR-0003 plugs into the same trait later.
//!
//! - **Inputs:** a prompt, the current [`GradeParameters`], a [`PromptMode`].
//! - **Outputs:** an [`NgaResult`] with sanitized parameters (only Stage 2 can
//!   differ from the input), matched/unmatched terms, per-term attributions,
//!   and the Explainability changes and summary.
//! - **Color space:** none of its own; every delta is in the target
//!   parameter's unit (see `apricity_core::param_spec`).
//! - **Does not own:** parameter ranges or the schema (`apricity-core`),
//!   rendering, or any I/O. No network, no clock, no ambient randomness.

#![forbid(unsafe_code)]

mod agent;
mod engine;
pub mod explain;
pub mod lexicon;
mod rng;

pub use agent::{Attribution, GradeAgent, LexiconAgent, NgaResult, PromptMode};
pub use explain::ParameterChange;
pub use lexicon::lexicon_terms;
