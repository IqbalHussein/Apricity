//! Recipe model and in-memory store.
//!
//! A [`Recipe`] wraps [`GradeParameters`] plus metadata (name, id, prompt,
//! timestamps, opt-in sharing flag). Persistence is host-supplied through
//! [`RecipeStore`] so the same crate drives `localStorage`/OPFS on the web,
//! files on desktop, and Room/Core Data on mobile. Time is always injected
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
