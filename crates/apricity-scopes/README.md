# apricity-scopes

Pure-Rust CPU scopes: luma waveform, RGB parade, vectorscope (+ graticule),
histogram, and a broadcast-legal check. No GPU, no ML, no I/O dependencies
(`apricity-core`, `glam`, `serde` only).

## Inputs / outputs

Every function takes `pixels: &[[f32; 3]]` of **output-encoded RGB**:
display-referred (e.g. sRGB-encoded), nominal range `[0, 1]`, **unclamped**
(values below 0 or above 1 are valid and are reported or clamped to scope
edges). Luma is Rec.709 `Y' = 0.2126R' + 0.7152G' + 0.0722B'` on the encoded
values. Raster scopes return `ScopeImage` (RGBA8, row-major, top row first,
alpha 0 where empty, non-premultiplied) for the host to composite.

| Function | Result |
|---|---|
| `waveform_luma(px, w, h, out_w, out_h)` | x = source column binned to `out_w`; y = `Y'`, 1.0 at the top row; out-of-range clamps to edge rows |
| `rgb_parade(px, w, h, out_w, out_h)` | three panels of `out_w / 3` each (image width `3 * (out_w / 3)`), R, G, B |
| `vectorscope(px, size)` | `size x size`; BT.709 `Cb = (B'-Y')/1.8556` right, `Cr = (R'-Y')/1.5748` up; scale 0.8 so 100% primaries/secondaries sit at radius 0.40 to 0.48 |
| `vectorscope_graticule()` | 75% targets `R MG B CY G YL`, skin-tone line angle (123.3 deg, from sRGB `(0.80, 0.58, 0.48)`, CCW from +Cb), Rec.709 hexagon, Display-P3 polygon; normalized coords, x right, y down |
| `histogram(px)` | 256 bins over `[0, 1]` for R, G, B, luma; out-of-range clamps into end bins; NaN counts as 0 |
| `broadcast_legal(px, tol)` | % luma over / under, % gamut excursion (any channel outside `[-tol, 1+tol]`); `legal` when all < 0.1% |

Intensity: hit counts map through `ln(1 + c) / ln(1 + ref)` where `ref` is the
99th percentile of nonzero bins, with alpha floored at 0.2 so single hits stay
visible. Vectorscope bins are tinted by the mean hue of their pixels (peak
channel normalized, lightened 25% toward white).

## Palette (color-blind safe, PRD 8.7)

- Luma waveform: neutral cyan-white `#C8FFFF`.
- Parade panels (Okabe-Ito): R `#D55E00` vermillion, G `#009E73` bluish green,
  B `#56B4E9` sky blue. Panels are also spatially separated, so color is never
  the only cue; there are no red/green-only indicators.
- Vectorscope: hue tint plus the fixed graticule labels.

## Performance

`cargo bench -p apricity-scopes` times each scope on 640x360 input; budget is
well under 5 ms per scope on the reference CI machine.

## Does not own

Rendering the grade (that is `apricity-render`), drawing the graticule or
choosing backgrounds (shell UI), color conversion to output encoding, or any
GPU kernels.
