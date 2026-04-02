use crate::renderer::Renderer;
use crate::texture::TextureHandle;
use crate::types::{Color, DrawParams, Vec2};

pub struct TrailPoint {
    pub position: Vec2,
    pub age: f32,
}

pub struct TrailRenderer {
    pub points: Vec<TrailPoint>,
    pub max_points: usize,
    pub width: f32,
    pub lifetime: f32,
    pub color_start: Color,
    pub color_end: Color,
    pub texture: Option<TextureHandle>,
}

impl TrailRenderer {
    pub fn new(max_points: usize, width: f32, lifetime: f32) -> Self {
        Self {
            points: Vec::with_capacity(max_points),
            max_points,
            width,
            lifetime,
            color_start: Color::WHITE,
            color_end: Color::new(1.0, 1.0, 1.0, 0.0),
            texture: None,
        }
    }

    // add a new point, evict oldest if over max
    // skips if too close to the last point to avoid degenrate geometry
    pub fn add_point(&mut self, pos: Vec2) {
        if let Some(last) = self.points.last() {
            let dx = pos.x - last.position.x;
            let dy = pos.y - last.position.y;
            if dx * dx + dy * dy < 1.0 {
                return; // too close, skip
            }
        }
        self.points.push(TrailPoint { position: pos, age: 0.0 });
        while self.points.len() > self.max_points {
            self.points.remove(0);
        }
    }

    // age all points and remove expired ones
    pub fn update(&mut self, dt: f32) {
        for p in &mut self.points {
            p.age += dt;
        }
        self.points.retain(|p| p.age < self.lifetime);
    }

    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    // draw the trail as a series of quads forming a ribbon
    pub fn draw(&self, renderer: &mut dyn Renderer) {
        if self.points.len() < 2 {
            return;
        }

        let n = self.points.len();
        for i in 0..n - 1 {
            let p0 = &self.points[i];
            let p1 = &self.points[i + 1];

            let t0 = (p0.age / self.lifetime).clamp(0.0, 1.0);
            let t1 = (p1.age / self.lifetime).clamp(0.0, 1.0);

            let c0 = self.color_start.lerp(self.color_end, t0);
            let c1 = self.color_start.lerp(self.color_end, t1);
            // avg color for the quad segment
            let avg = Color::new(
                (c0.r + c1.r) * 0.5,
                (c0.g + c1.g) * 0.5,
                (c0.b + c1.b) * 0.5,
                (c0.a + c1.a) * 0.5,
            );

            let w0 = self.width * (1.0 - t0);
            let w1 = self.width * (1.0 - t1);
            if w0 < 0.5 && w1 < 0.5 { continue; } // skip nearly invisible segments

            // perpendiculr to segment direction
            let dx = p1.position.x - p0.position.x;
            let dy = p1.position.y - p0.position.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.001 { continue; }
            let nx = -dy / len;
            let ny = dx / len;

            let a = Vec2::new(p0.position.x + nx * w0 * 0.5, p0.position.y + ny * w0 * 0.5);
            let b = Vec2::new(p0.position.x - nx * w0 * 0.5, p0.position.y - ny * w0 * 0.5);
            let c = Vec2::new(p1.position.x - nx * w1 * 0.5, p1.position.y - ny * w1 * 0.5);
            let d = Vec2::new(p1.position.x + nx * w1 * 0.5, p1.position.y + ny * w1 * 0.5);

            renderer.draw_polygon(&[a, b, c, d], DrawParams::fill(avg));
        }
    }
}
