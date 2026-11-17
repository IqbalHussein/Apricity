//! CPU reference implementation of the full pipeline.
//!
//! This is the **specification** of the pipeline: `shaders/*.wgsl` mirror
//! these functions line for line, and the parity tests assert the two agree
//! within 1 LSB at 8-bit. It is also the production path when WebGPU is
//! unavailable (ADR-0007), for scopes (which need pre-clip float values),
//! and for recipe thumbnails.
//!
//! Per pixel:
//! 1. **Stage 1** — decode source (sRGB EOTF or camera log) → native gamut
//!    matrix → scene-linear Rec.2020 → WB × exposure gain → black point →
//!    optional log-to-display rendering.
//! 2. **Stage 2** — creative exposure/WB → vibrance/saturation (linear) →
//!    working encoding → contrast → highlights/shadows/whites/blacks →
//!    lift/gain/gamma → HSL bands → split tone → tone curves → LUT →
//!    scene-linear → vignette → grain.
//! 3. **Output** — Rec.2020 → output primaries → output transfer →
//!    optional preview overlay.
//!
//! No `wgpu`, no clock, no RNG: grain uses an integer hash of an explicit
//! seed and pixel coordinates.

use apricity_core::log_profiles::{
    apple_log_decode, display_render, logc3_decode, slog3_decode, vlog_decode,
};
use apricity_core::transfer::srgb_decode;
use apricity_lut::Lut3d;
use glam::{Vec2, Vec3};

use crate::uniforms::{lut_matrices, pivot, GradeUniforms};

const LUMA2020: Vec3 = apricity_core::white_balance::REC2020_LUMA;
const LUMA709: Vec3 = Vec3::new(0.2126, 0.7152, 0.0722);

#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

#[inline]
fn vmix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a * (1.0 - t) + b * t
}

#[inline]
fn clamp01(v: Vec3) -> Vec3 {
    v.clamp(Vec3::ZERO, Vec3::ONE)
}

/// Fix `signum(0) == 1` so `enc(0) == 0` exactly, matching WGSL `sign`.
#[inline]
fn sign0(v: Vec3) -> Vec3 {
    Vec3::new(
        if v.x == 0.0 { 0.0 } else { v.x.signum() },
        if v.y == 0.0 { 0.0 } else { v.y.signum() },
        if v.z == 0.0 { 0.0 } else { v.z.signum() },
    )
}

#[inline]
fn enc0(v: Vec3) -> Vec3 {
    sign0(v) * v.abs().powf(1.0 / apricity_core::WORKING_GAMMA)
}

#[inline]
fn dec0(v: Vec3) -> Vec3 {
    sign0(v) * v.abs().powf(apricity_core::WORKING_GAMMA)
}

#[inline]
fn row_mul(m: &[[f32; 4]; 3], v: Vec3) -> Vec3 {
    Vec3::new(
        m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
        m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
        m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
    )
}

#[inline]
fn xyz(a: [f32; 4]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}

/// Decode one source channel by profile id (see [`crate::uniforms::profile_id`]).
#[inline]
pub fn decode_channel(profile: u32, v: f32) -> f32 {
    match profile {
        1 => logc3_decode(v),
        2 => slog3_decode(v),
        3 => vlog_decode(v),
        4 => apple_log_decode(v),
        _ => srgb_decode(v),
    }
}

/// Source decode only: encoded source → scene-linear Rec.2020, no
/// correction. Feeds Stage 1 auto-analysis.
#[inline]
pub fn decode_source(u: &GradeUniforms, src: Vec3) -> Vec3 {
    let p = u.s1_flags[1];
    let native = Vec3::new(
        decode_channel(p, src.x),
        decode_channel(p, src.y),
        decode_channel(p, src.z),
    );
    row_mul(&u.s1_matrix, native)
}

/// Stage 1 — technical correction. Input: encoded source in [0, 1].
/// Output: scene-linear Rec.2020 (display-referred linear when
/// log-to-display is active).
pub fn stage1(u: &GradeUniforms, src: Vec3) -> Vec3 {
    let mut lin = decode_source(u, src);
    if u.s1_flags[0] != 0 {
        lin *= xyz(u.s1_gain);
        let bp = xyz(u.s1_black);
        lin = (lin - bp) / (Vec3::ONE - bp);
    }
    if u.s1_flags[2] != 0 {
        lin = Vec3::new(
            display_render(lin.x),
            display_render(lin.y),
            display_render(lin.z),
        );
    }
    lin
}

#[inline]
fn bell(l: f32, c: f32, w: f32) -> f32 {
    let d = (l - c) / w;
    (-(d * d)).exp()
}

fn rgb_to_hsv(c: Vec3) -> Vec3 {
    let mx = c.max_element();
    let mn = c.min_element();
    let d = mx - mn;
    let mut h = 0.0;
    if d > 1e-6 {
        if mx == c.x {
            h = ((c.y - c.z) / d) / 6.0;
        } else if mx == c.y {
            h = ((c.z - c.x) / d + 2.0) / 6.0;
        } else {
            h = ((c.x - c.y) / d + 4.0) / 6.0;
        }
        h -= h.floor();
    }
    let s = if mx > 1e-6 { d / mx } else { 0.0 };
    Vec3::new(h, s, mx)
}

fn hsv_to_rgb(hsv: Vec3) -> Vec3 {
    let h6 = (hsv.x - hsv.x.floor()) * 6.0;
    let k = |n: f32| {
        let k = (n + h6) % 6.0;
        hsv.z - hsv.z * hsv.y * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    Vec3::new(k(5.0), k(3.0), k(1.0))
}

fn hsl_adjust(u: &GradeUniforms, w: Vec3) -> Vec3 {
    let c = clamp01(w);
    let hsv = rgb_to_hsv(c);
    if hsv.y < 1e-5 {
        return w;
    }
    let h = hsv.x;
    let mut adj = Vec3::ZERO;
    for i in 0..8 {
        let c0 = u.s2_hsl[i][3];
        let c1 = u.s2_hsl[i + 1][3];
        if h >= c0 && h < c1 {
            let t = smoothstep(0.0, 1.0, (h - c0) / (c1 - c0));
            adj = vmix(xyz(u.s2_hsl[i]), xyz(u.s2_hsl[i + 1]), t);
        }
    }
    let nh = h + adj.x;
    let ns = (hsv.y * (1.0 + adj.y)).clamp(0.0, 1.0);
    let rgb = hsv_to_rgb(Vec3::new(nh - nh.floor(), ns, hsv.z));
    w + (rgb - c) + Vec3::splat(adj.z * 0.2 * hsv.y)
}

/// PCG hash (Jarzynski & Olano 2020). Mirrored in WGSL.
#[inline]
pub fn pcg(v: u32) -> u32 {
    let state = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

#[inline]
fn hash2(ix: i32, iy: i32, seed: u32) -> f32 {
    let h = pcg((ix as u32) ^ pcg((iy as u32) ^ pcg(seed)));
    ((h >> 8) as f32) / 16_777_215.0 * 2.0 - 1.0
}

fn value_noise(q: Vec2, seed: u32) -> f32 {
    let i = q.floor();
    let f = q - i;
    let s = f * f * (Vec2::splat(3.0) - 2.0 * f);
    let (ix, iy) = (i.x as i32, i.y as i32);
    let a = hash2(ix, iy, seed);
    let b = hash2(ix.wrapping_add(1), iy, seed);
    let c = hash2(ix, iy.wrapping_add(1), seed);
    let d = hash2(ix.wrapping_add(1), iy.wrapping_add(1), seed);
    mix(mix(a, b, s.x), mix(c, d, s.x), s.y)
}

/// Grain noise in [−1, 1] at source pixel `px` (preview-proxy pixels).
pub fn grain_noise(u: &GradeUniforms, px: Vec2) -> f32 {
    let q = px * u.s2_grain[3] / u.s2_grain[1];
    let seed = u.s2_grain_seed[0];
    let n1 = value_noise(q, seed);
    let n2 = value_noise(q * 2.0 + Vec2::splat(17.0), seed ^ 0x9E37_79B9);
    let r = u.s2_grain[2];
    (n1 + r * 0.5 * n2) / (1.0 + 0.5 * r)
}

fn sample_lut(lut: &Lut3d, e: Vec3) -> Vec3 {
    Vec3::from_array(lut.sample_trilinear(e.to_array()))
}

/// Stage 2 — creative grade. Input/output: scene-linear Rec.2020.
/// `px` is the pixel center in source-image pixels.
pub fn stage2(u: &GradeUniforms, lut: Option<&Lut3d>, lin_in: Vec3, px: Vec2) -> Vec3 {
    if u.s2_flags[0] == 0 {
        return lin_in;
    }
    let mut lin = lin_in * xyz(u.s2_gain);

    // Vibrance + saturation (scene-linear).
    let y = lin.dot(LUMA2020);
    let mx = lin.max_element();
    let mn = lin.min_element();
    let chroma = if mx > 1e-6 {
        ((mx - mn) / mx).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let sat = u.s2_gain[3] * (1.0 + u.s2_tone2[1] * (1.0 - chroma));
    lin = Vec3::splat(y) + (lin - Vec3::splat(y)) * sat;

    // Working encoding.
    let mut w = enc0(lin);
    let p = pivot();
    w = Vec3::splat(p) + (w - Vec3::splat(p)) * u.s2_tone[0];

    let l = clamp01(w).dot(LUMA2020);
    let d = 0.25
        * (u.s2_tone2[0] * bell(l, 0.05, 0.15)
            + u.s2_tone[2] * bell(l, 0.25, 0.2)
            + u.s2_tone[1] * bell(l, 0.75, 0.2)
            + u.s2_tone[3] * bell(l, 0.95, 0.15));
    w += Vec3::splat(d);

    // Lift / gain / gamma.
    w += xyz(u.s2_lift) * (Vec3::ONE - clamp01(w));
    w *= xyz(u.s2_gain_wheel);
    w = sign0(w) * w.abs().powf_vec(xyz(u.s2_gamma));

    if u.s2_flags[1] != 0 {
        w = hsl_adjust(u, w);
    }

    // Split tone.
    let l = clamp01(w).dot(LUMA2020);
    let sw = 1.0 - smoothstep(0.0, 1.0, l + u.s2_tone2[3] * 0.5);
    w += xyz(u.s2_split_shadow) * sw + xyz(u.s2_split_highlight) * (1.0 - sw);

    // Tone curves: master then per channel.
    if u.s2_flags[2] != 0 {
        w = Vec3::new(
            curve_sample(u, 1, curve_sample(u, 0, w.x)),
            curve_sample(u, 2, curve_sample(u, 0, w.y)),
            curve_sample(u, 3, curve_sample(u, 0, w.z)),
        );
    }

    // 3D LUT in sRGB-encoded Rec.709.
    if u.s2_flags[3] != 0 {
        if let Some(lut) = lut {
            let (to709, to2020) = lut_matrices();
            let e709 = clamp01(to709 * dec0(w));
            let e = Vec3::new(
                apricity_core::transfer::srgb_encode(e709.x),
                apricity_core::transfer::srgb_encode(e709.y),
                apricity_core::transfer::srgb_encode(e709.z),
            );
            let o = sample_lut(lut, e);
            let o709 = Vec3::new(srgb_decode(o.x), srgb_decode(o.y), srgb_decode(o.z));
            w = vmix(w, enc0(to2020 * o709), u.s2_tone2[2]);
        }
    }

    lin = dec0(w);

    // Vignette (multiplicative, scene-linear).
    let v = u.s2_vignette;
    if v[0] != 0.0 {
        let size = Vec2::new(u.out_params[1], u.out_params[2]);
        let mut q = (px / size) * 2.0 - Vec2::ONE;
        q -= Vec2::new(u.s2_vignette2[0], u.s2_vignette2[1]);
        let t = (v[3] + 1.0) * 0.5;
        let sx = mix(1.0, u.s2_vignette2[2], t);
        let dist = Vec2::new(q.x * sx, q.y).length() / Vec2::new(sx, 1.0).length();
        let mask = smoothstep(v[1], v[1] + 0.01 + v[2], dist);
        lin *= (1.0 + v[0] * mask).max(0.0);
    }

    // Grain (working encoding, midtone weighted).
    if u.s2_grain[0] > 0.0 {
        let mut w = enc0(lin);
        let n = grain_noise(u, px);
        let l = clamp01(w).dot(LUMA2020);
        let weight = 0.25 + 3.0 * l * (1.0 - l);
        w += Vec3::splat(u.s2_grain[0] * 0.12 * n * weight);
        lin = dec0(w);
    }
    lin
}

#[inline]
fn curve_sample(u: &GradeUniforms, ch: usize, x: f32) -> f32 {
    let n = u.curves.len();
    if x <= 0.0 {
        return u.curves[0][ch] + x;
    }
    if x >= 1.0 {
        return u.curves[n - 1][ch] + (x - 1.0);
    }
    let f = x * (n - 1) as f32;
    let i = (f.floor() as usize).min(n - 2);
    let t = f - i as f32;
    u.curves[i][ch] + (u.curves[i + 1][ch] - u.curves[i][ch]) * t
}

/// Output transform: scene-linear Rec.2020 → output encoding (unclamped).
pub fn output(u: &GradeUniforms, lin: Vec3) -> Vec3 {
    let o = row_mul(&u.out_matrix, lin);
    match u.out_flags[0] {
        0 => Vec3::new(
            apricity_core::transfer::srgb_encode(o.x),
            apricity_core::transfer::srgb_encode(o.y),
            apricity_core::transfer::srgb_encode(o.z),
        ),
        1 => sign0(o) * o.abs().powf(1.0 / 2.4),
        _ => o,
    }
}

/// Preview overlay on the output-encoded value.
pub fn overlay(u: &GradeUniforms, out: Vec3, px: Vec2) -> Vec3 {
    match u.out_flags[1] {
        1 => {
            let y = clamp01(out).dot(LUMA709);
            if y < 0.02 {
                Vec3::new(0.5, 0.0, 0.6)
            } else if y < 0.08 {
                Vec3::new(0.1, 0.25, 0.9)
            } else if y < 0.42 {
                Vec3::splat(y)
            } else if y < 0.50 {
                Vec3::new(0.2, 0.75, 0.3)
            } else if y < 0.62 {
                Vec3::splat(y)
            } else if y < 0.72 {
                Vec3::new(0.95, 0.55, 0.7)
            } else if y < 0.97 {
                Vec3::splat(y)
            } else if y < 0.995 {
                Vec3::new(1.0, 0.9, 0.1)
            } else {
                Vec3::new(0.9, 0.1, 0.1)
            }
        }
        2 => {
            let y = clamp01(out).dot(LUMA709);
            let stripe = ((px.x + px.y) / 8.0).fract() < 0.5;
            if y > u.out_params[0] && stripe {
                Vec3::splat(0.05)
            } else {
                out
            }
        }
        _ => out,
    }
}

/// The full per-pixel pipeline. `src` is the encoded source in [0, 1];
/// `px` the pixel center in source pixels. Returns the output-encoded
/// value **before** clamping (scopes need the excursions).
pub fn shade(u: &GradeUniforms, lut: Option<&Lut3d>, src: Vec3, px: Vec2) -> Vec3 {
    let lin = stage1(u, src);
    let lin = stage2(u, lut, lin, px);
    let out = output(u, lin);
    overlay(u, out, px)
}

/// Quantize an output-encoded float to 8 bits (round to nearest, clamped).
#[inline]
pub fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Render an RGBA8 source of `w × h` to output-encoded floats (RGB,
/// unclamped). Alpha is ignored.
pub fn render_float(
    u: &GradeUniforms,
    lut: Option<&Lut3d>,
    src: &[u8],
    w: u32,
    h: u32,
) -> Vec<[f32; 3]> {
    assert_eq!(
        src.len(),
        (w * h * 4) as usize,
        "source buffer size mismatch"
    );
    let mut out = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let s = Vec3::new(src[i] as f32, src[i + 1] as f32, src[i + 2] as f32) / 255.0;
            let px = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            out.push(shade(u, lut, s, px).to_array());
        }
    }
    out
}

/// Render an RGBA8 source to RGBA8 output (alpha copied from source).
pub fn render_rgba8(u: &GradeUniforms, lut: Option<&Lut3d>, src: &[u8], w: u32, h: u32) -> Vec<u8> {
    let f = render_float(u, lut, src, w, h);
    let mut out = vec![0u8; src.len()];
    for (i, c) in f.iter().enumerate() {
        out[i * 4] = to_u8(c[0]);
        out[i * 4 + 1] = to_u8(c[1]);
        out[i * 4 + 2] = to_u8(c[2]);
        out[i * 4 + 3] = src[i * 4 + 3];
    }
    out
}

/// Decode an RGBA8 source to scene-linear Rec.2020 (no correction), for
/// [`apricity_core::analysis::analyze`].
pub fn decode_linear(u: &GradeUniforms, src: &[u8]) -> Vec<[f32; 3]> {
    src.chunks_exact(4)
        .map(|p| {
            let s = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) / 255.0;
            decode_source(u, s).to_array()
        })
        .collect()
}

trait PowfVec {
    fn powf_vec(self, e: Vec3) -> Vec3;
}

impl PowfVec for Vec3 {
    #[inline]
    fn powf_vec(self, e: Vec3) -> Vec3 {
        Vec3::new(self.x.powf(e.x), self.y.powf(e.y), self.z.powf(e.z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uniforms::{prepare, RenderContext};
    use apricity_core::GradeParameters;

    fn ramp() -> (Vec<u8>, u32, u32) {
        let (w, h) = (64u32, 8u32);
        let mut v = Vec::new();
        for y in 0..h {
            for x in 0..w {
                v.extend_from_slice(&[(x * 4) as u8, (y * 32) as u8, (255 - x * 4) as u8, 255]);
            }
        }
        (v, w, h)
    }

    #[test]
    fn identity_reproduces_source() {
        let (src, w, h) = ramp();
        let u = prepare(&GradeParameters::IDENTITY, &RenderContext::srgb(w, h));
        let out = render_rgba8(&u, None, &src, w, h);
        let max = src
            .iter()
            .zip(&out)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(max <= 1, "identity drift {max}");
    }

    #[test]
    fn original_is_exact_passthrough() {
        let (src, w, h) = ramp();
        let u = prepare(&GradeParameters::original(), &RenderContext::srgb(w, h));
        let out = render_rgba8(&u, None, &src, w, h);
        let max = src
            .iter()
            .zip(&out)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(max <= 1);
    }

    #[test]
    fn exposure_brightens() {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.exposure = 1.0;
        let u = prepare(&p, &RenderContext::srgb(1, 1));
        let o = shade(&u, None, Vec3::splat(0.4), Vec2::splat(0.5));
        assert!(o.x > 0.5);
    }

    #[test]
    fn saturation_minus_100_is_grey() {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.saturation = -100.0;
        let u = prepare(&p, &RenderContext::srgb(1, 1));
        let o = shade(&u, None, Vec3::new(0.8, 0.2, 0.3), Vec2::splat(0.5));
        assert!(
            (o.x - o.y).abs() < 1e-4 && (o.y - o.z).abs() < 1e-4,
            "{o:?}"
        );
    }

    #[test]
    fn contrast_preserves_pivot() {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.contrast = 80.0;
        let u = prepare(&p, &RenderContext::srgb(1, 1));
        let grey = apricity_core::transfer::srgb_encode(0.18);
        let o = shade(&u, None, Vec3::splat(grey), Vec2::splat(0.5));
        assert!((o.x - grey).abs() < 0.03, "{o:?} vs {grey}");
    }

    #[test]
    fn hsl_shift_leaves_neutrals_alone() {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.hsl.red.hue = 30.0;
        p.stage2.hsl.red.saturation = 50.0;
        let u = prepare(&p, &RenderContext::srgb(1, 1));
        let o = shade(&u, None, Vec3::splat(0.5), Vec2::splat(0.5));
        assert!(
            (o.x - 0.5).abs() < 2e-3 && (o.z - 0.5).abs() < 2e-3,
            "{o:?}"
        );
        let red = shade(&u, None, Vec3::new(0.8, 0.2, 0.2), Vec2::splat(0.5));
        assert!(red.y > 0.25, "red should shift toward orange: {red:?}");
    }

    #[test]
    fn grain_is_deterministic_and_seeded() {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.grain.amount = 50.0;
        p.stage2.grain.seed = 7;
        let u = prepare(&p, &RenderContext::srgb(1, 1));
        let a = grain_noise(&u, Vec2::new(10.5, 3.5));
        let b = grain_noise(&u, Vec2::new(10.5, 3.5));
        assert_eq!(a, b);
        p.stage2.grain.seed = 8;
        let u2 = prepare(&p, &RenderContext::srgb(1, 1));
        assert_ne!(a, grain_noise(&u2, Vec2::new(10.5, 3.5)));
        assert!((-1.0..=1.0).contains(&a));
    }

    #[test]
    fn vignette_darkens_corners_not_center() {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.vignette.amount = -80.0;
        let u = prepare(&p, &RenderContext::srgb(100, 100));
        let c = shade(&u, None, Vec3::splat(0.5), Vec2::new(50.0, 50.0));
        let k = shade(&u, None, Vec3::splat(0.5), Vec2::new(0.5, 0.5));
        assert!((c.x - 0.5).abs() < 2e-3);
        assert!(k.x < 0.3, "{k:?}");
    }

    #[test]
    fn identity_lut_is_transparent() {
        let mut p = GradeParameters::IDENTITY;
        p.stage2.lut_id = Some("id".into());
        let lut = Lut3d::identity(33);
        let mut ctx = RenderContext::srgb(1, 1);
        ctx.lut_size = 33;
        let u = prepare(&p, &ctx);
        assert_eq!(u.s2_flags[3], 1);
        let src = Vec3::new(0.3, 0.6, 0.9);
        let o = shade(&u, Some(&lut), src, Vec2::splat(0.5));
        assert!((o - src).abs().max_element() < 3e-3, "{o:?}");
    }

    #[test]
    fn log_profile_mid_grey_lands_mid_grey() {
        let mut p = GradeParameters::IDENTITY;
        p.stage1.input_profile = apricity_core::LogProfile::SonySLog3;
        let u = prepare(&p, &RenderContext::srgb(1, 1));
        let o = shade(&u, None, Vec3::splat(420.0 / 1023.0), Vec2::splat(0.5));
        let grey = apricity_core::transfer::srgb_encode(0.18);
        assert!((o.x - grey).abs() < 0.03, "{o:?}");
    }
}
