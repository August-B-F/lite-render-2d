//! Minimal example: open a window, draw a red rectangle and a blue circle.
//! The "does it work?" test.

use lite_render_2d_core::prelude::*;
#[cfg(feature = "use-glow")]
use lite_render_2d_glow::GlowRenderer as Renderer2D;
#[cfg(feature = "use-wgpu")]
use lite_render_2d_wgpu::WgpuRenderer as Renderer2D;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

struct App {
    window: Option<Window>,
    renderer: Option<Renderer2D>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let win = event_loop
            .create_window(WindowAttributes::default().with_title("hello"))
            .expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::rgb(0.15, 0.15, 0.2));
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
                if let Some(ren) = &mut self.renderer {
                    ren.begin_frame().expect("begin frame");

                    ren.draw_rect(Rect::new(50.0, 50.0, 200.0, 100.0), DrawParams::fill(Color::RED));
                    ren.draw_circle(Vec2::new(400.0, 200.0), 60.0, DrawParams::fill(Color::BLUE));
                    ren.draw_line(
                        Vec2::new(50.0, 300.0),
                        Vec2::new(500.0, 300.0),
                        LineParams::new(Color::WHITE, 2.0),
                    );

                    ren.end_frame().expect("end frame");
                }
                if let Some(win) = &self.window { win.request_redraw(); }
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
