# apricity-io

Image read/write with color-space tagging.

## Owns

- JPEG / PNG / WebP decode + encode (via `image`)
- ICC profile parsing where present; fallback assumptions documented per format
- Color-space tagging on every loaded image

## Does not own

- HEIC / ProRAW / video decode — host shell supplies these via trait impls
  (web: WebCodecs; Android: MediaCodec; iOS: ImageIO/AVFoundation)
