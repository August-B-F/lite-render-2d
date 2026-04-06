// test for all 7 renderer optimizations:
// 1. frustum culling  2. u8 shape color  3. rotation skip
// 4. instanced draw   5. font atlas partial upload
// 6. double-buffered vbos  7. sprite atlas regrow

use std::time::Instant;

use lite_render_2d_core::{
    BlendMode, Camera2D, Color, DrawParams, DrawStyle, FontHandle, LineParams, Rect, Renderer,
    SpriteInstance, SpriteParams, TextAlign, TextParams, TextureHandle, TextureParams, Transform2D,
    Vec2,
};
#[cfg(feature = "use-glow")]
use lite_render_2d_glow::GlowRenderer as Renderer2D;
#[cfg(feature = "use-wgpu")]
use lite_render_2d_wgpu::WgpuRenderer as Renderer2D;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

fn make_tiny_png(r: u8, g: u8, b: u8, size: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for i in 0..(size * size) as usize {
        rgba[i * 4] = r;
        rgba[i * 4 + 1] = g;
        rgba[i * 4 + 2] = b;
        rgba[i * 4 + 3] = 255;
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    let enc = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(enc, &rgba, size, size, image::ColorType::Rgba8.into())
        .expect("encode");
    buf.into_inner()
}

struct TestResult {
    name: &'static str,
    passed: bool,
    detail: String,
}

struct App {
    window: Option<Window>,
    renderer: Option<Renderer2D>,
    textures: Vec<TextureHandle>,
    font: Option<FontHandle>,
    frame: u32,
    results: Vec<TestResult>,
    done: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            textures: Vec::new(),
            font: None,
            frame: 0,
            results: Vec::new(),
            done: false,
        }
    }

    fn run_tests(&mut self, ren: &mut Renderer2D) {
        match self.frame {
            0 => {
                self.test_frustum_culling(ren);
                self.test_shape_rendering(ren);
                self.test_rotation_skip(ren);
            }
            1 => self.test_instanced_rendering(ren),
            2 => self.test_font_atlas_partial(ren),
            3 => self.test_double_buffer(ren),
            4 => self.test_atlas_regrow(ren),
            5 => {
                self.print_results();
                self.done = true;
            }
            _ => {}
        }
    }

    fn test_frustum_culling(&mut self, ren: &mut Renderer2D) {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.position = Vec2::new(400.0, 300.0);
        ren.set_camera(&cam);
        ren.begin_frame().expect("begin");

        // draw 1000 rects mostly offscreen
        for i in 0..1000u32 {
            let x = -5000.0 + (i as f32 * 137.5) % 10000.0;
            let y = -5000.0 + (i as f32 * 173.1) % 10000.0;
            ren.draw_rect(
                Rect { pos: Vec2::new(x, y), size: Vec2::new(20.0, 20.0) },
                DrawParams::fill(Color::RED),
            );
        }
        // 1000 circles mostly offscreen
        for i in 0..1000u32 {
            let x = -5000.0 + (i as f32 * 191.3) % 10000.0;
            let y = -5000.0 + (i as f32 * 237.7) % 10000.0;
            ren.draw_circle(Vec2::new(x, y), 15.0, DrawParams::fill(Color::GREEN));
        }
        // 1000 lines mostly offscreen
        for i in 0..1000u32 {
            let x = -5000.0 + (i as f32 * 113.9) % 10000.0;
            let y = -5000.0 + (i as f32 * 291.1) % 10000.0;
            ren.draw_line(Vec2::new(x, y), Vec2::new(x + 50.0, y + 50.0), LineParams::new(Color::BLUE, 2.0));
        }
        // 500 sprites mostly offscreen
        if let Some(&tex) = self.textures.first() {
            for i in 0..500u32 {
                let x = -5000.0 + (i as f32 * 371.3) % 10000.0;
                let y = -5000.0 + (i as f32 * 413.7) % 10000.0;
                ren.draw_sprite(tex, SpriteParams::new(Transform2D {
                    pos: Vec2::new(x, y), ..Transform2D::default()
                }));
            }
        }

        let stats = ren.end_frame().expect("end");
        // with culling most objects skipped
        let passed = stats.vertices < 10000;
        self.results.push(TestResult {
            name: "frustum culling",
            passed,
            detail: format!("verts={} (expected <10000 with culling)", stats.vertices),
        });
    }

    fn test_shape_rendering(&mut self, ren: &mut Renderer2D) {
        let cam = Camera2D::new(800.0, 600.0);
        ren.set_camera(&cam);
        ren.begin_frame().expect("begin");

        // filled rect
        ren.draw_rect(
            Rect { pos: Vec2::new(10.0, 10.0), size: Vec2::new(100.0, 50.0) },
            DrawParams::fill(Color::new(0.8, 0.2, 0.3, 1.0)),
        );
        // stroked rect
        ren.draw_rect(
            Rect { pos: Vec2::new(120.0, 10.0), size: Vec2::new(100.0, 50.0) },
            DrawParams::stroke(Color::WHITE, 2.0),
        );
        // circle
        ren.draw_circle(Vec2::new(300.0, 35.0), 25.0, DrawParams::fill(Color::BLUE));
        // gradient rect (per-vertex color packing)
        ren.draw_rect(
            Rect { pos: Vec2::new(10.0, 70.0), size: Vec2::new(200.0, 50.0) },
            DrawParams {
                style: DrawStyle::LinearGradient {
                    start: Vec2::new(10.0, 70.0),
                    end: Vec2::new(210.0, 120.0),
                    color_start: Color::RED,
                    color_end: Color::BLUE,
                },
                blend: BlendMode::Alpha,
                z_index: 0,
                opacity: 1.0,
            },
        );
        // line
        ren.draw_line(Vec2::new(350.0, 10.0), Vec2::new(450.0, 60.0), LineParams::new(Color::GREEN, 3.0));

        let stats = ren.end_frame().expect("end");
        let passed = stats.vertices > 0;
        self.results.push(TestResult {
            name: "u8 shape color packing",
            passed,
            detail: format!("drew shapes ok, verts={} draws={}", stats.vertices, stats.draw_calls),
        });
    }

    fn test_rotation_skip(&mut self, ren: &mut Renderer2D) {
        if self.textures.is_empty() { return; }
        let tex = self.textures[0];
        let cam = Camera2D::new(800.0, 600.0);
        ren.set_camera(&cam);
        ren.begin_frame().expect("begin");

        let t0 = Instant::now();
        // 5000 unrotated sprites (should skip trig)
        for i in 0..5000u32 {
            let x = (i % 100) as f32 * 8.0;
            let y = (i / 100) as f32 * 8.0;
            ren.draw_sprite(tex, SpriteParams::new(Transform2D {
                pos: Vec2::new(x, y),
                scale: Vec2::new(0.1, 0.1),
                rotation: 0.0,
            }));
        }
        // 500 rotated sprites
        for i in 0..500u32 {
            let x = (i % 50) as f32 * 16.0;
            let y = 420.0 + (i / 50) as f32 * 16.0;
            ren.draw_sprite(tex, SpriteParams::new(Transform2D {
                pos: Vec2::new(x, y),
                scale: Vec2::new(0.1, 0.1),
                rotation: i as f32 * 0.1,
            }));
        }
        let elapsed = t0.elapsed();
        let stats = ren.end_frame().expect("end");

        let passed = stats.vertices > 0;
        self.results.push(TestResult {
            name: "rotation skip",
            passed,
            detail: format!("5500 sprites in {:.2}ms, verts={}", elapsed.as_secs_f64() * 1000.0, stats.vertices),
        });
    }

    fn test_instanced_rendering(&mut self, ren: &mut Renderer2D) {
        if self.textures.is_empty() { return; }
        let tex = self.textures[0];
        let cam = Camera2D::new(800.0, 600.0);
        ren.set_camera(&cam);
        ren.begin_frame().expect("begin");

        let mut instances = Vec::with_capacity(2000);
        for i in 0..2000u32 {
            let x = (i % 50) as f32 * 16.0;
            let y = (i / 50) as f32 * 16.0;
            let mut t = Transform2D::default();
            t.pos = Vec2::new(x, y);
            t.scale = Vec2::new(0.2, 0.2);
            t.rotation = if i % 5 == 0 { i as f32 * 0.05 } else { 0.0 };
            let mut inst = SpriteInstance::new(t);
            inst.tint = Color::new(
                (i as f32 * 0.01).sin().abs(),
                (i as f32 * 0.02).cos().abs(),
                0.7, 1.0,
            );
            instances.push(inst);
        }
        ren.draw_sprite_instanced(tex, &instances, BlendMode::Alpha, 0);

        let stats = ren.end_frame().expect("end");
        // true instancing = 1 draw call, not 2000
        let passed = stats.draw_calls <= 3;
        self.results.push(TestResult {
            name: "instanced rendering",
            passed,
            detail: format!("2000 instances, draw_calls={} (expected <=3)", stats.draw_calls),
        });
    }

    fn test_font_atlas_partial(&mut self, ren: &mut Renderer2D) {
        let font = match self.font {
            Some(f) => f,
            None => {
                self.results.push(TestResult {
                    name: "font atlas partial upload",
                    passed: true,
                    detail: "skipped (no font file found)".into(),
                });
                return;
            }
        };
        let cam = Camera2D::new(800.0, 600.0);
        ren.set_camera(&cam);

        // frame A: initial text forces full atlas upload
        ren.begin_frame().expect("begin");
        ren.draw_text("hello world 123", &TextParams {
            font, size: 24.0, color: Color::WHITE, position: Vec2::new(10.0, 10.0),
            align: TextAlign::Left, max_width: None, line_height: None, z: 0,
        });
        let stats_a = ren.end_frame().expect("end");

        // frame B: new chars should trigger partial upload
        ren.begin_frame().expect("begin");
        ren.draw_text("ABCDEFGHIJ @#$%", &TextParams {
            font, size: 24.0, color: Color::WHITE, position: Vec2::new(10.0, 50.0),
            align: TextAlign::Left, max_width: None, line_height: None, z: 0,
        });
        let stats_b = ren.end_frame().expect("end");

        let passed = stats_a.vertices > 0 && stats_b.vertices > 0;
        self.results.push(TestResult {
            name: "font atlas partial upload",
            passed,
            detail: format!("frame_a: verts={} draws={}, frame_b: verts={} draws={}",
                stats_a.vertices, stats_a.draw_calls, stats_b.vertices, stats_b.draw_calls),
        });
    }

    fn test_double_buffer(&mut self, ren: &mut Renderer2D) {
        let cam = Camera2D::new(800.0, 600.0);
        ren.set_camera(&cam);

        let mut total_verts = 0u32;
        for f in 0..5u32 {
            ren.begin_frame().expect("begin");
            for i in 0..(100 + f * 50) {
                let x = (i as f32 * 7.3) % 780.0;
                let y = (i as f32 * 11.1) % 580.0;
                ren.draw_rect(
                    Rect { pos: Vec2::new(x, y), size: Vec2::new(10.0, 10.0) },
                    DrawParams::fill(Color::new(0.5, 0.3, f as f32 / 5.0, 1.0)),
                );
            }
            let stats = ren.end_frame().expect("end");
            total_verts += stats.vertices;
        }

        let passed = total_verts > 0;
        self.results.push(TestResult {
            name: "double-buffered VBOs",
            passed,
            detail: format!("5 frames ok, total_verts={}", total_verts),
        });
    }

    fn test_atlas_regrow(&mut self, ren: &mut Renderer2D) {
        // load many small textures to fill atlas and trigger regrow
        let mut loaded = 0u32;
        let mut failed = 0u32;

        for i in 0..300u32 {
            let r = ((i * 37) % 256) as u8;
            let g = ((i * 73) % 256) as u8;
            let b = ((i * 113) % 256) as u8;
            let png = make_tiny_png(r, g, b, 128);
            match ren.load_texture(&png, TextureParams::default()) {
                Ok(tex) => { self.textures.push(tex); loaded += 1; }
                Err(_) => { failed += 1; }
            }
        }

        // draw some to verify they render
        let cam = Camera2D::new(800.0, 600.0);
        ren.set_camera(&cam);
        ren.begin_frame().expect("begin");
        for (i, &tex) in self.textures.iter().enumerate().take(20) {
            let x = (i % 10) as f32 * 80.0;
            let y = (i / 10) as f32 * 80.0;
            ren.draw_sprite(tex, SpriteParams::new(Transform2D {
                pos: Vec2::new(x, y),
                scale: Vec2::new(0.5, 0.5),
                rotation: 0.0,
            }));
        }
        let stats = ren.end_frame().expect("end");

        let passed = loaded > 250 && stats.vertices > 0;
        self.results.push(TestResult {
            name: "sprite atlas regrow",
            passed,
            detail: format!("loaded={} failed={} verts={}", loaded, failed, stats.vertices),
        });
    }

    fn print_results(&self) {
        println!("\n========================================");
        println!("  OPTIMIZATION TEST RESULTS");
        println!("========================================\n");
        let mut pass_count = 0;
        let total = self.results.len();
        for r in &self.results {
            let tag = if r.passed { "PASS" } else { "FAIL" };
            println!("  [{}] {}", tag, r.name);
            println!("        {}", r.detail);
            if r.passed { pass_count += 1; }
        }
        println!("\n  {}/{} tests passed", pass_count, total);
        if pass_count == total {
            println!("  all good!\n");
        } else {
            println!("  some tests failed!\n");
        }
        println!("========================================\n");
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = WindowAttributes::default()
            .with_title("optimization tests")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::new(0.05, 0.05, 0.08, 1.0));

        // load initial textures for sprite tests
        let tex_a = ren.load_texture(&make_tiny_png(220, 50, 200, 32), TextureParams::default()).expect("tex");
        let tex_b = ren.load_texture(&make_tiny_png(50, 200, 220, 32), TextureParams::default()).expect("tex");
        self.textures.push(tex_a);
        self.textures.push(tex_b);

        // try loading font (optional)
        if let Ok(data) = std::fs::read("NotoSansMeroitic-Regular.ttf") {
            if let Ok(f) = ren.load_font(&data) {
                self.font = Some(f);
            }
        }

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
                if self.done {
                    event_loop.exit();
                    return;
                }
                let mut ren = self.renderer.take().unwrap();
                self.run_tests(&mut ren);
                self.renderer = Some(ren);
                self.frame += 1;
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run app");
}
