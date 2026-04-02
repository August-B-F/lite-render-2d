use crate::types::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
    pub viewport: Vec2,
    // shake state
    shake_intensity: f32,
    shake_duration: f32,
    shake_elapsed: f32,
    shake_offset: Vec2,
    shake_seed: u32,
}

impl Camera2D {
    pub fn new(viewport_w: f32, viewport_h: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            viewport: Vec2::new(viewport_w, viewport_h),
            shake_intensity: 0.0,
            shake_duration: 0.0,
            shake_elapsed: 0.0,
            shake_offset: Vec2::ZERO,
            shake_seed: 42,
        }
    }

    // start a screen shake that decays over duration
    pub fn shake(&mut self, intensity: f32, duration: f32) {
        self.shake_intensity = intensity;
        self.shake_duration = duration;
        self.shake_elapsed = 0.0;
    }

    // smoothly lerp camera toward target position
    pub fn follow(&mut self, target: Vec2, smoothing: f32, dt: f32) {
        let t = (smoothing * dt).min(1.0);
        self.position.x += (target.x - self.position.x) * t;
        self.position.y += (target.y - self.position.y) * t;
    }

    // step shake decay and update offset — call each frame
    pub fn update(&mut self, dt: f32) {
        if self.shake_elapsed < self.shake_duration {
            self.shake_elapsed += dt;
            let decay = 1.0 - (self.shake_elapsed / self.shake_duration).min(1.0);
            let mag = self.shake_intensity * decay;
            // cheap prng for shake offsets
            self.shake_seed ^= self.shake_seed << 13;
            self.shake_seed ^= self.shake_seed >> 17;
            self.shake_seed ^= self.shake_seed << 5;
            let rx = (self.shake_seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            self.shake_seed ^= self.shake_seed << 13;
            self.shake_seed ^= self.shake_seed >> 17;
            self.shake_seed ^= self.shake_seed << 5;
            let ry = (self.shake_seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            self.shake_offset = Vec2::new(rx * mag, ry * mag);
        } else {
            self.shake_offset = Vec2::ZERO;
        }
    }

    /// Orthographic projection matrix (y-down, matching screen coordinates).
    pub fn projection_matrix(&self) -> [f32; 16] {
        let hw = self.viewport.x / (2.0 * self.zoom);
        let hh = self.viewport.y / (2.0 * self.zoom);

        // apply shake offset to effective positon
        let px = self.position.x + self.shake_offset.x;
        let py = self.position.y + self.shake_offset.y;

        let left = px - hw;
        let right = px + hw;
        // y-down: top < bottom so the y-axis points downward
        let top = py - hh;
        let bottom = py + hh;

        let tx = -(right + left) / (right - left);
        let ty = -(top + bottom) / (top - bottom);

        [
            2.0 / (right - left), 0.0,                  0.0, 0.0,
            0.0,                  2.0 / (top - bottom),  0.0, 0.0,
            0.0,                  0.0,                  -1.0, 0.0,
            tx,                   ty,                    0.0, 1.0,
        ]
    }

    /// Screen pixels -> world coordinates (y-down).
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        let hw = self.viewport.x / (2.0 * self.zoom);
        let hh = self.viewport.y / (2.0 * self.zoom);

        Vec2::new(
            self.position.x + (screen_pos.x / self.viewport.x - 0.5) * 2.0 * hw,
            self.position.y + (screen_pos.y / self.viewport.y - 0.5) * 2.0 * hh,
        )
    }

    /// World coordinates -> screen pixels (y-down).
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        let hw = self.viewport.x / (2.0 * self.zoom);
        let hh = self.viewport.y / (2.0 * self.zoom);

        Vec2::new(
            ((world_pos.x - self.position.x) / (2.0 * hw) + 0.5) * self.viewport.x,
            ((world_pos.y - self.position.y) / (2.0 * hh) + 0.5) * self.viewport.y,
        )
    }
}
