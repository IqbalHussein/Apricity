//! `wasm-bindgen` adapter over [`crate::api`]. Values cross the boundary as
//! plain JS objects via `serde-wasm-bindgen`; their TypeScript types are the
//! `tsify-next` declarations generated from the Rust structs. Failures throw
//! a JS `Error`.

use apricity_nga::PromptMode;
use apricity_render::BakeScope;
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::api;
use crate::compose::{ScopeRaster, ScopesResult};

/// Install the panic hook so Rust panics reach the console with a message.
#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
}

/// Serialize to a plain JS value through JSON text.
///
/// Going through `serde_json` keeps `f32` values at their shortest decimal
/// form (`0.45`, not `0.44999998807907104`), which direct serde-wasm-bindgen
/// serialization would widen to `f64`. `None` becomes `null`. Byte buffers
/// do not belong here; see [`scopes_to_js`].
pub(crate) fn to_js<T: Serialize>(v: &T) -> Result<JsValue, JsError> {
    let text = serde_json::to_string(v).map_err(|e| JsError::new(&e.to_string()))?;
    js_sys::JSON::parse(&text).map_err(|_| JsError::new("internal: invalid JSON produced"))
}

/// Convert scopes to a JS object whose rasters carry `Uint8Array` pixels.
pub(crate) fn scopes_to_js(r: &ScopesResult) -> Result<JsValue, JsError> {
    let obj = js_sys::Object::new();
    let set = |k: &str, v: JsValue| {
        js_sys::Reflect::set(&obj, &JsValue::from_str(k), &v)
            .map(|_| ())
            .map_err(|_| JsError::new("internal: cannot build scopes object"))
    };
    let raster = |r: &ScopeRaster| -> Result<JsValue, JsError> {
        let o = js_sys::Object::new();
        for (k, v) in [("width", r.width), ("height", r.height)] {
            js_sys::Reflect::set(&o, &JsValue::from_str(k), &JsValue::from_f64(v as f64))
                .map_err(|_| JsError::new("internal: cannot build raster"))?;
        }
        js_sys::Reflect::set(
            &o,
            &JsValue::from_str("rgba"),
            &js_sys::Uint8Array::from(r.rgba.as_slice()).into(),
        )
        .map_err(|_| JsError::new("internal: cannot build raster"))?;
        Ok(o.into())
    };
    for (k, v) in [
        ("waveform", &r.waveform),
        ("parade", &r.parade),
        ("vectorscope", &r.vectorscope),
    ] {
        if let Some(v) = v {
            set(k, raster(v)?)?;
        }
    }
    if let Some(h) = &r.histogram {
        set("histogram", to_js(h)?)?;
    }
    if let Some(l) = &r.legal {
        set("legal", to_js(l)?)?;
    }
    Ok(obj.into())
}

/// Deserialize a JS value, throwing a JS `Error` that names the problem.
pub(crate) fn from_js<T: DeserializeOwned>(v: JsValue) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(v).map_err(|e| JsError::new(&e.to_string()))
}

/// Library version, to confirm the shell loaded the matching WASM.
#[wasm_bindgen]
pub fn version() -> String {
    api::version()
}

/// The full parameter table: scalars (path, label, group, unit, stage, range,
/// default, step), curve paths and HSL band names/centers.
#[wasm_bindgen(js_name = paramSpecs, unchecked_return_type = "SpecTable")]
pub fn param_specs() -> Result<JsValue, JsError> {
    to_js(&api::param_specs())
}

/// Selectable camera log profiles in UI order.
#[wasm_bindgen(js_name = logProfiles, unchecked_return_type = "LogProfileInfo[]")]
pub fn log_profiles() -> Result<JsValue, JsError> {
    to_js(&api::log_profiles())
}

/// The identity grade.
#[wasm_bindgen(js_name = identityParams, unchecked_return_type = "GradeParameters")]
pub fn identity_params() -> Result<JsValue, JsError> {
    to_js(&api::identity_params())
}

/// Clamp every parameter into its spec range.
#[wasm_bindgen(js_name = sanitizeParams, unchecked_return_type = "GradeParameters")]
pub fn sanitize_params(
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] p: JsValue,
) -> Result<JsValue, JsError> {
    to_js(&api::sanitize_params(&from_js(p)?))
}

/// List out-of-range or malformed parameters.
#[wasm_bindgen(js_name = validateParams, unchecked_return_type = "ValidationIssue[]")]
pub fn validate_params(
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] p: JsValue,
) -> Result<JsValue, JsError> {
    to_js(&api::validate_params(&from_js(p)?))
}

/// Apply a prompt to `current` with the on-device agent.
#[wasm_bindgen(js_name = ngaApply, unchecked_return_type = "NgaResult")]
pub fn nga_apply(
    prompt: &str,
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] current: JsValue,
    #[wasm_bindgen(unchecked_param_type = "PromptMode")] mode: JsValue,
) -> Result<JsValue, JsError> {
    let mode: PromptMode = from_js(mode)?;
    to_js(&api::nga_apply(prompt, &from_js(current)?, mode))
}

/// "Surprise Me"; `seed` is a non-negative integer below 2^53.
#[wasm_bindgen(js_name = ngaSurprise, unchecked_return_type = "NgaResult")]
pub fn nga_surprise(
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] current: JsValue,
    last_prompt: Option<String>,
    seed: f64,
) -> Result<JsValue, JsError> {
    to_js(&api::nga_surprise(
        &from_js(current)?,
        last_prompt.as_deref(),
        seed as u64,
    ))
}

/// Explainability diff of two grades.
#[wasm_bindgen(js_name = explainDiff, unchecked_return_type = "ExplainResult")]
pub fn explain_diff(
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] before: JsValue,
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] after: JsValue,
) -> Result<JsValue, JsError> {
    to_js(&api::explain_diff(&from_js(before)?, &from_js(after)?))
}

/// Every phrase the lexicon understands.
#[wasm_bindgen(js_name = lexiconTerms)]
pub fn lexicon_terms() -> Vec<String> {
    api::lexicon_terms_list()
}

/// Create a recipe; `now_ms` is epoch milliseconds.
#[wasm_bindgen(js_name = recipeNew, unchecked_return_type = "Recipe")]
pub fn recipe_new(
    id: &str,
    name: &str,
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] params: JsValue,
    prompt: Option<String>,
    now_ms: f64,
) -> Result<JsValue, JsError> {
    let r = api::recipe_new(
        id,
        name,
        &from_js(params)?,
        prompt.as_deref(),
        now_ms as i64,
    )?;
    to_js(&r)
}

/// Parse (and migrate) recipe JSON; throws on malformed or too-new input.
#[wasm_bindgen(js_name = recipeParse, unchecked_return_type = "Recipe")]
pub fn recipe_parse(json: &str) -> Result<JsValue, JsError> {
    to_js(&api::recipe_parse(json)?)
}

/// Recipe as pretty-printed JSON.
#[wasm_bindgen(js_name = recipeToJson)]
pub fn recipe_to_json(
    #[wasm_bindgen(unchecked_param_type = "Recipe")] recipe: JsValue,
) -> Result<String, JsError> {
    Ok(api::recipe_to_json(&from_js(recipe)?))
}

/// Parameters to render when applying `recipe` over an image graded with `current`.
#[wasm_bindgen(js_name = recipeApply, unchecked_return_type = "GradeParameters")]
pub fn recipe_apply(
    #[wasm_bindgen(unchecked_param_type = "Recipe")] recipe: JsValue,
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] current: JsValue,
) -> Result<JsValue, JsError> {
    to_js(&api::recipe_apply(&from_js(recipe)?, &from_js(current)?))
}

/// Rename a recipe.
#[wasm_bindgen(js_name = recipeRename, unchecked_return_type = "Recipe")]
pub fn recipe_rename(
    #[wasm_bindgen(unchecked_param_type = "Recipe")] recipe: JsValue,
    name: &str,
    now_ms: f64,
) -> Result<JsValue, JsError> {
    to_js(&api::recipe_rename(&from_js(recipe)?, name, now_ms as i64)?)
}

/// Title and size of `.cube` text; throws if it does not parse.
#[wasm_bindgen(js_name = lutParse, unchecked_return_type = "LutInfo")]
pub fn lut_parse(text: &str) -> Result<JsValue, JsError> {
    to_js(&api::lut_parse(text)?)
}

/// Bake a grade into `.cube` text (`size` points per axis).
#[wasm_bindgen(js_name = lutBake)]
pub fn lut_bake(
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] params: JsValue,
    size: u32,
    #[wasm_bindgen(unchecked_param_type = "BakeScope")] scope: JsValue,
    title: Option<String>,
) -> Result<String, JsError> {
    let scope: BakeScope = from_js(scope)?;
    Ok(api::lut_bake(&from_js(params)?, size, scope, title))
}

/// Stage 1 auto-analysis of an sRGB RGBA8 image.
#[wasm_bindgen(js_name = analyzeStage1, unchecked_return_type = "Stage1Analysis")]
pub fn analyze_stage1(
    rgba: &[u8],
    w: u32,
    h: u32,
    #[wasm_bindgen(unchecked_param_type = "CorrectionParameters")] stage1_params: JsValue,
) -> Result<JsValue, JsError> {
    to_js(&api::analyze_stage1(rgba, w, h, &from_js(stage1_params)?)?)
}

/// CPU render of an sRGB RGBA8 image through the full pipeline.
#[wasm_bindgen(js_name = renderCpuRgba)]
pub fn render_cpu_rgba(
    rgba: &[u8],
    w: u32,
    h: u32,
    #[wasm_bindgen(unchecked_param_type = "GradeParameters")] params: JsValue,
    lut_text: Option<String>,
    full_width: f64,
) -> Result<Vec<u8>, JsError> {
    Ok(api::render_cpu_rgba(
        rgba,
        w,
        h,
        &from_js(params)?,
        lut_text.as_deref(),
        full_width,
    )?)
}

/// Vectorscope overlay geometry.
#[wasm_bindgen(unchecked_return_type = "Graticule")]
pub fn graticule() -> Result<JsValue, JsError> {
    to_js(&api::graticule())
}
