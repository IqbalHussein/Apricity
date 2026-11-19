# apricity-lut

Pure-Rust `.cube` parser/writer and 3D LUT sampling. The GPU kernel that
consumes these tables lives in `apricity-render`, and baking a grade into a
LUT is `apricity_render::bake_lut`.

## API

- `parse_cube(text) -> Lut3d`: Adobe/Resolve `.cube`. Requires `LUT_3D_SIZE`
  (2 to 129; 1D-only files are rejected). A non-`[0, 1]` `DOMAIN_MIN` /
  `DOMAIN_MAX` is rescaled on load. Errors are `LutError` values with line
  numbers.
- `write_cube(&Lut3d) -> String`: writes a `TITLE` (if set), a header comment,
  `DOMAIN_MIN/MAX 0..1`, and rows with 6 decimals.
- `Lut3d`: `identity(size)`, `from_fn(size, f)`, `at(r, g, b)`,
  `sample_trilinear(rgb)` (mirrored in `stage2.wgsl`), `sample_tetrahedral(rgb)`
  (offline resampling), and `to_rgba_f32()` for texture upload.
- `DEFAULT_SIZE` is 33; `MAX_SIZE` is 129.

## Color-space contract

Apricity treats every `.cube` LUT as mapping **sRGB-encoded Rec.709 in
[0, 1]** to **sRGB-encoded Rec.709**, the de-facto convention for creative
LUTs exchanged with Resolve, Premiere, and Final Cut. LUTs carry no
color-space tag. The render pipeline converts its working signal into that
domain before sampling and back afterwards. Data order is red fastest, then
green, then blue.

## Does not own

Color-space conversion, applying a LUT inside the pipeline, or baking a grade
into a LUT (`apricity-render`). No GPU, ML, or file-system access.
