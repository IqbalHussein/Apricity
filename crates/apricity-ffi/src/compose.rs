//! Pure helpers behind the web `Engine`: pixel buffers, downscaling,
//! compare-side resolution, CPU split compositing and scope assembly.
//!
//! No GPU or DOM types, so everything is unit-tested natively. All pixel
//! data is **sRGB-encoded RGBA8**, row-major, top row first.

use apricity_core::GradeParameters;
use apricity_render::gpu::CompareParams;
use apricity_scopes::{
    broadcast_legal, histogram, rgb_parade, vectorscope, waveform_luma, Histogram, LegalReport,
    ScopeImage,
};
use serde::{Deserialize, Serialize, Serializer};

use crate::api::{ApiError, ApiResult};

/// Waveform scope raster size in pixels (width, height).
pub const WAVEFORM_SIZE: (u32, u32) = (256, 128);
/// RGB parade raster size in pixels (width, height); three 128-wide panels.
pub const PARADE_SIZE: (u32, u32) = (384, 128);
/// Vectorscope raster edge in pixels.
pub const VECTORSCOPE_SIZE: u32 = 256;
/// Tolerance of the broadcast-legal check, in output-encoded unit-interval
/// units (0.005 is about 0.5 IRE).
pub const LEGAL_TOLERANCE: f32 = 0.005;

/// An RGBA8 image (sRGB-encoded, straight alpha).
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// `width * height * 4` bytes.
    pub rgba: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Image {
    /// Validate the buffer length against the dimensions.
    pub fn new(rgba: Vec<u8>, width: u32, height: u32) -> ApiResult<Self> {
        let want = width as usize * height as usize * 4;
        if width == 0 || height == 0 || rgba.len() != want {
            return Err(ApiError::new(format!(
                "pixel buffer is {} bytes, expected {want} for {width}x{height} RGBA",
                rgba.len()
            )));
        }
        Ok(Self {
            rgba,
            width,
            height,
        })
    }
}

/// Box-filter downscale so the result fits `max_w × max_h`, preserving aspect.
/// Averages the encoded values (adequate for previews and analysis). Returns a
/// copy unchanged when it already fits.
pub fn box_downscale(img: &Image, max_w: u32, max_h: u32) -> Image {
    let fit = (max_w as f64 / img.width as f64).min(max_h as f64 / img.height as f64);
    if fit >= 1.0 {
        return img.clone();
    }
    let dw = ((img.width as f64 * fit).round() as u32).max(1);
    let dh = ((img.height as f64 * fit).round() as u32).max(1);
    let mut out = Vec::with_capacity(dw as usize * dh as usize * 4);
    for dy in 0..dh {
        let y0 = (dy as u64 * img.height as u64 / dh as u64) as u32;
        let y1 = (((dy as u64 + 1) * img.height as u64).div_ceil(dh as u64) as u32)
            .clamp(y0 + 1, img.height);
        for dx in 0..dw {
            let x0 = (dx as u64 * img.width as u64 / dw as u64) as u32;
            let x1 = (((dx as u64 + 1) * img.width as u64).div_ceil(dw as u64) as u32)
                .clamp(x0 + 1, img.width);
            let mut acc = [0u32; 4];
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y as usize * img.width as usize + x as usize) * 4;
                    for (a, v) in acc.iter_mut().zip(&img.rgba[i..i + 4]) {
                        *a += *v as u32;
                    }
                }
            }
            let n = (y1 - y0) * (x1 - x0);
            for a in acc {
                out.push(((a + n / 2) / n) as u8);
            }
        }
    }
    Image {
        rgba: out,
        width: dw,
        height: dh,
    }
}

/// Nearest-neighbour resample to `w × h`.
pub fn resample_nearest(img: &Image, w: u32, h: u32) -> Image {
    if (img.width, img.height) == (w, h) {
        return img.clone();
    }
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h {
        let sy = ((y as u64 * 2 + 1) * img.height as u64 / (h as u64 * 2)) as usize;
        for x in 0..w {
            let sx = ((x as u64 * 2 + 1) * img.width as u64 / (w as u64 * 2)) as usize;
            let i = (sy * img.width as usize + sx) * 4;
            out.extend_from_slice(&img.rgba[i..i + 4]);
        }
    }
    Image {
        rgba: out,
        width: w,
        height: h,
    }
}

/// What side B of a comparison shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum CompareSource {
    /// The ungraded source (identity grade, input profile still decoded).
    #[default]
    Original,
    /// Stage 1 only.
    Stage1,
    /// The grade supplied in `previous`.
    Previous,
    /// The imported reference image.
    Reference,
}

/// Wipe comparison settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
#[serde(default)]
pub struct CompareConfig {
    /// Draw a split.
    pub enabled: bool,
    /// `true`: the split line is vertical (A left, B right); `false`: horizontal (A top).
    pub vertical: bool,
    /// Split position as a fraction of the image, 0..1.
    pub position: f32,
    /// What side B shows.
    pub source: CompareSource,
    /// Grade for [`CompareSource::Previous`]; `None` falls back to the original.
    pub previous: Option<GradeParameters>,
}

impl Default for CompareConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            vertical: true,
            position: 0.5,
            source: CompareSource::Original,
            previous: None,
        }
    }
}

/// Parameters for side B and whether B samples the reference image.
pub fn resolve_compare(params: &GradeParameters, cfg: &CompareConfig) -> (GradeParameters, bool) {
    match cfg.source {
        CompareSource::Original | CompareSource::Reference => (
            GradeParameters::original(),
            cfg.source == CompareSource::Reference,
        ),
        CompareSource::Stage1 => (params.stage1_only(), false),
        CompareSource::Previous => (
            cfg.previous
                .clone()
                .unwrap_or_else(GradeParameters::original),
            false,
        ),
    }
}

/// GPU compare uniforms for `cfg`.
pub fn gpu_compare(cfg: &CompareConfig) -> CompareParams {
    CompareParams {
        enabled: cfg.enabled,
        vertical: cfg.vertical,
        position: cfg.position,
        b_uses_reference: cfg.source == CompareSource::Reference,
    }
}

/// Composite two equally sized renders with a wipe at `position` (0..1) in
/// image space. Pixels whose centre lies before the split come from `a`.
pub fn composite_split(a: &Image, b: &Image, vertical: bool, position: f32) -> Image {
    debug_assert_eq!((a.width, a.height), (b.width, b.height));
    let pos = position.clamp(0.0, 1.0);
    let mut out = a.clone();
    for y in 0..a.height {
        for x in 0..a.width {
            let in_a = if vertical {
                (x as f32 + 0.5) < pos * a.width as f32
            } else {
                (y as f32 + 0.5) < pos * a.height as f32
            };
            if !in_a {
                let i = (y as usize * a.width as usize + x as usize) * 4;
                out.rgba[i..i + 4].copy_from_slice(&b.rgba[i..i + 4]);
            }
        }
    }
    out
}

fn bytes<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_bytes(v)
}

/// A scope raster: RGBA8, non-premultiplied, alpha 0 where empty.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
pub struct ScopeRaster {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, sRGB-encoded.
    #[serde(serialize_with = "bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub rgba: Vec<u8>,
}

impl From<ScopeImage> for ScopeRaster {
    fn from(s: ScopeImage) -> Self {
        Self {
            width: s.width,
            height: s.height,
            rgba: s.rgba,
        }
    }
}

/// Requested scopes; absent members were not requested.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
pub struct ScopesResult {
    /// Luma waveform.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waveform: Option<ScopeRaster>,
    /// RGB parade.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parade: Option<ScopeRaster>,
    /// Vectorscope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vectorscope: Option<ScopeRaster>,
    /// 256-bin histograms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub histogram: Option<Histogram>,
    /// Broadcast-legal report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal: Option<LegalReport>,
}

/// Build the requested scopes from output-encoded `pixels` (`w × h`).
/// `kinds` entries: `waveform`, `parade`, `vectorscope`, `histogram`, `legal`.
pub fn assemble_scopes(
    pixels: &[[f32; 3]],
    w: u32,
    h: u32,
    kinds: &[String],
) -> ApiResult<ScopesResult> {
    let mut r = ScopesResult::default();
    for k in kinds {
        match k.as_str() {
            "waveform" => {
                r.waveform =
                    Some(waveform_luma(pixels, w, h, WAVEFORM_SIZE.0, WAVEFORM_SIZE.1).into())
            }
            "parade" => {
                r.parade = Some(rgb_parade(pixels, w, h, PARADE_SIZE.0, PARADE_SIZE.1).into())
            }
            "vectorscope" => r.vectorscope = Some(vectorscope(pixels, VECTORSCOPE_SIZE).into()),
            "histogram" => r.histogram = Some(histogram(pixels)),
            "legal" => r.legal = Some(broadcast_legal(pixels, LEGAL_TOLERANCE)),
            other => return Err(ApiError::new(format!("unknown scope kind {other:?}"))),
        }
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, c: [u8; 4]) -> Image {
        Image::new(c.repeat((w * h) as usize), w, h).unwrap()
    }

    #[test]
    fn downscale_preserves_flat_color_and_aspect() {
        let d = box_downscale(&solid(100, 50, [10, 20, 30, 255]), 20, 20);
        assert_eq!((d.width, d.height), (20, 10));
        assert!(d.rgba.chunks(4).all(|p| p == [10, 20, 30, 255]));
        let same = box_downscale(&solid(8, 8, [1, 2, 3, 4]), 64, 64);
        assert_eq!((same.width, same.height), (8, 8));
    }

    #[test]
    fn downscale_averages_blocks() {
        let mut img = solid(2, 1, [0, 0, 0, 255]);
        img.rgba[0] = 200;
        let d = box_downscale(&img, 1, 1);
        assert_eq!(d.rgba[0], 100);
    }

    #[test]
    fn resample_changes_size() {
        let r = resample_nearest(&solid(3, 3, [9, 9, 9, 255]), 5, 2);
        assert_eq!((r.width, r.height, r.rgba.len()), (5, 2, 40));
    }

    #[test]
    fn compare_sides_resolve() {
        let mut p = GradeParameters::IDENTITY;
        p.stage1.exposure = 1.0;
        p.stage2.contrast = 30.0;
        let mut cfg = CompareConfig::default();
        assert_eq!(
            resolve_compare(&p, &cfg),
            (GradeParameters::original(), false)
        );
        cfg.source = CompareSource::Stage1;
        assert_eq!(resolve_compare(&p, &cfg).0, p.stage1_only());
        cfg.source = CompareSource::Reference;
        assert_eq!(
            resolve_compare(&p, &cfg),
            (GradeParameters::original(), true)
        );
        assert!(gpu_compare(&cfg).b_uses_reference);
        cfg.source = CompareSource::Previous;
        cfg.previous = Some(p.clone());
        assert_eq!(resolve_compare(&p, &cfg).0, p);
    }

    #[test]
    fn split_composites_by_position() {
        let a = solid(4, 2, [1, 1, 1, 255]);
        let b = solid(4, 2, [2, 2, 2, 255]);
        let v = composite_split(&a, &b, true, 0.5);
        assert_eq!(v.rgba[0], 1);
        assert_eq!(v.rgba[4 * 3], 2);
        let h = composite_split(&a, &b, false, 0.5);
        assert_eq!(h.rgba[0], 1);
        assert_eq!(h.rgba[4 * 4], 2);
    }

    #[test]
    fn scopes_assemble_and_reject_unknown() {
        let px = vec![[0.5f32, 0.4, 0.3]; 64];
        let kinds: Vec<String> = ["waveform", "parade", "vectorscope", "histogram", "legal"]
            .map(String::from)
            .into();
        let r = assemble_scopes(&px, 8, 8, &kinds).unwrap();
        assert_eq!(r.waveform.as_ref().unwrap().rgba.len(), 256 * 128 * 4);
        assert_eq!(r.parade.as_ref().unwrap().width, 384);
        assert_eq!(r.vectorscope.as_ref().unwrap().height, 256);
        assert!(r.histogram.is_some() && r.legal.is_some());
        assert!(assemble_scopes(&px, 8, 8, &["nope".into()]).is_err());
    }
}
