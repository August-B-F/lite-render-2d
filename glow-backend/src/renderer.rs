use std::num::NonZeroU32;

use glow::HasContext;
use glutin::surface::GlSurface;

use lite_render_2d_core::{
    BlendMode, Camera2D, Color, DrawParams, FontHandle, LineParams, Path, Rect, Renderer,
    RendererError, RoundedRect, SpriteParams, StrokeParams, TextParams, TextureHandle,
    TextureParams, Transform2D, Vec2,
};

use crate::context::{self, GlContext, Surface};

pub struct GlowRenderer {
    gl: glow::Context,
    surface: Surface,
    gl_ctx: GlContext,
    clear_color: Color,
    w: u32,
    h: u32,
}

impl Renderer for GlowRenderer {
    fn new(window: &winit::window::Window) -> Result<Self, RendererError>
    where
        Self: Sized,
    {
        let (gl, surface, gl_ctx) = context::create_gl_context(window)?;
        let size = window.inner_size();
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
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.w = width;
        self.h = height;
        self.surface.resize(
            &self.gl_ctx,
            NonZeroU32::new(width.max(1)).unwrap(),
            NonZeroU32::new(height.max(1)).unwrap(),
        );
        unsafe {
            self.gl.viewport(0, 0, width as i32, height as i32);
        }
    }

    fn set_camera(&mut self, _camera: &Camera2D) {}

    fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    fn set_blend_mode(&mut self, _mode: BlendMode) {}

    fn begin_frame(&mut self) -> Result<(), RendererError> {
        let c = self.clear_color;
        unsafe {
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

    fn draw_rect(&mut self, _rect: Rect, _params: DrawParams) {}
    fn draw_rounded_rect(&mut self, _rrect: RoundedRect, _params: DrawParams) {}
    fn draw_circle(&mut self, _center: Vec2, _radius: f32, _params: DrawParams) {}
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

    fn draw_line(&mut self, _from: Vec2, _to: Vec2, _params: LineParams) {}
    fn draw_polyline(&mut self, _points: &[Vec2], _params: LineParams) {}
    fn draw_path(&mut self, _path: &Path, _params: DrawParams) {}
    fn stroke_path(&mut self, _path: &Path, _params: StrokeParams) {}

    fn load_texture(
        &mut self,
        _data: &[u8],
        _params: TextureParams,
    ) -> Result<TextureHandle, RendererError> {
        Ok(TextureHandle::new(0))
    }

    fn unload_texture(&mut self, _handle: TextureHandle) {}
    fn draw_sprite(&mut self, _handle: TextureHandle, _params: SpriteParams) {}

    fn load_font(&mut self, _data: &[u8]) -> Result<FontHandle, RendererError> {
        Ok(FontHandle::new(0))
    }

    fn unload_font(&mut self, _handle: FontHandle) {}
    fn draw_text(&mut self, _text: &str, _params: &TextParams) {}

    fn measure_text(&self, _text: &str, _params: &TextParams) -> Vec2 {
        Vec2::ZERO
    }

    fn end_frame(&mut self) -> Result<(), RendererError> {
        self.surface
            .swap_buffers(&self.gl_ctx)
            .expect("swap buffers");
        Ok(())
    }
}
