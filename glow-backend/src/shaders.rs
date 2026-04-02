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
precision highp float;

in vec2 v_local;
flat in vec4 v_color;
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
layout(location = 3) in float a_opacity;

uniform mat4 u_proj;

out vec2 v_uv;
flat out vec4 v_tint;
flat out float v_opacity;

void main() {
    v_uv = a_uv;
    v_tint = a_tint;
    v_opacity = a_opacity;
    gl_Position = u_proj * vec4(a_pos, 0.0, 1.0);
}
"#;

pub const BATCH_SPRITE_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;
flat in vec4 v_tint;
flat in float v_opacity;

uniform sampler2D u_tex;

out vec4 o_color;

void main() {
    vec4 tex = texture(u_tex, v_uv);
    o_color = tex * v_tint * vec4(1.0, 1.0, 1.0, v_opacity);
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
