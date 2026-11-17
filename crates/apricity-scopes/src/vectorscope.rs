use crate::raster::{alpha_for, reference_count, ScopeImage};

/// Scope scale `S`: scope coordinate = `0.5 ± C * S` for chroma `C` in
/// BT.709 Cb/Cr units. 100 % primaries and secondaries land at radius
/// 0.40 to 0.48 from the centre (green/magenta are the extremes), leaving a
/// margin inside the unit square.
pub(crate) const SCALE: f32 = 0.8;

/// BT.709 `(Cb, Cr)` of output-encoded RGB: `Cb = (B'-Y')/1.8556`,
/// `Cr = (R'-Y')/1.5748`, each nominally in `[-0.5, 0.5]`.
#[inline]
pub(crate) fn cbcr(p: &[f32; 3]) -> (f32, f32) {
    let y = crate::luma(p);
    ((p[2] - y) / 1.8556, (p[0] - y) / 1.5748)
}

/// Map `(Cb, Cr)` to normalised scope coordinates (x right, y down).
#[inline]
pub(crate) fn to_scope(cb: f32, cr: f32) -> (f32, f32) {
    (0.5 + cb * SCALE, 0.5 - cr * SCALE)
}

/// Vectorscope of output-encoded RGB `pixels` as a `size × size` image.
///
/// Cb runs right, Cr runs up (BT.709 chroma, see [`SCALE`](self) docs and
/// [`crate::vectorscope_graticule`]); the centre is neutral. Points outside
/// the square are clamped to its border. Each bin is tinted by the mean
/// hue of the pixels that landed in it (normalised to full brightness, then
/// lightened 25 % toward white) so colours remain readable on dark
/// backgrounds; alpha carries the log-scaled density. Empty if `size == 0`.
pub fn vectorscope(pixels: &[[f32; 3]], size: u32) -> ScopeImage {
    let s = size as usize;
    if s == 0 {
        return ScopeImage::empty(0, 0);
    }
    let mut counts = vec![0u32; s * s];
    let mut sums = vec![[0f32; 3]; s * s];
    let max = (s - 1) as f32;
    for p in pixels {
        let (cb, cr) = cbcr(p);
        let (x, y) = to_scope(cb, cr);
        // NaN falls through clamp unchanged and `as usize` maps it to 0.
        let xi = (x.clamp(0.0, 1.0) * max).round() as usize;
        let yi = (y.clamp(0.0, 1.0) * max).round() as usize;
        let i = yi * s + xi;
        counts[i] += 1;
        for (acc, v) in sums[i].iter_mut().zip(p) {
            *acc += v.clamp(0.0, 1.0);
        }
    }
    let mut img = ScopeImage::empty(size, size);
    let Some(reference) = reference_count(&counts) else {
        return img;
    };
    for ((&c, sum), px) in counts.iter().zip(&sums).zip(img.rgba.chunks_exact_mut(4)) {
        if c == 0 {
            continue;
        }
        let peak = sum.iter().fold(0f32, |a, &b| a.max(b));
        for (out, &s) in px.iter_mut().zip(sum) {
            let hue = if peak > 1e-6 { s / peak } else { 1.0 };
            *out = (255.0 * (0.75 * hue + 0.25)).round() as u8;
        }
        px[3] = alpha_for(c, reference);
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vectorscope_graticule;

    fn hit(img: &ScopeImage) -> Vec<(f32, f32)> {
        let s = img.width as usize;
        let mut v = vec![];
        for y in 0..s {
            for x in 0..s {
                if img.rgba[(y * s + x) * 4 + 3] > 0 {
                    v.push((x as f32 / (s - 1) as f32, y as f32 / (s - 1) as f32));
                }
            }
        }
        v
    }

    #[test]
    fn grey_is_at_centre() {
        let img = vectorscope(&vec![[0.4; 3]; 50], 101);
        assert_eq!(hit(&img), vec![(0.5, 0.5)]);
    }

    #[test]
    fn pure_red_lands_on_red_target() {
        let img = vectorscope(&vec![[1.0, 0.0, 0.0]; 50], 201);
        let g = vectorscope_graticule();
        let t = g.targets.iter().find(|t| t.label == "R").unwrap();
        // 100 % red is further out than the 75 % target, on the same ray.
        let h = hit(&img);
        assert_eq!(h.len(), 1);
        let (px, py) = (h[0].0 - 0.5, h[0].1 - 0.5);
        let (tx, ty) = (t.x - 0.5, t.y - 0.5);
        let cross = px * ty - py * tx;
        assert!(cross.abs() < 1e-3, "not on the R ray: {cross}");
        assert!(px * tx + py * ty > 0.0);
        assert!(px.hypot(py) > tx.hypot(ty));
        let (xi, yi) = (
            (h[0].0 * 200.0).round() as usize,
            (h[0].1 * 200.0).round() as usize,
        );
        let o = (yi * 201 + xi) * 4;
        assert!(img.rgba[o] > img.rgba[o + 2], "tinted toward red");
    }

    #[test]
    fn out_of_range_clamps_and_empty_is_transparent() {
        let img = vectorscope(&[[20.0, -20.0, 0.0]], 32);
        assert_eq!(hit(&img).len(), 1);
        assert!(vectorscope(&[], 8).rgba.iter().all(|&b| b == 0));
        assert_eq!(vectorscope(&[[0.1; 3]], 0).rgba.len(), 0);
    }
}
