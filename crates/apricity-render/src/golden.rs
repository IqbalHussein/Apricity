//! Golden-image harness.
//!
//! Gated behind the `golden-images` feature so non-test builds don't pull in
//! `image`. Two fixture families:
//!
//! - [`render_identity_patch`] — the M1 color-space round-trip fixture
//!   (sRGB → linear Rec.2020 → sRGB, no grade).
//! - [`fixtures`] — one parameter set per pipeline stage plus a full grade,
//!   rendered over [`test_patch`] by the CPU reference. The checked-in PNGs
//!   pin the CPU output; `tests/gpu_parity.rs` holds the GPU to the CPU.

use apricity_core::{convert, ColorSpace, CurvePoint, GradeParameters, LogProfile};
use apricity_lut::Lut3d;
use glam::Vec3;

use crate::{cpu, prepare, Overlay, RenderContext};

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

/// The multi-region test patch used by the per-stage fixtures (RGBA8,
/// [`PATCH_SIZE`]²):
/// - rows 0–63: 16-step grey ramp;
/// - rows 64–127: smooth horizontal grey gradient;
/// - rows 128–191: hue sweep (HSV s = 0.6, v = 0.8);
/// - rows 192–255: 8-band color wheel.
pub fn test_patch() -> Vec<u8> {
    let n = PATCH_SIZE;
    let mut out = vec![0u8; (n * n * 4) as usize];
    for y in 0..n {
        for x in 0..n {
            let fx = x as f32 / (n - 1) as f32;
            let c = match y / 64 {
                0 => Vec3::splat((x * 16 / n).min(15) as f32 / 15.0),
                1 => Vec3::splat(fx),
                2 => {
                    let h = fx * 6.0;
                    let k = |m: f32| {
                        let k = (m + h) % 6.0;
                        0.8 - 0.8 * 0.6 * k.min(4.0 - k).clamp(0.0, 1.0)
                    };
                    Vec3::new(k(5.0), k(3.0), k(1.0))
                }
                _ => source_pixel(x, n / 2 + y % (n / 2)),
            };
            let i = ((y * n + x) * 4) as usize;
            out[i] = float_to_u8(c.x);
            out[i + 1] = float_to_u8(c.y);
            out[i + 2] = float_to_u8(c.z);
            out[i + 3] = 255;
        }
    }
    out
}

/// A golden fixture: a name, a grade, render context tweaks, optional LUT.
pub struct Fixture {
    /// File stem of the reference PNG.
    pub name: &'static str,
    /// The grade.
    pub params: GradeParameters,
    /// Preview overlay.
    pub overlay: Overlay,
    /// Optional 3D LUT.
    pub lut: Option<Lut3d>,
}

impl Fixture {
    /// Render context for this fixture over the test patch.
    pub fn context(&self) -> RenderContext {
        let mut ctx = RenderContext::srgb(PATCH_SIZE, PATCH_SIZE);
        ctx.overlay = self.overlay;
        ctx.lut_size = self.lut.as_ref().map_or(0, |l| l.size as u32);
        ctx
    }

    /// CPU reference render of the test patch (RGBA8).
    pub fn render_cpu(&self) -> Vec<u8> {
        let u = prepare(&self.params, &self.context());
        cpu::render_rgba8(&u, self.lut.as_ref(), &test_patch(), PATCH_SIZE, PATCH_SIZE)
    }
}

/// One fixture per pipeline stage, plus a full grade and overlays.
pub fn fixtures() -> Vec<Fixture> {
    let base = GradeParameters::IDENTITY;
    let f = |name, params| Fixture {
        name,
        params,
        overlay: Overlay::None,
        lut: None,
    };

    let mut s1 = base.clone();
    s1.stage1.temperature = -40.0;
    s1.stage1.tint = 10.0;
    s1.stage1.exposure = 0.5;
    s1.stage1.black_point = apricity_core::Rgb {
        r: 0.01,
        g: 0.008,
        b: 0.012,
    };

    let mut log = base.clone();
    log.stage1.input_profile = LogProfile::SonySLog3;

    let mut prim = base.clone();
    prim.stage2.exposure = 0.3;
    prim.stage2.contrast = 30.0;
    prim.stage2.highlights = -40.0;
    prim.stage2.shadows = 30.0;
    prim.stage2.whites = 10.0;
    prim.stage2.blacks = -10.0;
    prim.stage2.temperature = 15.0;
    prim.stage2.tint = -5.0;
    prim.stage2.vibrance = 25.0;
    prim.stage2.saturation = -10.0;

    let mut wheels = base.clone();
    wheels.stage2.lift.b = 0.05;
    wheels.stage2.lift.r = -0.02;
    wheels.stage2.gamma.master = 0.1;
    wheels.stage2.gain.r = 0.05;

    let mut hsl = base.clone();
    hsl.stage2.hsl.orange.saturation = 40.0;
    hsl.stage2.hsl.orange.hue = -10.0;
    hsl.stage2.hsl.blue.saturation = -50.0;
    hsl.stage2.hsl.blue.luminance = 30.0;
    hsl.stage2.hsl.green.hue = 20.0;

    let mut tone = base.clone();
    tone.stage2.split_tone.shadow_hue = 210.0;
    tone.stage2.split_tone.shadow_saturation = 40.0;
    tone.stage2.split_tone.highlight_hue = 40.0;
    tone.stage2.split_tone.highlight_saturation = 30.0;
    tone.stage2.split_tone.balance = 20.0;
    tone.stage2.tone_curve.master = vec![
        CurvePoint::new(0.0, 0.03),
        CurvePoint::new(0.25, 0.2),
        CurvePoint::new(0.75, 0.82),
        CurvePoint::new(1.0, 0.97),
    ];
    tone.stage2.tone_curve.blue = vec![CurvePoint::new(0.0, 0.05), CurvePoint::new(1.0, 0.95)];

    let mut fx = base.clone();
    fx.stage2.vignette.amount = -60.0;
    fx.stage2.vignette.midpoint = 40.0;
    fx.stage2.vignette.feather = 60.0;
    fx.stage2.vignette.roundness = 50.0;
    fx.stage2.grain.amount = 40.0;
    fx.stage2.grain.size = 1.5;
    fx.stage2.grain.roughness = 60.0;
    fx.stage2.grain.seed = 42;

    let mut full = tone.clone();
    full.stage1 = s1.stage1.clone();
    full.stage2.contrast = 20.0;
    full.stage2.saturation = 15.0;
    full.stage2.lift = wheels.stage2.lift;
    full.stage2.hsl = hsl.stage2.hsl;
    full.stage2.vignette = fx.stage2.vignette;

    let mut lut_p = base.clone();
    lut_p.stage2.lut_id = Some("golden".into());
    lut_p.stage2.lut_strength = 70.0;
    let lut = Lut3d::from_fn(17, |[r, g, b]| [g * 0.9 + 0.05, b, r * r]);

    vec![
        f("stage1_correction", s1),
        f("stage1_slog3", log),
        f("stage2_primaries", prim),
        f("stage2_wheels", wheels),
        f("stage2_hsl", hsl),
        f("stage2_split_curves", tone),
        f("stage2_effects", fx),
        f("full_grade", full),
        Fixture {
            name: "stage2_lut",
            params: lut_p,
            overlay: Overlay::None,
            lut: Some(lut),
        },
        Fixture {
            name: "overlay_false_color",
            params: base.clone(),
            overlay: Overlay::FalseColor,
            lut: None,
        },
        Fixture {
            name: "overlay_zebra",
            params: base,
            overlay: Overlay::Zebra,
            lut: None,
        },
    ]
}

/// Fraction of channel values whose absolute difference exceeds `tol`.
pub fn fraction_over(a: &[u8], b: &[u8], tol: u8) -> f64 {
    assert_eq!(a.len(), b.len());
    let mut n = 0usize;
    let mut over = 0usize;
    for i in 0..a.len() {
        if i % 4 == 3 {
            continue;
        }
        n += 1;
        if a[i].abs_diff(b[i]) > tol {
            over += 1;
        }
    }
    over as f64 / n.max(1) as f64
}
