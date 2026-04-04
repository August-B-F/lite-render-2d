//! Renders a feature showcase animation and saves frames as a GIF.
//! Uses render target + read_pixels to capture, then Pillow to stitch.
//!
//! Run: cargo run -p lite-render-2d-glow --example gif_capture --release

use lite_render_2d_core::prelude::*;
use lite_render_2d_core::types::StrokeParams;
use lite_render_2d_core::{ParticleConfig, ParticleSystem, TrailRenderer};
#[cfg(feature = "use-glow")]
use lite_render_2d_glow::GlowRenderer as Renderer2D;
#[cfg(feature = "use-wgpu")]
use lite_render_2d_wgpu::WgpuRenderer as Renderer2D;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const W: u32 = 640;
const H: u32 = 400;
const TOTAL_FRAMES: u32 = 120;
const FRAME_DT: f32 = 1.0 / 30.0;

fn make_png(w: u32, h: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for i in 0..(w * h) as usize {
        rgba[i * 4] = r; rgba[i * 4 + 1] = g; rgba[i * 4 + 2] = b; rgba[i * 4 + 3] = 255;
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    let enc = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(enc, &rgba, w, h, image::ColorType::Rgba8.into()).unwrap();
    buf.into_inner()
}

struct App {
    window: Option<Window>,
    renderer: Option<Renderer2D>,
    texture: Option<TextureHandle>,
    font: Option<FontHandle>,
    particles: ParticleSystem,
    trail: TrailRenderer,
    frame: u32,
    frames_data: Vec<Vec<u8>>,
}

fn render_scene(
    ren: &mut Renderer2D,
    tex: TextureHandle,
    font: Option<FontHandle>,
    t: f32,
    particles: &mut ParticleSystem,
    trail: &mut TrailRenderer,
) {
    let w = W as f32;
    let h = H as f32;

    // dark background
    ren.draw_rect(Rect::new(0.0, 0.0, w, h), DrawParams::fill(Color::new(0.06, 0.06, 0.10, 1.0)));

    // --- top bar with title ---
    ren.draw_rect(Rect::new(0.0, 0.0, w, 32.0), DrawParams::fill(Color::new(0.10, 0.10, 0.16, 1.0)));
    if let Some(f) = font {
        ren.draw_text("lite-render-2d", &TextParams {
            font: f, size: 18.0, color: Color::new(0.9, 0.9, 0.95, 1.0),
            align: TextAlign::Left, position: Vec2::new(12.0, 7.0),
            max_width: None, line_height: None,
        });
        ren.draw_text("feature showcase", &TextParams {
            font: f, size: 12.0, color: Color::new(0.5, 0.5, 0.6, 1.0),
            align: TextAlign::Left, position: Vec2::new(200.0, 11.0),
            max_width: None, line_height: None,
        });
    }

    let top = 38.0;
    let panel_w = (w - 20.0) / 3.0;
    let panel_h = (h - top - 70.0) / 2.0;

    // panel backgrounds
    for row in 0..2 {
        for col in 0..3 {
            let px = 5.0 + col as f32 * (panel_w + 5.0);
            let py = top + row as f32 * (panel_h + 5.0);
            ren.draw_rect(Rect::new(px, py, panel_w, panel_h), DrawParams::fill(Color::new(0.10, 0.10, 0.15, 0.8)));
        }
    }

    // --- panel 1: gradient shapes ---
    let p1x = 8.0;
    let p1y = top + 3.0;
    ren.draw_rect(
        Rect::new(p1x, p1y + 15.0, panel_w - 10.0, panel_h - 25.0),
        DrawParams {
            style: DrawStyle::LinearGradient {
                start: Vec2::new(p1x, p1y + 15.0),
                end: Vec2::new(p1x + panel_w - 10.0, p1y + panel_h - 10.0),
                color_start: Color::new(0.15, 0.3, 0.9, 1.0),
                color_end: Color::new(0.7, 0.15, 0.7, 1.0),
            },
            blend: BlendMode::Alpha, z_index: 0, opacity: 1.0,
        },
    );
    // animated circle bouncing inside
    let cx = p1x + panel_w * 0.5 + (t * 2.5).sin() * (panel_w * 0.3);
    let cy = p1y + panel_h * 0.55 + (t * 3.5).cos() * (panel_h * 0.2);
    ren.draw_circle(Vec2::new(cx, cy), 16.0 + (t * 4.0).sin() * 4.0,
        DrawParams::fill(Color::new(1.0, 0.85, 0.2, 0.9)));
    // stroke outline
    ren.draw_rect(
        Rect::new(p1x, p1y + 15.0, panel_w - 10.0, panel_h - 25.0),
        DrawParams { style: DrawStyle::Stroke(StrokeParams::new(Color::new(1.0, 1.0, 1.0, 0.3), 1.0)),
            blend: BlendMode::Alpha, z_index: 1, opacity: 1.0 },
    );

    // --- panel 2: spinning sprites ---
    let p2x = 5.0 + panel_w + 5.0 + panel_w * 0.5;
    let p2y = top + panel_h * 0.5 + 3.0;
    let sprite_count = 14;
    for i in 0..sprite_count {
        let angle = (i as f32 / sprite_count as f32) * std::f32::consts::TAU + t * 1.5;
        let r = 45.0 + (t * 2.0 + i as f32 * 0.5).sin() * 8.0;
        let sx = p2x + angle.cos() * r;
        let sy = p2y + angle.sin() * r;
        let scale = 0.7 + (t * 3.0 + i as f32).sin() * 0.25;
        ren.draw_sprite(tex, SpriteParams {
            transform: Transform2D {
                pos: Vec2::new(sx - 8.0 * scale, sy - 8.0 * scale),
                scale: Vec2::new(scale, scale),
                rotation: angle + t,
            },
            tint: Color::new(
                0.5 + (i as f32 * 0.25).sin() * 0.5,
                0.5 + (i as f32 * 0.4 + 1.0).sin() * 0.5,
                0.5 + (i as f32 * 0.6 + 2.0).sin() * 0.5,
                1.0,
            ),
            src_rect: None, flip_x: false, flip_y: false,
            blend: BlendMode::Alpha, z_index: 0, opacity: 0.9,
        });
    }

    // --- panel 3: star polygon ---
    let p3x = 5.0 + 2.0 * (panel_w + 5.0) + panel_w * 0.5;
    let p3y = top + panel_h * 0.55;
    let star_r = 40.0 + (t * 2.0).sin() * 6.0;
    let outer: Vec<Vec2> = (0..10).map(|i| {
        let angle = std::f32::consts::TAU * i as f32 / 10.0 - std::f32::consts::FRAC_PI_2 + t * 0.4;
        let r = if i % 2 == 0 { star_r } else { star_r * 0.42 };
        Vec2::new(p3x + angle.cos() * r, p3y + angle.sin() * r)
    }).collect();
    ren.draw_complex_polygon(&outer, &[], DrawParams::fill(Color::new(1.0, 0.25, 0.35, 0.9)));

    // --- panel 4: trail ---
    let p4x = 8.0 + panel_w * 0.5;
    let p4y = top + panel_h + 8.0 + panel_h * 0.5;
    let trail_x = p4x + (t * 1.8).cos() * (panel_w * 0.35);
    let trail_y = p4y + (t * 2.8).sin() * (panel_h * 0.3);
    trail.add_point(Vec2::new(trail_x, trail_y));
    trail.update(FRAME_DT);
    trail.draw(ren);

    // --- panel 5: particles ---
    particles.update(FRAME_DT);
    particles.draw(ren);

    // --- panel 6: polyline wave + circles ---
    let p6x = 5.0 + 2.0 * (panel_w + 5.0) + 8.0;
    let p6y = top + panel_h + 8.0;
    let n_pts = 35;
    let points: Vec<Vec2> = (0..n_pts).map(|i| {
        let px = p6x + i as f32 * ((panel_w - 16.0) / n_pts as f32);
        let py = p6y + panel_h * 0.35 + ((i as f32 * 0.35 + t * 3.5).sin() * panel_h * 0.25);
        Vec2::new(px, py)
    }).collect();
    ren.draw_polyline(&points, LineParams::new(Color::new(0.3, 1.0, 0.5, 0.8), 2.5));
    for i in 0..6 {
        let px = p6x + 10.0 + i as f32 * ((panel_w - 30.0) / 5.0);
        let py = p6y + panel_h * 0.7 + (t * 2.5 + i as f32 * 0.8).sin() * 12.0;
        ren.draw_circle(Vec2::new(px, py), 7.0, DrawParams::fill(Color::new(0.3, 0.6, 1.0, 0.75)));
    }

    // --- panel labels ---
    if let Some(f) = font {
        let labels = ["gradients", "sprites", "polygons", "trails", "particles", "lines"];
        for (idx, label) in labels.iter().enumerate() {
            let col = idx % 3;
            let row = idx / 3;
            let lx = 10.0 + col as f32 * (panel_w + 5.0);
            let ly = top + 3.0 + row as f32 * (panel_h + 5.0);
            ren.draw_text(label, &TextParams {
                font: f, size: 10.0, color: Color::new(0.6, 0.6, 0.7, 0.8),
                align: TextAlign::Left, position: Vec2::new(lx, ly),
                max_width: None, line_height: None,
            });
        }
    }

    // --- bottom: Dune quote ---
    ren.draw_rect(Rect::new(0.0, h - 38.0, w, 38.0), DrawParams::fill(Color::new(0.05, 0.05, 0.08, 0.95)));
    if let Some(f) = font {
        // animated quote opacity
        let alpha = 0.6 + (t * 0.8).sin() * 0.3;
        ren.draw_text("I must not fear. Fear is the mind-killer.", &TextParams {
            font: f, size: 14.0, color: Color::new(0.85, 0.75, 0.5, alpha),
            align: TextAlign::Center, position: Vec2::new(0.0, h - 32.0),
            max_width: Some(w), line_height: None,
        });
        ren.draw_text("- Frank Herbert, Dune", &TextParams {
            font: f, size: 10.0, color: Color::new(0.5, 0.45, 0.35, alpha * 0.7),
            align: TextAlign::Center, position: Vec2::new(0.0, h - 14.0),
            max_width: Some(w), line_height: None,
        });
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let win = event_loop
            .create_window(WindowAttributes::default()
                .with_title("gif capture")
                .with_inner_size(winit::dpi::LogicalSize::new(W, H)))
            .unwrap();
        let mut ren = Renderer2D::new(&win).unwrap();
        ren.set_clear_color(Color::new(0.06, 0.06, 0.10, 1.0));
        let tex = ren.load_texture(&make_png(16, 16, 200, 120, 255), TextureParams::default()).unwrap();
        // load font for labels
        let font = match std::fs::read("NotoSansMeroitic-Regular.ttf") {
            Ok(data) => ren.load_font(&data).ok(),
            Err(_) => None,
        };
        self.texture = Some(tex);
        self.font = font;
        self.renderer = Some(ren);
        self.window = Some(win);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if self.frame >= TOTAL_FRAMES {
                    let out_dir = "gif_frames";
                    std::fs::create_dir_all(out_dir).unwrap();
                    for (i, data) in self.frames_data.iter().enumerate() {
                        let path = format!("{}/frame_{:04}.rgba", out_dir, i);
                        std::fs::write(&path, data).unwrap();
                    }
                    eprintln!("wrote {} frames to {}/", self.frames_data.len(), out_dir);

                    let py_script = format!(
"from PIL import Image\nimport glob\nframes = sorted(glob.glob('{}/frame_*.rgba'))\nimages = []\nfor f in frames:\n    data = open(f, 'rb').read()\n    img = Image.frombytes('RGBA', ({}, {}), data)\n    img = img.convert('RGB')\n    images.append(img)\nif images:\n    images[0].save('showcase.gif', save_all=True, append_images=images[1:], duration=33, loop=0, optimize=True)\n    print('saved showcase.gif (' + str(len(images)) + ' frames)')\n",
                        out_dir, W, H);
                    let status = std::process::Command::new("python3")
                        .arg("-c")
                        .arg(&py_script)
                        .status();

                    match status {
                        Ok(s) if s.success() => eprintln!("GIF created: showcase.gif"),
                        Ok(s) => eprintln!("Python exited with: {}", s),
                        Err(e) => eprintln!("failed to run python3: {}", e),
                    }

                    let _ = std::fs::remove_dir_all(out_dir);
                    event_loop.exit();
                    return;
                }

                let ren = self.renderer.as_mut().unwrap();
                let tex = self.texture.unwrap();
                let font = self.font;
                let t = self.frame as f32 * FRAME_DT;

                // render to render target for pixel readback
                let rt = ren.create_render_target(W, H).unwrap();
                ren.begin_render_to_texture(rt).unwrap();
                ren.begin_frame().unwrap();
                render_scene(ren, tex, font, t, &mut self.particles, &mut self.trail);
                ren.end_frame().unwrap();
                ren.end_render_to_texture();

                // read pixels — read_pixels returns data that may need flipping
                if let Ok(pixels) = ren.read_pixels(rt) {
                    // just use the raw pixels directly (driver returns correct orientation)
                    self.frames_data.push(pixels);
                }

                // also draw to screen
                ren.begin_frame().unwrap();
                render_scene(ren, tex, font, t, &mut self.particles, &mut self.trail);
                ren.end_frame().unwrap();

                ren.destroy_render_target(rt);

                self.frame += 1;
                if self.frame % 30 == 0 {
                    eprint!("  frame {}/{}\n", self.frame, TOTAL_FRAMES);
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
    eprintln!("capturing {} frames at {}x{} ...", TOTAL_FRAMES, W, H);
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut particles = ParticleSystem::new();
    let panel_w = (W as f32 - 20.0) / 3.0;
    let panel_h = (H as f32 - 38.0 - 70.0) / 2.0;
    particles.add_emitter(
        ParticleConfig {
            spawn_rate: 30.0,
            lifetime: (0.5, 1.5),
            velocity: (Vec2::new(-25.0, -55.0), Vec2::new(25.0, -10.0)),
            size: (5.0, 1.0),
            color_start: Color::new(1.0, 0.5, 0.1, 1.0),
            color_end: Color::new(1.0, 0.1, 0.0, 0.0),
            gravity: Vec2::new(0.0, 45.0),
            texture: None,
        },
        Vec2::new(5.0 + panel_w + 5.0 + panel_w * 0.5, 38.0 + panel_h + 8.0 + panel_h * 0.6),
    );
    let mut trail = TrailRenderer::new(80, 5.0, 1.2);
    trail.color_start = Color::new(1.0, 0.55, 0.1, 0.9);
    trail.color_end = Color::new(0.8, 0.15, 0.0, 0.0);

    let mut app = App {
        window: None, renderer: None, texture: None, font: None,
        particles, trail,
        frame: 0, frames_data: Vec::new(),
    };
    event_loop.run_app(&mut app).unwrap();
}
