# apricity-nga

Neural Grading Agent. Wraps the ONNX model behind a typed Rust API:
`prompt → GradeParameters`. Also owns the Explainability differ.

## Backends

ONNX Runtime via `ort`, with per-platform Execution Providers selected by the
host shell:

| Shell | EP |
|---|---|
| Web | WebGPU EP (CPU fallback if absent) |
| Android | NNAPI EP |
| Windows | DirectML EP |
| iOS / macOS | Core ML EP |

## Privacy invariant

All inference is on-device. This crate must not perform any network I/O.

## Test invariants

- Every model output validates against the `GradeParameters` schema.
- Every parameter the model can emit has an Explainability template.
- Tokenizer round-trips ASCII.
- **Model-quality tests live in an offline eval harness, not the unit suite.**
