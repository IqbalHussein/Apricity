# ADR-0003: NGA — Bespoke Narrow Model, ONNX-exported

- Status: accepted
- Date: 2026-05-05

## Context

The Neural Grading Agent translates natural-language prompts into
`GradeParameters`. Three model strategies were considered:

1. **Hosted LLM API** — violates the on-device privacy invariant (PRD G5).
2. **Large local LLM with structured output** — hundreds of MB to multiple
   GB; latency budget unrealistic for browser/mobile; output is text and
   must be parsed.
3. **Bespoke narrow model: small text encoder + parameter-regression head**
   emitting `GradeParameters` fields directly.

## Decision

Build (3): a small text encoder (~30–80M parameters, distilled BERT-small or
sentence-transformer base) with a parameter-regression head. Train in PyTorch,
quantize INT8 post-training, export to **ONNX**, run via `ort` in Rust with
per-platform Execution Providers (WebGPU EP for browsers, NNAPI EP for
Android, DirectML EP for Windows, Core ML EP for iOS later).

The model's output is the `GradeParameters` schema — there is no text
generation step, no parsing, no JSON-validity bug class.

## Consequences

- Single model, every shell. No platform-specific retraining.
- INT8 quantization keeps the on-disk model under 200MB and active memory
  under the PRD's 200MB budget.
- Training data sourcing becomes the highest-risk M5 sub-task. ADR-0006 will
  cover dataset and licensing once the corpus is built.
- Rules out free-form rationale text from the model itself; the
  Explainability Panel uses **data-driven templates** keyed off the
  `GradeParameters` diff, not model-generated text. This is a feature, not a
  workaround — it makes explanations deterministic and auditable.
- Frees us from the prompt-injection / hallucination failure modes of
  text-generating LLMs.
