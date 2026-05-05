# Getting started

## Install

```sh
cargo add lite-render-2d-glow
```

This pulls in `lite-render-2d-core` automatically. Default features include `paths` (bezier tessellation) and `text` (TTF font rendering).

Optional features you can enable on the core crate:

| Feature | What it adds |
|---------|-------------|
| `input` | Gamepad + keyboard input manager (gilrs) |
| `audio` | Sound playback (rodio) |
| `svg`   | SVG parsing and rendering (usvg) |

## Minimal example

This opens a window and draws a red rectangle, a blue circle, and a white line. Copy-paste it and it runs.

```rust
use lite_render_2d_core::prelude::*;
use lite_render_2d_glow::GlowRenderer;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

struct App {
    window: Option<Window>,
    renderer: Option<GlowRenderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        // Create window
        let win = event_loop
            .create_window(WindowAttributes::default().with_title("hello"))
            .expect("create window");

        // Create renderer attached to the window
        let mut ren = GlowRenderer::new(&win).expect("create renderer");
        ren.set_clear_color(Color::rgb(0.15, 0.15, 0.2));

        self.renderer = Some(ren);
        self.window = Some(win);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            // Close on X button
            WindowEvent::CloseRequested => event_loop.exit(),

            // Handle resize
            WindowEvent::Resized(size) => {
                if let Some(ren) = &mut self.renderer {
                    ren.resize(size.width, size.height);
                }
            }

            // Draw every frame
            WindowEvent::RedrawRequested => {
                if let Some(ren) = &mut self.renderer {
                    // 1. Start the frame
                    ren.begin_frame().expect("begin frame");

                    // 2. Draw stuff
                    ren.draw_rect(
                        Rect::new(50.0, 50.0, 200.0, 100.0),
                        DrawParams::fill(Color::RED),
                    );
                    ren.draw_circle(
                        Vec2::new(400.0, 200.0),
                        60.0,
                        DrawParams::fill(Color::BLUE),
                    );
                    ren.draw_line(
                        Vec2::new(50.0, 300.0),
                        Vec2::new(500.0, 300.0),
                        LineParams::new(Color::WHITE, 2.0),
                    );

                    // 3. End the frame — submits to GPU
                    let _stats = ren.end_frame().expect("end frame");
                }

                // Request next frame
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App { window: None, renderer: None };
    event_loop.run_app(&mut app).expect("run");
}
```

## Frame lifecycle

Every frame follows this pattern:

1. **`GlowRenderer::new(&window)`** — create renderer once (in `resumed`)
2. **`renderer.begin_frame()`** — start a new frame, clears the screen
3. **Draw stuff** — shapes, sprites, text, in any order
4. **`renderer.end_frame()`** — submit to GPU, returns `FrameStats`
5. **Repeat** every frame via `request_redraw()`

All draw calls MUST happen between `begin_frame` and `end_frame`. The order of draws doesn't matter — the renderer sorts and batches automatically.

## Coordinate system

- Origin is **top-left (0, 0)**
- **x** increases rightward
- **y** increases downward
- Rotation is in **radians**, **clockwise**
- Use `with_rotation_deg()` for degrees

```
(0,0) ──────► x
  │
  │
  ▼
  y
```

## Error handling

All fallible methods return `Result<T, RendererError>`. The error variants are:

| Variant | When it happens |
|---------|----------------|
| `ContextCreation` | GPU drivers missing or context creation failed |
| `Surface` | Display surface error (e.g., after minimize) |
| `Shader` | Custom material GLSL/WGSL syntax error |
| `Texture` | Invalid image data or texture too large for GPU |
| `Font` | Invalid TTF/OTF font data |
| `Io` | File read failure (includes path and source error) |
| `Other` | Catch-all for unimplemented features |

In examples we use `.expect()` or `.unwrap()` for brevity. In real apps, handle errors with `?` or `anyhow`.

## Using the wgpu backend

Same API, just change the import:

```rust
use lite_render_2d_wgpu::WgpuRenderer;

let renderer = WgpuRenderer::new(&window)?;
```

Both backends implement the same `Renderer` trait. Your drawing code doesn't change.

## Next steps

- [Drawing Shapes](shapes.md) — rects, circles, polygons, lines, gradients
- [Sprites & Textures](sprites.md) — loading images, transforms, batching
- [Camera](camera.md) — pan, zoom, mouse-to-world conversion
