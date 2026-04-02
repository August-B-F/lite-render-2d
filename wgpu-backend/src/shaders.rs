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

// -- post-processing effect shaders --

pub const EFFECT_GRAYSCALE_SHADER: &str = r#"
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    // fullscreen tri trick: 3 verts cover the screen
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var out: VsOut;
    out.clip_pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.uv);
    let g = dot(tex.rgb, vec3<f32>(0.299, 0.587, 0.114));
    return vec4<f32>(g, g, g, tex.a);
}
"#;

pub const EFFECT_INVERT_SHADER: &str = r#"
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var out: VsOut;
    out.clip_pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.uv);
    return vec4<f32>(1.0 - tex.rgb, tex.a);
}
"#;

pub const EFFECT_BRIGHTNESS_SHADER: &str = r#"
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var<uniform> brightness: f32;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var out: VsOut;
    out.clip_pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.uv);
    return vec4<f32>(tex.rgb * brightness, tex.a);
}
"#;

pub const EFFECT_VIGNETTE_SHADER: &str = r#"
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var out: VsOut;
    out.clip_pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.uv);
    // darken edges based on distnace from center
    let d = in.uv - vec2<f32>(0.5);
    let vign = clamp(1.0 - dot(d, d) * 2.0, 0.0, 1.0);
    return vec4<f32>(tex.rgb * vign, tex.a);
}
"#;

// sdf text shader - same vertex layout as sprite, smoothstep on distance in alpha
pub const SDF_TEXT_SHADER: &str = r#"
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
    let dist = textureSample(t_diffuse, s_diffuse, in.uv).a;
    let edge = 0.5;
    let aa = fwidth(dist) * 0.75;
    let alpha = smoothstep(edge - aa, edge + aa, dist);
    if alpha < 0.001 {
        discard;
    }
    return vec4<f32>(in.tint.rgb, in.tint.a * alpha * in.opacity);
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
