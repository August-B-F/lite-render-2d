# API quick reference

Every public method and type in lite-render-2d, grouped logically.

---

## Renderer lifecycle

```
GlowRenderer::new(window: &winit::window::Window) -> Result<Self, RendererError>
WgpuRenderer::new(window: &winit::window::Window) -> Result<Self, RendererError>
```
    Create renderer attached to a winit window.

```
renderer.resize(width: u32, height: u32)
```
    Call when window size changes.

```
renderer.set_clear_color(color: Color)
```
    Set the background color used when begin_frame clears.

```
renderer.set_blend_mode(mode: BlendMode)
```
    Set the global blend mode for subsequent draws.

```
renderer.begin_frame() -> Result<(), RendererError>
```
    Start a new frame. Clears the screen.

```
renderer.end_frame() -> Result<FrameStats, RendererError>
```
    Submit all draws to GPU. Returns performance stats.

---

## Camera

```
Camera2D::new(viewport_w: f32, viewport_h: f32) -> Self
```
    Create camera with top-left origin.

```
Camera2D::centered(viewport_w: f32, viewport_h: f32) -> Self
```
    Create camera centered on (0, 0).

```
camera.with_position(pos: Vec2) -> Self
```
    Builder: set position.

```
camera.with_zoom(zoom: f32) -> Self
```
    Builder: set zoom level.

```
camera.look_at(target: Vec2)
```
    Snap camera to target position.

```
camera.follow(target: Vec2, smoothing: f32, dt: f32)
```
    Smoothly lerp toward target.

```
camera.shake(intensity: f32, duration: f32)
```
    Start screen shake that decays over duration.

```
camera.update(dt: f32)
```
    Step shake decay. Call each frame.

```
camera.screen_to_world(screen_pos: Vec2) -> Vec2
```
    Convert screen pixels to world coordinates.

```
camera.world_to_screen(world_pos: Vec2) -> Vec2
```
    Convert world coordinates to screen pixels.

```
camera.projection_matrix() -> [f32; 16]
```
    Get orthographic projection matrix (y-down).

```
renderer.set_camera(camera: &Camera2D)
```
    Set active camera for all subsequent draws.

```
renderer.camera() -> &Camera2D
```
    Get current camera reference.

---

## Shapes

```
renderer.draw_rect(rect: Rect, params: DrawParams)
```
    Draw a filled or stroked rectangle.

```
renderer.draw_rounded_rect(rrect: RoundedRect, params: DrawParams)
```
    Draw a rounded rectangle.

```
renderer.draw_circle(center: Vec2, radius: f32, params: DrawParams)
```
    Draw a filled or stroked circle.

```
renderer.draw_ellipse(center: Vec2, radii: Vec2, params: DrawParams)
```
    Draw an ellipse. Radii as Vec2(rx, ry).

```
renderer.draw_arc(center: Vec2, radius: f32, start_angle: f32, end_angle: f32, params: DrawParams)
```
    Draw an arc. Angles in radians.

```
renderer.draw_triangle(a: Vec2, b: Vec2, c: Vec2, params: DrawParams)
```
    Draw a triangle.

```
renderer.draw_polygon(points: &[Vec2], params: DrawParams)
```
    Draw a convex polygon from a list of points.

```
renderer.draw_complex_polygon(outer: &[Vec2], holes: &[&[Vec2]], params: DrawParams)
```
    Draw a complex polygon (concave, with holes).

---

## Lines & paths

```
renderer.draw_line(from: Vec2, to: Vec2, params: LineParams)
```
    Draw a line between two points.

```
renderer.draw_polyline(points: &[Vec2], params: LineParams)
```
    Draw a connected line strip.

```
renderer.draw_path(path: &Path, params: DrawParams)
```
    Fill or stroke a bezier path.

```
renderer.stroke_path(path: &Path, params: StrokeParams)
```
    Stroke a bezier path with line parameters.

```
LineParams::new(color: Color, thickness: f32) -> Self
```
    Create line params with defaults (Butt cap, Miter join, Solid style).

```
LineParams { thickness, color, cap: LineCap, join: LineJoin, style: StrokeStyle, blend, z_index, opacity }
```
    Full line parameter struct.

```
StrokeParams::new(color: Color, thickness: f32) -> Self
```
    Create stroke params with defaults.

```
StrokeParams { color, thickness, style: StrokeStyle, cap: LineCap, join: LineJoin }
```
    Full stroke parameter struct.

```
Path::new() -> Self
```
    Create empty bezier path.

```
path.move_to(p: Vec2) -> Self
path.line_to(p: Vec2) -> Self
path.quad_to(ctrl: Vec2, to: Vec2) -> Self
path.cubic_to(ctrl1: Vec2, ctrl2: Vec2, to: Vec2) -> Self
path.close() -> Self
```
    Fluent path builder methods.

---

## Sprites

```
renderer.load_texture(data: &[u8], params: TextureParams) -> Result<TextureHandle, RendererError>
```
    Load texture from raw RGBA8 image bytes (already decoded).

```
renderer.load_texture_from_file(path: &Path, params: TextureParams) -> Result<TextureHandle, RendererError>
```
    Load texture from file path. Reads and decodes internally.

```
renderer.unload_texture(handle: TextureHandle)
```
    Free GPU memory for a texture.

```
renderer.texture_size(handle: TextureHandle) -> Option<(u32, u32)>
```
    Get texture dimensions (width, height) in pixels.

```
renderer.draw_sprite(handle: TextureHandle, params: SpriteParams)
```
    Draw a textured sprite with full control.

```
renderer.draw_sprite_instanced(handle: TextureHandle, instances: &[SpriteInstance], blend: BlendMode, z_index: i32)
```
    Draw many copies of a sprite with per-instance transforms.

```
renderer.draw_sprite_with_material(handle: TextureHandle, material: &MaterialHandle, uniforms: &[(&str, UniformValue)], params: SpriteParams)
```
    Draw a sprite using a custom shader material.

```
renderer.draw_nine_slice(nine_slice: &NineSlice, target: Rect, tint: Color, z_index: i32)
```
    Draw a nine-slice sprite scaled to target rect.

```
SpriteParams::new(transform: Transform2D) -> Self
```
    Create with defaults (WHITE tint, no flip, Alpha blend, z=0, opacity=1).

```
sprite_params.with_tint(tint: Color) -> Self
sprite_params.with_src_rect(rect: Rect) -> Self
sprite_params.with_flip(x: bool, y: bool) -> Self
sprite_params.with_z(z: i32) -> Self
sprite_params.with_opacity(opacity: f32) -> Self
sprite_params.with_blend(blend: BlendMode) -> Self
```
    SpriteParams builder methods.

```
SpriteInstance::new(transform: Transform2D) -> Self
```
    Create instance with defaults (WHITE tint, opacity 1, no flip).

```
SpriteInstance { transform, tint, opacity, src_rect: Option<Rect>, flip_x, flip_y }
```
    Full instance struct.

---

## Textures

```
TextureParams::default()
```
    Linear filter, Clamp wrap.

```
TextureParams::nearest()
```
    Nearest filter (pixel art), Clamp wrap.

```
TextureParams { filter: FilterMode, wrap: WrapMode }
```
    Full texture params struct.

```
TextureHandle — opaque u64 handle, Copy + Eq + Hash
RenderTargetHandle — opaque u64 handle, Copy + Eq + Hash
```

---

## Text

```
renderer.load_font(data: &[u8]) -> Result<FontHandle, RendererError>
```
    Load font from raw TTF/OTF bytes.

```
renderer.unload_font(handle: FontHandle)
```
    Unload a font.

```
renderer.draw_text(text: &str, params: &TextParams)
```
    Draw a text string.

```
renderer.measure_text(text: &str, params: &TextParams) -> Vec2
```
    Measure text bounds without drawing.

```
TextParams { font: FontHandle, size: f32, color: Color, align: TextAlign, position: Vec2, max_width: Option<f32>, line_height: Option<f32> }
```
    Full text params struct.

```
TextAlign::Left (default) | Center | Right
```

### SDF text [feature: "text"]

```
renderer.load_sdf_font(data: &[u8]) -> Result<FontHandle, RendererError>
renderer.unload_sdf_font(handle: FontHandle)
renderer.draw_sdf_text(text: &str, params: &TextParams)
renderer.measure_sdf_text(text: &str, params: &TextParams) -> Vec2
```
    SDF fonts stay crisp at any size.

### Rich text [feature: "text"]

```
renderer.draw_rich_text(rich: &RichText)
renderer.measure_rich_text(rich: &RichText) -> Vec2
```

```
RichText { spans: Vec<RichTextSpan>, align: TextAlign, max_width: Option<f32>, line_height: Option<f32>, position: Vec2 }
RichTextSpan { text: String, font: FontHandle, size: f32, color: Color, bold: bool, italic: bool }
```

### Bitmap fonts

```
BitmapFont::from_grid(texture: TextureHandle, cell_w: f32, cell_h: f32, columns: u32, first_char: u32, char_count: u32) -> Self
```
    Create grid-based bitmap font. first_char is ASCII code (32 = space).

```
bfont.set_glyph(ch: char, glyph: BitmapGlyph)
bfont.get_glyph(ch: char) -> Option<&BitmapGlyph>
bfont.measure(text: &str) -> Vec2
bfont.layout(text: &str, pos: Vec2, color: Color) -> Vec<BitmapGlyphQuad>
```

```
BitmapGlyph { src_rect: Rect, offset: Vec2, advance: f32 }
BitmapGlyphQuad { src_rect: Rect, pos: Vec2, size: Vec2 }
```

---

## Transform stack & clipping

```
renderer.push_transform(transform: Transform2D)
```
    Push transform onto stack (multiplies with current).

```
renderer.pop_transform()
```
    Pop top transform.

```
renderer.reset_transform()
```
    Reset stack to identity.

```
renderer.push_clip_rect(rect: Rect)
```
    Push scissor rect.

```
renderer.pop_clip_rect()
```
    Pop scissor rect.

---

## Stencil masking

```
renderer.begin_stencil_write()
```
    Subsequent draws write to stencil buffer only.

```
renderer.end_stencil_write()
```
    End stencil write, begin stencil test.

```
renderer.pop_stencil_mask()
```
    Pop stencil mask, restore previous state.

---

## Render targets

```
renderer.create_render_target(width: u32, height: u32) -> Result<RenderTargetHandle, RendererError>
```
    Create offscreen render target.

```
renderer.destroy_render_target(target: RenderTargetHandle)
```
    Destroy render target.

```
renderer.begin_render_to_texture(target: RenderTargetHandle) -> Result<(), RendererError>
```
    Begin rendering to offscreen target.

```
renderer.end_render_to_texture()
```
    End offscreen rendering, restore default target.

```
renderer.render_target_texture(target: RenderTargetHandle) -> Option<TextureHandle>
```
    Get texture handle for render target (for use with draw_sprite).

```
renderer.read_pixels(target: RenderTargetHandle) -> Result<Vec<u8>, RendererError>
```
    Read RGBA8 pixel data from render target.

---

## Post-processing

```
renderer.apply_post_effect(effect: &PostEffect, source: RenderTargetHandle)
```
    Apply post-processing effect to render target.

```
PostEffect::Grayscale
PostEffect::Invert
PostEffect::Brightness(f32)
PostEffect::Vignette
PostEffect::Blur(u32)                                   // radius in pixels
PostEffect::Bloom { threshold: f32, intensity: f32, radius: u32 }
```

---

## Custom materials

```
renderer.create_material(frag_src: &str) -> Result<MaterialHandle, RendererError>
```
    Compile a custom fragment shader.

```
renderer.destroy_material(material: MaterialHandle)
```
    Destroy compiled material.

```
renderer.draw_sprite_with_material(handle: TextureHandle, material: &MaterialHandle, uniforms: &[(&str, UniformValue)], params: SpriteParams)
```
    Draw sprite with custom shader and uniforms.

```
UniformValue::Float(f32)
UniformValue::Vec2(Vec2)
UniformValue::Vec4(Color)
UniformValue::Int(i32)
```

---

## Tilemaps

```
Tilemap::new(width: u32, height: u32, tile_size: f32, texture: TextureHandle, tileset: TilesetInfo) -> Self
```
    Create tilemap with one empty layer.

```
TilesetInfo { tile_width: f32, tile_height: f32, columns: u32 }
```

```
tilemap.add_layer() -> usize
```
    Add empty layer, returns index.

```
tilemap.layer_count() -> usize
tilemap.set_tile(x: u32, y: u32, tile_id: u16)
tilemap.set_tile_layer(layer: usize, x: u32, y: u32, tile_id: u16)
tilemap.get_tile(x: u32, y: u32) -> u16
tilemap.get_tile_layer(layer: usize, x: u32, y: u32) -> u16
```

```
tilemap.add_animated_tile(tile_id: u16, anim: AnimatedTile)
tilemap.update(dt: f32)
```
    Register animated tile and step animation timer.

```
AnimatedTile { frames: Vec<u16>, frame_duration: f32 }
```

```
tilemap.resolve_tile_id(raw_id: u16) -> u16
Tilemap::tile_flip_h(raw_id: u16) -> bool
Tilemap::tile_flip_v(raw_id: u16) -> bool
tilemap.tile_src_rect(tile_id: u16) -> Rect
tilemap.grid_to_world(col: u32, row: u32, offset: Vec2) -> Vec2
```

```
TilemapProjection::Orthogonal (default) | Isometric
TILE_FLIP_H: u16 = 0x8000
TILE_FLIP_V: u16 = 0x4000
TILE_ID_MASK: u16 = 0x3FFF
```

```
renderer.draw_tilemap(tilemap: &Tilemap, position: Vec2, z_index: i32)
```
    Draw tilemap with automatic frustum culling.

---

## Particles

```
ParticleSystem::new() -> Self
```
    Create empty particle system.

```
particles.add_emitter(config: ParticleConfig, position: Vec2) -> usize
```
    Add emitter, returns index.

```
particles.set_emitter_position(idx: usize, pos: Vec2)
particles.remove_emitter(idx: usize)
particles.particle_count() -> usize
particles.update(dt: f32)
particles.draw(renderer: &mut dyn Renderer)
```

```
ParticleConfig {
    spawn_rate: f32,                    // particles per second
    lifetime: (f32, f32),               // (min, max) seconds
    velocity: (Vec2, Vec2),             // (min, max) velocity
    size: (f32, f32),                   // (start, end) size
    color_start: Color,
    color_end: Color,
    gravity: Vec2,
    texture: Option<TextureHandle>,     // None = circles
}
```
    Default: spawn_rate=10, lifetime=(1,2), gravity=(0, 98).

```
ParticleEmitter { config: ParticleConfig, position: Vec2, active: bool }
ParticleEmitter::new(config: ParticleConfig, position: Vec2) -> Self
```

---

## Trails

```
TrailRenderer::new(max_points: usize, width: f32, lifetime: f32) -> Self
```
    Create trail renderer.

```
trail.add_point(pos: Vec2)
```
    Add point. Skips if too close to last point.

```
trail.update(dt: f32)
```
    Age points and remove expired.

```
trail.point_count() -> usize
trail.draw(renderer: &mut dyn Renderer)
```
    Draw trail as ribbon of quads.

```
TrailRenderer { points, max_points, width, lifetime, color_start: Color, color_end: Color, texture: Option<TextureHandle> }
TrailPoint { position: Vec2, age: f32 }
```

---

## Sprite sheets

```
SpriteSheet::new(frame_width: f32, frame_height: f32, columns: u32, frame_count: u32) -> Self
sheet.frame_rect(frame_index: u32) -> Rect
```
    Get source rect for a frame.

```
SpriteAnimation::new(sheet: SpriteSheet, frame_duration: f32, mode: PlaybackMode) -> Self
anim.update(dt: f32)
anim.current_frame() -> u32
anim.current_src_rect() -> Rect
anim.is_finished() -> bool
anim.reset()
anim.set_frame(frame: u32)
```

```
PlaybackMode::Loop | Once | PingPong
```

---

## Collision helpers

```
rect.contains(point: Vec2) -> bool
```
    Point inside rect.

```
rect.intersects(other: &Rect) -> bool
```
    Rect-rect overlap.

```
circle_contains(center: Vec2, radius: f32, point: Vec2) -> bool
```
    Point inside circle.

```
circle_intersects_rect(center: Vec2, radius: f32, rect: &Rect) -> bool
```
    Circle-rect overlap.

```
point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool
```
    Point inside arbitrary polygon (ray casting).

```
line_intersects_line(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> Option<Vec2>
```
    Line segment intersection point.

---

## Texture atlas

```
TextureAtlas::new(width: u32, height: u32) -> Self
```
    Create CPU-side RGBA atlas.

```
atlas.add_image(pixels: &[u8], img_w: u32, img_h: u32) -> Option<AtlasRegion>
```
    Pack image using shelf algorithm. Returns None if full.

```
atlas.grow() -> Option<Vec<AtlasRegion>>
```
    Double atlas size and repack. Returns new regions. None if at max (4096).

```
atlas.region_count() -> usize
atlas.texture_data() -> (&[u8], u32, u32)
atlas.is_dirty() -> bool
atlas.dirty_region() -> Option<(u32, u32, u32, u32)>
atlas.atlas_sub_data(x: u32, y: u32, w: u32, h: u32) -> Vec<u8>
atlas.clear_dirty()
```

```
AtlasRegion { x: u32, y: u32, width: u32, height: u32 }
region.uv_rect(atlas_w: u32, atlas_h: u32) -> Rect
region.src_rect() -> Rect
```

---

## Input [feature: "input"]

```
InputManager::new() -> Self
input.bind_action(name: &str, binding: ActionBinding)
input.update(events: &[WindowEvent])
input.is_pressed(action: &str) -> bool
input.just_pressed(action: &str) -> bool
input.just_released(action: &str) -> bool
input.axis_value(action: &str) -> f32
input.gamepad_count() -> usize
input.set_deadzone(dz: f32)
```

```
ActionBinding::Button(ButtonBinding) | Axis(AxisBinding)
ButtonBinding::Key(KeyCode) | Gamepad(GamepadButton)
AxisBinding::GamepadAxis(GamepadAxis) | KeyPair { neg: KeyCode, pos: KeyCode }
ActionState { pressed: bool, prev_pressed: bool, value: f32 }
action_state.just_pressed() -> bool
action_state.just_released() -> bool
```

---

## Audio [feature: "audio"]

```
AudioManager::new() -> Self
audio.load_sound(data: &[u8]) -> SoundHandle
audio.unload_sound(handle: SoundHandle)
audio.play(handle: SoundHandle) -> Option<PlaybackHandle>
audio.play_looped(handle: SoundHandle) -> Option<PlaybackHandle>
audio.play_with_volume(handle: SoundHandle, vol: f32) -> Option<PlaybackHandle>
```

```
playback.stop()
playback.pause()
playback.resume()
playback.set_volume(vol: f32)
playback.is_playing() -> bool
```

---

## SVG [feature: "svg"]

```
SvgImage::from_data(data: &[u8]) -> Result<Self, String>
svg.width() -> f32
svg.height() -> f32
svg.to_commands() -> Vec<SvgDrawCommand>
```

```
svg::draw_svg(renderer: &mut dyn Renderer, svg: &SvgImage, position: Vec2, scale: f32)
```
    Convenience: render SVG at position with uniform scale.

```
SvgDrawCommand::FillPath { path, color, opacity }
SvgDrawCommand::StrokePath { path, params: StrokeParams, opacity }
SvgDrawCommand::PushTransform(Transform2D)
SvgDrawCommand::PopTransform
```

---

## Types

### Color

```
Color { r: f32, g: f32, b: f32, a: f32 }
Color::new(r, g, b, a)              Color::rgb(r, g, b)
Color::with_alpha(a) -> Color
Color::from_hex(hex: u32) -> Color          // 0xRRGGBB or 0xRRGGBBAA
Color::from_hex_str(s: &str) -> Option<Color>   // "#FF8800" or "FF8800FF"
Color::hsl(h, s, l) -> Color                // h: 0..360, s/l: 0..1
Color::hsv(h, s, v) -> Color                // h: 0..360, s/v: 0..1
Color::from_srgb(r, g, b, a) -> Color       // sRGB gamma decode
color.to_srgb() -> Color
color.lerp(other, t) -> Color
```

Constants: `WHITE BLACK RED GREEN BLUE YELLOW CYAN MAGENTA GRAY TRANSPARENT`

From conversions: `[f32; 4]`, `[f32; 3]`, `[u8; 4]`, `u32`

### Vec2

```
Vec2 { x: f32, y: f32 }
Vec2::new(x, y)
v.length()  v.length_squared()  v.normalize()
v.dot(other)  v.cross(other)  v.distance_to(other)
```

Constants: `ZERO ONE UP DOWN LEFT RIGHT`

Operators: `+ - * (f32)` and `+= -= *=`, `Neg`

From conversions: `[f32; 2]`, `(f32, f32)`

### Rect

```
Rect { pos: Vec2, size: Vec2 }
Rect::new(x, y, w, h)
Rect::from_center(cx, cy, w, h)
r.width()  r.height()  r.center()
r.left()  r.top()  r.right()  r.bottom()
r.contains(point: Vec2) -> bool
r.intersects(other: &Rect) -> bool
```

### Transform2D

```
Transform2D { pos: Vec2, scale: Vec2, rotation: f32 }
Transform2D::new(x, y)              Transform2D::IDENTITY
t.with_scale(sx, sy) -> Self
t.with_uniform_scale(s) -> Self
t.with_rotation(radians) -> Self
t.with_rotation_deg(degrees) -> Self
```

### RoundedRect

```
RoundedRect { rect: Rect, radius: f32, radius_tl, radius_tr, radius_bl, radius_br: f32 }
RoundedRect::new(rect: Rect, radius: f32) -> Self
RoundedRect::with_radii(rect: Rect, tl: f32, tr: f32, bl: f32, br: f32) -> Self
```

### DrawParams

```
DrawParams { style: DrawStyle, blend: BlendMode, z_index: i32, opacity: f32 }
DrawParams::fill(color) -> Self
DrawParams::stroke(color, thickness) -> Self
dp.with_z(z) -> Self
dp.with_opacity(opacity) -> Self
dp.with_blend(blend) -> Self
```

### DrawStyle

```
DrawStyle::Fill(Color)
DrawStyle::Stroke(StrokeParams)
DrawStyle::LinearGradient { start, end, color_start, color_end }
DrawStyle::RadialGradient { center, radius, color_inner, color_outer }
DrawStyle::LinearGradientStops { start, end, stops: Vec<GradientStop> }
DrawStyle::RadialGradientStops { center, radius, stops: Vec<GradientStop> }
DrawStyle::fill(color) -> Self
DrawStyle::stroke(color, thickness) -> Self
```

### Enums

```
BlendMode::Alpha (default) | Additive | Multiply | Screen | PremultipliedAlpha
FilterMode::Nearest | Linear
WrapMode::Clamp | Repeat
StrokeStyle::Solid (default) | Dashed { dash_len, gap_len } | Dotted { spacing }
LineCap::Butt (default) | Round | Square
LineJoin::Miter (default) | Round | Bevel
```

### Other types

```
GradientStop { offset: f32, color: Color }
NineSlice { texture: TextureHandle, border_left, border_right, border_top, border_bottom: f32 }
MaterialHandle — opaque u64, Copy + Eq + Hash
FrameStats { frame_time_ms: f64, draw_calls: u32, vertices: u32, texture_binds: u32, batch_flushes: u32, ram_bytes: u64, fps: f64 }
FontHandle — opaque u64, Copy + Eq + Hash
SoundHandle — opaque u64, Copy + Eq + Hash
```

### TransformStack (internal helper)

```
TransformStack::new() -> Self
stack.push(transform: Transform2D)
stack.pop()
stack.reset()
stack.is_identity() -> bool
stack.apply(p: Vec2) -> Vec2
```
