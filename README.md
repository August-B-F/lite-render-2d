## lite-render-2d

lightweight 2d rendering in rust. 100k sprites, 1 draw call, 30 MB of RAM.

![showcase](showcase.gif)

## quick start

```rust
use lite_render_2d_core::prelude::*;
use lite_render_2d_glow::GlowRenderer;

let mut ren = GlowRenderer::new(&window)?;
ren.set_clear_color(Color::rgb(0.15, 0.15, 0.2));

ren.begin_frame()?;
ren.draw_rect(Rect::new(50.0, 50.0, 200.0, 100.0), DrawParams::fill(Color::RED));
ren.draw_circle(Vec2::new(400.0, 300.0), 60.0, DrawParams::fill(Color::CYAN));
ren.draw_line(Vec2::new(50.0, 450.0), Vec2::new(550.0, 450.0), LineParams::new(Color::WHITE, 2.0));
let stats = ren.end_frame()?;
```

full runnable version in `examples/hello.rs`.

---

## performance

### sprite stress test (NVIDIA MX450, 800x600, vsync off)

| sprites | fps | frame (ms) | draw calls | RAM (MB) |
|---|---|---|---|---|
| 0 | 1,554 | 0.64 | 0 | 114 |
| 1,000 | 1,490 | 0.67 | 1 | 114 |
| 5,000 | 1,005 | 1.00 | 1 | 115 |
| 10,000 | 716 | 1.40 | 1 | 115 |
| 25,000 | 433 | 2.31 | 1 | 117 |
| 50,000 | 209 | 4.78 | 1 | 119 |
| 100,000 | 108 | 9.27 | 1 | 123 |
| 150,000 | 72 | 13.82 | 1 | 128 |
| 250,000 | 44 | 22.79 | 1 | 137 |
| 500,000 | 22 | 45.66 | 1 | 160 |
| 750,000 | 15 | 67.35 | 1 | 183 |

**60fps ceiling: ~150k sprites** | **30fps ceiling: ~350k sprites** | **always 1 draw call**

<!-- ### sprite stress test (NVIDIA RTX 4060 Ti, 800x600, vsync off)
TODO: paste results from running: cargo run -p lite-render-2d-glow --example stress_test --release
-->

### mixed workloads (NVIDIA MX450)

| benchmark | count | frame (ms) | fps | draw calls |
|---|---|---|---|---|
| 10k sprites | 10,000 | 1.16 | 861 | 1 |
| 100k sprites | 100,000 | 9.29 | 108 | 1 |
| 100k instanced | 100,000 | 7.63 | 131 | 0 |
| 10k filled rects | 10,000 | 1.33 | 754 | 1 |
| 10k filled circles | 10,000 | 2.44 | 409 | 1 |
| 10k lines | 10,000 | 1.08 | 925 | 1 |
| 5k sprites + 5k rects | 10,000 | 1.41 | 708 | 2 |

<!-- ### mixed workloads (NVIDIA RTX 4060 Ti)
TODO: paste results from running: cargo run -p lite-render-2d-glow --example perf_metrics --release
-->

### memory

| scenario | RSS | private heap |
|---|---|---|
| minimal app (hello example) | 4.4 MB | 604 KB |
| renderer idle (0 sprites) | ~30 MB private | ~2.5 MB ours, rest is GL driver |
| 100k sprites | 123 MB | ~10 MB ours |
| 750k sprites | 183 MB | ~70 MB instance buffers |

binary size: 4.7 MB (release, hello example).

run the benchmarks yourself:

```bash
cargo run -p lite-render-2d-glow --example stress_test --release
cargo run -p lite-render-2d-glow --example perf_metrics --release
```

---

## features

- **shapes** — rects, rounded rects, circles, ellipses, arcs, triangles, convex and concave polygons with holes
- **sprites** — texture loading, transform, tint, opacity, flip, instanced batching, nine-slice scaling
- **text** — bitmap fonts, SDF fonts (crisp at any scale), rich text with mixed styles
- **paths** — bezier curves and polylines with dash/dot styles, line caps, join modes
- **camera** — pan, zoom, shake, follow, screen-to-world coordinate conversion
- **render targets** — offscreen rendering, post-processing (blur, bloom, grayscale, vignette, invert)
- **tilemaps** — multi-layer, animated tiles, flip flags, orthogonal + isometric projection, frustum culling
- **particles** — configurable emitters with gravity, lifetime, color fade
- **trails** — ribbon renderer with width/color fade over lifetime
- **gradients** — linear and radial, solid or multi-stop
- **batching** — automatic draw call sorting by texture/z-index, instanced sprite rendering
- **collision** — circle, rect, polygon, line intersection helpers
- **stats** — per-frame draw calls, vertices, texture binds, frame time

two backends behind the same `Renderer` trait:
- **glow** — OpenGL ES 3.0, lightweight, fast compile
- **wgpu** — Vulkan/Metal/DX12/WebGPU, broader platform support

## what it doesn't do

no 3D. no ECS. no game loop. no asset pipeline.

this is a renderer. plug it into whatever architecture you already have.

---

## installation

```toml
# glow backend (default, lightweight)
lite-render-2d-glow = "0.1"

# wgpu backend (heavier, broader platform support)
lite-render-2d-wgpu = "0.1"
```

optional core features:

```toml
[dependencies.lite-render-2d-core]
version = "0.1"
features = ["text", "paths", "input", "audio", "svg"]
```

`text` and `paths` are enabled by default. the rest are opt-in.

---

## examples

```bash
# basic
cargo run -p lite-render-2d-glow --example hello --release
cargo run -p lite-render-2d-glow --example shapes --release
cargo run -p lite-render-2d-glow --example sprites --release

# intermediate
cargo run -p lite-render-2d-glow --example new_features --release
cargo run -p lite-render-2d-glow --example advanced_features --release
cargo run -p lite-render-2d-glow --example interactive --release    # arrow keys to pan, scroll to zoom

# advanced
cargo run -p lite-render-2d-glow --example next_features --release       # nine-slice, particles, tilemaps, text
cargo run -p lite-render-2d-glow --example all_new_features --release    # 15 features in one window
cargo run -p lite-render-2d-glow --example feature_showcase --release    # sprite sheets, animations

# benchmarks
cargo run -p lite-render-2d-glow --example stress_test --release         # find your sprite ceiling
cargo run -p lite-render-2d-glow --example perf_metrics --release        # mixed workload benchmarks
cargo run -p lite-render-2d-glow --example benchmark_simple --release    # raw FPS counter
```

---

## testing

390 tests (308 unit + 82 GPU integration), 0 failures.

```bash
# unit tests (fast, no GPU needed)
cargo test -p lite-render-2d-core --release

# GPU integration tests (opens a window, runs 82 tests, exits)
cargo run -p lite-render-2d-glow --example integration_tests --release
```

see [TESTING.md](TESTING.md) for the full test map.

---

## how it compares

miniquad gives you a thin GL wrapper and you write your own batching. macroquad gives you a full game framework but pulls in more than you need for a 2D renderer. femtovg does vector graphics well but isn't built for sprite-heavy workloads with tilemaps and particles.

lite-render-2d sits in the middle — macroquad's feature set, miniquad's memory footprint, no framework lock-in.

---

## license

MIT
