# apricity-recipes

Recipe model, migration, in-memory store, and the persistence trait. Pure
Rust (`apricity-core`, `serde`, `serde_json`, `thiserror`); no I/O and no clock.

## API

- `Recipe` - `schema_version`, `id`, `name`, `params: GradeParameters`,
  `prompt`, `created_at_ms` / `updated_at_ms` (epoch milliseconds, host
  supplied), `shareable` (default `false`, PRD 14 Q4), `thumbnail` (host
  generated data URL).
- `new_recipe`, `parse_recipe`, `to_json`, `rename` - names are trimmed and
  1..=64 chars; ids are 1..=128 chars of `[A-Za-z0-9_-]`. Params always pass
  through `apricity_core::validate::sanitize`.
- `apply_recipe(recipe, current)` - returns the recipe's params, except that
  Stage 1 groups the recipe flagged `auto_exposure`, `auto_white_balance`, or
  `auto_black_point` keep the current image's values (the host re-runs
  analysis). `input_profile` always comes from the recipe.
- `RecipeStore` trait (host implements I/O), `MemoryStore` (list: newest
  `updated_at_ms` first, then id), `put_with_limit` (replacing an existing id
  never counts toward the cap), `rename_stored`.

## Versioning and migration

`RECIPE_SCHEMA_VERSION = 1`. A document without `schema_version` is version 0;
its `params` may be the legacy flat `{exposure, temperature, tint}`, mapped onto
Stage 2. Newer versions fail with `UnsupportedVersion`. `GradeParameters` has
its own `schema_version`; its migrations live with it in `apricity-core`.

## Color-space contract

None of its own: parameter units and color spaces are those of
`GradeParameters`.

## Does not own

Persistence backends, thumbnail rendering, clocks, id generation, sharing or
sync, and the free-tier limit value (callers pass it to `put_with_limit`).
