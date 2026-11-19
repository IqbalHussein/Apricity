# apricity-render

The Stage 1 + Stage 2 rendering pipeline. One WGSL source runs through `wgpu`
on WebGPU (browser), Vulkan, Metal, and DX12, and a pure-Rust CPU
implementation of the same math is the specification the GPU is tested
against.

## Modules

| Module | Role |
|---|---|
| `uniforms` | `prepare(&GradeParameters, &RenderContext) -> GradeUniforms`: the derived, render-ready parameters (WB gains, baked curves, matrices, flags), computed once per parameter change. `repr(C)` `vec4` layout shared byte-for-byte with WGSL. Also `Overlay` (none, false color, zebra) and `RenderContext` (image size, source scale, output color space, overlay, zebra threshold, LUT size; the grain seed is a `GradeParameters` field). |
| `cpu` | **Reference implementation and spec** (always built, pure Rust, no `wgpu`/clock/RNG). `stage1`, `stage2`, `output`, `overlay`, `shade`, `render_rgba8`, `render_float`. Also the production path when WebGPU is missing (ADR-0007), for scopes (they need pre-clip floats), and for recipe thumbnails. |
| `gpu` | Feature `gpu` (default). `GpuRenderer` over **wgpu 30**: draws the source as a quad into any target (canvas surface for preview, offscreen texture for export and tests), two scissored draws for wipe comparison, external-image upload via `copy_external_image_to_texture`, `place`/`ViewParams` for zoom and pan. WGSL lives in `shaders/` (`common`, `stage1`, `stage2`, `output`, `main`). |
| `lut_bake` | `bake_lut(params, size, BakeScope::{Creative, Full}, title)`: bakes a grade into a `Lut3d` on the sRGB-encoded Rec.709 lattice (`apricity-lut` contract). Vignette and grain are disabled because they depend on position. |
| `golden` | Feature `golden-images`. Fixture parameter sets per stage, the 256x256 reference patch, and image-diff helpers. |

## Color-space contract

Sources are decoded to f32 scene-linear Rec.2020 before Stage 1 (sRGB EOTF or
a camera log profile). Stage 2 runs in that space, with tone and wheel math in
the working encoding (`x^(1/2.4)`). The output transform to the display space
comes last. The interior representation is never sRGB-encoded.

## Does not own

The `GradeParameters` schema and color-math primitives (`apricity-core`),
scopes (`apricity-scopes`), `.cube` file I/O (`apricity-lut`), image
decode/encode (`apricity-io` and host shells).

## Tests

- Unit tests: `cargo test -p apricity-render`.
- **Golden images** (`tests/golden_identity.rs`, `tests/golden_stages.rs`):
  each pipeline stage and a full grade are rendered by the CPU reference over a
  fixed patch and compared with the PNGs in `tests/golden/`.
- **CPU/GPU parity** (`tests/gpu_parity.rs`): every fixture is rendered through
  wgpu and must match the CPU output within 1 LSB at 8-bit.

Both run with the `golden-images` feature. CI uses lavapipe (software Vulkan):

```
WGPU_BACKEND=vulkan APRICITY_REQUIRE_GPU=1 cargo test -p apricity-render --features golden-images
```

Without `APRICITY_REQUIRE_GPU=1` the parity test skips when no adapter is
found. After a deliberate pipeline change, rewrite the reference PNGs with
`APRICITY_REGEN_GOLDEN=1` on the same command and review the image diff.

## Benchmarks

`cargo bench -p apricity-render --bench pipeline` (criterion) times a 640x360
CPU render with a full grade, a 33-point `Full` LUT bake, and `prepare`.
