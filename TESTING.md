# lite-render-2d testing

all the test commands in one place. copy-paste and go.

---

## unit tests (no gpu needed)

pure logic tests for math types, camera, and collision in `core/`.
run these first — they're fast and catch the dumbest bugs.

```bash
cargo test -p lite-render-2d-core --release
```

run just one module:

```bash
cargo test -p lite-render-2d-core --release tests::types_test
cargo test -p lite-render-2d-core --release tests::camera_test
cargo test -p lite-render-2d-core --release tests::collision_test
```

run a single test:

```bash
cargo test -p lite-render-2d-core --release test_color_from_hex_str_invalid
```

what's tested:
- **types_test** (48 tests) — Color constants, rgb, hex parsing, lerp, srgb, hsl, From impls. Vec2 math ops, length, normalize, dot, cross, distance. Rect constructors and accessors. Transform2D builders and chaining.
- **camera_test** (17 tests) — ortho projection, screen_to_world/world_to_screen roundtrips, zoom halves visible area, pan offsets, follow lerp, shake decay, zero viewport, extreme zoom.
- **collision_test** (18 tests) — rect contains/intersects, circle_contains, circle_intersects_rect, point_in_polygon (convex, concave, degenerate), line_intersects_line (crossing, parallel, collinear, endpoint).

---

## integration tests (needs gpu)

opens a window, runs 45+ tests against the real glow renderer, prints pass/fail, exits.
covers renderer lifecycle, shapes, lines, sprites, blend modes.

```bash
cargo run -p lite-render-2d-glow --example integration_tests --release
```

what's tested:
- **lifecycle** — empty frames, 100 empty frames, set_clear_color, resize 1x1, resize 4096x4096, resize 100 times, framestats after drawing
- **shapes** — rect fill/stroke/alpha/zero/negative/10k, rounded rect basic/zero/huge/per-corner, circle fill/stroke/zero/negative/10k, ellipse, arc normal/zero/full/negative sweep, triangle normal/degenerate/zero-area, polygon octagon/0/1/2 verts
- **lines** — basic, same point, zero thickness, polyline 0/1/1000 points
- **sprites** — load 1x1 png, load empty (expect error), draw valid, draw with transform, tint+opacity, flip, unload then draw, invalid handle draw/unload, load 100 + unload all, same data different handles
- **blend** — all 5 blend modes in one frame

---

## visual examples (needs gpu)

demos you run and look at. useful for eyeballing rendering correctness.

```bash
# minimal sanity check — red rect + blue circle
cargo run -p lite-render-2d-glow --example hello --release

# rectangles, lines, circles
cargo run -p lite-render-2d-glow --example shapes --release

# texture loading, sprite transforms
cargo run -p lite-render-2d-glow --example sprites --release

# camera pan (arrow keys), zoom (scroll), mouse crosshair
cargo run -p lite-render-2d-glow --example mixed --release

# camera pan/zoom + world-space crosshair
cargo run -p lite-render-2d-glow --example interactive --release

# rounded rects, paths, stroke/line styling
cargo run -p lite-render-2d-glow --example new_features --release

# blend modes, draw styles
cargo run -p lite-render-2d-glow --example advanced_features --release

# blend modes, line styles, sprite animations, sprite sheets
cargo run -p lite-render-2d-glow --example feature_showcase --release

# particles, tilemaps, nine-slice, text, draw styles
cargo run -p lite-render-2d-glow --example next_features --release

# 15 features: animated tiles, cameras, gradients, render targets, trails
cargo run -p lite-render-2d-glow --example all_new_features --release
```

---

## benchmarks (needs gpu)

```bash
# sprite stress test — draws N sprites, prints FPS
cargo run -p lite-render-2d-glow --example benchmark_simple --release

# tests 7 renderer optimizations: frustum culling, instanced draw, atlas regrow
cargo run -p lite-render-2d-glow --example test_optimizations --release
```

---

## file map

```
core/src/tests/
    mod.rs                — test module root
    types_test.rs         — Color, Vec2, Rect, Transform2D        (48 tests)
    camera_test.rs        — Camera2D projection, coordinate conv   (17 tests)
    collision_test.rs     — rect/circle/polygon/line collision     (18 tests)

examples/
    integration_tests.rs  — automated glow renderer integration    (45+ tests)
    hello.rs              — minimal window sanity check
    shapes.rs             — basic shape rendering
    sprites.rs            — texture loading + sprite drawing
    mixed.rs              — camera + input demo
    interactive.rs        — camera pan/zoom + mouse crosshair
    new_features.rs       — rounded rects, paths, strokes
    advanced_features.rs  — blend modes, draw styles
    feature_showcase.rs   — animations, sprite sheets, line styles
    next_features.rs      — particles, tilemaps, nine-slice, text
    all_new_features.rs   — comprehensive 15-feature demo
    benchmark_simple.rs   — sprite count stress test
    test_optimizations.rs — renderer optimization verification
```

---

## known issues

- **`cargo test --workspace` linker collisions** — glow and wgpu backends share example names which causes link errors. test crates individually with `-p lite-render-2d-core` or `-p lite-render-2d-glow`.
- **`Color::from_hex` RRGGBBAA bug** — values starting with `00` (e.g. `0x00FF00FF`) get misread as 6-char RGB because it checks `hex > 0xFFFFFF`. unit test works around it with a comment noting the bug.
