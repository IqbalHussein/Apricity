// Stage 2 — creative grade. Mirrors `cpu::stage2`.

fn bell(l: f32, c: f32, w: f32) -> f32 {
    let d = (l - c) / w;
    return exp(-(d * d));
}

fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let mx = max(c.x, max(c.y, c.z));
    let mn = min(c.x, min(c.y, c.z));
    let d = mx - mn;
    var h = 0.0;
    if (d > 1e-6) {
        if (mx == c.x) {
            h = ((c.y - c.z) / d) / 6.0;
        } else if (mx == c.y) {
            h = ((c.z - c.x) / d + 2.0) / 6.0;
        } else {
            h = ((c.x - c.y) / d + 4.0) / 6.0;
        }
        h = h - floor(h);
    }
    var s = 0.0;
    if (mx > 1e-6) {
        s = d / mx;
    }
    return vec3<f32>(h, s, mx);
}

fn hsv_k(n: f32, h6: f32, hsv: vec3<f32>) -> f32 {
    let k = (n + h6) % 6.0;
    return hsv.z - hsv.z * hsv.y * clamp(min(k, 4.0 - k), 0.0, 1.0);
}

fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h6 = (hsv.x - floor(hsv.x)) * 6.0;
    return vec3<f32>(hsv_k(5.0, h6, hsv), hsv_k(3.0, h6, hsv), hsv_k(1.0, h6, hsv));
}

fn hsl_adjust(w: vec3<f32>) -> vec3<f32> {
    let c = clamp01(w);
    let hsv = rgb_to_hsv(c);
    if (hsv.y < 1e-5) {
        return w;
    }
    let h = hsv.x;
    var adj = vec3<f32>(0.0);
    for (var i = 0u; i < 8u; i = i + 1u) {
        let c0 = grade.s2_hsl[i].w;
        let c1 = grade.s2_hsl[i + 1u].w;
        if (h >= c0 && h < c1) {
            let t = smooth1(0.0, 1.0, (h - c0) / (c1 - c0));
            adj = mix3(grade.s2_hsl[i].xyz, grade.s2_hsl[i + 1u].xyz, t);
        }
    }
    let nh = h + adj.x;
    let ns = clamp(hsv.y * (1.0 + adj.y), 0.0, 1.0);
    let rgb = hsv_to_rgb(vec3<f32>(nh - floor(nh), ns, hsv.z));
    return w + (rgb - c) + vec3<f32>(adj.z * 0.2 * hsv.y);
}

fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash2(ix: i32, iy: i32, seed: u32) -> f32 {
    let h = pcg(bitcast<u32>(ix) ^ pcg(bitcast<u32>(iy) ^ pcg(seed)));
    return f32(h >> 8u) / 16777215.0 * 2.0 - 1.0;
}

fn value_noise(q: vec2<f32>, seed: u32) -> f32 {
    let i = floor(q);
    let f = q - i;
    let s = f * f * (vec2<f32>(3.0) - 2.0 * f);
    let ix = i32(i.x);
    let iy = i32(i.y);
    let a = hash2(ix, iy, seed);
    let b = hash2(ix + 1, iy, seed);
    let c = hash2(ix, iy + 1, seed);
    let d = hash2(ix + 1, iy + 1, seed);
    return mix1(mix1(a, b, s.x), mix1(c, d, s.x), s.y);
}

fn grain_noise(px: vec2<f32>) -> f32 {
    let q = px * grade.s2_grain.w / grade.s2_grain.y;
    let seed = grade.s2_grain_seed.x;
    let n1 = value_noise(q, seed);
    let n2 = value_noise(q * 2.0 + vec2<f32>(17.0), seed ^ 0x9E3779B9u);
    let r = grade.s2_grain.z;
    return (n1 + r * 0.5 * n2) / (1.0 + 0.5 * r);
}

fn curve_sample(ch: u32, x: f32) -> f32 {
    let n = BAKED_SIZE;
    if (x <= 0.0) {
        return grade.curves[0][ch] + x;
    }
    if (x >= 1.0) {
        return grade.curves[n - 1u][ch] + (x - 1.0);
    }
    let f = x * f32(n - 1u);
    let i = min(u32(floor(f)), n - 2u);
    let t = f - f32(i);
    return lerp1(grade.curves[i][ch], grade.curves[i + 1u][ch], t);
}

fn lut_at(r: i32, gg: i32, b: i32) -> vec3<f32> {
    return textureLoad(lut_tex, vec3<i32>(r, gg, b), 0).xyz;
}

fn lerp3(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    return a + (b - a) * t;
}

fn sample_lut(size: u32, rgb: vec3<f32>) -> vec3<f32> {
    let n1 = f32(size - 1u);
    let x = clamp01(rgb) * n1;
    let fl = floor(x);
    let i0 = vec3<i32>(fl);
    let i1 = min(i0 + vec3<i32>(1), vec3<i32>(i32(size) - 1));
    let f = x - fl;
    let c00 = lerp3(lut_at(i0.x, i0.y, i0.z), lut_at(i1.x, i0.y, i0.z), f.x);
    let c10 = lerp3(lut_at(i0.x, i1.y, i0.z), lut_at(i1.x, i1.y, i0.z), f.x);
    let c01 = lerp3(lut_at(i0.x, i0.y, i1.z), lut_at(i1.x, i0.y, i1.z), f.x);
    let c11 = lerp3(lut_at(i0.x, i1.y, i1.z), lut_at(i1.x, i1.y, i1.z), f.x);
    let c0 = lerp3(c00, c10, f.y);
    let c1 = lerp3(c01, c11, f.y);
    return lerp3(c0, c1, f.z);
}

fn stage2(lin_in: vec3<f32>, px: vec2<f32>) -> vec3<f32> {
    if (grade.s2_flags.x == 0u) {
        return lin_in;
    }
    var lin = lin_in * grade.s2_gain.xyz;

    let y = dot(lin, LUMA2020);
    let mx = max(lin.x, max(lin.y, lin.z));
    let mn = min(lin.x, min(lin.y, lin.z));
    var chroma = 0.0;
    if (mx > 1e-6) {
        chroma = clamp((mx - mn) / mx, 0.0, 1.0);
    }
    let sat = grade.s2_gain.w * (1.0 + grade.s2_tone2.y * (1.0 - chroma));
    lin = vec3<f32>(y) + (lin - vec3<f32>(y)) * sat;

    var w = enc0(lin);
    w = vec3<f32>(PIVOT) + (w - vec3<f32>(PIVOT)) * grade.s2_tone.x;

    var l = dot(clamp01(w), LUMA2020);
    let d = 0.25 * (grade.s2_tone2.x * bell(l, 0.05, 0.15)
        + grade.s2_tone.z * bell(l, 0.25, 0.2)
        + grade.s2_tone.y * bell(l, 0.75, 0.2)
        + grade.s2_tone.w * bell(l, 0.95, 0.15));
    w = w + vec3<f32>(d);

    w = w + grade.s2_lift.xyz * (vec3<f32>(1.0) - clamp01(w));
    w = w * grade.s2_gain_wheel.xyz;
    w = sign(w) * pow(abs(w), grade.s2_gamma.xyz);

    if (grade.s2_flags.y != 0u) {
        w = hsl_adjust(w);
    }

    l = dot(clamp01(w), LUMA2020);
    let sw = 1.0 - smooth1(0.0, 1.0, l + grade.s2_tone2.w * 0.5);
    w = w + grade.s2_split_shadow.xyz * sw + grade.s2_split_highlight.xyz * (1.0 - sw);

    if (grade.s2_flags.z != 0u) {
        w = vec3<f32>(
            curve_sample(1u, curve_sample(0u, w.x)),
            curve_sample(2u, curve_sample(0u, w.y)),
            curve_sample(3u, curve_sample(0u, w.z)));
    }

    if (grade.s2_flags.w != 0u) {
        let e709 = clamp01(M_2020_TO_709 * dec0(w));
        let e = vec3<f32>(srgb_encode1(e709.x), srgb_encode1(e709.y), srgb_encode1(e709.z));
        let o = sample_lut(grade.out_flags.z, e);
        let o709 = vec3<f32>(srgb_decode1(o.x), srgb_decode1(o.y), srgb_decode1(o.z));
        w = mix3(w, enc0(M_709_TO_2020 * o709), grade.s2_tone2.z);
    }

    lin = dec0(w);

    let v = grade.s2_vignette;
    if (v.x != 0.0) {
        let size = vec2<f32>(grade.out_params.y, grade.out_params.z);
        var q = (px / size) * 2.0 - vec2<f32>(1.0);
        q = q - grade.s2_vignette2.xy;
        let t = (v.w + 1.0) * 0.5;
        let sx = mix1(1.0, grade.s2_vignette2.z, t);
        let dist = length(vec2<f32>(q.x * sx, q.y)) / length(vec2<f32>(sx, 1.0));
        let mask = smooth1(v.y, v.y + 0.01 + v.z, dist);
        lin = lin * max(1.0 + v.x * mask, 0.0);
    }

    if (grade.s2_grain.x > 0.0) {
        var wg = enc0(lin);
        let n = grain_noise(px);
        let lg = dot(clamp01(wg), LUMA2020);
        let weight = 0.25 + 3.0 * lg * (1.0 - lg);
        wg = wg + vec3<f32>(grade.s2_grain.x * 0.12 * n * weight);
        lin = dec0(wg);
    }
    return lin;
}
