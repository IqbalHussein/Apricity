# apricity-core

Pure-Rust color science core. The single source of truth for `GradeParameters`,
color-space conversions, and scene-linear math.

## Owns

- `GradeParameters` schema (NGA output ↔ Recipe ↔ Manual Console)
- `ColorSpace` enum and primaries
- Pipeline-internal scene-linear Rec.2020 math
- `serde` JSON serialization for the parameter schema

## Does not own

- GPU code (see `apricity-render`)
- ML inference (see `apricity-nga`)
- File I/O (see `apricity-io`)
- Recipe persistence (see `apricity-recipes`)

## Color-space contract

The crate's interior numeric format is **f32 scene-linear Rec.2020**. Inputs in
other spaces must be converted at the seam; outputs are converted at the seam.
No `wgpu`, no `ort`, no shell-specific deps.
