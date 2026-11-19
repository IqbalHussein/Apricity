// Bindings and entry points. A full-image quad is drawn once per compare
// side (scissored); the fragment shader runs the whole pipeline per pixel.

struct View {
    // NDC rectangle of the image quad: x0, y0 (top-left), x1, y1.
    quad: vec4<f32>,
    // x, y: source size in pixels; z: sample mode (0 = exact texel load,
    // 1 = bilinear); w: unused.
    image: vec4<f32>,
};

@group(0) @binding(0) var<uniform> grade: Grade;
@group(0) @binding(1) var<uniform> view: View;
@group(0) @binding(2) var src_tex: texture_2d<f32>;
@group(0) @binding(3) var src_samp: sampler;
@group(0) @binding(4) var lut_tex: texture_3d<f32>;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0));
    let c = corners[vi];
    var o: VsOut;
    o.pos = vec4<f32>(mix(view.quad.x, view.quad.z, c.x), mix(view.quad.y, view.quad.w, c.y), 0.0, 1.0);
    o.uv = c;
    return o;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let size = view.image.xy;
    let px = in.uv * size;
    var src: vec3<f32>;
    if (view.image.z == 0.0) {
        let ip = clamp(vec2<i32>(floor(px)), vec2<i32>(0), vec2<i32>(size) - vec2<i32>(1));
        src = textureLoad(src_tex, ip, 0).xyz;
    } else {
        src = textureSampleLevel(src_tex, src_samp, in.uv, 0.0).xyz;
    }
    let lin1 = stage1(src);
    let lin2 = stage2(lin1, px);
    let out = overlay(output_transform(lin2), px);
    return vec4<f32>(out, 1.0);
}
