# lite-render-2d — full feature testing suite

Read the ENTIRE codebase first. Catalog every public feature, then build tests for all of them. Nothing ships untested.

## 1. unit tests (core/)

### types.rs
- Color::new clamps to 0.0..1.0
- Color::rgb sets alpha to 1.0
- Color::from_hex parses "#FF0000", "#00FF00FF", "FF0000", invalid input
- Color::with_alpha returns new color with changed alpha
- Color::RED, GREEN, BLUE, WHITE, BLACK, TRANSPARENT are correct values
- Color == Color works (PartialEq)
- Color::from([1.0, 0.0, 0.0, 1.0]) works
- Color::from(0xFF0000FF_u32) works
- Vec2::new, ZERO, ONE, UP, DOWN, LEFT, RIGHT correct
- Vec2 add, sub, mul, div, neg operators
- Vec2::length — known triangle (3,4) = 5.0
- Vec2::normalize — unit vector has length ~1.0
- Vec2::normalize on ZERO doesnt panic or NaN (returns ZERO or handled)
- Vec2::dot — perpendicular = 0, parallel = product of lengths
- Vec2::distance_to — same as (a - b).length()
- Vec2 PartialEq works
- Vec2::from((1.0, 2.0)) and Vec2::from([1.0, 2.0]) work
- Rect::new(x, y, w, h) stores correctly
- Rect::from_center produces correct pos/size
- Rect::width(), height(), center(), right(), bottom() correct
- Rect::contains — point inside, outside, on edge
- Rect::intersects — overlapping, adjacent, separate, contained
- Rect PartialEq works
- Transform2D::new(x, y) has scale (1,1) rotation 0
- Transform2D::IDENTITY is pos 0, scale 1, rotation 0
- Transform2D::with_scale, with_uniform_scale, with_rotation chain correctly
- Transform2D::with_rotation_deg(90) == with_rotation(PI/2) approximately
- Transform2D PartialEq works

### camera.rs
- Camera2D::new produces valid ortho matrix
- Camera2D::centered puts origin at center of viewport
- Camera2D::default is usable (not zeroed out)
- zoom 1.0 maps viewport correctly
- zoom 2.0 changes visible area correctly
- with_position offsets the view
- look_at sets position to target (or centered on target)
- screen_to_world at camera default maps corners correctly
- world_to_screen is inverse of screen_to_world (roundtrip)
- screen_to_world with zoom applied produces correct result
- screen_to_world with pan applied produces correct result
- zero viewport doesnt panic
- extreme zoom (0.001, 10000) doesnt produce NaN/infinity

### collision helpers (if in core)
- circle vs circle — overlapping, touching, separate
- circle vs rect — inside, outside, edge touching, corner case
- rect vs rect — same as Rect::intersects but verify collision API matches
- point in polygon — convex, concave, on edge, outside
- line vs line intersection — crossing, parallel, collinear, endpoint touching
- line vs rect — intersecting, fully inside, fully outside, touching edge
- line vs circle — intersecting, tangent, missing

### text (if types are in core)
- font loading doesnt panic on valid font
- measure_text returns nonzero size for nonempty string
- measure_text returns zero width for empty string
- word wrap: known string at known width produces expected line count

## 2. integration tests (glow-backend/)

need a real GL context. create a hidden/headless window test harness.

### renderer lifecycle
- Renderer2D::new succeeds
- new with 1x1 window succeeds
- resize to 0x0 doesnt panic
- resize to 1x1, 4096x4096 succeeds
- resize 100 times rapidly — no leak or crash
- begin_frame + end_frame empty frame — no panic
- 1000 empty frames — no memory growth
- begin_frame twice without end_frame — returns error or panics predictably
- end_frame without begin_frame — returns error
- draw methods outside begin/end — returns error
- set_clear_color works (no crash, framebuffer clears to color)

### shapes — rects
- draw_rect with zero size — no panic
- draw_rect with negative size — no panic
- draw_rect fill — renders (check FrameStats shows verts > 0)
- draw_rect stroke — renders
- draw_rect with alpha 0.5 — renders (no crash)
- 10000 rects in one frame — no panic, no leak

### shapes — rounded rects
- draw rounded rect with radius 0 = same as normal rect
- draw rounded rect with radius larger than half the rect size — clamped, no panic
- per-corner radii — no crash
- rounded rect stroke — no crash

### shapes — circles and ellipses
- draw_circle zero radius — no panic
- draw_circle negative radius — no panic
- draw_circle fill and stroke both work
- draw_ellipse — both radii different, renders correctly
- 10000 circles in one frame — no panic

### shapes — arcs
- arc with 0 sweep angle — no panic
- arc with 360 degree sweep — should look like full circle
- arc with negative sweep — handled gracefully

### shapes — triangles
- equilateral triangle renders
- degenerate triangle (all points collinear) — no panic
- zero area triangle — no panic

### shapes — polygons
- convex polygon with 3 verts (= triangle)
- convex polygon with 8 verts (octagon)
- concave polygon — renders without artifacts
- polygon with hole — renders correctly
- polygon with 0 or 1 or 2 verts — returns error or no-op, no panic
- polygon with duplicate adjacent verts — no panic

### lines and paths
- draw_line from == to — no panic
- draw_line zero thickness — no panic or invisible
- polyline with 0 points — no panic
- polyline with 1 point — no panic
- polyline with 1000 points — no panic, reasonable performance
- bezier path — renders without crash
- dashed line — renders
- dotted line — renders
- line cap styles (butt, round, square) — all render without crash
- line join styles (miter, round, bevel) — all render without crash

### sprites
- load_texture from raw RGBA bytes — succeeds
- load_texture from file (png) — succeeds
- load_texture from memory (encoded bytes) — succeeds
- load_texture 1x1, 4096x4096 — both succeed
- load_texture 0 bytes — returns error
- load_texture wrong byte count — returns error
- draw_sprite with valid handle — renders
- draw_sprite with invalid handle — returns error, no crash
- draw_sprite with transform (position, scale, rotation) — renders
- draw_sprite with tint — renders
- draw_sprite with opacity 0.0, 0.5, 1.0 — renders
- draw_sprite with flip horizontal — renders
- draw_sprite with flip vertical — renders
- unload_texture then draw — returns error
- unload_texture invalid handle — no panic
- load 100 textures, unload all — no leak
- load same data twice — different handles

### nine-slice sprites
- nine-slice with valid margins — renders
- nine-slice with margins larger than texture — no panic, clamped
- nine-slice at different sizes — corners stay unscaled, edges stretch

### sprite sheets and animation
- sprite sheet with valid frame size — renders correct frame
- animate through frames — frame index advances
- frame index out of bounds — clamped or wrapped, no panic

### instanced batching
- 1000 sprites same texture — verify FrameStats draw calls = 1
- 500 texture A + 500 texture B — draw calls = 2
- interleave textures — verify sort/batch handles it
- batch size boundary — exactly max + 1 sprites, verify correct flush and no visual glitch

### text rendering
- draw bitmap text — renders
- draw SDF text — renders, looks crisp
- empty string — no panic
- very long string (10000 chars) — no panic, reasonable performance
- text with newlines — renders multiline
- word wrap with max_width — wraps correctly
- rich text with mixed styles — renders without crash
- text with unknown characters — renders placeholder or skips, no panic
- measure_text matches visual output approximately

### gradients
- linear gradient — renders
- radial gradient — renders
- gradient with 0 stops — no panic
- gradient with 1 stop — solid color, no panic
- gradient with many stops (10+) — renders

### blend modes
- each blend mode renders without crash (normal, add, multiply, screen, etc)
- switching blend modes mid-frame works

### camera integration
- set camera then draw — objects offset correctly
- camera zoom affects all draw types equally
- camera affects sprites, shapes, and text
- camera shake — no crash, visual offset
- camera follow — position updates toward target

### render targets
- create render target — succeeds
- draw to render target instead of screen — no crash
- read back render target as texture — works
- nested render targets (if supported) — no crash

### post-processing
- blur — renders
- bloom — renders
- grayscale — renders
- vignette — renders
- brightness — renders
- invert — renders
- chain multiple effects — renders
- effect with zero intensity — no visual change, no crash
- effect with extreme intensity — no crash

### tilemaps
- create tilemap with valid data — renders
- tilemap with multiple layers — renders in correct order
- animated tiles — frame advances
- tilemap frustum culling — only visible tiles rendered (check FrameStats)
- empty tilemap (0 tiles) — no crash
- tilemap with invalid tile ids — handled gracefully

### particle systems
- create particle emitter — emits particles
- particles update and die — no leak after particles expire
- 0 particles — no crash
- 10000 particles — no crash, reasonable performance
- particle system with no texture — uses shape or handled

### trail rendering
- trail with moving source — renders trail behind
- trail with 0 length — no crash
- trail fading works

### FrameStats
- stats after empty frame — all zeros or near-zero
- stats after drawing 100 sprites — draw calls, verts, tex binds are reasonable
- stats.ram_bytes is nonzero
- stats.fps is nonzero after a few frames

## 3. stress test (examples/stress.rs)

```
cargo run --example stress --release
```

### memory leak detection
- draw loop 60 seconds, log ram every 5s — must not grow
- load/unload 50 textures per second for 30s — ram returns to baseline
- create/destroy 100 particle emitters over 30s — no leak
- rapid resize every frame for 10s — no leak
- create/destroy render targets repeatedly — no leak

### edge cases
- 0 objects per frame, 1000 frames
- 1 object per frame, 1000 frames
- 1000000 objects single frame (expect slow, no crash)
- alternate 0 and 10000 objects every frame
- camera at extreme values (zoom 0.001, zoom 10000, position f32::MAX)
- sprites at positions beyond f32 precision (1e10, -1e10)
- draw every shape type in a single frame
- draw every feature (shapes + sprites + text + particles + tilemap + effects) in same frame
- switch blend modes every draw call
- switch render targets every frame

### determinism
- same scene twice — FrameStats identical
- same scene different window size — draw call count identical

## 4. visual regression (examples/visual_test.rs)

renders known scenes to 800x600, captures via glReadPixels, saves as PNG to `test_output/`.

```
cargo run --example visual_test --release
```

### scenes (save each as separate PNG)

#### basics
1. solid_colors — 4 rects: red, green, blue, white on black
2. shapes_fill — rect, rounded rect, circle, ellipse, triangle, all filled
3. shapes_stroke — same shapes, all stroked
4. shapes_alpha — overlapping semi-transparent shapes (test blending)
5. rounded_rect_radii — rounded rects with different per-corner radii
6. polygon_convex — hexagon, octagon
7. polygon_concave — star shape, L-shape
8. polygon_holes — rect with circular hole

#### lines and paths
9. lines_basic — horizontal, vertical, diagonal, different thicknesses
10. line_caps — butt, round, square side by side
11. line_joins — miter, round, bevel on a zigzag path
12. dashed_lines — dashed, dotted, dash-dot patterns
13. bezier_curves — quadratic and cubic bezier paths

#### sprites
14. sprite_basic — single texture, no transform
15. sprite_transforms — same texture at 0°, 45°, 90°, 180°, flipped H, flipped V
16. sprite_scales — same texture at 0.5x, 1x, 2x, 4x
17. sprite_tints — same texture tinted red, green, blue, white, 50% opacity
18. nine_slice — nine-slice sprite at 3 different sizes

#### text
19. text_bitmap — "Hello World" in bitmap font
20. text_sdf — same text in SDF font, multiple sizes (should stay crisp)
21. text_rich — mixed bold/italic/color in one block
22. text_wrap — long paragraph with word wrap

#### advanced
23. gradient_linear — horizontal and diagonal linear gradients
24. gradient_radial — centered radial gradient
25. blend_modes — same sprite drawn with different blend modes, labeled
26. tilemap_basic — small tilemap with visible tile grid
27. particles_snapshot — freeze a particle system mid-emission
28. trail_snapshot — trail behind a circular path
29. render_target — scene drawn to render target, then that texture drawn to screen
30. postfx_blur — scene with blur applied
31. postfx_bloom — scene with bloom applied
32. postfx_grayscale — scene in grayscale
33. postfx_chain — blur + vignette + brightness chained

#### camera
34. camera_offset — shapes at known positions, camera panned (100, 50)
35. camera_zoom — same scene at zoom 0.5, 1.0, 2.0 side by side
36. camera_shake — snapshot during shake (verify offset is nonzero)

#### edge cases
37. subpixel — shapes at fractional positions (0.5, 0.33)
38. batch_boundary — exactly max_batch_size sprites, check for seam
39. mixed_everything — one of every feature type in the same frame
40. empty_frame — nothing drawn, clean clear color

### after generating
- eyeball every PNG manually
- save as golden references for future comparison
- optionally: automated pixel diff against golden set

## 5. API misuse tests

test graceful failure for wrong usage:

### frame management
- end_frame before begin_frame — error
- begin_frame twice — error
- draw after end_frame — error

### texture misuse
- load_texture empty data — error
- load_texture dimensions dont match data — error
- draw_sprite with unloaded handle — error
- unload same handle twice — no panic

### render target misuse
- draw to destroyed render target — error
- destroy render target while active — error or safe fallback

### tilemap misuse
- tilemap with 0 width or 0 height — error or no-op
- tile id out of tileset range — skip or placeholder, no crash

### particle misuse
- particle emitter with 0 max particles — no crash
- negative emission rate — no crash

### text misuse
- draw text with no font loaded — error
- draw text with null/empty font path — error

### general
- create two renderers on same window — error or works, never UB
- use renderer after window destroyed — error, not segfault

## 6. platform test matrix

manual testing per platform:

### on each platform run:
```bash
cargo test --workspace
cargo run --example shapes --release
cargo run --example mixed --release -- --count 10000 --no-vsync
cargo run --example visual_test --release
cargo run --example stress --release
```

### windows
- works at 100%, 150%, 200% display scaling
- nvidia, amd, and intel integrated all work
- alt+tab, minimize+restore, resize all work

### linux
- works on x11 and wayland
- nvidia proprietary and mesa drivers work
- integrated graphics works

### macos
- opengl 4.1 core profile works
- retina display correct resolution
- window resize smooth
- no deprecation warnings at runtime

## file structure

```
core/src/tests/
    types_test.rs
    camera_test.rs
    collision_test.rs
    text_test.rs (if applicable)
glow-backend/src/tests/
    lifecycle_test.rs
    shapes_test.rs
    sprites_test.rs
    text_test.rs
    advanced_test.rs    (gradients, blend modes, render targets, postfx)
    tilemap_test.rs
    particles_test.rs
    batching_test.rs
examples/
    stress.rs
    visual_test.rs
test_output/            (gitignored)
```

## priorities
1. core unit tests (fast, catch math/logic bugs)
2. renderer lifecycle tests (catch crashes on init/resize/frame)
3. shape + sprite drawing tests (catch panics on basic usage)
4. visual regression binary (catch rendering bugs by eyeball)
5. advanced feature tests (text, gradients, postfx, tilemaps, particles)
6. stress test (catch leaks)
7. API misuse tests (catch bad UX)
8. platform matrix (catch cross-platform issues)

## code style
same rules — lowercase internal comments, short, slightly misspelled. test function names should be descriptive: `test_draw_rect_zero_size_no_panic`, `test_color_from_hex_invalid_returns_error`.

## DO NOT
- dont use mocking frameworks
- dont require gpu for core unit tests
- dont test wgpu backend
- dont add sleeps except in the stress test timers
- dont make visual tests depend on pixel-perfect output (gpu AA varies)
- dont test features that dont exist — only test whats actually in the codebase. if a feature from this list isnt implemented, skip its tests entirely.