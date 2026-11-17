use apricity_core::{convert, ColorSpace};
use glam::Vec3;
use serde::Serialize;

use crate::vectorscope::{cbcr, to_scope};

/// Named vectorscope target box centre in normalised scope coordinates.
#[derive(Debug, Clone, Serialize)]
pub struct VectorTarget {
    /// One of `"R"`, `"MG"`, `"B"`, `"CY"`, `"G"`, `"YL"`.
    pub label: &'static str,
    /// Horizontal position, `[0, 1]`, 0 = left (negative Cb).
    pub x: f32,
    /// Vertical position, `[0, 1]`, 0 = top (positive Cr).
    pub y: f32,
}

/// Overlay geometry for [`crate::vectorscope`], in the same normalised
/// coordinates (x right, y down, unit square).
#[derive(Debug, Clone, Serialize)]
pub struct Graticule {
    /// 75 %-amplitude colour-bar targets.
    pub targets: Vec<VectorTarget>,
    /// Angle of the skin-tone line in degrees, counter-clockwise from +Cb
    /// with Cr up (about 123°), derived from [`SKIN_TONE`].
    pub skin_tone_angle_deg: f32,
    /// Rec.709 gamut hexagon: 100 % R, Y, G, C, B, M in that order.
    pub rec709_gamut: Vec<[f32; 2]>,
    /// Display-P3 gamut polygon: the same six vertices in the same order,
    /// obtained from P3 primaries/secondaries converted to sRGB-encoded
    /// Rec.709 (sign-preserving, so out-of-gamut vertices stay out).
    pub p3_gamut: Vec<[f32; 2]>,
}

/// Representative light-to-medium skin tone, sRGB-encoded in `[0, 1]`.
/// Its Cb/Cr angle (123.3°) defines the skin-tone line.
pub const SKIN_TONE: [f32; 3] = [0.80, 0.58, 0.48];

/// Hue order shared by both gamut polygons: R, Y, G, C, B, M.
const HUES: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 1.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
];

fn point(p: &[f32; 3]) -> (f32, f32) {
    let (cb, cr) = cbcr(p);
    to_scope(cb, cr)
}

/// Build the vectorscope overlay geometry (see [`Graticule`]). Pure and
/// deterministic.
pub fn vectorscope_graticule() -> Graticule {
    let bars: [(&'static str, [f32; 3]); 6] = [
        ("R", [0.75, 0.0, 0.0]),
        ("MG", [0.75, 0.0, 0.75]),
        ("B", [0.0, 0.0, 0.75]),
        ("CY", [0.0, 0.75, 0.75]),
        ("G", [0.0, 0.75, 0.0]),
        ("YL", [0.75, 0.75, 0.0]),
    ];
    let targets = bars
        .iter()
        .map(|(label, rgb)| {
            let (x, y) = point(rgb);
            VectorTarget { label, x, y }
        })
        .collect();

    let (cb, cr) = cbcr(&SKIN_TONE);
    let skin_tone_angle_deg = cr.atan2(cb).to_degrees();

    let rec709_gamut = HUES
        .iter()
        .map(|h| {
            let (x, y) = point(h);
            [x, y]
        })
        .collect();
    let p3_gamut = HUES
        .iter()
        .map(|h| {
            let c = convert(
                Vec3::from_array(*h),
                ColorSpace::DisplayP3,
                ColorSpace::Srgb,
            );
            let (x, y) = point(&c.to_array());
            [x, y]
        })
        .collect();

    Graticule {
        targets,
        skin_tone_angle_deg,
        rec709_gamut,
        p3_gamut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_targets_and_vertices() {
        let g = vectorscope_graticule();
        let labels: Vec<_> = g.targets.iter().map(|t| t.label).collect();
        assert_eq!(labels, ["R", "MG", "B", "CY", "G", "YL"]);
        assert_eq!(g.rec709_gamut.len(), 6);
        assert_eq!(g.p3_gamut.len(), 6);
    }

    #[test]
    fn skin_line_is_about_123_degrees() {
        let a = vectorscope_graticule().skin_tone_angle_deg;
        assert!((a - 123.0).abs() < 1.5, "{a}");
    }

    #[test]
    fn red_is_upper_right_of_centre_blue_is_right_green_lower_left() {
        let g = vectorscope_graticule();
        let t = |l| g.targets.iter().find(|t| t.label == l).unwrap();
        assert!(t("R").y < 0.5 && t("R").x < 0.5);
        assert!(t("B").x > 0.5);
        assert!(t("G").x < 0.5 && t("G").y > 0.5);
    }

    #[test]
    fn p3_hexagon_is_larger_than_rec709() {
        let g = vectorscope_graticule();
        let area = |v: &[[f32; 2]]| {
            (0..v.len())
                .map(|i| {
                    let (a, b) = (v[i], v[(i + 1) % v.len()]);
                    a[0] * b[1] - b[0] * a[1]
                })
                .sum::<f32>()
                .abs()
                / 2.0
        };
        assert!(area(&g.p3_gamut) > area(&g.rec709_gamut));
    }

    #[test]
    fn rec709_vertices_inside_unit_square() {
        for p in vectorscope_graticule().rec709_gamut {
            assert!((0.0..=1.0).contains(&p[0]) && (0.0..=1.0).contains(&p[1]));
        }
    }
}
