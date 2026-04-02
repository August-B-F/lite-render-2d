//! Sprite rendering example: load a texture, draw it at several positions
//! with different transforms. Shows Transform2D builder pattern.

use lite_render_2d_core::prelude::*;
#[cfg(feature = "use-glow")]
use lite_render_2d_glow::GlowRenderer as Renderer2D;
#[cfg(feature = "use-wgpu")]
use lite_render_2d_wgpu::WgpuRenderer as Renderer2D;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

/// Generate a 64x64 checkerboard PNG in memory (no file I/O needed).
fn make_checkerboard_png() -> Vec<u8> {
    let (w, h) = (64u32, 64u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let light = ((x / 8) + (y / 8)) % 2 == 0;
            let (r, g, b) = if light { (220, 50, 200) } else { (255, 255, 255) };
            rgba[i] = r; rgba[i + 1] = g; rgba[i + 2] = b; rgba[i + 3] = 255;
        }
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    let enc = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(enc, &rgba, w, h, image::ColorType::Rgba8.into())
        .expect("encode png");
    buf.into_inner()
}

struct App {
    window: Option<Window>,
    renderer: Option<Renderer2D>,
    texture: Option<TextureHandle>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let win = event_loop
            .create_window(WindowAttributes::default().with_title("sprites"))
            .expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::rgb(0.15, 0.15, 0.2));

        let tex = ren.load_texture(&make_checkerboard_png(), TextureParams::default())
            .expect("load texture");

        self.texture = Some(tex);
        self.renderer = Some(ren);
        self.window = Some(win);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(ren) = &mut self.renderer { ren.resize(size.width, size.height); }
            }
            WindowEvent::RedrawRequested => {
                let (Some(ren), Some(tex)) = (&mut self.renderer, self.texture) else { return; };
                ren.begin_frame().expect("begin frame");

                // 1. default — position only
                ren.draw_sprite(tex, SpriteParams::new(Transform2D::new(50.0, 50.0)));

                // 2. rotated 45 degrees
                ren.draw_sprite(tex, SpriteParams::new(
                    Transform2D::new(300.0, 80.0).with_rotation_deg(45.0),
                ));

                // 3. scaled 2x
                ren.draw_sprite(tex, SpriteParams::new(
                    Transform2D::new(50.0, 300.0).with_uniform_scale(2.0),
                ));

                // 4. flipped + tinted red
                ren.draw_sprite(tex, SpriteParams::new(
                    Transform2D::new(350.0, 300.0).with_uniform_scale(1.5),
                ).with_flip(true, false).with_tint(Color::rgb(1.0, 0.5, 0.5)));

                // 5. half-opacity, small rotation
                ren.draw_sprite(tex, SpriteParams::new(
                    Transform2D::new(550.0, 150.0).with_rotation(-0.3).with_uniform_scale(1.2),
                ).with_opacity(0.5));

                ren.end_frame().expect("end frame");
                if let Some(win) = &self.window { win.request_redraw(); }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App { window: None, renderer: None, texture: None };
    event_loop.run_app(&mut app).expect("run");
}
