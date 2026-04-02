use crate::types::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
    pub viewport: Vec2,
}

impl Camera2D {
    pub fn new(viewport_w: f32, viewport_h: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            viewport: Vec2::new(viewport_w, viewport_h),
        }
    }

    // ortho proj, ignroe z
    pub fn projection_matrix(&self) -> [f32; 16] {
        let hw = self.viewport.x / (2.0 * self.zoom);
        let hh = self.viewport.y / (2.0 * self.zoom);

        let left = self.position.x - hw;
        let right = self.position.x + hw;
        let bottom = self.position.y - hh;
        let top = self.position.y + hh;

        let tx = -(right + left) / (right - left);
        let ty = -(top + bottom) / (top - bottom);

        [
            2.0 / (right - left), 0.0,                  0.0, 0.0,
            0.0,                  2.0 / (top - bottom),  0.0, 0.0,
            0.0,                  0.0,                  -1.0, 0.0,
            tx,                   ty,                    0.0, 1.0,
        ]
    }

    // screen px -> world coords
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        let hw = self.viewport.x / (2.0 * self.zoom);
        let hh = self.viewport.y / (2.0 * self.zoom);

        Vec2::new(
            self.position.x + (screen_pos.x / self.viewport.x - 0.5) * 2.0 * hw,
            self.position.y + (screen_pos.y / self.viewport.y - 0.5) * 2.0 * hh,
        )
    }

    // world coords -> screen px
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        let hw = self.viewport.x / (2.0 * self.zoom);
        let hh = self.viewport.y / (2.0 * self.zoom);

        Vec2::new(
            ((world_pos.x - self.position.x) / (2.0 * hw) + 0.5) * self.viewport.x,
            ((world_pos.y - self.position.y) / (2.0 * hh) + 0.5) * self.viewport.y,
        )
    }
}
