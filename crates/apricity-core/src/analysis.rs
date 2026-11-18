//! Stage 1 auto-analysis: exposure normalization, white-balance estimation,
//! and black-point calibration.
//!
//! Input is the **decoded scene-linear Rec.2020** proxy image (after the
//! input-profile decode, before any correction). Output is a set of Stage 1
//! parameter values; the analysis never touches Stage 2. Results are stored
//! in [`CorrectionParameters`] so rendering stays a pure function of the
//! parameters (and the analysis is re-run on each new image only when the
//! corresponding `auto_*` flag is set).
//!
//! The analysis is deterministic: same pixels → same result, no RNG, no
//! clock.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::grade_parameters::{CorrectionParameters, Rgb};
use crate::transfer::srgb_decode;
use crate::white_balance::{gains, mired_shift_to_kelvin, REC2020_LUMA};

/// Dominant light source class inferred from the white-balance estimate.
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Illuminant {
    /// Below ~3800 K.
    Tungsten,
    /// ~3800–5000 K: fluorescent, mixed, golden hour.
    WarmMixed,
    /// ~5000–6800 K.
    Daylight,
    /// Above ~6800 K: shade, overcast.
    Shade,
    /// Not enough neutral pixels to estimate.
    Unknown,
}

/// Result of [`analyze`].
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage1Analysis {
    /// Suggested Stage 1 exposure in **stops**.
    pub exposure: f32,
    /// Suggested white-balance temperature correction in **mireds**.
    pub temperature: f32,
    /// Suggested tint correction in **tint units**.
    pub tint: f32,
    /// Suggested per-channel black point, **scene-linear Rec.2020**.
    pub black_point: Rgb,
    /// Median Rec.2020 luminance of the input, **scene-linear**.
    pub median_luminance: f32,
    /// Estimated source correlated color temperature in **kelvin**.
    pub estimated_kelvin: f32,
    /// Inferred light source.
    pub illuminant: Illuminant,
}

/// Analyze a decoded proxy. `exposure_target` is the desired median as an
/// **sRGB-encoded** value (PRD default 0.45).
pub fn analyze(pixels: &[[f32; 3]], exposure_target: f32) -> Stage1Analysis {
    let (temperature, tint, illuminant) = estimate_white_balance(pixels);
    let wb = gains(temperature, tint);

    let mut lum: Vec<f32> = pixels
        .iter()
        .map(|p| (Vec3::from_array(*p) * wb).dot(REC2020_LUMA))
        .collect();
    let median_luminance = percentile(&mut lum, 0.5);
    let target_linear = srgb_decode(exposure_target.clamp(0.05, 0.95));
    let exposure = if median_luminance > 1e-5 {
        (target_linear / median_luminance).log2().clamp(-3.0, 3.0)
    } else {
        3.0
    };
    let scale = 2f32.powf(exposure);

    let mut channels: [Vec<f32>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for p in pixels {
        let v = Vec3::from_array(*p) * wb * scale;
        channels[0].push(v.x);
        channels[1].push(v.y);
        channels[2].push(v.z);
    }
    let bp = |c: &mut Vec<f32>| percentile(c, 0.001).clamp(0.0, 0.03);
    let black_point = Rgb {
        r: bp(&mut channels[0]),
        g: bp(&mut channels[1]),
        b: bp(&mut channels[2]),
    };

    Stage1Analysis {
        exposure,
        temperature,
        tint,
        black_point,
        median_luminance,
        estimated_kelvin: mired_shift_to_kelvin(-temperature),
        illuminant,
    }
}

/// Copy analysis results into `stage1`, honoring its `auto_*` flags.
pub fn apply_analysis(stage1: &mut CorrectionParameters, a: &Stage1Analysis) {
    if stage1.auto_exposure {
        stage1.exposure = a.exposure;
    }
    if stage1.auto_white_balance {
        stage1.temperature = a.temperature;
        stage1.tint = a.tint;
    }
    if stage1.auto_black_point {
        stage1.black_point = a.black_point;
    }
}

/// Gray-world estimate over mid-luminance, unclipped pixels. Returns the
/// (temperature, tint) correction that neutralizes the average.
fn estimate_white_balance(pixels: &[[f32; 3]]) -> (f32, f32, Illuminant) {
    let mut sum = Vec3::ZERO;
    let mut n = 0usize;
    for p in pixels {
        let v = Vec3::from_array(*p);
        let y = v.dot(REC2020_LUMA);
        if (0.02..=0.9).contains(&y) && v.max_element() < 0.98 && v.min_element() > 0.0 {
            sum += v / y;
            n += 1;
        }
    }
    if n < pixels.len().max(1) / 100 || n == 0 {
        return (0.0, 0.0, Illuminant::Unknown);
    }
    let avg = sum / n as f32;

    let err = |m: f32, t: f32| {
        let c = avg * gains(m, t);
        let a = (c.x / c.y).ln();
        let b = (c.z / c.y).ln();
        a * a + b * b
    };
    let search = |best: (f32, f32, f32), m_lo: f32, m_hi: f32, t_lo: f32, t_hi: f32, step: f32| {
        let mut best = best;
        let mut m = m_lo;
        while m <= m_hi + 1e-3 {
            let mut t = t_lo;
            while t <= t_hi + 1e-3 {
                let e = err(m, t);
                if e < best.2 {
                    best = (m, t, e);
                }
                t += step;
            }
            m += step;
        }
        best
    };
    let coarse = search((0.0, 0.0, err(0.0, 0.0)), -250.0, 250.0, -100.0, 100.0, 5.0);
    let (cm, ct, _) = coarse;
    let (best_m, best_t, _) = search(coarse, cm - 5.0, cm + 5.0, ct - 5.0, ct + 5.0, 0.5);
    let (m, t) = (best_m.clamp(-250.0, 250.0), best_t.clamp(-100.0, 100.0));

    let source_k = mired_shift_to_kelvin(-m);
    let illuminant = match source_k {
        k if k < 3800.0 => Illuminant::Tungsten,
        k if k < 5000.0 => Illuminant::WarmMixed,
        k if k < 6800.0 => Illuminant::Daylight,
        _ => Illuminant::Shade,
    };
    (m, t, illuminant)
}

/// `q`-quantile (0..1) by sorting in place. Empty input → 0.
fn percentile(v: &mut [f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    let idx = ((v.len() - 1) as f32 * q).round() as usize;
    v[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(tint: Vec3, scale: f32) -> Vec<[f32; 3]> {
        (0..1000)
            .map(|i| {
                let l = 0.02 + 0.6 * (i as f32 / 999.0);
                (tint * l * scale).to_array()
            })
            .collect()
    }

    #[test]
    fn neutral_image_needs_no_white_balance() {
        let a = analyze(&gradient(Vec3::ONE, 1.0), 0.45);
        assert!(a.temperature.abs() < 1.0 && a.tint.abs() < 1.0, "{a:?}");
    }

    #[test]
    fn warm_cast_is_cooled() {
        // A neutral scene lit by ~3200 K light.
        let cast = gains(150.0, 0.0);
        let a = analyze(&gradient(cast, 1.0), 0.45);
        assert!(a.temperature < -60.0, "{a:?}");
        let corrected = cast * gains(a.temperature, a.tint);
        assert!(
            (corrected.x / corrected.y - 1.0).abs() < 0.03,
            "{corrected:?}"
        );
        assert!(
            (corrected.z / corrected.y - 1.0).abs() < 0.03,
            "{corrected:?}"
        );
        assert_eq!(a.illuminant, Illuminant::Tungsten);
    }

    #[test]
    fn exposure_hits_target_median() {
        let a = analyze(&gradient(Vec3::ONE, 0.25), 0.45);
        let after = a.median_luminance * 2f32.powf(a.exposure);
        assert!((after - srgb_decode(0.45)).abs() < 1e-3, "{a:?}");
    }

    #[test]
    fn black_point_is_bounded() {
        let mut px = gradient(Vec3::ONE, 1.0);
        px.iter_mut()
            .for_each(|p| p.iter_mut().for_each(|c| *c += 0.2));
        let a = analyze(&px, 0.45);
        assert!(a.black_point.r <= 0.03 && a.black_point.r >= 0.0);
    }

    #[test]
    fn apply_respects_flags() {
        let a = analyze(&gradient(gains(80.0, 0.0), 0.3), 0.45);
        let mut s = CorrectionParameters {
            auto_white_balance: true,
            ..CorrectionParameters::IDENTITY
        };
        apply_analysis(&mut s, &a);
        assert_eq!(s.exposure, 0.0);
        assert_eq!(s.temperature, a.temperature);
    }

    #[test]
    fn empty_input_is_safe() {
        let a = analyze(&[], 0.45);
        assert_eq!(a.illuminant, Illuminant::Unknown);
        assert!(a.exposure.is_finite());
    }
}
