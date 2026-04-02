## lite-render-2d

wgpu allocated way too much RAM to render a few rectangles in a notes app. This crate is a lightweight alternative — actual benchmarks coming soon.

## Quick look

```rust
let mut ren = Renderer2D::new(&window)?;
ren.set_clear_color(Color::new(0.392, 0.584, 0.929, 1.0));

ren.begin_frame()?;

ren.draw_rect(
    Rect { pos: Vec2::new(50.0, 50.0), size: Vec2::new(200.0, 120.0) },
    DrawParams::fill(Color::RED),
);
ren.draw_circle(Vec2::new(150.0, 320.0), 60.0, DrawParams::fill(Color::BLUE));
ren.draw_line(
    Vec2::new(50.0, 450.0), Vec2::new(550.0, 450.0),
    LineParams::new(Color::WHITE, 2.0),
);

let stats = ren.end_frame()?;
```

Full runnable version in `examples/shapes.rs`.

## Numbers

<!-- TODO: fill in after benchmarking -->

## What it does

- Shapes: rects, rounded rects, circles, ellipses, arcs, triangles, convex and concave polygons (with holes)
- Sprites with transform, tint, opacity, flip, instanced batching, and nine-slice scaling
- Text rendering — bitmap fonts, SDF fonts (crisp at any scale), rich text with mixed styles
- Bezier paths and polylines with dash/dot styles, line caps, and join modes
- Camera with pan, zoom, shake, follow, and screen-to-world coordinate conversion
- Render targets and post-processing — blur, bloom, grayscale, vignette, brightness, invert
- Tilemaps with multiple layers, animated tiles, and frustum culling
- Particle systems and trail rendering
- Automatic draw call batching, sorted by texture to minimize GPU state changes
- Collision helpers — circle, rect, polygon, line intersection tests
- Per-frame stats: draw calls, vertices, texture binds, RAM usage, FPS
- Two backends (glow and wgpu) behind the same `Renderer` trait

## What it doesn't do

No 3D. No ECS. No game loop. No networking. No asset pipeline.

This is a renderer. Plug it into whatever architecture you already have — games, tools, desktop apps.

## How it compares

miniquad gives you a thin GL wrapper and you write your own batching. macroquad gives you a full game framework but pulls in more than you need for a 2D renderer. femtovg does vector graphics well but isn't built for sprite-heavy workloads with tilemaps and particles. lite-render-2d sits in the middle — macroquad's feature set for 2D rendering, miniquad's memory footprint, no framework lock-in.

## Installation

```bash
cargo add lite-render-2d-glow
```

Or pick your backend with feature flags:

```toml
# glow backend (default, lightweight, OpenGL)
lite-render-2d-glow = "0.1"

# wgpu backend (heavier, broader platform support)
lite-render-2d-wgpu = "0.1"
```

Optional core features:

```toml
[dependencies.lite-render-2d-core]
version = "0.1"
features = ["text", "paths", "input", "audio", "svg"]
```

## Backends

**glow** is the default. It targets OpenGL ES 3.0 with a GL 3.3 core fallback on macOS. Lightweight, fast to compile, minimal dependencies. Good enough for most 2D apps.

**wgpu** is the fallback for when you need Vulkan, Metal, DX12, or WebGPU. Same `Renderer` trait, same API. Heavier on RAM and compile time, but covers platforms where GL context creation is painful.

Both backends implement the same trait. Swap them with a single import change.

## Examples

Run any example with:

```bash
cargo run --example <name>
```

**Basic:**
- `shapes` — rects, circles, and lines with colors and opacity
- `sprites` — texture loading, transforms, flipping, tinting

**Intermediate:**
- `new_features` — polygons, bezier paths, stroke styles, dashed lines
- `advanced_features` — linear and radial gradients, blend modes
- `mixed` — shapes + text + performance stats overlay

**Advanced:**
- `next_features` — rounded rects with per-corner radii, complex polygons with holes, ellipses
- `feature_showcase` — sprite sheets, frame-based animation playback
- `all_new_features` — tilemaps, render targets, post-effects, trails, camera shake
- `test_optimizations` — frustum culling, instanced drawing, atlas regrow, double-buffered VBOs

## License

Free for personal and non-commercial use. Commercial use requires written permission from the author.

This software is provided as-is, with no warranty of any kind. The author assumes no responsibility or liability for any outcomes resulting from its use.
