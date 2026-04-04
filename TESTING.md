# lite-render-2d testing

390 tests total (308 unit + 82 GPU integration), 0 failures.

---

## unit tests (no gpu needed)

pure logic tests for types, camera, collision, tessellation, atlas packing, fonts, and more.

```bash
cargo test -p lite-render-2d-core --release
```

run just one module:

```bash
cargo test -p lite-render-2d-core --release tests::types_test
cargo test -p lite-render-2d-core --release tests::camera_test
cargo test -p lite-render-2d-core --release tests::atlas_test
```

run a single test:

```bash
cargo test -p lite-render-2d-core --release test_color_from_hex_str_invalid
```

what's tested:
- **types_test** (58 tests) — Color constants, rgb, hex parsing, lerp, srgb, hsl, From impls. Vec2 math ops, length, normalize, dot, cross, distance. Rect constructors and accessors. Transform2D builders and chaining.
- **camera_test** (20 tests) — ortho projection, screen_to_world/world_to_screen roundtrips, zoom halves visible area, pan offsets, follow lerp, shake decay, zero viewport, extreme zoom.
- **collision_test** (24 tests) — rect contains/intersects, circle_contains, circle_intersects_rect, point_in_polygon (convex, concave, degenerate), line_intersects_line (crossing, parallel, collinear, endpoint).
- **transform_stack_test** (11 tests) — push/pop/reset, translation, scale, rotation, nested transforms, identity passthrough.
- **sprite_sheet_test** (16 tests) — frame_rect indexing and clamping, Loop/Once/PingPong playback modes, reset, set_frame, zero/single frame edge cases.
- **dash_test** (12 tests) — solid passthrough, dashed/dotted splitting, diagonal lines, multi-segment polylines, bezier path flattening.
- **tessellation_test** (37 tests) — linear/radial gradient color, gradient stops, triangle/polygon/ellipse/arc/rounded rect tessellation, polyline, apply_transform, apply_gradient.
- **tilemap_test** (21 tests) — set/get tile, layers, tile_src_rect, flip flags, grid_to_world (ortho + isometric), animated tile resolution.
- **trail_test** (9 tests) — add_point with distance skip, eviction, aging, expiry.
- **particle_test** (11 tests) — emitter add/remove/position, spawn rate, particle lifetime, zero rate.
- **bitmap_font_test** (12 tests) — from_grid, glyph lookup, measure, layout, newlines, custom glyphs.
- **atlas_test** (21 tests) — add_image, pixel readback, dirty tracking, shelf packing, grow/repack, UV normalization.
- **path_tessellation_test** (10 tests) — path fill/stroke, complex polygons with holes, dashed stroke. *(feature = "paths")*
- **font_atlas_test** (14 tests) — FontSystem load/unload, layout, measure, dirty tracking, glyph advance. *(feature = "text")*
- **sdf_font_test** (10 tests) — SdfFontSystem load/unload, layout, measure, atlas dirty tracking. *(feature = "text")*
- **texture_test** (6 tests) — TextureHandle/RenderTargetHandle/FontHandle constructors, TextureParams defaults.
- **inline tests** (16 tests) — sdf_font compute_sdf (4), rich_text layout (5), input deadzone (4), audio ID generation (3).

---

## integration tests (needs gpu)

opens a window, runs 82 tests against the real glow renderer, prints pass/fail, exits.

```bash
cargo run -p lite-render-2d-glow --example integration_tests --release
```

what's tested:
- **lifecycle** (7) — empty frames, 100 frames, set_clear_color, resize 1x1/4096/100x, framestats
- **shapes** (27) — rect fill/stroke/alpha/zero/negative/10k, rounded rect basic/zero/huge/per-corner, circle fill/stroke/zero/negative/10k, ellipse, arc normal/zero/full/negative, triangle normal/degenerate/zero, polygon octagon/0/1/2 verts
- **lines** (6) — basic, same point, zero thickness, polyline 0/1/1000 points
- **sprites** (11) — load 1x1 png, load empty, draw valid/transform/tint/flip, unload then draw, invalid handle, load 100 + unload all, same data different handles
- **blend** (1) — all 5 blend modes in one frame
- **transform** (3) — push/pop, nested, reset
- **camera** (2) — set/get, draw with offset camera
- **clip** (3) — push/pop, nested, draw outside clip
- **text** (5) — load/unload font, load invalid, draw text, measure text, unload then draw
- **paths** (5) — path fill, stroke, empty, complex polygon, complex polygon with hole
- **render targets** (5) — create/destroy, render to texture, get texture handle, draw RT as sprite, destroy invalid
- **post-processing** (2) — grayscale, invert
- **pixels** (1) — read_pixels from render target
- **nine-slice** (1) — draw nine-slice sprite
- **tilemap** (1) — draw tilemap with tiles
- **stencil** (1) — begin/end stencil write + pop mask
- **instanced** (1) — draw_sprite_instanced with 10 instances

---

## benchmarks

```bash
# find your sprite ceiling (escalates until fps < 15, reports 60fps and 30fps thresholds)
cargo run -p lite-render-2d-glow --example stress_test --release

# mixed workload benchmarks (sprites, rects, circles, lines, instanced)
cargo run -p lite-render-2d-glow --example perf_metrics --release

# raw sprite FPS counter (10k sprites, prints every 60 frames)
cargo run -p lite-render-2d-glow --example benchmark_simple --release

# renderer optimization verification (frustum culling, instanced draw, atlas regrow)
cargo run -p lite-render-2d-glow --example test_optimizations --release
```

---

## visual examples

demos you run and look at.

```bash
cargo run -p lite-render-2d-glow --example hello --release              # red rect + blue circle
cargo run -p lite-render-2d-glow --example shapes --release             # rectangles, lines, circles
cargo run -p lite-render-2d-glow --example sprites --release            # texture loading, transforms
cargo run -p lite-render-2d-glow --example mixed --release              # camera pan/zoom + mouse
cargo run -p lite-render-2d-glow --example interactive --release        # camera controls (arrows + scroll)
cargo run -p lite-render-2d-glow --example new_features --release       # rounded rects, paths, strokes
cargo run -p lite-render-2d-glow --example advanced_features --release  # blend modes, gradients
cargo run -p lite-render-2d-glow --example feature_showcase --release   # sprite sheets, animations
cargo run -p lite-render-2d-glow --example next_features --release      # nine-slice, particles, tilemaps, text
cargo run -p lite-render-2d-glow --example all_new_features --release   # 15 features in one window
```

---

## gif capture

regenerate the showcase GIF (requires Python 3 + Pillow):

```bash
cargo run -p lite-render-2d-glow --example gif_capture --release
# outputs showcase.gif (640x400, 120 frames)
```

---

## file map

```
core/src/tests/
    mod.rs                     — test module root
    types_test.rs              — Color, Vec2, Rect, Transform2D        (58 tests)
    camera_test.rs             — Camera2D projection, coordinate conv   (20 tests)
    collision_test.rs          — rect/circle/polygon/line collision     (24 tests)
    transform_stack_test.rs    — push/pop/apply/reset/identity          (11 tests)
    sprite_sheet_test.rs       — SpriteSheet + SpriteAnimation          (16 tests)
    dash_test.rs               — dash/dot polyline splitting            (12 tests)
    tessellation_test.rs       — gradients, shapes, apply_transform     (37 tests)
    tilemap_test.rs            — tiles, layers, flip, projection        (21 tests)
    trail_test.rs              — trail point management                 (9 tests)
    particle_test.rs           — particle system lifecycle              (11 tests)
    bitmap_font_test.rs        — bitmap font layout/measure             (12 tests)
    atlas_test.rs              — texture atlas packing/grow             (21 tests)
    path_tessellation_test.rs  — path fill/stroke, complex polygons     (10 tests)
    font_atlas_test.rs         — FontSystem load/layout/measure         (14 tests)
    sdf_font_test.rs           — SdfFontSystem load/layout/measure      (10 tests)
    texture_test.rs            — handle constructors, TextureParams     (6 tests)

examples/
    integration_tests.rs       — automated glow renderer tests          (82 tests)
    perf_metrics.rs            — mixed workload benchmarks
    stress_test.rs             — sprite ceiling finder
    gif_capture.rs             — showcase GIF generator
    benchmark_simple.rs        — raw sprite FPS counter
    test_optimizations.rs      — renderer optimization checks
    hello.rs                   — minimal sanity check
    shapes.rs                  — basic shape rendering
    sprites.rs                 — texture loading + drawing
    mixed.rs                   — camera + input demo
    interactive.rs             — camera pan/zoom controls
    new_features.rs            — rounded rects, paths, strokes
    advanced_features.rs       — blend modes, gradients
    feature_showcase.rs        — sprite sheets, animations
    next_features.rs           — nine-slice, particles, tilemaps, text
    all_new_features.rs        — 15 features in one window
```

---

## known issues

- **`cargo test --workspace` linker collisions** — glow and wgpu backends share example names which causes link errors. test crates individually with `-p lite-render-2d-core` or `-p lite-render-2d-glow`.
- **`Color::from_hex` RRGGBBAA bug** — values starting with `00` (e.g. `0x00FF00FF`) get misread as 6-char RGB because it checks `hex > 0xFFFFFF`. unit test documents the bug.
