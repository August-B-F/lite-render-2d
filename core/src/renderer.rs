use crate::camera::Camera2D;
use crate::post_process::PostEffect;
use crate::text::{FontHandle, TextLayout, TextParams};
use crate::atlas::AtlasRegion;
use crate::texture::{AtlasHandle, RenderTargetHandle, TextureHandle, TextureParams};
use crate::tilemap::Tilemap;
use crate::types::{
    BlendMode, Color, DrawParams, FrameStats, LineParams, MaterialHandle, NineSlice, Path, Rect,
    RoundedRect, SpriteInstance, SpriteParams, StrokeParams, Transform2D, UniformValue, Vec2,
};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RendererError {
    #[error("failed to create rendering context: {0} — hint: check that your GPU drivers are up to date")]
    ContextCreation(String),

    #[error("display surface error: {0} — hint: this may happen after a window resize or minimize")]
    Surface(String),

    #[error("shader compilation failed: {0} — hint: check GLSL/WGSL syntax if using a custom material")]
    Shader(String),

    #[error("texture error: {0} — hint: ensure the image data is a valid PNG or other supported format")]
    Texture(String),

    #[error("font error: {0} — hint: ensure the file is a valid TTF or OTF font")]
    Font(String),

    #[error("I/O error reading '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

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

    /// get the current camera
    fn camera(&self) -> &Camera2D;

    /// set the background clear color
    fn set_clear_color(&mut self, color: Color);

    /// set the global blend mode
    fn set_blend_mode(&mut self, mode: BlendMode);

    /// switch to screen-space camera for HUD drawing.
    /// coordinates map to pixels: (0,0) = top-left, (width, height) = bottom-right.
    /// returns the saved camera to pass to end_screen_space().
    fn begin_screen_space(&mut self) -> Camera2D {
        let saved = *self.camera();
        let vp = self.camera().viewport;
        // camera position is the center of the visible area, so set it to
        // half the viewport so that (0,0) maps to the top-left corner
        let screen_cam = Camera2D::new(vp.x, vp.y)
            .with_position(Vec2::new(vp.x * 0.5, vp.y * 0.5));
        self.set_camera(&screen_cam);
        saved
    }

    /// restore a previously saved camera (from begin_screen_space)
    fn end_screen_space(&mut self, saved: Camera2D) {
        self.set_camera(&saved);
    }

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

    /// draw a complex (concave, with holes) polygon
    fn draw_complex_polygon(&mut self, outer: &[Vec2], holes: &[&[Vec2]], params: DrawParams);

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

    /// load a texture from pre-decoded RGBA8 pixel data
    fn load_texture_raw(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        params: TextureParams,
    ) -> Result<TextureHandle, RendererError>;

    /// load a texture from a file path, reads + decodes internally
    fn load_texture_from_file(
        &mut self,
        path: &std::path::Path,
        params: TextureParams,
    ) -> Result<TextureHandle, RendererError> {
        let data = std::fs::read(path).map_err(|e| RendererError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        self.load_texture(&data, params)
    }

    /// unload a previously loaded texture
    fn unload_texture(&mut self, handle: TextureHandle);

    /// get texture dimensions (width, height) in pixels
    fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)>;

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
    fn measure_text(&mut self, text: &str, params: &TextParams) -> Vec2;

    /// compute per-character layout positions without drawing
    #[cfg(feature = "text")]
    fn layout_text(&mut self, _text: &str, _params: &TextParams) -> TextLayout {
        TextLayout { glyphs: vec![], size: Vec2::ZERO, line_count: 0, line_offsets: vec![] }
    }

    /// compute per-character layout positions for rich text without drawing
    #[cfg(feature = "text")]
    fn layout_rich_text(&mut self, _rich: &crate::rich_text::RichText) -> TextLayout {
        TextLayout { glyphs: vec![], size: Vec2::ZERO, line_count: 0, line_offsets: vec![] }
    }

    /// return the font ascent (distance from baseline to top of tallest glyph) for a given font and size
    #[cfg(feature = "text")]
    fn font_ascent(&self, _font: FontHandle, size: f32) -> f32 {
        size * 0.8
    }

    /// finish the frame and present, returns per-frame perf stats
    fn end_frame(&mut self) -> Result<FrameStats, RendererError>;

    // -- render targets --

    /// create an offscreen render target
    fn create_render_target(&mut self, width: u32, height: u32) -> Result<RenderTargetHandle, RendererError>;

    /// destroy a render target
    fn destroy_render_target(&mut self, target: RenderTargetHandle);

    /// begin rendering to an offscreen target (flushes current batch)
    fn begin_render_to_texture(&mut self, target: RenderTargetHandle) -> Result<(), RendererError>;

    /// end offscreen rendering and restore default target
    fn end_render_to_texture(&mut self);

    /// get the texture handle for a render target (for use with draw_sprite)
    fn render_target_texture(&self, target: RenderTargetHandle) -> Option<TextureHandle>;

    // -- post-processing --

    /// apply a post-processing effect to a render target, drawing result to current target
    fn apply_post_effect(&mut self, effect: &PostEffect, source: RenderTargetHandle);

    // -- custom materials --

    /// compile a custom fragment shader into a material
    fn create_material(&mut self, _frag_src: &str) -> Result<MaterialHandle, RendererError> {
        Err(RendererError::Other("create_material not implemented".into()))
    }

    /// destroy a compiled material
    fn destroy_material(&mut self, _material: MaterialHandle) {}

    /// draw a sprite using a custom material with uniforms
    fn draw_sprite_with_material(
        &mut self,
        _handle: TextureHandle,
        _material: &MaterialHandle,
        _uniforms: &[(&str, UniformValue)],
        _params: SpriteParams,
    ) {}

    // -- pixel readback --

    /// read rgba8 pixel data from a render target
    fn read_pixels(&self, _target: RenderTargetHandle) -> Result<Vec<u8>, RendererError> {
        Err(RendererError::Other("read_pixels not implemented".into()))
    }

    // -- stencil masking --

    /// begin stencil write mode — subsequent draws write to stencil buffer only
    fn begin_stencil_write(&mut self) {}

    /// end stencil write and begin stencil test — subsequent draws are clipped to the mask
    fn end_stencil_write(&mut self) {}

    /// pop stencil mask, restoring previous state
    fn pop_stencil_mask(&mut self) {}

    // -- texture atlas --

    /// create a user-managed texture atlas for packing many small sprites
    fn create_atlas(
        &mut self,
        _width: u32,
        _height: u32,
        _params: TextureParams,
    ) -> Result<AtlasHandle, RendererError> {
        Err(RendererError::Other("create_atlas not implemented".into()))
    }

    /// pack an RGBA8 image into an atlas, returns the region for use as src_rect
    fn atlas_pack(
        &mut self,
        _atlas: AtlasHandle,
        _rgba: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<AtlasRegion, RendererError> {
        Err(RendererError::Other("atlas_pack not implemented".into()))
    }

    /// get the texture handle for an atlas (uploads/re-uploads if dirty)
    fn atlas_texture(
        &mut self,
        _atlas: AtlasHandle,
    ) -> Result<TextureHandle, RendererError> {
        Err(RendererError::Other("atlas_texture not implemented".into()))
    }

    // -- instanced drawing --

    /// draw many copies of a sprite with per-instance transforms
    fn draw_sprite_instanced(
        &mut self,
        handle: TextureHandle,
        instances: &[SpriteInstance],
        blend: BlendMode,
        z_index: i32,
    ) {
        // default fallback: individual draw calls
        for inst in instances {
            self.draw_sprite(handle, SpriteParams {
                transform: inst.transform,
                tint: inst.tint,
                opacity: inst.opacity,
                src_rect: inst.src_rect,
                flip_x: inst.flip_x,
                flip_y: inst.flip_y,
                blend,
                z_index,
                origin: inst.origin,
            });
        }
    }

    // -- nine-slice --

    /// draw a nine-slice sprite scaled to a target rect
    fn draw_nine_slice(
        &mut self,
        nine_slice: &NineSlice,
        target: Rect,
        tint: Color,
        z_index: i32,
    ) {
        let (tw, th) = match self.texture_size(nine_slice.texture) {
            Some(s) => (s.0 as f32, s.1 as f32),
            None => return,
        };

        let bl = nine_slice.border_left;
        let br = nine_slice.border_right;
        let bt = nine_slice.border_top;
        let bb = nine_slice.border_bottom;

        let mid_w = target.size.x - bl - br;
        let mid_h = target.size.y - bt - bb;

        // helper: draw one slice as a sprite
        // src in pixel coords, dst in world coords
        let tex = nine_slice.texture;
        let mut draw_patch = |sx: f32, sy: f32, sw: f32, sh: f32, dx: f32, dy: f32, dw: f32, dh: f32| {
            if dw <= 0.0 || dh <= 0.0 { return; }
            self.draw_sprite(tex, SpriteParams {
                transform: Transform2D {
                    pos: Vec2::new(dx, dy),
                    scale: Vec2::new(dw / sw, dh / sh),
                    rotation: 0.0,
                },
                tint,
                src_rect: Some(Rect { pos: Vec2::new(sx, sy), size: Vec2::new(sw, sh) }),
                flip_x: false,
                flip_y: false,
                blend: BlendMode::Alpha,
                z_index,
                opacity: 1.0,
                origin: Vec2::ZERO,
            });
        };

        let x = target.pos.x;
        let y = target.pos.y;
        let src_mid_w = tw - bl - br;
        let src_mid_h = th - bt - bb;

        // corners
        draw_patch(0.0, 0.0, bl, bt, x, y, bl, bt);
        draw_patch(tw - br, 0.0, br, bt, x + bl + mid_w, y, br, bt);
        draw_patch(0.0, th - bb, bl, bb, x, y + bt + mid_h, bl, bb);
        draw_patch(tw - br, th - bb, br, bb, x + bl + mid_w, y + bt + mid_h, br, bb);

        // edges
        draw_patch(bl, 0.0, src_mid_w, bt, x + bl, y, mid_w, bt);
        draw_patch(bl, th - bb, src_mid_w, bb, x + bl, y + bt + mid_h, mid_w, bb);
        draw_patch(0.0, bt, bl, src_mid_h, x, y + bt, bl, mid_h);
        draw_patch(tw - br, bt, br, src_mid_h, x + bl + mid_w, y + bt, br, mid_h);

        // center
        draw_patch(bl, bt, src_mid_w, src_mid_h, x + bl, y + bt, mid_w, mid_h);
    }

    // -- sdf text (requires "text" feature) --

    /// load an sdf font from raw ttf/otf bytes
    #[cfg(feature = "text")]
    fn load_sdf_font(&mut self, _data: &[u8]) -> Result<FontHandle, RendererError> {
        Err(RendererError::Font("sdf fonts not supported".into()))
    }

    /// unload an sdf font
    #[cfg(feature = "text")]
    fn unload_sdf_font(&mut self, _handle: FontHandle) {}

    /// draw text using sdf rendering (crisp at any scale)
    #[cfg(feature = "text")]
    fn draw_sdf_text(&mut self, _text: &str, _params: &TextParams) {}

    /// measure sdf text bounds without drawing
    #[cfg(feature = "text")]
    fn measure_sdf_text(&mut self, _text: &str, _params: &TextParams) -> Vec2 {
        Vec2::ZERO
    }

    // -- rich text (requires "text" feature) --

    /// draw rich text with mixed fonts, sizes, and colors
    #[cfg(feature = "text")]
    fn draw_rich_text(&mut self, _rich: &crate::rich_text::RichText) {}

    /// measure rich text bounding box
    #[cfg(feature = "text")]
    fn measure_rich_text(&mut self, _rich: &crate::rich_text::RichText) -> Vec2 {
        Vec2::ZERO
    }

    // -- tilemap --

    /// draw a tilemap at a position, culling to camera viewport
    fn draw_tilemap(&mut self, tilemap: &Tilemap, position: Vec2, z_index: i32) {
        let cam = *self.camera();
        let ts = tilemap.tile_size;

        // compute visible tile range from camera
        let cam_left = cam.position.x - cam.viewport.x / (2.0 * cam.zoom);
        let cam_top = cam.position.y - cam.viewport.y / (2.0 * cam.zoom);
        let cam_right = cam.position.x + cam.viewport.x / (2.0 * cam.zoom);
        let cam_bottom = cam.position.y + cam.viewport.y / (2.0 * cam.zoom);

        let col_min = ((cam_left - position.x) / ts).floor().max(0.0) as u32;
        let row_min = ((cam_top - position.y) / ts).floor().max(0.0) as u32;
        let col_max = ((cam_right - position.x) / ts).ceil().min(tilemap.width as f32) as u32;
        let row_max = ((cam_bottom - position.y) / ts).ceil().min(tilemap.height as f32) as u32;

        // draw all layers back-to-front
        for layer in 0..tilemap.layer_count() {
            for row in row_min..row_max {
                for col in col_min..col_max {
                    let raw_id = tilemap.get_tile_layer(layer, col, row);
                    if raw_id == 0 { continue; }

                    let resolved = tilemap.resolve_tile_id(raw_id);
                    let src = tilemap.tile_src_rect(resolved);
                    let world_pos = tilemap.grid_to_world(col, row, position);

                    let flip_x = crate::tilemap::Tilemap::tile_flip_h(raw_id);
                    let flip_y = crate::tilemap::Tilemap::tile_flip_v(raw_id);

                    self.draw_sprite(tilemap.texture, SpriteParams {
                        transform: Transform2D {
                            pos: world_pos,
                            scale: Vec2::new(ts / src.size.x, ts / src.size.y),
                            rotation: 0.0,
                        },
                        tint: Color::WHITE,
                        src_rect: Some(src),
                        flip_x,
                        flip_y,
                        blend: BlendMode::Alpha,
                        z_index: z_index + layer as i32,
                        opacity: 1.0,
                        origin: Vec2::ZERO,
                    });
                }
            }
        }
    }
}
