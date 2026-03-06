struct Uniform {
    screen_size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniform;
@group(0) @binding(1)
var image_texture: texture_2d<f32>;
@group(0) @binding(2)
var image_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let x = (in.position.x / u.screen_size.x) * 2.0 - 1.0;
    let y = 1.0 - (in.position.y / u.screen_size.y) * 2.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = in.uv;
    out.opacity = in.opacity;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(image_texture, image_sampler, in.uv);
    return vec4<f32>(color.rgb, color.a * in.opacity);
}
