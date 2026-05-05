# lite-render-2d — AI assistant reference

This file helps AI coding assistants (Claude, Copilot, etc) give accurate
advice about lite-render-2d without reading the full source.

## Crate structure

- `lite-render-2d-core` — types, traits, no rendering
- `lite-render-2d-glow` — OpenGL ES 3.0 backend (default, lightweight)
- `lite-render-2d-wgpu` — wgpu backend (heavier, broader GPU support)

User imports:

```rust
use lite_render_2d_core::prelude::*;
use lite_render_2d_glow::GlowRenderer;
// or for wgpu:
// use lite_render_2d_wgpu::WgpuRenderer;
```

## Feature flags

Default features: `paths`, `text`

| Feature | Dependency | What it enables |
|---------|------------|-----------------|
| `paths` | `lyon_tessellation` | Bezier path fill/stroke (`draw_path`, `stroke_path`) |
| `text`  | `fontdue` | TTF/OTF font rendering, SDF fonts, rich text |
| `input` | `gilrs` | Gamepad + keyboard input manager |
| `audio` | `rodio` | Sound playback (WAV/OGG/MP3) |
| `svg`   | `usvg` | SVG parsing and rendering |

## Frame loop pattern (ALWAYS follows this structure)

```rust
renderer.begin_frame()?;
renderer.set_camera(&camera);  // optional
// ... all draws here ...
let stats = renderer.end_frame()?;
```

All draw methods MUST be called between `begin_frame` and `end_frame`.
Order of draw calls doesn't matter — the renderer sorts and batches.

## Renderer lifecycle

```rust
// Create (once, in resumed/init):
let mut renderer = GlowRenderer::new(&window)?;
renderer.set_clear_color(Color::rgb(0.15, 0.15, 0.2));

// Resize (on WindowEvent::Resized):
renderer.resize(width, height);
```

## Common task: draw shapes

```rust
// Filled rectangle
renderer.draw_rect(Rect::new(x, y, w, h), DrawParams::fill(Color::RED));

// Stroked rectangle
renderer.draw_rect(Rect::new(x, y, w, h), DrawParams::stroke(Color::WHITE, 2.0));

// Circle
renderer.draw_circle(Vec2::new(x, y), radius, DrawParams::fill(Color::CYAN));

// Line
renderer.draw_line(Vec2::new(x1, y1), Vec2::new(x2, y2), LineParams::new(Color::WHITE, 1.0));

// Rounded rectangle
renderer.draw_rounded_rect(
    RoundedRect::new(Rect::new(x, y, w, h), 10.0),
    DrawParams::fill(Color::GREEN),
);

// Per-corner radii
renderer.draw_rounded_rect(
    RoundedRect::with_radii(Rect::new(x, y, w, h), 5.0, 10.0, 15.0, 20.0),
    DrawParams::fill(Color::BLUE),
);

// Ellipse (radii as Vec2)
renderer.draw_ellipse(Vec2::new(cx, cy), Vec2::new(rx, ry), DrawParams::fill(Color::YELLOW));

// Arc (angles in radians)
renderer.draw_arc(center, radius, start_angle, end_angle, DrawParams::fill(Color::RED));

// Triangle
renderer.draw_triangle(a, b, c, DrawParams::fill(Color::GREEN));

// Convex polygon
renderer.draw_polygon(&[p1, p2, p3, p4], DrawParams::fill(Color::BLUE));

// Complex polygon with holes
renderer.draw_complex_polygon(&outer_pts, &[&hole_pts], DrawParams::fill(Color::RED));
```

## DrawParams and DrawStyle

```rust
// DrawParams wraps DrawStyle + blend + z_index + opacity
DrawParams::fill(color)                           // solid fill
DrawParams::stroke(color, thickness)              // solid stroke
DrawParams::fill(color).with_z(5)                 // z-ordering
DrawParams::fill(color).with_opacity(0.5)         // transparency
DrawParams::fill(color).with_blend(BlendMode::Additive)

// DrawStyle variants (used inside DrawParams.style):
DrawStyle::Fill(Color)
DrawStyle::Stroke(StrokeParams)
DrawStyle::LinearGradient { start: Vec2, end: Vec2, color_start: Color, color_end: Color }
DrawStyle::RadialGradient { center: Vec2, radius: f32, color_inner: Color, color_outer: Color }
DrawStyle::LinearGradientStops { start: Vec2, end: Vec2, stops: Vec<GradientStop> }
DrawStyle::RadialGradientStops { center: Vec2, radius: f32, stops: Vec<GradientStop> }

// Gradient example:
renderer.draw_rect(rect, DrawParams {
    style: DrawStyle::LinearGradient {
        start: Vec2::new(0.0, 0.0),
        end: Vec2::new(200.0, 0.0),
        color_start: Color::RED,
        color_end: Color::BLUE,
    },
    blend: BlendMode::Alpha,
    z_index: 0,
    opacity: 1.0,
});
```

## Lines and paths

```rust
// Line
renderer.draw_line(from, to, LineParams::new(Color::WHITE, 2.0));

// Polyline
renderer.draw_polyline(&points, LineParams::new(Color::GREEN, 1.5));

// Dashed line
renderer.draw_line(from, to, LineParams {
    color: Color::WHITE,
    thickness: 2.0,
    style: StrokeStyle::Dashed { dash_len: 10.0, gap_len: 5.0 },
    cap: LineCap::Round,
    join: LineJoin::Round,
    blend: BlendMode::Alpha,
    z_index: 0,
    opacity: 1.0,
});

// Bezier path (fluent builder)
let path = Path::new()
    .move_to(Vec2::new(0.0, 0.0))
    .cubic_to(
        Vec2::new(50.0, -50.0),   // ctrl1
        Vec2::new(100.0, 50.0),   // ctrl2
        Vec2::new(150.0, 0.0),    // end
    )
    .close();

renderer.draw_path(&path, DrawParams::fill(Color::RED));
renderer.stroke_path(&path, StrokeParams::new(Color::WHITE, 2.0));
```

## Common task: load and draw a sprite

```rust
// From file (reads + decodes internally):
let tex = renderer.load_texture_from_file(
    "assets/player.png".as_ref(),
    TextureParams::default(),
)?;

// From raw RGBA bytes (already decoded):
let tex = renderer.load_texture(&rgba_bytes, TextureParams::default())?;

// Draw with position only:
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(100.0, 200.0)));

// Draw with transform chain:
renderer.draw_sprite(tex, SpriteParams::new(
    Transform2D::new(100.0, 200.0)
        .with_uniform_scale(2.0)
        .with_rotation_deg(45.0),
));

// Draw with tint, flip, opacity:
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y))
    .with_tint(Color::RED)
    .with_flip(true, false)
    .with_opacity(0.5)
    .with_z(10)
);

// Sub-region of texture (sprite sheet frame):
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y))
    .with_src_rect(Rect::new(0.0, 0.0, 32.0, 32.0))
);

// For pixel art, use nearest filtering:
let tex = renderer.load_texture_from_file(
    "pixel.png".as_ref(),
    TextureParams::nearest(),
)?;
```

## SpriteParams fields

```rust
SpriteParams {
    transform: Transform2D,         // position, scale, rotation
    tint: Color,                    // multiplied with texture (WHITE = no tint)
    src_rect: Option<Rect>,         // sub-region in pixels (None = full texture)
    flip_x: bool,
    flip_y: bool,
    blend: BlendMode,               // default: Alpha
    z_index: i32,                   // draw order
    opacity: f32,                   // 0.0-1.0
}
```

## Instanced drawing (many identical sprites)

```rust
let instances: Vec<SpriteInstance> = positions.iter().map(|pos| {
    SpriteInstance::new(Transform2D::new(pos.x, pos.y))
}).collect();
renderer.draw_sprite_instanced(tex, &instances, BlendMode::Alpha, 0);
```

## Common task: camera with mouse interaction

```rust
let mut camera = Camera2D::new(window_w, window_h);
camera.look_at(player_position);
camera = camera.with_zoom(2.0);
renderer.set_camera(&camera);

// Convert mouse click to world position:
let world_pos = camera.screen_to_world(Vec2::new(mouse_x, mouse_y));

// Convert world position to screen:
let screen_pos = camera.world_to_screen(enemy_position);

// Smooth follow:
camera.follow(target, 5.0, dt);  // smoothing factor, delta time

// Screen shake:
camera.shake(10.0, 0.3);  // intensity, duration in seconds
camera.update(dt);         // call every frame to decay shake
```

## Common task: draw text

```rust
// Load font from raw bytes (read the file yourself):
let font_data = std::fs::read("assets/font.ttf")?;
let font = renderer.load_font(&font_data)?;

// Draw text:
renderer.draw_text("Hello", &TextParams {
    font,
    size: 24.0,
    color: Color::WHITE,
    align: TextAlign::Left,
    position: Vec2::new(10.0, 10.0),
    max_width: None,
    line_height: None,
});

// Measure for centering:
let size = renderer.measure_text("Hello", &TextParams {
    font,
    size: 24.0,
    color: Color::WHITE,
    align: TextAlign::Left,
    position: Vec2::ZERO,
    max_width: None,
    line_height: None,
});
let centered_x = (window_w - size.x) / 2.0;

// Word wrap:
renderer.draw_text("Long text here...", &TextParams {
    font,
    size: 16.0,
    color: Color::WHITE,
    align: TextAlign::Left,
    position: Vec2::new(10.0, 10.0),
    max_width: Some(300.0),
    line_height: None,
});

// TextAlign: Left (default), Center, Right
```

## SDF text (feature: "text")

```rust
let sdf_font = renderer.load_sdf_font(&font_data)?;
renderer.draw_sdf_text("Crisp at any size", &TextParams {
    font: sdf_font,
    size: 48.0,
    color: Color::WHITE,
    align: TextAlign::Left,
    position: Vec2::new(10.0, 10.0),
    max_width: None,
    line_height: None,
});
```

## Rich text (feature: "text")

```rust
use lite_render_2d_core::{RichText, RichTextSpan};

renderer.draw_rich_text(&RichText {
    spans: vec![
        RichTextSpan {
            text: "Bold red ".into(),
            font,
            size: 24.0,
            color: Color::RED,
            bold: true,
            italic: false,
        },
        RichTextSpan {
            text: "normal white".into(),
            font,
            size: 24.0,
            color: Color::WHITE,
            bold: false,
            italic: false,
        },
    ],
    align: TextAlign::Left,
    max_width: None,
    line_height: None,
    position: Vec2::new(10.0, 10.0),
});
```

## Common task: render to texture

```rust
let target = renderer.create_render_target(512, 512)?;

// Draw to the offscreen target:
renderer.begin_render_to_texture(target)?;
renderer.draw_rect(Rect::new(0.0, 0.0, 512.0, 512.0), DrawParams::fill(Color::RED));
renderer.end_render_to_texture();

// Use the render target as a sprite texture:
let tex = renderer.render_target_texture(target).unwrap();
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(100.0, 100.0)));

// Read pixels back to CPU:
let pixels = renderer.read_pixels(target)?;  // Vec<u8>, RGBA8

// Cleanup:
renderer.destroy_render_target(target);
```

## Common task: post-processing

```rust
use lite_render_2d_core::post_process::PostEffect;

// Post-effects are applied to render targets:
let scene_target = renderer.create_render_target(width, height)?;
renderer.begin_render_to_texture(scene_target)?;
// ... draw your scene ...
renderer.end_render_to_texture();

// Apply effects:
renderer.apply_post_effect(&PostEffect::Blur(3), scene_target);
renderer.apply_post_effect(&PostEffect::Bloom { threshold: 0.8, intensity: 1.5, radius: 5 }, scene_target);
renderer.apply_post_effect(&PostEffect::Vignette, scene_target);
renderer.apply_post_effect(&PostEffect::Grayscale, scene_target);
renderer.apply_post_effect(&PostEffect::Invert, scene_target);
renderer.apply_post_effect(&PostEffect::Brightness(1.2), scene_target);

// Then draw the result to screen:
let tex = renderer.render_target_texture(scene_target).unwrap();
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(0.0, 0.0)));
```

## Common task: check performance

```rust
let stats = renderer.end_frame()?;
// stats.fps            — frames per second
// stats.frame_time_ms  — time for this frame in ms
// stats.draw_calls     — GPU draw calls (lower = better)
// stats.vertices       — total vertices submitted
// stats.texture_binds  — texture switches (1 = ideal)
// stats.batch_flushes  — batch flush count
// stats.ram_bytes      — process memory usage
```

## Blend modes

```rust
renderer.set_blend_mode(BlendMode::Additive);  // global
// or per-draw:
DrawParams::fill(color).with_blend(BlendMode::Multiply)

// Available: Alpha (default), Additive, Multiply, Screen, PremultipliedAlpha
```

## Transform stack

```rust
renderer.push_transform(Transform2D::new(100.0, 100.0).with_rotation_deg(45.0));
// draws here are transformed relative to the pushed transform
renderer.draw_rect(Rect::new(0.0, 0.0, 50.0, 50.0), DrawParams::fill(Color::RED));
renderer.pop_transform();
renderer.reset_transform();  // clears entire stack
```

## Clip rects (scissoring)

```rust
renderer.push_clip_rect(Rect::new(10.0, 10.0, 200.0, 200.0));
// only pixels inside the clip rect are drawn
renderer.pop_clip_rect();
```

## Stencil masking

```rust
renderer.begin_stencil_write();
// draw mask shapes (writes to stencil buffer, not color)
renderer.draw_circle(center, 100.0, DrawParams::fill(Color::WHITE));
renderer.end_stencil_write();
// subsequent draws are clipped to the stencil mask
renderer.draw_rect(large_rect, DrawParams::fill(Color::RED));
renderer.pop_stencil_mask();
```

## Nine-slice sprites

```rust
use lite_render_2d_core::types::NineSlice;

let nine = NineSlice {
    texture: tex,
    border_left: 10.0,
    border_right: 10.0,
    border_top: 10.0,
    border_bottom: 10.0,
};
renderer.draw_nine_slice(&nine, Rect::new(x, y, w, h), Color::WHITE, 0);
```

## Sprite sheets and animation

```rust
use lite_render_2d_core::{SpriteSheet, SpriteAnimation, PlaybackMode};

let sheet = SpriteSheet::new(32.0, 32.0, 8, 24);  // 32x32 frames, 8 columns, 24 frames
let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Loop);

// Each frame:
anim.update(dt);
let src = anim.current_src_rect();
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y))
    .with_src_rect(src)
);

// PlaybackMode: Loop, Once, PingPong
// anim.is_finished() — true when Once mode completes
// anim.reset() — restart from frame 0
// anim.set_frame(5) — jump to specific frame
```

## Tilemaps

```rust
use lite_render_2d_core::tilemap::{Tilemap, TilesetInfo, AnimatedTile, TilemapProjection};

let tileset = TilesetInfo { tile_width: 16.0, tile_height: 16.0, columns: 16 };
let mut map = Tilemap::new(100, 100, 16.0, tileset_texture, tileset);

// Set tiles (0 = empty):
map.set_tile(5, 3, 1);  // layer 0
map.set_tile_layer(1, 5, 3, 42);  // specific layer

// Add layers:
let fg_layer = map.add_layer();

// Animated tiles:
map.add_animated_tile(7, AnimatedTile {
    frames: vec![7, 8, 9, 10],
    frame_duration: 0.2,
});

// Flip flags (packed into tile id):
map.set_tile(x, y, tile_id | TILE_FLIP_H);  // horizontal flip
map.set_tile(x, y, tile_id | TILE_FLIP_V);  // vertical flip

// Isometric projection:
map.projection = TilemapProjection::Isometric;

// Update + draw (automatic frustum culling):
map.update(dt);
renderer.draw_tilemap(&map, Vec2::new(0.0, 0.0), 0);
```

## Particle system

```rust
use lite_render_2d_core::particle::{ParticleSystem, ParticleConfig};

let mut particles = ParticleSystem::new();
let emitter = particles.add_emitter(ParticleConfig {
    spawn_rate: 50.0,
    lifetime: (0.5, 1.5),
    velocity: (Vec2::new(-30.0, -80.0), Vec2::new(30.0, -20.0)),
    size: (6.0, 1.0),
    color_start: Color::YELLOW,
    color_end: Color::new(1.0, 0.0, 0.0, 0.0),
    gravity: Vec2::new(0.0, 98.0),
    texture: None,  // None = draws circles, Some(tex) = draws sprites
}, Vec2::new(400.0, 300.0));

// Each frame:
particles.update(dt);
particles.draw(&mut renderer);

// Move emitter:
particles.set_emitter_position(emitter, new_pos);
```

## Trail rendering

```rust
use lite_render_2d_core::trail::TrailRenderer;

let mut trail = TrailRenderer::new(100, 4.0, 1.0);  // max_points, width, lifetime
trail.color_start = Color::WHITE;
trail.color_end = Color::TRANSPARENT;

// Each frame:
trail.add_point(object_position);
trail.update(dt);
trail.draw(&mut renderer);
```

## Collision helpers

```rust
use lite_render_2d_core::{circle_contains, circle_intersects_rect, point_in_polygon, line_intersects_line};

rect.contains(point)                               // point in rect
rect.intersects(&other_rect)                        // rect-rect overlap
circle_contains(center, radius, point)              // point in circle
circle_intersects_rect(center, radius, &rect)       // circle-rect overlap
point_in_polygon(point, &polygon_vertices)          // point in arbitrary polygon
line_intersects_line(a1, a2, b1, b2) -> Option<Vec2> // line segment intersection
```

## Bitmap fonts

```rust
use lite_render_2d_core::{BitmapFont, BitmapGlyph};

let bfont = BitmapFont::from_grid(font_texture, 8.0, 16.0, 16, 32, 96);
// cell_w, cell_h, columns, first_char (ASCII 32 = space), char_count

let size = bfont.measure("Hello");
let quads = bfont.layout("Hello", Vec2::new(10.0, 10.0), Color::WHITE);
// Render quads manually using draw_sprite with src_rect
```

## Texture atlas

```rust
use lite_render_2d_core::{TextureAtlas, AtlasRegion};

let mut atlas = TextureAtlas::new(1024, 1024);
let region = atlas.add_image(&rgba_pixels, img_w, img_h).unwrap();

// Upload atlas to GPU:
let (data, w, h) = atlas.texture_data();
let atlas_tex = renderer.load_texture(data, TextureParams::default())?;

// Draw sub-region:
renderer.draw_sprite(atlas_tex, SpriteParams::new(Transform2D::new(x, y))
    .with_src_rect(region.src_rect())
);

// Grow if full:
if let Some(new_regions) = atlas.grow() {
    // Re-upload the atlas and update region references
}
```

## Custom materials (shaders)

```rust
let material = renderer.create_material(fragment_glsl_source)?;
renderer.draw_sprite_with_material(
    tex,
    &material,
    &[("u_time", UniformValue::Float(time)), ("u_color", UniformValue::Vec4(Color::RED))],
    SpriteParams::new(Transform2D::new(x, y)),
);
renderer.destroy_material(material);

// UniformValue: Float(f32), Vec2(Vec2), Vec4(Color), Int(i32)
```

## Input manager (feature: "input")

```rust
use lite_render_2d_core::{InputManager, ActionBinding, ButtonBinding, AxisBinding};
use winit::keyboard::KeyCode;

let mut input = InputManager::new();
input.bind_action("jump", ActionBinding::Button(ButtonBinding::Key(KeyCode::Space)));
input.bind_action("move_x", ActionBinding::Axis(AxisBinding::KeyPair {
    neg: KeyCode::KeyA, pos: KeyCode::KeyD,
}));

// Each frame (pass winit events):
input.update(&window_events);
if input.just_pressed("jump") { /* ... */ }
let move_x = input.axis_value("move_x");  // -1.0, 0.0, or 1.0
```

## Audio manager (feature: "audio")

```rust
use lite_render_2d_core::{AudioManager, SoundHandle};

let mut audio = AudioManager::new();
let sfx = audio.load_sound(&std::fs::read("sound.wav")?);
let music = audio.load_sound(&std::fs::read("music.ogg")?);

audio.play(sfx);                          // fire and forget
let handle = audio.play_looped(music)?;   // loop forever
handle.set_volume(0.5);
handle.pause();
handle.resume();
handle.stop();
```

## SVG rendering (feature: "svg")

```rust
use lite_render_2d_core::{SvgImage, svg::draw_svg};

let svg = SvgImage::from_data(&std::fs::read("icon.svg")?)?;
draw_svg(&mut renderer, &svg, Vec2::new(100.0, 100.0), 1.0);  // position, scale
```

## Coordinate system

- Origin: top-left (0, 0)
- X: increases rightward
- Y: increases downward
- Rotation: radians, clockwise. Use `with_rotation_deg()` for degrees.
- Camera: `set_camera` affects all draws until next `set_camera` or frame end

## Core types quick reference

```rust
Color::new(r, g, b, a)          Color::rgb(r, g, b)         Color::from_hex(0xFF8800)
Color::from_hex_str("#FF8800")  Color::hsl(h, s, l)         Color::hsv(h, s, v)
Color::WHITE BLACK RED GREEN BLUE YELLOW CYAN MAGENTA GRAY TRANSPARENT
color.with_alpha(0.5)           color.lerp(other, t)         color.from_srgb() .to_srgb()

Vec2::new(x, y)                 Vec2::ZERO ONE UP DOWN LEFT RIGHT
v.length()  .length_squared()   v.normalize()  .dot(other)  .cross(other)  .distance_to(other)

Rect::new(x, y, w, h)          Rect::from_center(cx, cy, w, h)
r.width() .height() .center()  r.left() .top() .right() .bottom()
r.contains(point)              r.intersects(&other)

Transform2D::new(x, y)         Transform2D::IDENTITY
t.with_scale(sx, sy)           t.with_uniform_scale(s)
t.with_rotation(radians)       t.with_rotation_deg(degrees)
```

## Performance rules of thumb

- Sprites with same texture = 1 draw call (automatic batching)
- 100k sprites at 115fps, 16MB RAM is the tested baseline
- Check `FrameStats.draw_calls` to verify batching is working
- Use `TextureParams::nearest()` for pixel art (avoids blurring)
- Frustum culling is automatic for tilemaps
- Don't manually sort draws — the renderer does it
- Use `draw_sprite_instanced` for many identical sprites

## Error handling

All fallible ops return `Result<T, RendererError>`.

| Variant | When |
|---------|------|
| `ContextCreation(String)` | GPU context or driver issue |
| `Surface(String)` | Display surface error (resize, minimize) |
| `Shader(String)` | Custom material GLSL/WGSL syntax error |
| `Texture(String)` | Invalid image data or GPU limit exceeded |
| `Font(String)` | Invalid TTF/OTF data |
| `Io { path, source }` | File read failure |
| `Other(String)` | Unimplemented or catch-all |

## Backend switching

```rust
// Default (glow/OpenGL):
use lite_render_2d_glow::GlowRenderer;
let renderer = GlowRenderer::new(&window)?;

// Alternative (wgpu):
use lite_render_2d_wgpu::WgpuRenderer;
let renderer = WgpuRenderer::new(&window)?;
```

Same `Renderer` trait. Same API. Just change the import.

## DO NOT assume

- Don't assume wgpu types exist — this is NOT raw wgpu
- Don't assume ECS — there is none
- Don't assume a game loop — the user provides their own (typically winit `ApplicationHandler`)
- Don't assume winit version — check `Cargo.toml` (uses winit 0.30+ with `ApplicationHandler`)
- Don't import from sub-modules directly — use the prelude
- Don't call draw methods outside `begin_frame`/`end_frame`
- Don't assume `load_font` takes a path — it takes `&[u8]` raw bytes
- Don't assume `draw_text` takes separate font/size/color args — it takes `&TextParams`
- Don't call the backend type `Renderer2D` — it's `GlowRenderer` or `WgpuRenderer`
