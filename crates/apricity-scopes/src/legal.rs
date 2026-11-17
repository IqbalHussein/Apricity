use serde::Serialize;

use crate::luma;

/// Result of [`broadcast_legal`]. Percentages are of total pixels, 0–100.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LegalReport {
    /// Percent of pixels with luma `Y' > 1.0 + tolerance` (over 100 IRE).
    pub luma_over_pct: f32,
    /// Percent of pixels with luma `Y' < 0 - tolerance` (under 0 IRE).
    pub luma_under_pct: f32,
    /// Percent of pixels with any encoded channel outside
    /// `[-tolerance, 1 + tolerance]` (NaN counts as an excursion).
    pub gamut_excursion_pct: f32,
    /// `true` when all three percentages are below 0.1.
    pub legal: bool,
}

/// Broadcast-legality check on output-encoded RGB `pixels`.
///
/// `tolerance` is in the same unit as the signal (encoded, unit interval;
/// e.g. 0.005 ≈ 0.5 IRE). An empty input is legal.
pub fn broadcast_legal(pixels: &[[f32; 3]], tolerance: f32) -> LegalReport {
    if pixels.is_empty() {
        return LegalReport {
            luma_over_pct: 0.0,
            luma_under_pct: 0.0,
            gamut_excursion_pct: 0.0,
            legal: true,
        };
    }
    let (mut over, mut under, mut gamut) = (0usize, 0usize, 0usize);
    let (lo, hi) = (-tolerance, 1.0 + tolerance);
    for p in pixels {
        let y = luma(p);
        over += (y > hi) as usize;
        under += (y < lo) as usize;
        gamut += !p.iter().all(|&c| c >= lo && c <= hi) as usize;
    }
    let pct = |n: usize| 100.0 * n as f32 / pixels.len() as f32;
    let r = LegalReport {
        luma_over_pct: pct(over),
        luma_under_pct: pct(under),
        gamut_excursion_pct: pct(gamut),
        legal: false,
    };
    let legal = r.luma_over_pct < 0.1 && r.luma_under_pct < 0.1 && r.gamut_excursion_pct < 0.1;
    LegalReport { legal, ..r }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_range_is_legal() {
        let px = vec![[0.0, 0.5, 1.0]; 100];
        let r = broadcast_legal(&px, 0.0);
        assert!(r.legal);
        assert_eq!(r.gamut_excursion_pct, 0.0);
    }

    #[test]
    fn out_of_range_is_flagged() {
        let mut px = vec![[0.5; 3]; 99];
        px.push([1.5, 1.5, 1.5]);
        let r = broadcast_legal(&px, 0.0);
        assert!(!r.legal);
        assert!((r.luma_over_pct - 1.0).abs() < 1e-4);
        assert!((r.gamut_excursion_pct - 1.0).abs() < 1e-4);
        assert_eq!(r.luma_under_pct, 0.0);

        let r = broadcast_legal(&[[-0.2; 3]], 0.0);
        assert_eq!(r.luma_under_pct, 100.0);
    }

    #[test]
    fn gamut_only_excursion_and_tolerance() {
        let px = [[1.2, 0.0, 0.0]; 10];
        let r = broadcast_legal(&px, 0.0);
        assert_eq!(r.luma_over_pct, 0.0);
        assert_eq!(r.gamut_excursion_pct, 100.0);
        assert!(!r.legal);
        assert!(broadcast_legal(&px, 0.25).legal);
    }

    #[test]
    fn empty_and_nan() {
        assert!(broadcast_legal(&[], 0.0).legal);
        assert!(!broadcast_legal(&[[f32::NAN; 3]], 0.0).legal);
    }
}
