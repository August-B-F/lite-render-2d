use std::collections::HashMap;
use std::num::NonZeroU32;

use glow::HasContext;
use glutin::surface::GlSurface;

use lite_render_2d_core::{
    BlendMode, Camera2D, Color, DrawParams, DrawStyle, FilterMode, FontHandle, LineParams, Path,
    Rect, Renderer, RendererError, RoundedRect, SpriteParams, StrokeParams, TextParams,
    TextureHandle, TextureParams, Transform2D, Vec2, WrapMode,
};

use crate::batch::{Batcher, FlushContext};
use crate::context::{self, GlContext, Surface};
use crate::shaders;

struct TextureInfo {
    gl_tex: glow::Texture,
    width: u32,
    height: u32,
}

pub struct GlowRenderer {
    gl: glow::Context,
    surface: Surface,
    gl_ctx: GlContext,
    clear_color: Color,
    w: u32,
    h: u32,
    proj: [f32; 16],
    // batched shape pipeline
    shape_prog: glow::Program,
    shape_vao: glow::VertexArray,
    shape_loc_proj: Option<glow::UniformLocation>,
    // batched sprite pipeline
    sprite_prog: glow::Program,
    sprite_vao: glow::VertexArray,
    sprite_loc_proj: Option<glow::UniformLocation>,
    sprite_loc_tex: Option<glow::UniformLocation>,
    // shared vbo for both pipelines
    batch_vbo: glow::Buffer,
    // batcher
    batcher: Batcher,
    // texture storage
    textures: HashMap<u64, TextureInfo>,
    next_tex_id: u64,
}

// y-down screen ortho, origin top-left
fn screen_ortho(w: u32, h: u32) -> [f32; 16] {
    let w = w as f32;
    let h = h as f32;
    [
        2.0 / w, 0.0,      0.0,  0.0,
        0.0,    -2.0 / h,  0.0,  0.0,
        0.0,     0.0,      -1.0, 0.0,
       -1.0,     1.0,       0.0, 1.0,
    ]
}

impl GlowRenderer {
    // build the gl texture lookup for flush context
    fn gl_tex_map(&self) -> HashMap<u64, glow::Texture> {
        self.textures.iter().map(|(&id, info)| (id, info.gl_tex)).collect()
    }

    pub fn draw_calls(&self) -> u32 {
        self.batcher.draw_calls()
    }
}

impl Renderer for GlowRenderer {
    fn new(window: &winit::window::Window) -> Result<Self, RendererError>
    where
        Self: Sized,
    {
        let (gl, surface, gl_ctx) = context::create_gl_context(window)?;
        let size = window.inner_size();

        let batch_vbo = unsafe {
            gl.create_buffer().expect("create batch vbo")
        };

        // -- batched shape pipeline --
        let (shape_prog, shape_vao, shape_loc_proj) = unsafe {
            let prog = shaders::compile_program(&gl, shaders::BATCH_SHAPE_VERT, shaders::BATCH_SHAPE_FRAG)?;
            let loc_proj = gl.get_uniform_location(prog, "u_proj");

            let vao = gl.create_vertex_array().expect("create shape vao");
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(batch_vbo));

            // stride = 12 floats = 48 bytes
            let stride = 48;
            // loc 0: a_pos vec2 offset 0
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            // loc 1: a_local vec2 offset 8
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);
            // loc 2: a_color vec4 offset 16
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 16);
            // loc 3: a_mode float offset 32
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 1, glow::FLOAT, false, stride, 32);
            // loc 4: a_stroke_w float offset 36
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(4, 1, glow::FLOAT, false, stride, 36);
            // loc 5: a_size vec2 offset 40
            gl.enable_vertex_attrib_array(5);
            gl.vertex_attrib_pointer_f32(5, 2, glow::FLOAT, false, stride, 40);

            gl.bind_vertex_array(None);

            (prog, vao, loc_proj)
        };

        // -- batched sprite pipeline --
        let (sprite_prog, sprite_vao, sprite_loc_proj, sprite_loc_tex) = unsafe {
            let prog = shaders::compile_program(&gl, shaders::BATCH_SPRITE_VERT, shaders::BATCH_SPRITE_FRAG)?;
            let loc_proj = gl.get_uniform_location(prog, "u_proj");
            let loc_tex = gl.get_uniform_location(prog, "u_tex");

            let vao = gl.create_vertex_array().expect("create sprite vao");
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(batch_vbo));

            // stride = 9 floats = 36 bytes
            let stride = 36;
            // loc 0: a_pos vec2 offset 0
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            // loc 1: a_uv vec2 offset 8
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);
            // loc 2: a_tint vec4 offset 16
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 16);
            // loc 3: a_opacity float offset 32
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 1, glow::FLOAT, false, stride, 32);

            gl.bind_vertex_array(None);

            (prog, vao, loc_proj, loc_tex)
        };

        unsafe {
            gl.viewport(0, 0, size.width as i32, size.height as i32);
        }

        Ok(Self {
            gl,
            surface,
            gl_ctx,
            clear_color: Color::BLACK,
            w: size.width,
            h: size.height,
            proj: screen_ortho(size.width, size.height),
            shape_prog,
            shape_vao,
            shape_loc_proj,
            sprite_prog,
            sprite_vao,
            sprite_loc_proj,
            sprite_loc_tex,
            batch_vbo,
            batcher: Batcher::new(),
            textures: HashMap::new(),
            next_tex_id: 1,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.w = width;
        self.h = height;
        self.proj = screen_ortho(width, height);
        self.surface.resize(
            &self.gl_ctx,
            NonZeroU32::new(width.max(1)).unwrap(),
            NonZeroU32::new(height.max(1)).unwrap(),
        );
        unsafe {
            self.gl.viewport(0, 0, width as i32, height as i32);
        }
    }

    fn set_camera(&mut self, camera: &Camera2D) {
        self.proj = camera.projection_matrix();
    }

    fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    fn set_blend_mode(&mut self, _mode: BlendMode) {}

    fn begin_frame(&mut self) -> Result<(), RendererError> {
        self.batcher.clear();
        let c = self.clear_color;
        unsafe {
            self.gl.enable(glow::BLEND);
            self.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.gl.clear_color(c.r, c.g, c.b, c.a);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        Ok(())
    }

    fn push_transform(&mut self, _transform: Transform2D) {}
    fn pop_transform(&mut self) {}
    fn reset_transform(&mut self) {}

    fn push_clip_rect(&mut self, _rect: Rect) {}
    fn pop_clip_rect(&mut self) {}

    fn draw_rect(&mut self, rect: Rect, params: DrawParams) {
        let (color, mode, stroke_w) = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                (c, 0.0_f32, 0.0_f32)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut c = sp.color;
                c.a *= params.opacity;
                (c, 1.0, sp.thickness)
            }
            _ => return,
        };

        let x = rect.pos.x;
        let y = rect.pos.y;
        let w = rect.size.x;
        let h = rect.size.y;

        // 6 verts × 12 floats = 72
        #[rustfmt::skip]
        let verts: [f32; 72] = [
            // v0: top-left
            x,     y,     0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            // v1: top-right
            x + w, y,     w,   0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            // v2: bottom-right
            x + w, y + h, w,   h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            // v3: top-left
            x,     y,     0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            // v4: bottom-right
            x + w, y + h, w,   h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            // v5: bottom-left
            x,     y + h, 0.0, h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
        ];

        self.batcher.push_shape(&verts);
    }

    fn draw_rounded_rect(&mut self, _rrect: RoundedRect, _params: DrawParams) {}

    fn draw_circle(&mut self, center: Vec2, radius: f32, params: DrawParams) {
        let (color, mode, stroke_w_norm) = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                (c, 2.0_f32, 0.0_f32)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut c = sp.color;
                c.a *= params.opacity;
                // normalzied inner edge for sdf ring
                let norm = 1.0 - sp.thickness / radius;
                (c, 3.0, norm)
            }
            _ => return,
        };

        // pad quad slightly for aa fringe
        let pad = 2.0;
        let ext = radius + pad;
        let cx = center.x;
        let cy = center.y;
        let ln = ext / radius;

        #[rustfmt::skip]
        let verts: [f32; 72] = [
            cx - ext, cy - ext, -ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx + ext, cy - ext,  ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx + ext, cy + ext,  ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx - ext, cy - ext, -ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx + ext, cy + ext,  ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx - ext, cy + ext, -ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
        ];

        self.batcher.push_shape(&verts);
    }

    fn draw_ellipse(&mut self, _center: Vec2, _radii: Vec2, _params: DrawParams) {}
    fn draw_arc(
        &mut self,
        _center: Vec2,
        _radius: f32,
        _start_angle: f32,
        _end_angle: f32,
        _params: DrawParams,
    ) {
    }
    fn draw_polygon(&mut self, _points: &[Vec2], _params: DrawParams) {}
    fn draw_triangle(&mut self, _a: Vec2, _b: Vec2, _c: Vec2, _params: DrawParams) {}

    fn draw_line(&mut self, from: Vec2, to: Vec2, params: LineParams) {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }

        let half = params.thickness * 0.5;
        // perpendiculr direction
        let nx = -dy / len * half;
        let ny = dx / len * half;

        let mut color = params.color;
        color.a *= params.opacity;
        let mode = 4.0_f32;

        #[rustfmt::skip]
        let verts: [f32; 72] = [
            from.x + nx, from.y + ny, 0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            from.x - nx, from.y - ny, 0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            to.x + nx,   to.y + ny,   0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            to.x + nx,   to.y + ny,   0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            from.x - nx, from.y - ny, 0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            to.x - nx,   to.y - ny,   0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
        ];

        self.batcher.push_shape(&verts);
    }

    fn draw_polyline(&mut self, _points: &[Vec2], _params: LineParams) {}
    fn draw_path(&mut self, _path: &Path, _params: DrawParams) {}
    fn stroke_path(&mut self, _path: &Path, _params: StrokeParams) {}

    fn load_texture(
        &mut self,
        data: &[u8],
        params: TextureParams,
    ) -> Result<TextureHandle, RendererError> {
        let img = image::load_from_memory(data)
            .map_err(|e| RendererError::Texture(e.to_string()))?
            .into_rgba8();
        let (w, h) = img.dimensions();
        let pixels = img.into_raw();

        let gl_tex = unsafe {
            let tex = self.gl.create_texture().map_err(|e| RendererError::Texture(e))?;
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                w as i32,
                h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&pixels)),
            );

            let filter = match params.filter {
                FilterMode::Nearest => glow::NEAREST as i32,
                FilterMode::Linear => glow::LINEAR as i32,
            };
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter);

            let wrap = match params.wrap {
                WrapMode::Clamp => glow::CLAMP_TO_EDGE as i32,
                WrapMode::Repeat => glow::REPEAT as i32,
            };
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap);

            self.gl.bind_texture(glow::TEXTURE_2D, None);
            tex
        };

        let id = self.next_tex_id;
        self.next_tex_id += 1;
        self.textures.insert(id, TextureInfo { gl_tex, width: w, height: h });
        Ok(TextureHandle::new(id))
    }

    fn unload_texture(&mut self, handle: TextureHandle) {
        if let Some(info) = self.textures.remove(&handle.id()) {
            unsafe {
                self.gl.delete_texture(info.gl_tex);
            }
        }
    }

    fn draw_sprite(&mut self, handle: TextureHandle, params: SpriteParams) {
        let info = match self.textures.get(&handle.id()) {
            Some(i) => i,
            None => return,
        };

        let tw = info.width as f32;
        let th = info.height as f32;

        // build model matrix: translate × rotate × scale(texture_size * user_scale)
        let t = &params.transform;
        let sx = t.scale.x * tw;
        let sy = t.scale.y * th;
        let cos = t.rotation.cos();
        let sin = t.rotation.sin();

        // transform unit quad corners through model matrix on cpu
        // column-major: T * R * S applied to point (px, py)
        // out_x = cos*sx*px + (-sin*sy)*py + t.pos.x
        // out_y = sin*sx*px + cos*sy*py + t.pos.y
        let transform = |px: f32, py: f32| -> (f32, f32) {
            let x = cos * sx * px + (-sin * sy) * py + t.pos.x;
            let y = sin * sx * px + cos * sy * py + t.pos.y;
            (x, y)
        };

        let (x0, y0) = transform(0.0, 0.0); // top-left
        let (x1, y1) = transform(1.0, 0.0); // top-right
        let (x2, y2) = transform(1.0, 1.0); // bottom-right
        let (x3, y3) = transform(0.0, 1.0); // bottom-left

        // bake uv_rect + flip into uv coords
        let (uv_min_x, uv_min_y, uv_max_x, uv_max_y) = match params.src_rect {
            Some(r) => (
                r.pos.x / tw,
                r.pos.y / th,
                (r.pos.x + r.size.x) / tw,
                (r.pos.y + r.size.y) / th,
            ),
            None => (0.0, 0.0, 1.0, 1.0),
        };

        // remap uv: apply flip then map into uv_rect
        let bake_uv = |mut u: f32, mut v: f32| -> (f32, f32) {
            if params.flip_x { u = 1.0 - u; }
            if params.flip_y { v = 1.0 - v; }
            let u = uv_min_x + u * (uv_max_x - uv_min_x);
            let v = uv_min_y + v * (uv_max_y - uv_min_y);
            (u, v)
        };

        let (u0, v0) = bake_uv(0.0, 0.0);
        let (u1, v1) = bake_uv(1.0, 0.0);
        let (u2, v2) = bake_uv(1.0, 1.0);
        let (u3, v3) = bake_uv(0.0, 1.0);

        let r = params.tint.r;
        let g = params.tint.g;
        let b = params.tint.b;
        let a = params.tint.a;
        let op = params.opacity;

        // 6 verts × 9 floats = 54
        #[rustfmt::skip]
        let verts: [f32; 54] = [
            x0, y0, u0, v0, r, g, b, a, op,
            x1, y1, u1, v1, r, g, b, a, op,
            x2, y2, u2, v2, r, g, b, a, op,
            x0, y0, u0, v0, r, g, b, a, op,
            x2, y2, u2, v2, r, g, b, a, op,
            x3, y3, u3, v3, r, g, b, a, op,
        ];

        self.batcher.push_sprite(handle.id(), &verts);
    }

    fn load_font(&mut self, _data: &[u8]) -> Result<FontHandle, RendererError> {
        Ok(FontHandle::new(0))
    }

    fn unload_font(&mut self, _handle: FontHandle) {}
    fn draw_text(&mut self, _text: &str, _params: &TextParams) {}

    fn measure_text(&self, _text: &str, _params: &TextParams) -> Vec2 {
        Vec2::ZERO
    }

    fn end_frame(&mut self) -> Result<(), RendererError> {
        let tex_map = self.gl_tex_map();
        let ctx = FlushContext {
            gl: &self.gl,
            proj: &self.proj,
            vbo: self.batch_vbo,
            shape_prog: self.shape_prog,
            shape_vao: self.shape_vao,
            shape_loc_proj: &self.shape_loc_proj,
            sprite_prog: self.sprite_prog,
            sprite_vao: self.sprite_vao,
            sprite_loc_proj: &self.sprite_loc_proj,
            sprite_loc_tex: &self.sprite_loc_tex,
            textures: &tex_map,
        };
        self.batcher.flush(&ctx);

        self.surface
            .swap_buffers(&self.gl_ctx)
            .expect("swap buffers");
        Ok(())
    }
}
