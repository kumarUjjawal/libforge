struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) v_uv: vec2<f32>,
    @location(1) v_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u_transform: mat4x4<f32>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = u_transform * vec4<f32>(in.pos.x, in.pos.y, 0.0, 1.0);
    out.v_uv = in.uv;
    out.v_color = in.color;
    return out;
}

@fragment
fn fs_color(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.v_color;
}

@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

@fragment
fn fs_texture(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.v_uv);
    return t * in.v_color;
}
