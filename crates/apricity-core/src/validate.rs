//! Parameter validation and sanitization.
//!
//! [`validate`] reports every problem without changing anything;
//! [`sanitize`] returns a copy that is guaranteed renderable (all scalars
//! finite and in range, curves sorted and bounded). Every value crossing
//! the FFI or loaded from a Recipe passes through [`sanitize`].

use serde::{Deserialize, Serialize};

use crate::curve;
use crate::grade_parameters::{GradeParameters, ToneCurve, SCHEMA_VERSION};
use crate::param_spec::{all_scalar_specs, curve, CURVE_PATHS};

/// One validation problem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Dotted path of the offending field.
    pub path: String,
    /// Human-readable description.
    pub message: String,
}

/// Report every out-of-range, non-finite, or malformed value.
pub fn validate(p: &GradeParameters) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if p.schema_version > SCHEMA_VERSION {
        issues.push(ValidationIssue {
            path: "schema_version".into(),
            message: format!(
                "schema version {} is newer than supported {}",
                p.schema_version, SCHEMA_VERSION
            ),
        });
    }
    for s in all_scalar_specs() {
        let v = s.get(p);
        if !v.is_finite() {
            issues.push(ValidationIssue {
                path: s.path.into(),
                message: "value is not finite".into(),
            });
        } else if v < s.min || v > s.max {
            issues.push(ValidationIssue {
                path: s.path.into(),
                message: format!("{v} outside [{}, {}]", s.min, s.max),
            });
        }
    }
    for path in CURVE_PATHS {
        let pts = curve(p, path).expect("known curve path");
        if pts.len() == 1 || pts.len() > ToneCurve::MAX_POINTS {
            issues.push(ValidationIssue {
                path: path.into(),
                message: format!(
                    "curve has {} points (need 0 or 2..={})",
                    pts.len(),
                    ToneCurve::MAX_POINTS
                ),
            });
        }
        if pts
            .iter()
            .any(|q| !(0.0..=1.0).contains(&q.x) || !(0.0..=1.0).contains(&q.y))
        {
            issues.push(ValidationIssue {
                path: path.into(),
                message: "curve point outside [0, 1]".into(),
            });
        }
        if pts.windows(2).any(|w| w[1].x <= w[0].x) {
            issues.push(ValidationIssue {
                path: path.into(),
                message: "curve points not strictly increasing in x".into(),
            });
        }
    }
    issues
}

/// Return a renderable copy: scalars clamped (NaN → default), curves
/// sanitized, identity curves normalized to empty, schema version current.
pub fn sanitize(p: &GradeParameters) -> GradeParameters {
    let mut out = p.clone();
    out.schema_version = SCHEMA_VERSION;
    for s in all_scalar_specs() {
        let v = s.clamp(s.get(&out));
        s.set(&mut out, v);
    }
    let tc = &mut out.stage2.tone_curve;
    for c in [&mut tc.master, &mut tc.red, &mut tc.green, &mut tc.blue] {
        if c.is_empty() {
            continue;
        }
        let clean = curve::sanitize(c, ToneCurve::MAX_POINTS);
        *c = if clean.len() == 2 && clean.iter().all(|q| (q.x - q.y).abs() < 1e-6) {
            Vec::new()
        } else {
            clean
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grade_parameters::CurvePoint;

    #[test]
    fn identity_is_valid() {
        assert!(validate(&GradeParameters::IDENTITY).is_empty());
        assert_eq!(
            sanitize(&GradeParameters::IDENTITY),
            GradeParameters::IDENTITY
        );
    }

    #[test]
    fn detects_and_fixes_out_of_range() {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.contrast = 400.0;
        p.stage2.saturation = f32::NAN;
        p.stage2.tone_curve.master = vec![CurvePoint::new(0.9, 0.1), CurvePoint::new(0.1, 0.9)];
        let issues = validate(&p);
        assert!(issues.iter().any(|i| i.path == "stage2.contrast"));
        assert!(issues.iter().any(|i| i.path == "stage2.saturation"));
        assert!(issues.iter().any(|i| i.path == "stage2.tone_curve.master"));
        let s = sanitize(&p);
        assert!(validate(&s).is_empty(), "{:?}", validate(&s));
        assert_eq!(s.stage2.contrast, 100.0);
        assert_eq!(s.stage2.saturation, 0.0);
    }

    #[test]
    fn identity_curve_normalizes_to_empty() {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.tone_curve.red = crate::grade_parameters::identity_curve();
        assert!(sanitize(&p).stage2.tone_curve.red.is_empty());
    }
}
