//! Explainability: turn a `(before, after)` pair of grades into an ordered
//! list of human-readable parameter changes with one-sentence rationales.
//!
//! Rationales are deterministic templates keyed on the parameter path, filled
//! from the parameter spec's `describes` phrase, the change direction, its
//! signed amount with unit, and the prompt terms that caused it. No text is
//! model-generated (ADR-0003, ADR-0006).

use apricity_core::curve;
use apricity_core::grade_parameters::{CurvePoint, GradeParameters};
use apricity_core::param_spec::{
    all_scalar_specs, curve as curve_of, curve_label, scalar_spec, ScalarSpec, CURVE_PATHS,
};
use serde::{Deserialize, Serialize};

use crate::Attribution;

/// One changed parameter, ready for the Explainability Panel.
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterChange {
    /// Dotted parameter path (scalar or tone-curve path).
    pub path: String,
    /// Manual Console label.
    pub label: String,
    /// Manual Console group heading.
    pub group: String,
    /// `"correction"` (Stage 1) or `"creative"` (Stage 2).
    pub stage: String,
    /// Unit name, snake_case (`"percent"`, `"mireds"`, ...); `"curve"` for
    /// tone curves.
    pub unit: String,
    /// Display suffix for the unit (`"%"`, `" mired"`, ...).
    pub unit_suffix: String,
    /// Value before, in the parameter's own unit. `0.0` for curves.
    pub before: f32,
    /// Value after, in the parameter's own unit. `0.0` for curves.
    pub after: f32,
    /// `after - before`, in the parameter's own unit. `0.0` for curves.
    pub delta: f32,
    /// `before` formatted with unit suffix, or a curve description.
    pub before_text: String,
    /// `after` formatted with unit suffix, or a curve description.
    pub after_text: String,
    /// One-sentence reason.
    pub rationale: String,
    /// Manual Console control to scroll to; equal to `path`.
    pub console_anchor: String,
}

const CURVE_TEMPLATE: &str = "Reshaped the {what} ({shape})";

/// The rationale template for `path`: `"UP|DOWN"` sentence stems using the
/// placeholders `{what}` (the spec's `describes` noun phrase) and, for curves,
/// `{shape}`. A template without `|` serves both directions. Returns `None`
/// only for paths that are not a scalar spec or tone curve; every NGA-emittable
/// path has one.
pub fn rationale_template(path: &str) -> Option<&'static str> {
    if CURVE_PATHS.contains(&path) {
        return Some(CURVE_TEMPLATE);
    }
    scalar_spec(path)?;
    Some(match path {
        "stage1.temperature" | "stage2.temperature" => "Warmed the {what}|Cooled the {what}",
        "stage1.tint" | "stage2.tint" => {
            "Shifted the {what} toward magenta|Shifted the {what} toward green"
        }
        "stage2.split_tone.balance" => {
            "Shifted the {what} toward the shadows|Shifted the {what} toward the highlights"
        }
        "stage2.vignette.amount" => {
            "Lightened the frame edges with {what}|Darkened the frame edges with {what}"
        }
        "stage2.grain.amount" => "Added {what}|Reduced {what}",
        p if p.ends_with(".hue") || p.ends_with("_hue") => "Shifted {what}|Shifted {what}",
        _ => "Raised {what}|Lowered {what}",
    })
}

fn decimals(step: f32) -> usize {
    let mut d = 0;
    while d < 6 && step * 10f32.powi(d as i32) < 0.999 {
        d += 1;
    }
    d
}

fn format_abs(v: f32, step: f32) -> String {
    let d = decimals(step);
    let mut s = format!("{:.*}", d, v.abs());
    let trim = d > 0 && (step * 10f32.powi(d as i32) - 1.0).abs() > 1e-3;
    if trim && s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_owned();
    }
    s
}

fn signed(v: f32, step: f32) -> String {
    let body = format_abs(v, step);
    if body.chars().all(|c| c == '0' || c == '.') {
        body
    } else if v < 0.0 {
        format!("\u{2212}{body}")
    } else {
        format!("+{body}")
    }
}

fn value_text(spec: &ScalarSpec, v: f32) -> String {
    let n = if spec.min < 0.0 {
        signed(v, spec.step)
    } else {
        format_abs(v, spec.step)
    };
    format!("{n}{}", spec.unit.suffix())
}

fn unit_name(spec: &ScalarSpec) -> String {
    serde_json::to_value(spec.unit)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn terms_for(attributions: &[Attribution], path: &str, delta: f32) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    for a in attributions {
        if a.path == path && a.delta * delta > 0.0 && !terms.contains(&a.term) {
            terms.push(a.term.clone());
        }
    }
    if terms.is_empty() {
        for a in attributions.iter().filter(|a| a.path == path) {
            if !terms.contains(&a.term) {
                terms.push(a.term.clone());
            }
        }
    }
    terms.truncate(3);
    terms
}

fn reset_sentence(what: &str, amount: Option<&str>, with_reason: bool) -> String {
    let mut s = match amount {
        Some(a) => format!("Reset {what} to neutral ({a})"),
        None => format!("Reset the {what} to linear"),
    };
    if with_reason {
        s.push_str(" for the new look");
    }
    s.push('.');
    s
}

fn render(
    template: &str,
    up: bool,
    what: &str,
    shape: &str,
    amount: Option<&str>,
    terms: &[String],
) -> String {
    let stem = match template.split_once('|') {
        Some((u, d)) => {
            if up {
                u
            } else {
                d
            }
        }
        None => template,
    };
    let mut s = stem.replace("{what}", what).replace("{shape}", shape);
    if let Some(a) = amount {
        s.push_str(&format!(" ({a})"));
    }
    if !terms.is_empty() {
        let quoted: Vec<String> = terms.iter().map(|t| format!("'{t}'")).collect();
        s.push_str(&format!(" for {}", quoted.join(", ")));
    }
    s.push('.');
    s
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn curves_differ(a: &[CurvePoint], b: &[CurvePoint]) -> bool {
    let sample = |c: &[CurvePoint], x: f32| {
        if c.is_empty() {
            x
        } else {
            curve::evaluate(&curve::sanitize(c, 16), x)
        }
    };
    (0..=16).any(|i| {
        let x = i as f32 / 16.0;
        !near(sample(a, x), sample(b, x))
    })
}

/// Classify a tone curve for display. Returns `(Capitalized, inline)` names.
fn curve_shape(points: &[CurvePoint]) -> (&'static str, &'static str) {
    if points.is_empty() {
        return ("Linear", "linear");
    }
    let c = curve::sanitize(points, 16);
    let y = |x: f32| curve::evaluate(&c, x);
    if (0..=16).all(|i| near(y(i as f32 / 16.0), i as f32 / 16.0)) {
        return ("Linear", "linear");
    }
    let t = 0.01;
    if y(0.0) > 0.02 {
        ("Faded blacks", "faded blacks")
    } else if y(0.25) < 0.25 - t && y(0.75) > 0.75 + t {
        ("S-curve", "S-curve")
    } else if y(0.25) > 0.25 + t && y(0.75) < 0.75 - t {
        ("Soft curve", "soft curve")
    } else if y(0.125) < 0.085 && y(0.75) > 0.73 {
        ("Crushed blacks", "crushed blacks")
    } else if y(0.5) > 0.5 + t {
        ("Brightening curve", "brightening curve")
    } else if y(0.5) < 0.5 - t {
        ("Darkening curve", "darkening curve")
    } else {
        ("Custom curve", "custom curve")
    }
}

fn curve_text(points: &[CurvePoint]) -> String {
    let (name, _) = curve_shape(points);
    if name == "Linear" {
        "Linear".to_owned()
    } else {
        format!("{name} ({} points)", curve::sanitize(points, 16).len())
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Every parameter that differs between `before` and `after`.
///
/// Scalars are reported when they moved by more than half their spec step;
/// tone curves when any sampled output moved by more than `1e-4` (working
/// encoding). Order is the spec table order with curves last. Works for any
/// two grades: pass empty `attributions` for manual edits or Recipe diffs and
/// the rationale omits the "for 'term'" clause.
pub fn diff(
    before: &GradeParameters,
    after: &GradeParameters,
    attributions: &[Attribution],
) -> Vec<ParameterChange> {
    let mut out = Vec::new();
    for spec in all_scalar_specs() {
        let (b, a) = (spec.get(before), spec.get(after));
        let delta = a - b;
        if delta.is_nan() || delta.abs() <= spec.step * 0.5 {
            continue;
        }
        let template = rationale_template(spec.path).unwrap_or("Raised {what}|Lowered {what}");
        let amount = format!("{}{}", signed(delta, spec.step), spec.unit.suffix());
        let terms = terms_for(attributions, spec.path, delta);
        out.push(ParameterChange {
            path: spec.path.to_owned(),
            label: spec.label.to_owned(),
            group: spec.group.to_owned(),
            stage: match spec.stage {
                apricity_core::param_spec::Stage::Correction => "correction",
                apricity_core::param_spec::Stage::Creative => "creative",
            }
            .to_owned(),
            unit: unit_name(spec),
            unit_suffix: spec.unit.suffix().to_owned(),
            before: b,
            after: a,
            delta,
            before_text: value_text(spec, b),
            after_text: value_text(spec, a),
            rationale: if terms.is_empty() && (a - spec.default).abs() <= spec.step * 0.5 {
                reset_sentence(spec.describes, Some(&amount), !attributions.is_empty())
            } else {
                render(
                    template,
                    delta > 0.0,
                    spec.describes,
                    "",
                    Some(&amount),
                    &terms,
                )
            },
            console_anchor: spec.path.to_owned(),
        });
    }
    for path in CURVE_PATHS {
        let (Some(b), Some(a)) = (curve_of(before, path), curve_of(after, path)) else {
            continue;
        };
        if !curves_differ(b, a) {
            continue;
        }
        let terms: Vec<String> = {
            let mut t: Vec<String> = Vec::new();
            for at in attributions.iter().filter(|x| x.path == path) {
                if !t.contains(&at.term) {
                    t.push(at.term.clone());
                }
            }
            t.truncate(3);
            t
        };
        let what = curve_label(path);
        out.push(ParameterChange {
            path: path.to_owned(),
            label: capitalize(what),
            group: "Curves".to_owned(),
            stage: "creative".to_owned(),
            unit: "curve".to_owned(),
            unit_suffix: String::new(),
            before: 0.0,
            after: 0.0,
            delta: 0.0,
            before_text: curve_text(b),
            after_text: curve_text(a),
            rationale: if terms.is_empty() && curve_shape(a).0 == "Linear" {
                reset_sentence(what, None, !attributions.is_empty())
            } else {
                render(CURVE_TEMPLATE, true, what, curve_shape(a).1, None, &terms)
            },
            console_anchor: path.to_owned(),
        });
    }
    out
}

/// One-line summary for the Explainability Panel header, e.g.
/// `"The AI made 6 adjustments."`.
pub fn summary(changes: &[ParameterChange]) -> String {
    match changes.len() {
        0 => "No adjustments were needed.".to_owned(),
        1 => "The AI made 1 adjustment.".to_owned(),
        n => format!("The AI made {n} adjustments."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formatting_follows_step() {
        assert_eq!(signed(0.3, 0.01), "+0.30");
        assert_eq!(signed(12.0, 0.5), "+12");
        assert_eq!(signed(-35.0, 1.0), "\u{2212}35");
        assert_eq!(signed(0.03, 0.001), "+0.030");
        assert_eq!(format_abs(12.5, 0.5), "12.5");
    }

    #[test]
    fn summary_wording() {
        assert_eq!(summary(&[]), "No adjustments were needed.");
    }

    #[test]
    fn manual_edit_rationale_has_no_quotes() {
        let mut after = GradeParameters::IDENTITY;
        after.stage2.contrast = 20.0;
        let ch = diff(&GradeParameters::IDENTITY, &after, &[]);
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].rationale, "Raised contrast (+20%).");
        assert_eq!(ch[0].after_text, "+20%");
    }

    #[test]
    fn terms_are_quoted_and_signed() {
        let mut after = GradeParameters::IDENTITY;
        after.stage2.saturation = -35.0;
        let attrs = vec![
            Attribution {
                term: "noir".into(),
                path: "stage2.saturation".into(),
                delta: -35.0,
            },
            Attribution {
                term: "desaturated".into(),
                path: "stage2.saturation".into(),
                delta: -10.0,
            },
        ];
        let ch = diff(&GradeParameters::IDENTITY, &after, &attrs);
        assert_eq!(
            ch[0].rationale,
            "Lowered overall saturation (\u{2212}35%) for 'noir', 'desaturated'."
        );
    }
}
