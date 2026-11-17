// Shared declarations. Prepended (after the generated constant prelude)
// to every stage file. Layout of `Grade` must match
// `apricity_render::uniforms::GradeUniforms` byte for byte.

struct Grade {
    s1_flags: vec4<u32>,
    s1_matrix: array<vec4<f32>, 3>,
    s1_gain: vec4<f32>,
    s1_black: vec4<f32>,
    s2_flags: vec4<u32>,
    s2_gain: vec4<f32>,
    s2_tone: vec4<f32>,
    s2_tone2: vec4<f32>,
    s2_lift: vec4<f32>,
    s2_gamma: vec4<f32>,
    s2_gain_wheel: vec4<f32>,
    s2_hsl: array<vec4<f32>, 9>,
    s2_split_shadow: vec4<f32>,
    s2_split_highlight: vec4<f32>,
    s2_vignette: vec4<f32>,
    s2_vignette2: vec4<f32>,
    s2_grain: vec4<f32>,
    s2_grain_seed: vec4<u32>,
    out_flags: vec4<u32>,
    out_matrix: array<vec4<f32>, 3>,
    out_params: vec4<f32>,
    curves: array<vec4<f32>, BAKED_SIZE>,
};

fn row_mul(m: array<vec4<f32>, 3>, v: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(dot(m[0].xyz, v), dot(m[1].xyz, v), dot(m[2].xyz, v));
}

fn clamp01(v: vec3<f32>) -> vec3<f32> {
    return clamp(v, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn enc0(v: vec3<f32>) -> vec3<f32> {
    return sign(v) * pow(abs(v), vec3<f32>(WORKING_INV_GAMMA));
}

fn dec0(v: vec3<f32>) -> vec3<f32> {
    return sign(v) * pow(abs(v), vec3<f32>(WORKING_GAMMA));
}

fn lerp1(a: f32, b: f32, t: f32) -> f32 {
    return a + (b - a) * t;
}

fn mix1(a: f32, b: f32, t: f32) -> f32 {
    return a * (1.0 - t) + b * t;
}

fn mix3(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    return a * (1.0 - t) + b * t;
}

fn smooth1(e0: f32, e1: f32, x: f32) -> f32 {
    let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn srgb_encode1(l: f32) -> f32 {
    if (l <= 0.0031308) {
        return 12.92 * l;
    }
    return 1.055 * pow(l, 1.0 / 2.4) - 0.055;
}

fn srgb_decode1(e: f32) -> f32 {
    if (e <= 0.04045) {
        return e / 12.92;
    }
    return pow((e + 0.055) / 1.055, 2.4);
}

fn pow10(x: f32) -> f32 {
    return pow(10.0, x);
}
