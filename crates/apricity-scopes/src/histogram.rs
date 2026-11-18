use serde::Serialize;

use crate::{luma, unit};

/// 256-bin histograms over `[0, 1]` of output-encoded values.
///
/// Bin `i` covers `[i/256, (i+1)/256)`; the last bin also holds 1.0. Values
/// outside `[0, 1]` (and NaN, as 0) are clamped into the end bins. Every
/// vector sums to the pixel count.
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
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
    ((unit(v) * 256.0) as usize).min(255)
}

/// Histogram of output-encoded RGB `pixels` (see [`Histogram`]).
pub fn histogram(pixels: &[[f32; 3]]) -> Histogram {
    // Two interleaved tables break the store-to-load dependency chain that
    // smooth gradients create by hitting the same bin on consecutive pixels.
    let mut t = [[[0u32; 256]; 4]; 2];
    let add = |t: &mut [[u32; 256]; 4], p: &[f32; 3]| {
        t[0][bin(p[0])] += 1;
        t[1][bin(p[1])] += 1;
        t[2][bin(p[2])] += 1;
        t[3][bin(luma(p))] += 1;
    };
    let mut pairs = pixels.chunks_exact(2);
    for pair in &mut pairs {
        add(&mut t[0], &pair[0]);
        add(&mut t[1], &pair[1]);
    }
    for p in pairs.remainder() {
        add(&mut t[0], p);
    }
    let sum = |c: usize| (0..256).map(|i| t[0][c][i] + t[1][c][i]).collect();
    Histogram {
        red: sum(0),
        green: sum(1),
        blue: sum(2),
        luma: sum(3),
    }
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
