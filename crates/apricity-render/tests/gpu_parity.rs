//! CPU/GPU parity: every golden fixture rendered through wgpu must match
//! the CPU reference within 1 LSB at 8-bit.
//!
//! Runs on any adapter wgpu finds; CI uses lavapipe (software Vulkan). When
//! no adapter exists the test is skipped with a message unless
//! `APRICITY_REQUIRE_GPU=1` is set, in which case it fails.

#![cfg(feature = "golden-images")]

use apricity_render::golden::{fixtures, fraction_over, max_rgb_delta, test_patch, PATCH_SIZE};
use apricity_render::gpu::GpuRenderer;
use apricity_render::prepare;

fn renderer() -> Option<GpuRenderer> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    match pollster::block_on(GpuRenderer::request(
        &instance,
        None,
        wgpu::TextureFormat::Rgba8Unorm,
    )) {
        Ok(r) => Some(r),
        Err(e) => {
            if std::env::var_os("APRICITY_REQUIRE_GPU").is_some() {
                panic!("GPU required but unavailable: {e}");
            }
            eprintln!("skipping GPU parity: {e}");
            None
        }
    }
}

#[test]
fn gpu_matches_cpu_on_every_fixture() {
    let Some(mut gpu) = renderer() else { return };
    let patch = test_patch();
    gpu.set_source_rgba8(&patch, PATCH_SIZE, PATCH_SIZE);
    let mut report = Vec::new();
    for fx in fixtures() {
        gpu.set_lut(fx.lut.as_ref());
        let u = prepare(&fx.params, &fx.context());
        gpu.set_grades(&u, None);
        let g = gpu.render_rgba8_blocking().expect("render");
        let c = fx.render_cpu();
        let max = max_rgb_delta(&g, &c);
        let frac = fraction_over(&g, &c, 1);
        report.push((fx.name, max, frac));
    }
    for (name, max, frac) in &report {
        eprintln!("{name:24} max Δ {max:3}  >1 LSB: {:.4}%", frac * 100.0);
    }
    let bad: Vec<_> = report.iter().filter(|(_, max, _)| *max > 1).collect();
    assert!(bad.is_empty(), "CPU/GPU parity exceeded 1 LSB: {bad:?}");
}
