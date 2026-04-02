use glow::HasContext;
use lite_render_2d_core::RendererError;

// -- batched shape shader: per-vertex color/mode/stroke/size --

pub const BATCH_SHAPE_VERT: &str = r#"#version 300 es
precision highp float;

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_local;
layout(location = 2) in vec4 a_color;
layout(location = 3) in float a_mode;
layout(location = 4) in float a_stroke_w;
layout(location = 5) in vec2 a_size;

uniform mat4 u_proj;

out vec2 v_local;
flat out vec4 v_color;
flat out int v_mode;
flat out float v_stroke_w;
flat out vec2 v_size;

void main() {
    v_local = a_local;
    v_color = a_color;
    v_mode = int(a_mode + 0.5);
    v_stroke_w = a_stroke_w;
    v_size = a_size;
    gl_Position = u_proj * vec4(a_pos, 0.0, 1.0);
}
"#;

pub const BATCH_SHAPE_FRAG: &str = r#"#version 300 es
precision mediump float;

in vec2 v_local;
flat in lowp vec4 v_color;
flat in int v_mode;
flat in float v_stroke_w;
flat in vec2 v_size;

out vec4 o_color;

void main() {
    if (v_mode == 0 || v_mode == 4) {
        // rect fill or line
        o_color = v_color;
    } else if (v_mode == 1) {
        // rect stroke - v_local in [0..size]
        float dx = min(v_local.x, v_size.x - v_local.x);
        float dy = min(v_local.y, v_size.y - v_local.y);
        float d = min(dx, dy);
        if (d > v_stroke_w) discard;
        o_color = v_color;
    } else if (v_mode == 2) {
        // sdf circle fill
        float d = length(v_local);
        float aa = fwidth(d);
        float a = smoothstep(1.0 + aa, 1.0 - aa, d);
        if (a < 0.001) discard;
        o_color = vec4(v_color.rgb, v_color.a * a);
    } else if (v_mode == 3) {
        // sdf circle stroke ring
        float d = length(v_local);
        float aa = fwidth(d);
        float outer = smoothstep(1.0 + aa, 1.0 - aa, d);
        float inner = smoothstep(v_stroke_w - aa, v_stroke_w + aa, d);
        float a = outer * inner;
        if (a < 0.001) discard;
        o_color = vec4(v_color.rgb, v_color.a * a);
    }
}
"#;

// -- batched sprite shader: pre-transformed verts, per-vertex tint --

pub const BATCH_SPRITE_VERT: &str = r#"#version 300 es
precision highp float;

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;
layout(location = 2) in vec4 a_tint;

uniform mat4 u_proj;

out vec2 v_uv;
flat out vec4 v_tint;

void main() {
    v_uv = a_uv;
    v_tint = a_tint;
    gl_Position = u_proj * vec4(a_pos, 0.0, 1.0);
}
"#;

pub const BATCH_SPRITE_FRAG: &str = r#"#version 300 es
precision mediump float;

in mediump vec2 v_uv;
flat in lowp vec4 v_tint;

uniform sampler2D u_tex;

out lowp vec4 o_color;

void main() {
    vec4 tex = texture(u_tex, v_uv);
    o_color = tex * v_tint;
}
"#;

// -- instanced sprite shader: static unit quad + per-instance transform/uv/tint --

pub const INSTANCED_SPRITE_VERT: &str = r#"#version 300 es
precision highp float;

// per-vertex (static quad)
layout(location = 0) in vec2 a_corner;

// per-instance (divisor 1)
layout(location = 1) in vec2 a_pos;
layout(location = 2) in vec2 a_scale;
layout(location = 3) in float a_rot;
layout(location = 4) in vec2 a_uv_min;
layout(location = 5) in vec2 a_uv_max;
layout(location = 6) in vec4 a_tint;

uniform mat4 u_proj;

out vec2 v_uv;
flat out vec4 v_tint;

void main() {
    // trs: rotate then scale then translate
    float c = cos(a_rot);
    float s = sin(a_rot);
    vec2 scaled = a_corner * a_scale;
    vec2 rotated = vec2(c * scaled.x - s * scaled.y, s * scaled.x + c * scaled.y);
    vec2 world = rotated + a_pos;
    v_uv = mix(a_uv_min, a_uv_max, a_corner);
    v_tint = a_tint;
    gl_Position = u_proj * vec4(world, 0.0, 1.0);
}
"#;

// reuses BATCH_SPRITE_FRAG for fragment shader

// -- sdf text shader: same vertex layout as sprites, smoothstep on distance in alpha --

pub const SDF_TEXT_VERT: &str = BATCH_SPRITE_VERT;

pub const SDF_TEXT_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
flat in vec4 v_tint;

uniform sampler2D u_tex;

out vec4 o_color;

void main() {
    float dist = texture(u_tex, v_uv).a;
    float edge = 0.5;
    float aa = fwidth(dist) * 0.75;
    float alpha = smoothstep(edge - aa, edge + aa, dist);
    if (alpha < 0.001) discard;
    o_color = vec4(v_tint.rgb, v_tint.a * alpha);
}
"#;

// -- post-processing effect shaders --

pub const EFFECT_VERT: &str = r#"#version 300 es
precision highp float;

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;

out vec2 v_uv;

void main() {
    v_uv = a_uv;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

pub const EFFECT_GRAYSCALE_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_tex;
out vec4 o_color;

void main() {
    vec4 tex = texture(u_tex, v_uv);
    float g = dot(tex.rgb, vec3(0.299, 0.587, 0.114));
    o_color = vec4(g, g, g, tex.a);
}
"#;

pub const EFFECT_INVERT_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_tex;
out vec4 o_color;

void main() {
    vec4 tex = texture(u_tex, v_uv);
    o_color = vec4(1.0 - tex.rgb, tex.a);
}
"#;

pub const EFFECT_BRIGHTNESS_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_tex;
uniform float u_brightness;
out vec4 o_color;

void main() {
    vec4 tex = texture(u_tex, v_uv);
    o_color = vec4(tex.rgb * u_brightness, tex.a);
}
"#;

pub const EFFECT_VIGNETTE_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_tex;
out vec4 o_color;

void main() {
    vec4 tex = texture(u_tex, v_uv);
    // distnace from center, darken edges
    vec2 d = v_uv - vec2(0.5);
    float vign = 1.0 - dot(d, d) * 2.0;
    vign = clamp(vign, 0.0, 1.0);
    o_color = vec4(tex.rgb * vign, tex.a);
}
"#;

// -- blur shaders for gaussian blur (separable, horizontal + vertical) --

pub const EFFECT_BLUR_H_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_tex;
uniform float u_radius;
out vec4 o_color;

void main() {
    vec2 tex_size = vec2(textureSize(u_tex, 0));
    float px = 1.0 / tex_size.x;
    int r = int(u_radius);
    float total_weight = 0.0;
    vec4 sum = vec4(0.0);
    for (int i = -r; i <= r; i++) {
        float w = exp(-float(i * i) / (2.0 * u_radius * u_radius / 4.0 + 1.0));
        sum += texture(u_tex, v_uv + vec2(float(i) * px, 0.0)) * w;
        total_weight += w;
    }
    o_color = sum / total_weight;
}
"#;

pub const EFFECT_BLUR_V_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_tex;
uniform float u_radius;
out vec4 o_color;

void main() {
    vec2 tex_size = vec2(textureSize(u_tex, 0));
    float px = 1.0 / tex_size.y;
    int r = int(u_radius);
    float total_weight = 0.0;
    vec4 sum = vec4(0.0);
    for (int i = -r; i <= r; i++) {
        float w = exp(-float(i * i) / (2.0 * u_radius * u_radius / 4.0 + 1.0));
        sum += texture(u_tex, v_uv + vec2(0.0, float(i) * px)) * w;
        total_weight += w;
    }
    o_color = sum / total_weight;
}
"#;

pub const EFFECT_BLOOM_THRESHOLD_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_tex;
uniform float u_threshold;
out vec4 o_color;

void main() {
    vec4 tex = texture(u_tex, v_uv);
    float brightness = dot(tex.rgb, vec3(0.299, 0.587, 0.114));
    if (brightness > u_threshold) {
        o_color = tex;
    } else {
        o_color = vec4(0.0);
    }
}
"#;

pub const EFFECT_BLOOM_COMPOSITE_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_tex;
uniform sampler2D u_bloom;
uniform float u_intensity;
out vec4 o_color;

void main() {
    vec4 original = texture(u_tex, v_uv);
    vec4 bloom = texture(u_bloom, v_uv);
    o_color = original + bloom * u_intensity;
}
"#;

// compile vert+frag into a linked program
pub unsafe fn compile_program(
    gl: &glow::Context,
    vert_src: &str,
    frag_src: &str,
) -> Result<glow::Program, RendererError> {
    let vs = gl.create_shader(glow::VERTEX_SHADER).expect("create vert");
    gl.shader_source(vs, vert_src);
    gl.compile_shader(vs);
    if !gl.get_shader_compile_status(vs) {
        let log = gl.get_shader_info_log(vs);
        gl.delete_shader(vs);
        return Err(RendererError::Shader(format!("vert: {log}")));
    }

    let fs = gl.create_shader(glow::FRAGMENT_SHADER).expect("create frag");
    gl.shader_source(fs, frag_src);
    gl.compile_shader(fs);
    if !gl.get_shader_compile_status(fs) {
        let log = gl.get_shader_info_log(fs);
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        return Err(RendererError::Shader(format!("frag: {log}")));
    }

    let prog = gl.create_program().expect("create program");
    gl.attach_shader(prog, vs);
    gl.attach_shader(prog, fs);
    gl.link_program(prog);

    gl.delete_shader(vs);
    gl.delete_shader(fs);

    if !gl.get_program_link_status(prog) {
        let log = gl.get_program_info_log(prog);
        gl.delete_program(prog);
        return Err(RendererError::Shader(format!("link: {log}")));
    }

    Ok(prog)
}
