# ADR-0001: Platform Strategy — Portable Rust Core + Per-Platform UI Shells

- Status: accepted
- Date: 2026-05-05

## Context

The original PRD assumed an iOS-first stack: Swift, SwiftUI, Metal, Core ML,
StoreKit 2. The developer works on Windows + WSL with no Mac access, no
Apple Developer Program membership, and a $0 development budget. None of the
iOS toolchain is available locally; cloud Mac options either cost real money
or come with allocation minimums incompatible with solo iteration.

Four platform shapes were evaluated:

1. **Web-only PWA** — fastest to ship, but iOS PWA UX gaps undercut the
   "professional" claim.
2. **Rust core + native UI shells** — one engine, many shells; defers Mac work.
3. **Flutter** — single codebase, but adds a runtime layer and weakens the
   GPU/ML story.
4. **Capacitor / web-in-a-shell** — same iOS PWA limitations as (1).

## Decision

Adopt **Shape B**: a portable Rust core (`apricity-*` crates) compiled to WASM
for the web and to native libraries for Android, Windows, iOS, and macOS via
`uniffi-rs`. Rendering uses `wgpu` (one WGSL source, all backends). ML uses
ONNX Runtime via `ort` with per-platform Execution Providers.

Sequencing: **web (v1.0) → Android + Windows desktop (v1.5) → iOS + macOS
(v2.0, gated on Mac access)**.

## Consequences

- The Rust core is the durable IP. No rewrite when each shell ships.
- v1.0 is a real, deployable product on web — not a placeholder.
- iPhone users get the web build first; native iOS arrives in v2.0.
- Adds Rust + WASM + wgpu + ort to the project's tooling surface.
- ProRAW, ProRes, and HEIC are deferred to native shells (v1.5+); v1.0 web
  ships JPEG/PNG/WebP.
- Privacy invariant (PRD G5) is preserved across every shell: ONNX inference
  is on-device on every target.
