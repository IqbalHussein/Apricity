//! CPU-only golden-image harness for the identity-grade pipeline.
//!
//! Gated behind the `golden-images` feature so non-test builds don't pull in
//! `image`. The first M1 fixture exercises only color-space round-tripping
//! (sRGB → linear Rec.2020 → sRGB) — no Stage 2 grade, no GPU. M2 will add
//! parity fixtures comparing the CPU reference against the wgpu output.

use apricity_core::{convert, ColorSpace};
use glam::Vec3;

/// Side length of the reference patch in pixels.
pub const PATCH_SIZE: u32 = 256;

/// Render the identity-grade reference patch.
///
/// Layout (deterministic — drives the fixture):
/// - Top half: 16-step grayscale ramp, dark (0) on the left to white (255)
///   on the right, row-constant.
/// - Bottom half: 8-color wheel — red, orange, yellow, green, cyan, blue,
///   magenta, white — eight equal vertical bands.
///
/// Each pixel is sRGB-encoded, run through the v1.0 conversion pipeline
/// (sRGB → LinearRec2020 → sRGB), and returned as 8-bit RGBA. With the
/// identity grade, the round-trip should match the source within a small
/// f32 / matrix-inverse tolerance.
pub fn render_identity_patch() -> Vec<u8> {
    let mut out = vec![0u8; (PATCH_SIZE * PATCH_SIZE * 4) as usize];
    for y in 0..PATCH_SIZE {
        for x in 0..PATCH_SIZE {
            let src = source_pixel(x, y);
            let processed = identity_pipeline(src);
            let i = ((y * PATCH_SIZE + x) * 4) as usize;
            out[i] = float_to_u8(processed.x);
            out[i + 1] = float_to_u8(processed.y);
            out[i + 2] = float_to_u8(processed.z);
            out[i + 3] = 255;
        }
    }
    out
}

fn source_pixel(x: u32, y: u32) -> Vec3 {
    if y < PATCH_SIZE / 2 {
        // 16-step grayscale ramp.
        let step = (x * 16 / PATCH_SIZE).min(15);
        let v = step as f32 / 15.0;
        Vec3::splat(v)
    } else {
        // 8-color wheel — fully saturated primaries + complementaries + white.
        const WHEEL: [(f32, f32, f32); 8] = [
            (1.0, 0.0, 0.0), // red
            (1.0, 0.5, 0.0), // orange
            (1.0, 1.0, 0.0), // yellow
            (0.0, 1.0, 0.0), // green
            (0.0, 1.0, 1.0), // cyan
            (0.0, 0.0, 1.0), // blue
            (1.0, 0.0, 1.0), // magenta
            (1.0, 1.0, 1.0), // white
        ];
        let band = (x * 8 / PATCH_SIZE).min(7) as usize;
        let (r, g, b) = WHEEL[band];
        Vec3::new(r, g, b)
    }
}

fn identity_pipeline(srgb: Vec3) -> Vec3 {
    // sRGB encoded → LinearRec2020 (pipeline interior) → sRGB encoded.
    let linear = convert(srgb, ColorSpace::Srgb, ColorSpace::LinearRec2020);
    convert(linear, ColorSpace::LinearRec2020, ColorSpace::Srgb)
}

#[inline]
fn float_to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Maximum per-channel absolute difference between two RGBA8 buffers of the
/// same dimensions. Returned as `u8` because both inputs are 8-bit; a delta
/// of 1 means "rounded to a different code value at 8-bit precision."
///
/// This is the M1 first-slice metric — a regression detector. M2 upgrades
/// to ΔE2000 in CIELAB once a wider fixture set lands.
pub fn max_rgb_delta(a: &[u8], b: &[u8]) -> u8 {
    assert_eq!(a.len(), b.len(), "buffer length mismatch");
    let mut max = 0u8;
    for i in 0..a.len() {
        // Skip alpha (every 4th byte).
        if i % 4 == 3 {
            continue;
        }
        let d = a[i].abs_diff(b[i]);
        if d > max {
            max = d;
        }
    }
    max
}
