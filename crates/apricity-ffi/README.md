# apricity-ffi

The single FFI surface for every shell. Shells depend only on this crate.

| Module / feature | Role |
|---|---|
| `api` | All logic as plain Rust (typed in/out, no platform types). Natively unit-tested. |
| `compose`, `state` | Pure parts of the web `Engine`: downscaling, compare-side resolution, CPU split compositing, scope assembly, analysis. |
| `wasm` feature | `wasm-bindgen` adapters over `api`; on `wasm32` also the GPU/canvas `Engine`. |
| `uniffi` feature | Reserved stub for Kotlin/Swift. |

Units and color spaces are those of the underlying crates (see `apricity_core::param_spec`). Pixel buffers are **sRGB-encoded RGBA8**, row-major, top row first. Not owned here: color math, parameter schema, rendering, scopes (their own crates).

## Build (web)

```
cd shells/web && pnpm run build:wasm   # cargo build --release --features wasm + wasm-bindgen 0.2.130 -> src/wasm/
```

## TypeScript types

Types are generated from the Rust structs by `tsify-next` (optional `tsify` feature on `apricity-core`, `-recipes`, `-nga`, `-scopes`, `-render`, enabled by this crate's `wasm` feature) and land in `src/wasm/apricity_ffi.d.ts`. Functions use `unchecked_param_type` / `unchecked_return_type` to reference them. Never hand-write `GradeParameters`, `Recipe`, `NgaResult`, `SpecTable` etc. in TypeScript. `f32` values are serialized through JSON so they keep their shortest decimal form (`0.45`).

## Free functions (all throw `Error` on failure where noted)

`version()`, `paramSpecs()`, `logProfiles()`, `identityParams()`, `sanitizeParams(p)`, `validateParams(p)`, `ngaApply(prompt, current, "auto"|"fresh"|"refine")`, `ngaSurprise(current, lastPrompt|null, seed)` (seed: integer below 2^53), `explainDiff(before, after)`, `lexiconTerms()`, `recipeNew(id, name, params, prompt|null, nowMs)` (throws), `recipeParse(json)` (throws), `recipeToJson(recipe)`, `recipeApply(recipe, current)`, `recipeRename(recipe, name, nowMs)` (throws), `lutParse(text)` (throws), `lutBake(params, size, "creative"|"full", title|null)`, `analyzeStage1(rgba, w, h, stage1Params)`, `renderCpuRgba(rgba, w, h, params, lutText|null, fullWidth)`, `graticule()`.

## `Engine` (wasm32 only)

`await Engine.create(canvas, forceCpu)` tries WebGPU (non-sRGB surface format, opaque alpha) and falls back to a 2D-canvas CPU renderer; it does not throw because WebGPU is missing. Methods: `backend()`, `setSourceRgba(rgba, w, h, fullWidth, fullHeight)`, `setSourceVideo(video)` / `setSourceBitmap(bitmap)` (GPU only; CPU returns an error), `setScopeProxyRgba`, `setReferenceRgba`, `clearReference`, `setParams`, `setCompare`, `setView({zoom, pan, exact})`, `setOverlay(kind, zebraThreshold)`, `setLut(text|null)`, `setBackground(r, g, b)`, `resize(w, h)`, `render()`, `scopes(kinds)`, `analyzeStage1()`.

CPU mode renders the source downscaled to at most 640 px and places it with the same `place` function as the GPU path. For full-resolution export, upload the full image with `setSourceRgba`, `setView({exact: true})` on a canvas of the export size, `render()`, then `canvas.toBlob`.

## Tests

`cargo test -p apricity-ffi` (native). The wasm boundary is covered by `shells/web/src/core/wasm.node.test.ts`.
