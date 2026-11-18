//! The parameter specification table: label, unit, range, default, and
//! console group for every scalar in [`GradeParameters`].
//!
//! This table is the **only** place ranges live. The Manual Console builds
//! its sliders from it (via FFI), the NGA clamps its output against it, and
//! the Explainability differ phrases its rationale from it. A test asserts
//! every spec's `default` equals the field's value in
//! [`GradeParameters::IDENTITY`].
//!
//! Paths are dotted field paths, e.g. `stage2.lift.r`, and are stable across
//! the FFI boundary and in Recipe files.

use serde::Serialize;

use crate::grade_parameters::{CurvePoint, GradeParameters, HslBand};

/// Unit of a scalar parameter. Shown in the console and Explainability
/// Panel; never implied.
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    /// Photographic stops (log2 multiplier).
    Stops,
    /// Mireds relative to D65 (positive = warmer).
    Mireds,
    /// Tint units (positive = magenta).
    Tint,
    /// Percent slider units.
    Percent,
    /// Hue degrees.
    Degrees,
    /// Normalized offset in the working encoding.
    Normalized,
    /// Scene-linear Rec.2020 value.
    Linear,
    /// Source pixels.
    Pixels,
}

impl Unit {
    /// Short suffix for display (`"EV"`, `"%"`, …).
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::Stops => " EV",
            Unit::Mireds => " mired",
            Unit::Tint => "",
            Unit::Percent => "%",
            Unit::Degrees => "°",
            Unit::Normalized => "",
            Unit::Linear => "",
            Unit::Pixels => " px",
        }
    }
}

/// Which pipeline stage a parameter belongs to.
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Stage 1 — technical correction.
    Correction,
    /// Stage 2 — creative grade.
    Creative,
}

/// Specification of one scalar parameter.
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[derive(Clone, Copy, Serialize)]
pub struct ScalarSpec {
    /// Dotted field path, e.g. `stage2.contrast`.
    pub path: &'static str,
    /// Console label.
    pub label: &'static str,
    /// Console group (section heading).
    pub group: &'static str,
    /// Unit.
    pub unit: Unit,
    /// Stage.
    pub stage: Stage,
    /// Minimum allowed value (inclusive).
    pub min: f32,
    /// Maximum allowed value (inclusive).
    pub max: f32,
    /// Identity / reset value.
    pub default: f32,
    /// Slider step.
    pub step: f32,
    /// Noun phrase naming what this parameter changes, used by the
    /// Explainability templates (e.g. "overall brightness").
    pub describes: &'static str,
    /// Whether the NGA may emit this parameter.
    pub nga: bool,
    #[serde(skip)]
    get: fn(&GradeParameters) -> f32,
    #[serde(skip)]
    set: fn(&mut GradeParameters, f32),
}

impl std::fmt::Debug for ScalarSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalarSpec")
            .field("path", &self.path)
            .finish()
    }
}

impl ScalarSpec {
    /// Read this parameter from `p`.
    #[inline]
    pub fn get(&self, p: &GradeParameters) -> f32 {
        (self.get)(p)
    }

    /// Write `v` (unclamped) into `p`.
    #[inline]
    pub fn set(&self, p: &mut GradeParameters, v: f32) {
        (self.set)(p, v)
    }

    /// Clamp `v` to this spec's range; NaN becomes the default.
    #[inline]
    pub fn clamp(&self, v: f32) -> f32 {
        if v.is_nan() {
            self.default
        } else {
            v.clamp(self.min, self.max)
        }
    }
}

macro_rules! scalar_specs {
    ($(
        $path:literal => [$($field:ident).+] {
            $label:literal, $group:literal, $unit:ident, $stage:ident,
            $min:expr, $max:expr, $default:expr, $step:expr, $nga:expr, $describes:literal
        }
    ),* $(,)?) => {
        /// Every scalar parameter, in console order.
        pub static SCALAR_SPECS: &[ScalarSpec] = &[
            $(ScalarSpec {
                path: $path,
                label: $label,
                group: $group,
                unit: Unit::$unit,
                stage: Stage::$stage,
                min: $min,
                max: $max,
                default: $default,
                step: $step,
                describes: $describes,
                nga: $nga,
                get: |p| p$(.$field)+,
                set: |p, v| p$(.$field)+ = v,
            }),*
        ];
    };
}

macro_rules! hsl_specs {
    ($($band:ident),*) => {
        /// HSL band specs: hue, saturation, luminance per band.
        pub static HSL_SPECS: &[ScalarSpec] = &[
            $(
                ScalarSpec {
                    path: concat!("stage2.hsl.", stringify!($band), ".hue"),
                    label: concat!(stringify!($band), " hue"),
                    group: "HSL",
                    unit: Unit::Degrees,
                    stage: Stage::Creative,
                    min: -30.0, max: 30.0, default: 0.0, step: 0.5,
                    describes: concat!("the hue of ", stringify!($band), " tones"),
                    nga: true,
                    get: |p| p.stage2.hsl.$band.hue,
                    set: |p, v| p.stage2.hsl.$band.hue = v,
                },
                ScalarSpec {
                    path: concat!("stage2.hsl.", stringify!($band), ".saturation"),
                    label: concat!(stringify!($band), " saturation"),
                    group: "HSL",
                    unit: Unit::Percent,
                    stage: Stage::Creative,
                    min: -100.0, max: 100.0, default: 0.0, step: 1.0,
                    describes: concat!("the saturation of ", stringify!($band), " tones"),
                    nga: true,
                    get: |p| p.stage2.hsl.$band.saturation,
                    set: |p, v| p.stage2.hsl.$band.saturation = v,
                },
                ScalarSpec {
                    path: concat!("stage2.hsl.", stringify!($band), ".luminance"),
                    label: concat!(stringify!($band), " luminance"),
                    group: "HSL",
                    unit: Unit::Percent,
                    stage: Stage::Creative,
                    min: -100.0, max: 100.0, default: 0.0, step: 1.0,
                    describes: concat!("the brightness of ", stringify!($band), " tones"),
                    nga: true,
                    get: |p| p.stage2.hsl.$band.luminance,
                    set: |p, v| p.stage2.hsl.$band.luminance = v,
                },
            )*
        ];
    };
}

scalar_specs! {
    // ---- Stage 1: technical correction ----
    "stage1.exposure" => [stage1.exposure] {
        "Exposure", "Correction", Stops, Correction, -5.0, 5.0, 0.0, 0.01, false,
        "baseline exposure"
    },
    "stage1.temperature" => [stage1.temperature] {
        "White balance", "Correction", Mireds, Correction, -250.0, 250.0, 0.0, 0.5, false,
        "white balance temperature"
    },
    "stage1.tint" => [stage1.tint] {
        "WB tint", "Correction", Tint, Correction, -100.0, 100.0, 0.0, 0.5, false,
        "white balance tint"
    },
    "stage1.black_point.r" => [stage1.black_point.r] {
        "Black point R", "Correction", Linear, Correction, 0.0, 0.05, 0.0, 0.0005, false,
        "red black point"
    },
    "stage1.black_point.g" => [stage1.black_point.g] {
        "Black point G", "Correction", Linear, Correction, 0.0, 0.05, 0.0, 0.0005, false,
        "green black point"
    },
    "stage1.black_point.b" => [stage1.black_point.b] {
        "Black point B", "Correction", Linear, Correction, 0.0, 0.05, 0.0, 0.0005, false,
        "blue black point"
    },
    "stage1.exposure_target" => [stage1.exposure_target] {
        "Exposure target", "Correction", Normalized, Correction, 0.2, 0.7, 0.45, 0.01, false,
        "auto-exposure target"
    },

    // ---- Stage 2: primaries ----
    "stage2.exposure" => [stage2.exposure] {
        "Exposure", "Light", Stops, Creative, -4.0, 4.0, 0.0, 0.01, true,
        "overall brightness"
    },
    "stage2.contrast" => [stage2.contrast] {
        "Contrast", "Light", Percent, Creative, -100.0, 100.0, 0.0, 1.0, true,
        "contrast"
    },
    "stage2.highlights" => [stage2.highlights] {
        "Highlights", "Light", Percent, Creative, -100.0, 100.0, 0.0, 1.0, true,
        "highlight brightness"
    },
    "stage2.shadows" => [stage2.shadows] {
        "Shadows", "Light", Percent, Creative, -100.0, 100.0, 0.0, 1.0, true,
        "shadow brightness"
    },
    "stage2.whites" => [stage2.whites] {
        "Whites", "Light", Percent, Creative, -100.0, 100.0, 0.0, 1.0, true,
        "white level"
    },
    "stage2.blacks" => [stage2.blacks] {
        "Blacks", "Light", Percent, Creative, -100.0, 100.0, 0.0, 1.0, true,
        "black level"
    },
    "stage2.temperature" => [stage2.temperature] {
        "Temperature", "Color", Mireds, Creative, -100.0, 100.0, 0.0, 0.5, true,
        "color temperature"
    },
    "stage2.tint" => [stage2.tint] {
        "Tint", "Color", Tint, Creative, -100.0, 100.0, 0.0, 0.5, true,
        "green–magenta tint"
    },
    "stage2.vibrance" => [stage2.vibrance] {
        "Vibrance", "Color", Percent, Creative, -100.0, 100.0, 0.0, 1.0, true,
        "vibrance of muted colors"
    },
    "stage2.saturation" => [stage2.saturation] {
        "Saturation", "Color", Percent, Creative, -100.0, 100.0, 0.0, 1.0, true,
        "overall saturation"
    },

    // ---- Stage 2: wheels ----
    "stage2.lift.r" => [stage2.lift.r] {
        "Lift R", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "red in the shadows"
    },
    "stage2.lift.g" => [stage2.lift.g] {
        "Lift G", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "green in the shadows"
    },
    "stage2.lift.b" => [stage2.lift.b] {
        "Lift B", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "blue in the shadows"
    },
    "stage2.lift.master" => [stage2.lift.master] {
        "Lift", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "shadow level (lift)"
    },
    "stage2.gamma.r" => [stage2.gamma.r] {
        "Gamma R", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "red in the midtones"
    },
    "stage2.gamma.g" => [stage2.gamma.g] {
        "Gamma G", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "green in the midtones"
    },
    "stage2.gamma.b" => [stage2.gamma.b] {
        "Gamma B", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "blue in the midtones"
    },
    "stage2.gamma.master" => [stage2.gamma.master] {
        "Gamma", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "midtone level (gamma)"
    },
    "stage2.gain.r" => [stage2.gain.r] {
        "Gain R", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "red in the highlights"
    },
    "stage2.gain.g" => [stage2.gain.g] {
        "Gain G", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "green in the highlights"
    },
    "stage2.gain.b" => [stage2.gain.b] {
        "Gain B", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "blue in the highlights"
    },
    "stage2.gain.master" => [stage2.gain.master] {
        "Gain", "Wheels", Normalized, Creative, -0.5, 0.5, 0.0, 0.001, true, "highlight level (gain)"
    },

    // ---- Stage 2: split tone ----
    "stage2.split_tone.shadow_hue" => [stage2.split_tone.shadow_hue] {
        "Shadow hue", "Split Tone", Degrees, Creative, 0.0, 360.0, 0.0, 1.0, true, "shadow tint hue"
    },
    "stage2.split_tone.shadow_saturation" => [stage2.split_tone.shadow_saturation] {
        "Shadow saturation", "Split Tone", Percent, Creative, 0.0, 100.0, 0.0, 1.0, true,
        "shadow tint strength"
    },
    "stage2.split_tone.highlight_hue" => [stage2.split_tone.highlight_hue] {
        "Highlight hue", "Split Tone", Degrees, Creative, 0.0, 360.0, 0.0, 1.0, true,
        "highlight tint hue"
    },
    "stage2.split_tone.highlight_saturation" => [stage2.split_tone.highlight_saturation] {
        "Highlight saturation", "Split Tone", Percent, Creative, 0.0, 100.0, 0.0, 1.0, true,
        "highlight tint strength"
    },
    "stage2.split_tone.balance" => [stage2.split_tone.balance] {
        "Balance", "Split Tone", Percent, Creative, -100.0, 100.0, 0.0, 1.0, true,
        "split-tone balance"
    },

    // ---- Stage 2: LUT ----
    "stage2.lut_strength" => [stage2.lut_strength] {
        "LUT strength", "LUT", Percent, Creative, 0.0, 100.0, 100.0, 1.0, false, "LUT strength"
    },

    // ---- Stage 2: effects ----
    "stage2.vignette.amount" => [stage2.vignette.amount] {
        "Vignette", "Effects", Percent, Creative, -100.0, 100.0, 0.0, 1.0, true, "vignette strength"
    },
    "stage2.vignette.midpoint" => [stage2.vignette.midpoint] {
        "Vignette size", "Effects", Percent, Creative, 0.0, 100.0, 50.0, 1.0, true, "vignette size"
    },
    "stage2.vignette.feather" => [stage2.vignette.feather] {
        "Vignette feather", "Effects", Percent, Creative, 0.0, 100.0, 50.0, 1.0, true,
        "vignette softness"
    },
    "stage2.vignette.roundness" => [stage2.vignette.roundness] {
        "Vignette roundness", "Effects", Percent, Creative, -100.0, 100.0, 0.0, 1.0, false,
        "vignette shape"
    },
    "stage2.vignette.center_x" => [stage2.vignette.center_x] {
        "Vignette center X", "Effects", Percent, Creative, -100.0, 100.0, 0.0, 1.0, false,
        "vignette horizontal center"
    },
    "stage2.vignette.center_y" => [stage2.vignette.center_y] {
        "Vignette center Y", "Effects", Percent, Creative, -100.0, 100.0, 0.0, 1.0, false,
        "vignette vertical center"
    },
    "stage2.grain.amount" => [stage2.grain.amount] {
        "Grain", "Effects", Percent, Creative, 0.0, 100.0, 0.0, 1.0, true, "film grain"
    },
    "stage2.grain.size" => [stage2.grain.size] {
        "Grain size", "Effects", Pixels, Creative, 0.5, 4.0, 1.0, 0.05, true, "grain size"
    },
    "stage2.grain.roughness" => [stage2.grain.roughness] {
        "Grain roughness", "Effects", Percent, Creative, 0.0, 100.0, 50.0, 1.0, true,
        "grain roughness"
    },
}

hsl_specs!(red, orange, yellow, green, aqua, blue, purple, magenta);

/// Iterate every scalar spec (primaries, wheels, split tone, effects, HSL).
pub fn all_scalar_specs() -> impl Iterator<Item = &'static ScalarSpec> {
    SCALAR_SPECS.iter().chain(HSL_SPECS.iter())
}

/// Look up a scalar spec by dotted path.
pub fn scalar_spec(path: &str) -> Option<&'static ScalarSpec> {
    all_scalar_specs().find(|s| s.path == path)
}

/// Read a scalar by path.
pub fn get_scalar(p: &GradeParameters, path: &str) -> Option<f32> {
    scalar_spec(path).map(|s| s.get(p))
}

/// Write a scalar by path, clamped to its range. Returns `false` if the
/// path is unknown.
pub fn set_scalar(p: &mut GradeParameters, path: &str, v: f32) -> bool {
    match scalar_spec(path) {
        Some(s) => {
            s.set(p, s.clamp(v));
            true
        }
        None => false,
    }
}

/// Paths of the four tone curves (non-scalar parameters).
pub const CURVE_PATHS: [&str; 4] = [
    "stage2.tone_curve.master",
    "stage2.tone_curve.red",
    "stage2.tone_curve.green",
    "stage2.tone_curve.blue",
];

/// Borrow a tone curve by path.
pub fn curve<'a>(p: &'a GradeParameters, path: &str) -> Option<&'a Vec<CurvePoint>> {
    let c = &p.stage2.tone_curve;
    match path {
        "stage2.tone_curve.master" => Some(&c.master),
        "stage2.tone_curve.red" => Some(&c.red),
        "stage2.tone_curve.green" => Some(&c.green),
        "stage2.tone_curve.blue" => Some(&c.blue),
        _ => None,
    }
}

/// Mutably borrow a tone curve by path.
pub fn curve_mut<'a>(p: &'a mut GradeParameters, path: &str) -> Option<&'a mut Vec<CurvePoint>> {
    let c = &mut p.stage2.tone_curve;
    match path {
        "stage2.tone_curve.master" => Some(&mut c.master),
        "stage2.tone_curve.red" => Some(&mut c.red),
        "stage2.tone_curve.green" => Some(&mut c.green),
        "stage2.tone_curve.blue" => Some(&mut c.blue),
        _ => None,
    }
}

/// Human label for a curve path.
pub fn curve_label(path: &str) -> &'static str {
    match path {
        "stage2.tone_curve.master" => "master tone curve",
        "stage2.tone_curve.red" => "red tone curve",
        "stage2.tone_curve.green" => "green tone curve",
        "stage2.tone_curve.blue" => "blue tone curve",
        _ => "tone curve",
    }
}

#[allow(dead_code)]
fn _assert_hsl_band_is_three_scalars(b: HslBand) -> [f32; 3] {
    // If HslBand gains a field, HSL_SPECS must gain a matching entry.
    let HslBand {
        hue,
        saturation,
        luminance,
    } = b;
    [hue, saturation, luminance]
}

/// Serializable view of the whole table for the FFI.
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[derive(Debug, Serialize)]
pub struct SpecTable {
    /// Scalars in console order.
    pub scalars: Vec<ScalarSpec>,
    /// Tone-curve paths.
    pub curves: Vec<&'static str>,
    /// HSL band names in order.
    pub hsl_bands: Vec<&'static str>,
    /// HSL band centers in degrees.
    pub hsl_centers_deg: Vec<f32>,
}

/// Build the full spec table.
pub fn spec_table() -> SpecTable {
    SpecTable {
        scalars: all_scalar_specs().copied().collect(),
        curves: CURVE_PATHS.to_vec(),
        hsl_bands: crate::grade_parameters::HslBands::NAMES.to_vec(),
        hsl_centers_deg: crate::grade_parameters::HslBands::CENTERS_DEG.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn defaults_match_identity() {
        let id = GradeParameters::IDENTITY;
        for s in all_scalar_specs() {
            assert_eq!(s.get(&id), s.default, "default mismatch for {}", s.path);
            assert!(s.min <= s.default && s.default <= s.max, "{}", s.path);
            assert!(s.step > 0.0, "{}", s.path);
        }
    }

    #[test]
    fn paths_are_unique() {
        let mut seen = HashSet::new();
        for s in all_scalar_specs() {
            assert!(seen.insert(s.path), "duplicate path {}", s.path);
        }
    }

    #[test]
    fn get_set_round_trip_every_path() {
        for s in all_scalar_specs() {
            let mut p = GradeParameters::IDENTITY;
            let v = (s.min + s.max) * 0.5 + s.step;
            assert!(set_scalar(&mut p, s.path, v));
            assert_eq!(get_scalar(&p, s.path), Some(s.clamp(v)), "{}", s.path);
            // Only this field changed.
            for other in all_scalar_specs().filter(|o| o.path != s.path) {
                assert_eq!(
                    other.get(&p),
                    other.default,
                    "{} leaked into {}",
                    s.path,
                    other.path
                );
            }
        }
    }

    #[test]
    fn json_path_matches_serde_field_path() {
        // Every dotted path must resolve in the serialized JSON, so the
        // Recipe format and the FFI agree on names.
        let json = serde_json::to_value(GradeParameters::IDENTITY).unwrap();
        for s in all_scalar_specs() {
            let mut cur = &json;
            for part in s.path.split('.') {
                cur = cur
                    .get(part)
                    .unwrap_or_else(|| panic!("{} missing segment {part}", s.path));
            }
            assert!(cur.is_number(), "{} is not a number in JSON", s.path);
        }
    }

    #[test]
    fn clamp_handles_nan_and_range() {
        let s = scalar_spec("stage2.contrast").unwrap();
        assert_eq!(s.clamp(f32::NAN), 0.0);
        assert_eq!(s.clamp(500.0), 100.0);
        assert_eq!(s.clamp(-500.0), -100.0);
    }

    #[test]
    fn unknown_path_rejected() {
        let mut p = GradeParameters::IDENTITY;
        assert!(!set_scalar(&mut p, "stage2.nope", 1.0));
        assert!(get_scalar(&p, "stage9").is_none());
    }
}
