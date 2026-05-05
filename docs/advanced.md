# Advanced rendering

## Gradients

Gradients are applied through `DrawStyle` variants inside `DrawParams`. Any shape method that takes `DrawParams` supports gradients.

### Linear gradient

```rust
renderer.draw_rect(Rect::new(10.0, 10.0, 300.0, 100.0), DrawParams {
    style: DrawStyle::LinearGradient {
        start: Vec2::new(10.0, 60.0),    // gradient start point (world coords)
        end: Vec2::new(310.0, 60.0),     // gradient end point
        color_start: Color::RED,
        color_end: Color::BLUE,
    },
    blend: BlendMode::Alpha,
    z_index: 0,
    opacity: 1.0,
});
```

### Radial gradient

```rust
renderer.draw_circle(Vec2::new(200.0, 200.0), 100.0, DrawParams {
    style: DrawStyle::RadialGradient {
        center: Vec2::new(200.0, 200.0),
        radius: 100.0,
        color_inner: Color::WHITE,
        color_outer: Color::TRANSPARENT,
    },
    blend: BlendMode::Alpha,
    z_index: 0,
    opacity: 1.0,
});
```

### Multi-stop gradients

For gradients with more than two colors, use the `Stops` variants:

```rust
renderer.draw_rect(rect, DrawParams {
    style: DrawStyle::LinearGradientStops {
        start: Vec2::new(0.0, 0.0),
        end: Vec2::new(400.0, 0.0),
        stops: vec![
            GradientStop { offset: 0.0, color: Color::RED },
            GradientStop { offset: 0.33, color: Color::YELLOW },
            GradientStop { offset: 0.66, color: Color::GREEN },
            GradientStop { offset: 1.0, color: Color::BLUE },
        ],
    },
    blend: BlendMode::Alpha,
    z_index: 0,
    opacity: 1.0,
});
```

`GradientStop.offset` ranges from 0.0 (start) to 1.0 (end). Stops should be in ascending order.

Radial multi-stop works the same way with `DrawStyle::RadialGradientStops`.

## Blend modes

Blend modes control how drawn pixels combine with the existing framebuffer.

### Available modes

| Mode | Effect |
|------|--------|
| `Alpha` | Standard alpha blending (default) |
| `Additive` | Adds colors together — good for glow, fire, light effects |
| `Multiply` | Darkens — good for shadows, overlays |
| `Screen` | Brightens — inverse of Multiply |
| `PremultipliedAlpha` | For pre-multiplied alpha textures |

### Setting blend mode

```rust
// Global — affects all subsequent draws
renderer.set_blend_mode(BlendMode::Additive);

// Per-draw — on shapes
renderer.draw_circle(pos, 50.0, DrawParams::fill(Color::YELLOW).with_blend(BlendMode::Additive));

// Per-draw — on sprites
renderer.draw_sprite(tex, SpriteParams::new(transform).with_blend(BlendMode::Multiply));
```

## Render targets

Render targets let you draw to an offscreen texture instead of the screen. Use them for post-processing, minimaps, reflections, or pre-rendering complex scenes.

### Create a render target

```rust
let target = renderer.create_render_target(512, 512)?;
```

### Draw to the render target

```rust
renderer.begin_render_to_texture(target)?;

// Everything drawn here goes to the offscreen texture
renderer.draw_rect(Rect::new(0.0, 0.0, 512.0, 512.0), DrawParams::fill(Color::RED));
renderer.draw_circle(Vec2::new(256.0, 256.0), 100.0, DrawParams::fill(Color::WHITE));

renderer.end_render_to_texture();
```

### Use render target as a texture

```rust
let tex = renderer.render_target_texture(target).unwrap();
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(100.0, 100.0)));
```

### Read pixels back to CPU

```rust
let pixels: Vec<u8> = renderer.read_pixels(target)?;
// pixels is RGBA8 data, length = width * height * 4
```

### Clean up

```rust
renderer.destroy_render_target(target);
```

## Post-processing

Post-processing effects are applied to render targets. The workflow is:

1. Render your scene to a render target
2. Apply effects to the render target
3. Draw the result to the screen

```rust
use lite_render_2d_core::post_process::PostEffect;

// 1. Render scene to offscreen target
let scene = renderer.create_render_target(width, height)?;
renderer.begin_render_to_texture(scene)?;
// ... draw your scene ...
renderer.end_render_to_texture();

// 2. Apply effects
renderer.apply_post_effect(&PostEffect::Blur(3), scene);
renderer.apply_post_effect(&PostEffect::Vignette, scene);

// 3. Draw result to screen
let tex = renderer.render_target_texture(scene).unwrap();
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::IDENTITY));
```

### Available effects

| Effect | Description |
|--------|-------------|
| `Grayscale` | Convert to grayscale |
| `Invert` | Invert all colors |
| `Brightness(f32)` | Adjust brightness (1.0 = normal, >1.0 = brighter) |
| `Vignette` | Darken edges of the image |
| `Blur(u32)` | Gaussian blur with radius in pixels |
| `Bloom { threshold, intensity, radius }` | Glow effect on bright areas |

### Bloom parameters

```rust
PostEffect::Bloom {
    threshold: 0.8,     // brightness threshold (0.0-1.0) — pixels above this glow
    intensity: 1.5,     // bloom strength
    radius: 5,          // blur radius for the glow
}
```

### Chaining effects

Apply multiple effects in sequence:

```rust
renderer.apply_post_effect(&PostEffect::Bloom { threshold: 0.7, intensity: 2.0, radius: 4 }, scene);
renderer.apply_post_effect(&PostEffect::Vignette, scene);
renderer.apply_post_effect(&PostEffect::Brightness(1.1), scene);
```

Effects are applied in order — each modifies the result of the previous.

## Transform stack

The transform stack lets you create hierarchical transforms. Each pushed transform is multiplied with the current one, affecting all subsequent draws.

```rust
// Draw a group of objects rotated 45 degrees around (200, 200)
renderer.push_transform(Transform2D::new(200.0, 200.0).with_rotation_deg(45.0));

// These draws are now in the transformed coordinate space
renderer.draw_rect(Rect::new(-25.0, -25.0, 50.0, 50.0), DrawParams::fill(Color::RED));
renderer.draw_circle(Vec2::new(0.0, 50.0), 10.0, DrawParams::fill(Color::BLUE));

renderer.pop_transform();

// Back to normal coordinates
renderer.draw_rect(Rect::new(10.0, 10.0, 50.0, 50.0), DrawParams::fill(Color::GREEN));
```

### Nesting transforms

```rust
renderer.push_transform(Transform2D::new(100.0, 100.0));  // translate
    renderer.push_transform(Transform2D::IDENTITY.with_rotation_deg(30.0));  // rotate
        renderer.draw_rect(Rect::new(0.0, 0.0, 50.0, 50.0), DrawParams::fill(Color::RED));
    renderer.pop_transform();
renderer.pop_transform();
```

### Reset

```rust
renderer.reset_transform();  // clears entire stack back to identity
```

## Clip rects (scissoring)

Clip rects restrict drawing to a rectangular region. Pixels outside the rect are discarded.

```rust
renderer.push_clip_rect(Rect::new(50.0, 50.0, 200.0, 200.0));

// Only pixels inside the 200x200 region starting at (50, 50) are drawn
renderer.draw_circle(Vec2::new(150.0, 150.0), 200.0, DrawParams::fill(Color::RED));

renderer.pop_clip_rect();
```

Clip rects can be nested — the effective clip is the intersection of all active clip rects.

## Stencil masking

Stencil masking lets you use arbitrary shapes as masks, not just rectangles.

```rust
// 1. Begin writing to the stencil buffer (draws are invisible)
renderer.begin_stencil_write();

// 2. Draw the mask shape — only the stencil buffer is written
renderer.draw_circle(Vec2::new(200.0, 200.0), 100.0, DrawParams::fill(Color::WHITE));

// 3. End stencil write — subsequent draws are clipped to the mask
renderer.end_stencil_write();

// 4. Draw normally — only pixels inside the circle mask appear
renderer.draw_rect(Rect::new(100.0, 100.0, 200.0, 200.0), DrawParams::fill(Color::RED));
// This rect is clipped to the circle shape

// 5. Pop the mask
renderer.pop_stencil_mask();
```

Use stencil masking for circular viewports, shaped windows, reveal effects, etc.
