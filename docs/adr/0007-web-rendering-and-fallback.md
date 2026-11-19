# ADR-0007: Web Rendering via wgpu-in-WASM; CPU Reference as the Fallback

- Status: accepted
- Date: 2026-11-17
- Amends: ADR-0004 (fallback mechanism), PRD §14 Q11

## Context

The web shell needs a GPU preview at < 16 ms per frame and a fallback for
browsers without WebGPU. ADR-0004 suggested a WebGL2 preview-only fallback.
A WebGL2 path would mean a second shader source (GLSL ES) kept in lockstep
with the WGSL, which is exactly the class of color-math drift the parity
suite exists to prevent. The workspace was pinned to wgpu 0.20, which predates
two years of WebGPU fixes.

## Decision

- The web preview runs `apricity-render`'s wgpu pipeline compiled to WASM on
  the browser's WebGPU backend. The single WGSL source is shared with every
  native backend. The workspace moves to **wgpu 30** (MSRV 1.88).
- Without WebGPU, the shell uses `apricity-render::cpu`, the same reference
  implementation the parity tests check the GPU against, rendering a
  downscaled preview into a 2D canvas. Scopes always use the CPU path on a
  proxy, because they need pre-clip float values.
- The NGA stays **enabled** in fallback mode. ADR-0004 disabled it only
  because the planned ONNX model needed the WebGPU EP, and the v1 lexicon
  engine (ADR-0006) doesn't need it.

## Consequences

- One shader source, so CPU/GPU parity stays enforceable (lavapipe in CI, ≤ 1
  LSB at 8-bit).
- Fallback preview is lower resolution and slower, but color-identical.
- Images and video frames reach the GPU via `copy_external_image_to_texture`,
  avoiding WASM-memory copies of full-resolution pixels.
