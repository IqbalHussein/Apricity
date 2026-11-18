//! The web `Engine`: owns the canvas, the WebGPU renderer (or the CPU
//! fallback) and an [`EngineState`]. Only compiled for `wasm32`; everything
//! it decides lives in [`crate::state`] / [`crate::compose`].

use apricity_render::gpu::{place, GpuRenderer, ViewParams};
use apricity_render::Overlay;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlVideoElement, ImageBitmap, ImageData,
};

use crate::api::{ApiError, BackendKind};
use crate::compose::CompareConfig;
use crate::state::EngineState;
use crate::wasm::{from_js, scopes_to_js, to_js};

struct GpuBackend {
    _instance: wgpu::Instance,
    renderer: GpuRenderer,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

struct CpuBackend {
    ctx: CanvasRenderingContext2d,
    scratch: HtmlCanvasElement,
    scratch_ctx: CanvasRenderingContext2d,
}

enum Backend {
    Gpu(Box<GpuBackend>),
    Cpu(CpuBackend),
}

fn js_err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

async fn init_gpu(canvas: &HtmlCanvasElement) -> Result<GpuBackend, String> {
    let window = web_sys::window().ok_or("no window")?;
    let gpu = js_sys::Reflect::get(&window.navigator(), &JsValue::from_str("gpu"))
        .map_err(|_| "navigator.gpu unavailable")?;
    if gpu.is_undefined() || gpu.is_null() {
        return Err("WebGPU is not supported by this browser".into());
    }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("apricity device"),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| e.to_string())?;
    let caps = surface.get_capabilities(&adapter);
    // The shader writes display-encoded values, so the surface must not re-encode.
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| {
            matches!(
                f,
                wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
            )
        })
        .ok_or("no non-sRGB canvas format")?;
    let mut config = surface
        .get_default_config(&adapter, canvas.width().max(1), canvas.height().max(1))
        .ok_or("surface not supported by adapter")?;
    config.format = format;
    config.alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        wgpu::CompositeAlphaMode::Opaque
    } else {
        wgpu::CompositeAlphaMode::Auto
    };
    surface.configure(&device, &config);
    let renderer = GpuRenderer::new(device, queue, format);
    Ok(GpuBackend {
        _instance: instance,
        renderer,
        surface,
        config,
    })
}

fn context_2d(c: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d, String> {
    c.get_context("2d")
        .ok()
        .flatten()
        .and_then(|o| o.dyn_into::<CanvasRenderingContext2d>().ok())
        .ok_or_else(|| "2D canvas context unavailable".to_owned())
}

fn init_cpu(canvas: &HtmlCanvasElement) -> Result<CpuBackend, String> {
    let ctx = context_2d(canvas)?;
    let scratch = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .create_element("canvas")
        .map_err(|_| "cannot create canvas")?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| "not a canvas")?;
    let scratch_ctx = context_2d(&scratch)?;
    Ok(CpuBackend {
        ctx,
        scratch,
        scratch_ctx,
    })
}

/// The preview/export renderer bound to one canvas.
#[wasm_bindgen]
pub struct Engine {
    canvas: HtmlCanvasElement,
    state: EngineState,
    backend: Backend,
}

#[wasm_bindgen]
impl Engine {
    /// Bind to `canvas`, preferring WebGPU unless `force_cpu`. Falls back to
    /// the CPU renderer on any WebGPU failure; never throws merely because
    /// WebGPU is missing.
    pub async fn create(canvas: HtmlCanvasElement, force_cpu: bool) -> Result<Engine, JsError> {
        let mut gpu = None;
        if !force_cpu {
            match init_gpu(&canvas).await {
                Ok(g) => gpu = Some(g),
                Err(e) => {
                    web_sys::console::warn_1(&format!("WebGPU unavailable, using CPU: {e}").into())
                }
            }
        }
        let backend = match gpu {
            Some(g) => Backend::Gpu(Box::new(g)),
            None => Backend::Cpu(init_cpu(&canvas).map_err(js_err)?),
        };
        Ok(Engine {
            canvas,
            state: EngineState::default(),
            backend,
        })
    }

    /// `"webgpu"` or `"cpu"`.
    #[wasm_bindgen(unchecked_return_type = "BackendKind")]
    pub fn backend(&self) -> Result<JsValue, JsError> {
        to_js(&match self.backend {
            Backend::Gpu(_) => BackendKind::Webgpu,
            Backend::Cpu(_) => BackendKind::Cpu,
        })
    }

    /// Set the preview proxy: sRGB RGBA8 `w × h` pixels of an image that is
    /// `full_width × full_height` at full resolution (grain scales by `full_width / w`).
    #[wasm_bindgen(js_name = setSourceRgba)]
    pub fn set_source_rgba(
        &mut self,
        rgba: &[u8],
        w: u32,
        h: u32,
        full_width: u32,
        _full_height: u32,
    ) -> Result<(), JsError> {
        self.state.set_source_rgba(rgba, w, h, full_width)?;
        if let Backend::Gpu(g) = &mut self.backend {
            g.renderer.set_source_rgba8(rgba, w, h);
        }
        Ok(())
    }

    /// Upload the current frame of `video` (GPU only; call once per frame).
    #[wasm_bindgen(js_name = setSourceVideo)]
    pub fn set_source_video(&mut self, video: HtmlVideoElement) -> Result<(), JsError> {
        let (w, h) = (video.video_width(), video.video_height());
        self.upload_external(wgpu::ExternalImageSource::HTMLVideoElement(video), w, h)
    }

    /// Upload `bitmap` (GPU only).
    #[wasm_bindgen(js_name = setSourceBitmap)]
    pub fn set_source_bitmap(&mut self, bitmap: ImageBitmap) -> Result<(), JsError> {
        let (w, h) = (bitmap.width(), bitmap.height());
        self.upload_external(wgpu::ExternalImageSource::ImageBitmap(bitmap), w, h)
    }

    fn upload_external(
        &mut self,
        source: wgpu::ExternalImageSource,
        w: u32,
        h: u32,
    ) -> Result<(), JsError> {
        let Backend::Gpu(g) = &mut self.backend else {
            return Err(js_err(
                "CPU backend cannot take external sources; pass RGBA with setSourceRgba",
            ));
        };
        if w == 0 || h == 0 {
            return Err(js_err("source has no pixels yet"));
        }
        g.renderer.set_source_external(
            &wgpu::CopyExternalImageSourceInfo {
                source,
                origin: wgpu::Origin2d::ZERO,
                flip_y: false,
            },
            w,
            h,
        );
        self.state.set_source_external(w, h);
        Ok(())
    }

    /// Replace only the copy used by scopes and analysis (sRGB RGBA8).
    #[wasm_bindgen(js_name = setScopeProxyRgba)]
    pub fn set_scope_proxy_rgba(&mut self, rgba: &[u8], w: u32, h: u32) -> Result<(), JsError> {
        Ok(self.state.set_scope_proxy(rgba, w, h)?)
    }

    /// Set the imported reference image (sRGB RGBA8).
    #[wasm_bindgen(js_name = setReferenceRgba)]
    pub fn set_reference_rgba(&mut self, rgba: &[u8], w: u32, h: u32) -> Result<(), JsError> {
        self.state.set_reference(rgba, w, h)?;
        if let Backend::Gpu(g) = &mut self.backend {
            g.renderer.set_reference_rgba8(rgba, w, h);
        }
        Ok(())
    }

    /// Drop the reference image.
    #[wasm_bindgen(js_name = clearReference)]
    pub fn clear_reference(&mut self) {
        self.state.clear_reference();
        if let Backend::Gpu(g) = &mut self.backend {
            g.renderer.clear_reference();
        }
    }

    /// Set the grade; sanitized.
    #[wasm_bindgen(js_name = setParams)]
    pub fn set_params(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "GradeParameters")] params: JsValue,
    ) -> Result<(), JsError> {
        self.state.set_params(&from_js(params)?);
        Ok(())
    }

    /// Configure the wipe comparison.
    #[wasm_bindgen(js_name = setCompare)]
    pub fn set_compare(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "CompareConfig")] config: JsValue,
    ) -> Result<(), JsError> {
        let c: CompareConfig = from_js(config)?;
        self.state.set_compare(c);
        Ok(())
    }

    /// Zoom (1 = fit), pan (fraction of the displayed image) and `exact`
    /// (fill the canvas, used for full-resolution export).
    #[wasm_bindgen(js_name = setView)]
    pub fn set_view(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "ViewParams")] view: JsValue,
    ) -> Result<(), JsError> {
        let v: ViewParams = from_js(view)?;
        self.state.set_view(v);
        Ok(())
    }

    /// Preview overlay; `zebra_threshold` is output-encoded luma in [0, 1].
    #[wasm_bindgen(js_name = setOverlay)]
    pub fn set_overlay(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Overlay")] kind: JsValue,
        zebra_threshold: f32,
    ) -> Result<(), JsError> {
        let o: Overlay = from_js(kind)?;
        self.state.set_overlay(o, zebra_threshold);
        Ok(())
    }

    /// Set (or clear with `null`) the creative LUT from `.cube` text.
    #[wasm_bindgen(js_name = setLut, unchecked_return_type = "LutInfo | null")]
    pub fn set_lut(&mut self, text: Option<String>) -> Result<JsValue, JsError> {
        let info = self.state.set_lut(text.as_deref())?;
        if let Backend::Gpu(g) = &mut self.backend {
            g.renderer.set_lut(self.state.lut());
        }
        to_js(&info)
    }

    /// Letterbox color, each channel 0..1.
    #[wasm_bindgen(js_name = setBackground)]
    pub fn set_background(&mut self, r: f64, g: f64, b: f64) {
        self.state.set_background([r, g, b]);
    }

    /// Resize the drawing surface to `width × height` device pixels.
    pub fn resize(&mut self, width: u32, height: u32) {
        let (w, h) = (width.max(1), height.max(1));
        self.canvas.set_width(w);
        self.canvas.set_height(h);
        if let Backend::Gpu(g) = &mut self.backend {
            g.config.width = w;
            g.config.height = h;
            g.surface.configure(g.renderer.device(), &g.config);
        }
    }

    /// Draw the current frame. A no-op until a source has been set.
    pub fn render(&mut self) -> Result<(), JsError> {
        match &mut self.backend {
            Backend::Gpu(g) => {
                let Some((a, b, cmp)) = self.state.gpu_uniforms() else {
                    return Ok(());
                };
                g.renderer.set_grades(&a, b.as_ref());
                let frame = match g.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(t) => t,
                    wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                        g.renderer.queue().present(t);
                        g.surface.configure(g.renderer.device(), &g.config);
                        return Ok(());
                    }
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        g.surface.configure(g.renderer.device(), &g.config);
                        return Ok(());
                    }
                    _ => return Ok(()),
                };
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                g.renderer
                    .draw(
                        &view,
                        (g.config.width, g.config.height),
                        &self.state.view(),
                        &cmp,
                        self.state.background(),
                    )
                    .map_err(js_err)?;
                g.renderer.queue().present(frame);
                Ok(())
            }
            Backend::Cpu(c) => {
                let (cw, ch) = (self.canvas.width(), self.canvas.height());
                let [r, g, b] = self.state.background().map(|v| (v * 255.0).round() as u8);
                c.ctx.set_fill_style_str(&format!("rgb({r},{g},{b})"));
                c.ctx.fill_rect(0.0, 0.0, cw as f64, ch as f64);
                let Some(dims) = self.state.source_dims() else {
                    return Ok(());
                };
                let view = self.state.view();
                let Some(frame) = self.state.cpu_frame() else {
                    return Ok(());
                };
                if c.scratch.width() != frame.width || c.scratch.height() != frame.height {
                    c.scratch.set_width(frame.width);
                    c.scratch.set_height(frame.height);
                }
                let data = ImageData::new_with_u8_clamped_array_and_sh(
                    Clamped(&frame.rgba),
                    frame.width,
                    frame.height,
                )
                .map_err(|_| js_err("cannot create ImageData"))?;
                c.scratch_ctx
                    .put_image_data(&data, 0.0, 0.0)
                    .map_err(|_| js_err("putImageData failed"))?;
                let p = place(dims, (cw, ch), &view).rect_px;
                c.ctx.set_image_smoothing_enabled(true);
                c.ctx
                    .draw_image_with_html_canvas_element_and_dw_and_dh(
                        &c.scratch,
                        p[0] as f64,
                        p[1] as f64,
                        (p[2] - p[0]) as f64,
                        (p[3] - p[1]) as f64,
                    )
                    .map_err(|_| js_err("drawImage failed"))?;
                Ok(())
            }
        }
    }

    /// Requested scopes (`waveform`, `parade`, `vectorscope`, `histogram`,
    /// `legal`) of the graded scope copy.
    #[wasm_bindgen(unchecked_return_type = "ScopesResult")]
    pub fn scopes(&self, kinds: Vec<String>) -> Result<JsValue, JsError> {
        scopes_to_js(&self.state.scopes(&kinds)?)
    }

    /// Stage 1 auto-analysis of the scope copy with the current input profile.
    #[wasm_bindgen(js_name = analyzeStage1, unchecked_return_type = "Stage1Analysis")]
    pub fn analyze_stage1(&self) -> Result<JsValue, JsError> {
        match self.state.analyze_stage1() {
            Some(a) => to_js(&a),
            None => Err(js_err(ApiError("no image loaded".into()))),
        }
    }
}
