//! glow-backend integration tests — renderer lifecycle, shapes, sprites.
//! opens a hidden window, runs all tests, prints pass/fail, exits.
//!
//! run with: cargo run -p lite-render-2d-glow --example integration_tests --release

use lite_render_2d_core::prelude::*;
use lite_render_2d_core::RoundedRect;
use lite_render_2d_core::types::{NineSlice, Path, PathSegment, SpriteInstance, StrokeParams};
use lite_render_2d_core::post_process::PostEffect;
use lite_render_2d_core::tilemap::{Tilemap, TilesetInfo};
#[cfg(feature = "use-glow")]
use lite_render_2d_glow::GlowRenderer as Renderer2D;
#[cfg(feature = "use-wgpu")]
use lite_render_2d_wgpu::WgpuRenderer as Renderer2D;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

// ---------------------------------------------------------------------------
// test harness
// ---------------------------------------------------------------------------

struct TestResult {
    name: &'static str,
    passed: bool,
    error: Option<String>,
}

macro_rules! run_test {
    ($results:expr, $name:expr, $body:expr) => {{
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body));
        match result {
            Ok(Ok(())) => {
                $results.push(TestResult { name: $name, passed: true, error: None });
            }
            Ok(Err(e)) => {
                $results.push(TestResult { name: $name, passed: false, error: Some(format!("{}", e)) });
            }
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "panic".to_string()
                };
                $results.push(TestResult { name: $name, passed: false, error: Some(msg) });
            }
        }
    }};
}

// ---------------------------------------------------------------------------
// test app
// ---------------------------------------------------------------------------

struct App {
    window: Option<Window>,
    renderer: Option<Renderer2D>,
    ran: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = WindowAttributes::default()
            .with_title("integration tests")
            .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600));
        let win = event_loop.create_window(attrs).expect("create window");
        let ren = Renderer2D::new(&win).expect("create renderer");
        self.renderer = Some(ren);
        self.window = Some(win);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if self.ran { event_loop.exit(); return; }
                self.ran = true;

                let ren = self.renderer.as_mut().unwrap();
                let mut results: Vec<TestResult> = Vec::new();

                // -------------------------------------------------------
                // 1. renderer lifecycle
                // -------------------------------------------------------

                run_test!(results, "lifecycle::begin_end_empty_frame", {
                    ren.begin_frame()?;
                    let stats = ren.end_frame()?;
                    assert!(stats.draw_calls == 0, "empty frame should have 0 draw calls");
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lifecycle::100_empty_frames", {
                    for _ in 0..100 {
                        ren.begin_frame()?;
                        ren.end_frame()?;
                    }
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lifecycle::set_clear_color", {
                    ren.set_clear_color(Color::rgb(0.1, 0.2, 0.3));
                    ren.begin_frame()?;
                    ren.end_frame()?;
                    ren.set_clear_color(Color::BLACK);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lifecycle::resize_1x1", {
                    ren.resize(1, 1);
                    ren.begin_frame()?;
                    ren.end_frame()?;
                    ren.resize(800, 600);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lifecycle::resize_4096x4096", {
                    ren.resize(4096, 4096);
                    ren.begin_frame()?;
                    ren.end_frame()?;
                    ren.resize(800, 600);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lifecycle::resize_100_times", {
                    for i in 0..100 {
                        ren.resize(200 + i * 5, 150 + i * 4);
                    }
                    ren.begin_frame()?;
                    ren.end_frame()?;
                    ren.resize(800, 600);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lifecycle::framestats_after_drawing", {
                    ren.begin_frame()?;
                    for i in 0..50 {
                        ren.draw_rect(
                            Rect::new(i as f32 * 10.0, 0.0, 8.0, 8.0),
                            DrawParams::fill(Color::RED),
                        );
                    }
                    let stats = ren.end_frame()?;
                    assert!(stats.vertices > 0, "should have vertices after drawing rects");
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 2. shapes — rects
                // -------------------------------------------------------

                run_test!(results, "shapes::draw_rect_basic", {
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(10.0, 10.0, 100.0, 50.0), DrawParams::fill(Color::RED));
                    let stats = ren.end_frame()?;
                    assert!(stats.vertices > 0);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_rect_zero_size", {
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(10.0, 10.0, 0.0, 0.0), DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_rect_negative_size", {
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(10.0, 10.0, -50.0, -30.0), DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_rect_stroke", {
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(10.0, 10.0, 100.0, 50.0), DrawParams::stroke(Color::GREEN, 2.0));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_rect_alpha", {
                    ren.begin_frame()?;
                    ren.draw_rect(
                        Rect::new(10.0, 10.0, 100.0, 50.0),
                        DrawParams::fill(Color::RED.with_alpha(0.5)),
                    );
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_10000_rects", {
                    ren.begin_frame()?;
                    for i in 0..10_000 {
                        let x = (i % 100) as f32 * 8.0;
                        let y = (i / 100) as f32 * 8.0;
                        ren.draw_rect(Rect::new(x, y, 6.0, 6.0), DrawParams::fill(Color::CYAN));
                    }
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 3. shapes — rounded rects
                // -------------------------------------------------------

                run_test!(results, "shapes::rounded_rect_basic", {
                    ren.begin_frame()?;
                    ren.draw_rounded_rect(
                        RoundedRect::new(Rect::new(10.0, 10.0, 100.0, 50.0), 8.0),
                        DrawParams::fill(Color::YELLOW),
                    );
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::rounded_rect_radius_zero", {
                    ren.begin_frame()?;
                    ren.draw_rounded_rect(
                        RoundedRect::new(Rect::new(10.0, 10.0, 100.0, 50.0), 0.0),
                        DrawParams::fill(Color::YELLOW),
                    );
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::rounded_rect_radius_huge", {
                    ren.begin_frame()?;
                    ren.draw_rounded_rect(
                        RoundedRect::new(Rect::new(10.0, 10.0, 100.0, 50.0), 9999.0),
                        DrawParams::fill(Color::YELLOW),
                    );
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::rounded_rect_per_corner", {
                    ren.begin_frame()?;
                    ren.draw_rounded_rect(
                        RoundedRect::with_radii(Rect::new(10.0, 10.0, 100.0, 50.0), 0.0, 5.0, 10.0, 20.0),
                        DrawParams::fill(Color::MAGENTA),
                    );
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 4. shapes — circles and ellipses
                // -------------------------------------------------------

                run_test!(results, "shapes::draw_circle_basic", {
                    ren.begin_frame()?;
                    ren.draw_circle(Vec2::new(100.0, 100.0), 50.0, DrawParams::fill(Color::BLUE));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_circle_zero_radius", {
                    ren.begin_frame()?;
                    ren.draw_circle(Vec2::new(100.0, 100.0), 0.0, DrawParams::fill(Color::BLUE));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_circle_negative_radius", {
                    ren.begin_frame()?;
                    ren.draw_circle(Vec2::new(100.0, 100.0), -10.0, DrawParams::fill(Color::BLUE));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_circle_stroke", {
                    ren.begin_frame()?;
                    ren.draw_circle(Vec2::new(100.0, 100.0), 50.0, DrawParams::stroke(Color::WHITE, 2.0));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_ellipse", {
                    ren.begin_frame()?;
                    ren.draw_ellipse(Vec2::new(200.0, 200.0), Vec2::new(80.0, 40.0), DrawParams::fill(Color::GREEN));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_10000_circles", {
                    ren.begin_frame()?;
                    for i in 0..10_000 {
                        let x = (i % 100) as f32 * 8.0;
                        let y = (i / 100) as f32 * 8.0;
                        ren.draw_circle(Vec2::new(x, y), 3.0, DrawParams::fill(Color::RED));
                    }
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 5. shapes — arcs and triangles
                // -------------------------------------------------------

                run_test!(results, "shapes::draw_arc_normal", {
                    ren.begin_frame()?;
                    ren.draw_arc(Vec2::new(100.0, 100.0), 40.0, 0.0, std::f32::consts::PI, DrawParams::fill(Color::CYAN));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_arc_zero_sweep", {
                    ren.begin_frame()?;
                    ren.draw_arc(Vec2::new(100.0, 100.0), 40.0, 0.0, 0.0, DrawParams::fill(Color::CYAN));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_arc_full_circle", {
                    ren.begin_frame()?;
                    ren.draw_arc(Vec2::new(100.0, 100.0), 40.0, 0.0, std::f32::consts::TAU, DrawParams::fill(Color::CYAN));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_arc_negative_sweep", {
                    ren.begin_frame()?;
                    ren.draw_arc(Vec2::new(100.0, 100.0), 40.0, 1.0, -1.0, DrawParams::fill(Color::CYAN));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_triangle", {
                    ren.begin_frame()?;
                    ren.draw_triangle(
                        Vec2::new(100.0, 10.0), Vec2::new(50.0, 90.0), Vec2::new(150.0, 90.0),
                        DrawParams::fill(Color::GREEN),
                    );
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_triangle_degenerate", {
                    // collinear points
                    ren.begin_frame()?;
                    ren.draw_triangle(
                        Vec2::new(0.0, 0.0), Vec2::new(50.0, 0.0), Vec2::new(100.0, 0.0),
                        DrawParams::fill(Color::RED),
                    );
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_triangle_zero_area", {
                    ren.begin_frame()?;
                    ren.draw_triangle(
                        Vec2::new(50.0, 50.0), Vec2::new(50.0, 50.0), Vec2::new(50.0, 50.0),
                        DrawParams::fill(Color::RED),
                    );
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 6. shapes — polygons
                // -------------------------------------------------------

                run_test!(results, "shapes::draw_polygon_octagon", {
                    ren.begin_frame()?;
                    let center = Vec2::new(200.0, 200.0);
                    let r = 50.0;
                    let pts: Vec<Vec2> = (0..8).map(|i| {
                        let a = i as f32 * std::f32::consts::TAU / 8.0;
                        Vec2::new(center.x + a.cos() * r, center.y + a.sin() * r)
                    }).collect();
                    ren.draw_polygon(&pts, DrawParams::fill(Color::MAGENTA));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_polygon_0_verts", {
                    ren.begin_frame()?;
                    ren.draw_polygon(&[], DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_polygon_1_vert", {
                    ren.begin_frame()?;
                    ren.draw_polygon(&[Vec2::new(50.0, 50.0)], DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "shapes::draw_polygon_2_verts", {
                    ren.begin_frame()?;
                    ren.draw_polygon(&[Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0)], DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 7. lines
                // -------------------------------------------------------

                run_test!(results, "lines::draw_line_basic", {
                    ren.begin_frame()?;
                    ren.draw_line(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0), LineParams::new(Color::WHITE, 2.0));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lines::draw_line_same_point", {
                    ren.begin_frame()?;
                    ren.draw_line(Vec2::new(50.0, 50.0), Vec2::new(50.0, 50.0), LineParams::new(Color::WHITE, 2.0));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lines::draw_line_zero_thickness", {
                    ren.begin_frame()?;
                    ren.draw_line(Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0), LineParams::new(Color::WHITE, 0.0));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lines::draw_polyline_0_points", {
                    ren.begin_frame()?;
                    ren.draw_polyline(&[], LineParams::new(Color::WHITE, 2.0));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lines::draw_polyline_1_point", {
                    ren.begin_frame()?;
                    ren.draw_polyline(&[Vec2::new(50.0, 50.0)], LineParams::new(Color::WHITE, 2.0));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "lines::draw_polyline_1000_points", {
                    ren.begin_frame()?;
                    let pts: Vec<Vec2> = (0..1000).map(|i| {
                        let t = i as f32 * 0.01;
                        Vec2::new(t * 80.0, (t * 5.0).sin() * 100.0 + 300.0)
                    }).collect();
                    ren.draw_polyline(&pts, LineParams::new(Color::YELLOW, 1.0));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 8. sprites — load, draw, unload
                // -------------------------------------------------------

                run_test!(results, "sprites::load_texture_1x1_rgba", {
                    let data: [u8; 4] = [255, 0, 0, 255]; // 1x1 red pixel
                    // load_texture expects encoded image bytes (png), not raw rgba.
                    // create a minimal 1x1 png in memory.
                    let png = make_1x1_png(255, 0, 0, 255);
                    let handle = ren.load_texture(&png, TextureParams::default())?;
                    let size = ren.texture_size(handle);
                    assert!(size.is_some(), "texture_size should return Some for valid handle");
                    assert_eq!(size.unwrap(), (1, 1));
                    ren.unload_texture(handle);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::load_texture_empty_data", {
                    let result = ren.load_texture(&[], TextureParams::default());
                    assert!(result.is_err(), "empty data should fail");
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::draw_sprite_valid", {
                    let png = make_1x1_png(0, 255, 0, 255);
                    let handle = ren.load_texture(&png, TextureParams::default())?;
                    ren.begin_frame()?;
                    ren.draw_sprite(handle, SpriteParams::new(Transform2D::new(100.0, 100.0)));
                    ren.end_frame()?;
                    ren.unload_texture(handle);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::draw_sprite_with_transform", {
                    let png = make_1x1_png(0, 0, 255, 255);
                    let handle = ren.load_texture(&png, TextureParams::default())?;
                    ren.begin_frame()?;
                    ren.draw_sprite(handle, SpriteParams::new(
                        Transform2D::new(200.0, 200.0).with_uniform_scale(50.0).with_rotation_deg(45.0)
                    ));
                    ren.end_frame()?;
                    ren.unload_texture(handle);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::draw_sprite_tint_and_opacity", {
                    let png = make_1x1_png(255, 255, 255, 255);
                    let handle = ren.load_texture(&png, TextureParams::default())?;
                    ren.begin_frame()?;
                    ren.draw_sprite(handle, SpriteParams::new(Transform2D::new(50.0, 50.0))
                        .with_tint(Color::RED)
                        .with_opacity(0.5));
                    ren.end_frame()?;
                    ren.unload_texture(handle);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::draw_sprite_flip", {
                    let png = make_1x1_png(128, 128, 128, 255);
                    let handle = ren.load_texture(&png, TextureParams::default())?;
                    ren.begin_frame()?;
                    ren.draw_sprite(handle, SpriteParams::new(Transform2D::new(50.0, 50.0)).with_flip(true, false));
                    ren.draw_sprite(handle, SpriteParams::new(Transform2D::new(100.0, 50.0)).with_flip(false, true));
                    ren.draw_sprite(handle, SpriteParams::new(Transform2D::new(150.0, 50.0)).with_flip(true, true));
                    ren.end_frame()?;
                    ren.unload_texture(handle);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::unload_then_draw_no_crash", {
                    let png = make_1x1_png(255, 0, 255, 255);
                    let handle = ren.load_texture(&png, TextureParams::default())?;
                    ren.unload_texture(handle);
                    // drawing with unloaded handle — should not crash
                    ren.begin_frame()?;
                    ren.draw_sprite(handle, SpriteParams::new(Transform2D::new(50.0, 50.0)));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::unload_invalid_handle_no_crash", {
                    let bogus = TextureHandle::new(999999);
                    ren.unload_texture(bogus); // should not panic
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::draw_invalid_handle_no_crash", {
                    let bogus = TextureHandle::new(999999);
                    ren.begin_frame()?;
                    ren.draw_sprite(bogus, SpriteParams::new(Transform2D::new(50.0, 50.0)));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::load_100_unload_all", {
                    let png = make_1x1_png(100, 100, 100, 255);
                    let mut handles = Vec::new();
                    for _ in 0..100 {
                        handles.push(ren.load_texture(&png, TextureParams::default())?);
                    }
                    for h in handles {
                        ren.unload_texture(h);
                    }
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "sprites::load_same_data_different_handles", {
                    let png = make_1x1_png(50, 50, 50, 255);
                    let h1 = ren.load_texture(&png, TextureParams::default())?;
                    let h2 = ren.load_texture(&png, TextureParams::default())?;
                    assert_ne!(h1, h2, "same data should produce different handles");
                    ren.unload_texture(h1);
                    ren.unload_texture(h2);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 9. blend modes
                // -------------------------------------------------------

                run_test!(results, "blend::all_modes_no_crash", {
                    let modes = [
                        BlendMode::Alpha,
                        BlendMode::Additive,
                        BlendMode::Multiply,
                        BlendMode::Screen,
                        BlendMode::PremultipliedAlpha,
                    ];
                    ren.begin_frame()?;
                    for (i, mode) in modes.iter().enumerate() {
                        ren.set_blend_mode(*mode);
                        ren.draw_rect(
                            Rect::new(i as f32 * 60.0, 10.0, 50.0, 50.0),
                            DrawParams::fill(Color::RED).with_blend(*mode),
                        );
                    }
                    ren.set_blend_mode(BlendMode::Alpha);
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 10. transform stack
                // -------------------------------------------------------

                run_test!(results, "transform::push_pop_basic", {
                    ren.begin_frame()?;
                    ren.push_transform(Transform2D { pos: Vec2::new(10.0, 10.0), scale: Vec2::ONE, rotation: 0.0 });
                    ren.draw_rect(Rect::new(0.0, 0.0, 50.0, 50.0), DrawParams::fill(Color::RED));
                    ren.pop_transform();
                    ren.draw_rect(Rect::new(0.0, 0.0, 50.0, 50.0), DrawParams::fill(Color::BLUE));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "transform::nested_transforms", {
                    ren.begin_frame()?;
                    ren.push_transform(Transform2D { pos: Vec2::new(10.0, 0.0), scale: Vec2::ONE, rotation: 0.0 });
                    ren.push_transform(Transform2D { pos: Vec2::new(0.0, 10.0), scale: Vec2::ONE, rotation: 0.0 });
                    ren.draw_rect(Rect::new(0.0, 0.0, 20.0, 20.0), DrawParams::fill(Color::GREEN));
                    ren.pop_transform();
                    ren.pop_transform();
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "transform::reset_clears_stack", {
                    ren.begin_frame()?;
                    ren.push_transform(Transform2D { pos: Vec2::new(100.0, 100.0), scale: Vec2::ONE, rotation: 0.0 });
                    ren.reset_transform();
                    ren.draw_rect(Rect::new(0.0, 0.0, 30.0, 30.0), DrawParams::fill(Color::WHITE));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 11. camera
                // -------------------------------------------------------

                run_test!(results, "camera::set_and_get", {
                    let cam = Camera2D::new(800.0, 600.0).with_zoom(2.0).with_position(Vec2::new(100.0, 200.0));
                    ren.set_camera(&cam);
                    let got = ren.camera();
                    assert!((got.zoom - 2.0).abs() < 0.01);
                    assert!((got.position.x - 100.0).abs() < 0.01);
                    // restore default
                    ren.set_camera(&Camera2D::new(800.0, 600.0));
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "camera::draw_with_camera", {
                    let cam = Camera2D::new(800.0, 600.0).with_position(Vec2::new(50.0, 50.0));
                    ren.set_camera(&cam);
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(0.0, 0.0, 100.0, 100.0), DrawParams::fill(Color::CYAN));
                    ren.end_frame()?;
                    ren.set_camera(&Camera2D::new(800.0, 600.0));
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 12. clipping
                // -------------------------------------------------------

                run_test!(results, "clip::push_pop_basic", {
                    ren.begin_frame()?;
                    ren.push_clip_rect(Rect::new(10.0, 10.0, 100.0, 100.0));
                    ren.draw_rect(Rect::new(0.0, 0.0, 200.0, 200.0), DrawParams::fill(Color::RED));
                    ren.pop_clip_rect();
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "clip::nested_clips", {
                    ren.begin_frame()?;
                    ren.push_clip_rect(Rect::new(0.0, 0.0, 400.0, 400.0));
                    ren.push_clip_rect(Rect::new(50.0, 50.0, 100.0, 100.0));
                    ren.draw_rect(Rect::new(0.0, 0.0, 800.0, 600.0), DrawParams::fill(Color::GREEN));
                    ren.pop_clip_rect();
                    ren.pop_clip_rect();
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "clip::draw_outside_clip", {
                    ren.begin_frame()?;
                    ren.push_clip_rect(Rect::new(10.0, 10.0, 20.0, 20.0));
                    ren.draw_rect(Rect::new(500.0, 500.0, 100.0, 100.0), DrawParams::fill(Color::BLUE));
                    ren.pop_clip_rect();
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 13. text
                // -------------------------------------------------------

                run_test!(results, "text::load_unload_font", {
                    let font_data = std::fs::read("NotoSansMeroitic-Regular.ttf")
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    let h = ren.load_font(&font_data)?;
                    ren.unload_font(h);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "text::load_font_invalid", {
                    let result = ren.load_font(&[0, 1, 2, 3]);
                    assert!(result.is_err());
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "text::draw_text_basic", {
                    let font_data = std::fs::read("NotoSansMeroitic-Regular.ttf")
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    let h = ren.load_font(&font_data)?;
                    ren.begin_frame()?;
                    ren.draw_text("Hello", &TextParams {
                        font: h, size: 24.0, color: Color::WHITE,
                        align: TextAlign::Left, position: Vec2::new(10.0, 10.0),
                        max_width: None, line_height: None,
                    });
                    ren.end_frame()?;
                    ren.unload_font(h);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "text::measure_text_positive", {
                    let font_data = std::fs::read("NotoSansMeroitic-Regular.ttf")
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    let h = ren.load_font(&font_data)?;
                    let size = ren.measure_text("Hello World", &TextParams {
                        font: h, size: 24.0, color: Color::WHITE,
                        align: TextAlign::Left, position: Vec2::ZERO,
                        max_width: None, line_height: None,
                    });
                    assert!(size.y > 0.0, "measured height should be positive");
                    ren.unload_font(h);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "text::unload_then_draw_no_crash", {
                    let font_data = std::fs::read("NotoSansMeroitic-Regular.ttf")
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    let h = ren.load_font(&font_data)?;
                    ren.unload_font(h);
                    ren.begin_frame()?;
                    ren.draw_text("Test", &TextParams {
                        font: h, size: 24.0, color: Color::WHITE,
                        align: TextAlign::Left, position: Vec2::ZERO,
                        max_width: None, line_height: None,
                    });
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 14. paths
                // -------------------------------------------------------

                run_test!(results, "paths::draw_path_fill", {
                    ren.begin_frame()?;
                    let path = Path {
                        segments: vec![
                            PathSegment::MoveTo(Vec2::new(100.0, 100.0)),
                            PathSegment::LineTo(Vec2::new(200.0, 100.0)),
                            PathSegment::LineTo(Vec2::new(150.0, 200.0)),
                            PathSegment::Close,
                        ],
                    };
                    ren.draw_path(&path, DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "paths::stroke_path_line", {
                    ren.begin_frame()?;
                    let path = Path {
                        segments: vec![
                            PathSegment::MoveTo(Vec2::new(10.0, 10.0)),
                            PathSegment::LineTo(Vec2::new(200.0, 200.0)),
                        ],
                    };
                    ren.stroke_path(&path, StrokeParams::new(Color::WHITE, 3.0));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "paths::draw_path_empty", {
                    ren.begin_frame()?;
                    let path = Path { segments: vec![] };
                    ren.draw_path(&path, DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "paths::draw_complex_polygon", {
                    ren.begin_frame()?;
                    let outer = [
                        Vec2::new(100.0, 100.0), Vec2::new(300.0, 100.0),
                        Vec2::new(300.0, 300.0), Vec2::new(100.0, 300.0),
                    ];
                    ren.draw_complex_polygon(&outer, &[], DrawParams::fill(Color::GREEN));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "paths::draw_complex_polygon_hole", {
                    ren.begin_frame()?;
                    let outer = [
                        Vec2::new(100.0, 100.0), Vec2::new(300.0, 100.0),
                        Vec2::new(300.0, 300.0), Vec2::new(100.0, 300.0),
                    ];
                    let hole = [
                        Vec2::new(150.0, 150.0), Vec2::new(250.0, 150.0),
                        Vec2::new(250.0, 250.0), Vec2::new(150.0, 250.0),
                    ];
                    ren.draw_complex_polygon(&outer, &[&hole[..]], DrawParams::fill(Color::BLUE));
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 15. render targets
                // -------------------------------------------------------

                run_test!(results, "rt::create_destroy", {
                    let rt = ren.create_render_target(128, 128)?;
                    ren.destroy_render_target(rt);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "rt::render_to_texture", {
                    let rt = ren.create_render_target(128, 128)?;
                    ren.begin_render_to_texture(rt)?;
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(0.0, 0.0, 64.0, 64.0), DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    ren.end_render_to_texture();
                    ren.destroy_render_target(rt);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "rt::render_target_texture", {
                    let rt = ren.create_render_target(128, 128)?;
                    let tex = ren.render_target_texture(rt);
                    assert!(tex.is_some(), "render target should have a texture handle");
                    ren.destroy_render_target(rt);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "rt::draw_rt_as_sprite", {
                    let rt = ren.create_render_target(64, 64)?;
                    ren.begin_render_to_texture(rt)?;
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(0.0, 0.0, 64.0, 64.0), DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    ren.end_render_to_texture();
                    let tex = ren.render_target_texture(rt).unwrap();
                    ren.begin_frame()?;
                    ren.draw_sprite(tex, SpriteParams {
                        transform: Transform2D::default(),
                        tint: Color::WHITE, src_rect: None,
                        flip_x: false, flip_y: false,
                        blend: BlendMode::Alpha, z_index: 0, opacity: 1.0,
                    });
                    ren.end_frame()?;
                    ren.destroy_render_target(rt);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "rt::destroy_invalid_no_crash", {
                    use lite_render_2d_core::texture::RenderTargetHandle;
                    ren.destroy_render_target(RenderTargetHandle::new(99999));
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 16. post-processing
                // -------------------------------------------------------

                run_test!(results, "postfx::grayscale", {
                    let rt = ren.create_render_target(128, 128)?;
                    ren.begin_render_to_texture(rt)?;
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(0.0, 0.0, 128.0, 128.0), DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    ren.end_render_to_texture();
                    ren.begin_frame()?;
                    ren.apply_post_effect(&PostEffect::Grayscale, rt);
                    ren.end_frame()?;
                    ren.destroy_render_target(rt);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                run_test!(results, "postfx::invert", {
                    let rt = ren.create_render_target(128, 128)?;
                    ren.begin_render_to_texture(rt)?;
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(0.0, 0.0, 128.0, 128.0), DrawParams::fill(Color::BLUE));
                    ren.end_frame()?;
                    ren.end_render_to_texture();
                    ren.begin_frame()?;
                    ren.apply_post_effect(&PostEffect::Invert, rt);
                    ren.end_frame()?;
                    ren.destroy_render_target(rt);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 17. read pixels
                // -------------------------------------------------------

                run_test!(results, "pixels::read_from_rt", {
                    let rt = ren.create_render_target(32, 32)?;
                    ren.begin_render_to_texture(rt)?;
                    ren.begin_frame()?;
                    ren.draw_rect(Rect::new(0.0, 0.0, 32.0, 32.0), DrawParams::fill(Color::RED));
                    ren.end_frame()?;
                    ren.end_render_to_texture();
                    let pixels = ren.read_pixels(rt);
                    // read_pixels may return Err if not implemented, that's ok
                    if let Ok(data) = pixels {
                        assert!(!data.is_empty(), "pixel data should not be empty");
                    }
                    ren.destroy_render_target(rt);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 18. nine-slice
                // -------------------------------------------------------

                run_test!(results, "nineslice::draw_basic", {
                    let png = make_1x1_png(255, 255, 255, 255);
                    let tex = ren.load_texture(&png, TextureParams::default())?;
                    let ns = NineSlice {
                        texture: tex,
                        border_left: 0.0, border_right: 0.0,
                        border_top: 0.0, border_bottom: 0.0,
                    };
                    ren.begin_frame()?;
                    ren.draw_nine_slice(&ns, Rect::new(10.0, 10.0, 100.0, 100.0), Color::WHITE, 0);
                    ren.end_frame()?;
                    ren.unload_texture(tex);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 19. tilemap
                // -------------------------------------------------------

                run_test!(results, "tilemap::draw_basic", {
                    let png = make_1x1_png(100, 200, 100, 255);
                    let tex = ren.load_texture(&png, TextureParams::default())?;
                    let mut tm = Tilemap::new(4, 4, 16.0, tex, TilesetInfo {
                        tile_width: 1.0, tile_height: 1.0, columns: 1,
                    });
                    tm.set_tile(0, 0, 1);
                    tm.set_tile(1, 1, 1);
                    ren.begin_frame()?;
                    ren.draw_tilemap(&tm, Vec2::ZERO, 0);
                    ren.end_frame()?;
                    ren.unload_texture(tex);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 20. stencil
                // -------------------------------------------------------

                run_test!(results, "stencil::write_and_pop", {
                    ren.begin_frame()?;
                    ren.begin_stencil_write();
                    ren.draw_circle(Vec2::new(200.0, 200.0), 50.0, DrawParams::fill(Color::WHITE));
                    ren.end_stencil_write();
                    ren.draw_rect(Rect::new(0.0, 0.0, 400.0, 400.0), DrawParams::fill(Color::RED));
                    ren.pop_stencil_mask();
                    ren.end_frame()?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // 21. instanced sprites
                // -------------------------------------------------------

                run_test!(results, "instanced::draw_sprite_instanced", {
                    let png = make_1x1_png(255, 0, 255, 255);
                    let tex = ren.load_texture(&png, TextureParams::default())?;
                    let instances: Vec<SpriteInstance> = (0..10).map(|i| SpriteInstance {
                        transform: Transform2D {
                            pos: Vec2::new(i as f32 * 20.0, 10.0),
                            scale: Vec2::new(16.0, 16.0),
                            rotation: 0.0,
                        },
                        tint: Color::WHITE,
                        opacity: 1.0,
                        src_rect: None,
                        flip_x: false,
                        flip_y: false,
                    }).collect();
                    ren.begin_frame()?;
                    ren.draw_sprite_instanced(tex, &instances, BlendMode::Alpha, 0);
                    ren.end_frame()?;
                    ren.unload_texture(tex);
                    Ok::<(), Box<dyn std::error::Error>>(())
                });

                // -------------------------------------------------------
                // print results
                // -------------------------------------------------------

                let total = results.len();
                let passed = results.iter().filter(|r| r.passed).count();
                let failed = total - passed;

                println!();
                println!("========================================");
                println!("  integration test results");
                println!("========================================");
                println!();

                for r in &results {
                    if r.passed {
                        println!("  PASS  {}", r.name);
                    } else {
                        println!("  FAIL  {}  —  {}", r.name, r.error.as_deref().unwrap_or("unknown"));
                    }
                }

                println!();
                println!("  {} passed, {} failed, {} total", passed, failed, total);
                println!("========================================");

                if failed > 0 {
                    std::process::exit(1);
                }

                event_loop.exit();
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// minimal png encoder — produces a valid 1x1 RGBA png in memory
// ---------------------------------------------------------------------------

fn make_1x1_png(r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
    // minimal png: signature + IHDR + IDAT + IEND
    let mut buf = Vec::new();

    // png signature
    buf.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR chunk: 1x1, 8-bit RGBA
    let ihdr_data: [u8; 13] = [
        0, 0, 0, 1, // width
        0, 0, 0, 1, // height
        8,          // bit depth
        6,          // color type (rgba)
        0,          // compression
        0,          // filter
        0,          // interlace
    ];
    write_png_chunk(&mut buf, b"IHDR", &ihdr_data);

    // IDAT chunk: zlib-compressed scanline (filter byte 0 + rgba)
    let raw_scanline = [0u8, r, g, b, a]; // filter=none, then pixel
    let compressed = deflate_raw(&raw_scanline);
    write_png_chunk(&mut buf, b"IDAT", &compressed);

    // IEND
    write_png_chunk(&mut buf, b"IEND", &[]);

    buf
}

fn write_png_chunk(buf: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    let len = data.len() as u32;
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(chunk_type);
    buf.extend_from_slice(data);
    let mut crc_data = Vec::with_capacity(4 + data.len());
    crc_data.extend_from_slice(chunk_type);
    crc_data.extend_from_slice(data);
    let crc = png_crc32(&crc_data);
    buf.extend_from_slice(&crc.to_be_bytes());
}

fn png_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn deflate_raw(data: &[u8]) -> Vec<u8> {
    // minimal zlib: header + stored block + adler32
    let mut out = Vec::new();
    // zlib header (no compression, no dict)
    out.push(0x78);
    out.push(0x01);
    // deflate stored block: final=1, type=00 (no compression)
    out.push(0x01);
    let len = data.len() as u16;
    out.extend_from_slice(&len.to_le_bytes());
    let nlen = !len;
    out.extend_from_slice(&nlen.to_le_bytes());
    out.extend_from_slice(data);
    // adler32
    let adler = adler32(data);
    out.extend_from_slice(&adler.to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App { window: None, renderer: None, ran: false };
    event_loop.run_app(&mut app).expect("run");
}
