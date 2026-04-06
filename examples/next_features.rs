use lite_render_2d_core::{
    BlendMode, Color, DrawParams, DrawStyle, FontHandle, NineSlice, ParticleConfig, ParticleSystem,
    Rect, Renderer, SpriteParams, TextAlign, TextParams, TextureHandle, TextureParams, Tilemap,
    TilesetInfo, Transform2D, Vec2,
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
    nine_slice_tex: Option<TextureHandle>,
    tileset_tex: Option<TextureHandle>,
    font: Option<FontHandle>,
    particles: ParticleSystem,
    last_time: std::time::Instant,
}

// generate a nine-slice panel texture (64x64 with visible borders)
fn make_nine_slice_png() -> Vec<u8> {
    let w = 64u32;
    let h = 64u32;
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    let border = 12u32;

    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let in_border = x < border || x >= w - border || y < border || y >= h - border;
            if in_border {
                // darker border region
                pixels[idx] = 80;
                pixels[idx + 1] = 120;
                pixels[idx + 2] = 200;
                pixels[idx + 3] = 255;
            } else {
                // lighter center
                pixels[idx] = 40;
                pixels[idx + 1] = 60;
                pixels[idx + 2] = 120;
                pixels[idx + 3] = 220;
            }
            // corner highlights
            let corner_dist = |cx: u32, cy: u32| -> f32 {
                let dx = x as f32 - cx as f32;
                let dy = y as f32 - cy as f32;
                (dx * dx + dy * dy).sqrt()
            };
            let corners = [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)];
            for &(cx, cy) in &corners {
                if corner_dist(cx, cy) < border as f32 {
                    pixels[idx] = 100;
                    pixels[idx + 1] = 150;
                    pixels[idx + 2] = 230;
                }
            }
        }
    }
    encode_png(&pixels, w, h)
}

// generate a simple tileset texture (4x2 grid of 16x16 tiles = 64x32)
fn make_tileset_png() -> Vec<u8> {
    let tw = 16u32;
    let cols = 4u32;
    let rows = 2u32;
    let w = tw * cols;
    let h = tw * rows;
    let mut pixels = vec![0u8; (w * h * 4) as usize];

    let tile_colors: [[u8; 3]; 8] = [
        [0, 0, 0],       // 0: empty/black
        [80, 160, 80],   // 1: grass green
        [120, 80, 40],   // 2: dirt brown
        [60, 100, 200],  // 3: water blue
        [140, 140, 140], // 4: stone gray
        [200, 180, 60],  // 5: sand yellow
        [40, 100, 40],   // 6: dark grass
        [180, 80, 60],   // 7: brick red
    ];

    for tile_id in 0..8u32 {
        let tx = (tile_id % cols) * tw;
        let ty = (tile_id / cols) * tw;
        let c = tile_colors[tile_id as usize];
        for py in 0..tw {
            for px in 0..tw {
                let idx = (((ty + py) * w + tx + px) * 4) as usize;
                // add slight variation for visual interest
                let v = ((px + py) % 3) as i16 * 8;
                pixels[idx] = (c[0] as i16 + v).clamp(0, 255) as u8;
                pixels[idx + 1] = (c[1] as i16 + v).clamp(0, 255) as u8;
                pixels[idx + 2] = (c[2] as i16 - v).clamp(0, 255) as u8;
                pixels[idx + 3] = 255;
                // draw grid lines at edges
                if px == 0 || py == 0 {
                    pixels[idx] = pixels[idx].saturating_add(30);
                    pixels[idx + 1] = pixels[idx + 1].saturating_add(30);
                    pixels[idx + 2] = pixels[idx + 2].saturating_add(30);
                }
            }
        }
    }
    encode_png(&pixels, w, h)
}

fn encode_png(pixels: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let encoder = image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut buf));
        use image::ImageEncoder;
        encoder.write_image(pixels, w, h, image::ExtendedColorType::Rgba8).unwrap();
    }
    buf
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("next features showcase")
            .with_inner_size(winit::dpi::LogicalSize::new(1000, 700));
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::new(0.1, 0.1, 0.15, 1.0));

        // load font
        let font_data = std::fs::read("NotoSansMeroitic-Regular.ttf").expect("read font file");
        let font = ren.load_font(&font_data).expect("load font");
        self.font = Some(font);

        // load nine-slice texture
        let ns_data = make_nine_slice_png();
        let ns_tex = ren.load_texture(&ns_data, TextureParams::default()).expect("load nine-slice");
        self.nine_slice_tex = Some(ns_tex);

        // load tileset texture
        let ts_data = make_tileset_png();
        let ts_tex = ren.load_texture(&ts_data, TextureParams::default()).expect("load tileset");
        self.tileset_tex = Some(ts_tex);

        // setup particle emitter
        self.particles.add_emitter(
            ParticleConfig {
                spawn_rate: 30.0,
                lifetime: (0.8, 2.0),
                velocity: (Vec2::new(-40.0, -80.0), Vec2::new(40.0, -20.0)),
                size: (6.0, 1.0),
                color_start: Color::new(1.0, 0.5, 0.1, 1.0),
                color_end: Color::new(1.0, 0.1, 0.0, 0.0),
                gravity: Vec2::new(0.0, 60.0),
                texture: None,
            },
            Vec2::new(800.0, 500.0),
        );

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
                let now = std::time::Instant::now();
                let dt = now.duration_since(self.last_time).as_secs_f32().min(0.1);
                self.last_time = now;

                self.particles.update(dt);

                let ren = self.renderer.as_mut().unwrap();
                ren.begin_frame().unwrap();

                // -- row 1: nine-slice panels at different sizes --
                draw_section_bg(ren, 10.0, 10.0, 480.0, 200.0);
                if let Some(ns_tex) = self.nine_slice_tex {
                    let ns = NineSlice {
                        texture: ns_tex,
                        border_left: 12.0,
                        border_right: 12.0,
                        border_top: 12.0,
                        border_bottom: 12.0,
                    };
                    // small panel
                    ren.draw_nine_slice(&ns, Rect { pos: Vec2::new(20.0, 30.0), size: Vec2::new(100.0, 60.0) }, Color::WHITE, 1);
                    // medium panel
                    ren.draw_nine_slice(&ns, Rect { pos: Vec2::new(140.0, 30.0), size: Vec2::new(200.0, 80.0) }, Color::WHITE, 1);
                    // wide panel
                    ren.draw_nine_slice(&ns, Rect { pos: Vec2::new(20.0, 120.0), size: Vec2::new(460.0, 50.0) }, Color::new(1.0, 0.8, 0.6, 1.0), 1);
                    // tinted panel
                    ren.draw_nine_slice(&ns, Rect { pos: Vec2::new(360.0, 30.0), size: Vec2::new(120.0, 80.0) }, Color::new(0.6, 1.0, 0.6, 1.0), 1);
                }

                // -- row 1 right: complex polygon with hole --
                draw_section_bg(ren, 500.0, 10.0, 490.0, 200.0);
                // star shape (outer) with square hole
                let star_cx = 620.0_f32;
                let star_cy = 110.0_f32;
                let outer: Vec<Vec2> = (0..10).map(|i| {
                    let angle = std::f32::consts::PI * 2.0 * i as f32 / 10.0 - std::f32::consts::FRAC_PI_2;
                    let r = if i % 2 == 0 { 70.0 } else { 35.0 };
                    Vec2::new(star_cx + angle.cos() * r, star_cy + angle.sin() * r)
                }).collect();

                let hole = vec![
                    Vec2::new(star_cx - 15.0, star_cy - 15.0),
                    Vec2::new(star_cx + 15.0, star_cy - 15.0),
                    Vec2::new(star_cx + 15.0, star_cy + 15.0),
                    Vec2::new(star_cx - 15.0, star_cy + 15.0),
                ];
                let hole_refs: Vec<&[Vec2]> = vec![&hole];
                ren.draw_complex_polygon(&outer, &hole_refs, DrawParams::fill(Color::new(0.9, 0.3, 0.5, 1.0)));

                // concave L-shape
                let l_shape = vec![
                    Vec2::new(800.0, 30.0), Vec2::new(900.0, 30.0), Vec2::new(900.0, 80.0),
                    Vec2::new(850.0, 80.0), Vec2::new(850.0, 180.0), Vec2::new(800.0, 180.0),
                ];
                ren.draw_complex_polygon(&l_shape, &[], DrawParams::fill(Color::new(0.3, 0.7, 0.9, 1.0)));

                // -- row 2: tilemap --
                draw_section_bg(ren, 10.0, 220.0, 480.0, 230.0);
                if let Some(ts_tex) = self.tileset_tex {
                    let tileset = TilesetInfo {
                        tile_width: 16.0,
                        tile_height: 16.0,
                        columns: 4,
                    };
                    let mut tmap = Tilemap::new(12, 8, 24.0, ts_tex, tileset);
                    // fill with a simple pattern
                    for y in 0..8u32 {
                        for x in 0..12u32 {
                            let tile = if y == 0 || y == 7 {
                                4 // stone border
                            } else if x == 0 || x == 11 {
                                4
                            } else if y >= 5 {
                                2 // dirt bottom
                            } else if y == 4 {
                                1 // grass
                            } else if x > 3 && x < 7 && y > 1 && y < 4 {
                                3 // water pond
                            } else {
                                5 // sand
                            };
                            tmap.set_tile(x, y, tile);
                        }
                    }
                    ren.draw_tilemap(&tmap, Vec2::new(50.0, 240.0), 0);
                }

                // -- row 2 right: particle system --
                draw_section_bg(ren, 500.0, 220.0, 490.0, 230.0);
                self.particles.draw(ren);

                // -- row 3: text measurement demo --
                draw_section_bg(ren, 10.0, 460.0, 980.0, 220.0);
                if let Some(font) = self.font {
                    let texts = ["Hello, lite-render-2d!", "measure_text test", "Nine-Slice + Tilemap + Particles"];
                    let sizes = [24.0, 32.0, 18.0];
                    let colors = [
                        Color::new(1.0, 1.0, 1.0, 1.0),
                        Color::new(0.5, 1.0, 0.5, 1.0),
                        Color::new(1.0, 0.7, 0.3, 1.0),
                    ];

                    let mut y_offset = 480.0;
                    for i in 0..3 {
                        let params = TextParams {
                            font,
                            size: sizes[i],
                            color: colors[i],
                            align: TextAlign::Left,
                            position: Vec2::new(30.0, y_offset),
                            max_width: None,
                            line_height: None,
                            z: 0,
                        };

                        // measure text to get bounding box
                        let bounds = ren.measure_text(texts[i], &params);

                        // draw bounding box behind text
                        ren.draw_rect(
                            Rect {
                                pos: Vec2::new(30.0, y_offset),
                                size: bounds,
                            },
                            DrawParams {
                                style: DrawStyle::Stroke(lite_render_2d_core::StrokeParams {
                                    color: Color::new(1.0, 1.0, 0.0, 0.6),
                                    thickness: 1.0,
                                    style: lite_render_2d_core::StrokeStyle::Solid,
                                    cap: lite_render_2d_core::LineCap::Butt,
                                    join: lite_render_2d_core::LineJoin::Miter,
                                }),
                                blend: BlendMode::Alpha,
                                z_index: 0,
                                opacity: 1.0,
                            },
                        );

                        // draw the text
                        ren.draw_text(texts[i], &params);

                        y_offset += sizes[i] + 20.0;
                    }
                }

                ren.end_frame().unwrap();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}

fn draw_section_bg(ren: &mut Renderer2D, x: f32, y: f32, w: f32, h: f32) {
    ren.draw_rect(
        Rect { pos: Vec2::new(x, y), size: Vec2::new(w, h) },
        DrawParams {
            style: DrawStyle::Fill(Color::new(0.15, 0.15, 0.22, 0.8)),
            blend: BlendMode::Alpha,
            z_index: -1,
            opacity: 1.0,
        },
    );
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        window: None,
        renderer: None,
        nine_slice_tex: None,
        tileset_tex: None,
        font: None,
        particles: ParticleSystem::new(),
        last_time: std::time::Instant::now(),
    };
    event_loop.run_app(&mut app).unwrap();
}
