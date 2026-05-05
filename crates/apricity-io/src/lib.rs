//! Image read/write with color-space tagging.
//!
//! JPEG/PNG/WebP via the `image` crate. HEIC and ProRAW are platform-specific
//! and arrive via host-supplied traits (browser: WebCodecs; Android:
//! MediaCodec; iOS: ImageIO). Video I/O is host-supplied throughout.

#![forbid(unsafe_code)]
