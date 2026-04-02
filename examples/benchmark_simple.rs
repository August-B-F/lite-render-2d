//! Simplest possible stress test. Draw N sprites, print FPS.
//! Change SPRITE_COUNT to experiment.

use std::time::Instant;

use lite_render_2d_core::prelude::*;
#[cfg(feature = "use-glow")]
use lite_render_2d_glow::GlowRenderer as Renderer2D;
#[cfg(feature = "use-wgpu")]
use lite_render_2d_wgpu::WgpuRenderer as Renderer2D;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const SPRITE_COUNT: u32 = 10_000;

fn make_tiny_png() -> Vec<u8> {
    let (w, h) = (16u32, 16u32);
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            rgba[i] = 200; rgba[i + 1] = 80; rgba[i + 2] = 220;
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
    frame_count: u32,
    last_print: Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let win = event_loop
            .create_window(WindowAttributes::default().with_title("benchmark_simple"))
            .expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::BLACK);
        let tex = ren.load_texture(&make_tiny_png(), TextureParams::default()).expect("load tex");
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

                for i in 0..SPRITE_COUNT {
                    let fi = i as f32;
                    let x = (fi * 137.5) % 1024.0;
                    let y = (fi * 97.3) % 768.0;
                    ren.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y)));
                }

                let stats = ren.end_frame().expect("end frame");

                self.frame_count += 1;
                if self.frame_count % 60 == 0 {
                    let elapsed = self.last_print.elapsed().as_secs_f64();
                    let fps = 60.0 / elapsed;
                    println!(
                        "sprites: {}  fps: {:.1}  draw_calls: {}  verts: {}",
                        SPRITE_COUNT, fps, stats.draw_calls, stats.vertices,
                    );
                    self.last_print = Instant::now();
                }

                if let Some(win) = &self.window { win.request_redraw(); }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(win) = &self.window { win.request_redraw(); }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    println!("drawing {} sprites per frame (vsync off)", SPRITE_COUNT);
    let mut app = App {
        window: None,
        renderer: None,
        texture: None,
        frame_count: 0,
        last_print: Instant::now(),
    };
    event_loop.run_app(&mut app).expect("run");
}
