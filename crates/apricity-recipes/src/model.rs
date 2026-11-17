use apricity_core::validate::sanitize;
use apricity_core::GradeParameters;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current Recipe file-format version. Bump on a breaking change and add a
/// migration arm in [`parse_recipe`].
pub const RECIPE_SCHEMA_VERSION: u32 = 1;

const MAX_NAME_CHARS: usize = 64;
const MAX_ID_CHARS: usize = 128;

/// A saved, nameable grade.
///
/// All timestamps are **milliseconds since the Unix epoch (UTC)**, supplied
/// by the host. `params` is in the units and color spaces documented on
/// [`GradeParameters`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    /// File-format version, [`RECIPE_SCHEMA_VERSION`] when written by this build.
    pub schema_version: u32,
    /// Stable identifier: 1..=128 chars of `[A-Za-z0-9_-]`.
    pub id: String,
    /// User-visible name, trimmed, 1..=64 chars.
    pub name: String,
    /// The two-stage grade. Always sanitized; absent means identity.
    #[serde(default)]
    pub params: GradeParameters,
    /// Natural-language prompt that produced the grade, if any.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Creation time, epoch milliseconds.
    pub created_at_ms: i64,
    /// Last-modified time, epoch milliseconds.
    pub updated_at_ms: i64,
    /// Opt-in sharing flag (PRD §14 Q4); `false` unless the user enables it.
    #[serde(default)]
    pub shareable: bool,
    /// Host-generated preview as a `data:` URL; never produced by this crate.
    #[serde(default)]
    pub thumbnail: Option<String>,
}

/// Errors from recipe construction, parsing, and storage.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RecipeError {
    /// Name is empty after trimming.
    #[error("recipe name is empty")]
    EmptyName,
    /// Name exceeds 64 characters after trimming.
    #[error("recipe name exceeds 64 characters")]
    NameTooLong,
    /// Id is empty, longer than 128 chars, or has characters outside `[A-Za-z0-9_-]`.
    #[error("recipe id must be 1..=128 chars of [A-Za-z0-9_-]")]
    InvalidId,
    /// Input is not valid recipe JSON; carries the parser message.
    #[error("invalid recipe JSON: {0}")]
    Parse(String),
    /// File was written by a newer build; carries its version.
    #[error("recipe schema version {0} is newer than supported {RECIPE_SCHEMA_VERSION}")]
    UnsupportedVersion(u32),
    /// Store already holds `limit` recipes (free-tier cap).
    #[error("recipe limit of {limit} reached")]
    LimitReached {
        /// The configured maximum.
        limit: usize,
    },
    /// No recipe with this id exists in the store.
    #[error("recipe {0:?} not found")]
    NotFound(String),
}

fn check_name(name: &str) -> Result<String, RecipeError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(RecipeError::EmptyName);
    }
    if n.chars().count() > MAX_NAME_CHARS {
        return Err(RecipeError::NameTooLong);
    }
    Ok(n.to_owned())
}

fn check_id(id: &str) -> Result<(), RecipeError> {
    let ok = !id.is_empty()
        && id.len() <= MAX_ID_CHARS
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(RecipeError::InvalidId)
    }
}

/// Create a recipe. `name` is trimmed (1..=64 chars), `id` must be 1..=128
/// chars of `[A-Za-z0-9_-]`, `params` is sanitized, and both timestamps are
/// `now_ms` (epoch milliseconds, UTC).
pub fn new_recipe(
    id: &str,
    name: &str,
    params: &GradeParameters,
    prompt: Option<&str>,
    now_ms: i64,
) -> Result<Recipe, RecipeError> {
    check_id(id)?;
    let name = check_name(name)?;
    Ok(Recipe {
        schema_version: RECIPE_SCHEMA_VERSION,
        id: id.to_owned(),
        name,
        params: sanitize(params),
        prompt: prompt.map(str::to_owned),
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
        shareable: false,
        thumbnail: None,
    })
}

/// Parse recipe JSON, migrating older versions and sanitizing `params`.
///
/// Version 0 is any document without `schema_version`; its `params` may be
/// the legacy flat form `{"exposure", "temperature", "tint"}` (no `stage1` /
/// `stage2` keys), which maps onto Stage 2. Documents newer than
/// [`RECIPE_SCHEMA_VERSION`] fail with [`RecipeError::UnsupportedVersion`].
pub fn parse_recipe(json: &str) -> Result<Recipe, RecipeError> {
    let mut doc: Value =
        serde_json::from_str(json).map_err(|e| RecipeError::Parse(e.to_string()))?;
    let obj = doc
        .as_object_mut()
        .ok_or_else(|| RecipeError::Parse("recipe must be a JSON object".into()))?;

    let version = match obj.get("schema_version") {
        None => 0,
        Some(v) => v
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| {
                RecipeError::Parse("schema_version must be a non-negative integer".into())
            })?,
    };
    if version > RECIPE_SCHEMA_VERSION {
        return Err(RecipeError::UnsupportedVersion(version));
    }
    if version == 0 {
        migrate_v0(obj);
    }

    let mut r: Recipe =
        serde_json::from_value(doc).map_err(|e| RecipeError::Parse(e.to_string()))?;
    check_id(&r.id)?;
    r.name = check_name(&r.name)?;
    r.params = sanitize(&r.params);
    r.schema_version = RECIPE_SCHEMA_VERSION;
    Ok(r)
}

fn migrate_v0(obj: &mut serde_json::Map<String, Value>) {
    obj.insert("schema_version".into(), RECIPE_SCHEMA_VERSION.into());
    let created = obj.get("created_at_ms").cloned().unwrap_or(0.into());
    obj.entry("created_at_ms").or_insert(0.into());
    obj.entry("updated_at_ms").or_insert(created);

    let Some(Value::Object(params)) = obj.get_mut("params") else {
        return;
    };
    if params.contains_key("stage1") || params.contains_key("stage2") {
        return;
    }
    let mut stage2 = serde_json::Map::new();
    for key in ["exposure", "temperature", "tint"] {
        if let Some(v) = params.get(key) {
            stage2.insert(key.into(), v.clone());
        }
    }
    let mut migrated = serde_json::Map::new();
    migrated.insert("stage2".into(), Value::Object(stage2));
    obj.insert("params".into(), Value::Object(migrated));
}

/// Serialize to pretty-printed JSON (stable field order).
pub fn to_json(r: &Recipe) -> String {
    // `Recipe` contains only string-keyed maps and plain scalars.
    serde_json::to_string_pretty(r).expect("Recipe serialization is infallible")
}

/// Rename in place (trimmed, 1..=64 chars) and stamp `updated_at_ms = now_ms`.
/// On error the recipe is unchanged.
pub fn rename(r: &mut Recipe, name: &str, now_ms: i64) -> Result<(), RecipeError> {
    r.name = check_name(name)?;
    r.updated_at_ms = now_ms;
    Ok(())
}

/// Parameters to render when applying `recipe` to an image currently graded
/// with `current`.
///
/// Returns the recipe's parameters, except that Stage 1 fields the recipe
/// marked as auto-analysed (`auto_exposure`, `auto_white_balance`,
/// `auto_black_point`) keep `current`'s values: the host re-runs analysis for
/// the new image and a stale per-image correction must not be imported.
/// `input_profile` always comes from the recipe. The result is sanitized.
pub fn apply_recipe(recipe: &Recipe, current: &GradeParameters) -> GradeParameters {
    let mut out = recipe.params.clone();
    let s1 = &mut out.stage1;
    let cur = &current.stage1;
    if s1.auto_exposure {
        s1.exposure = cur.exposure;
    }
    if s1.auto_white_balance {
        s1.temperature = cur.temperature;
        s1.tint = cur.tint;
    }
    if s1.auto_black_point {
        s1.black_point = cur.black_point;
    }
    sanitize(&out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apricity_core::LogProfile;

    fn graded() -> GradeParameters {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.contrast = 20.0;
        p.stage2.saturation = -10.0;
        p
    }

    #[test]
    fn new_recipe_trims_validates_and_sanitizes() {
        let mut wild = graded();
        wild.stage2.contrast = 1e9;
        wild.stage2.exposure = f32::NAN;
        let r = new_recipe("a_b-1", "  Warm Teal  ", &wild, Some("warm teal"), 42).unwrap();
        assert_eq!(r.name, "Warm Teal");
        assert_eq!((r.created_at_ms, r.updated_at_ms), (42, 42));
        assert!(!r.shareable && r.thumbnail.is_none());
        assert_eq!(r.prompt.as_deref(), Some("warm teal"));
        assert!(apricity_core::validate::validate(&r.params).is_empty());
    }

    #[test]
    fn new_recipe_rejects_bad_name_and_id() {
        let p = graded();
        assert_eq!(
            new_recipe("a", "   ", &p, None, 0),
            Err(RecipeError::EmptyName)
        );
        let long = "x".repeat(65);
        assert_eq!(
            new_recipe("a", &long, &p, None, 0),
            Err(RecipeError::NameTooLong)
        );
        assert!(new_recipe("a", &"x".repeat(64), &p, None, 0).is_ok());
        for bad in ["", "has space", "slash/", "é", &"i".repeat(129)] {
            assert_eq!(
                new_recipe(bad, "n", &p, None, 0),
                Err(RecipeError::InvalidId)
            );
        }
        assert!(new_recipe(&"i".repeat(128), "n", &p, None, 0).is_ok());
    }

    #[test]
    fn json_round_trip() {
        let mut r = new_recipe("r1", "Look", &graded(), None, 5).unwrap();
        r.shareable = true;
        r.thumbnail = Some("data:image/png;base64,AAAA".into());
        assert_eq!(parse_recipe(&to_json(&r)).unwrap(), r);
    }

    #[test]
    fn parse_sanitizes_out_of_range_params() {
        let r = new_recipe("r1", "Look", &graded(), None, 5).unwrap();
        let json = to_json(&r).replace("\"contrast\": 20.0", "\"contrast\": 9999.0");
        let back = parse_recipe(&json).unwrap();
        assert!(back.params.stage2.contrast <= 100.0);
        assert!(apricity_core::validate::validate(&back.params).is_empty());
    }

    #[test]
    fn parse_errors() {
        assert!(matches!(parse_recipe("nope"), Err(RecipeError::Parse(_))));
        assert!(matches!(parse_recipe("[]"), Err(RecipeError::Parse(_))));
        assert!(matches!(
            parse_recipe(r#"{"schema_version":"1"}"#),
            Err(RecipeError::Parse(_))
        ));
        assert_eq!(
            parse_recipe(r#"{"schema_version":2,"id":"a","name":"n"}"#),
            Err(RecipeError::UnsupportedVersion(2))
        );
        assert_eq!(
            parse_recipe(
                r#"{"schema_version":1,"id":"a b","name":"n","params":{},"created_at_ms":0,"updated_at_ms":0}"#
            ),
            Err(RecipeError::InvalidId)
        );
        assert_eq!(
            parse_recipe(
                r#"{"schema_version":1,"id":"a","name":" ","params":{},"created_at_ms":0,"updated_at_ms":0}"#
            ),
            Err(RecipeError::EmptyName)
        );
    }

    #[test]
    fn migrates_v0_flat_params() {
        let json = r#"{"id":"old","name":"Legacy","created_at_ms":7,
            "params":{"exposure":0.5,"temperature":10.0,"tint":-5.0}}"#;
        let r = parse_recipe(json).unwrap();
        assert_eq!(r.schema_version, RECIPE_SCHEMA_VERSION);
        assert_eq!(r.params.stage2.exposure, 0.5);
        assert_eq!(r.params.stage2.temperature, 10.0);
        assert_eq!(r.params.stage2.tint, -5.0);
        assert_eq!(r.params.stage1, GradeParameters::IDENTITY.stage1);
        assert_eq!((r.created_at_ms, r.updated_at_ms), (7, 7));
        assert!(!r.shareable);
    }

    #[test]
    fn v0_with_structured_or_missing_params_still_parses() {
        let structured = to_json(&new_recipe("a", "n", &graded(), None, 1).unwrap())
            .replace("\"schema_version\": 1,\n  \"id\"", "\"id\"");
        let r = parse_recipe(&structured).unwrap();
        assert_eq!(r.params.stage2.contrast, 20.0);
        let r = parse_recipe(r#"{"id":"a","name":"n"}"#).unwrap();
        assert_eq!(r.params, sanitize(&GradeParameters::IDENTITY));
    }

    #[test]
    fn rename_updates_timestamp_only_on_success() {
        let mut r = new_recipe("a", "Old", &graded(), None, 1).unwrap();
        rename(&mut r, " New ", 9).unwrap();
        assert_eq!(
            (r.name.as_str(), r.updated_at_ms, r.created_at_ms),
            ("New", 9, 1)
        );
        assert_eq!(rename(&mut r, "", 99), Err(RecipeError::EmptyName));
        assert_eq!((r.name.as_str(), r.updated_at_ms), ("New", 9));
    }

    #[test]
    fn apply_returns_recipe_params_without_auto_flags() {
        let mut p = graded();
        p.stage1.exposure = 1.0;
        p.stage1.input_profile = LogProfile::SonySLog3;
        let r = new_recipe("a", "n", &p, None, 0).unwrap();
        let mut cur = GradeParameters::IDENTITY;
        cur.stage1.exposure = -1.0;
        assert_eq!(apply_recipe(&r, &cur), r.params);
    }

    #[test]
    fn apply_keeps_current_auto_fields_but_recipe_input_profile() {
        let mut p = graded();
        p.stage1.input_profile = LogProfile::ArriLogC3;
        p.stage1.auto_exposure = true;
        p.stage1.auto_white_balance = true;
        p.stage1.auto_black_point = true;
        p.stage1.exposure = 2.0;
        p.stage1.temperature = 50.0;
        p.stage1.tint = 20.0;
        p.stage1.black_point.r = 0.05;
        let r = new_recipe("a", "n", &p, None, 0).unwrap();

        let mut cur = GradeParameters::IDENTITY;
        cur.stage1.exposure = -0.5;
        cur.stage1.temperature = -12.0;
        cur.stage1.tint = 3.0;
        cur.stage1.black_point.g = 0.01;
        cur.stage1.input_profile = LogProfile::None;

        let out = apply_recipe(&r, &cur);
        assert_eq!(out.stage1.exposure, -0.5);
        assert_eq!(out.stage1.temperature, -12.0);
        assert_eq!(out.stage1.tint, 3.0);
        assert_eq!(out.stage1.black_point, cur.stage1.black_point);
        assert_eq!(out.stage1.input_profile, LogProfile::ArriLogC3);
        assert_eq!(out.stage2, r.params.stage2);
    }

    #[test]
    fn apply_only_replaces_flagged_groups() {
        let mut p = GradeParameters::IDENTITY;
        p.stage1.auto_exposure = true;
        p.stage1.temperature = 30.0;
        let r = new_recipe("a", "n", &p, None, 0).unwrap();
        let mut cur = GradeParameters::IDENTITY;
        cur.stage1.exposure = 0.7;
        cur.stage1.temperature = -30.0;
        let out = apply_recipe(&r, &cur);
        assert_eq!(out.stage1.exposure, 0.7);
        assert_eq!(out.stage1.temperature, 30.0);
    }
}
