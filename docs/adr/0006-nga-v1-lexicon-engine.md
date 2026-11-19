# ADR-0006: NGA v1 — Deterministic Lexicon Engine behind a `GradeAgent` Trait

- Status: accepted
- Date: 2026-11-17
- Amends: ADR-0003 (sequencing only)

## Context

ADR-0003 chose a bespoke ONNX text-encoder with a parameter-regression head.
Training it needs a licensed prompt → grade corpus (ADR-0003 calls this the
highest-risk M5 sub-task), and no corpus exists yet. Shipping v1.0 without the
NGA would remove the product's core differentiator; shipping an untrained or
synthetic-data model would produce unreliable grades behind a confident UI.

## Decision

v1.0 ships `LexiconAgent`, a deterministic, fully on-device engine in
`apricity-nga`. It maps recognized phrases (moods, palettes, technical terms,
film stocks, genre and cinematographer references) to weighted parameter
deltas, with intensity modifiers, negation, removal, comparatives, and an
Auto/Fresh/Refine iteration mode. It records which prompt term moved which
parameter, so the Explainability Panel can cite the user's own words. It sits
behind a `GradeAgent` trait; the ONNX model from ADR-0003 implements the same
trait behind the off-by-default `onnx` feature once it's trained. `ort` is no
longer a default dependency, and the wasm32 build doesn't link it. Voice input
isn't offered on web, because the Web Speech API sends audio to a server,
which violates PRD G5.

## Consequences

- The NGA works offline everywhere, with zero model download, and its output
  is reproducible (grain seeds are hashed from the prompt; "Surprise Me" takes
  an explicit seed).
- Prompt coverage is bounded by the lexicon: unknown words are reported back to
  the user ("didn't understand: …") instead of being guessed at.
- The contract tests (schema conformance, console round-trip, a template for
  every emittable parameter) apply unchanged to the future ONNX agent.
- The latency budget (< 3 s p95 web) is met with orders of magnitude to spare.
