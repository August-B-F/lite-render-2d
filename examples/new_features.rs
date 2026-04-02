use lite_render_2d_core::{
    Color, DrawParams, LineParams, Path, Rect, Renderer, RoundedRect, StrokeParams, Transform2D,
    Vec2,
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
            .with_title("new features test")
            .with_inner_size(winit::dpi::LogicalSize::new(900, 700));
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::new(0.15, 0.15, 0.2, 1.0));
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

                    // ── Row 1: Triangles & Polygons ──

                    // filled triangle
                    ren.draw_triangle(
                        Vec2::new(50.0, 30.0),
                        Vec2::new(150.0, 130.0),
                        Vec2::new(20.0, 130.0),
                        DrawParams::fill(Color::new(1.0, 0.4, 0.2, 1.0)),
                    );

                    // stroked triangle
                    ren.draw_triangle(
                        Vec2::new(180.0, 30.0),
                        Vec2::new(280.0, 130.0),
                        Vec2::new(160.0, 130.0),
                        DrawParams::stroke(Color::new(0.2, 1.0, 0.6, 1.0), 2.0),
                    );

                    // filled pentagon
                    let pentagon: Vec<Vec2> = (0..5)
                        .map(|i| {
                            let a = std::f32::consts::TAU * i as f32 / 5.0 - std::f32::consts::FRAC_PI_2;
                            Vec2::new(370.0 + a.cos() * 55.0, 80.0 + a.sin() * 55.0)
                        })
                        .collect();
                    ren.draw_polygon(
                        &pentagon,
                        DrawParams::fill(Color::new(0.3, 0.5, 1.0, 1.0)),
                    );

                    // stroked hexagon
                    let hexagon: Vec<Vec2> = (0..6)
                        .map(|i| {
                            let a = std::f32::consts::TAU * i as f32 / 6.0;
                            Vec2::new(510.0 + a.cos() * 50.0, 80.0 + a.sin() * 50.0)
                        })
                        .collect();
                    ren.draw_polygon(
                        &hexagon,
                        DrawParams::stroke(Color::new(1.0, 0.8, 0.0, 1.0), 2.5),
                    );

                    // ── Row 2: Ellipses & Arcs ──

                    // filled ellipse
                    ren.draw_ellipse(
                        Vec2::new(100.0, 210.0),
                        Vec2::new(80.0, 40.0),
                        DrawParams::fill(Color::new(0.8, 0.2, 0.8, 1.0)),
                    );

                    // stroked ellipse
                    ren.draw_ellipse(
                        Vec2::new(280.0, 210.0),
                        Vec2::new(50.0, 70.0),
                        DrawParams::stroke(Color::new(0.0, 1.0, 1.0, 1.0), 2.0),
                    );

                    // filled arc (pie slice)
                    ren.draw_arc(
                        Vec2::new(440.0, 210.0),
                        60.0,
                        -std::f32::consts::FRAC_PI_4,
                        std::f32::consts::PI,
                        DrawParams::fill(Color::new(1.0, 0.6, 0.0, 1.0)),
                    );

                    // stroked arc
                    ren.draw_arc(
                        Vec2::new(590.0, 210.0),
                        50.0,
                        0.0,
                        std::f32::consts::PI * 1.5,
                        DrawParams::stroke(Color::GREEN, 3.0),
                    );

                    // ── Row 3: Rounded Rects ──

                    // filled rounded rect
                    ren.draw_rounded_rect(
                        RoundedRect {
                            rect: Rect { pos: Vec2::new(30.0, 300.0), size: Vec2::new(180.0, 90.0) },
                            radius: 15.0,
                        },
                        DrawParams::fill(Color::new(0.2, 0.7, 0.4, 1.0)),
                    );

                    // stroked rounded rect
                    ren.draw_rounded_rect(
                        RoundedRect {
                            rect: Rect { pos: Vec2::new(240.0, 300.0), size: Vec2::new(160.0, 90.0) },
                            radius: 25.0,
                        },
                        DrawParams::stroke(Color::new(1.0, 0.3, 0.5, 1.0), 3.0),
                    );

                    // rounded rect with large radius (pill shape)
                    ren.draw_rounded_rect(
                        RoundedRect {
                            rect: Rect { pos: Vec2::new(430.0, 310.0), size: Vec2::new(200.0, 60.0) },
                            radius: 30.0,
                        },
                        DrawParams::fill(Color::new(0.4, 0.4, 0.9, 1.0)),
                    );

                    // ── Row 4: Polylines & Bezier Paths ──

                    // polyline (zigzag)
                    let zigzag: Vec<Vec2> = (0..8)
                        .map(|i| {
                            Vec2::new(
                                30.0 + i as f32 * 40.0,
                                440.0 + if i % 2 == 0 { 0.0 } else { 40.0 },
                            )
                        })
                        .collect();
                    ren.draw_polyline(
                        &zigzag,
                        LineParams::new(Color::new(1.0, 1.0, 0.3, 1.0), 3.0),
                    );

                    // bezier path (heart shape)
                    let heart = Path::new()
                        .move_to(Vec2::new(450.0, 460.0))
                        .cubic_to(
                            Vec2::new(450.0, 430.0),
                            Vec2::new(400.0, 420.0),
                            Vec2::new(400.0, 450.0),
                        )
                        .cubic_to(
                            Vec2::new(400.0, 470.0),
                            Vec2::new(450.0, 490.0),
                            Vec2::new(450.0, 510.0),
                        )
                        .cubic_to(
                            Vec2::new(450.0, 490.0),
                            Vec2::new(500.0, 470.0),
                            Vec2::new(500.0, 450.0),
                        )
                        .cubic_to(
                            Vec2::new(500.0, 420.0),
                            Vec2::new(450.0, 430.0),
                            Vec2::new(450.0, 460.0),
                        )
                        .close();

                    ren.draw_path(
                        &heart,
                        DrawParams::fill(Color::RED),
                    );

                    // stroked bezier curve
                    let wave = Path::new()
                        .move_to(Vec2::new(550.0, 440.0))
                        .cubic_to(
                            Vec2::new(600.0, 400.0),
                            Vec2::new(650.0, 500.0),
                            Vec2::new(700.0, 440.0),
                        )
                        .cubic_to(
                            Vec2::new(750.0, 380.0),
                            Vec2::new(800.0, 520.0),
                            Vec2::new(850.0, 440.0),
                        );
                    ren.stroke_path(
                        &wave,
                        StrokeParams::new(Color::new(0.5, 0.8, 1.0, 1.0), 3.0),
                    );

                    // ── Row 5: Transform stack demo ──

                    // draw 3 rects with progressive rotation using transform stack
                    let colors = [
                        Color::new(1.0, 0.2, 0.2, 0.8),
                        Color::new(0.2, 1.0, 0.2, 0.8),
                        Color::new(0.2, 0.2, 1.0, 0.8),
                    ];
                    for (i, color) in colors.iter().enumerate() {
                        ren.push_transform(Transform2D {
                            pos: Vec2::new(150.0, 600.0),
                            scale: Vec2::ONE,
                            rotation: i as f32 * 0.3,
                        });
                        ren.draw_rect(
                            Rect {
                                pos: Vec2::new(-40.0, -20.0),
                                size: Vec2::new(80.0, 40.0),
                            },
                            DrawParams::fill(*color),
                        );
                        ren.pop_transform();
                    }

                    // nested transforms: parent rotation + child offset
                    ren.push_transform(Transform2D {
                        pos: Vec2::new(400.0, 600.0),
                        scale: Vec2::ONE,
                        rotation: 0.5,
                    });
                    ren.draw_circle(
                        Vec2::ZERO,
                        30.0,
                        DrawParams::fill(Color::new(1.0, 0.5, 0.0, 1.0)),
                    );
                    // child: offset from parent
                    ren.push_transform(Transform2D {
                        pos: Vec2::new(80.0, 0.0),
                        scale: Vec2::new(0.7, 0.7),
                        rotation: 0.0,
                    });
                    ren.draw_circle(
                        Vec2::ZERO,
                        25.0,
                        DrawParams::fill(Color::new(0.0, 0.8, 1.0, 1.0)),
                    );
                    ren.pop_transform();
                    ren.pop_transform();

                    // ── Labels ──
                    ren.draw_line(
                        Vec2::new(0.0, 150.0), Vec2::new(900.0, 150.0),
                        LineParams::new(Color::new(1.0, 1.0, 1.0, 0.2), 1.0),
                    );
                    ren.draw_line(
                        Vec2::new(0.0, 280.0), Vec2::new(900.0, 280.0),
                        LineParams::new(Color::new(1.0, 1.0, 1.0, 0.2), 1.0),
                    );
                    ren.draw_line(
                        Vec2::new(0.0, 410.0), Vec2::new(900.0, 410.0),
                        LineParams::new(Color::new(1.0, 1.0, 1.0, 0.2), 1.0),
                    );
                    ren.draw_line(
                        Vec2::new(0.0, 550.0), Vec2::new(900.0, 550.0),
                        LineParams::new(Color::new(1.0, 1.0, 1.0, 0.2), 1.0),
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
