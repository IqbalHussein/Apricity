# ADR-0005: v1.0 Web Scope

- Status: accepted
- Date: 2026-11-17

## Context

PRD §13 Phase 1 lists the v1.0 P0 set, and CLAUDE.md forbids pulling v1.5
items forward without discussion. When asked to finish the app, the owner
delegated the scope decision. Several v1.5 items are either required by the
P0 schema anyway (§6.1 says the NGA emits lift/gamma/gain, hue curves, split
toning and grain, so the schema, renderer and console must carry them) or
cost little once the Rust core exists (LUT `.cube` I/O, A/B, history,
histogram, false color/zebra overlays).

## Decision

v1.0 web ships the full P0 set plus these web-feasible items: lift/gamma/gain
wheels, 8-band HSL curves, split tone, vignette, seeded grain, `.cube` LUT
import/export, histogram, A/B grade toggle, history/undo, broadcast-legal flag,
and false color/zebra overlays. Excluded from v1.0: the Shot Coherence Engine
(batch), native shells (Tauri/Android/iOS can't be built or verified in the
current toolchain), ProRAW/ProRes, lens corrections, and recipe deep-link
sharing. The `shareable` flag ships in the Recipe schema now (PRD §14 Q4).

## Consequences

- The Free/Pro tier table (PRD §9.1) gates several of these as Pro features,
  so pulling them forward doesn't devalue Pro.
- Every included feature still goes through the single `GradeParameters`
  schema and the CPU/GPU parity suite, so pulling it forward adds no new seams.
- PRD §7.4's "3D LUT (.cube) — web v1.5" row becomes v1.0.
