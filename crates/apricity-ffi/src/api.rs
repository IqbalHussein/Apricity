//! Plain-Rust API behind every FFI binding.
//!
//! Nothing here touches `wasm-bindgen`, the GPU or the DOM, so all of it runs
//! under `cargo test` on any host. The `wasm` module is a thin serde adapter
//! over these functions. Units and color spaces are those of the underlying
//! crates (see `apricity_core::param_spec`); pixel buffers are **sRGB-encoded
//! RGBA8**, row-major, top row first.

use apricity_core::analysis::{analyze, Stage1Analysis};
use apricity_core::param_spec::{spec_table, SpecTable};
use apricity_core::validate::{sanitize, validate, ValidationIssue};
use apricity_core::{CorrectionParameters, GradeParameters, LogProfile};
use apricity_lut::{parse_cube, write_cube, Lut3d, MAX_SIZE};
use apricity_nga::explain::{diff, summary};
use apricity_nga::{
    lexicon_terms, GradeAgent, LexiconAgent, NgaResult, ParameterChange, PromptMode,
};
use apricity_recipes::{apply_recipe, new_recipe, parse_recipe, rename, to_json, Recipe};
use apricity_render::cpu;
use apricity_render::{bake_lut, prepare, BakeScope, RenderContext};
use apricity_scopes::{vectorscope_graticule, Graticule};
use serde::{Deserialize, Serialize};

use crate::compose::{box_downscale, Image};

/// Maximum longest side of the copy analysed by Stage 1 auto-analysis, in pixels.
pub const ANALYSIS_MAX_SIDE: u32 = 256;

/// Error crossing the FFI boundary; becomes a thrown JS `Error` on the web.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError(pub String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

/// Result alias for the FFI API.
pub type ApiResult<T> = Result<T, ApiError>;

/// One selectable camera log profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
pub struct LogProfileInfo {
    /// Serialized `input_profile` value.
    pub id: LogProfile,
    /// Human-readable name.
    pub label: String,
}

/// Title and size of a parsed `.cube` LUT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
pub struct LutInfo {
    /// `TITLE` keyword, if present.
    pub title: Option<String>,
    /// Points per axis.
    pub size: u32,
}

/// Explainability diff between two grades.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
pub struct ExplainResult {
    /// Every parameter that moved, in spec-table order.
    pub changes: Vec<ParameterChange>,
    /// Header line, e.g. "The AI made 6 adjustments."
    pub summary: String,
}

/// Which renderer an `Engine` ended up with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    /// wgpu on the browser's WebGPU.
    Webgpu,
    /// The CPU reference renderer into a 2D canvas.
    Cpu,
}

/// Library version, to confirm the shell loaded the matching WASM.
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// The full scalar/curve/HSL parameter table.
pub fn param_specs() -> SpecTable {
    spec_table()
}

/// Selectable input log profiles in UI order.
pub fn log_profiles() -> Vec<LogProfileInfo> {
    LogProfile::ALL
        .iter()
        .map(|&id| LogProfileInfo {
            id,
            label: id.label().to_owned(),
        })
        .collect()
}

/// The identity grade.
pub fn identity_params() -> GradeParameters {
    GradeParameters::IDENTITY
}

/// Clamp every parameter into its spec range.
pub fn sanitize_params(p: &GradeParameters) -> GradeParameters {
    sanitize(p)
}

/// Report every out-of-range or malformed parameter.
pub fn validate_params(p: &GradeParameters) -> Vec<ValidationIssue> {
    validate(p)
}

/// Apply a natural-language prompt to `current` with the on-device lexicon agent.
pub fn nga_apply(prompt: &str, current: &GradeParameters, mode: PromptMode) -> NgaResult {
    LexiconAgent.apply(prompt, current, mode)
}

/// "Surprise Me": a grade fully determined by `seed`, within the mood of `last_prompt`.
pub fn nga_surprise(current: &GradeParameters, last_prompt: Option<&str>, seed: u64) -> NgaResult {
    LexiconAgent.surprise(current, last_prompt, seed)
}

/// Explainability diff of two grades with no prompt attributions (manual edits, recipes).
pub fn explain_diff(before: &GradeParameters, after: &GradeParameters) -> ExplainResult {
    let changes = diff(before, after, &[]);
    let summary = summary(&changes);
    ExplainResult { changes, summary }
}

/// Every phrase the lexicon understands.
pub fn lexicon_terms_list() -> Vec<String> {
    lexicon_terms().into_iter().map(str::to_owned).collect()
}

/// Create a recipe; `now_ms` is epoch milliseconds (UTC), injected by the host.
pub fn recipe_new(
    id: &str,
    name: &str,
    params: &GradeParameters,
    prompt: Option<&str>,
    now_ms: i64,
) -> ApiResult<Recipe> {
    new_recipe(id, name, params, prompt, now_ms).map_err(|e| ApiError::new(e.to_string()))
}

/// Parse (and migrate, and sanitize) recipe JSON.
pub fn recipe_parse(json: &str) -> ApiResult<Recipe> {
    parse_recipe(json).map_err(|e| ApiError::new(e.to_string()))
}

/// Pretty-printed recipe JSON.
pub fn recipe_to_json(recipe: &Recipe) -> String {
    to_json(recipe)
}

/// Parameters to render when applying `recipe` over an image graded with `current`.
pub fn recipe_apply(recipe: &Recipe, current: &GradeParameters) -> GradeParameters {
    apply_recipe(recipe, current)
}

/// Rename a recipe and stamp `updated_at_ms = now_ms`.
pub fn recipe_rename(recipe: &Recipe, name: &str, now_ms: i64) -> ApiResult<Recipe> {
    let mut r = recipe.clone();
    rename(&mut r, name, now_ms).map_err(|e| ApiError::new(e.to_string()))?;
    Ok(r)
}

/// Parse `.cube` text into a [`Lut3d`].
pub fn lut_parse_text(text: &str) -> ApiResult<Lut3d> {
    parse_cube(text).map_err(|e| ApiError::new(e.to_string()))
}

/// Title and size of `.cube` text; errors if it does not parse.
pub fn lut_parse(text: &str) -> ApiResult<LutInfo> {
    let l = lut_parse_text(text)?;
    Ok(lut_info(&l))
}

/// [`LutInfo`] of a parsed LUT.
pub fn lut_info(l: &Lut3d) -> LutInfo {
    LutInfo {
        title: l.title.clone(),
        size: l.size as u32,
    }
}

/// Bake `params` into `.cube` text; `size` is clamped to `2..=129`.
pub fn lut_bake(
    params: &GradeParameters,
    size: u32,
    scope: BakeScope,
    title: Option<String>,
) -> String {
    let size = (size as usize).clamp(2, MAX_SIZE);
    write_cube(&bake_lut(&sanitize(params), size, scope, title))
}

/// Stage 1 auto-analysis of an RGBA8 image.
///
/// The image is box-downscaled to at most [`ANALYSIS_MAX_SIDE`] pixels, decoded
/// with `stage1.input_profile` to scene-linear Rec.2020, and analysed against
/// `stage1.exposure_target`.
pub fn analyze_stage1(
    rgba: &[u8],
    w: u32,
    h: u32,
    stage1: &CorrectionParameters,
) -> ApiResult<Stage1Analysis> {
    let img = Image::new(rgba.to_vec(), w, h)?;
    Ok(analyze_image(&img, stage1))
}

pub(crate) fn analyze_image(img: &Image, stage1: &CorrectionParameters) -> Stage1Analysis {
    let small = box_downscale(img, ANALYSIS_MAX_SIDE, ANALYSIS_MAX_SIDE);
    let mut p = GradeParameters::IDENTITY;
    p.stage1 = stage1.clone();
    let u = prepare(
        &sanitize(&p),
        &RenderContext::srgb(small.width, small.height),
    );
    let linear = cpu::decode_linear(&u, &small.rgba);
    analyze(&linear, stage1.exposure_target)
}

/// Render an RGBA8 image through the whole pipeline on the CPU, to sRGB RGBA8.
///
/// `full_width` is the width in pixels of the full-resolution image this
/// `w`-wide buffer stands for; grain is generated in full-resolution
/// coordinates (`source_scale = full_width / w`). `lut_text` is `.cube` text
/// used when `params.stage2.lut_id` is set.
pub fn render_cpu_rgba(
    rgba: &[u8],
    w: u32,
    h: u32,
    params: &GradeParameters,
    lut_text: Option<&str>,
    full_width: f64,
) -> ApiResult<Vec<u8>> {
    let img = Image::new(rgba.to_vec(), w, h)?;
    let lut = lut_text.map(lut_parse_text).transpose()?;
    let mut ctx = RenderContext::srgb(w, h);
    ctx.source_scale = scale_for(full_width, w);
    ctx.lut_size = lut.as_ref().map_or(0, |l| l.size as u32);
    let u = prepare(&sanitize(params), &ctx);
    Ok(cpu::render_rgba8(&u, lut.as_ref(), &img.rgba, w, h))
}

/// `full_width / w`, or 1 when either is non-positive or not finite.
pub fn scale_for(full_width: f64, w: u32) -> f32 {
    if w == 0 || !full_width.is_finite() || full_width <= 0.0 {
        1.0
    } else {
        (full_width / w as f64) as f32
    }
}

/// Vectorscope overlay geometry.
pub fn graticule() -> Graticule {
    vectorscope_graticule()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::new();
        for y in 0..h {
            for x in 0..w {
                v.extend_from_slice(&[
                    (x * 255 / (w - 1)) as u8,
                    (y * 255 / (h - 1)) as u8,
                    ((x + y) * 255 / (w + h - 2)) as u8,
                    255,
                ]);
            }
        }
        v
    }

    #[test]
    fn identity_cpu_render_reproduces_source() {
        let src = ramp(16, 8);
        let out = render_cpu_rgba(&src, 16, 8, &identity_params(), None, 16.0).unwrap();
        for (a, b) in src.iter().zip(&out) {
            assert!((*a as i32 - *b as i32).abs() <= 1);
        }
    }

    #[test]
    fn bad_buffer_length_is_an_error() {
        assert!(render_cpu_rgba(&[0; 7], 2, 2, &identity_params(), None, 2.0).is_err());
    }

    #[test]
    fn lut_bake_round_trips_through_parse() {
        let text = lut_bake(
            &identity_params(),
            33,
            BakeScope::Creative,
            Some("T".into()),
        );
        let info = lut_parse(&text).unwrap();
        assert_eq!(
            info,
            LutInfo {
                title: Some("T".into()),
                size: 33
            }
        );
        assert!(lut_parse("nonsense").is_err());
    }

    #[test]
    fn recipe_round_trip_and_errors() {
        let r = recipe_new("a-1", " Warm ", &identity_params(), Some("warm"), 5).unwrap();
        assert_eq!(r.name, "Warm");
        let back = recipe_parse(&recipe_to_json(&r)).unwrap();
        assert_eq!(back, r);
        assert!(recipe_new("bad id", "x", &identity_params(), None, 0).is_err());
        let r2 = recipe_rename(&r, "Cool", 9).unwrap();
        assert_eq!((r2.name.as_str(), r2.updated_at_ms), ("Cool", 9));
    }

    #[test]
    fn nga_changes_carry_rationale_and_are_deterministic() {
        let a = nga_apply("warm golden hour", &identity_params(), PromptMode::Auto);
        assert!(!a.changes.is_empty());
        assert!(a.changes.iter().all(|c| !c.rationale.is_empty()));
        let s1 = nga_surprise(&identity_params(), None, 7);
        assert_eq!(s1, nga_surprise(&identity_params(), None, 7));
        let e = explain_diff(&identity_params(), &a.params);
        assert!(!e.changes.is_empty());
    }

    #[test]
    fn analysis_runs_on_flat_grey() {
        let src = [118u8, 118, 118, 255].repeat(64);
        let a = analyze_stage1(&src, 8, 8, &identity_params().stage1).unwrap();
        assert!(a.median_luminance > 0.0);
    }

    #[test]
    fn spec_table_and_profiles_are_populated() {
        assert!(!param_specs().scalars.is_empty());
        assert_eq!(log_profiles().len(), LogProfile::ALL.len());
        assert!(!lexicon_terms_list().is_empty());
        assert!(validate_params(&identity_params()).is_empty());
        assert_eq!(scale_for(0.0, 4), 1.0);
        assert_eq!(scale_for(8.0, 4), 2.0);
    }
}
