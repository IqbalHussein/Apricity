# Changelog

All notable changes to Apricity are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased] — v1.0.0

The v1.0 web release. It is implemented and waits on the first deploy to GitHub Pages.

### Added

**Rust core (`crates/`)**

- `apricity-core`: the `GradeParameters` schema (Stage 1 correction and Stage 2 grade) with a versioned JSON snapshot test, the `param_spec` table of every parameter's unit, range and default, `validate`/`sanitize`, color-space conversion (sRGB, Display P3, Rec.709, Rec.2020) and transfer functions, camera log decoding (ARRI LogC3, Sony S-Log3, Panasonic V-Log, Apple Log), a white-balance model in mireds, monotone tone curves, and Stage 1 auto-analysis with illuminant detection.
- `apricity-render`: a CPU reference pipeline in scene-linear Rec.2020, a wgpu 30 / WGSL pipeline on WebGPU, Vulkan, Metal and DX12, derived uniforms, wipe comparison, false color and zebra overlays, and `.cube` baking in creative or full-pipeline scope. Golden-image fixtures per stage and CPU/GPU parity tests (at most 1 LSB at 8-bit) run on lavapipe in CI. Criterion benchmarks cover the CPU proxy render, LUT bake and uniform preparation.
- `apricity-scopes`: luma waveform, RGB parade, vectorscope with graticule, histogram, and a broadcast-legal check, with color-blind-safe scope colors.
- `apricity-lut`: `.cube` parser and writer, trilinear and tetrahedral sampling, and the sRGB-encoded Rec.709 convention.
- `apricity-recipes`: the Recipe model with schema versioning and migration, an in-memory store, the `RecipeStore` persistence trait, and a free-tier cap helper.
- `apricity-nga`: the `GradeAgent` trait and the deterministic `LexiconAgent` (ADR-0006), with moods, palettes, technical terms, film stocks, genres and scenes, strength modifiers, negation, comparatives, Auto/Fresh/Refine modes, and seeded Surprise me. Includes the Explainability differ, with a rationale template for every emittable parameter, and contract tests. The `onnx` feature is reserved for the ADR-0003 model.
- `apricity-io`: PNG, JPEG and WebP decode and encode with EXIF orientation and ICC-based Display P3 tagging, plus an alpha-weighted downscaler.
- `apricity-ffi`: the single FFI surface. Plain-Rust `api`, `wasm-bindgen` adapters, a WebGPU and CPU canvas `Engine`, and TypeScript types generated from the Rust structs with `tsify-next`.

**Web shell (`shells/web/`)**

- Solid.js and Vite application with Quick, Grade and Review modes, a desktop layout and a mobile bottom-sheet layout, light and dark themes, and a neutral-surround design (`docs/design/ui-spec.md`).
- Photo import by picker, drag and drop, or a built-in sample image (JPEG, PNG, WebP, HEIC where the browser decodes it).
- The Correction panel with Auto toggles, detected-light readout and input profiles, and the Manual Console with Light, Color, wheels, 8-band HSL, tone curves, split tone, effects and LUT sections, generated from the core's parameter table.
- The prompt bar with Apply, Refine, Surprise me, a 20-entry prompt history, and "Didn't recognize" feedback. The Explainability Panel with before and after values, rationale and "Show in console".
- Scopes panel with a broadcast-legal badge. Review tools: wipe against Original, Stage 1 only, Previous grade or an imported reference; false color and zebra overlays; zoom and pan.
- A/B grade slots, and undo and redo with up to 200 coalesced steps.
- Recipes: save, rename, delete, import and export as JSON, six starter looks, and a Quick-mode recipe strip.
- `.cube` LUT import with a strength slider, and `.cube` export at 17, 33 or 65 points.
- Image export to JPEG, PNG and WebP at full resolution with a Free-tier cap and mark.
- Video: MP4 and MOV import, a transport bar, and MP4 export through WebCodecs with progress and cancel.
- Free and Pro tier gating with local monthly counters and a `BETA_ALL_PRO` flag that unlocks everything during the beta (ADR-0009). No checkout.
- IndexedDB persistence for recipes, LUTs, settings and counters (ADR-0010).
- Offline support through a service worker and an installable PWA manifest.
- A three-step first-run walkthrough behind the `onboarding` flag.
- A CPU fallback renderer for browsers without WebGPU, with an explanatory banner (ADR-0007).
- Vitest unit tests and Playwright end-to-end tests with `webgpu` and `cpu-fallback` projects.

**CI, deploy and documentation**

- GitHub Actions CI: rustfmt, clippy with warnings denied, workspace tests, lavapipe golden and parity tests, a wasm32 clippy and web build, web typecheck, lint and tests, and Playwright on the CPU renderer.
- A GitHub Pages deploy workflow that builds the WASM core and Vite bundle under `/Apricity/` and publishes on pushes to `main` (ADR-0008).
- Architecture decision records ADR-0001 to ADR-0010, the web UI design spec, a user guide, a repository README, and README files for each crate.

### Changed

- The workspace moved from wgpu 0.20 to wgpu 30 (MSRV 1.88), so the web preview uses the browser's WebGPU backend through the same WGSL as native (ADR-0007).
- v1.0 scope now includes the web-feasible v1.5 items from ADR-0005: lift/gamma/gain wheels, HSL curves, split tone, vignette, grain, `.cube` LUT import and export, histogram, A/B, history, the broadcast-legal flag, and false color and zebra overlays. `PRD.md` marks the web `.cube` row as v1.0.
- The NGA ships as a deterministic lexicon engine instead of an ONNX model (ADR-0006). `ort` is optional and off by default, and the wasm32 build does not link it.
- The no-WebGPU fallback is the CPU reference renderer, not WebGL2, and the NGA stays enabled there (ADR-0007, amending ADR-0004).
- Web UI framework resolved as Solid.js (ADR-0002). The payment provider is still open (ADR-0009).
- `CLAUDE.md` and `PRD.md` updated to describe the implemented state and the resolved open questions.
