# apricity-render

GPU compute pipeline for Apricity, written in WGSL via `wgpu`. One shader source
compiles to Metal (iOS/macOS), Vulkan (Android/Linux), DX12 (Windows), and
WebGPU (browser).

## Owns

- WGSL shaders for Stage 1 (correction) and Stage 2 (creative grade)
- wgpu device/queue plumbing and bind-group layouts
- A pure-Rust CPU reference implementation kept in lockstep with the GPU path
  for parity tests

## Does not own

- The `GradeParameters` schema (lives in `apricity-core`)
- Image decoding/encoding (`apricity-io`)
- Scopes (`apricity-scopes`)

## Color-space contract

Inputs are decoded to f32 scene-linear Rec.2020 before Stage 1; outputs are
transformed to the requested display space at the very end. The interior
representation is never sRGB-encoded.

## Test invariants

- Every WGSL shader has a CPU reference; outputs match within 1 LSB at 8-bit.
- Golden-image tests under `--features golden-images` run via lavapipe in CI.
