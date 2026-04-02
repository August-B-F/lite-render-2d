use std::collections::HashSet;
use std::time::Instant;

use lite_render_2d_core::{
    Camera2D, Color, DrawParams, LineParams, Rect, Renderer, SpriteParams, TextureHandle,
    TextureParams, Transform2D, Vec2,
};
#[cfg(feature = "use-glow")]
use lite_render_2d_glow::GlowRenderer as Renderer2D;
#[cfg(feature = "use-wgpu")]
use lite_render_2d_wgpu::WgpuRenderer as Renderer2D;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

struct CliArgs {
    no_vsync: bool,
    count: u32,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut no_vsync = false;
    let mut count = 3000u32;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--no-vsync" => no_vsync = true,
            "--count" => {
                i += 1;
                if i < args.len() {
                    count = args[i].parse().unwrap_or(3000);
                }
            }
            _ => {
                // try --count=N form
                if args[i].starts_with("--count=") {
                    count = args[i][8..].parse().unwrap_or(3000);
                }
            }
        }
        i += 1;
    }
    CliArgs { no_vsync, count }
}

// 64x64 checkerboard
fn make_checkerboard_png() -> Vec<u8> {
    let (w, h) = (64u32, 64u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let checker = ((x / 8) + (y / 8)) % 2 == 0;
            if checker {
                rgba[i] = 220; rgba[i + 1] = 50; rgba[i + 2] = 200; rgba[i + 3] = 255;
            } else {
                rgba[i] = 255; rgba[i + 1] = 255; rgba[i + 2] = 255; rgba[i + 3] = 255;
            }
        }
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(encoder, &rgba, w, h, image::ColorType::Rgba8.into())
        .expect("encode png");
    buf.into_inner()
}

// 32x32 gradient
fn make_gradient_png() -> Vec<u8> {
    let (w, h) = (32u32, 32u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let u = x as f32 / w as f32;
            let v = y as f32 / h as f32;
            rgba[i] = (u * 255.0) as u8;
            rgba[i + 1] = (v * 200.0) as u8;
            rgba[i + 2] = 120;
            rgba[i + 3] = 255;
        }
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(encoder, &rgba, w, h, image::ColorType::Rgba8.into())
        .expect("encode png");
    buf.into_inner()
}

struct App {
    window: Option<Window>,
    renderer: Option<Renderer2D>,
    tex_a: Option<TextureHandle>,
    tex_b: Option<TextureHandle>,
    camera: Camera2D,
    keys_held: HashSet<KeyCode>,
    frame_count: u32,
    last_print: Instant,
    no_vsync: bool,
    count: u32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let title = format!(
            "mixed — {} objects{}",
            self.count,
            if self.no_vsync { " (vsync off)" } else { "" }
        );
        let attrs = WindowAttributes::default()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0));
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = Renderer2D::new_with_vsync(&win, !self.no_vsync).expect("create renderer");
        ren.set_clear_color(Color::new(0.1, 0.1, 0.15, 1.0));

        let tex_a = ren.load_texture(&make_checkerboard_png(), TextureParams::default()).expect("load tex a");
        let tex_b = ren.load_texture(&make_gradient_png(), TextureParams::default()).expect("load tex b");

        self.tex_a = Some(tex_a);
        self.tex_b = Some(tex_b);
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
                self.camera.viewport = Vec2::new(size.width as f32, size.height as f32);
                self.camera.position = Vec2::new(size.width as f32 / 2.0, size.height as f32 / 2.0);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => { self.keys_held.insert(code); }
                        ElementState::Released => { self.keys_held.remove(&code); }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
                };
                let factor = if scroll_y > 0.0 { 1.1 } else { 1.0 / 1.1 };
                self.camera.zoom = (self.camera.zoom * factor).clamp(0.1, 10.0);
            }
            WindowEvent::RedrawRequested => {
                let (Some(ren), Some(tex_a), Some(tex_b)) =
                    (&mut self.renderer, self.tex_a, self.tex_b)
                else {
                    return;
                };

                ren.begin_frame().expect("begin frame");

                // pan camera with arrow keys
                let pan_speed = 5.0 / self.camera.zoom;
                if self.keys_held.contains(&KeyCode::ArrowLeft) {
                    self.camera.position.x -= pan_speed;
                }
                if self.keys_held.contains(&KeyCode::ArrowRight) {
                    self.camera.position.x += pan_speed;
                }
                if self.keys_held.contains(&KeyCode::ArrowUp) {
                    self.camera.position.y -= pan_speed;
                }
                if self.keys_held.contains(&KeyCode::ArrowDown) {
                    self.camera.position.y += pan_speed;
                }

                ren.set_camera(&self.camera);

                let sw = 1024.0_f32;
                let sh = 768.0_f32;
                let n = self.count;

                // distribute objects: rects 1/6, circles 1/6, stroked 1/12, lines 1/12, spriteA 1/3, spriteB 1/6
                let n_rects = n / 6;
                let n_circles = n / 6;
                let n_stroked = n / 12;
                let n_lines = n / 12;
                let n_sprite_b = n / 6;
                let n_sprite_a = n - n_rects - n_circles - n_stroked - n_lines - n_sprite_b;

                // -- filled rects --
                for i in 0..n_rects {
                    let fi = i as f32;
                    let x = (fi * 137.5) % sw;
                    let y = (fi * 97.3) % sh;
                    let r = ((fi * 0.03).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                    let g = ((fi * 0.05).cos() * 0.5 + 0.5).clamp(0.0, 1.0);
                    ren.draw_rect(
                        Rect { pos: Vec2::new(x, y), size: Vec2::new(30.0, 20.0) },
                        DrawParams::fill(Color::new(r, g, 0.4, 0.8)),
                    );
                }

                // -- filled circles --
                for i in 0..n_circles {
                    let fi = i as f32;
                    let x = (fi * 103.7) % sw;
                    let y = (fi * 89.1) % sh;
                    let b = ((fi * 0.07).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                    ren.draw_circle(
                        Vec2::new(x, y),
                        8.0 + (fi % 12.0),
                        DrawParams::fill(Color::new(0.3, 0.5, b, 0.7)),
                    );
                }

                // -- stroked rects --
                for i in 0..n_stroked {
                    let fi = i as f32;
                    let x = (fi * 151.3) % sw;
                    let y = (fi * 113.7) % sh;
                    ren.draw_rect(
                        Rect { pos: Vec2::new(x, y), size: Vec2::new(40.0, 30.0) },
                        DrawParams::stroke(Color::new(1.0, 0.8, 0.2, 0.9), 2.0),
                    );
                }

                // -- lines --
                for i in 0..n_lines {
                    let fi = i as f32;
                    let x0 = (fi * 127.1) % sw;
                    let y0 = (fi * 83.9) % sh;
                    let x1 = ((fi + 1.0) * 137.5) % sw;
                    let y1 = ((fi + 1.0) * 97.3) % sh;
                    let r = ((fi * 0.1).cos() * 0.5 + 0.5).clamp(0.0, 1.0);
                    ren.draw_line(
                        Vec2::new(x0, y0),
                        Vec2::new(x1, y1),
                        LineParams::new(Color::new(r, 0.6, 1.0, 0.6), 2.0),
                    );
                }

                // -- sprites with texture A --
                for i in 0..n_sprite_a {
                    let fi = i as f32;
                    let x = (fi * 119.3) % sw;
                    let y = (fi * 79.7) % sh;
                    let rot = fi * 0.05;
                    let sc = 0.3 + (fi % 10.0) * 0.07;
                    let r = ((fi * 0.04).sin() * 0.3 + 0.7).clamp(0.0, 1.0);
                    ren.draw_sprite(
                        tex_a,
                        SpriteParams::new(Transform2D {
                            pos: Vec2::new(x, y),
                            rotation: rot,
                            scale: Vec2::new(sc, sc),
                        })
                        .with_tint(Color::new(r, 0.9, 1.0, 1.0))
                        .with_opacity(0.7 + (fi % 5.0) * 0.06),
                    );
                }

                // -- sprites with texture B --
                for i in 0..n_sprite_b {
                    let fi = i as f32;
                    let x = (fi * 143.1) % sw;
                    let y = (fi * 67.9) % sh;
                    let rot = -fi * 0.03;
                    let sc = 0.5 + (fi % 8.0) * 0.1;
                    ren.draw_sprite(
                        tex_b,
                        SpriteParams::new(Transform2D {
                            pos: Vec2::new(x, y),
                            rotation: rot,
                            scale: Vec2::new(sc, sc),
                        })
                        .with_opacity(0.6 + (fi % 4.0) * 0.1),
                    );
                }

                let stats = ren.end_frame().expect("end frame");

                // fps + draw call reporting
                self.frame_count += 1;
                if self.frame_count % 60 == 0 {
                    let elapsed = self.last_print.elapsed().as_secs_f64();
                    let fps = 60.0 / elapsed;
                    println!(
                        "fps: {:.1}  frame_ms: {:.2}  draws: {}  verts: {}  tex_binds: {}  ram: {}KB  objects: {}",
                        fps, stats.frame_time_ms, stats.draw_calls,
                        stats.vertices, stats.texture_binds, stats.ram_bytes / 1024, self.count,
                    );
                    self.last_print = Instant::now();
                }

                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(win) = &self.window {
            win.request_redraw();
        }
    }
}

fn main() {
    let cli = parse_args();
    let event_loop = EventLoop::new().expect("event loop");
    if cli.no_vsync {
        event_loop.set_control_flow(ControlFlow::Poll);
    }
    println!("count: {}  vsync: {}", cli.count, !cli.no_vsync);
    let mut app = App {
        window: None,
        renderer: None,
        tex_a: None,
        tex_b: None,
        camera: Camera2D::new(1024.0, 768.0),
        keys_held: HashSet::new(),
        frame_count: 0,
        last_print: Instant::now(),
        no_vsync: cli.no_vsync,
        count: cli.count,
    };
    app.camera.position = Vec2::new(512.0, 384.0);
    event_loop.run_app(&mut app).expect("run");
}
