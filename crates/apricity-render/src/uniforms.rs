//! [`GradeUniforms`]: the derived, render-ready form of [`GradeParameters`].
//!
//! Both the CPU reference ([`crate::cpu`]) and the WGSL shaders consume this
//! exact struct, so CPU/GPU parity reduces to "the shader mirrors
//! `cpu::shade`". Every member is a `vec4` (or array of `vec4`) so the
//! Rust `repr(C)` layout and the WGSL uniform layout agree byte-for-byte.
//!
//! Anything expensive or branchy that depends only on parameters (white
//! balance Planckian math, curve baking, matrices) is computed here once
//! per parameter change, never per pixel.

use apricity_core::{
    curve, log_profiles::*, white_balance::gains, ColorSpace, GradeParameters, HslBands, LogProfile,
};
use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Vec3};

/// Preview-only overlay drawn after the output transform.
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Overlay {
    /// No overlay.
    #[default]
    None,
    /// Exposure zones mapped to a color key.
    FalseColor,
    /// Diagonal stripes over pixels above the zebra threshold.
    Zebra,
}

/// Non-grade inputs to [`prepare`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderContext {
    /// Size in pixels of the source image being sampled (the proxy for
    /// preview, the full image for export).
    pub image_size: [f32; 2],
    /// Full-resolution width ÷ `image_size[0]`. Grain is generated in
    /// full-resolution pixel coordinates so preview and export match.
    pub source_scale: f32,
    /// Output color space (encoding + primaries).
    pub output: ColorSpace,
    /// Preview overlay.
    pub overlay: Overlay,
    /// Zebra threshold as an output-encoded luma in [0, 1].
    pub zebra_threshold: f32,
    /// Size of the bound 3D LUT (0 = none).
    pub lut_size: u32,
}

impl RenderContext {
    /// A context for rendering a `w × h` image at full resolution to sRGB
    /// with no overlay and no LUT.
    pub fn srgb(w: u32, h: u32) -> Self {
        Self {
            image_size: [w as f32, h as f32],
            source_scale: 1.0,
            output: ColorSpace::Srgb,
            overlay: Overlay::None,
            zebra_threshold: 0.95,
            lut_size: 0,
        }
    }
}

/// Render-ready uniforms. Layout mirrored by `struct Grade` in
/// `shaders/common.wgsl`. See field docs there for units.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GradeUniforms {
    /// x: stage1 enabled, y: input profile id, z: apply log-to-display.
    pub s1_flags: [u32; 4],
    /// Input gamut → linear Rec.2020, row-major (xyz used).
    pub s1_matrix: [[f32; 4]; 3],
    /// Combined WB × exposure gain (rgb), scene-linear.
    pub s1_gain: [f32; 4],
    /// Black point (rgb), scene-linear.
    pub s1_black: [f32; 4],
    /// x: stage2 enabled, y: has HSL, z: has curves, w: has LUT.
    pub s2_flags: [u32; 4],
    /// rgb: creative exposure × WB gain; w: saturation factor.
    pub s2_gain: [f32; 4],
    /// x: contrast slope, y: highlights, z: shadows, w: whites (−1..1).
    pub s2_tone: [f32; 4],
    /// x: blacks, y: vibrance (−1..1), z: LUT mix (0..1), w: split balance (−1..1).
    pub s2_tone2: [f32; 4],
    /// Lift offsets (rgb), working encoding.
    pub s2_lift: [f32; 4],
    /// Gamma exponents (rgb).
    pub s2_gamma: [f32; 4],
    /// Gain multipliers (rgb).
    pub s2_gain_wheel: [f32; 4],
    /// Per band: hue shift (turns), saturation delta, luminance delta,
    /// center (turns). Index 8 repeats band 0 at center 1.0.
    pub s2_hsl: [[f32; 4]; 9],
    /// Shadow split-tone offset (rgb), working encoding.
    pub s2_split_shadow: [f32; 4],
    /// Highlight split-tone offset (rgb), working encoding.
    pub s2_split_highlight: [f32; 4],
    /// x: amount (−1..1), y: midpoint (0..1), z: feather (0..1), w: roundness (−1..1).
    pub s2_vignette: [f32; 4],
    /// x, y: center (−1..1), z: aspect (w/h).
    pub s2_vignette2: [f32; 4],
    /// x: amount (0..1), y: cell size (px), z: roughness (0..1), w: source scale.
    pub s2_grain: [f32; 4],
    /// x: seed.
    pub s2_grain_seed: [u32; 4],
    /// x: transfer (0 sRGB, 1 gamma 2.4, 2 linear), y: overlay, z: LUT size.
    pub out_flags: [u32; 4],
    /// Linear Rec.2020 → output linear primaries, row-major.
    pub out_matrix: [[f32; 4]; 3],
    /// x: zebra threshold, y: image width, z: image height.
    pub out_params: [f32; 4],
    /// Baked tone curves: (master, red, green, blue) per entry.
    pub curves: [[f32; 4]; curve::BAKED_SIZE],
}

/// Numeric id of a [`LogProfile`] used by the shader.
pub fn profile_id(p: LogProfile) -> u32 {
    match p {
        LogProfile::None => 0,
        LogProfile::ArriLogC3 => 1,
        LogProfile::SonySLog3 => 2,
        LogProfile::PanasonicVLog => 3,
        LogProfile::AppleLog => 4,
    }
}

fn rows(m: Mat3) -> [[f32; 4]; 3] {
    let t = m.transpose();
    [
        [t.x_axis.x, t.x_axis.y, t.x_axis.z, 0.0],
        [t.y_axis.x, t.y_axis.y, t.y_axis.z, 0.0],
        [t.z_axis.x, t.z_axis.y, t.z_axis.z, 0.0],
    ]
}

fn v4(v: Vec3, w: f32) -> [f32; 4] {
    [v.x, v.y, v.z, w]
}

/// Zero-luminance RGB offset for a hue (degrees), in the working encoding.
fn hue_offset(hue_deg: f32) -> Vec3 {
    let h = (hue_deg.rem_euclid(360.0)) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let c = match h as u32 {
        0 => Vec3::new(1.0, x, 0.0),
        1 => Vec3::new(x, 1.0, 0.0),
        2 => Vec3::new(0.0, 1.0, x),
        3 => Vec3::new(0.0, x, 1.0),
        4 => Vec3::new(x, 0.0, 1.0),
        _ => Vec3::new(1.0, 0.0, x),
    };
    c - Vec3::splat(c.dot(apricity_core::white_balance::REC2020_LUMA))
}

/// Linear Rec.2020 → output-space linear matrix and transfer id.
fn output_transform(cs: ColorSpace) -> (Mat3, u32) {
    let to = |target: ColorSpace| target.xyz_to_rgb() * ColorSpace::LinearRec2020.rgb_to_xyz();
    match cs {
        ColorSpace::Srgb => (to(ColorSpace::LinearSrgb), 0),
        ColorSpace::DisplayP3 => (to(ColorSpace::DisplayP3), 0),
        ColorSpace::Rec709 => (to(ColorSpace::LinearSrgb), 1),
        ColorSpace::LinearSrgb => (to(ColorSpace::LinearSrgb), 2),
        ColorSpace::Rec2020 => (Mat3::IDENTITY, 1),
        ColorSpace::LinearRec2020 => (Mat3::IDENTITY, 2),
    }
}

/// Derive uniforms from (already sanitized) parameters.
pub fn prepare(p: &GradeParameters, ctx: &RenderContext) -> GradeUniforms {
    let s1 = &p.stage1;
    let s2 = &p.stage2;
    let mut u = GradeUniforms::zeroed();

    // ---- Stage 1 ----
    let s1_gain = gains(s1.temperature, s1.tint) * 2f32.powf(s1.exposure);
    u.s1_flags = [
        s1.enabled as u32,
        profile_id(s1.input_profile),
        (s1.enabled && s1.input_profile.is_log() && s1.log_to_display) as u32,
        0,
    ];
    u.s1_matrix = rows(s1.input_profile.to_rec2020_matrix());
    u.s1_gain = v4(s1_gain, 0.0);
    u.s1_black = [
        s1.black_point.r.min(0.5),
        s1.black_point.g.min(0.5),
        s1.black_point.b.min(0.5),
        0.0,
    ];

    // ---- Stage 2 ----
    let hsl_bands = s2.hsl.as_array();
    let has_hsl = hsl_bands
        .iter()
        .any(|b| *b != apricity_core::HslBand::IDENTITY);
    let has_curves = !s2.tone_curve.is_identity();
    let has_lut = ctx.lut_size >= 2 && s2.lut_id.is_some() && s2.lut_strength > 0.0;
    u.s2_flags = [
        s2.enabled as u32,
        has_hsl as u32,
        has_curves as u32,
        has_lut as u32,
    ];
    let wb2 = gains(s2.temperature, s2.tint) * 2f32.powf(s2.exposure);
    u.s2_gain = v4(wb2, 1.0 + s2.saturation / 100.0);
    u.s2_tone = [
        2f32.powf(s2.contrast / 100.0),
        s2.highlights / 100.0,
        s2.shadows / 100.0,
        s2.whites / 100.0,
    ];
    u.s2_tone2 = [
        s2.blacks / 100.0,
        s2.vibrance / 100.0,
        if has_lut {
            s2.lut_strength / 100.0
        } else {
            0.0
        },
        s2.split_tone.balance / 100.0,
    ];
    let [lr, lg, lb] = s2.lift.effective();
    let [gr, gg, gb] = s2.gamma.effective();
    let [nr, ng, nb] = s2.gain.effective();
    u.s2_lift = [lr, lg, lb, 0.0];
    u.s2_gamma = [2f32.powf(-gr), 2f32.powf(-gg), 2f32.powf(-gb), 0.0];
    u.s2_gain_wheel = [1.0 + nr, 1.0 + ng, 1.0 + nb, 0.0];
    for i in 0..9 {
        let band = hsl_bands[i % 8];
        let center = if i == 8 {
            1.0
        } else {
            HslBands::CENTERS_DEG[i] / 360.0
        };
        u.s2_hsl[i] = [
            band.hue / 360.0,
            band.saturation / 100.0,
            band.luminance / 100.0,
            center,
        ];
    }
    let st = &s2.split_tone;
    u.s2_split_shadow = v4(
        hue_offset(st.shadow_hue) * (st.shadow_saturation / 100.0) * 0.2,
        0.0,
    );
    u.s2_split_highlight = v4(
        hue_offset(st.highlight_hue) * (st.highlight_saturation / 100.0) * 0.2,
        0.0,
    );
    let vg = &s2.vignette;
    u.s2_vignette = [
        vg.amount / 100.0,
        vg.midpoint / 100.0,
        vg.feather / 100.0,
        vg.roundness / 100.0,
    ];
    let aspect = if ctx.image_size[1] > 0.0 {
        ctx.image_size[0] / ctx.image_size[1]
    } else {
        1.0
    };
    u.s2_vignette2 = [vg.center_x / 100.0, vg.center_y / 100.0, aspect, 0.0];
    u.s2_grain = [
        s2.grain.amount / 100.0,
        s2.grain.size.max(0.25),
        s2.grain.roughness / 100.0,
        ctx.source_scale,
    ];
    u.s2_grain_seed = [s2.grain.seed, 0, 0, 0];

    let tables = s2.tone_curve.channels().map(|c| curve::bake(c));
    for (i, entry) in u.curves.iter_mut().enumerate() {
        *entry = [tables[0][i], tables[1][i], tables[2][i], tables[3][i]];
    }

    // ---- Output ----
    let (m, transfer) = output_transform(ctx.output);
    let overlay = match ctx.overlay {
        Overlay::None => 0,
        Overlay::FalseColor => 1,
        Overlay::Zebra => 2,
    };
    u.out_flags = [transfer, overlay, ctx.lut_size, 0];
    u.out_matrix = rows(m);
    u.out_params = [
        ctx.zebra_threshold,
        ctx.image_size[0],
        ctx.image_size[1],
        0.0,
    ];
    u
}

/// WGSL constant prelude generated from the same Rust constants the CPU
/// path uses, so both sides see bit-identical f32 values.
pub fn wgsl_constants() -> String {
    let l = apricity_core::white_balance::REC2020_LUMA;
    let m709 = ColorSpace::LinearSrgb.xyz_to_rgb() * ColorSpace::LinearRec2020.rgb_to_xyz();
    let m2020 = m709.inverse();
    let mat = |m: Mat3| {
        let c = m.to_cols_array();
        format!(
            "mat3x3<f32>({:e}, {:e}, {:e}, {:e}, {:e}, {:e}, {:e}, {:e}, {:e})",
            c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8]
        )
    };
    format!(
        "const LUMA2020: vec3<f32> = vec3<f32>({:e}, {:e}, {:e});\n\
         const WORKING_INV_GAMMA: f32 = {:e};\n\
         const WORKING_GAMMA: f32 = {:e};\n\
         const PIVOT: f32 = {:e};\n\
         const M_2020_TO_709: mat3x3<f32> = {};\n\
         const M_709_TO_2020: mat3x3<f32> = {};\n\
         const BAKED_SIZE: u32 = {}u;\n",
        l.x,
        l.y,
        l.z,
        1.0 / apricity_core::WORKING_GAMMA,
        apricity_core::WORKING_GAMMA,
        pivot(),
        mat(m709),
        mat(m2020),
        curve::BAKED_SIZE,
    )
}

/// Contrast pivot: mid grey (0.18 scene-linear) in the working encoding.
pub fn pivot() -> f32 {
    apricity_core::working_encode(0.18)
}

/// Matrices shared by the CPU LUT path (must equal [`wgsl_constants`]).
pub(crate) fn lut_matrices() -> (Mat3, Mat3) {
    let m709 = ColorSpace::LinearSrgb.xyz_to_rgb() * ColorSpace::LinearRec2020.rgb_to_xyz();
    (m709, m709.inverse())
}

// Keep the import list honest: these functions are mirrored in WGSL.
#[allow(dead_code)]
const _MIRRORED: [fn(f32) -> f32; 5] = [
    logc3_decode,
    slog3_decode,
    vlog_decode,
    apple_log_decode,
    display_render,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_vec4_aligned() {
        assert_eq!(std::mem::size_of::<GradeUniforms>() % 16, 0);
        assert_eq!(std::mem::size_of::<GradeUniforms>(), (33 + 256) * 16);
    }

    #[test]
    fn identity_flags() {
        let u = prepare(&GradeParameters::IDENTITY, &RenderContext::srgb(4, 4));
        assert_eq!(u.s2_flags, [1, 0, 0, 0]);
        assert_eq!(u.s1_gain, [1.0, 1.0, 1.0, 0.0]);
        assert_eq!(u.s2_tone[0], 1.0);
    }

    #[test]
    fn hue_offset_has_zero_luminance() {
        for h in [0.0, 45.0, 200.0, 330.0] {
            assert!(
                hue_offset(h)
                    .dot(apricity_core::white_balance::REC2020_LUMA)
                    .abs()
                    < 1e-6
            );
        }
    }
}
