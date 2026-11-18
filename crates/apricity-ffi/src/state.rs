//! Backend-independent state of the web `Engine`: the proxy pixels, grade,
//! comparison, LUT and the CPU frame/scopes/analysis derived from them.
//!
//! The GPU path only needs [`EngineState::gpu_uniforms`]; the CPU path uses
//! [`EngineState::cpu_frame`]. Both derive from the same fields, so the two
//! backends cannot disagree about what is being shown.

use apricity_core::analysis::Stage1Analysis;
use apricity_core::validate::sanitize;
use apricity_core::GradeParameters;
use apricity_lut::Lut3d;
use apricity_render::gpu::{CompareParams, ViewParams};
use apricity_render::{cpu, prepare, GradeUniforms, Overlay, RenderContext};

use crate::api::{analyze_image, lut_info, lut_parse_text, scale_for, ApiResult, LutInfo};
use crate::compose::{
    assemble_scopes, box_downscale, composite_split, gpu_compare, resample_nearest,
    resolve_compare, CompareConfig, CompareSource, Image, ScopesResult,
};

/// Longest side of the CPU-preview render, in pixels.
pub const CPU_PREVIEW_MAX_SIDE: u32 = 640;
/// Widest scope copy, in pixels.
pub const SCOPE_MAX_WIDTH: u32 = 320;

/// Everything the engine knows, independent of GPU or canvas.
#[derive(Debug)]
pub struct EngineState {
    source: Option<Image>,
    source_dims: Option<(u32, u32)>,
    source_scale: f32,
    scope_proxy: Option<Image>,
    reference: Option<Image>,
    params: GradeParameters,
    compare: CompareConfig,
    view: ViewParams,
    overlay: Overlay,
    zebra_threshold: f32,
    lut: Option<Lut3d>,
    background: [f64; 3],
    cpu_source: Option<Image>,
    frame: Option<Image>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            source: None,
            source_dims: None,
            source_scale: 1.0,
            scope_proxy: None,
            reference: None,
            params: GradeParameters::IDENTITY,
            compare: CompareConfig::default(),
            view: ViewParams::default(),
            overlay: Overlay::None,
            zebra_threshold: 0.95,
            lut: None,
            background: [0.0; 3],
            cpu_source: None,
            frame: None,
        }
    }
}

impl EngineState {
    /// Store the preview proxy (`w × h`, sRGB RGBA8) of a `full_width × full_height` image.
    pub fn set_source_rgba(
        &mut self,
        rgba: &[u8],
        w: u32,
        h: u32,
        full_width: u32,
    ) -> ApiResult<()> {
        let img = Image::new(rgba.to_vec(), w, h)?;
        self.source_dims = Some((w, h));
        self.source_scale = scale_for(full_width as f64, w);
        self.source = Some(img);
        self.cpu_source = None;
        self.frame = None;
        Ok(())
    }

    /// Record an externally uploaded source (video/bitmap) of `w × h` with no CPU pixels.
    pub fn set_source_external(&mut self, w: u32, h: u32) {
        self.source = None;
        self.source_dims = Some((w, h));
        self.source_scale = 1.0;
        self.cpu_source = None;
        self.frame = None;
    }

    /// Replace only the copy used by scopes and analysis.
    pub fn set_scope_proxy(&mut self, rgba: &[u8], w: u32, h: u32) -> ApiResult<()> {
        self.scope_proxy = Some(Image::new(rgba.to_vec(), w, h)?);
        Ok(())
    }

    /// Store the imported reference image.
    pub fn set_reference(&mut self, rgba: &[u8], w: u32, h: u32) -> ApiResult<()> {
        self.reference = Some(Image::new(rgba.to_vec(), w, h)?);
        self.frame = None;
        Ok(())
    }

    /// Drop the reference image.
    pub fn clear_reference(&mut self) {
        self.reference = None;
        self.frame = None;
    }

    /// Set the grade (sanitized).
    pub fn set_params(&mut self, p: &GradeParameters) {
        self.params = sanitize(p);
        self.frame = None;
    }

    /// Set the comparison configuration.
    pub fn set_compare(&mut self, c: CompareConfig) {
        self.compare = c;
        self.frame = None;
    }

    /// Set zoom/pan/exact placement.
    pub fn set_view(&mut self, v: ViewParams) {
        self.view = v;
    }

    /// Set the preview overlay and zebra threshold (output-encoded luma, 0..1).
    pub fn set_overlay(&mut self, o: Overlay, zebra_threshold: f32) {
        self.overlay = o;
        self.zebra_threshold = zebra_threshold.clamp(0.0, 1.0);
        self.frame = None;
    }

    /// Set the 3D LUT from `.cube` text, or clear it.
    pub fn set_lut(&mut self, text: Option<&str>) -> ApiResult<Option<LutInfo>> {
        let lut = text.map(lut_parse_text).transpose()?;
        let info = lut.as_ref().map(lut_info);
        self.lut = lut;
        self.frame = None;
        Ok(info)
    }

    /// Letterbox color, linear 0..1 per channel (sRGB-encoded as drawn).
    pub fn set_background(&mut self, rgb: [f64; 3]) {
        self.background = rgb.map(|c| c.clamp(0.0, 1.0));
    }

    /// Letterbox color.
    pub fn background(&self) -> [f64; 3] {
        self.background
    }

    /// Current view placement.
    pub fn view(&self) -> ViewParams {
        self.view
    }

    /// Source size in pixels, if a source is set.
    pub fn source_dims(&self) -> Option<(u32, u32)> {
        self.source_dims
    }

    /// Current LUT, if any.
    pub fn lut(&self) -> Option<&Lut3d> {
        self.lut.as_ref()
    }

    /// Current (sanitized) grade.
    pub fn params(&self) -> &GradeParameters {
        &self.params
    }

    fn ctx(&self, dims: (u32, u32), scale: f32, overlay: Overlay) -> RenderContext {
        RenderContext {
            image_size: [dims.0 as f32, dims.1 as f32],
            source_scale: scale,
            output: apricity_core::ColorSpace::Srgb,
            overlay,
            zebra_threshold: self.zebra_threshold,
            lut_size: self.lut.as_ref().map_or(0, |l| l.size as u32),
        }
    }

    /// Uniforms for the GPU draw: side A, side B (when comparing) and the
    /// compare parameters. `None` until a source size is known.
    pub fn gpu_uniforms(&self) -> Option<(GradeUniforms, Option<GradeUniforms>, CompareParams)> {
        let dims = self.source_dims?;
        let ctx = self.ctx(dims, self.source_scale, self.overlay);
        let a = prepare(&self.params, &ctx);
        let b = self.compare.enabled.then(|| {
            let (bp, _) = resolve_compare(&self.params, &self.compare);
            prepare(&bp, &ctx)
        });
        Some((a, b, gpu_compare(&self.compare)))
    }

    fn render_side(&self, src: &Image, p: &GradeParameters, scale: f32) -> Image {
        let ctx = self.ctx((src.width, src.height), scale, self.overlay);
        let u = prepare(p, &ctx);
        Image {
            rgba: cpu::render_rgba8(&u, self.lut.as_ref(), &src.rgba, src.width, src.height),
            width: src.width,
            height: src.height,
        }
    }

    /// The CPU-fallback frame: the source downscaled to
    /// [`CPU_PREVIEW_MAX_SIDE`], graded, and (when comparing) split-composited
    /// in image space. Cached until a pixel-affecting input changes. `None`
    /// without CPU source pixels.
    pub fn cpu_frame(&mut self) -> Option<&Image> {
        if self.frame.is_none() {
            let src = self.source.as_ref()?;
            if self.cpu_source.is_none() {
                self.cpu_source = Some(box_downscale(
                    src,
                    CPU_PREVIEW_MAX_SIDE,
                    CPU_PREVIEW_MAX_SIDE,
                ));
            }
            let small = self.cpu_source.as_ref()?;
            let scale = scale_for(src.width as f64 * self.source_scale as f64, small.width);
            let a = self.render_side(small, &self.params, scale);
            let frame = if self.compare.enabled {
                let (bp, uses_ref) = resolve_compare(&self.params, &self.compare);
                let b_src = match (&self.reference, uses_ref) {
                    (Some(r), true) => resample_nearest(r, small.width, small.height),
                    _ => small.clone(),
                };
                let b = self.render_side(&b_src, &bp, scale);
                composite_split(&a, &b, self.compare.vertical, self.compare.position)
            } else {
                a
            };
            self.frame = Some(frame);
        }
        self.frame.as_ref()
    }

    fn scope_image(&self) -> Option<&Image> {
        self.scope_proxy.as_ref().or(self.source.as_ref())
    }

    /// Requested scopes of the current grade on the scope copy (no overlay).
    pub fn scopes(&self, kinds: &[String]) -> ApiResult<ScopesResult> {
        let Some(src) = self.scope_image() else {
            return Ok(ScopesResult::default());
        };
        let small = box_downscale(src, SCOPE_MAX_WIDTH, u32::MAX);
        let scale = scale_for(self.full_width(src), small.width);
        let ctx = self.ctx((small.width, small.height), scale, Overlay::None);
        let u = prepare(&self.params, &ctx);
        let px = cpu::render_float(
            &u,
            self.lut.as_ref(),
            &small.rgba,
            small.width,
            small.height,
        );
        assemble_scopes(&px, small.width, small.height, kinds)
    }

    fn full_width(&self, scope_src: &Image) -> f64 {
        match self.source_dims {
            Some((w, _)) => w as f64 * self.source_scale as f64,
            None => scope_src.width as f64,
        }
    }

    /// Stage 1 auto-analysis of the scope copy with the current input profile.
    pub fn analyze_stage1(&self) -> Option<Stage1Analysis> {
        self.scope_image()
            .map(|i| analyze_image(i, &self.params.stage1))
    }

    /// Whether the compare source needs the reference texture.
    pub fn compare_uses_reference(&self) -> bool {
        self.compare.source == CompareSource::Reference
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: u32, h: u32, v: u8) -> Vec<u8> {
        [v, v, v, 255].repeat((w * h) as usize)
    }

    #[test]
    fn cpu_frame_downscales_and_caches() {
        let mut s = EngineState::default();
        assert!(s.cpu_frame().is_none());
        s.set_source_rgba(&flat(1280, 720, 100), 1280, 720, 4000)
            .unwrap();
        let f = s.cpu_frame().unwrap();
        assert_eq!((f.width, f.height), (640, 360));
        s.set_params(&{
            let mut p = GradeParameters::IDENTITY;
            p.stage2.exposure = 1.0;
            p
        });
        let bright = s.cpu_frame().unwrap().rgba[0];
        assert!(bright > 100);
    }

    #[test]
    fn cpu_compare_splits_graded_and_original() {
        let mut s = EngineState::default();
        s.set_source_rgba(&flat(64, 32, 100), 64, 32, 64).unwrap();
        let mut p = GradeParameters::IDENTITY;
        p.stage2.exposure = 1.0;
        s.set_params(&p);
        s.set_compare(CompareConfig {
            enabled: true,
            ..CompareConfig::default()
        });
        let f = s.cpu_frame().unwrap().clone();
        assert!(f.rgba[0] > 100);
        let last = f.rgba.len() - 4;
        assert!((f.rgba[last] as i32 - 100).abs() <= 1);
    }

    #[test]
    fn reference_compare_uses_reference_pixels() {
        let mut s = EngineState::default();
        s.set_source_rgba(&flat(8, 8, 100), 8, 8, 8).unwrap();
        s.set_reference(&flat(4, 4, 200), 4, 4).unwrap();
        s.set_compare(CompareConfig {
            enabled: true,
            source: CompareSource::Reference,
            position: 0.0,
            ..CompareConfig::default()
        });
        assert!((s.cpu_frame().unwrap().rgba[0] as i32 - 200).abs() <= 1);
    }

    #[test]
    fn gpu_uniforms_follow_compare_flag() {
        let mut s = EngineState::default();
        assert!(s.gpu_uniforms().is_none());
        s.set_source_external(10, 10);
        assert!(s.gpu_uniforms().unwrap().1.is_none());
        s.set_compare(CompareConfig {
            enabled: true,
            ..CompareConfig::default()
        });
        let (_, b, c) = s.gpu_uniforms().unwrap();
        assert!(b.is_some() && c.enabled);
    }

    #[test]
    fn scopes_and_analysis_use_scope_proxy() {
        let mut s = EngineState::default();
        assert_eq!(
            s.scopes(&["legal".into()]).unwrap(),
            ScopesResult::default()
        );
        s.set_source_external(1920, 1080);
        s.set_scope_proxy(&flat(16, 9, 120), 16, 9).unwrap();
        let r = s.scopes(&["histogram".into(), "legal".into()]).unwrap();
        assert!(r.histogram.is_some() && r.legal.unwrap().legal);
        assert!(s.analyze_stage1().is_some());
    }

    #[test]
    fn lut_setter_reports_info() {
        let mut s = EngineState::default();
        let text = crate::api::lut_bake(
            &GradeParameters::IDENTITY,
            5,
            apricity_render::BakeScope::Creative,
            None,
        );
        assert_eq!(s.set_lut(Some(&text)).unwrap().unwrap().size, 5);
        assert!(s.set_lut(Some("bad")).is_err());
        assert_eq!(s.set_lut(None).unwrap(), None);
    }
}
