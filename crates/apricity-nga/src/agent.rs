//! The [`GradeAgent`] trait and the deterministic [`LexiconAgent`].

use apricity_core::grade_parameters::{CreativeParameters, GradeParameters};
use apricity_core::validate::sanitize;
use serde::{Deserialize, Serialize};

use crate::engine::{self, Applied};
use crate::explain::{self, ParameterChange};
use crate::lexicon::{Category, ENTRIES};
use crate::rng::{fnv1a32, SplitMix64};

/// How a prompt relates to the current grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptMode {
    /// Choose [`PromptMode::Refine`] when the prompt reads as a follow-up
    /// ("a bit warmer", "less green") and the current Stage 2 is not
    /// identity; otherwise [`PromptMode::Fresh`].
    Auto,
    /// Reset Stage 2 to identity (Stage 1 untouched), then apply the prompt.
    Fresh,
    /// Apply the prompt as deltas on top of the current Stage 2.
    Refine,
}

/// How much one prompt term moved one parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribution {
    /// The lexicon phrase (normalized) that caused the change, or
    /// `"variation"` for "Surprise Me" jitter.
    pub term: String,
    /// Scalar parameter path or tone-curve path.
    pub path: String,
    /// Effective change in the parameter's own unit, after clamping. For
    /// tone curves, the signed strength of the applied preset (unitless).
    pub delta: f32,
}

/// Outcome of one agent invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NgaResult {
    /// Sanitized grade. Only `stage2` may differ from the input grade.
    pub params: GradeParameters,
    /// The mode actually used (never [`PromptMode::Auto`]).
    pub mode_used: PromptMode,
    /// Lexicon phrases recognized, in prompt order, without repeats.
    pub matched_terms: Vec<String>,
    /// Content words the agent did not understand (stopwords excluded).
    pub unmatched_words: Vec<String>,
    /// Which term moved which parameter, and by how much.
    pub attributions: Vec<Attribution>,
    /// Parameter-level diff between the input grade and `params`.
    pub changes: Vec<ParameterChange>,
    /// Panel header text, e.g. "The AI made 6 adjustments."
    pub summary: String,
}

/// A prompt-to-grade engine. Implementations must be deterministic: the same
/// arguments yield the same result, and all randomness comes from `seed`.
pub trait GradeAgent {
    /// Stable identifier of the engine, e.g. `"lexicon"`.
    fn name(&self) -> &'static str;

    /// Apply `prompt` to `current`. Only Stage 2 of the returned grade may
    /// differ from `current`.
    fn apply(&self, prompt: &str, current: &GradeParameters, mode: PromptMode) -> NgaResult;

    /// "Surprise Me": a grade within the mood of `last_prompt` (or a random
    /// mood when `None`), fully determined by `seed`. Always starts from a
    /// reset Stage 2.
    fn surprise(
        &self,
        current: &GradeParameters,
        last_prompt: Option<&str>,
        seed: u64,
    ) -> NgaResult;
}

/// Deterministic, on-device lexicon engine (ADR-0006).
#[derive(Debug, Clone, Copy, Default)]
pub struct LexiconAgent;

struct Plan {
    fresh: bool,
    mode_used: PromptMode,
    applied: Vec<Applied>,
    jitter: Vec<(&'static str, f32)>,
    grain_seed: u32,
    matched_terms: Vec<String>,
    unmatched_words: Vec<String>,
}

fn assemble(current: &GradeParameters, plan: Plan) -> NgaResult {
    let (params, attributions) = if plan.applied.is_empty() && plan.jitter.is_empty() {
        (sanitize(current), Vec::new())
    } else {
        let mut base = current.clone();
        if plan.fresh {
            let CreativeParameters {
                lut_id,
                lut_strength,
                ..
            } = base.stage2.clone();
            base.stage2 = CreativeParameters {
                lut_id,
                lut_strength,
                ..CreativeParameters::IDENTITY
            };
        }
        let mut attrs = engine::apply_terms(&mut base, &plan.applied, plan.grain_seed);
        attrs.extend(engine::apply_jitter(&mut base, &plan.jitter));
        (sanitize(&base), attrs)
    };
    let mut params = params;
    params.stage1 = current.stage1.clone();
    let changes = explain::diff(current, &params, &attributions);
    let summary = explain::summary(&changes);
    NgaResult {
        params,
        mode_used: plan.mode_used,
        matched_terms: plan.matched_terms,
        unmatched_words: plan.unmatched_words,
        attributions,
        changes,
        summary,
    }
}

fn is_identity_stage2(p: &GradeParameters) -> bool {
    p.stage2 == CreativeParameters::IDENTITY
}

fn surprise_pool(c: Category) -> bool {
    matches!(
        c,
        Category::Mood | Category::Palette | Category::Stock | Category::Genre | Category::Scene
    )
}

const JITTER: [(&str, f32); 5] = [
    ("contrast", 8.0),
    ("saturation", 8.0),
    ("temperature", 6.0),
    ("exposure", 0.12),
    ("vibrance", 8.0),
];

impl GradeAgent for LexiconAgent {
    fn name(&self) -> &'static str {
        "lexicon"
    }

    fn apply(&self, prompt: &str, current: &GradeParameters, mode: PromptMode) -> NgaResult {
        let tokens = engine::normalize(prompt);
        let parsed = engine::parse(&tokens);
        let mode_used = match mode {
            PromptMode::Auto => {
                if engine::has_iterative_marker(&tokens) && !is_identity_stage2(current) {
                    PromptMode::Refine
                } else {
                    PromptMode::Fresh
                }
            }
            m => m,
        };
        let grain_seed = fnv1a32(tokens.join(" ").as_bytes());
        let has_terms = !parsed.applied.is_empty();
        assemble(
            current,
            Plan {
                fresh: mode_used == PromptMode::Fresh && has_terms,
                mode_used,
                applied: parsed.applied,
                jitter: Vec::new(),
                grain_seed,
                matched_terms: parsed.matched_terms,
                unmatched_words: parsed.unmatched,
            },
        )
    }

    fn surprise(
        &self,
        current: &GradeParameters,
        last_prompt: Option<&str>,
        seed: u64,
    ) -> NgaResult {
        let mut rng = SplitMix64::new(seed);
        let pool: Vec<usize> = (0..ENTRIES.len())
            .filter(|&i| surprise_pool(ENTRIES[i].category))
            .collect();
        let moods: Vec<usize> = pool
            .iter()
            .copied()
            .filter(|&i| ENTRIES[i].category == Category::Mood)
            .collect();
        let preferred: Vec<usize> = last_prompt
            .map(|p| {
                engine::parse(&engine::normalize(p))
                    .applied
                    .iter()
                    .map(|a| a.entry)
                    .filter(|&i| surprise_pool(ENTRIES[i].category))
                    .collect()
            })
            .unwrap_or_default();

        let count = 2 + rng.below(2);
        let mut chosen: Vec<usize> = Vec::new();
        for n in 0..count {
            let source: &[usize] = if n == 0 {
                if preferred.is_empty() {
                    &moods
                } else {
                    &preferred
                }
            } else if !preferred.is_empty() && rng.below(2) == 0 {
                &preferred
            } else {
                &pool
            };
            for _ in 0..8 {
                let pick = source[rng.below(source.len())];
                if !chosen.contains(&pick) {
                    chosen.push(pick);
                    break;
                }
            }
        }

        let applied: Vec<Applied> = chosen
            .iter()
            .map(|&entry| Applied {
                entry,
                scale: 0.6 + 0.7 * rng.unit(),
                remove: false,
                term: ENTRIES[entry].phrases[0].to_owned(),
            })
            .collect();
        let first = rng.below(JITTER.len());
        let second = (first + 1 + rng.below(JITTER.len() - 1)) % JITTER.len();
        let jitter: Vec<(&'static str, f32)> = [first, second]
            .iter()
            .map(|&i| (JITTER[i].0, (rng.unit() * 2.0 - 1.0) * JITTER[i].1))
            .collect();

        let matched_terms = applied.iter().map(|a| a.term.clone()).collect();
        assemble(
            current,
            Plan {
                fresh: true,
                mode_used: PromptMode::Fresh,
                applied,
                jitter,
                grain_seed: fnv1a32(&seed.to_le_bytes()),
                matched_terms,
                unmatched_words: Vec::new(),
            },
        )
    }
}
