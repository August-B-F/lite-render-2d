//! Performance metrics collector — runs several benchmarks, prints results, exits.
//! Tracks FPS, draw calls, frame time, and process RAM usage.
//! Outputs structured data suitable for README inclusion.
//!
//! Run: cargo run -p lite-render-2d-glow --example perf_metrics --release

use std::time::Instant;

use lite_render_2d_core::prelude::*;
use lite_render_2d_core::types::SpriteInstance;
#[cfg(feature = "use-glow")]
use lite_render_2d_glow::GlowRenderer as Renderer2D;
#[cfg(feature = "use-wgpu")]
use lite_render_2d_wgpu::WgpuRenderer as Renderer2D;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

fn make_png(w: u32, h: u32) -> Vec<u8> {
    let rgba = vec![200u8; (w * h * 4) as usize];
    let mut buf = std::io::Cursor::new(Vec::new());
    let enc = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(enc, &rgba, w, h, image::ColorType::Rgba8.into()).unwrap();
    buf.into_inner()
}

// read process RSS from /proc/self/status (Linux)
fn rss_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let kb: f64 = line.split_whitespace().nth(1)
                    .and_then(|s| s.parse().ok()).unwrap_or(0.0);
                return kb / 1024.0;
            }
        }
    }
    0.0
}

struct Metric {
    name: String,
    count: u32,
    avg_ms: f64,
    fps: f64,
    draw_calls: u32,
    ram_mb: f64,
}

struct App {
    window: Option<Window>,
    renderer: Option<Renderer2D>,
    texture: Option<TextureHandle>,
    phase: usize,
    warmup: u32,
    frame_count: u32,
    phase_start: Instant,
    metrics: Vec<Metric>,
    baseline_ram: f64,
}

const WARMUP_FRAMES: u32 = 30;
const MEASURE_FRAMES: u32 = 120;

fn run_phase(phase: usize, ren: &mut Renderer2D, tex: TextureHandle) -> Option<(&'static str, u32)> {
    match phase {
        // --- sprite scaling ---
        0 => {
            for i in 0..10_000u32 {
                let x = (i as f32 * 137.5) % 800.0;
                let y = (i as f32 * 97.3) % 600.0;
                ren.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y)));
            }
            Some(("10k sprites", 10_000))
        }
        1 => {
            for i in 0..50_000u32 {
                let x = (i as f32 * 137.5) % 800.0;
                let y = (i as f32 * 97.3) % 600.0;
                ren.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y)));
            }
            Some(("50k sprites", 50_000))
        }
        2 => {
            for i in 0..100_000u32 {
                let x = (i as f32 * 137.5) % 800.0;
                let y = (i as f32 * 97.3) % 600.0;
                ren.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y)));
            }
            Some(("100k sprites", 100_000))
        }
        3 => {
            for i in 0..250_000u32 {
                let x = (i as f32 * 137.5) % 800.0;
                let y = (i as f32 * 97.3) % 600.0;
                ren.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y)));
            }
            Some(("250k sprites", 250_000))
        }
        4 => {
            for i in 0..500_000u32 {
                let x = (i as f32 * 137.5) % 800.0;
                let y = (i as f32 * 97.3) % 600.0;
                ren.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y)));
            }
            Some(("500k sprites", 500_000))
        }
        // --- instanced sprites ---
        5 => {
            let instances: Vec<SpriteInstance> = (0..100_000u32).map(|i| {
                SpriteInstance {
                    transform: Transform2D::new((i as f32 * 137.5) % 800.0, (i as f32 * 97.3) % 600.0),
                    tint: Color::WHITE, opacity: 1.0, src_rect: None,
                    flip_x: false, flip_y: false,
                }
            }).collect();
            ren.draw_sprite_instanced(tex, &instances, BlendMode::Alpha, 0);
            Some(("100k instanced", 100_000))
        }
        // --- shapes ---
        6 => {
            for i in 0..10_000u32 {
                let x = (i as f32 * 137.5) % 800.0;
                let y = (i as f32 * 97.3) % 600.0;
                ren.draw_rect(Rect::new(x, y, 16.0, 16.0), DrawParams::fill(Color::RED));
            }
            Some(("10k filled rects", 10_000))
        }
        7 => {
            for i in 0..10_000u32 {
                let x = (i as f32 * 137.5) % 800.0;
                let y = (i as f32 * 97.3) % 600.0;
                ren.draw_circle(Vec2::new(x, y), 8.0, DrawParams::fill(Color::CYAN));
            }
            Some(("10k filled circles", 10_000))
        }
        8 => {
            for i in 0..10_000u32 {
                let x = (i as f32 * 37.5) % 800.0;
                let y = (i as f32 * 27.3) % 600.0;
                ren.draw_line(Vec2::new(x, y), Vec2::new(x + 30.0, y + 20.0), LineParams::new(Color::GREEN, 2.0));
            }
            Some(("10k lines", 10_000))
        }
        // --- mixed workload ---
        9 => {
            for i in 0..5_000u32 {
                let x = (i as f32 * 137.5) % 800.0;
                let y = (i as f32 * 97.3) % 600.0;
                ren.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y)));
            }
            for i in 0..5_000u32 {
                let x = (i as f32 * 77.7) % 800.0;
                let y = (i as f32 * 53.1) % 600.0;
                ren.draw_rect(Rect::new(x, y, 12.0, 12.0), DrawParams::fill(Color::BLUE));
            }
            Some(("5k sprites + 5k rects", 10_000))
        }
        _ => None,
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let win = event_loop
            .create_window(WindowAttributes::default()
                .with_title("perf metrics")
                .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600)))
            .expect("window");
        let mut ren = Renderer2D::new_with_vsync(&win, false).expect("renderer");
        ren.set_clear_color(Color::BLACK);
        let tex = ren.load_texture(&make_png(16, 16), TextureParams::default()).unwrap();
        self.texture = Some(tex);
        self.renderer = Some(ren);
        self.window = Some(win);
        self.baseline_ram = rss_mb();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(ren) = &mut self.renderer { ren.resize(size.width, size.height); }
            }
            WindowEvent::RedrawRequested => {
                let ren = self.renderer.as_mut().unwrap();
                let tex = self.texture.unwrap();

                ren.begin_frame().unwrap();
                let info = run_phase(self.phase, ren, tex);
                let stats = ren.end_frame().unwrap();

                if info.is_none() {
                    // all phases done — print results
                    let ram_now = rss_mb();
                    println!();
                    println!("╔════════════════════════════════════════════════════════════════════╗");
                    println!("║                lite-render-2d performance metrics                  ║");
                    println!("║                    800x600 window, --release                       ║");
                    println!("╠════════════════════════════════════════════════════════════════════╣");
                    println!("║ {:<26} {:>8} {:>8} {:>7} {:>5} {:>7} ║",
                        "benchmark", "count", "avg ms", "fps", "draws", "RAM MB");
                    println!("╠════════════════════════════════════════════════════════════════════╣");
                    for m in &self.metrics {
                        println!("║ {:<26} {:>8} {:>7.2}  {:>6.0} {:>5} {:>6.1} ║",
                            m.name, m.count, m.avg_ms, m.fps, m.draw_calls, m.ram_mb);
                    }
                    println!("╠════════════════════════════════════════════════════════════════════╣");
                    println!("║ baseline RAM: {:.1} MB    peak RAM: {:.1} MB{:>24}║",
                        self.baseline_ram, ram_now, "");
                    println!("╚════════════════════════════════════════════════════════════════════╝");
                    println!();

                    // markdown table
                    println!("### markdown table:");
                    println!();
                    println!("| benchmark | count | avg frame (ms) | fps | draw calls | RAM (MB) |");
                    println!("|---|---|---|---|---|---|");
                    for m in &self.metrics {
                        println!("| {} | {:>} | {:.2} | {:.0} | {} | {:.1} |",
                            m.name, m.count, m.avg_ms, m.fps, m.draw_calls, m.ram_mb);
                    }
                    println!();
                    println!("baseline RAM: {:.1} MB | peak RAM: {:.1} MB", self.baseline_ram, ram_now);
                    println!();

                    event_loop.exit();
                    return;
                }

                let (name, count) = info.unwrap();

                self.warmup += 1;
                if self.warmup <= WARMUP_FRAMES {
                    if self.warmup == WARMUP_FRAMES {
                        self.phase_start = Instant::now();
                        self.frame_count = 0;
                    }
                } else {
                    self.frame_count += 1;
                    if self.frame_count >= MEASURE_FRAMES {
                        let elapsed = self.phase_start.elapsed().as_secs_f64();
                        let avg_ms = (elapsed / MEASURE_FRAMES as f64) * 1000.0;
                        let fps = MEASURE_FRAMES as f64 / elapsed;
                        let ram = rss_mb();
                        self.metrics.push(Metric {
                            name: name.to_string(),
                            count,
                            avg_ms,
                            fps,
                            draw_calls: stats.draw_calls,
                            ram_mb: ram,
                        });
                        eprint!("  done: {:<26} {:>6.0} fps  {:.1} MB\n", name, fps, ram);
                        self.phase += 1;
                        self.warmup = 0;
                        self.frame_count = 0;
                    }
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
    eprintln!("running performance benchmarks ({} warmup + {} measure frames per test)...", WARMUP_FRAMES, MEASURE_FRAMES);
    eprintln!("baseline RAM: {:.1} MB", rss_mb());
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        window: None, renderer: None, texture: None,
        phase: 0, warmup: 0, frame_count: 0,
        phase_start: Instant::now(), metrics: Vec::new(),
        baseline_ram: 0.0,
    };
    event_loop.run_app(&mut app).unwrap();
}
