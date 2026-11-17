//! Tone-curve interpolation and baking.
//!
//! Curves are interpolated with a **monotone cubic Hermite spline**
//! (Fritsch–Carlson tangents), so a monotone set of control points never
//! overshoots — no ringing near the endpoints, which matters for tone curves
//! where overshoot produces posterization.
//!
//! The CPU reference pipeline and the GPU shader both consume the **baked**
//! table from [`bake`] and sample it with [`sample_baked`]'s linear
//! interpolation, so CPU/GPU parity depends only on that one function.
//!
//! Domain: the working encoding (see [`crate::working_encode`]), [0, 1].

use crate::grade_parameters::CurvePoint;

/// Number of entries in a baked curve table.
pub const BAKED_SIZE: usize = 256;

/// Sanitize control points: drop non-finite values, clamp to [0, 1], sort by
/// `x`, merge points closer than 1e-4 in `x` (keeping the later one), and
/// cap at `max_points`. An empty or single-point result becomes the
/// two-point identity.
pub fn sanitize(points: &[CurvePoint], max_points: usize) -> Vec<CurvePoint> {
    let mut pts: Vec<CurvePoint> = points
        .iter()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .map(|p| CurvePoint::new(p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0)))
        .collect();
    pts.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut out: Vec<CurvePoint> = Vec::with_capacity(pts.len());
    for p in pts {
        match out.last_mut() {
            Some(last) if (p.x - last.x).abs() < 1e-4 => *last = p,
            _ => out.push(p),
        }
    }
    out.truncate(max_points);
    if out.len() < 2 {
        return crate::grade_parameters::identity_curve();
    }
    out
}

/// Evaluate the monotone cubic through `points` (must be sanitized) at `x`.
/// Outside the first/last control point the curve is held flat.
pub fn evaluate(points: &[CurvePoint], x: f32) -> f32 {
    let n = points.len();
    if n == 0 {
        return x;
    }
    if n == 1 {
        return points[0].y;
    }
    if x <= points[0].x {
        return points[0].y;
    }
    if x >= points[n - 1].x {
        return points[n - 1].y;
    }
    let tangents = tangents(points);
    let mut k = 0;
    while k + 1 < n && points[k + 1].x < x {
        k += 1;
    }
    let (p0, p1) = (points[k], points[k + 1]);
    let h = p1.x - p0.x;
    let t = (x - p0.x) / h;
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * p0.y + h10 * h * tangents[k] + h01 * p1.y + h11 * h * tangents[k + 1]
}

fn tangents(points: &[CurvePoint]) -> Vec<f32> {
    let n = points.len();
    let secants: Vec<f32> = points
        .windows(2)
        .map(|w| (w[1].y - w[0].y) / (w[1].x - w[0].x))
        .collect();
    let mut m = vec![0.0f32; n];
    m[0] = secants[0];
    m[n - 1] = secants[n - 2];
    for i in 1..n - 1 {
        if secants[i - 1] * secants[i] <= 0.0 {
            m[i] = 0.0;
        } else {
            m[i] = (secants[i - 1] + secants[i]) * 0.5;
        }
    }
    // Fritsch–Carlson: limit tangents so each segment stays monotone.
    for i in 0..n - 1 {
        let d = secants[i];
        if d == 0.0 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
            continue;
        }
        let a = m[i] / d;
        let b = m[i + 1] / d;
        let s = a * a + b * b;
        if s > 9.0 {
            let tau = 3.0 / s.sqrt();
            m[i] = tau * a * d;
            m[i + 1] = tau * b * d;
        }
    }
    m
}

/// Bake a curve to a [`BAKED_SIZE`]-entry table over x ∈ [0, 1]. An empty
/// point list bakes to the exact identity table.
pub fn bake(points: &[CurvePoint]) -> [f32; BAKED_SIZE] {
    let mut out = [0.0f32; BAKED_SIZE];
    if points.is_empty() {
        for (i, v) in out.iter_mut().enumerate() {
            *v = i as f32 / (BAKED_SIZE - 1) as f32;
        }
        return out;
    }
    let pts = sanitize(points, usize::MAX);
    for (i, v) in out.iter_mut().enumerate() {
        *v = evaluate(&pts, i as f32 / (BAKED_SIZE - 1) as f32);
    }
    out
}

/// Sample a baked table at `x` with linear interpolation. Inputs above 1.0
/// continue with slope 1 from the last entry (preserving super-white
/// headroom); inputs below 0.0 continue with slope 1 from the first entry.
///
/// **This exact function is mirrored in `stage2.wgsl`.**
#[inline]
pub fn sample_baked(table: &[f32; BAKED_SIZE], x: f32) -> f32 {
    if x <= 0.0 {
        return table[0] + x;
    }
    if x >= 1.0 {
        return table[BAKED_SIZE - 1] + (x - 1.0);
    }
    let f = x * (BAKED_SIZE - 1) as f32;
    let i = (f.floor() as usize).min(BAKED_SIZE - 2);
    let t = f - i as f32;
    table[i] + (table[i + 1] - table[i]) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn empty_bakes_to_identity() {
        let t = bake(&[]);
        for (i, v) in t.iter().enumerate() {
            assert_eq!(*v, i as f32 / 255.0);
        }
        assert_eq!(sample_baked(&t, 0.5), 0.5);
    }

    #[test]
    fn two_point_identity_bakes_to_identity() {
        let t = bake(&crate::grade_parameters::identity_curve());
        for (i, v) in t.iter().enumerate() {
            assert!((v - i as f32 / 255.0).abs() < 1e-6);
        }
    }

    #[test]
    fn passes_through_control_points() {
        let pts = vec![
            CurvePoint::new(0.0, 0.05),
            CurvePoint::new(0.25, 0.2),
            CurvePoint::new(0.75, 0.85),
            CurvePoint::new(1.0, 0.95),
        ];
        for p in &pts {
            assert!((evaluate(&pts, p.x) - p.y).abs() < 1e-5);
        }
    }

    #[test]
    fn sanitize_sorts_dedupes_and_pads() {
        let s = sanitize(
            &[
                CurvePoint::new(0.8, 0.9),
                CurvePoint::new(f32::NAN, 0.1),
                CurvePoint::new(0.2, 2.0),
            ],
            16,
        );
        assert_eq!(
            s,
            vec![CurvePoint::new(0.2, 1.0), CurvePoint::new(0.8, 0.9)]
        );
        assert_eq!(sanitize(&[], 16).len(), 2);
    }

    #[test]
    fn sample_extrapolates_with_unit_slope() {
        let t = bake(&[]);
        assert!((sample_baked(&t, 1.5) - 1.5).abs() < 1e-6);
        assert!((sample_baked(&t, -0.25) + 0.25).abs() < 1e-6);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        #[test]
        fn monotone_points_give_monotone_curve(
            mut ys in proptest::collection::vec(0.0f32..1.0, 3..8)
        ) {
            ys.sort_by(f32::total_cmp);
            let n = ys.len();
            let pts: Vec<CurvePoint> = ys.iter().enumerate()
                .map(|(i, y)| CurvePoint::new(i as f32 / (n - 1) as f32, *y))
                .collect();
            let t = bake(&pts);
            for w in t.windows(2) {
                prop_assert!(w[1] >= w[0] - 1e-5, "non-monotone: {:?}", w);
            }
        }
    }
}
