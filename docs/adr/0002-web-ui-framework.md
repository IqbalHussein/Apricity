# ADR-0002: Web UI Framework — Solid.js

- Status: accepted
- Date: 2026-05-05

## Context

The web shell is a creative tool with high-frequency state updates: sliders
emitting dozens of events per second, scope canvases re-rendering at 60fps,
WebGPU draws keyed to fine-grained reactive state. Candidates considered were
React, Solid.js, Svelte, and vanilla TS + signals.

React's virtual-DOM reconciliation has measurable per-update overhead that
shows up at high update rates; the workarounds (memoization, refs, manual
batching) add complexity precisely where the product needs to stay fast.

## Decision

Use **Solid.js + Vite + TypeScript**. Solid's fine-grained reactivity model
updates only the DOM nodes that depend on a changed signal — no reconciler,
no virtual DOM. Slider drags and scope updates flow into signals directly.
Bundle size and runtime overhead are both lower than React.

## Consequences

- Smaller talent pool than React, but the API surface is small enough to learn
  in a day.
- Solid's ecosystem is smaller; some libraries will need to be written or
  ported. Acceptable given the small dependency footprint we want anyway.
- The reactivity primitives (`createSignal`, `createMemo`, `createEffect`)
  map cleanly onto the Manual Console parameter graph.
- Tauri 2 (v1.5 desktop) reuses this same UI codebase — Solid choice locks in
  desktop too.
