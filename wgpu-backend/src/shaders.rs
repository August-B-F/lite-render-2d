pub const SHAPE_SHADER: &str = r#"
struct Uniforms {
    proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) mode: f32,
    @location(4) stroke_w: f32,
    @location(5) size: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) @interpolate(flat) mode: f32,
    @location(3) @interpolate(flat) stroke_w: f32,
    @location(4) @interpolate(flat) size: vec2<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = u.proj * vec4<f32>(in.pos, 0.0, 1.0);
    out.local = in.local;
    out.color = in.color;
    out.mode = in.mode;
    out.stroke_w = in.stroke_w;
    out.size = in.size;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let mode = i32(in.mode + 0.5);

    if mode == 0 || mode == 4 {
        return in.color;
    } else if mode == 1 {
        // rect stroke
        let dx = min(in.local.x, in.size.x - in.local.x);
        let dy = min(in.local.y, in.size.y - in.local.y);
        let d = min(dx, dy);
        if d > in.stroke_w {
            discard;
        }
        return in.color;
    } else if mode == 2 {
        // sdf circle fill
        let d = length(in.local);
        let aa = fwidth(d);
        let a = smoothstep(1.0 + aa, 1.0 - aa, d);
        if a < 0.001 {
            discard;
        }
        return vec4<f32>(in.color.rgb, in.color.a * a);
    } else if mode == 3 {
        // sdf circle stroke ring
        let d = length(in.local);
        let aa = fwidth(d);
        let outer_edge = smoothstep(1.0 + aa, 1.0 - aa, d);
        let inner_edge = smoothstep(in.stroke_w - aa, in.stroke_w + aa, d);
        let a = outer_edge * inner_edge;
        if a < 0.001 {
            discard;
        }
        return vec4<f32>(in.color.rgb, in.color.a * a);
    }

    return in.color;
}
"#;

pub const SPRITE_SHADER: &str = r#"
struct Uniforms {
    proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var t_diffuse: texture_2d<f32>;
@group(0) @binding(2) var s_diffuse: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) opacity: f32,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) tint: vec4<f32>,
    @location(2) @interpolate(flat) opacity: f32,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = u.proj * vec4<f32>(in.pos, 0.0, 1.0);
    out.uv = in.uv;
    out.tint = in.tint;
    out.opacity = in.opacity;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.uv);
    return tex * in.tint * vec4<f32>(1.0, 1.0, 1.0, in.opacity);
}
"#;
