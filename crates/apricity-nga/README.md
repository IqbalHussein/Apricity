# apricity-nga

The Neural Grading Agent (NGA): natural-language prompt in, Stage 2
`GradeParameters` out, with a full explanation of every change.

v1 ships a **deterministic, on-device lexicon engine** (ADR-0006) behind the
`GradeAgent` trait. The ONNX model from ADR-0003 plugs into the same trait
later; shells and the Explainability Panel do not change.

## API

```rust
use apricity_nga::{GradeAgent, LexiconAgent, PromptMode};

let agent = LexiconAgent;
let result = agent.apply("moody, a bit cooler", &current, PromptMode::Auto);
result.params;          // sanitized GradeParameters; only stage2 can differ
result.changes;         // Vec<ParameterChange> for the Explainability Panel
result.summary;         // "The AI made 6 adjustments."
result.unmatched_words; // "didn't understand: ..."
let surprise = agent.surprise(&current, Some("noir"), /* seed */ 42);
```

- `PromptMode::Fresh` resets Stage 2 to identity (Stage 1 untouched), then
  applies the prompt. `Refine` applies deltas on top of the current Stage 2
  ("make it cooler, less green"). `Auto` picks `Refine` when the prompt has an
  iterative marker (more, less, a bit, slightly, too, increase, reduce, add,
  remove, or a comparative such as warmer) and Stage 2 is not identity.
- A prompt that matches nothing returns the current grade unchanged.
- `lexicon_terms()` lists every phrase for autocomplete and docs.
- `Attribution { term, path, delta }` records which prompt term moved which
  parameter; `NgaResult` is `serde`-serializable for the FFI.

## Lexicon design

`src/lexicon.rs` is a static table of entries (phrases and aliases, a
category, scalar deltas, optional absolute split-tone hues, optional master
tone-curve preset). Coverage: moods, palette and temperature, technical terms,
comparatives, film stocks, genres and eras, and nature and scenes. Film and
director names are descriptive keywords for a look only.

- Deltas are in each parameter's own unit (percent, mireds, tint units, stops,
  degrees, normalized wheel offsets in the working encoding). Paths are
  relative to `stage2.`.
- Tokenization lowercases and splits on punctuation (`teal-and-orange` equals
  `teal and orange`); matching is greedy longest-phrase.
- Modifiers apply to the next matched term: `slightly`, `a bit`, `subtle`
  (x0.5); `very`, `really`, `strong`, `more` (x1.6); `less`, `not`, `too`,
  `reduce` (negate); `no`, `without`, `remove` (reset that entry's parameters
  to their spec defaults). Terms sum and results are clamped to spec ranges and
  sanitized by `apricity-core`.
- Split-tone hues are absolute targets (set, not added) and only apply at
  positive strength; their saturations are deltas.
- Curve presets (`SCurve`, `Soft`, `Fade`, `Crush`, `Brighten`, `Darken`)
  compose with the existing master curve and scale with strength.
- Grain seed is an FNV-1a hash of the normalized prompt. "Surprise Me" uses a
  local SplitMix64 seeded explicitly: same seed, same output. There is no
  `rand`, clock, or ambient randomness anywhere in the crate.

## ONNX plug-in path

`ort` is an optional dependency behind the non-default `onnx` feature, so the
default and `wasm32` builds do not include it. A future `OnnxAgent` implements
`GradeAgent` (ADR-0003: small text encoder plus regression head emitting
`GradeParameters`) and reuses `explain::diff`, `NgaResult`, and the attribution
types; nothing else in the crate requires the feature.

## Explainability contract

`explain::diff(before, after, attributions)` reports every scalar that moved
by more than half its spec step, plus changed tone curves, in spec-table order
with curves last. It works for any two grades (manual edits, Recipe diffs) and
omits the "for '...'" clause when no attributions are given. Rationales are
deterministic templates (`explain::rationale_template`) filled from the spec's
`describes` phrase, direction, signed amount with unit, and attributing terms,
for example "Warmed the color temperature (+12 mired) for 'golden hour'."
Every NGA-emittable path and every tone-curve path has a template (tested).
`ParameterChange::console_anchor` equals the parameter path so the shell can
scroll the Manual Console to that control.

## Privacy

Fully on-device. This crate performs no I/O of any kind: no network, no files.

## Tests

`tests/contract.rs` holds contract tests (schema validity, stage 1 untouched,
template coverage, Manual Console round-trip, JSON round-trip, determinism,
modes, modifiers). Model-quality evaluation belongs in an offline eval
harness, not the unit suite.
