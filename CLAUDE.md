# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Status

This repository is **pre-implementation**. The committed files are `PRD.md` (the product requirements doc), this `CLAUDE.md`, and `/docs/adr/` (architectural decision records once they land). Apricity is an AI-assisted color grading app built on a **portable Rust core** with **per-platform UI shells**, launching on the web in v1.0, Android + Windows desktop in v1.5, and iOS + macOS in v2.0. Treat `PRD.md` as the source of truth for product scope and the ADRs as the source of truth for technical decisions until those decisions are encoded in code.

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
| `apricity-core` | Pure Rust, `no_std`-friendly where possible | `GradeParameters` (single source of truth for NGA output ↔ Recipe ↔ Manual Console), `ColorSpace`, scene-linear math, parameter validation, `serde` JSON. |
| `apricity-render` | `wgpu` | Stage 1 + Stage 2 compute pipeline in WGSL. CPU reference impl for tests. |
| `apricity-scopes` | `wgpu` + pure Rust | Waveform, vectorscope, RGB parade, histogram. CPU + GPU impls. |
| `apricity-lut` | Pure Rust | `.cube` parser/writer, 3D LUT sampling. |
| `apricity-recipes` | Pure Rust | Recipe model, in-memory store, persistence trait (host implements I/O). |
| `apricity-nga` | `ort` (ONNX Runtime) | Tokenizer, model wrapper, prompt → `GradeParameters` decoder, Explainability differ. |
| `apricity-io` | `image` + format-specific crates | Image read/write with color-space tagging. Video I/O is host-supplied via traits. |
| `apricity-ffi` | `wasm-bindgen` + `uniffi-rs` | The single FFI surface — WASM for web, UniFFI for Kotlin (Android) and Swift (iOS later). |

### Shells (`/shells/`)

| Shell | Stack | Phase |
|---|---|---|
| `web/` | TypeScript + Solid.js + Vite + WebGPU + WASM core | **v1.0 (primary launch)** |
| `desktop/` | Tauri 2 wrapping the web UI; Rust core linked directly | v1.5 (Windows; macOS later) |
| `android/` | Kotlin + Jetpack Compose + UniFFI | v1.5 |
| `ios/` | Swift + SwiftUI + UniFFI | v2.0, gated on Mac access |

### Performance and color invariants

- **Pipeline color space:** 32-bit float scene-linear Rec.2020 internally, regardless of platform. Output transform happens last.
- **Performance floor:** < 16ms per frame (60fps preview) on every supported platform. < 8ms target on native shells with hardware acceleration.
- **NGA:** ONNX, INT8 quantized, < 200MB active memory, < 500MB on-disk. Latency target < 3s p95 on web (WebGPU EP), < 2s on native shells.
- **Single source of truth:** `GradeParameters` in `apricity-core` is the schema for the NGA output, the Recipe file format, and the Manual Console controls. Never redefine it in a shell.

### Architectural seams

When designing a module, check which seam it crosses: Stage 1 (correction) → scene-linear core → Stage 2 (creative grade) → output transform → display/export, plus the orthogonal NGA, Scopes, Recipe, and LUT subsystems. The crate names map 1:1 to these seams. Do not let logic from one seam leak into another (e.g. no Stage 1 math inside `apricity-render`'s Stage 2 shader; no UI assumptions in `apricity-core`).

## Priority Discipline

The PRD assigns P0/P1/P2 to every sub-feature. v1.0 (Q3 2026 target) ships only the P0 set listed in §13 Phase 1. When asked to add or change a feature, check its priority and phase before expanding scope. Shot Coherence Engine, full 3-way wheels/HSL curves, and LUT import/export are explicitly v1.5 — do not pull them forward without discussion.

## Open Questions Not Yet Resolved

The cross-platform pivot resolved most of the original PRD §14 questions (see ADR-0001 through ADR-0004 once written). The remaining open questions are:

- **Onboarding depth** (PRD §14 Q7) — validate after first beta.
- **Web UI framework** — Solid.js recommended; not yet committed. Decide before M4.
- **Web payment provider** — Stripe recommended; alternatives (Lemonsqueezy / Paddle as MoR) under consideration. Decide before M6.
- **WebGPU fallback strategy** — graceful degradation to WebGL2 for preview only, NGA disabled. Decide before M4.
- **Tauri vs egui for desktop (v1.5)** — Tauri 2 recommended; not v1.0 work.

Do not unilaterally answer any of these — flag and ask.

## Code Quality Expectations (apply from the first commit)

The product's correctness claims (60fps preview, < 2s NGA latency, broadcast-legal output, on-device-only AI) are only credible if the code is structured to be measured. Bake this in from day one rather than retrofitting.

### Testability

- **Separate pure logic from device-bound code.** Stage 1 correction math, Stage 2 parameter application, NGA JSON parsing, LUT read/write, scope computations, and parameter validation live in pure-Rust crates (`apricity-core`, `apricity-lut`, `apricity-recipes`, parts of `apricity-scopes` and `apricity-render`) with **no `wgpu`, no `ort`, no shell-specific dependencies**. These crates run `cargo test` on Linux/Windows/macOS without a GPU. `wgpu` shaders, `ort` model inference, and shell UIs are thin adapters over them.
- **Golden-image tests for the rendering pipeline.** Each pipeline stage (and the full pipeline) is tested by rendering a fixed input (checked-in reference PNG or synthetic gradient) with a fixed parameter set and comparing against a checked-in reference output within a documented ΔE tolerance. Run via **lavapipe** (software Vulkan) on Linux CI runners — no real GPU needed.
- **CPU/GPU parity tests.** For every WGSL shader, a CPU reference impl exists in pure Rust. Tests assert CPU and GPU outputs match within 1 LSB at 8-bit on the test patch. Color-math regressions are otherwise invisible.
- **NGA contract tests, not model-quality tests.** Test that the NGA's output conforms to the `GradeParameters` schema, that every field round-trips through the Manual Console, and that the Explainability differ has a template for every parameter the model can emit. Do **not** write tests asserting specific model outputs for specific prompts — those belong in an offline eval harness, not the unit suite.
- **Performance budgets are tests.** The < 16ms preview and < 3s NGA targets from PRD §10.3 are enforced by `criterion` benches (Rust core) and `vitest` perf tests (web shell) that fail CI if regressed beyond a threshold. Run on a designated reference machine in CI, not in the dev's hot loop.
- **Determinism.** Anything seeded (grain, "Surprise Me") takes an explicit seed parameter so tests are reproducible. **No `thread_rng()`, no `Instant::now()`, no `SystemTime::now()` inside pipeline code** — inject the clock and RNG. Same rule on the web side: no `Date.now()` or `Math.random()` in pipeline-adjacent code.

### Documentation

- **Every public Rust item gets a doc comment** (`///`) stating purpose, units, and color space where relevant. "Units and color space" is the part that gets skipped and causes the worst bugs in this domain — a function taking `exposure: f32` must say whether the unit is stops, linear gain, or IRE. Same rule for TypeScript: every exported function in the web shell gets a TSDoc comment with the same units/color-space discipline.
- **Crate-level README per architectural seam.** Each `apricity-*` crate gets a `README.md` (or top-level `//! ` doc-comment) explaining its inputs, outputs, color-space contract, and what it does *not* own. These are the boundaries — documenting them prevents leakage between seams.
- **Document the parameter schema once, in code.** The NGA output schema, the Recipe file format, and the Manual Console controls must be a single Rust source of truth — the `GradeParameters` struct with `serde::{Serialize, Deserialize}` in `apricity-core`. The web shell consumes it via WASM bindings; never redefine it in TypeScript. Tests assert the JSON schema is stable across the FFI boundary.
- **ADRs for non-obvious choices.** When resolving any of the open questions in `PRD.md §14`, picking a specific quantization scheme, choosing between candidate crates (`ort` vs `candle`, `solid` vs `react`), or making any decision that overrides a default — write a short ADR at `/docs/adr/NNNN-title.md`: context, decision, consequences. Three paragraphs is enough.
- **Don't comment what the code says.** Doc comments and ADRs are the right places for prose. Inline comments only for non-obvious *why* (a workaround, a numerical-stability trick, a color-space subtlety). See the project-wide guidance against narrative comments.

### Build & Test Commands

The Cargo workspace and web shell are not yet scaffolded. Once M0 lands, this section will list:

- `cargo build` / `cargo test` / `cargo clippy --all-targets --all-features -- -D warnings` for the Rust core
- `cargo bench` (criterion) for perf budgets
- `wasm-pack build crates/apricity-ffi --target web` for the web FFI artifact
- `pnpm dev` / `pnpm build` / `pnpm test` (vitest) inside `shells/web/`
- How to run a single Rust test (`cargo test -p apricity-core some_test_name`)
- How to run golden-image tests under lavapipe (Linux CI: `LAVAPIPE=1 cargo test -p apricity-render --features golden-images`)

Do not guess these now — write them once the scaffolding actually exists and the commands have been run successfully.
