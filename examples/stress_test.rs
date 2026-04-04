//! Stress test — escalates sprite count until FPS drops below 30, finds the ceiling.
//! Also measures idle RAM (no sprites) and minimal workload RAM.
//!
//! Run: cargo run -p lite-render-2d-glow --example stress_test --release

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

fn make_png(w: u32, h: u32) -> Vec<u8> {
    let rgba = vec![200u8; (w * h * 4) as usize];
    let mut buf = std::io::Cursor::new(Vec::new());
    let enc = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(enc, &rgba, w, h, image::ColorType::Rgba8.into()).unwrap();
    buf.into_inner()
}

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

struct Result {
    count: u32,
    fps: f64,
    avg_ms: f64,
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
    results: Vec<Result>,
    idle_ram: f64,
    renderer_ram: f64,
    counts: Vec<u32>,
    done: bool,
}

const WARMUP: u32 = 20;
const MEASURE: u32 = 90;

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        self.idle_ram = rss_mb();
        let win = event_loop
            .create_window(WindowAttributes::default()
                .with_title("stress test")
                .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600)))
            .expect("window");
        let mut ren = Renderer2D::new_with_vsync(&win, false).expect("renderer");
        ren.set_clear_color(Color::BLACK);
        let tex = ren.load_texture(&make_png(16, 16), TextureParams::default()).unwrap();
        self.texture = Some(tex);
        self.renderer = Some(ren);
        self.window = Some(win);
        self.renderer_ram = rss_mb();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(ren) = &mut self.renderer { ren.resize(size.width, size.height); }
            }
            WindowEvent::RedrawRequested => {
                if self.done {
                    event_loop.exit();
                    return;
                }

                if self.phase >= self.counts.len() {
                    // print results
                    println!();
                    println!("╔══════════════════════════════════════════════════════════════════════╗");
                    println!("║                   lite-render-2d stress test                         ║");
                    println!("║               800x600, vsync off, --release                          ║");
                    println!("╠══════════════════════════════════════════════════════════════════════╣");
                    println!("║  idle process RAM:     {:>6.1} MB                                     ║", self.idle_ram);
                    println!("║  renderer + window:    {:>6.1} MB                                     ║", self.renderer_ram);
                    println!("╠══════════════════════════════════════════════════════════════════════╣");
                    println!("║ {:>10}  {:>8}  {:>8}  {:>6}  {:>8} ║",
                        "sprites", "fps", "frame ms", "draws", "RAM MB");
                    println!("╠══════════════════════════════════════════════════════════════════════╣");
                    for r in &self.results {
                        let marker = if r.fps >= 60.0 { " " }
                            else if r.fps >= 30.0 { "*" }
                            else { "!" };
                        println!("║ {:>10}  {:>7.0}{}  {:>7.2}  {:>6}  {:>7.1} ║",
                            r.count, r.fps, marker, r.avg_ms, r.draw_calls, r.ram_mb);
                    }
                    println!("╠══════════════════════════════════════════════════════════════════════╣");

                    // find thresholds
                    let at60 = self.results.iter().rev().find(|r| r.fps >= 60.0).map(|r| r.count).unwrap_or(0);
                    let at30 = self.results.iter().rev().find(|r| r.fps >= 30.0).map(|r| r.count).unwrap_or(0);
                    let peak_ram = self.results.last().map(|r| r.ram_mb).unwrap_or(0.0);
                    let ram_delta = peak_ram - self.renderer_ram;

                    println!("║  60 fps ceiling:  ~{:<10} sprites                                ║", at60);
                    println!("║  30 fps ceiling:  ~{:<10} sprites                                ║", at30);
                    println!("║  peak RAM:        {:>6.1} MB  (renderer overhead: {:.1} MB)           ║", peak_ram, ram_delta);
                    println!("╚══════════════════════════════════════════════════════════════════════╝");
                    println!();

                    // markdown
                    println!("| sprites | fps | frame (ms) | draw calls | RAM (MB) |");
                    println!("|---|---|---|---|---|");
                    for r in &self.results {
                        println!("| {} | {:.0} | {:.2} | {} | {:.1} |",
                            r.count, r.fps, r.avg_ms, r.draw_calls, r.ram_mb);
                    }
                    println!();
                    println!("60fps ceiling: ~{} sprites | 30fps ceiling: ~{} sprites", at60, at30);
                    println!("idle RAM: {:.1} MB | renderer RAM: {:.1} MB | peak RAM: {:.1} MB",
                        self.idle_ram, self.renderer_ram, peak_ram);

                    self.done = true;
                    event_loop.exit();
                    return;
                }

                let ren = self.renderer.as_mut().unwrap();
                let tex = self.texture.unwrap();
                let count = self.counts[self.phase];

                ren.begin_frame().unwrap();
                for i in 0..count {
                    let fi = i as f32;
                    let x = (fi * 137.5) % 800.0;
                    let y = (fi * 97.3) % 600.0;
                    ren.draw_sprite(tex, SpriteParams::new(Transform2D::new(x, y)));
                }
                let stats = ren.end_frame().unwrap();

                self.warmup += 1;
                if self.warmup <= WARMUP {
                    if self.warmup == WARMUP {
                        self.phase_start = Instant::now();
                        self.frame_count = 0;
                    }
                } else {
                    self.frame_count += 1;
                    if self.frame_count >= MEASURE {
                        let elapsed = self.phase_start.elapsed().as_secs_f64();
                        let fps = MEASURE as f64 / elapsed;
                        let avg_ms = (elapsed / MEASURE as f64) * 1000.0;
                        let ram = rss_mb();
                        eprint!("  {:>8} sprites: {:>6.0} fps  {:.1} ms  {:.1} MB\n",
                            count, fps, avg_ms, ram);
                        self.results.push(Result {
                            count, fps, avg_ms,
                            draw_calls: stats.draw_calls,
                            ram_mb: ram,
                        });

                        // stop if fps dropped below 15
                        if fps < 15.0 {
                            self.phase = self.counts.len(); // skip to end
                        } else {
                            self.phase += 1;
                        }
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
    eprintln!("lite-render-2d stress test — finding sprite ceiling...");
    eprintln!("idle process RAM: {:.1} MB", rss_mb());
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        window: None, renderer: None, texture: None,
        phase: 0, warmup: 0, frame_count: 0,
        phase_start: Instant::now(),
        results: Vec::new(),
        idle_ram: 0.0, renderer_ram: 0.0,
        counts: vec![
            0,         // empty frame (RAM baseline)
            1_000,
            5_000,
            10_000,
            25_000,
            50_000,
            75_000,
            100_000,
            150_000,
            200_000,
            250_000,
            350_000,
            500_000,
            750_000,
            1_000_000,
        ],
        done: false,
    };
    event_loop.run_app(&mut app).unwrap();
}
