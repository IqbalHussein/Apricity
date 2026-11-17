// Stage 1 — technical correction. Mirrors `cpu::stage1`.

fn logc3_decode(t: f32) -> f32 {
    let cut = 0.010591;
    let a = 5.555556;
    let b = 0.052272;
    let c = 0.247190;
    let d = 0.385537;
    let e = 5.367655;
    let f = 0.092809;
    if (t > e * cut + f) {
        return (pow10((t - d) / c) - b) / a;
    }
    return (t - f) / e;
}

fn slog3_decode(t: f32) -> f32 {
    let cv = t * 1023.0;
    if (cv >= 171.2103) {
        return pow10((cv - 420.0) / 261.5) * (0.18 + 0.01) - 0.01;
    }
    return (cv - 95.0) * 0.01125 / (171.2103 - 95.0);
}

fn vlog_decode(t: f32) -> f32 {
    if (t < 0.181) {
        return (t - 0.125) / 5.6;
    }
    return pow10((t - 0.598206) / 0.241514) - 0.00873;
}

fn apple_log_decode(p: f32) -> f32 {
    let r0 = -0.05641088;
    let rt = 0.01;
    let c = 47.287112;
    let beta = 0.00964052;
    let gamma = 0.08550479;
    let delta = 0.69336945;
    let pt = c * (rt - r0) * (rt - r0);
    if (p >= pt) {
        return exp2((p - delta) / gamma) - beta;
    } else if (p >= 0.0) {
        return sqrt(p / c) + r0;
    }
    return r0;
}

fn decode_channel(profile: u32, v: f32) -> f32 {
    switch profile {
        case 1u: { return logc3_decode(v); }
        case 2u: { return slog3_decode(v); }
        case 3u: { return vlog_decode(v); }
        case 4u: { return apple_log_decode(v); }
        default: { return srgb_decode1(v); }
    }
}

fn display_render1(x_in: f32) -> f32 {
    let x = max(x_in, 0.0) * 0.72;
    let num = x * (2.51 * x + 0.03);
    let den = x * (2.43 * x + 0.59) + 0.14;
    return clamp(num / den, 0.0, 1.0);
}

fn stage1(src: vec3<f32>) -> vec3<f32> {
    let p = grade.s1_flags.y;
    let native = vec3<f32>(decode_channel(p, src.x), decode_channel(p, src.y), decode_channel(p, src.z));
    var lin = row_mul(grade.s1_matrix, native);
    if (grade.s1_flags.x != 0u) {
        lin = lin * grade.s1_gain.xyz;
        let bp = grade.s1_black.xyz;
        lin = (lin - bp) / (vec3<f32>(1.0) - bp);
    }
    if (grade.s1_flags.z != 0u) {
        lin = vec3<f32>(display_render1(lin.x), display_render1(lin.y), display_render1(lin.z));
    }
    return lin;
}
