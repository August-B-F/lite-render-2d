// showcase of all 15 new features added in the latest batch
use lite_render_2d_core::{
    AnimatedTile, BlendMode, Camera2D, Color, DrawParams, DrawStyle, FontHandle, GradientStop,
    LineParams, Rect, RenderTargetHandle, Renderer, RoundedRect, StrokeStyle,
    TextAlign, TextParams, TextureHandle, TextureParams, Tilemap, TilesetInfo, TrailRenderer,
    Transform2D, Vec2,
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
    font: Option<FontHandle>,
    tileset_tex: Option<TextureHandle>,
    rt: Option<RenderTargetHandle>,
    trail: TrailRenderer,
    cam: Camera2D,
    time: f32,
    last_time: std::time::Instant,
}

// generate a simple 4x2 tileset (64x32 pixels)
fn make_tileset_png() -> Vec<u8> {
    let tw = 16u32;
    let (cols, rows) = (4u32, 2u32);
    let (w, h) = (tw * cols, tw * rows);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let colors: [[u8; 3]; 8] = [
        [0, 0, 0], [80, 160, 80], [120, 80, 40], [60, 100, 200],
        [140, 140, 140], [200, 180, 60], [40, 100, 40], [180, 80, 60],
    ];
    for tid in 0..8u32 {
        let (tx, ty) = ((tid % cols) * tw, (tid / cols) * tw);
        let c = colors[tid as usize];
        for py in 0..tw {
            for pxx in 0..tw {
                let i = (((ty + py) * w + tx + pxx) * 4) as usize;
                let v = ((pxx + py) % 3) as i16 * 8;
                px[i] = (c[0] as i16 + v).clamp(0, 255) as u8;
                px[i + 1] = (c[1] as i16 + v).clamp(0, 255) as u8;
                px[i + 2] = (c[2] as i16 - v).clamp(0, 255) as u8;
                px[i + 3] = 255;
            }
        }
    }
    encode_png(&px, w, h)
}

fn encode_png(pixels: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let enc = image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut buf));
        use image::ImageEncoder;
        enc.write_image(pixels, w, h, image::ExtendedColorType::Rgba8).unwrap();
    }
    buf
}

fn draw_label(ren: &mut Renderer2D, font: FontHandle, text: &str, x: f32, y: f32) {
    ren.draw_text(text, &TextParams {
        font,
        size: 14.0,
        color: Color::new(0.7, 0.8, 0.9, 1.0),
        align: TextAlign::Left,
        position: Vec2::new(x, y),
        max_width: None,
        line_height: None,
        z: 0,
        letter_spacing: None,
        underline: false,
        strikethrough: false,
    });
}

fn section_bg(ren: &mut Renderer2D, x: f32, y: f32, w: f32, h: f32) {
    ren.draw_rect(
        Rect { pos: Vec2::new(x, y), size: Vec2::new(w, h) },
        DrawParams {
            style: DrawStyle::Fill(Color::new(0.12, 0.12, 0.18, 0.85)),
            blend: BlendMode::Alpha, z_index: -1, opacity: 1.0,
        },
    );
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = WindowAttributes::default()
            .with_title("all 15 new features")
            .with_inner_size(winit::dpi::LogicalSize::new(1200, 900));
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::new(0.08, 0.08, 0.12, 1.0));

        // load font
        let font_data = std::fs::read("NotoSansMeroitic-Regular.ttf").expect("read font file");
        let font = ren.load_font(&font_data).expect("load font");
        self.font = Some(font);

        // load tileset
        let ts_data = make_tileset_png();
        let ts_tex = ren.load_texture(&ts_data, TextureParams::default()).expect("load tileset");
        self.tileset_tex = Some(ts_tex);

        // create render target for blur/bloom demo
        let rt = ren.create_render_target(200, 120).expect("create rt");
        self.rt = Some(rt);

        self.cam = Camera2D::new(1200.0, 900.0);
        // center camera so (0,0) is top-left
        self.cam.position = Vec2::new(600.0, 450.0);
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
                self.time += dt;

                // update trail with circular motion inside row 3 right section
                let tx = 985.0 + (self.time * 2.0).cos() * 70.0;
                let ty = 470.0 + (self.time * 2.0).sin() * 50.0;
                // only add a point every few frames to get a smoother trail
                self.trail.add_point(Vec2::new(tx, ty));
                self.trail.update(dt);

                // update camera shake demo (not applied to main view — just tracked)
                if (self.time % 5.0) < dt {
                    self.cam.shake(4.0, 0.3);
                }
                self.cam.update(dt);

                let ren = self.renderer.as_mut().unwrap();
                ren.set_camera(&self.cam);
                ren.begin_frame().unwrap();

                let font = self.font.unwrap();

                // ═══════════════════════════════════════════
                // ROW 1 — features 1, 3, 4
                // ═══════════════════════════════════════════

                // -- feat 1: multi-line text with word wrap --
                section_bg(ren, 10.0, 10.0, 370.0, 175.0);
                draw_label(ren, font, "1. multi-line text + word wrap", 15.0, 14.0);
                let wrapped = TextParams {
                    font,
                    size: 18.0,
                    color: Color::WHITE,
                    align: TextAlign::Left,
                    position: Vec2::new(20.0, 35.0),
                    max_width: Some(340.0),
                    line_height: Some(22.0),
                    z: 0,
                    letter_spacing: None,
                    underline: false,
                    strikethrough: false,
                };
                ren.draw_text("this text wraps automatically when it exceeds the max_width. word boundaries are respected, and newlines\nwork too!", &wrapped);

                // centered wrap
                let centered = TextParams {
                    font,
                    size: 16.0,
                    color: Color::new(0.5, 1.0, 0.7, 1.0),
                    align: TextAlign::Center,
                    position: Vec2::new(20.0, 130.0),
                    max_width: Some(340.0),
                    line_height: Some(20.0),
                    z: 0,
                    letter_spacing: None,
                    underline: false,
                    strikethrough: false,
                };
                ren.draw_text("center-aligned multi-line text within max_width", &centered);

                // -- feat 3: per-corner rounded rects --
                section_bg(ren, 390.0, 10.0, 380.0, 175.0);
                draw_label(ren, font, "3. per-corner rounded rects", 395.0, 14.0);
                // uniform radius
                ren.draw_rounded_rect(
                    RoundedRect::new(Rect { pos: Vec2::new(400.0, 35.0), size: Vec2::new(100.0, 55.0) }, 12.0),
                    DrawParams::fill(Color::new(0.3, 0.6, 0.9, 1.0)),
                );
                // per-corner: only top corners rounded
                ren.draw_rounded_rect(
                    RoundedRect::with_radii(Rect { pos: Vec2::new(520.0, 35.0), size: Vec2::new(100.0, 55.0) }, 20.0, 20.0, 0.0, 0.0),
                    DrawParams::fill(Color::new(0.9, 0.4, 0.3, 1.0)),
                );
                // per-corner: diagonal
                ren.draw_rounded_rect(
                    RoundedRect::with_radii(Rect { pos: Vec2::new(640.0, 35.0), size: Vec2::new(100.0, 55.0) }, 25.0, 0.0, 25.0, 0.0),
                    DrawParams::fill(Color::new(0.4, 0.8, 0.4, 1.0)),
                );
                // stroked per-corner
                ren.draw_rounded_rect(
                    RoundedRect::with_radii(Rect { pos: Vec2::new(400.0, 105.0), size: Vec2::new(350.0, 55.0) }, 30.0, 5.0, 5.0, 30.0),
                    DrawParams::stroke(Color::new(1.0, 0.8, 0.3, 1.0), 2.5),
                );

                // -- feat 4: color helpers --
                section_bg(ren, 780.0, 10.0, 410.0, 175.0);
                draw_label(ren, font, "4. color helpers (hex, hsl, hsv, srgb)", 785.0, 14.0);
                // draw swatches from hex
                let hex_colors = [0xFF0000, 0x00FF00, 0x0000FF, 0xFF8800, 0x8800FF, 0x00FFFF];
                for (i, &hex) in hex_colors.iter().enumerate() {
                    let c = Color::from_hex(hex);
                    ren.draw_rect(
                        Rect { pos: Vec2::new(790.0 + i as f32 * 38.0, 35.0), size: Vec2::new(32.0, 32.0) },
                        DrawParams::fill(c),
                    );
                }
                // hsl rainbow
                for i in 0..16 {
                    let c = Color::hsl(i as f32 * 22.5, 0.8, 0.5);
                    ren.draw_rect(
                        Rect { pos: Vec2::new(790.0 + i as f32 * 24.0, 75.0), size: Vec2::new(20.0, 20.0) },
                        DrawParams::fill(c),
                    );
                }
                // hsv saturation ramp
                for i in 0..12 {
                    let c = Color::hsv(200.0, i as f32 / 11.0, 1.0);
                    ren.draw_rect(
                        Rect { pos: Vec2::new(790.0 + i as f32 * 32.0, 105.0), size: Vec2::new(28.0, 20.0) },
                        DrawParams::fill(c),
                    );
                }
                // srgb demo
                let srgb = Color::from_srgb(0.5, 0.5, 0.5, 1.0);
                ren.draw_rect(
                    Rect { pos: Vec2::new(790.0, 135.0), size: Vec2::new(60.0, 30.0) },
                    DrawParams::fill(srgb),
                );
                draw_label(ren, font, "srgb 50%", 855.0, 143.0);

                // ═══════════════════════════════════════════
                // ROW 2 — features 5, 6, 7
                // ═══════════════════════════════════════════
                let row2_y = 195.0;

                // -- feat 5: multi-stop gradients --
                section_bg(ren, 10.0, row2_y, 370.0, 175.0);
                draw_label(ren, font, "5. multi-stop gradients", 15.0, row2_y + 4.0);
                // linear 4-stop rainbow
                ren.draw_rect(
                    Rect { pos: Vec2::new(20.0, row2_y + 25.0), size: Vec2::new(340.0, 40.0) },
                    DrawParams {
                        style: DrawStyle::LinearGradientStops {
                            start: Vec2::new(20.0, 0.0),
                            end: Vec2::new(360.0, 0.0),
                            stops: vec![
                                GradientStop { offset: 0.0, color: Color::RED },
                                GradientStop { offset: 0.33, color: Color::new(1.0, 1.0, 0.0, 1.0) },
                                GradientStop { offset: 0.66, color: Color::GREEN },
                                GradientStop { offset: 1.0, color: Color::BLUE },
                            ],
                        },
                        blend: BlendMode::Alpha, z_index: 0, opacity: 1.0,
                    },
                );
                // radial multi-stop
                ren.draw_circle(
                    Vec2::new(100.0, row2_y + 120.0), 50.0,
                    DrawParams {
                        style: DrawStyle::RadialGradientStops {
                            center: Vec2::new(100.0, row2_y + 120.0),
                            radius: 50.0,
                            stops: vec![
                                GradientStop { offset: 0.0, color: Color::WHITE },
                                GradientStop { offset: 0.4, color: Color::new(1.0, 0.5, 0.0, 1.0) },
                                GradientStop { offset: 0.8, color: Color::RED },
                                GradientStop { offset: 1.0, color: Color::new(0.2, 0.0, 0.0, 1.0) },
                            ],
                        },
                        blend: BlendMode::Alpha, z_index: 0, opacity: 1.0,
                    },
                );
                // another linear: cool-to-warm
                ren.draw_rect(
                    Rect { pos: Vec2::new(170.0, row2_y + 80.0), size: Vec2::new(190.0, 80.0) },
                    DrawParams {
                        style: DrawStyle::LinearGradientStops {
                            start: Vec2::new(170.0, row2_y + 80.0),
                            end: Vec2::new(170.0, row2_y + 160.0),
                            stops: vec![
                                GradientStop { offset: 0.0, color: Color::new(0.1, 0.3, 0.8, 1.0) },
                                GradientStop { offset: 0.5, color: Color::new(0.9, 0.9, 0.9, 1.0) },
                                GradientStop { offset: 1.0, color: Color::new(0.8, 0.2, 0.1, 1.0) },
                            ],
                        },
                        blend: BlendMode::Alpha, z_index: 0, opacity: 1.0,
                    },
                );

                // -- feat 6: more blend modes --
                section_bg(ren, 390.0, row2_y, 380.0, 175.0);
                draw_label(ren, font, "6. blend modes: Screen, PremultAlpha", 395.0, row2_y + 4.0);
                // base rects
                let bx = 400.0;
                let by = row2_y + 30.0;
                ren.draw_rect(Rect { pos: Vec2::new(bx, by), size: Vec2::new(70.0, 70.0) }, DrawParams::fill(Color::new(0.8, 0.2, 0.2, 1.0)));
                // screen blend on top
                ren.draw_rect(
                    Rect { pos: Vec2::new(bx + 25.0, by + 25.0), size: Vec2::new(70.0, 70.0) },
                    DrawParams::fill(Color::new(0.2, 0.2, 0.8, 0.7)).with_blend(BlendMode::Screen),
                );
                draw_label(ren, font, "screen", bx + 20.0, by + 80.0);

                // additive
                ren.draw_rect(Rect { pos: Vec2::new(bx + 130.0, by), size: Vec2::new(70.0, 70.0) }, DrawParams::fill(Color::new(0.8, 0.2, 0.2, 1.0)));
                ren.draw_rect(
                    Rect { pos: Vec2::new(bx + 155.0, by + 25.0), size: Vec2::new(70.0, 70.0) },
                    DrawParams::fill(Color::new(0.2, 0.2, 0.8, 0.7)).with_blend(BlendMode::Additive),
                );
                draw_label(ren, font, "additive", bx + 145.0, by + 80.0);

                // multiply
                ren.draw_rect(Rect { pos: Vec2::new(bx + 260.0, by), size: Vec2::new(70.0, 70.0) }, DrawParams::fill(Color::new(0.9, 0.8, 0.3, 1.0)));
                ren.draw_rect(
                    Rect { pos: Vec2::new(bx + 285.0, by + 25.0), size: Vec2::new(70.0, 70.0) },
                    DrawParams::fill(Color::new(0.3, 0.8, 0.9, 0.7)).with_blend(BlendMode::Multiply),
                );
                draw_label(ren, font, "multiply", bx + 275.0, by + 80.0);

                // -- feat 7: collision helpers (visual demo) --
                section_bg(ren, 780.0, row2_y, 410.0, 175.0);
                draw_label(ren, font, "7. collision helpers", 785.0, row2_y + 4.0);
                // rect containment test
                let test_rect = Rect { pos: Vec2::new(800.0, row2_y + 30.0), size: Vec2::new(120.0, 80.0) };
                ren.draw_rect(test_rect, DrawParams::stroke(Color::new(0.5, 0.8, 0.5, 1.0), 1.5));
                // animate a test point
                let px = 860.0 + (self.time * 1.5).cos() * 80.0;
                let py = row2_y + 70.0 + (self.time * 1.2).sin() * 50.0;
                let inside = test_rect.contains(Vec2::new(px, py));
                let dot_col = if inside { Color::GREEN } else { Color::RED };
                ren.draw_circle(Vec2::new(px, py), 5.0, DrawParams::fill(dot_col));
                draw_label(ren, font, if inside { "inside!" } else { "outside" }, 800.0, row2_y + 120.0);

                // circle-rect intersection
                let cr_cx = 1030.0;
                let cr_cy = row2_y + 70.0;
                let cr_r = 30.0 + (self.time * 0.8).sin() * 15.0;
                let cr_rect = Rect { pos: Vec2::new(1020.0, row2_y + 40.0), size: Vec2::new(80.0, 60.0) };
                let cr_hit = lite_render_2d_core::circle_intersects_rect(Vec2::new(cr_cx, cr_cy), cr_r, &cr_rect);
                let cr_col = if cr_hit { Color::new(1.0, 0.3, 0.3, 0.5) } else { Color::new(0.3, 0.3, 1.0, 0.3) };
                ren.draw_rect(cr_rect, DrawParams::stroke(Color::new(0.8, 0.8, 0.8, 0.8), 1.0));
                ren.draw_circle(Vec2::new(cr_cx, cr_cy), cr_r, DrawParams::fill(cr_col));
                draw_label(ren, font, if cr_hit { "hit" } else { "miss" }, 1040.0, row2_y + 120.0);

                // ═══════════════════════════════════════════
                // ROW 3 — features 2, 8, 9
                // ═══════════════════════════════════════════
                let row3_y = 380.0;

                // -- feat 2: blur/bloom (render-to-texture + effect) --
                section_bg(ren, 10.0, row3_y, 370.0, 175.0);
                draw_label(ren, font, "2. blur/bloom post-processing", 15.0, row3_y + 4.0);
                if let Some(rt) = self.rt {
                    // draw some bright shapes into the render target
                    ren.begin_render_to_texture(rt).unwrap();
                    ren.draw_rect(
                        Rect { pos: Vec2::new(10.0, 10.0), size: Vec2::new(180.0, 100.0) },
                        DrawParams::fill(Color::new(0.05, 0.05, 0.1, 1.0)),
                    );
                    ren.draw_circle(Vec2::new(60.0, 60.0), 25.0, DrawParams::fill(Color::new(1.0, 0.4, 0.1, 1.0)));
                    ren.draw_circle(Vec2::new(140.0, 60.0), 20.0, DrawParams::fill(Color::new(0.2, 0.6, 1.0, 1.0)));
                    ren.draw_rect(Rect { pos: Vec2::new(85.0, 30.0), size: Vec2::new(30.0, 60.0) }, DrawParams::fill(Color::new(0.1, 1.0, 0.3, 1.0)));
                    ren.end_render_to_texture();

                    // draw the original (unblurred) as sprite
                    if let Some(tex) = ren.render_target_texture(rt) {
                        ren.draw_sprite(tex, lite_render_2d_core::SpriteParams {
                            transform: Transform2D { pos: Vec2::new(20.0, row3_y + 25.0), scale: Vec2::new(1.6, 1.2), rotation: 0.0 },
                            tint: Color::WHITE, src_rect: None, flip_x: false, flip_y: false,
                            blend: BlendMode::Alpha, z_index: 0, opacity: 1.0,
                        });
                    }
                    draw_label(ren, font, "(render target content)", 20.0, row3_y + 155.0);
                }

                // -- feat 8: stencil masking --
                section_bg(ren, 390.0, row3_y, 380.0, 175.0);
                draw_label(ren, font, "8. stencil masking", 395.0, row3_y + 4.0);
                // draw a circle-masked rect (the stencil clips to circle shape)
                ren.begin_stencil_write();
                ren.draw_circle(Vec2::new(530.0, row3_y + 95.0), 55.0, DrawParams::fill(Color::WHITE));
                ren.end_stencil_write();
                // this gradient rect gets clipped to the circle stencil
                ren.draw_rect(
                    Rect { pos: Vec2::new(440.0, row3_y + 30.0), size: Vec2::new(180.0, 130.0) },
                    DrawParams {
                        style: DrawStyle::LinearGradient {
                            start: Vec2::new(440.0, row3_y + 30.0),
                            end: Vec2::new(620.0, row3_y + 160.0),
                            color_start: Color::new(1.0, 0.3, 0.1, 1.0),
                            color_end: Color::new(0.1, 0.3, 1.0, 1.0),
                        },
                        blend: BlendMode::Alpha, z_index: 0, opacity: 1.0,
                    },
                );
                ren.pop_stencil_mask();

                // draw circle outline for reference
                ren.draw_circle(Vec2::new(530.0, row3_y + 95.0), 55.0, DrawParams::stroke(Color::new(1.0, 1.0, 1.0, 0.5), 1.5));
                draw_label(ren, font, "gradient clipped to circle", 640.0, row3_y + 80.0);

                // -- feat 9: trail / ribbon renderer --
                section_bg(ren, 780.0, row3_y, 410.0, 175.0);
                draw_label(ren, font, "9. trail / ribbon renderer", 785.0, row3_y + 4.0);
                self.trail.draw(ren);
                // draw the trail head
                ren.draw_circle(Vec2::new(tx, ty), 6.0, DrawParams::fill(Color::WHITE));

                // ═══════════════════════════════════════════
                // ROW 4 — features 10, 11, 12
                // ═══════════════════════════════════════════
                let row4_y = 565.0;

                // -- feat 10: read_pixels (render-to-image) --
                section_bg(ren, 10.0, row4_y, 370.0, 155.0);
                draw_label(ren, font, "10. render-to-image (read_pixels)", 15.0, row4_y + 4.0);
                draw_label(ren, font, "read_pixels() reads RGBA8 from", 20.0, row4_y + 30.0);
                draw_label(ren, font, "any render target for CPU access", 20.0, row4_y + 50.0);
                draw_label(ren, font, "use for screenshots or image export", 20.0, row4_y + 70.0);
                // show a small render target readback demo
                if let Some(rt) = self.rt {
                    match ren.read_pixels(rt) {
                        Ok(data) => {
                            let pixel_count = data.len() / 4;
                            let msg = format!("OK: {} pixels read ({} bytes)", pixel_count, data.len());
                            draw_label(ren, font, &msg, 20.0, row4_y + 100.0);
                        }
                        Err(e) => {
                            let msg = format!("err: {}", e);
                            draw_label(ren, font, &msg, 20.0, row4_y + 100.0);
                        }
                    }
                }

                // -- feat 11: custom shader / material system --
                section_bg(ren, 390.0, row4_y, 380.0, 155.0);
                draw_label(ren, font, "11. custom shader / material system", 395.0, row4_y + 4.0);
                draw_label(ren, font, "create_material(frag_src) compiles", 400.0, row4_y + 30.0);
                draw_label(ren, font, "custom fragment shaders for sprites.", 400.0, row4_y + 50.0);
                draw_label(ren, font, "set uniforms per-draw call.", 400.0, row4_y + 70.0);
                draw_label(ren, font, "enables water, dissolve, outlines...", 400.0, row4_y + 90.0);

                // -- feat 12: texture atlas packing --
                section_bg(ren, 780.0, row4_y, 410.0, 155.0);
                draw_label(ren, font, "12. texture atlas packing", 785.0, row4_y + 4.0);
                // demo the atlas packing
                let mut atlas = lite_render_2d_core::TextureAtlas::new(256, 256);
                // pack some fake images
                let img1 = vec![255u8; 32 * 32 * 4]; // white 32x32
                let img2 = vec![128u8; 16 * 48 * 4]; // gray 16x48
                let img3 = vec![200u8; 64 * 16 * 4]; // light 64x16
                let r1 = atlas.add_image(&img1, 32, 32);
                let r2 = atlas.add_image(&img2, 16, 48);
                let r3 = atlas.add_image(&img3, 64, 16);
                // show region info
                if let Some(r) = r1 {
                    draw_label(ren, font, &format!("region 1: {}x{} at ({},{})", r.width, r.height, r.x, r.y), 790.0, row4_y + 30.0);
                }
                if let Some(r) = r2 {
                    draw_label(ren, font, &format!("region 2: {}x{} at ({},{})", r.width, r.height, r.x, r.y), 790.0, row4_y + 50.0);
                }
                if let Some(r) = r3 {
                    draw_label(ren, font, &format!("region 3: {}x{} at ({},{})", r.width, r.height, r.x, r.y), 790.0, row4_y + 70.0);
                }
                draw_label(ren, font, &format!("atlas: {} regions packed in 256x256", atlas.region_count()), 790.0, row4_y + 100.0);

                // ═══════════════════════════════════════════
                // ROW 5 — features 13, 14, 15
                // ═══════════════════════════════════════════
                let row5_y = 730.0;

                // -- feat 13: bitmap font support --
                section_bg(ren, 10.0, row5_y, 370.0, 155.0);
                draw_label(ren, font, "13. bitmap font support", 15.0, row5_y + 4.0);
                draw_label(ren, font, "BitmapFont::from_grid() for pixel fonts", 20.0, row5_y + 30.0);
                draw_label(ren, font, "set_glyph() for custom metrics", 20.0, row5_y + 50.0);
                draw_label(ren, font, "measure() + layout() for rendering", 20.0, row5_y + 70.0);
                draw_label(ren, font, "renders via sprite pipeline (no atlas)", 20.0, row5_y + 90.0);

                // -- feat 14: tilemap improvements --
                section_bg(ren, 390.0, row5_y, 380.0, 155.0);
                draw_label(ren, font, "14. tilemap: layers, anim, flip, iso", 395.0, row5_y + 4.0);
                if let Some(ts_tex) = self.tileset_tex {
                    let tileset = TilesetInfo { tile_width: 16.0, tile_height: 16.0, columns: 4 };
                    let mut tmap = Tilemap::new(6, 4, 20.0, ts_tex, tileset);

                    // layer 0: ground
                    for y in 0..4u32 {
                        for x in 0..6u32 {
                            tmap.set_tile(x, y, 1); // grass
                        }
                    }
                    // layer 1: details
                    tmap.add_layer();
                    tmap.set_tile_layer(1, 2, 1, 3); // water
                    tmap.set_tile_layer(1, 3, 1, 3);
                    tmap.set_tile_layer(1, 0, 0, 4); // stone
                    // flipped tile
                    tmap.set_tile_layer(1, 5, 3, 7 | lite_render_2d_core::TILE_FLIP_H);

                    // animated tile
                    tmap.add_animated_tile(3, AnimatedTile { frames: vec![3, 6, 3, 5], frame_duration: 0.4 });
                    tmap.update(self.time);

                    ren.draw_tilemap(&tmap, Vec2::new(400.0, row5_y + 25.0), 0);
                    draw_label(ren, font, "2 layers + animated water + flip", 400.0, row5_y + 120.0);
                }

                // -- feat 15: camera shake + smoothing --
                section_bg(ren, 780.0, row5_y, 410.0, 155.0);
                draw_label(ren, font, "15. camera shake + smoothing", 785.0, row5_y + 4.0);
                draw_label(ren, font, "shake triggers every 5s — watch!", 790.0, row5_y + 30.0);
                draw_label(ren, font, "cam.shake(intensity, duration)", 790.0, row5_y + 55.0);
                draw_label(ren, font, "cam.follow(target, smoothing, dt)", 790.0, row5_y + 75.0);
                draw_label(ren, font, "cam.update(dt) steps the decay", 790.0, row5_y + 95.0);
                // visual indicator of shake
                let bar_w = 5.0_f32.min(self.time % 5.0);
                let shake_active = (self.time % 5.0) < 0.3;
                let ind_col = if shake_active { Color::new(1.0, 0.3, 0.1, 1.0) } else { Color::new(0.3, 0.6, 0.3, 1.0) };
                ren.draw_rect(
                    Rect { pos: Vec2::new(790.0, row5_y + 120.0), size: Vec2::new(bar_w * 76.0, 12.0) },
                    DrawParams::fill(ind_col),
                );
                draw_label(ren, font, if shake_active { "SHAKING" } else { "calm" }, 1090.0, row5_y + 120.0);

                // -- polyline border around entire window for visual flair --
                ren.draw_polyline(
                    &[Vec2::new(5.0, 5.0), Vec2::new(1195.0, 5.0), Vec2::new(1195.0, 895.0), Vec2::new(5.0, 895.0), Vec2::new(5.0, 5.0)],
                    LineParams {
                        thickness: 1.0,
                        color: Color::new(0.3, 0.4, 0.5, 0.5),
                        cap: lite_render_2d_core::LineCap::Butt,
                        join: lite_render_2d_core::LineJoin::Miter,
                        style: StrokeStyle::Solid,
                        blend: BlendMode::Alpha,
                        z_index: 10,
                        opacity: 1.0,
                    },
                );

                ren.end_frame().unwrap();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        window: None,
        renderer: None,
        font: None,
        tileset_tex: None,
        rt: None,
        trail: TrailRenderer::new(60, 10.0, 1.5),
        cam: Camera2D::new(1200.0, 900.0),
        time: 0.0,
        last_time: std::time::Instant::now(),
    };
    // configure trail colors
    app.trail.color_start = Color::new(1.0, 0.6, 0.1, 1.0);
    app.trail.color_end = Color::new(1.0, 0.1, 0.0, 0.0);
    event_loop.run_app(&mut app).unwrap();
}
