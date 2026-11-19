//! Scope cost at the 640×360 proxy resolution; each must stay well under
//! 5 ms on the reference CI machine.

use apricity_scopes::{broadcast_legal, histogram, rgb_parade, vectorscope, waveform_luma};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn proxy() -> Vec<[f32; 3]> {
    (0..640u32 * 360)
        .map(|i| {
            let f = |k: u32| ((i.wrapping_mul(k) % 1013) as f32) / 1000.0 - 0.05;
            [f(7), f(13), f(29)]
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let px = proxy();
    c.bench_function("waveform_luma_640x360", |b| {
        b.iter(|| waveform_luma(black_box(&px), 640, 360, 640, 256))
    });
    c.bench_function("rgb_parade_640x360", |b| {
        b.iter(|| rgb_parade(black_box(&px), 640, 360, 639, 256))
    });
    c.bench_function("vectorscope_640x360", |b| {
        b.iter(|| vectorscope(black_box(&px), 256))
    });
    c.bench_function("histogram_640x360", |b| {
        b.iter(|| histogram(black_box(&px)))
    });
    c.bench_function("broadcast_legal_640x360", |b| {
        b.iter(|| broadcast_legal(black_box(&px), 0.005))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
