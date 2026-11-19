//! Recipe model and in-memory store.
//!
//! A [`Recipe`] wraps [`GradeParameters`] plus metadata (name, id, prompt,
//! timestamps, opt-in sharing flag). Persistence is host-supplied through
//! [`RecipeStore`]. On the web the host implements it over IndexedDB
//! (ADR-0010); desktop and mobile hosts supply their own storage. Time is always injected
//! as epoch milliseconds; this crate never reads a clock.
//!
//! Every `GradeParameters` entering a recipe (creation or parsing) passes
//! through [`apricity_core::validate::sanitize`], so a stored recipe is
//! always renderable.

#![forbid(unsafe_code)]

mod model;
mod store;

pub use model::{
    apply_recipe, new_recipe, parse_recipe, rename, to_json, Recipe, RecipeError,
    RECIPE_SCHEMA_VERSION,
};
pub use store::{put_with_limit, rename_stored, MemoryStore, RecipeStore};
