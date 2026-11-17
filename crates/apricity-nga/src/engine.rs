//! Prompt tokenization, phrase matching, and application of lexicon entries
//! to [`GradeParameters`].

use apricity_core::curve;
use apricity_core::grade_parameters::{CurvePoint, GradeParameters, ToneCurve};
use apricity_core::param_spec::scalar_spec;

use crate::lexicon::{
    index, CurvePreset, Entry, Hit, Modifier, COMPARATIVES, ENTRIES, ITERATIVE_MARKERS, STOPWORDS,
};
use crate::Attribution;

const MASTER_CURVE: &str = "stage2.tone_curve.master";
const MAX_SCALE: f32 = 3.0;

/// Lowercase `prompt` and split it into alphanumeric words. Hyphens and other
/// punctuation separate words (`"teal-and-orange"` equals `"teal and orange"`),
/// `&` reads as "and", and apostrophes are dropped.
pub(crate) fn normalize(prompt: &str) -> Vec<String> {
    let mut out = String::with_capacity(prompt.len());
    for c in prompt.chars() {
        if c == '&' {
            out.push_str(" and ");
        } else if c == '\'' || c == '\u{2019}' {
        } else if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().map(str::to_owned).collect()
}

/// One lexicon entry selected for application.
#[derive(Debug, Clone)]
pub(crate) struct Applied {
    /// Index into [`ENTRIES`].
    pub entry: usize,
    /// Signed strength multiplier (1.0 = table values).
    pub scale: f32,
    /// Reset the entry's parameters to their defaults instead.
    pub remove: bool,
    /// Normalized phrase that triggered the entry.
    pub term: String,
}

/// Result of matching a prompt against the lexicon.
#[derive(Debug, Default)]
pub(crate) struct Parsed {
    /// Entries to apply, in prompt order.
    pub applied: Vec<Applied>,
    /// Distinct matched phrases, in prompt order.
    pub matched_terms: Vec<String>,
    /// Distinct content words that matched nothing.
    pub unmatched: Vec<String>,
}

#[derive(Clone, Copy)]
struct Pending {
    magnitude: f32,
    negate: bool,
    remove: bool,
}

impl Pending {
    const NONE: Self = Self {
        magnitude: 1.0,
        negate: false,
        remove: false,
    };

    fn absorb(&mut self, m: Modifier) {
        match m {
            Modifier::Scale(f) => self.magnitude = (self.magnitude * f).min(MAX_SCALE),
            Modifier::Negate => self.negate = true,
            Modifier::Remove => self.remove = true,
        }
    }

    fn scale(self) -> f32 {
        if self.negate {
            -self.magnitude
        } else {
            self.magnitude
        }
    }
}

/// Greedy longest-phrase match of `tokens` against the lexicon. Modifiers
/// apply to the next matched term and are cleared by an unmatched content
/// word.
pub(crate) fn parse(tokens: &[String]) -> Parsed {
    let idx = index();
    let mut parsed = Parsed::default();
    let mut pending = Pending::NONE;
    let mut i = 0;
    while i < tokens.len() {
        let mut found = None;
        for len in (1..=idx.max_words.min(tokens.len() - i)).rev() {
            let key = tokens[i..i + len].join(" ");
            if let Some(hit) = idx.map.get(key.as_str()) {
                found = Some((*hit, len, key));
                break;
            }
        }
        match found {
            Some((Hit::Entry(entry), len, key)) => {
                if !parsed.matched_terms.contains(&key) {
                    parsed.matched_terms.push(key.clone());
                }
                parsed.applied.push(Applied {
                    entry,
                    scale: pending.scale(),
                    remove: pending.remove,
                    term: key,
                });
                pending = Pending::NONE;
                i += len;
            }
            Some((Hit::Mod(m), len, _)) => {
                pending.absorb(m);
                i += len;
            }
            None => {
                let w = &tokens[i];
                if !STOPWORDS.contains(&w.as_str()) {
                    if !parsed.unmatched.contains(w) {
                        parsed.unmatched.push(w.clone());
                    }
                    pending = Pending::NONE;
                }
                i += 1;
            }
        }
    }
    parsed
}

/// Whether `tokens` contain an iterative marker or a comparative.
pub(crate) fn has_iterative_marker(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|t| ITERATIVE_MARKERS.contains(&t.as_str()) || COMPARATIVES.contains(&t.as_str()))
}

fn apply_curve(master: &mut Vec<CurvePoint>, preset: CurvePreset, scale: f32) {
    let base = if master.is_empty() {
        None
    } else {
        Some(curve::sanitize(master, ToneCurve::MAX_POINTS))
    };
    let pts: Vec<CurvePoint> = CurvePreset::XS
        .iter()
        .map(|&x| {
            let b = base.as_ref().map_or(x, |c| curve::evaluate(c, x));
            CurvePoint::new(x, (b + scale * preset.offset(x)).clamp(0.0, 1.0))
        })
        .collect();
    let clean = curve::sanitize(&pts, ToneCurve::MAX_POINTS);
    *master = if clean.iter().all(|p| (p.x - p.y).abs() < 1e-4) {
        Vec::new()
    } else {
        clean
    };
}

/// Apply `applied` entries in order to `p`'s Stage 2, clamping every scalar
/// to its spec range. When an applied entry raises `grain.amount`, the grain
/// seed becomes `grain_seed`. Returns one attribution per parameter actually
/// moved (delta is the effective, post-clamp change).
pub(crate) fn apply_terms(
    p: &mut GradeParameters,
    applied: &[Applied],
    grain_seed: u32,
) -> Vec<Attribution> {
    let mut attrs = Vec::new();
    let mut grain_on = false;
    for a in applied {
        let entry: &Entry = &ENTRIES[a.entry];
        for (short, d) in entry.deltas {
            let path = Entry::full_path(short);
            let Some(spec) = scalar_spec(&path) else {
                continue;
            };
            let old = spec.get(p);
            let target = if a.remove {
                spec.default
            } else {
                old + d * a.scale
            };
            let new = spec.clamp(target);
            spec.set(p, new);
            if path == "stage2.grain.amount" && !a.remove && d * a.scale > 0.0 {
                grain_on = true;
            }
            push_attr(&mut attrs, &a.term, path, new - old);
        }
        for (short, v) in entry.sets {
            let path = Entry::full_path(short);
            let Some(spec) = scalar_spec(&path) else {
                continue;
            };
            let target = if a.remove {
                spec.default
            } else if a.scale > 0.0 {
                *v
            } else {
                continue;
            };
            let old = spec.get(p);
            let new = spec.clamp(target);
            spec.set(p, new);
            push_attr(&mut attrs, &a.term, path, new - old);
        }
        if let Some(preset) = entry.curve {
            let master = &mut p.stage2.tone_curve.master;
            let before = master.clone();
            if a.remove {
                master.clear();
            } else {
                apply_curve(master, preset, a.scale);
            }
            if *master != before {
                let delta = if a.remove { -1.0 } else { a.scale };
                attrs.push(Attribution {
                    term: a.term.clone(),
                    path: MASTER_CURVE.to_owned(),
                    delta,
                });
            }
        }
    }
    if grain_on {
        p.stage2.grain.seed = grain_seed;
    }
    attrs
}

/// Add `(path, delta)` jitter directly to `p` under the term `variation`.
pub(crate) fn apply_jitter(p: &mut GradeParameters, jitter: &[(&str, f32)]) -> Vec<Attribution> {
    let mut attrs = Vec::new();
    for (short, d) in jitter {
        let path = Entry::full_path(short);
        let Some(spec) = scalar_spec(&path) else {
            continue;
        };
        let old = spec.get(p);
        let new = spec.clamp(old + d);
        spec.set(p, new);
        push_attr(&mut attrs, "variation", path, new - old);
    }
    attrs
}

fn push_attr(attrs: &mut Vec<Attribution>, term: &str, path: String, delta: f32) {
    if delta.abs() > 1e-6 {
        attrs.push(Attribution {
            term: term.to_owned(),
            path,
            delta,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyphens_and_punctuation_normalize() {
        assert_eq!(normalize("Teal-and-Orange!"), vec!["teal", "and", "orange"]);
        assert_eq!(normalize("B&W, it's"), vec!["b", "and", "w", "its"]);
    }

    #[test]
    fn longest_phrase_wins() {
        let p = parse(&normalize("blue hour"));
        assert_eq!(p.matched_terms, vec!["blue hour"]);
    }

    #[test]
    fn hyphenated_equals_spaced() {
        let a = parse(&normalize("teal-and-orange"));
        let b = parse(&normalize("teal and orange"));
        assert_eq!(a.matched_terms, b.matched_terms);
        assert_eq!(a.matched_terms.len(), 1);
    }

    #[test]
    fn stopwords_are_not_unmatched() {
        let p = parse(&normalize("make it a bit warm and zzzq"));
        assert_eq!(p.unmatched, vec!["zzzq"]);
    }
}
