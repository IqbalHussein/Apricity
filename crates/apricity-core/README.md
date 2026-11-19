# apricity-core

Pure-Rust color science core. The single source of truth for `GradeParameters`,
the parameter specification table, color-space conversions, and scene-linear
math. No GPU, ML, or I/O dependencies (`glam`, `serde`, `serde_json`,
`thiserror`; `tsify-next` and `wasm-bindgen` only behind the `tsify` feature).

## Owns

| Module | Contents |
|---|---|
| `grade_parameters` | `GradeParameters` (NGA output, Recipe file, and Manual Console share it), `CorrectionParameters` (Stage 1), `CreativeParameters` (Stage 2), wheels, HSL bands, split tone, tone curves, vignette, grain, `LogProfile`, `SCHEMA_VERSION`. |
| `param_spec` | The table of every scalar's path, label, unit, range, step, default, and console group. The only place ranges live; also provides `get_scalar`/`set_scalar`, curve path helpers, and `spec_table()` for the FFI. |
| `validate` | `validate` reports every problem; `sanitize` returns a renderable copy (finite, in range, curves sorted and bounded). Everything crossing the FFI or loaded from a Recipe passes through `sanitize`. |
| `color_space`, `transfer` | `ColorSpace`, primaries and RGB/XYZ matrices, `convert`, and the sRGB, BT.1886, and BT.2020 transfer functions. |
| `log_profiles` | ARRI LogC3, Sony S-Log3, Panasonic V-Log, and Apple Log decoding to scene-linear Rec.2020, plus the log-to-display rendering curve. |
| `white_balance` | Temperature (mireds relative to D65) and tint to Rec.2020 channel gains, the Planckian locus, and unit-luminance normalization. |
| `curve` | Monotone cubic Hermite tone curves: sanitize, evaluate, bake to a 256-entry table, and sample it. CPU and GPU both use the baked table. |
| `analysis` | Stage 1 auto-analysis of a scene-linear proxy: exposure, white balance with a detected illuminant, and black point. Deterministic, never touches Stage 2. |
| crate root | `WORKING_GAMMA`, `working_encode`, `working_decode`: the perceptual working encoding used by Stage 2 tone and wheel math. |

## Does not own

- GPU code or the render pipeline (`apricity-render`)
- ML inference or prompt handling (`apricity-nga`)
- File I/O (`apricity-io`), LUT files (`apricity-lut`)
- Recipe persistence (`apricity-recipes`)

## Color-space contract

The interior numeric format is **f32 scene-linear Rec.2020**. Inputs in other
spaces are converted at the seam, and outputs are converted at the seam.
Every public numeric parameter documents its unit (stops, mireds, percent,
degrees, and so on) in `param_spec`.

## Schema stability

`tests/schema_snapshot.rs` compares the serialized `GradeParameters` and spec
table against `tests/schema_snapshot.json`. Regenerate it only for a
deliberate schema change:

```
APRICITY_REGEN_SNAPSHOT=1 cargo test -p apricity-core --test schema_snapshot
```
