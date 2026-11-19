# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Status

The v1.0 web shell is **implemented and pending its first deploy** to GitHub Pages (`.github/workflows/deploy.yml`, ADR-0008). The eight Rust crates under `/crates/` and the Solid.js web shell under `/shells/web/` exist, build, and pass CI (Rust fmt/clippy/test, lavapipe golden + parity tests, wasm build, web typecheck/lint/test, Playwright on the CPU renderer). Native shells (`desktop/`, `android/`, `ios/`) do not exist yet. Documents: `README.md` (overview and quick start), `PRD.md` (product requirements; source of truth for product scope), `/docs/adr/` (ADR-0001 to ADR-0010; source of truth for technical decisions), `/docs/design/ui-spec.md` (web UI spec), `/docs/user-guide.md`, and `CHANGELOG.md`. Apricity is an AI-assisted color grading app built on a **portable Rust core** with **per-platform UI shells**, launching on the web in v1.0, Android + Windows desktop in v1.5, and iOS + macOS in v2.0.

v1.0 scope is the PRD §13 Phase 1 set plus the web-feasible v1.5 items ADR-0005 pulled forward (wheels, HSL, split tone, vignette, grain, `.cube` LUT import/export, histogram, A/B, history, broadcast-legal flag, false color/zebra). Shot Coherence, native shells, ProRAW/ProRes and lens corrections remain out of v1.0.

## Product Context (read before designing or coding)

Apricity is a professional color grading app whose core differentiator is the **Neural Grading Agent (NGA)** — an on-device model that translates natural-language prompts into explicit color science parameters, with full transparency about every adjustment. Three properties of the product shape almost every technical decision:

1. **Two-stage pipeline is non-negotiable.** Stage 1 (technical correction: exposure, white balance, log-to-display, black point) runs before Stage 2 (creative grade: lift/gamma/gain, HSL curves, split tone, grain, vignette). They are conceptually and visually distinct in the UI; do not blur them. See PRD §6.1 and §7.2.
2. **AI is a co-pilot, never an autopilot.** Every NGA adjustment must be auditable via the Explainability Panel and individually overridable in the Manual Console. The Explainability Panel is P0, not optional. See PRD G2 and §6.2.
3. **All AI inference is on-device.** No user media leaves the device. Network is only permitted for receipt validation, opt-in analytics, and model updates. See PRD G5 and §7.5.

## Target Architecture (per PRD §7)

Single Cargo workspace at the repo root. Eight Rust crates make up the **portable core**; per-platform shells live under `/shells/`.

### Rust core crates (`/crates/`)

| Crate | Layer | Purpose |
|---|---|---|
| `apricity-core` | Pure Rust | `GradeParameters` (single source of truth for NGA output ↔ Recipe ↔ Manual Console), `param_spec` (the only place parameter ranges live), `ColorSpace` and transfer functions, camera log profiles, white balance, tone-curve baking, Stage 1 auto-analysis, `validate`/`sanitize`, `serde` JSON. |
| `apricity-render` | `wgpu` 30 (feature `gpu`, default) | Stage 1 + Stage 2 pipeline in WGSL. The `cpu` module is the reference implementation and spec, and is also the web fallback renderer (ADR-0007). Also `uniforms` and `lut_bake`. |
| `apricity-scopes` | Pure Rust | Waveform, vectorscope, RGB parade, histogram, broadcast-legal check. CPU only. |
| `apricity-lut` | Pure Rust | `.cube` parser/writer, 3D LUT sampling. LUTs are sRGB-encoded Rec.709 in and out. |
| `apricity-recipes` | Pure Rust | Recipe model, migration, in-memory store, persistence trait (host implements I/O; the web shell uses IndexedDB, ADR-0010). |
| `apricity-nga` | Pure Rust (`ort` only behind the off-by-default `onnx` feature) | `GradeAgent` trait with the deterministic `LexiconAgent` (ADR-0006), prompt → Stage 2 `GradeParameters`, Explainability differ. The ONNX model of ADR-0003 plugs into the same trait later. |
| `apricity-io` | `image` (PNG, JPEG, WebP) | Image read/write with color-space tagging, for native shells and tests; the web shell decodes with the browser. Video I/O is host-supplied. |
| `apricity-ffi` | `wasm-bindgen` (feature `wasm`); `uniffi` feature is a reserved stub | The single FFI surface. `api` holds all logic as plain Rust; `wasm` adds the adapters and, on wasm32, the GPU/CPU canvas `Engine`. TypeScript types are generated from the Rust structs by `tsify-next` (never hand-written). |

### Shells (`/shells/`)

| Shell | Stack | Phase |
|---|---|---|
| `web/` | TypeScript + Solid.js + Vite + WebGPU (CPU fallback) + WASM core | **v1.0 (implemented; deploys via GitHub Pages)** |
| `desktop/` | Tauri 2 wrapping the web UI; Rust core linked directly | v1.5 (Windows; macOS later). Not started. |
| `android/` | Kotlin + Jetpack Compose + UniFFI | v1.5. Not started. |
| `ios/` | Swift + SwiftUI + UniFFI | v2.0, gated on Mac access. Not started. |

### Performance and color invariants

- **Pipeline color space:** 32-bit float scene-linear Rec.2020 internally, regardless of platform. Output transform happens last.
- **Performance floor:** < 16ms per frame (60fps preview) on every supported platform. < 8ms target on native shells with hardware acceleration.
- **NGA:** v1.0 is the on-device lexicon engine (ADR-0006), with no model download. The ONNX target for the later model stays: INT8 quantized, < 200MB active memory, < 500MB on-disk, latency < 3s p95 on web (WebGPU EP), < 2s on native shells.
- **Single source of truth:** `GradeParameters` in `apricity-core` is the schema for the NGA output, the Recipe file format, and the Manual Console controls. Never redefine it in a shell.

### Architectural seams

When designing a module, check which seam it crosses: Stage 1 (correction) → scene-linear core → Stage 2 (creative grade) → output transform → display/export, plus the orthogonal NGA, Scopes, Recipe, and LUT subsystems. The crate names map 1:1 to these seams. Do not let logic from one seam leak into another (e.g. no Stage 1 math inside `apricity-render`'s Stage 2 shader; no UI assumptions in `apricity-core`).

## Priority Discipline

The PRD assigns P0/P1/P2 to every sub-feature. v1.0 ships the P0 set listed in §13 Phase 1 plus the items ADR-0005 pulled forward (wheels, HSL curves, LUT import/export, and the others listed under Repository Status). When asked to add or change a feature, check its priority and phase before expanding scope. The Shot Coherence Engine and everything else ADR-0005 excludes remain v1.5 or later — do not pull them forward without discussion, and record any scope change in an ADR.

## Open Questions

Resolved (see the ADRs):

- **Web UI framework** — Solid.js + Vite + TypeScript (ADR-0002).
- **WebGPU fallback strategy** — the `apricity-render` CPU reference path at reduced resolution, NGA stays enabled; no WebGL2 (ADR-0007).
- **NGA v1 engine** — deterministic lexicon engine behind `GradeAgent` (ADR-0006).
- **Deploy target** — GitHub Pages (ADR-0008). **Persistence** — IndexedDB (ADR-0010).

Still open:

- **Onboarding depth** (PRD §14 Q7) — the three-step hypothesis ships behind the `onboarding` flag (`shells/web/src/state/flags.ts`); validate after first beta.
- **Web payment provider** (PRD §14 Q9) — tier gating is built behind an `EntitlementProvider` interface with a `BETA_ALL_PRO` flag and no checkout (ADR-0009). A new ADR must decide this before the flag is turned off.
- **Tauri vs egui for desktop (v1.5)** — Tauri 2 recommended; not v1.0 work.

Do not unilaterally answer any of the open ones — flag and ask.

## Code Quality Expectations (apply from the first commit)

The product's correctness claims (60fps preview, < 2s NGA latency, broadcast-legal output, on-device-only AI) are only credible if the code is structured to be measured. Bake this in from day one rather than retrofitting.

### Testability

- **Separate pure logic from device-bound code.** Stage 1 correction math, Stage 2 parameter application, NGA JSON parsing, LUT read/write, scope computations, and parameter validation live in pure-Rust crates (`apricity-core`, `apricity-lut`, `apricity-recipes`, parts of `apricity-scopes` and `apricity-render`) with **no `wgpu`, no `ort`, no shell-specific dependencies**. These crates run `cargo test` on Linux/Windows/macOS without a GPU. `wgpu` shaders, `ort` model inference, and shell UIs are thin adapters over them.
- **Golden-image tests for the rendering pipeline.** Each pipeline stage (and the full pipeline) is tested by rendering a fixed input (checked-in reference PNG or synthetic gradient) with a fixed parameter set and comparing against a checked-in reference output within a documented ΔE tolerance. Run via **lavapipe** (software Vulkan) on Linux CI runners — no real GPU needed.
- **CPU/GPU parity tests.** For every WGSL shader, a CPU reference impl exists in pure Rust. Tests assert CPU and GPU outputs match within 1 LSB at 8-bit on the test patch. Color-math regressions are otherwise invisible.
- **NGA contract tests, not model-quality tests.** Test that the NGA's output conforms to the `GradeParameters` schema, that every field round-trips through the Manual Console, and that the Explainability differ has a template for every parameter the model can emit. Do **not** write tests asserting specific model outputs for specific prompts — those belong in an offline eval harness, not the unit suite.
- **Performance budgets are tests.** The < 16ms preview and < 3s NGA targets from PRD §10.3 are tracked with `criterion` benches (Rust core), which exist today (`cargo bench -p apricity-render --bench pipeline`, `cargo bench -p apricity-scopes --bench scopes`) and are run manually. TODO: enforce budgets on a designated reference machine in CI (not in the dev's hot loop), and add `vitest` perf tests for the web shell.
- **Determinism.** Anything seeded (grain, "Surprise Me") takes an explicit seed parameter so tests are reproducible. **No `thread_rng()`, no `Instant::now()`, no `SystemTime::now()` inside pipeline code** — inject the clock and RNG. Same rule on the web side: no `Date.now()` or `Math.random()` in pipeline-adjacent code.

### Documentation

- **Every public Rust item gets a doc comment** (`///`) stating purpose, units, and color space where relevant. "Units and color space" is the part that gets skipped and causes the worst bugs in this domain — a function taking `exposure: f32` must say whether the unit is stops, linear gain, or IRE. Same rule for TypeScript: every exported function in the web shell gets a TSDoc comment with the same units/color-space discipline.
- **Crate-level README per architectural seam.** Each `apricity-*` crate gets a `README.md` (or top-level `//! ` doc-comment) explaining its inputs, outputs, color-space contract, and what it does *not* own. These are the boundaries — documenting them prevents leakage between seams.
- **Document the parameter schema once, in code.** The NGA output schema, the Recipe file format, and the Manual Console controls must be a single Rust source of truth — the `GradeParameters` struct with `serde::{Serialize, Deserialize}` in `apricity-core`. The web shell consumes it via WASM bindings; never redefine it in TypeScript. Tests assert the JSON schema is stable across the FFI boundary.
- **ADRs for non-obvious choices.** When resolving any of the open questions in `PRD.md §14`, picking a specific quantization scheme, choosing between candidate crates (`ort` vs `candle`, `solid` vs `react`), or making any decision that overrides a default — write a short ADR at `/docs/adr/NNNN-title.md`: context, decision, consequences. Three paragraphs is enough.
- **Don't comment what the code says.** Doc comments and ADRs are the right places for prose. Inline comments only for non-obvious *why* (a workaround, a numerical-stability trick, a color-space subtlety). See the project-wide guidance against narrative comments.

### Build & Test Commands

Prerequisites: Rust stable (MSRV 1.88) with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.130 (must match the `wasm-bindgen` version in `Cargo.lock`), Node 22, and pnpm 10. Install web dependencies once with `pnpm install --frozen-lockfile` at the repo root. Everything below was run successfully from the repo root unless it says otherwise.

Rust core:

- `cargo build`
- `cargo test --workspace`
- `cargo test -p apricity-core param_spec` runs the tests whose name contains `param_spec`; the same form works for any crate and test name.
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --features apricity-render/golden-images -- -D warnings`
- `cargo bench -p apricity-render --bench pipeline -- --quick` and `cargo bench -p apricity-scopes --bench scopes -- --quick` (criterion; naming the bench is required, because the lib target rejects `--quick`). Run manually; CI enforcement is a TODO (see Testability).

Golden-image and CPU/GPU parity tests (lavapipe is software Vulkan; `apt-get install mesa-vulkan-drivers vulkan-tools` on Linux):

- `WGPU_BACKEND=vulkan APRICITY_REQUIRE_GPU=1 cargo test -p apricity-render --features golden-images`. Without `APRICITY_REQUIRE_GPU=1` the parity test skips when no adapter is found.
- Regenerate the reference PNGs after a deliberate pipeline change: the same command with `APRICITY_REGEN_GOLDEN=1`, then review the image diff.
- Regenerate the schema snapshot after a deliberate `GradeParameters` change: `APRICITY_REGEN_SNAPSHOT=1 cargo test -p apricity-core --test schema_snapshot`.

WASM and web shell (`shells/web/`, run from that directory):

- `cargo clippy -p apricity-ffi --target wasm32-unknown-unknown --features wasm -- -D warnings` (repo root)
- `pnpm run build:wasm` builds `apricity-ffi` for wasm32 and runs `wasm-bindgen` into the git-ignored `src/wasm/`. The other scripts need it done at least once.
- `pnpm build` (runs `build:wasm`, then `vite build` into `dist/`)
- `pnpm test` (vitest), `pnpm lint` (eslint), `pnpm exec tsc --noEmit`
- `pnpm dev` (runs `build:wasm`, then the Vite dev server). It starts a server, so it was not run while writing this section.

Playwright end-to-end (`shells/web/`): `pnpm exec playwright test` (script `pnpm e2e`) builds the production bundle under the `/Apricity/` sub-path, serves it with `vite preview` on port 4173, and runs two projects: `webgpu` (software WebGPU through SwiftShader) and `cpu-fallback` (forces `?cpu=1`). CI runs only `--project=cpu-fallback`, because hosted runners have no GPU. First-time setup: `pnpm exec playwright install --with-deps chromium`. This was verified by the project overseer but not re-run while writing this section.
