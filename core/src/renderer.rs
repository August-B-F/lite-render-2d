use crate::camera::Camera2D;
use crate::text::{FontHandle, TextParams};
use crate::texture::{TextureHandle, TextureParams};
use crate::types::{
    BlendMode, Color, DrawParams, LineParams, Path, Rect, RoundedRect, SpriteParams, StrokeParams,
    Transform2D, Vec2,
};

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("failed to create context: {0}")]
    ContextCreation(String),

    #[error("surface error: {0}")]
    Surface(String),

    #[error("shader compilation failed: {0}")]
    Shader(String),

    #[error("texture error: {0}")]
    Texture(String),

    #[error("font error: {0}")]
    Font(String),

    #[error("{0}")]
    Other(String),
}

pub trait Renderer {
    /// create renderer from a winit window
    fn new(window: &winit::window::Window) -> Result<Self, RendererError>
    where
        Self: Sized;

    /// handle window resize
    fn resize(&mut self, width: u32, height: u32);

    /// set the active camera for projection
    fn set_camera(&mut self, camera: &Camera2D);

    /// set the background clear color
    fn set_clear_color(&mut self, color: Color);

    /// set the global blend mode
    fn set_blend_mode(&mut self, mode: BlendMode);

    /// start a new frame
    fn begin_frame(&mut self) -> Result<(), RendererError>;

    // -- transform stack --

    /// push a transform onto the stack (multiplies with current)
    fn push_transform(&mut self, transform: Transform2D);

    /// pop the top transform off the stack
    fn pop_transform(&mut self);

    /// reset the transform stack to identity
    fn reset_transform(&mut self);

    // -- clipping --

    /// push a scissor rect onto the clip stack
    fn push_clip_rect(&mut self, rect: Rect);

    /// pop the top scissor rect off the clip stack
    fn pop_clip_rect(&mut self);

    // -- shapes --

    /// draw a filled or stroked rectangle
    fn draw_rect(&mut self, rect: Rect, params: DrawParams);

    /// draw a rounded rectangle
    fn draw_rounded_rect(&mut self, rrect: RoundedRect, params: DrawParams);

    /// draw a filled or stroked circle
    fn draw_circle(&mut self, center: Vec2, radius: f32, params: DrawParams);

    /// draw an ellipse
    fn draw_ellipse(&mut self, center: Vec2, radii: Vec2, params: DrawParams);

    /// draw an arc (angles in radians)
    fn draw_arc(
        &mut self,
        center: Vec2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        params: DrawParams,
    );

    /// draw a convex polygon from a list of points
    fn draw_polygon(&mut self, points: &[Vec2], params: DrawParams);

    /// draw a triangle
    fn draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, params: DrawParams);

    // -- lines and paths --

    /// draw a line between two points
    fn draw_line(&mut self, from: Vec2, to: Vec2, params: LineParams);

    /// draw a connected line strip (polyline)
    fn draw_polyline(&mut self, points: &[Vec2], params: LineParams);

    /// draw a bezier path (fill or stroke)
    fn draw_path(&mut self, path: &Path, params: DrawParams);

    /// stroke a bezier path with line params
    fn stroke_path(&mut self, path: &Path, params: StrokeParams);

    // -- textures / sprites --

    /// load a texture from raw image bytes
    fn load_texture(
        &mut self,
        data: &[u8],
        params: TextureParams,
    ) -> Result<TextureHandle, RendererError>;

    /// unload a previously loaded texture
    fn unload_texture(&mut self, handle: TextureHandle);

    /// draw a textured sprite with full control
    fn draw_sprite(&mut self, handle: TextureHandle, params: SpriteParams);

    // -- text --

    /// load a font from raw ttf/otf bytes
    fn load_font(&mut self, data: &[u8]) -> Result<FontHandle, RendererError>;

    /// unload a previously loaded font
    fn unload_font(&mut self, handle: FontHandle);

    /// draw a text string
    fn draw_text(&mut self, text: &str, params: &TextParams);

    /// measure text bounds without drawing
    fn measure_text(&self, text: &str, params: &TextParams) -> Vec2;

    /// finish the frame and present
    fn end_frame(&mut self) -> Result<(), RendererError>;
}
