struct Uniform {
    screen_size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) local_pos: vec2<f32>,
    @location(3) half_size: vec2<f32>,
    @location(4) corner_radius: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) corner_radius: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let x = (in.position.x / u.screen_size.x) * 2.0 - 1.0;
    let y = 1.0 - (in.position.y / u.screen_size.y) * 2.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.color = in.color;
    out.local_pos = in.local_pos;
    out.half_size = in.half_size;
    out.corner_radius = in.corner_radius;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.corner_radius <= 0.0 {
        return in.color;
    }

    let d = abs(in.local_pos) - in.half_size + vec2<f32>(in.corner_radius);
    let dist = length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - in.corner_radius;
    let alpha = 1.0 - smoothstep(-0.75, 0.75, dist);

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
