use std::f32::consts::PI;

use lite_render_2d_core::{Color, Renderer, SpriteParams, TextureHandle, TextureParams, Transform2D, Vec2};
use lite_render_2d_glow::GlowRenderer;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

/// Generate a simple 64x64 checkerboard PNG in memory.
fn make_checkerboard_png() -> Vec<u8> {
    let (w, h) = (64u32, 64u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let checker = ((x / 8) + (y / 8)) % 2 == 0;
            if checker {
                // magenta
                rgba[i] = 220;
                rgba[i + 1] = 50;
                rgba[i + 2] = 200;
                rgba[i + 3] = 255;
            } else {
                // white
                rgba[i] = 255;
                rgba[i + 1] = 255;
                rgba[i + 2] = 255;
                rgba[i + 3] = 255;
            }
        }
    }
    // encode as PNG bytes
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        image::ImageEncoder::write_image(
            encoder,
            &rgba,
            w,
            h,
            image::ColorType::Rgba8.into(),
        )
        .expect("encode png");
    }
    buf.into_inner()
}

struct App {
    window: Option<Window>,
    renderer: Option<GlowRenderer>,
    texture: Option<TextureHandle>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default().with_title("sprites");
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = GlowRenderer::new(&win).expect("create renderer");
        ren.set_clear_color(Color::new(0.15, 0.15, 0.2, 1.0));

        let png_bytes = make_checkerboard_png();
        let tex = ren
            .load_texture(&png_bytes, TextureParams::default())
            .expect("load texture");

        self.texture = Some(tex);
        self.renderer = Some(ren);
        self.window = Some(win);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(ren) = &mut self.renderer {
                    ren.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let (Some(ren), Some(tex)) = (&mut self.renderer, self.texture) else {
                    return;
                };

                ren.begin_frame().expect("begin frame");

                // 1. default — no rotation, scale 1
                ren.draw_sprite(
                    tex,
                    SpriteParams::new(Transform2D {
                        pos: Vec2::new(50.0, 50.0),
                        ..Default::default()
                    }),
                );

                // 2. rotated 45 degrees
                ren.draw_sprite(
                    tex,
                    SpriteParams::new(Transform2D {
                        pos: Vec2::new(300.0, 80.0),
                        rotation: PI / 4.0,
                        ..Default::default()
                    }),
                );

                // 3. scaled 2x
                ren.draw_sprite(
                    tex,
                    SpriteParams::new(Transform2D {
                        pos: Vec2::new(50.0, 300.0),
                        scale: Vec2::new(2.0, 2.0),
                        ..Default::default()
                    }),
                );

                // 4. flipped + tinted red
                ren.draw_sprite(
                    tex,
                    SpriteParams::new(Transform2D {
                        pos: Vec2::new(350.0, 300.0),
                        scale: Vec2::new(1.5, 1.5),
                        ..Default::default()
                    })
                    .with_flip(true, false)
                    .with_tint(Color::new(1.0, 0.5, 0.5, 1.0)),
                );

                // 5. half-opacity, small rotation
                ren.draw_sprite(
                    tex,
                    SpriteParams::new(Transform2D {
                        pos: Vec2::new(550.0, 150.0),
                        rotation: -0.3,
                        scale: Vec2::new(1.2, 1.2),
                        ..Default::default()
                    })
                    .with_opacity(0.5),
                );

                ren.end_frame().expect("end frame");

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
    let mut app = App {
        window: None,
        renderer: None,
        texture: None,
    };
    event_loop.run_app(&mut app).expect("run");
}
