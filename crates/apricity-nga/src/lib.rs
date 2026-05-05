//! Neural Grading Agent.
//!
//! Tokenizer + ONNX model wrapper + prompt → `GradeParameters` decoder, plus a
//! pure-Rust Explainability differ that converts `(before, after)` parameter
//! pairs into ordered, human-readable changes with rationale templates.
//!
//! M0 stub. Real impl lands in M5 (the highest-risk milestone).

#![forbid(unsafe_code)]
