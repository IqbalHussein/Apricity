use crate::luma;
use crate::raster::{alpha_for, reference_count, ScopeImage};

/// Luma trace colour: neutral cyan-white, `sRGB`-encoded 8-bit.
const LUMA_TRACE: [u8; 3] = [200, 255, 255];

/// Parade panel tints (Okabe-Ito inspired, color-blind safe): vermillion
/// `#D55E00`, bluish green `#009E73`, sky blue `#56B4E9`. Panels are also
/// separated spatially, so colour is never the only cue.
const PARADE_TINTS: [[u8; 3]; 3] = [[213, 94, 0], [0, 158, 115], [86, 180, 233]];

#[derive(Clone, Copy)]
enum Mode {
    Luma,
    Parade,
}

/// Luma waveform of `pixels` (`width × height`, output-encoded RGB).
///
/// X is the source column binned into `out_w`; Y is `Y'` in `[0, 1]` with
/// 1.0 at the **top** row. Values outside `[0, 1]` (including NaN, which
/// counts as 0) clamp to the edge rows. Intensity is a log curve of hit
/// counts normalised by the 99th percentile of nonzero bins. Returns an
/// `out_w × out_h` image; empty if any dimension is 0.
pub fn waveform_luma(
    pixels: &[[f32; 3]],
    width: u32,
    height: u32,
    out_w: u32,
    out_h: u32,
) -> ScopeImage {
    build(pixels, width, height, out_w, out_h, Mode::Luma)
}

/// RGB parade: three side-by-side waveform panels (R, G, B) of the encoded
/// channel values, each `out_w / 3` wide (integer division). The returned
/// image is `3 * (out_w / 3)` wide and `out_h` tall. Value mapping and
/// intensity as in [`waveform_luma`].
pub fn rgb_parade(
    pixels: &[[f32; 3]],
    width: u32,
    height: u32,
    out_w: u32,
    out_h: u32,
) -> ScopeImage {
    build(pixels, width, height, out_w, out_h, Mode::Parade)
}

#[inline]
fn row_for(v: f32, max_row: usize) -> usize {
    let v = if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
    ((1.0 - v) * max_row as f32).round() as usize
}

fn build(
    pixels: &[[f32; 3]],
    width: u32,
    height: u32,
    out_w: u32,
    out_h: u32,
    mode: Mode,
) -> ScopeImage {
    let panels = match mode {
        Mode::Luma => 1usize,
        Mode::Parade => 3,
    };
    let pw = out_w as usize / panels;
    let total_w = pw * panels;
    if width == 0 || height == 0 || pw == 0 || out_h == 0 {
        return ScopeImage::empty(total_w as u32, out_h);
    }
    let (w, oh) = (width as usize, out_h as usize);
    let max_row = oh - 1;
    let mut counts = vec![0u32; total_w * oh];

    // Column → output bin is hoisted out of the per-pixel loop.
    let col_bin: Vec<usize> = (0..w).map(|c| c * pw / w).collect();
    let n = w * height as usize;
    for (i, p) in pixels.iter().take(n).enumerate() {
        let bin = col_bin[i % w];
        match mode {
            Mode::Luma => {
                counts[row_for(luma(p), max_row) * total_w + bin] += 1;
            }
            Mode::Parade => {
                for (k, &v) in p.iter().enumerate() {
                    counts[row_for(v, max_row) * total_w + k * pw + bin] += 1;
                }
            }
        }
    }

    let mut img = ScopeImage::empty(total_w as u32, out_h);
    let Some(reference) = reference_count(&counts) else {
        return img;
    };
    for (idx, &c) in counts.iter().enumerate() {
        if c == 0 {
            continue;
        }
        let tint = match mode {
            Mode::Luma => LUMA_TRACE,
            Mode::Parade => PARADE_TINTS[(idx % total_w) / pw],
        };
        let o = idx * 4;
        img.rgba[o..o + 3].copy_from_slice(&tint);
        img.rgba[o + 3] = alpha_for(c, reference);
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_with_energy(img: &ScopeImage) -> Vec<usize> {
        let w = img.width as usize;
        (0..img.height as usize)
            .filter(|&y| (0..w).any(|x| img.rgba[(y * w + x) * 4 + 3] > 0))
            .collect()
    }

    #[test]
    fn flat_grey_lands_on_one_row() {
        let px = vec![[0.5f32; 3]; 64 * 32];
        let img = waveform_luma(&px, 64, 32, 64, 101);
        assert_eq!(rows_with_energy(&img), vec![50]);
    }

    #[test]
    fn white_at_top_black_at_bottom_and_out_of_range_clamps() {
        let px = [[1.0; 3], [0.0; 3], [2.0; 3], [-1.0; 3]];
        let img = waveform_luma(&px, 4, 1, 4, 11);
        assert_eq!(rows_with_energy(&img), vec![0, 10]);
    }

    #[test]
    fn parade_panel_widths_and_channel_rows() {
        let px = vec![[1.0, 0.5, 0.0]; 33 * 4];
        let img = rgb_parade(&px, 33, 4, 100, 21);
        assert_eq!(img.width, 99);
        let at = |x: usize, y: usize| img.rgba[(y * 99 + x) * 4 + 3];
        assert!(at(10, 0) > 0, "red panel, top row");
        assert!(at(33 + 10, 10) > 0, "green panel, mid row");
        assert!(at(66 + 10, 20) > 0, "blue panel, bottom row");
        assert_eq!(at(10, 20), 0);
        assert_eq!(&img.rgba[(10 * 4)..(10 * 4 + 3)], &PARADE_TINTS[0]);
    }

    #[test]
    fn deterministic_and_degenerate_inputs() {
        let px: Vec<[f32; 3]> = (0..100).map(|i| [i as f32 / 99.0; 3]).collect();
        assert_eq!(
            waveform_luma(&px, 10, 10, 20, 20),
            waveform_luma(&px, 10, 10, 20, 20)
        );
        assert!(waveform_luma(&[], 0, 0, 10, 10)
            .rgba
            .iter()
            .all(|&b| b == 0));
        assert_eq!(rgb_parade(&px, 10, 10, 2, 5).width, 0);
        let nan = [[f32::NAN; 3]];
        assert_eq!(
            rows_with_energy(&waveform_luma(&nan, 1, 1, 4, 11)),
            vec![10]
        );
    }

    #[test]
    fn short_pixel_slice_does_not_panic() {
        let px = vec![[0.3f32; 3]; 5];
        let _ = waveform_luma(&px, 10, 10, 10, 10);
    }
}
