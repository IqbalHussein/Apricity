use std::collections::BTreeMap;

use crate::{rename, Recipe, RecipeError};

/// Host-implemented recipe persistence. Methods are synchronous; async hosts
/// keep a [`MemoryStore`] cache and flush it themselves.
pub trait RecipeStore {
    /// All recipes, most recently updated first, ties broken by id ascending.
    fn list(&self) -> Vec<Recipe>;
    /// The recipe with this id, if any.
    fn get(&self, id: &str) -> Option<Recipe>;
    /// Insert or replace by `r.id`.
    fn put(&mut self, r: Recipe);
    /// Remove by id; `true` if something was removed.
    fn delete(&mut self, id: &str) -> bool;
}

/// In-memory [`RecipeStore`]; the reference implementation and test double.
#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    items: BTreeMap<String, Recipe>,
}

impl RecipeStore for MemoryStore {
    fn list(&self) -> Vec<Recipe> {
        let mut v: Vec<Recipe> = self.items.values().cloned().collect();
        // BTreeMap iterates by id, and the sort is stable, so equal
        // timestamps stay in id order.
        v.sort_by_key(|r| std::cmp::Reverse(r.updated_at_ms));
        v
    }

    fn get(&self, id: &str) -> Option<Recipe> {
        self.items.get(id).cloned()
    }

    fn put(&mut self, r: Recipe) {
        self.items.insert(r.id.clone(), r);
    }

    fn delete(&mut self, id: &str) -> bool {
        self.items.remove(id).is_some()
    }
}

/// Insert `r`, enforcing an optional recipe-count cap (`None` = unlimited).
///
/// Replacing an existing id never counts against the limit. Returns
/// [`RecipeError::LimitReached`] and leaves the store unchanged when a new id
/// would exceed it.
pub fn put_with_limit<S: RecipeStore>(
    store: &mut S,
    r: Recipe,
    limit: Option<usize>,
) -> Result<(), RecipeError> {
    if let Some(limit) = limit {
        if store.get(&r.id).is_none() && store.list().len() >= limit {
            return Err(RecipeError::LimitReached { limit });
        }
    }
    store.put(r);
    Ok(())
}

/// Rename the stored recipe `id` (see [`rename`]) and persist it.
/// [`RecipeError::NotFound`] if absent; `now_ms` is epoch milliseconds.
pub fn rename_stored<S: RecipeStore>(
    store: &mut S,
    id: &str,
    name: &str,
    now_ms: i64,
) -> Result<(), RecipeError> {
    let mut r = store
        .get(id)
        .ok_or_else(|| RecipeError::NotFound(id.to_owned()))?;
    rename(&mut r, name, now_ms)?;
    store.put(r);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new_recipe;
    use apricity_core::GradeParameters;

    fn rec(id: &str, t: i64) -> Recipe {
        new_recipe(id, id, &GradeParameters::IDENTITY, None, t).unwrap()
    }

    #[test]
    fn put_get_delete() {
        let mut s = MemoryStore::default();
        assert!(s.get("a").is_none());
        s.put(rec("a", 1));
        assert_eq!(s.get("a").unwrap().id, "a");
        assert!(s.delete("a"));
        assert!(!s.delete("a"));
    }

    #[test]
    fn list_is_newest_first_then_id() {
        let mut s = MemoryStore::default();
        s.put(rec("b", 5));
        s.put(rec("a", 5));
        s.put(rec("c", 9));
        s.put(rec("old", 1));
        let ids: Vec<_> = s.list().into_iter().map(|r| r.id).collect();
        assert_eq!(ids, ["c", "a", "b", "old"]);
    }

    #[test]
    fn put_replaces_same_id() {
        let mut s = MemoryStore::default();
        s.put(rec("a", 1));
        s.put(rec("a", 2));
        assert_eq!(s.list().len(), 1);
        assert_eq!(s.get("a").unwrap().updated_at_ms, 2);
    }

    #[test]
    fn limit_blocks_new_but_not_replacement() {
        let mut s = MemoryStore::default();
        put_with_limit(&mut s, rec("a", 1), Some(2)).unwrap();
        put_with_limit(&mut s, rec("b", 2), Some(2)).unwrap();
        assert_eq!(
            put_with_limit(&mut s, rec("c", 3), Some(2)),
            Err(RecipeError::LimitReached { limit: 2 })
        );
        assert!(s.get("c").is_none());
        put_with_limit(&mut s, rec("a", 4), Some(2)).unwrap();
        assert_eq!(s.get("a").unwrap().updated_at_ms, 4);
        put_with_limit(&mut s, rec("c", 5), None).unwrap();
        assert_eq!(s.list().len(), 3);
        assert_eq!(
            put_with_limit(&mut s, rec("d", 6), Some(0)),
            Err(RecipeError::LimitReached { limit: 0 })
        );
    }

    #[test]
    fn rename_stored_paths() {
        let mut s = MemoryStore::default();
        s.put(rec("a", 1));
        rename_stored(&mut s, "a", "Fresh", 7).unwrap();
        let r = s.get("a").unwrap();
        assert_eq!((r.name.as_str(), r.updated_at_ms), ("Fresh", 7));
        assert_eq!(
            rename_stored(&mut s, "zz", "x", 8),
            Err(RecipeError::NotFound("zz".into()))
        );
        assert_eq!(
            rename_stored(&mut s, "a", "", 8),
            Err(RecipeError::EmptyName)
        );
    }
}
