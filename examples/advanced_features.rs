use lite_render_2d_core::{
    BlendMode, Color, DrawParams, DrawStyle, Rect, Renderer, Vec2,
};
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
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("advanced features test")
            .with_inner_size(winit::dpi::LogicalSize::new(900, 600));
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::new(0.12, 0.12, 0.18, 1.0));
        self.renderer = Some(ren);
        self.window = Some(win);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let ren = self.renderer.as_mut().unwrap();
                ren.begin_frame().unwrap();

                // -- row 1: gradients --
                // linear gradient rect
                ren.draw_rect(
                    Rect { pos: Vec2::new(30.0, 30.0), size: Vec2::new(180.0, 100.0) },
                    DrawParams {
                        style: DrawStyle::LinearGradient {
                            start: Vec2::new(30.0, 30.0),
                            end: Vec2::new(210.0, 130.0),
                            color_start: Color::RED,
                            color_end: Color::BLUE,
                        },
                        blend: BlendMode::Alpha,
                        z_index: 0,
                        opacity: 1.0,
                    },
                );

                // radial gradient circle
                ren.draw_circle(
                    Vec2::new(330.0, 80.0),
                    60.0,
                    DrawParams {
                        style: DrawStyle::RadialGradient {
                            center: Vec2::new(330.0, 80.0),
                            radius: 60.0,
                            color_inner: Color::new(1.0, 1.0, 0.0, 1.0),
                            color_outer: Color::new(1.0, 0.0, 0.5, 1.0),
                        },
                        blend: BlendMode::Alpha,
                        z_index: 0,
                        opacity: 1.0,
                    },
                );

                // gradient rounded rect
                ren.draw_rounded_rect(
                    lite_render_2d_core::RoundedRect::new(
                        Rect { pos: Vec2::new(430.0, 30.0), size: Vec2::new(180.0, 100.0) },
                        20.0,
                    ),
                    DrawParams {
                        style: DrawStyle::LinearGradient {
                            start: Vec2::new(430.0, 30.0),
                            end: Vec2::new(610.0, 30.0),
                            color_start: Color::GREEN,
                            color_end: Color::new(0.0, 0.5, 1.0, 1.0),
                        },
                        blend: BlendMode::Alpha,
                        z_index: 0,
                        opacity: 1.0,
                    },
                );

                // -- row 2: z-index sorting --
                // draw red rect at z=2 FIRST, then blue at z=0, then green at z=1
                // visual result should be: blue behind, green middle, red front
                ren.draw_rect(
                    Rect { pos: Vec2::new(60.0, 180.0), size: Vec2::new(120.0, 100.0) },
                    DrawParams::fill(Color::RED).with_z(2),
                );
                ren.draw_rect(
                    Rect { pos: Vec2::new(30.0, 200.0), size: Vec2::new(120.0, 100.0) },
                    DrawParams::fill(Color::new(0.2, 0.3, 1.0, 0.9)).with_z(0),
                );
                ren.draw_rect(
                    Rect { pos: Vec2::new(90.0, 190.0), size: Vec2::new(120.0, 100.0) },
                    DrawParams::fill(Color::new(0.2, 0.9, 0.3, 0.9)).with_z(1),
                );

                // -- row 2 continued: scissor/clip rect --
                // a big yellow rect clipped to a smaller region
                ren.push_clip_rect(Rect {
                    pos: Vec2::new(300.0, 180.0),
                    size: Vec2::new(150.0, 100.0),
                });
                // draw rect bigger than clip area
                ren.draw_rect(
                    Rect { pos: Vec2::new(270.0, 160.0), size: Vec2::new(250.0, 150.0) },
                    DrawParams::fill(Color::new(1.0, 0.9, 0.2, 1.0)),
                );
                // clipped circle
                ren.draw_circle(
                    Vec2::new(375.0, 230.0),
                    50.0,
                    DrawParams::fill(Color::new(0.8, 0.2, 0.8, 1.0)),
                );
                ren.pop_clip_rect();

                // outline showing the clip boundary
                ren.draw_rect(
                    Rect { pos: Vec2::new(300.0, 180.0), size: Vec2::new(150.0, 100.0) },
                    DrawParams::stroke(Color::WHITE, 2.0),
                );

                // -- row 3: blend modes --
                // background rect so blending is visble
                ren.draw_rect(
                    Rect { pos: Vec2::new(30.0, 350.0), size: Vec2::new(550.0, 120.0) },
                    DrawParams::fill(Color::new(0.3, 0.3, 0.5, 1.0)),
                );

                // normal alpha blend
                ren.draw_rect(
                    Rect { pos: Vec2::new(50.0, 370.0), size: Vec2::new(100.0, 80.0) },
                    DrawParams::fill(Color::new(1.0, 0.3, 0.3, 0.7))
                        .with_blend(BlendMode::Alpha),
                );

                // additive blend - should glow bright
                ren.draw_rect(
                    Rect { pos: Vec2::new(200.0, 370.0), size: Vec2::new(100.0, 80.0) },
                    DrawParams::fill(Color::new(0.3, 0.8, 1.0, 0.7))
                        .with_blend(BlendMode::Additive),
                );

                // multiply blend - should darken
                ren.draw_rect(
                    Rect { pos: Vec2::new(350.0, 370.0), size: Vec2::new(100.0, 80.0) },
                    DrawParams::fill(Color::new(1.0, 0.6, 0.2, 0.7))
                        .with_blend(BlendMode::Multiply),
                );

                // -- row 3: nested clip rects --
                ren.push_clip_rect(Rect {
                    pos: Vec2::new(620.0, 180.0),
                    size: Vec2::new(200.0, 150.0),
                });
                ren.draw_rect(
                    Rect { pos: Vec2::new(610.0, 170.0), size: Vec2::new(220.0, 170.0) },
                    DrawParams::fill(Color::new(0.2, 0.5, 0.3, 1.0)),
                );
                // inner clip
                ren.push_clip_rect(Rect {
                    pos: Vec2::new(650.0, 210.0),
                    size: Vec2::new(120.0, 80.0),
                });
                ren.draw_circle(
                    Vec2::new(710.0, 250.0),
                    60.0,
                    DrawParams::fill(Color::new(1.0, 0.8, 0.2, 1.0)),
                );
                ren.pop_clip_rect();
                ren.pop_clip_rect();

                // clip boundary outlines
                ren.draw_rect(
                    Rect { pos: Vec2::new(620.0, 180.0), size: Vec2::new(200.0, 150.0) },
                    DrawParams::stroke(Color::WHITE, 1.0),
                );
                ren.draw_rect(
                    Rect { pos: Vec2::new(650.0, 210.0), size: Vec2::new(120.0, 80.0) },
                    DrawParams::stroke(Color::new(1.0, 1.0, 0.5, 1.0), 1.0),
                );

                // -- labels (using simple rects as placeholders) --
                // gradient label area
                ren.draw_rect(
                    Rect { pos: Vec2::new(30.0, 140.0), size: Vec2::new(80.0, 3.0) },
                    DrawParams::fill(Color::WHITE),
                );
                // z-index label area
                ren.draw_rect(
                    Rect { pos: Vec2::new(30.0, 310.0), size: Vec2::new(60.0, 3.0) },
                    DrawParams::fill(Color::WHITE),
                );
                // blend label area
                ren.draw_rect(
                    Rect { pos: Vec2::new(30.0, 480.0), size: Vec2::new(60.0, 3.0) },
                    DrawParams::fill(Color::WHITE),
                );

                ren.end_frame().unwrap();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = App {
        window: None,
        renderer: None,
    };
    event_loop.run_app(&mut app).expect("run app");
}
