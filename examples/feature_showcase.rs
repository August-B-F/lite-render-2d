use lite_render_2d_core::{
    BlendMode, Color, DrawParams, LineCap, LineJoin, LineParams, Path, Rect, Renderer,
    SpriteAnimation, SpriteInstance, SpriteParams, SpriteSheet,
    StrokeParams, StrokeStyle, TextureHandle, Transform2D, Vec2, PlaybackMode,
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
    sprite_tex: Option<TextureHandle>,
    anim: Option<SpriteAnimation>,
    frame_count: u32,
    last_time: std::time::Instant,
}

// generate a simple 4-frame sprite sheet (128x32, 4 columns of 32x32)
fn make_sprite_sheet_png() -> Vec<u8> {
    let w = 128u32;
    let h = 32u32;
    let mut pixels = vec![0u8; (w * h * 4) as usize];

    for frame in 0..4u32 {
        let fx = frame * 32;
        // draw a circle with increasing size per frame to simulate animation
        let cx = 16.0_f32;
        let cy = 16.0;
        let r = 6.0 + frame as f32 * 3.0;
        for py in 0..32 {
            for px in 0..32 {
                let dx = px as f32 - cx;
                let dy = py as f32 - cy;
                if dx * dx + dy * dy <= r * r {
                    let idx = ((py * w + fx + px) * 4) as usize;
                    // cycle colors per frame
                    let colors: [[u8; 3]; 4] = [
                        [255, 100, 50],
                        [50, 200, 100],
                        [80, 120, 255],
                        [255, 200, 50],
                    ];
                    pixels[idx] = colors[frame as usize][0];
                    pixels[idx + 1] = colors[frame as usize][1];
                    pixels[idx + 2] = colors[frame as usize][2];
                    pixels[idx + 3] = 255;
                }
            }
        }
    }

    // encode as png
    let mut buf = Vec::new();
    {
        let encoder = image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut buf));
        use image::ImageEncoder;
        encoder.write_image(&pixels, w, h, image::ExtendedColorType::Rgba8).unwrap();
    }
    buf
}

fn draw_label(ren: &mut Renderer2D, x: f32, y: f32, text: &str) {
    // draw a small filled rect as background for the label
    let tw = text.len() as f32 * 7.0 + 8.0;
    ren.draw_rect(
        Rect { pos: Vec2::new(x - 2.0, y - 2.0), size: Vec2::new(tw, 18.0) },
        DrawParams::fill(Color::new(0.0, 0.0, 0.0, 0.6)),
    );
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("feature showcase - all 5 new features")
            .with_inner_size(winit::dpi::LogicalSize::new(1000, 750));
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::new(0.12, 0.12, 0.18, 1.0));

        // load sprite sheet texture
        let png_data = make_sprite_sheet_png();
        let tex = ren.load_texture(&png_data, Default::default()).expect("load sprite sheet");
        self.sprite_tex = Some(tex);

        // setup animation
        let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
        let anim = SpriteAnimation::new(sheet, 0.25, PlaybackMode::Loop);
        self.anim = Some(anim);

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
                // compute dt
                let now = std::time::Instant::now();
                let dt = now.duration_since(self.last_time).as_secs_f32();
                self.last_time = now;
                self.frame_count += 1;

                // update animation
                if let Some(anim) = &mut self.anim {
                    anim.update(dt);
                }

                let ren = self.renderer.as_mut().unwrap();
                ren.begin_frame().expect("begin frame");

                let section_y = [20.0_f32, 160.0, 310.0, 460.0, 590.0];

                // ═══════════════════════════════════════════════════
                // Row 1: Sprite Sheet Animation
                // ═══════════════════════════════════════════════════
                draw_label(ren, 20.0, section_y[0], "sprite sheet animation");

                if let (Some(tex), Some(anim)) = (self.sprite_tex, &self.anim) {
                    // draw all 4 frames statically
                    let sheet = &anim.sheet;
                    for i in 0..4 {
                        let src = sheet.frame_rect(i);
                        let x = 30.0 + i as f32 * 80.0;
                        let mut t = Transform2D::default();
                        t.pos = Vec2::new(x, section_y[0] + 25.0);
                        t.scale = Vec2::new(2.0, 2.0);
                        ren.draw_sprite(tex, SpriteParams::new(t).with_src_rect(src));
                    }

                    // draw animated sprite (larger)
                    let src = anim.current_src_rect();
                    let mut t = Transform2D::default();
                    t.pos = Vec2::new(400.0, section_y[0] + 25.0);
                    t.scale = Vec2::new(3.0, 3.0);
                    ren.draw_sprite(tex, SpriteParams::new(t).with_src_rect(src));

                    draw_label(ren, 400.0, section_y[0] + 100.0, &format!("frame {}", anim.current_frame()));
                }

                // ═══════════════════════════════════════════════════
                // Row 2: Line Joins & Caps
                // ═══════════════════════════════════════════════════
                draw_label(ren, 20.0, section_y[1], "line joins and caps");

                let join_types = [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel];
                let join_names = ["miter", "round", "bevel"];
                let zigzag = [
                    Vec2::new(0.0, 40.0),
                    Vec2::new(30.0, 0.0),
                    Vec2::new(60.0, 40.0),
                    Vec2::new(90.0, 0.0),
                    Vec2::new(120.0, 40.0),
                ];

                for (j, (&join, name)) in join_types.iter().zip(join_names.iter()).enumerate() {
                    let ox = 30.0 + j as f32 * 180.0;
                    let oy = section_y[1] + 25.0;
                    let shifted: Vec<Vec2> = zigzag.iter().map(|p| Vec2::new(p.x + ox, p.y + oy)).collect();
                    let mut params = LineParams::new(Color::new(0.3, 0.8, 1.0, 1.0), 10.0);
                    params.join = join;
                    params.cap = LineCap::Round;
                    ren.draw_polyline(&shifted, params);
                    draw_label(ren, ox, oy + 50.0, name);
                }

                // cap demos
                let cap_types = [LineCap::Butt, LineCap::Round, LineCap::Square];
                let cap_names = ["butt", "round", "square"];
                for (c, (&cap, name)) in cap_types.iter().zip(cap_names.iter()).enumerate() {
                    let ox = 600.0;
                    let oy = section_y[1] + 25.0 + c as f32 * 35.0;
                    let mut params = LineParams::new(Color::new(1.0, 0.6, 0.3, 1.0), 12.0);
                    params.cap = cap;
                    ren.draw_polyline(
                        &[Vec2::new(ox, oy), Vec2::new(ox + 120.0, oy)],
                        params,
                    );
                    draw_label(ren, ox + 130.0, oy - 7.0, name);
                }

                // ═══════════════════════════════════════════════════
                // Row 3: Dashed & Dotted Lines
                // ═══════════════════════════════════════════════════
                draw_label(ren, 20.0, section_y[2], "dashed and dotted lines");

                // dashed polyline
                let mut dash_params = LineParams::new(Color::new(0.4, 1.0, 0.4, 1.0), 6.0);
                dash_params.style = StrokeStyle::Dashed { dash_len: 15.0, gap_len: 8.0 };
                dash_params.cap = LineCap::Butt;
                let dash_pts: Vec<Vec2> = (0..12)
                    .map(|i| Vec2::new(30.0 + i as f32 * 40.0, section_y[2] + 35.0 + (i % 2) as f32 * 20.0))
                    .collect();
                ren.draw_polyline(&dash_pts, dash_params);
                draw_label(ren, 30.0, section_y[2] + 65.0, "dashed");

                // dotted polyline
                let mut dot_params = LineParams::new(Color::new(1.0, 0.5, 1.0, 1.0), 6.0);
                dot_params.style = StrokeStyle::Dotted { spacing: 12.0 };
                dot_params.cap = LineCap::Round;
                let dot_pts: Vec<Vec2> = (0..12)
                    .map(|i| Vec2::new(30.0 + i as f32 * 40.0, section_y[2] + 90.0 + (i % 2) as f32 * 20.0))
                    .collect();
                ren.draw_polyline(&dot_pts, dot_params);
                draw_label(ren, 30.0, section_y[2] + 120.0, "dotted");

                // dashed bezier path
                let dashed_path = Path::new()
                    .move_to(Vec2::new(550.0, section_y[2] + 30.0))
                    .cubic_to(
                        Vec2::new(650.0, section_y[2] - 20.0),
                        Vec2::new(750.0, section_y[2] + 100.0),
                        Vec2::new(850.0, section_y[2] + 50.0),
                    );
                let stroke = StrokeParams {
                    color: Color::new(1.0, 0.9, 0.3, 1.0),
                    thickness: 3.0,
                    style: StrokeStyle::Dashed { dash_len: 12.0, gap_len: 6.0 },
                    cap: LineCap::Round,
                    join: LineJoin::Round,
                };
                ren.stroke_path(&dashed_path, stroke);
                draw_label(ren, 550.0, section_y[2] + 120.0, "dashed bezier");

                // ═══════════════════════════════════════════════════
                // Row 4: Render-to-Texture
                // ═══════════════════════════════════════════════════
                draw_label(ren, 20.0, section_y[3], "render-to-texture");

                let rt = ren.create_render_target(200, 100).expect("create render target");
                ren.begin_render_to_texture(rt).expect("begin rt");

                ren.draw_rect(
                    Rect { pos: Vec2::new(5.0, 5.0), size: Vec2::new(190.0, 90.0) },
                    DrawParams::fill(Color::new(0.2, 0.1, 0.3, 1.0)),
                );
                ren.draw_circle(
                    Vec2::new(60.0, 50.0), 30.0,
                    DrawParams::fill(Color::new(1.0, 0.3, 0.3, 0.9)),
                );
                ren.draw_circle(
                    Vec2::new(140.0, 50.0), 30.0,
                    DrawParams::fill(Color::new(0.3, 0.3, 1.0, 0.9)),
                );

                ren.end_render_to_texture();

                if let Some(rt_tex) = ren.render_target_texture(rt) {
                    let mut t1 = Transform2D::default();
                    t1.pos = Vec2::new(30.0, section_y[3] + 25.0);
                    ren.draw_sprite(rt_tex, SpriteParams::new(t1));

                    let mut t2 = Transform2D::default();
                    t2.pos = Vec2::new(260.0, section_y[3] + 25.0);
                    t2.scale = Vec2::new(1.5, 1.5);
                    t2.rotation = 0.1;
                    ren.draw_sprite(rt_tex, SpriteParams::new(t2));

                    draw_label(ren, 30.0, section_y[3] + 105.0, "original");
                    draw_label(ren, 260.0, section_y[3] + 105.0, "scaled + rotated");
                }

                // ═══════════════════════════════════════════════════
                // Row 5: Instanced Drawing
                // ═══════════════════════════════════════════════════
                draw_label(ren, 20.0, section_y[4], "instanced drawing (100 sprites)");

                if let Some(tex) = self.sprite_tex {
                    let src = SpriteSheet::new(32.0, 32.0, 4, 4).frame_rect(0);
                    let mut instances = Vec::with_capacity(100);
                    for i in 0..100 {
                        let row = i / 20;
                        let col = i % 20;
                        let x = 30.0 + col as f32 * 48.0;
                        let y = section_y[4] + 25.0 + row as f32 * 24.0;
                        let mut t = Transform2D::default();
                        t.pos = Vec2::new(x, y);
                        t.scale = Vec2::new(0.6, 0.6);
                        t.rotation = (i as f32 * 0.15 + self.frame_count as f32 * 0.02).sin() * 0.3;
                        let mut inst = SpriteInstance::new(t);
                        inst.src_rect = Some(src);
                        inst.tint = Color::new(
                            0.5 + (i as f32 * 0.1).sin() * 0.5,
                            0.5 + (i as f32 * 0.15).cos() * 0.5,
                            0.8,
                            1.0,
                        );
                        instances.push(inst);
                    }
                    ren.draw_sprite_instanced(tex, &instances, BlendMode::Alpha, 0);
                }

                ren.end_frame().expect("end frame");
                ren.destroy_render_target(rt);

                if let Some(w) = &self.window {
                    w.request_redraw();
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
        sprite_tex: None,
        anim: None,
        frame_count: 0,
        last_time: std::time::Instant::now(),
    };
    event_loop.run_app(&mut app).expect("run app");
}
