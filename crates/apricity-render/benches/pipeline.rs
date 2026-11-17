//! Performance budget benches (PRD §10.3). The CPU reference is the
//! fallback preview path and the scope path, so its proxy-resolution
//! cost is budgeted: a 640×360 proxy must render well under 16 ms on the
//! reference CI machine.

use apricity_core::GradeParameters;
use apricity_render::{cpu, prepare, RenderContext};
use criterion::{criterion_group, criterion_main, Criterion};

fn kitchen_sink() -> GradeParameters {
    let mut p = GradeParameters::IDENTITY;
    p.stage1.temperature = -20.0;
    p.stage2.contrast = 25.0;
    p.stage2.saturation = 10.0;
    p.stage2.lift.b = 0.03;
    p.stage2.hsl.orange.saturation = 20.0;
    p.stage2.split_tone.shadow_saturation = 30.0;
    p.stage2.split_tone.shadow_hue = 200.0;
    p.stage2.vignette.amount = -30.0;
    p.stage2.grain.amount = 20.0;
    p
}

fn bench(c: &mut Criterion) {
    let (w, h) = (640u32, 360u32);
    let src: Vec<u8> = (0..w * h * 4).map(|i| (i * 7 % 251) as u8).collect();
    let u = prepare(&kitchen_sink(), &RenderContext::srgb(w, h));
    c.bench_function("cpu_proxy_640x360_full_grade", |b| {
        b.iter(|| cpu::render_float(&u, None, &src, w, h))
    });
    c.bench_function("prepare_uniforms", |b| {
        b.iter(|| prepare(&kitchen_sink(), &RenderContext::srgb(w, h)))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
