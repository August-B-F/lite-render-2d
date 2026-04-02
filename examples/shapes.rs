use lite_render_2d_core::{Color, DrawParams, LineParams, Rect, Renderer, Vec2};
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
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default().with_title("shapes");
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = GlowRenderer::new(&win).expect("create renderer");
        // cornflower blue
        ren.set_clear_color(Color::new(0.392, 0.584, 0.929, 1.0));
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
                if let Some(ren) = &mut self.renderer {
                    ren.begin_frame().expect("begin frame");

                    // -- rects --
                    ren.draw_rect(
                        Rect { pos: Vec2::new(50.0, 50.0), size: Vec2::new(200.0, 120.0) },
                        DrawParams::fill(Color::RED),
                    );
                    ren.draw_rect(
                        Rect { pos: Vec2::new(300.0, 50.0), size: Vec2::new(160.0, 160.0) },
                        DrawParams::stroke(Color::GREEN, 3.0),
                    );
                    // semi-transparent rect
                    ren.draw_rect(
                        Rect { pos: Vec2::new(120.0, 100.0), size: Vec2::new(180.0, 100.0) },
                        DrawParams::fill(Color::new(1.0, 1.0, 0.0, 1.0)).with_opacity(0.5),
                    );

                    // -- circles --
                    ren.draw_circle(
                        Vec2::new(150.0, 320.0),
                        60.0,
                        DrawParams::fill(Color::BLUE),
                    );
                    ren.draw_circle(
                        Vec2::new(350.0, 320.0),
                        50.0,
                        DrawParams::stroke(Color::WHITE, 4.0),
                    );
                    ren.draw_circle(
                        Vec2::new(530.0, 320.0),
                        45.0,
                        DrawParams::fill(Color::new(0.0, 0.8, 0.6, 1.0)),
                    );

                    // -- lines --
                    ren.draw_line(
                        Vec2::new(50.0, 450.0),
                        Vec2::new(550.0, 450.0),
                        LineParams::new(Color::WHITE, 2.0),
                    );
                    ren.draw_line(
                        Vec2::new(50.0, 480.0),
                        Vec2::new(400.0, 540.0),
                        LineParams::new(Color::new(1.0, 0.5, 0.0, 1.0), 5.0),
                    );
                    ren.draw_line(
                        Vec2::new(300.0, 200.0),
                        Vec2::new(500.0, 400.0),
                        LineParams::new(Color::new(1.0, 0.0, 0.8, 1.0), 3.0),
                    );

                    ren.end_frame().expect("end frame");
                }
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
    };
    event_loop.run_app(&mut app).expect("run");
}
