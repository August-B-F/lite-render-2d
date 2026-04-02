//! Interactive example: arrow keys pan the camera, mouse scroll zooms,
//! and a crosshair follows the mouse in world space via screen_to_world.

use std::collections::HashSet;

use lite_render_2d_core::prelude::*;
#[cfg(feature = "use-glow")]
use lite_render_2d_glow::GlowRenderer as Renderer2D;
#[cfg(feature = "use-wgpu")]
use lite_render_2d_wgpu::WgpuRenderer as Renderer2D;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

struct App {
    window: Option<Window>,
    renderer: Option<Renderer2D>,
    camera: Camera2D,
    keys: HashSet<KeyCode>,
    mouse_screen: Vec2,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = WindowAttributes::default()
            .with_title("interactive — arrows: pan, scroll: zoom")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
        let win = event_loop.create_window(attrs).expect("create window");
        let mut ren = Renderer2D::new(&win).expect("create renderer");
        ren.set_clear_color(Color::rgb(0.12, 0.12, 0.18));
        self.camera = Camera2D::new(800.0, 600.0)
            .with_position(Vec2::new(400.0, 300.0));
        self.renderer = Some(ren);
        self.window = Some(win);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(ren) = &mut self.renderer { ren.resize(size.width, size.height); }
                self.camera.viewport = Vec2::new(size.width as f32, size.height as f32);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => { self.keys.insert(code); }
                        ElementState::Released => { self.keys.remove(&code); }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_screen = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
                };
                let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
                self.camera.zoom = (self.camera.zoom * factor).clamp(0.1, 10.0);
            }
            WindowEvent::RedrawRequested => {
                let Some(ren) = &mut self.renderer else { return; };

                // pan camera with arrow keys
                let speed = 4.0 / self.camera.zoom;
                if self.keys.contains(&KeyCode::ArrowLeft)  { self.camera.position.x -= speed; }
                if self.keys.contains(&KeyCode::ArrowRight) { self.camera.position.x += speed; }
                if self.keys.contains(&KeyCode::ArrowUp)    { self.camera.position.y -= speed; }
                if self.keys.contains(&KeyCode::ArrowDown)  { self.camera.position.y += speed; }

                ren.begin_frame().expect("begin frame");
                ren.set_camera(&self.camera);

                // draw a grid of rects so camera movement is visible
                for row in 0..10 {
                    for col in 0..10 {
                        let x = col as f32 * 100.0;
                        let y = row as f32 * 80.0;
                        let color = if (row + col) % 2 == 0 { Color::rgb(0.25, 0.25, 0.35) } else { Color::rgb(0.18, 0.18, 0.28) };
                        ren.draw_rect(Rect::new(x, y, 98.0, 78.0), DrawParams::fill(color));
                    }
                }

                // crosshair at mouse position in world space
                let mouse_world = self.camera.screen_to_world(self.mouse_screen);
                let arm = 12.0;
                ren.draw_line(
                    mouse_world - Vec2::new(arm, 0.0),
                    mouse_world + Vec2::new(arm, 0.0),
                    LineParams::new(Color::YELLOW, 2.0),
                );
                ren.draw_line(
                    mouse_world - Vec2::new(0.0, arm),
                    mouse_world + Vec2::new(0.0, arm),
                    LineParams::new(Color::YELLOW, 2.0),
                );

                ren.end_frame().expect("end frame");
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
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App {
        window: None,
        renderer: None,
        camera: Camera2D::new(800.0, 600.0),
        keys: HashSet::new(),
        mouse_screen: Vec2::ZERO,
    };
    event_loop.run_app(&mut app).expect("run");
}
