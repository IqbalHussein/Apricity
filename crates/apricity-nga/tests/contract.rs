//! Contract tests for the NGA: schema conformance, explainability coverage,
//! determinism, and modifier semantics. Not model-quality tests.

use apricity_core::grade_parameters::{CreativeParameters, GradeParameters};
use apricity_core::param_spec::{all_scalar_specs, get_scalar, scalar_spec, CURVE_PATHS};
use apricity_core::validate::validate;
use apricity_nga::explain::{diff, rationale_template, summary};
use apricity_nga::lexicon::{lexicon_terms, Entry, ENTRIES};
use apricity_nga::{GradeAgent, LexiconAgent, NgaResult, PromptMode};

fn agent() -> LexiconAgent {
    LexiconAgent
}

fn run(prompt: &str, current: &GradeParameters, mode: PromptMode) -> NgaResult {
    agent().apply(prompt, current, mode)
}

fn identity() -> GradeParameters {
    GradeParameters::IDENTITY
}

fn with_stage1() -> GradeParameters {
    let mut p = identity();
    p.stage1.exposure = 0.7;
    p.stage1.temperature = -12.0;
    p.stage1.black_point.r = 0.004;
    p
}

#[test]
fn every_lexicon_path_exists_and_is_finite() {
    assert!(ENTRIES.len() >= 120);
    for e in ENTRIES {
        for (short, d) in e.deltas.iter().chain(e.sets.iter()) {
            let path = Entry::full_path(short);
            let spec = scalar_spec(&path).unwrap_or_else(|| panic!("unknown path {path}"));
            assert!(spec.nga, "{path} is not NGA-emittable");
            assert!(d.is_finite(), "{path}");
        }
    }
}

#[test]
fn every_entry_alone_yields_valid_params_and_untouched_stage1() {
    let base = with_stage1();
    for e in ENTRIES {
        for prompt in [
            e.phrases[0].to_owned(),
            format!("very {}", e.phrases[0]),
            format!("less {}", e.phrases[0]),
        ] {
            for mode in [PromptMode::Fresh, PromptMode::Refine] {
                let r = run(&prompt, &base, mode);
                assert!(
                    validate(&r.params).is_empty(),
                    "{prompt}: {:?}",
                    validate(&r.params)
                );
                assert_eq!(r.params.stage1, base.stage1, "{prompt}");
                assert_eq!(r.matched_terms.len(), 1, "{prompt}: {:?}", r.matched_terms);
            }
        }
    }
}

#[test]
fn every_phrase_is_recognized_as_itself() {
    for p in lexicon_terms() {
        let r = run(p, &identity(), PromptMode::Fresh);
        assert_eq!(r.matched_terms, vec![p.to_owned()], "{p}");
        assert!(r.unmatched_words.is_empty(), "{p}");
    }
}

#[test]
fn rationale_template_covers_every_nga_path() {
    for s in all_scalar_specs().filter(|s| s.nga) {
        assert!(rationale_template(s.path).is_some(), "{}", s.path);
    }
    for c in CURVE_PATHS {
        assert!(rationale_template(c).is_some(), "{c}");
    }
    assert!(rationale_template("stage2.nope").is_none());
}

#[test]
fn every_change_round_trips_through_the_console() {
    let base = with_stage1();
    for e in ENTRIES {
        let r = run(e.phrases[0], &base, PromptMode::Fresh);
        for c in &r.changes {
            assert_eq!(c.console_anchor, c.path);
            assert!(c.rationale.ends_with('.') && !c.rationale.is_empty());
            if scalar_spec(&c.path).is_some() {
                assert_eq!(get_scalar(&r.params, &c.path), Some(c.after), "{}", c.path);
                assert_eq!(c.after - c.before, c.delta);
                assert_eq!(
                    c.stage, "creative",
                    "NGA must only change Stage 2: {}",
                    c.path
                );
            } else {
                assert!(CURVE_PATHS.contains(&c.path.as_str()));
                assert_eq!(c.unit, "curve");
            }
        }
        for a in &r.attributions {
            assert!(scalar_spec(&a.path).is_some() || CURVE_PATHS.contains(&a.path.as_str()));
        }
    }
}

#[test]
fn result_round_trips_through_json() {
    let r = run(
        "moody golden hour with grain",
        &identity(),
        PromptMode::Auto,
    );
    assert!(!r.changes.is_empty());
    let json = serde_json::to_string(&r).unwrap();
    let back: NgaResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back, r);
    assert!(json.contains("\"mode_used\":\"fresh\""));
}

#[test]
fn apply_is_deterministic() {
    let base = with_stage1();
    for mode in [PromptMode::Auto, PromptMode::Fresh, PromptMode::Refine] {
        let a = run("gritty film grain, a bit cooler", &base, mode);
        let b = run("gritty film grain, a bit cooler", &base, mode);
        assert_eq!(a, b);
    }
}

#[test]
fn grain_seed_is_a_stable_hash_of_the_prompt() {
    let a = run("grainy", &identity(), PromptMode::Fresh);
    let b = run("Grainy!", &identity(), PromptMode::Fresh);
    let c = run("grainy night", &identity(), PromptMode::Fresh);
    assert!(a.params.stage2.grain.amount > 0.0);
    assert_eq!(a.params.stage2.grain.seed, b.params.stage2.grain.seed);
    assert_ne!(a.params.stage2.grain.seed, c.params.stage2.grain.seed);
}

#[test]
fn surprise_is_seeded() {
    let base = with_stage1();
    let a = agent().surprise(&base, Some("moody noir"), 42);
    let b = agent().surprise(&base, Some("moody noir"), 42);
    assert_eq!(a, b);
    assert!(validate(&a.params).is_empty());
    assert_eq!(a.params.stage1, base.stage1);
    assert!(!a.changes.is_empty());
    let distinct = (1..=8u64)
        .map(|s| agent().surprise(&base, None, s).params)
        .collect::<Vec<_>>();
    assert!(distinct.windows(2).any(|w| w[0] != w[1]));
    assert_ne!(
        agent().surprise(&base, None, 1).params,
        agent().surprise(&base, None, 2).params
    );
}

#[test]
fn surprise_prefers_last_prompt_mood() {
    let r = agent().surprise(&identity(), Some("noir"), 7);
    assert!(r.matched_terms.iter().any(|t| t == "noir"));
}

#[test]
fn fresh_resets_prior_stage2() {
    let mut cur = identity();
    cur.stage2.contrast = 40.0;
    cur.stage2.saturation = -30.0;
    let r = run("warm", &cur, PromptMode::Fresh);
    assert_eq!(r.mode_used, PromptMode::Fresh);
    assert_eq!(r.params.stage2.contrast, 0.0);
    assert_eq!(r.params.stage2.saturation, 3.0);
    assert!(r.params.stage2.temperature > 0.0);
}

#[test]
fn refine_accumulates() {
    let once = run("warm", &identity(), PromptMode::Fresh);
    let twice = run("warm", &once.params, PromptMode::Refine);
    assert!(twice.params.stage2.temperature > once.params.stage2.temperature);
    let mut cur = identity();
    cur.stage2.contrast = 40.0;
    let r = run("warm", &cur, PromptMode::Refine);
    assert_eq!(r.params.stage2.contrast, 40.0);
}

#[test]
fn auto_mode_selection() {
    let mut cur = identity();
    cur.stage2.contrast = 25.0;
    assert_eq!(
        run("a bit warmer", &cur, PromptMode::Auto).mode_used,
        PromptMode::Refine
    );
    assert_eq!(
        run("make it cooler, less green", &cur, PromptMode::Auto).mode_used,
        PromptMode::Refine
    );
    assert_eq!(
        run("a bit warmer", &identity(), PromptMode::Auto).mode_used,
        PromptMode::Fresh
    );
    assert_eq!(
        run("golden hour", &cur, PromptMode::Auto).mode_used,
        PromptMode::Fresh
    );
    let r = run("a bit warmer", &cur, PromptMode::Auto);
    assert_eq!(r.params.stage2.contrast, 25.0);
}

#[test]
fn directional_lexicon_contracts() {
    let t = |p: &str| run(p, &identity(), PromptMode::Fresh).params.stage2;
    assert!(t("warm").temperature > 0.0);
    assert!(t("cool").temperature < 0.0);
    assert!(t("high contrast").contrast > 0.0);
    assert!(t("desaturated").saturation < 0.0);
    assert!(t("bright").exposure > 0.0);
    assert!(t("dark").exposure < 0.0);
    assert!(t("lifted blacks").blacks > 0.0);
    assert!(t("matte").tone_curve.master.len() >= 2);
    assert_eq!(t("black and white").saturation, -100.0);
}

#[test]
fn modifiers_scale_negate_and_remove() {
    let base = run("warm", &identity(), PromptMode::Fresh)
        .params
        .stage2
        .temperature;
    let t = |p: &str| {
        run(p, &identity(), PromptMode::Fresh)
            .params
            .stage2
            .temperature
    };
    assert!((t("slightly warm") - base * 0.5).abs() < 1e-4);
    assert!((t("a bit warm") - base * 0.5).abs() < 1e-4);
    assert!((t("very warm") - base * 1.6).abs() < 1e-4);
    assert!((t("less warm") + base).abs() < 1e-4);
    assert!((t("not warm") + base).abs() < 1e-4);
    assert!((t("a bit less warm") + base * 0.5).abs() < 1e-4);
    assert!((t("warm and warm") - base * 2.0).abs() < 1e-4);

    let grain = run("grainy", &identity(), PromptMode::Fresh).params;
    assert!(grain.stage2.grain.amount > 0.0);
    let gone = run("no grain", &grain, PromptMode::Refine).params;
    assert_eq!(gone.stage2.grain.amount, 0.0);
    assert_eq!(gone.stage2.grain.size, 1.0);

    let matte = run("matte", &identity(), PromptMode::Fresh).params;
    assert!(!matte.stage2.tone_curve.master.is_empty());
    let cleared = run("without matte", &matte, PromptMode::Refine).params;
    assert!(cleared.stage2.tone_curve.master.is_empty());
    assert_eq!(cleared.stage2.blacks, 0.0);
}

#[test]
fn hyphenated_prompts_match_spaced() {
    let a = run("teal-and-orange", &identity(), PromptMode::Fresh);
    let b = run("teal and orange", &identity(), PromptMode::Fresh);
    assert_eq!(a, b);
    assert_eq!(a.matched_terms, vec!["teal and orange"]);
}

#[test]
fn clamps_to_spec_ranges() {
    let r = run(
        "very very very very black and white contrasty",
        &identity(),
        PromptMode::Fresh,
    );
    assert!(validate(&r.params).is_empty());
    assert_eq!(r.params.stage2.saturation, -100.0);
}

#[test]
fn empty_and_gibberish_prompts_change_nothing() {
    let mut cur = with_stage1();
    cur.stage2.contrast = 12.0;
    for (prompt, unmatched) in [
        ("", vec![]),
        ("   ", vec![]),
        ("qwxz plorf", vec!["qwxz", "plorf"]),
    ] {
        for mode in [PromptMode::Auto, PromptMode::Fresh, PromptMode::Refine] {
            let r = run(prompt, &cur, mode);
            assert_eq!(r.params, cur, "{prompt:?}");
            assert!(r.changes.is_empty());
            assert!(r.attributions.is_empty());
            assert_eq!(r.unmatched_words, unmatched);
            assert_eq!(r.summary, "No adjustments were needed.");
        }
    }
}

#[test]
fn partially_understood_prompts_report_unmatched_words() {
    let r = run("make it moody and zorblax", &identity(), PromptMode::Fresh);
    assert_eq!(r.matched_terms, vec!["moody"]);
    assert_eq!(r.unmatched_words, vec!["zorblax"]);
    assert!(r.summary.starts_with("The AI made "));
}

#[test]
fn rationales_name_the_prompt_terms() {
    let r = run("golden hour", &identity(), PromptMode::Fresh);
    let t = r
        .changes
        .iter()
        .find(|c| c.path == "stage2.temperature")
        .unwrap();
    assert!(
        t.rationale.starts_with("Warmed the color temperature (+"),
        "{}",
        t.rationale
    );
    assert!(
        t.rationale.ends_with(" mired) for 'golden hour'."),
        "{}",
        t.rationale
    );
    assert_eq!(t.unit, "mireds");
    assert_eq!(t.unit_suffix, " mired");
}

#[test]
fn curve_changes_are_reported_last() {
    let r = run("matte", &identity(), PromptMode::Fresh);
    let last = r.changes.last().unwrap();
    assert_eq!(last.path, "stage2.tone_curve.master");
    assert_eq!(last.unit, "curve");
    assert_eq!(last.before_text, "Linear");
    assert!(last.after_text.ends_with(" points)"), "{}", last.after_text);
}

#[test]
fn diff_on_manual_edits_without_attributions() {
    let mut after = identity();
    after.stage2.contrast = 20.0;
    after.stage2.temperature = -12.0;
    after.stage2.lift.b = 0.03;
    after.stage1.exposure = 0.5;
    let ch = diff(&identity(), &after, &[]);
    let paths: Vec<&str> = ch.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "stage1.exposure",
            "stage2.contrast",
            "stage2.temperature",
            "stage2.lift.b"
        ]
    );
    for c in &ch {
        assert!(!c.rationale.contains('\''), "{}", c.rationale);
        assert!(!c.rationale.contains(" for "), "{}", c.rationale);
    }
    assert_eq!(ch[1].rationale, "Raised contrast (+20%).");
    assert_eq!(
        ch[2].rationale,
        "Cooled the color temperature (\u{2212}12 mired)."
    );
    assert_eq!(ch[0].stage, "correction");
    assert_eq!(summary(&ch), "The AI made 4 adjustments.");
    assert_eq!(summary(&ch[..1]), "The AI made 1 adjustment.");
}

#[test]
fn diff_ignores_sub_step_noise() {
    let mut after = identity();
    after.stage2.contrast = 0.4;
    assert!(diff(&identity(), &after, &[]).is_empty());
}

#[test]
fn stage2_identity_constant_is_what_fresh_resets_to() {
    let mut cur = identity();
    cur.stage2.vibrance = 50.0;
    let r = run("unknownword", &cur, PromptMode::Fresh);
    assert_eq!(r.params.stage2.vibrance, 50.0);
    let r = run("moody", &cur, PromptMode::Fresh);
    assert_ne!(r.params.stage2, CreativeParameters::IDENTITY);
    assert!((r.params.stage2.vibrance - 0.0).abs() < 1e-6);
}

#[test]
fn stock_names_with_numbers_and_filler_are_understood() {
    let r = run(
        "Kodak Portra 400 film stock",
        &identity(),
        PromptMode::Fresh,
    );
    assert_eq!(r.matched_terms, vec!["kodak portra 400"]);
    assert!(r.unmatched_words.is_empty(), "{:?}", r.unmatched_words);
    for p in [
        "portra 400",
        "portra 160",
        "portra 800",
        "cinestill 800t",
        "ektar 100",
        "velvia 50",
        "superia 400",
        "hp5 plus",
        "tri-x 400",
    ] {
        let r = run(p, &identity(), PromptMode::Fresh);
        assert_eq!(r.matched_terms.len(), 1, "{p}: {:?}", r.matched_terms);
        assert!(r.unmatched_words.is_empty(), "{p}");
    }
    let r = run("warm 2024", &identity(), PromptMode::Fresh);
    assert!(r.unmatched_words.is_empty());
}

#[test]
fn fresh_reset_changes_are_labeled_as_resets() {
    let mut cur = identity();
    cur.stage2.saturation = 92.0;
    cur.stage2.tone_curve.master = apricity_core::grade_parameters::ToneCurve::IDENTITY.master;
    let r = run("cool", &cur, PromptMode::Fresh);
    let c = r
        .changes
        .iter()
        .find(|c| c.path == "stage2.saturation")
        .unwrap();
    assert_eq!(
        c.rationale,
        "Reset overall saturation to neutral (\u{2212}92%) for the new look."
    );
    let mut cur = identity();
    cur.stage2.tone_curve.master = apricity_nga::lexicon::CurvePreset::Fade.points(1.0);
    let r = run("warm", &cur, PromptMode::Fresh);
    let c = r
        .changes
        .iter()
        .find(|c| c.path == "stage2.tone_curve.master")
        .unwrap();
    assert_eq!(
        c.rationale,
        "Reset the master tone curve to linear for the new look."
    );
    let mut manual = identity();
    manual.stage2.contrast = 20.0;
    let ch = diff(&manual, &identity(), &[]);
    assert_eq!(ch[0].rationale, "Reset contrast to neutral (\u{2212}20%).");
}

#[test]
fn fresh_preserves_the_lut() {
    let mut cur = identity();
    cur.stage2.lut_id = Some("teal.cube".into());
    cur.stage2.lut_strength = 55.0;
    cur.stage2.contrast = 30.0;
    let r = run("warm", &cur, PromptMode::Fresh);
    assert_eq!(r.params.stage2.lut_id.as_deref(), Some("teal.cube"));
    assert_eq!(r.params.stage2.lut_strength, 55.0);
    assert_eq!(r.params.stage2.contrast, 0.0);
}

#[test]
fn unattributed_fresh_changes_are_always_resets() {
    let mut base = with_stage1();
    base.stage2.contrast = 33.0;
    base.stage2.vignette.amount = -40.0;
    base.stage2.tone_curve.master = apricity_nga::lexicon::CurvePreset::SCurve.points(1.0);
    for e in ENTRIES {
        let r = run(e.phrases[0], &base, PromptMode::Fresh);
        for c in &r.changes {
            let attributed = r.attributions.iter().any(|a| a.path == c.path);
            if !attributed {
                assert!(
                    c.rationale.contains("Reset"),
                    "{}: {}",
                    e.phrases[0],
                    c.rationale
                );
            }
        }
    }
}
