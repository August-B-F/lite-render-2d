# Performance

## FrameStats

Every `end_frame()` returns performance statistics for that frame:

```rust
let stats = renderer.end_frame()?;
println!(
    "fps: {:.0}  draws: {}  verts: {}  tex_binds: {}  ram: {}KB",
    stats.fps,
    stats.draw_calls,
    stats.vertices,
    stats.texture_binds,
    stats.ram_bytes / 1024,
);
```

## What the numbers mean

| Field | Type | What it tells you |
|-------|------|-------------------|
| `fps` | `f64` | Frames per second (smoothed) |
| `frame_time_ms` | `f64` | Time for this frame in milliseconds |
| `draw_calls` | `u32` | GPU draw calls — lower is better |
| `vertices` | `u32` | Total vertices submitted to the GPU |
| `texture_binds` | `u32` | Texture switches per frame — 1 is ideal |
| `batch_flushes` | `u32` | Number of batch flushes (state changes that break batching) |
| `ram_bytes` | `u64` | Total process memory usage |

### Rules of thumb

- **draw_calls**: 1-5 is excellent for most apps. If this is high, you have too many texture/blend mode switches.
- **texture_binds**: should be close to the number of unique textures you use per frame. 1-3 is common.
- **vertices**: informational. More complex shapes = more vertices.
- **batch_flushes**: ideally equals draw_calls. If much higher, something is breaking batches frequently.

## How batching works

The renderer automatically batches draw calls to minimize GPU state changes:

1. **Sprites** sharing the same texture are batched into a single draw call
2. **Different textures** are sorted to minimize texture switches
3. **Shapes** (rects, circles, etc.) are batched separately from sprites
4. **Blend mode changes** cause a batch flush

You don't need to sort your draw calls — the renderer does it. Draw in whatever order makes sense for your code.

### What breaks batching

- Switching textures (each unique texture needs at least one draw call)
- Changing blend mode
- Changing render target
- z_index ordering that interleaves different textures

## How to keep performance high

### 1. Use fewer textures

Pack multiple sprites into a texture atlas:

```rust
use lite_render_2d_core::{TextureAtlas, AtlasRegion};

let mut atlas = TextureAtlas::new(1024, 1024);

// Pack sprites into the atlas
let hero_region = atlas.add_image(&hero_pixels, hero_w, hero_h).unwrap();
let enemy_region = atlas.add_image(&enemy_pixels, enemy_w, enemy_h).unwrap();

// Upload once
let (data, w, h) = atlas.texture_data();
let atlas_tex = renderer.load_texture(data, TextureParams::nearest())?;

// Draw using sub-regions — all sprites share one texture = one draw call
renderer.draw_sprite(atlas_tex, SpriteParams::new(Transform2D::new(x, y))
    .with_src_rect(hero_region.src_rect())
);
renderer.draw_sprite(atlas_tex, SpriteParams::new(Transform2D::new(x2, y2))
    .with_src_rect(enemy_region.src_rect())
);
```

If the atlas fills up, grow it:

```rust
if atlas.add_image(&pixels, w, h).is_none() {
    if let Some(new_regions) = atlas.grow() {
        // Re-upload the grown atlas and update region references
    }
}
```

Maximum atlas size is 4096x4096.

### 2. Use instanced drawing

For many identical sprites (bullets, grass, particles, etc.):

```rust
let instances: Vec<SpriteInstance> = (0..10000).map(|i| {
    SpriteInstance::new(Transform2D::new(
        (i % 100) as f32 * 10.0,
        (i / 100) as f32 * 10.0,
    ))
}).collect();

renderer.draw_sprite_instanced(tex, &instances, BlendMode::Alpha, 0);
```

This can be significantly faster than 10,000 individual `draw_sprite` calls.

### 3. Frustum culling

Tiles outside the camera viewport are **automatically culled** by `draw_tilemap`. You get this for free.

For sprites, the renderer does NOT automatically cull off-screen sprites. If you have thousands of sprites, consider checking visibility yourself:

```rust
let cam = renderer.camera();
let view_rect = Rect::new(
    cam.position.x - cam.viewport.x / (2.0 * cam.zoom),
    cam.position.y - cam.viewport.y / (2.0 * cam.zoom),
    cam.viewport.x / cam.zoom,
    cam.viewport.y / cam.zoom,
);

for sprite in &sprites {
    let sprite_rect = Rect::new(sprite.x, sprite.y, sprite.w, sprite.h);
    if view_rect.intersects(&sprite_rect) {
        renderer.draw_sprite(sprite.tex, sprite.params);
    }
}
```

### 4. Check FrameStats if something feels slow

```rust
let stats = renderer.end_frame()?;
if stats.draw_calls > 20 {
    eprintln!("WARNING: {} draw calls, consider using a texture atlas", stats.draw_calls);
}
```

### 5. Use nearest filtering for pixel art

`TextureParams::nearest()` avoids unnecessary interpolation and is slightly faster than linear filtering for pixel-perfect rendering.

## Benchmark reference

Tested baseline on a mid-range GPU:

| Objects | FPS | RAM | Draw calls |
|---------|-----|------|------------|
| 1,000 sprites | 1000+ | 16MB | 2 |
| 10,000 sprites | 500+ | 16MB | 2 |
| 100,000 sprites | 115 | 16MB | 2 |
| 400,000 sprites | ~22 | 16MB | 2 |

These numbers assume all sprites share one texture (2 draw calls = 1 for clear + 1 for batch). Your mileage will vary with GPU, texture count, and scene complexity.
