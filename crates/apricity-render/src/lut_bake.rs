//! Bake a grade into a 3D LUT (`.cube` export, PRD §6.5).
//!
//! The lattice follows the `apricity-lut` contract: input and output are
//! **sRGB-encoded Rec.709 in [0, 1]**. Spatial effects (vignette, grain) are
//! position dependent and cannot live in a LUT, so they are disabled.

use apricity_core::{convert, ColorSpace, GradeParameters};
use apricity_lut::Lut3d;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::cpu;
use crate::uniforms::{prepare, RenderContext};

/// Which part of the pipeline a baked LUT contains.
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BakeScope {
    /// Stage 2 only. The lattice is treated as an already-corrected,
    /// display-encoded sRGB Rec.709 image.
    Creative,
    /// Stage 1 (including the input log profile) and Stage 2. The lattice is
    /// treated as a source pixel, so a log-profile grade bakes a log-to-display LUT.
    Full,
}

/// Bake `params` into a `size`-point cube (clamped to at least 2).
///
/// Input and output lattice values are sRGB-encoded Rec.709 in [0, 1]; the
/// output is clamped to [0, 1]. Vignette and grain are disabled. `title`
/// becomes the `.cube` `TITLE`.
pub fn bake_lut(
    params: &GradeParameters,
    size: usize,
    scope: BakeScope,
    title: Option<String>,
) -> Lut3d {
    let mut p = params.clone();
    p.stage2.vignette.amount = 0.0;
    p.stage2.grain.amount = 0.0;
    let u = prepare(&p, &RenderContext::srgb(1, 1));
    let px = Vec2::splat(0.5);
    let mut lut = Lut3d::from_fn(size, |e| {
        let e = Vec3::from_array(e);
        let out = match scope {
            BakeScope::Creative => {
                let lin = convert(e, ColorSpace::Srgb, ColorSpace::LinearRec2020);
                cpu::output(&u, cpu::stage2(&u, None, lin, px))
            }
            BakeScope::Full => cpu::shade(&u, None, e, px),
        };
        out.clamp(Vec3::ZERO, Vec3::ONE).to_array()
    });
    lut.title = title;
    lut
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grade() -> GradeParameters {
        let mut p = GradeParameters::IDENTITY;
        p.stage1.temperature = -8.0;
        p.stage2.contrast = 8.0;
        p.stage2.saturation = 8.0;
        p.stage2.lift.b = 0.01;
        p.stage2.vignette.amount = -30.0;
        p.stage2.grain.amount = 20.0;
        p
    }

    #[test]
    fn identity_bakes_to_identity() {
        let id = Lut3d::identity(17);
        for scope in [BakeScope::Creative, BakeScope::Full] {
            let l = bake_lut(&GradeParameters::IDENTITY, 17, scope, None);
            for (a, b) in l.data.iter().zip(&id.data) {
                for c in 0..3 {
                    assert!((a[c] - b[c]).abs() < 1e-4, "{scope:?} {a:?} {b:?}");
                }
            }
        }
    }

    #[test]
    fn baked_lut_matches_shade() {
        let p = grade();
        let lut = bake_lut(&p, 33, BakeScope::Full, None);
        let mut q = p.clone();
        q.stage2.vignette.amount = 0.0;
        q.stage2.grain.amount = 0.0;
        let u = prepare(&q, &RenderContext::srgb(1, 1));
        let mut s = 12345u32;
        let mut next = || {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            (s >> 8) as f32 / (1u32 << 24) as f32
        };
        for _ in 0..200 {
            let c = [0.3 + 0.4 * next(), 0.3 + 0.4 * next(), 0.3 + 0.4 * next()];
            let want = cpu::shade(&u, None, Vec3::from_array(c), Vec2::splat(0.5))
                .clamp(Vec3::ZERO, Vec3::ONE);
            let got = lut.sample_trilinear(c);
            for i in 0..3 {
                assert!(
                    (got[i] - want[i]).abs() <= 2.0 / 255.0,
                    "{c:?} {got:?} {want:?}"
                );
            }
        }
    }

    #[test]
    fn title_propagates() {
        let l = bake_lut(&grade(), 5, BakeScope::Creative, Some("Warm".into()));
        assert_eq!(l.title.as_deref(), Some("Warm"));
        assert_eq!(l.size, 5);
    }
}
