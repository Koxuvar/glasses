struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0,  1.0),
        vec2(-1.0,  1.0), vec2(1.0, -1.0), vec2( 1.0,  1.0)
    );
    var uvs = array<vec2<f32>, 6>(
        vec2(0.0, 1.0), vec2(1.0, 1.0), vec2(0.0, 0.0),
        vec2(0.0, 0.0), vec2(1.0, 1.0), vec2(1.0, 0.0)
    );
    var out: VertexOutput;
    out.pos = vec4(positions[idx], 0.0, 1.0);
    out.uv = uvs[idx];
    return out;
}

@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
@group(0) @binding(2) var<uniform> use_ycocg: u32;
@group(0) @binding(3) var<uniform> viewport: vec4<f32>; // x, y, w, h in normalized coords

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // letterbox/pillarbox: map uv into the active viewport rect
    let vx = viewport.x;
    let vy = viewport.y;
    let vw = viewport.z;
    let vh = viewport.w;

    // uv is 0..1 over the full window; remap to video rect
    let u = (in.uv.x - vx) / vw;
    let v = (in.uv.y - vy) / vh;

    // outside video rect = black
    if u < 0.0 || u > 1.0 || v < 0.0 || v > 1.0 {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }

    let c = textureSample(t, s, vec2(u, v));

    if use_ycocg == 1u {
        let scale = c.a * (255.0 / 8.0);
        let co_g = (c.r - (0.5 * 256.0 / 255.0)) * scale;
        let co_r = (c.g - (0.5 * 256.0 / 255.0)) * scale;
        let y = c.b;
        let r = y + co_r;
        let g = y - co_g * 0.25 - co_r * 0.5;
        let b = y + co_g;
        return vec4(r, g, b, 1.0);
    }

    return c;
}
