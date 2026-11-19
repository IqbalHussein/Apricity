// Output transform + preview overlays. Mirrors `cpu::output` / `cpu::overlay`.

const LUMA709: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

fn output_transform(lin: vec3<f32>) -> vec3<f32> {
    let o = row_mul(grade.out_matrix, lin);
    switch grade.out_flags.x {
        case 0u: { return vec3<f32>(srgb_encode1(o.x), srgb_encode1(o.y), srgb_encode1(o.z)); }
        case 1u: { return sign(o) * pow(abs(o), vec3<f32>(1.0 / 2.4)); }
        default: { return o; }
    }
}

fn overlay(out_in: vec3<f32>, px: vec2<f32>) -> vec3<f32> {
    let y = dot(clamp01(out_in), LUMA709);
    if (grade.out_flags.y == 1u) {
        if (y < 0.02) { return vec3<f32>(0.5, 0.0, 0.6); }
        if (y < 0.08) { return vec3<f32>(0.1, 0.25, 0.9); }
        if (y < 0.42) { return vec3<f32>(y); }
        if (y < 0.50) { return vec3<f32>(0.2, 0.75, 0.3); }
        if (y < 0.62) { return vec3<f32>(y); }
        if (y < 0.72) { return vec3<f32>(0.95, 0.55, 0.7); }
        if (y < 0.97) { return vec3<f32>(y); }
        if (y < 0.995) { return vec3<f32>(1.0, 0.9, 0.1); }
        return vec3<f32>(0.9, 0.1, 0.1);
    }
    if (grade.out_flags.y == 2u) {
        let s = (px.x + px.y) / 8.0;
        let stripe = (s - floor(s)) < 0.5;
        if (y > grade.out_params.x && stripe) {
            return vec3<f32>(0.05);
        }
    }
    return out_in;
}
