# apricity-io

Image decode/encode with color-space tagging, for native shells and tests.
The web shell decodes with the browser instead. Uses the `image` crate with
PNG, JPEG, and WebP only.

## API

- `decode(bytes) -> DecodedImage` - sniffs the format, applies EXIF
  orientation, returns RGBA8. `color_space` is `DisplayP3` when the embedded
  ICC profile description contains "P3" (case-insensitive), otherwise `Srgb`.
- `icc_description(icc) -> Option<String>` - reads the `desc` tag, both v2
  `textDescriptionType` and v4 `mluc` (first record). Safe on malformed input.
- `encode(rgba8, w, h, ExportFormat::{Png, Jpeg { quality }}) -> Vec<u8>` -
  JPEG drops alpha; quality clamps to 1..=100. No ICC profile is embedded.
- `downscale_to_fit(img, max_dim)` - alpha-weighted box filter, preserves
  aspect and tag, no-op if already within `max_dim` (or `max_dim == 0`).

## Color-space contract

Buffers are 8-bit, non-premultiplied RGBA holding **encoded** values in the
tagged `ColorSpace`. Nothing here converts between spaces or linearizes;
downscaling averages encoded values. Color management is the pipeline's job.

## Does not own

HEIC, RAW, ProRAW, and video (host-supplied), ICC profile transforms, ICC
embedding on export, or any GPU upload.
