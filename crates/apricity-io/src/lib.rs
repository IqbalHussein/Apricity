//! Image read/write with color-space tagging.
//!
//! JPEG/PNG/WebP via the `image` crate, for native shells and tests (the web
//! shell decodes with the browser). HEIC and ProRAW are platform-specific
//! and arrive via host-supplied traits (browser: WebCodecs; Android:
//! MediaCodec; iOS: ImageIO). Video I/O is host-supplied throughout.
//!
//! All pixel buffers are **8-bit RGBA, non-premultiplied, row-major, top row
//! first, holding encoded (non-linear) values** in the image's tagged
//! [`ColorSpace`]. No color conversion happens here; that is the render
//! pipeline's job.

#![forbid(unsafe_code)]

mod icc;
mod scale;

use std::io::Cursor;

use apricity_core::ColorSpace;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageReader};

pub use icc::icc_description;
pub use scale::downscale_to_fit;

/// Errors from decoding or encoding.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IoError {
    /// Bytes are not a supported/valid PNG, JPEG, or WebP; carries the codec message.
    #[error("decode failed: {0}")]
    Decode(String),
    /// The encoder failed; carries the codec message.
    #[error("encode failed: {0}")]
    Encode(String),
    /// Width or height is zero, or the buffer length is not `width * height * 4`.
    #[error("buffer does not match {width}x{height} RGBA8")]
    BadBuffer {
        /// Declared width in pixels.
        width: u32,
        /// Declared height in pixels.
        height: u32,
    },
}

/// A decoded raster.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8 bytes, 4 per pixel, **encoded values in [`Self::color_space`]**,
    /// non-premultiplied alpha. Length is `width * height * 4`.
    pub rgba8: Vec<u8>,
    /// Color space the encoded values are in: `DisplayP3` when the embedded
    /// ICC profile's description mentions "P3", otherwise `Srgb`.
    pub color_space: ColorSpace,
}

/// Export container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Lossless PNG, 8-bit RGBA.
    Png,
    /// Lossy JPEG, alpha dropped. `quality` is 1 to 100 (clamped).
    Jpeg {
        /// Encoder quality, 1 (smallest) to 100 (best); out-of-range clamps.
        quality: u8,
    },
}

/// Decode PNG, JPEG, or WebP bytes (format is sniffed from the content).
///
/// The embedded ICC profile, if any, selects the tag: a description
/// containing "P3" (case-insensitive) yields [`ColorSpace::DisplayP3`];
/// anything else, including no profile, yields [`ColorSpace::Srgb`]. EXIF
/// orientation is applied. The ICC profile itself is not retained.
pub fn decode(bytes: &[u8]) -> Result<DecodedImage, IoError> {
    let err = |e: image::ImageError| IoError::Decode(e.to_string());
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| IoError::Decode(e.to_string()))?;
    let mut decoder = reader.into_decoder().map_err(err)?;
    let icc = decoder.icc_profile().ok().flatten();
    let orientation = decoder.orientation().map_err(err)?;
    let mut img = DynamicImage::from_decoder(decoder).map_err(err)?;
    img.apply_orientation(orientation);

    let color_space = match icc.as_deref().and_then(icc_description) {
        Some(d) if d.to_ascii_lowercase().contains("p3") => ColorSpace::DisplayP3,
        _ => ColorSpace::Srgb,
    };
    let rgba = img.into_rgba8();
    Ok(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba8: rgba.into_raw(),
        color_space,
    })
}

/// Encode an RGBA8 buffer (`width * height * 4` bytes, encoded values,
/// non-premultiplied) as PNG or JPEG. No ICC profile is embedded; the caller
/// owns tagging policy.
pub fn encode(
    rgba8: &[u8],
    width: u32,
    height: u32,
    format: ExportFormat,
) -> Result<Vec<u8>, IoError> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4));
    if width == 0 || height == 0 || expected != Some(rgba8.len()) {
        return Err(IoError::BadBuffer { width, height });
    }
    let mut out = Vec::new();
    let res = match format {
        ExportFormat::Png => {
            PngEncoder::new(&mut out).write_image(rgba8, width, height, ExtendedColorType::Rgba8)
        }
        ExportFormat::Jpeg { quality } => {
            let rgb: Vec<u8> = rgba8
                .chunks_exact(4)
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect();
            JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100)).write_image(
                &rgb,
                width,
                height,
                ExtendedColorType::Rgb8,
            )
        }
    };
    res.map_err(|e| IoError::Encode(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(w: u32, h: u32) -> Vec<u8> {
        (0..w * h)
            .flat_map(|i| {
                let (x, y) = (i % w, i / w);
                [(x * 255 / w) as u8, (y * 255 / h) as u8, 128, 255]
            })
            .collect()
    }

    #[test]
    fn png_round_trip_is_exact_and_srgb_tagged() {
        let src = gradient(16, 8);
        let bytes = encode(&src, 16, 8, ExportFormat::Png).unwrap();
        let d = decode(&bytes).unwrap();
        assert_eq!((d.width, d.height), (16, 8));
        assert_eq!(d.rgba8, src);
        assert_eq!(d.color_space, ColorSpace::Srgb);
    }

    #[test]
    fn jpeg_round_trip_is_close() {
        let src = gradient(32, 32);
        let bytes = encode(&src, 32, 32, ExportFormat::Jpeg { quality: 95 }).unwrap();
        let d = decode(&bytes).unwrap();
        assert_eq!((d.width, d.height), (32, 32));
        let mean_err = src
            .iter()
            .zip(&d.rgba8)
            .map(|(a, b)| (*a as i32 - *b as i32).unsigned_abs())
            .sum::<u32>() as f32
            / src.len() as f32;
        assert!(mean_err < 4.0, "mean abs error {mean_err}");
        assert!(d.rgba8.chunks_exact(4).all(|p| p[3] == 255));
    }

    #[test]
    fn jpeg_quality_is_clamped() {
        let src = gradient(8, 8);
        assert!(encode(&src, 8, 8, ExportFormat::Jpeg { quality: 0 }).is_ok());
        assert!(encode(&src, 8, 8, ExportFormat::Jpeg { quality: 200 }).is_ok());
    }

    #[test]
    fn p3_profile_tags_display_p3() {
        let profile = icc::tests::build_desc_profile("Display P3");
        let src = gradient(4, 4);
        let mut bytes = Vec::new();
        let mut enc = PngEncoder::new(&mut bytes);
        enc.set_icc_profile(profile).unwrap();
        enc.write_image(&src, 4, 4, ExtendedColorType::Rgba8)
            .unwrap();
        assert_eq!(decode(&bytes).unwrap().color_space, ColorSpace::DisplayP3);

        let srgb = icc::tests::build_desc_profile("sRGB IEC61966-2.1");
        let mut bytes = Vec::new();
        let mut enc = PngEncoder::new(&mut bytes);
        enc.set_icc_profile(srgb).unwrap();
        enc.write_image(&src, 4, 4, ExtendedColorType::Rgba8)
            .unwrap();
        assert_eq!(decode(&bytes).unwrap().color_space, ColorSpace::Srgb);
    }

    #[test]
    fn bad_inputs_are_errors() {
        assert!(matches!(decode(b"not an image"), Err(IoError::Decode(_))));
        assert!(matches!(decode(&[]), Err(IoError::Decode(_))));
        assert_eq!(
            encode(&[0; 3], 1, 1, ExportFormat::Png),
            Err(IoError::BadBuffer {
                width: 1,
                height: 1
            })
        );
        assert!(encode(&[], 0, 0, ExportFormat::Png).is_err());
    }
}
