//! wgpu implementation of the pipeline. One WGSL source (the files in
//! `shaders/`, prefixed by [`crate::uniforms::wgsl_constants`]) runs on
//! WebGPU, Vulkan, Metal and DX12.
//!
//! The renderer draws the source image as a quad into any color target
//! (a canvas surface for preview, an offscreen texture for export and
//! tests). Wipe comparison is two scissored draws of the same quad with
//! different uniforms (and, for "imported reference", a different source
//! texture), so the shader itself has no comparison logic.

use std::borrow::Cow;

use apricity_lut::Lut3d;
use bytemuck::{Pod, Zeroable};

use crate::uniforms::{wgsl_constants, GradeUniforms};

/// Concatenated WGSL source for the pipeline.
pub fn shader_source() -> String {
    let mut s = wgsl_constants();
    for part in [
        include_str!("../shaders/common.wgsl"),
        include_str!("../shaders/stage1.wgsl"),
        include_str!("../shaders/stage2.wgsl"),
        include_str!("../shaders/output.wgsl"),
        include_str!("../shaders/main.wgsl"),
    ] {
        s.push_str(part);
        s.push('\n');
    }
    s
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ViewUniforms {
    quad: [f32; 4],
    image: [f32; 4],
}

/// How the image is placed in the target.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ViewParams {
    /// Zoom relative to "fit" (1.0 = whole image visible, letterboxed).
    pub zoom: f32,
    /// Pan offset as a fraction of the displayed image size; (0, 0) is
    /// centered.
    pub pan: [f32; 2],
    /// `true` to fill the target exactly (export / tests): no letterbox,
    /// exact texel loads.
    pub exact: bool,
}

impl Default for ViewParams {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: [0.0, 0.0],
            exact: false,
        }
    }
}

/// Wipe comparison.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CompareParams {
    /// Draw a split.
    pub enabled: bool,
    /// `true`: split line is vertical (left/right); `false`: horizontal.
    pub vertical: bool,
    /// Split position as a fraction of the image (0..1).
    pub position: f32,
    /// Side B samples the reference texture instead of the source.
    pub b_uses_reference: bool,
}

impl Default for CompareParams {
    fn default() -> Self {
        Self {
            enabled: false,
            vertical: true,
            position: 0.5,
            b_uses_reference: false,
        }
    }
}

/// Image placement in target pixels and NDC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Screen rect of the image in target pixels: x0, y0, x1, y1.
    pub rect_px: [f32; 4],
    /// NDC quad: x0, y0 (top-left), x1, y1.
    pub quad_ndc: [f32; 4],
}

/// Compute where an `image` sized image lands in a `target` sized surface.
pub fn place(image: (u32, u32), target: (u32, u32), view: &ViewParams) -> Placement {
    let (iw, ih) = (image.0.max(1) as f32, image.1.max(1) as f32);
    let (tw, th) = (target.0.max(1) as f32, target.1.max(1) as f32);
    let (w, h) = if view.exact {
        (tw, th)
    } else {
        let fit = (tw / iw).min(th / ih);
        (iw * fit * view.zoom, ih * fit * view.zoom)
    };
    let cx = tw * 0.5 + view.pan[0] * w;
    let cy = th * 0.5 + view.pan[1] * h;
    let rect = [cx - w * 0.5, cy - h * 0.5, cx + w * 0.5, cy + h * 0.5];
    let ndc_x = |x: f32| x / tw * 2.0 - 1.0;
    let ndc_y = |y: f32| 1.0 - y / th * 2.0;
    Placement {
        rect_px: rect,
        quad_ndc: [
            ndc_x(rect[0]),
            ndc_y(rect[1]),
            ndc_x(rect[2]),
            ndc_y(rect[3]),
        ],
    }
}

struct SourceTexture {
    view: wgpu::TextureView,
    size: (u32, u32),
    texture: wgpu::Texture,
}

/// The wgpu renderer. Owns the pipeline and its resources; draws into a
/// caller-supplied target view.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    grade_a: wgpu::Buffer,
    grade_b: wgpu::Buffer,
    view_buf: wgpu::Buffer,
    source: Option<SourceTexture>,
    reference: Option<SourceTexture>,
    lut: wgpu::TextureView,
    lut_size: u32,
    format: wgpu::TextureFormat,
}

/// Errors from the GPU renderer.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    /// No source image uploaded.
    #[error("no source image")]
    NoSource,
    /// Adapter or device unavailable.
    #[error("GPU unavailable: {0}")]
    Unavailable(String),
}

impl GpuRenderer {
    /// Build the pipeline for a color target `format`.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("apricity pipeline"),
            source: wgpu::ShaderSource::Wgsl(Cow::Owned(shader_source())),
        });
        let uniform_entry = |binding: u32, vis: wgpu::ShaderStages| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: vis,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("apricity bgl"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::FRAGMENT),
                uniform_entry(1, wgpu::ShaderStages::VERTEX_FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("apricity pl"),
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("apricity render"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("apricity sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let ubuf = |label: &str, size: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let grade_a = ubuf("grade a", std::mem::size_of::<GradeUniforms>());
        let grade_b = ubuf("grade b", std::mem::size_of::<GradeUniforms>());
        let view_buf = ubuf("view", std::mem::size_of::<ViewUniforms>());
        let lut = Self::make_lut(&device, &queue, &Lut3d::identity(2));
        Self {
            device,
            queue,
            pipeline,
            layout,
            sampler,
            grade_a,
            grade_b,
            view_buf,
            source: None,
            reference: None,
            lut,
            lut_size: 0,
            format,
        }
    }

    /// Request an adapter + device (native or web) and build a renderer.
    pub async fn request(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
        format: wgpu::TextureFormat,
    ) -> Result<Self, GpuError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: surface,
                ..Default::default()
            })
            .await
            .map_err(|e| GpuError::Unavailable(e.to_string()))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("apricity device"),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|e| GpuError::Unavailable(e.to_string()))?;
        Ok(Self::new(device, queue, format))
    }

    /// The device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Target format the pipeline was built for.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Size of the bound LUT (0 = none).
    pub fn lut_size(&self) -> u32 {
        self.lut_size
    }

    /// Current source size, if any.
    pub fn source_size(&self) -> Option<(u32, u32)> {
        self.source.as_ref().map(|s| s.size)
    }

    fn make_source(&self, w: u32, h: u32) -> SourceTexture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("source"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        SourceTexture {
            view,
            size: (w, h),
            texture,
        }
    }

    fn write_rgba8(&self, tex: &SourceTexture, data: &[u8]) {
        let (w, h) = tex.size;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Upload an RGBA8 (encoded) source image.
    pub fn set_source_rgba8(&mut self, data: &[u8], w: u32, h: u32) {
        if self.source.as_ref().map(|s| s.size) != Some((w, h)) {
            self.source = Some(self.make_source(w, h));
        }
        let tex = self.source.as_ref().expect("just created");
        self.write_rgba8(tex, data);
    }

    /// Upload an RGBA8 reference image for "imported reference" compare.
    pub fn set_reference_rgba8(&mut self, data: &[u8], w: u32, h: u32) {
        if self.reference.as_ref().map(|s| s.size) != Some((w, h)) {
            self.reference = Some(self.make_source(w, h));
        }
        let tex = self.reference.as_ref().expect("just created");
        self.write_rgba8(tex, data);
    }

    /// Drop the reference image.
    pub fn clear_reference(&mut self) {
        self.reference = None;
    }

    /// Upload a browser image source (ImageBitmap, video element, canvas)
    /// without a round trip through WASM memory.
    #[cfg(target_arch = "wasm32")]
    pub fn set_source_external(
        &mut self,
        source: &wgpu::CopyExternalImageSourceInfo,
        w: u32,
        h: u32,
    ) {
        if self.source.as_ref().map(|s| s.size) != Some((w, h)) {
            self.source = Some(self.make_source(w, h));
        }
        let tex = self.source.as_ref().expect("just created");
        self.queue.copy_external_image_to_texture(
            source,
            wgpu::CopyExternalImageDestInfo {
                texture: &tex.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
                color_space: wgpu::PredefinedColorSpace::Srgb,
                premultiplied_alpha: false,
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }

    fn make_lut(device: &wgpu::Device, queue: &wgpu::Queue, lut: &Lut3d) -> wgpu::TextureView {
        let n = lut.size as u32;
        let size = wgpu::Extent3d {
            width: n,
            height: n,
            depth_or_array_layers: n,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("lut"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&lut.to_rgba_f32()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(16 * n),
                rows_per_image: Some(n),
            },
            size,
        );
        texture.create_view(&Default::default())
    }

    /// Bind a 3D LUT (or clear it with `None`).
    pub fn set_lut(&mut self, lut: Option<&Lut3d>) {
        match lut {
            Some(l) => {
                self.lut = Self::make_lut(&self.device, &self.queue, l);
                self.lut_size = l.size as u32;
            }
            None => {
                self.lut = Self::make_lut(&self.device, &self.queue, &Lut3d::identity(2));
                self.lut_size = 0;
            }
        }
    }

    /// Upload uniforms for side A (and side B when comparing).
    pub fn set_grades(&self, a: &GradeUniforms, b: Option<&GradeUniforms>) {
        self.queue
            .write_buffer(&self.grade_a, 0, bytemuck::bytes_of(a));
        if let Some(b) = b {
            self.queue
                .write_buffer(&self.grade_b, 0, bytemuck::bytes_of(b));
        }
    }

    fn bind_group(&self, grade: &wgpu::Buffer, src: &SourceTexture) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apricity bg"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grade.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.view_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&src.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.lut),
                },
            ],
        })
    }

    /// Record a draw into `target` (size `target_size`) and submit it.
    /// `background` clears the area outside the image.
    pub fn draw(
        &self,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
        view: &ViewParams,
        compare: &CompareParams,
        background: [f64; 3],
    ) -> Result<(), GpuError> {
        let src = self.source.as_ref().ok_or(GpuError::NoSource)?;
        let placement = place(src.size, target_size, view);
        let vu = ViewUniforms {
            quad: placement.quad_ndc,
            image: [
                src.size.0 as f32,
                src.size.1 as f32,
                if view.exact { 0.0 } else { 1.0 },
                0.0,
            ],
        };
        self.queue
            .write_buffer(&self.view_buf, 0, bytemuck::bytes_of(&vu));

        let bg_a = self.bind_group(&self.grade_a, src);
        let b_src = if compare.b_uses_reference {
            self.reference.as_ref().unwrap_or(src)
        } else {
            src
        };
        let bg_b = compare
            .enabled
            .then(|| self.bind_group(&self.grade_b, b_src));

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("apricity frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("apricity pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: background[0],
                            g: background[1],
                            b: background[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            let (tw, th) = (target_size.0 as f32, target_size.1 as f32);
            let scissor = |pass: &mut wgpu::RenderPass, x0: f32, y0: f32, x1: f32, y1: f32| {
                let cx0 = x0.clamp(0.0, tw).round() as u32;
                let cy0 = y0.clamp(0.0, th).round() as u32;
                let cx1 = x1.clamp(0.0, tw).round() as u32;
                let cy1 = y1.clamp(0.0, th).round() as u32;
                if cx1 > cx0 && cy1 > cy0 {
                    pass.set_scissor_rect(cx0, cy0, cx1 - cx0, cy1 - cy0);
                    true
                } else {
                    false
                }
            };
            match bg_b {
                None => {
                    pass.set_bind_group(0, &bg_a, &[]);
                    pass.draw(0..6, 0..1);
                }
                Some(bg_b) => {
                    let r = placement.rect_px;
                    let pos = compare.position.clamp(0.0, 1.0);
                    let (a_rect, b_rect) = if compare.vertical {
                        let sx = r[0] + (r[2] - r[0]) * pos;
                        ([0.0, 0.0, sx, th], [sx, 0.0, tw, th])
                    } else {
                        let sy = r[1] + (r[3] - r[1]) * pos;
                        ([0.0, 0.0, tw, sy], [0.0, sy, tw, th])
                    };
                    if scissor(&mut pass, a_rect[0], a_rect[1], a_rect[2], a_rect[3]) {
                        pass.set_bind_group(0, &bg_a, &[]);
                        pass.draw(0..6, 0..1);
                    }
                    if scissor(&mut pass, b_rect[0], b_rect[1], b_rect[2], b_rect[3]) {
                        pass.set_bind_group(0, &bg_b, &[]);
                        pass.draw(0..6, 0..1);
                    }
                }
            }
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    /// Render the source at its own resolution and read back RGBA8.
    /// Blocks on the device; native targets only (tests, CLI export).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn render_rgba8_blocking(&self) -> Result<Vec<u8>, GpuError> {
        let (w, h) = self.source.as_ref().ok_or(GpuError::NoSource)?.size;
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("readback target"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        self.draw(
            &view,
            (w, h),
            &ViewParams {
                exact: true,
                ..Default::default()
            },
            &CompareParams::default(),
            [0.0; 3],
        )?;
        let row = 4 * w;
        let padded =
            row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| GpuError::Unavailable(e.to_string()))?;
        let data = slice
            .get_mapped_range()
            .map_err(|e| GpuError::Unavailable(e.to_string()))?;
        let mut out = Vec::with_capacity((row * h) as usize);
        for y in 0..h {
            let start = (y * padded) as usize;
            out.extend_from_slice(&data[start..start + row as usize]);
        }
        drop(data);
        buffer.unmap();
        let bgra = matches!(
            self.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        if bgra {
            for px in out.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_letterboxes_and_centers() {
        let p = place((200, 100), (400, 400), &ViewParams::default());
        assert_eq!(p.rect_px, [0.0, 100.0, 400.0, 300.0]);
        assert_eq!(p.quad_ndc, [-1.0, 0.5, 1.0, -0.5]);
    }

    #[test]
    fn exact_fills_target() {
        let p = place(
            (200, 100),
            (200, 100),
            &ViewParams {
                exact: true,
                ..Default::default()
            },
        );
        assert_eq!(p.quad_ndc, [-1.0, 1.0, 1.0, -1.0]);
    }

    #[test]
    fn shader_source_contains_all_stages() {
        let s = shader_source();
        for f in [
            "fn stage1(",
            "fn stage2(",
            "fn output_transform(",
            "fn fs_main(",
        ] {
            assert!(s.contains(f), "missing {f}");
        }
    }
}

#[cfg(test)]
mod shader_validation {
    /// Parse and validate the WGSL with naga — catches shader errors in
    /// `cargo test` on machines without a GPU.
    #[test]
    fn wgsl_validates() {
        let src = super::shader_source();
        let module = match naga::front::wgsl::parse_str(&src) {
            Ok(m) => m,
            Err(e) => panic!("{}", e.emit_to_string(&src)),
        };
        let mut v = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        );
        if let Err(e) = v.validate(&module) {
            panic!("{}", e.emit_to_string(&src));
        }
    }
}
