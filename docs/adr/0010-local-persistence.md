# ADR-0010: Local Persistence in IndexedDB

- Status: accepted
- Date: 2026-11-17

## Context

Recipes, grade history, prompt history (last 20, PRD §8.3), imported LUTs,
usage counters, and settings must persist across sessions. PRD G5 and §9.3 put
cloud sync out of scope for v1.0. The PRD mentions localStorage or OPFS.
localStorage is synchronous and capped at around 5 MB, which recipe thumbnails
and LUTs would exceed. OPFS is file-oriented and awkward for small records.

## Decision

The web shell persists to IndexedDB, through a small typed wrapper with one
object store per record type. The Rust `apricity-recipes` crate owns the Recipe
model, validation and migration, and defines a `RecipeStore` trait. The TS host
implements storage and always round-trips records through the WASM
`parse_recipe`, so migration and sanitization run on every load. No user media
is persisted; only parameters, LUT text, and small thumbnails are stored.

## Consequences

- Async API throughout; no main-thread stalls on large libraries.
- All data stays on the device and is lost if the user clears site data.
  Recipe export/import as JSON files is the backup path.
