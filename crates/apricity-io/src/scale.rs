use crate::DecodedImage;

/// Shrink `img` so its longer side is at most `max_dim` pixels, preserving
/// aspect ratio (each side at least 1 px).
///
/// Uses an area-weighted box filter on the **encoded** values (no
/// linearisation; adequate for scope/preview proxies) with alpha-weighted
/// color so transparent pixels do not bleed. Returns a clone when the image
/// already fits or `max_dim == 0`. The color tag is preserved.
pub fn downscale_to_fit(img: &DecodedImage, max_dim: u32) -> DecodedImage {
    let longest = img.width.max(img.height);
    if max_dim == 0 || longest <= max_dim || img.width == 0 || img.height == 0 {
        return img.clone();
    }
    let scale = max_dim as f64 / longest as f64;
    let nw = ((img.width as f64 * scale).round() as u32).clamp(1, max_dim);
    let nh = ((img.height as f64 * scale).round() as u32).clamp(1, max_dim);
    let (w, h) = (img.width as usize, img.height as usize);

    // Source span [lo, hi) of destination index i; spans always cover >= 1 px.
    let span = |i: u32, n: u32, len: usize| {
        let lo = i as usize * len / n as usize;
        let hi = ((i as usize + 1) * len / n as usize).max(lo + 1).min(len);
        (lo, hi)
    };

    let mut out = Vec::with_capacity(nw as usize * nh as usize * 4);
    for dy in 0..nh {
        let (y0, y1) = span(dy, nh, h);
        for dx in 0..nw {
            let (x0, x1) = span(dx, nw, w);
            let (mut r, mut g, mut b, mut a) = (0u64, 0u64, 0u64, 0u64);
            for y in y0..y1 {
                for p in img.rgba8[(y * w + x0) * 4..(y * w + x1) * 4].chunks_exact(4) {
                    let pa = p[3] as u64;
                    r += p[0] as u64 * pa;
                    g += p[1] as u64 * pa;
                    b += p[2] as u64 * pa;
                    a += pa;
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u64;
            if a == 0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let rnd = |v: u64| ((v + a / 2) / a) as u8;
                out.extend_from_slice(&[rnd(r), rnd(g), rnd(b), ((a + n / 2) / n) as u8]);
            }
        }
    }
    DecodedImage {
        width: nw,
        height: nh,
        rgba8: out,
        color_space: img.color_space,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apricity_core::ColorSpace;

    fn img(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> DecodedImage {
        DecodedImage {
            width: w,
            height: h,
            rgba8: (0..w * h).flat_map(|i| f(i % w, i / w)).collect(),
            color_space: ColorSpace::DisplayP3,
        }
    }

    #[test]
    fn dims_preserve_aspect() {
        let d = downscale_to_fit(&img(400, 200, |_, _| [9; 4]), 100);
        assert_eq!((d.width, d.height), (100, 50));
        assert_eq!(d.rgba8.len(), 100 * 50 * 4);
        let d = downscale_to_fit(&img(100, 1000, |_, _| [9; 4]), 10);
        assert_eq!((d.width, d.height), (1, 10));
        let d = downscale_to_fit(&img(1000, 3, |_, _| [9; 4]), 10);
        assert_eq!((d.width, d.height), (10, 1));
    }

    #[test]
    fn noop_when_fits_or_zero() {
        let a = img(8, 4, |x, y| [x as u8, y as u8, 0, 255]);
        assert_eq!(downscale_to_fit(&a, 8), a);
        assert_eq!(downscale_to_fit(&a, 100), a);
        assert_eq!(downscale_to_fit(&a, 0), a);
    }

    #[test]
    fn box_filter_averages_and_keeps_tag() {
        let a = img(4, 2, |x, _| {
            if x % 2 == 0 {
                [0, 0, 0, 255]
            } else {
                [200, 100, 50, 255]
            }
        });
        let d = downscale_to_fit(&a, 2);
        assert_eq!((d.width, d.height), (2, 1));
        assert_eq!(&d.rgba8[..4], &[100, 50, 25, 255]);
        assert_eq!(d.color_space, ColorSpace::DisplayP3);
    }

    #[test]
    fn uniform_stays_uniform_and_transparent_does_not_bleed() {
        let a = img(7, 5, |_, _| [10, 20, 30, 255]);
        let d = downscale_to_fit(&a, 3);
        assert!(d.rgba8.chunks_exact(4).all(|p| p == [10, 20, 30, 255]));
        let b = img(2, 1, |x, _| {
            if x == 0 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 0]
            }
        });
        let d = downscale_to_fit(&b, 1);
        assert_eq!(&d.rgba8[..4], &[255, 0, 0, 128]);
    }
}
