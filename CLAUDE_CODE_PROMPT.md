# lite-render-2d — lightweight 2d rendering crate

build a rust workspace crate called `lite-render-2d` that replaces wgpu for 2d-only apps. the goal is minimal ram footprint (target <15mb total) while being cross-platform (windows, linux, macos).

## workspace structure

```
lite-render-2d/
├── Cargo.toml          # workspace root
├── core/               # traits, types, math — zero backend deps
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── renderer.rs # main Renderer trait
│       ├── types.rs    # Color, Rect, Vec2, Transform2D, etc
│       ├── texture.rs  # TextureHandle, TextureParams
│       └── camera.rs   # Camera2D (ortho projection, pan, zoom)
├── glow-backend/       # opengl es 3.0 backend via glow + glutin
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── context.rs  # window + gl context setup via winit + glutin
│       ├── renderer.rs # implements core::Renderer
│       ├── shaders.rs  # baked-in glsl shaders (vert/frag for quads, shapes)
│       ├── batch.rs    # draw call batcher — sprites and shapes
│       └── texture.rs  # gl texture loading, atlas support
├── wgpu-backend/       # thin wgpu wrapper implementing same traits (fallback)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── renderer.rs
└── examples/
    ├── shapes.rs       # rects, circles, lines demo
    ├── sprites.rs      # textured quads demo
    └── mixed.rs        # both together + camera controls
```

## core traits (core/src/renderer.rs)

```rust
pub trait Renderer {
    fn new(window: &winit::window::Window) -> Result<Self, RendererError> where Self: Sized;
    fn resize(&mut self, width: u32, height: u32);
    fn set_camera(&mut self, camera: &Camera2D);
    fn begin_frame(&mut self) -> Result<(), RendererError>;

    // shapes
    fn draw_rect(&mut self, rect: Rect, style: DrawStyle);
    fn draw_circle(&mut self, center: Vec2, radius: f32, style: DrawStyle);
    fn draw_line(&mut self, from: Vec2, to: Vec2, thickness: f32, color: Color);

    // sprites
    fn load_texture(&mut self, data: &[u8], params: TextureParams) -> Result<TextureHandle, RendererError>;
    fn unload_texture(&mut self, handle: TextureHandle);
    fn draw_sprite(&mut self, handle: TextureHandle, transform: Transform2D, tint: Color);

    fn end_frame(&mut self) -> Result<(), RendererError>;
}
```

## core types (core/src/types.rs)

```rust
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }
pub struct Vec2 { pub x: f32, pub y: f32 }
pub struct Rect { pub pos: Vec2, pub size: Vec2 }
pub struct Transform2D { pub pos: Vec2, pub scale: Vec2, pub rotation: f32 }
pub struct TextureHandle(pub(crate) u64);
pub struct TextureParams { pub filter: FilterMode, pub wrap: WrapMode }
pub enum FilterMode { Nearest, Linear }
pub enum WrapMode { Clamp, Repeat }
pub enum DrawStyle {
    Fill(Color),
    Stroke { color: Color, thickness: f32 },
}
```

## camera (core/src/camera.rs)

```rust
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
    pub viewport: Vec2,
}
// should produce an ortho projection matrix (4x4) for the renderer
```

## glow backend implementation details

### dependencies
- `glow` — raw opengl bindings
- `glutin` + `glutin-winit` + `raw-window-handle` — gl context creation
- `winit` — windowing (re-export from core or shared dep)
- `image` — texture decoding (png, jpg)

### shaders (glow-backend/src/shaders.rs)
bake two shader programs as const &str:

1. **quad shader** — instanced rendering of textured/colored quads
   - vertex: takes quad corners + per-instance transform, uv, color
   - fragment: samples texture or uses flat color, multiplies by tint
2. **shape shader** — for circles and lines
   - circles via SDF in fragment shader (pass center + radius as uniforms or instance data)
   - lines as thin quads oriented along the line direction

target glsl 300 es for max compatibility.

### draw batching (glow-backend/src/batch.rs)
this is the performance-critical part:

- batch all sprites sharing the same texture into one draw call
- batch all shapes into one draw call
- use a single dynamic VBO that gets refilled each frame
- sort draw calls by texture to minimize binds
- flush batches on: texture change, shape/sprite switch, or end_frame

vertex layout per quad instance:
```
[pos.x, pos.y, scale.x, scale.y, rotation, uv_min.x, uv_min.y, uv_max.x, uv_max.y, r, g, b, a]
```

### context setup (glow-backend/src/context.rs)
- request opengl es 3.0 or opengl 3.3 core as fallback
- on macos: request forward-compatible core profile (required)
- minimize default buffer sizes — dont allocate a depth buffer, dont allocate stencil unless explicitly needed
- set swap interval to 1 (vsync on by default)

### texture management (glow-backend/src/texture.rs)
- simple hashmap of TextureHandle -> gl texture id
- basic texture atlas support: optional, but if a user loads many small sprites, pack them into one atlas
- handle texture uploads asynchronously-ish (dont block main thread if possible, but fine to start simple/synchronous)

## wgpu backend

thin wrapper — basically just adapts existing wgpu calls to the Renderer trait. this is so existing apps can switch via feature flag:

```toml
[features]
default = ["glow"]
glow = ["lite-render-2d-glow"]
wgpu = ["lite-render-2d-wgpu"]
```

dont over-invest here. keep it simple, just enough to compile and work.

## examples

each example should be ~50-100 lines showing usage:
- `shapes.rs` — draw some rects, circles, lines in different colors
- `sprites.rs` — load a png texture, draw it with rotation/scale
- `mixed.rs` — sprites + shapes + camera pan/zoom (arrow keys + scroll wheel)

## code style rules — IMPORTANT

1. **comments must be**: all lowercase, short, slightly misspelled like a real dev typing fast. sparingly placed — only where logic is non-obvious.

examples of GOOD comments:
```rust
// flush remainig quads before swaping texture
// ortho proj, ignroe z
// sdf circle - discard if outside raduis
// batch is full, submit and reset
// fallback to gl 3.3 if es3 isnt availble
// dont need depth buffer for 2d stuf
```

examples of BAD comments (do NOT write these):
```rust
// This function initializes the OpenGL context
// Calculate the orthographic projection matrix
// Loop through each sprite in the batch
```

2. **no doc comments** (///) on internal functions. only on the public trait methods in core.
3. **variable names**: short but readable. `vbo`, `vao`, `proj`, `cam`, `tex`, `ctx`, `w`, `h` are all fine.
4. **error handling**: use `thiserror` in core for RendererError. in glow backend, liberal use of `.expect("msg")` for gl failures that shouldnt happen — dont over-engineer error paths for init-time gl calls.
5. **no unsafe blocks unless absolutely required** (glow needs some, thats fine — keep them minimal and localized).

## build verification

after building the workspace, run:
1. `cargo check --workspace`
2. `cargo build --workspace`
3. `cargo clippy --workspace`
4. make sure examples compile (they dont need to run headless, just compile)

## priorities (in order)
1. core traits compile and make sense
2. glow backend compiles and renders shapes
3. glow backend renders textured sprites
4. draw batching works correctly
5. camera works
6. wgpu backend compiles (bare minimum)
7. examples compile

do NOT try to get everything perfect in one pass. get shapes rendering first, then add sprites, then batching, then camera. iterative.
