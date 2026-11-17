use serde::Serialize;

use crate::luma;

/// 256-bin histograms over `[0, 1]` of output-encoded values.
///
/// Bin `i` covers `[i/256, (i+1)/256)`; the last bin also holds 1.0. Values
/// outside `[0, 1]` (and NaN, as 0) are clamped into the end bins. Every
/// vector sums to the pixel count.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Histogram {
    /// Red `R'` counts.
    pub red: Vec<u32>,
    /// Green `G'` counts.
    pub green: Vec<u32>,
    /// Blue `B'` counts.
    pub blue: Vec<u32>,
    /// Rec.709 luma `Y'` counts.
    pub luma: Vec<u32>,
}

#[inline]
fn bin(v: f32) -> usize {
    if v.is_nan() {
        0
    } else {
        (v.clamp(0.0, 1.0) * 256.0).min(255.0) as usize
    }
}

/// Histogram of output-encoded RGB `pixels` (see [`Histogram`]).
pub fn histogram(pixels: &[[f32; 3]]) -> Histogram {
    let mut h = Histogram {
        red: vec![0; 256],
        green: vec![0; 256],
        blue: vec![0; 256],
        luma: vec![0; 256],
    };
    for p in pixels {
        h.red[bin(p[0])] += 1;
        h.green[bin(p[1])] += 1;
        h.blue[bin(p[2])] += 1;
        h.luma[bin(luma(p))] += 1;
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_sum_to_pixel_count_and_clamp() {
        let px = [[0.0; 3], [1.0; 3], [-3.0; 3], [9.0; 3], [0.5; 3]];
        let h = histogram(&px);
        for v in [&h.red, &h.green, &h.blue, &h.luma] {
            assert_eq!(v.iter().sum::<u32>(), 5);
            assert_eq!(v[0], 2);
            assert_eq!(v[255], 2);
            assert_eq!(v[128], 1);
        }
    }

    #[test]
    fn empty_input_is_all_zero() {
        assert!(histogram(&[]).luma.iter().all(|&c| c == 0));
    }
}
