# Sprites & Textures

## Loading textures

### From a file path

The simplest way. Reads the file and decodes it (PNG, JPG, BMP, etc.) internally:

```rust
let tex = renderer.load_texture_from_file(
    "assets/player.png".as_ref(),
    TextureParams::default(),
)?;
```

### From raw encoded bytes

If you already have the file bytes in memory (e.g., embedded with `include_bytes!`):

```rust
let tex = renderer.load_texture(
    include_bytes!("../assets/player.png"),
    TextureParams::default(),
)?;
```

Note: `load_texture` expects the image data to be in an encoded format (PNG, JPG, etc.) that the backend can decode, OR raw RGBA8 pixel data depending on the backend implementation. For guaranteed compatibility, use `load_texture_from_file`.

### Texture parameters

```rust
TextureParams::default()    // Linear filter, Clamp wrap — smooth, good for most textures
TextureParams::nearest()    // Nearest filter, Clamp wrap — sharp pixels, good for pixel art
```

Full struct:

```rust
TextureParams {
    filter: FilterMode,     // Linear or Nearest
    wrap: WrapMode,         // Clamp or Repeat
}
```

- **Linear** — smooth interpolation when texture is scaled
- **Nearest** — no interpolation, preserves sharp pixel edges (pixel art)
- **Clamp** — edge pixels are extended beyond the texture boundary
- **Repeat** — texture tiles when UVs exceed 0-1 range

## Drawing sprites

```rust
renderer.draw_sprite(handle: TextureHandle, params: SpriteParams)
```

### Basic drawing

```rust
// Draw at position (100, 200), no scale or rotation
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(100.0, 200.0)));
```

### Transform builder

`Transform2D` controls position, scale, and rotation:

```rust
renderer.draw_sprite(tex, SpriteParams::new(
    Transform2D::new(100.0, 200.0)          // position
        .with_scale(2.0, 1.5)               // non-uniform scale
        .with_uniform_scale(2.0)            // or uniform scale
        .with_rotation_deg(45.0)            // rotation in degrees
        .with_rotation(std::f32::consts::PI) // or in radians
));
```

### SpriteParams builder

```rust
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y))
    .with_tint(Color::RED)                  // color multiplied with texture
    .with_src_rect(Rect::new(0.0, 0.0, 32.0, 32.0))  // sub-region of texture
    .with_flip(true, false)                 // horizontal flip, no vertical flip
    .with_opacity(0.7)                      // transparency
    .with_z(10)                             // draw order (higher = on top)
    .with_blend(BlendMode::Additive)        // compositing mode
);
```

### SpriteParams fields

```rust
SpriteParams {
    transform: Transform2D,     // position, scale, rotation
    tint: Color,                // multiplied with texture (WHITE = no tint)
    src_rect: Option<Rect>,     // sub-region in pixels (None = full texture)
    flip_x: bool,               // horizontal mirror
    flip_y: bool,               // vertical mirror
    blend: BlendMode,           // Alpha, Additive, Multiply, Screen, PremultipliedAlpha
    z_index: i32,               // draw order
    opacity: f32,               // 0.0-1.0
}
```

## Opacity and tinting

Tint color is **multiplied** with the texture color per-pixel:

```rust
// No tint (original colors)
.with_tint(Color::WHITE)

// Red tint (keeps red channel, zeroes green/blue)
.with_tint(Color::RED)

// Darken to 50%
.with_tint(Color::rgb(0.5, 0.5, 0.5))

// Transparency via tint alpha
.with_tint(Color::WHITE.with_alpha(0.5))

// Or via opacity (equivalent to tint alpha for sprites)
.with_opacity(0.5)
```

## Flipping

```rust
// Horizontal flip (mirror left-right)
.with_flip(true, false)

// Vertical flip (mirror top-bottom)
.with_flip(false, true)

// Both
.with_flip(true, true)
```

## Nine-slice scaling

Nine-slice divides a texture into 9 regions (4 corners, 4 edges, 1 center). Corners stay fixed, edges stretch along one axis, center stretches both ways. Use this for scalable UI elements like buttons, panels, and windows.

```rust
use lite_render_2d_core::types::NineSlice;

let panel = NineSlice {
    texture: panel_tex,
    border_left: 10.0,      // pixels
    border_right: 10.0,
    border_top: 10.0,
    border_bottom: 10.0,
};

// Draw scaled to any size — corners stay crisp
renderer.draw_nine_slice(&panel, Rect::new(50.0, 50.0, 300.0, 200.0), Color::WHITE, 0);
```

The borders define the inset in pixels from each edge of the source texture.

## Sprite sheets and animation

### SpriteSheet

Describes the grid layout of a sprite sheet texture:

```rust
use lite_render_2d_core::{SpriteSheet, SpriteAnimation, PlaybackMode};

// 32x32 pixel frames, 8 columns, 24 total frames
let sheet = SpriteSheet::new(32.0, 32.0, 8, 24);

// Get source rect for a specific frame:
let src = sheet.frame_rect(5);  // 6th frame (0-indexed)
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y))
    .with_src_rect(src)
);
```

### SpriteAnimation

Automates frame advancement with configurable playback:

```rust
let mut anim = SpriteAnimation::new(
    SpriteSheet::new(32.0, 32.0, 8, 24),
    0.1,                    // 0.1 seconds per frame (10 fps)
    PlaybackMode::Loop,     // Loop, Once, or PingPong
);

// Each frame in your game loop:
anim.update(dt);

// Draw current frame:
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y))
    .with_src_rect(anim.current_src_rect())
);

// Query state:
anim.current_frame()    // current frame index
anim.is_finished()      // true when Once mode reaches the end
anim.reset()            // restart from frame 0
anim.set_frame(5)       // jump to frame 5
```

### Playback modes

- **Loop** — wraps from last frame back to first, forever
- **Once** — plays through once, then stops on the last frame (`is_finished()` returns true)
- **PingPong** — plays forward then backward, repeating

## Instanced drawing

For drawing many copies of the same sprite efficiently (e.g., bullets, particles, grass blades):

```rust
use lite_render_2d_core::types::SpriteInstance;

let instances: Vec<SpriteInstance> = positions.iter().map(|pos| {
    SpriteInstance::new(Transform2D::new(pos.x, pos.y))
}).collect();

renderer.draw_sprite_instanced(tex, &instances, BlendMode::Alpha, 0);
```

Each `SpriteInstance` can have its own transform, tint, opacity, src_rect, and flip:

```rust
SpriteInstance {
    transform: Transform2D,
    tint: Color,
    opacity: f32,
    src_rect: Option<Rect>,
    flip_x: bool,
    flip_y: bool,
}
```

The backend may optimize this into a single draw call with hardware instancing, or fall back to individual draws.

## Texture management

```rust
// Get texture dimensions
let (width, height) = renderer.texture_size(tex).unwrap();

// Free GPU memory when done
renderer.unload_texture(tex);

// Drawing with an unloaded handle does nothing (no crash, no error)
```

Notes:
- `TextureHandle` is `Copy` — cheap to pass around and store.
- Loading the same image twice creates two separate handles (no dedup).
- Textures are not reference-counted — you manage their lifetime.

## Batching

Sprites sharing the same texture are batched into a single draw call automatically. You don't need to sort your draw calls.

How to verify:

```rust
let stats = renderer.end_frame()?;
println!("draw calls: {}", stats.draw_calls);  // 1-5 is excellent
```

To minimize draw calls:
1. Use fewer textures (pack sprites into a texture atlas)
2. Use the same blend mode where possible
3. Let the renderer sort — don't micro-manage draw order

## Custom materials (shaders)

You can draw sprites with custom fragment shaders:

```rust
let material = renderer.create_material(r#"
    #version 300 es
    precision mediump float;
    uniform sampler2D u_texture;
    uniform float u_time;
    in vec2 v_uv;
    out vec4 fragColor;
    void main() {
        vec4 tex = texture(u_texture, v_uv);
        tex.rgb *= 0.5 + 0.5 * sin(u_time);
        fragColor = tex;
    }
"#)?;

renderer.draw_sprite_with_material(
    tex,
    &material,
    &[("u_time", UniformValue::Float(elapsed_time))],
    SpriteParams::new(Transform2D::new(x, y)),
);

// Cleanup when done:
renderer.destroy_material(material);
```

### UniformValue types

```rust
UniformValue::Float(f32)
UniformValue::Vec2(Vec2)
UniformValue::Vec4(Color)    // r, g, b, a as vec4
UniformValue::Int(i32)
```
