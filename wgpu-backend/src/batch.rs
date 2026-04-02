use std::collections::BTreeMap;

// floats per shape vertex (pos2 + local2 + color4 + mode1 + stroke_w1 + size2)
const SHAPE_FLOATS_PER_VERT: usize = 12;
pub(crate) const SHAPE_FLOATS_PER_QUAD: usize = SHAPE_FLOATS_PER_VERT * 6;

// floats per sprite vertex (pos2 + uv2 + tint4 + opacity1)
const SPRITE_FLOATS_PER_VERT: usize = 9;
pub(crate) const SPRITE_FLOATS_PER_QUAD: usize = SPRITE_FLOATS_PER_VERT * 6;

pub(crate) struct Batcher {
    pub shape_buf: Vec<f32>,
    pub sprite_runs: BTreeMap<u64, Vec<f32>>,
    draw_calls: u32,
}

impl Batcher {
    pub fn new() -> Self {
        Self {
            shape_buf: Vec::with_capacity(SHAPE_FLOATS_PER_QUAD * 256),
            sprite_runs: BTreeMap::new(),
            draw_calls: 0,
        }
    }

    pub fn clear(&mut self) {
        self.shape_buf.clear();
        for buf in self.sprite_runs.values_mut() {
            buf.clear();
        }
        self.draw_calls = 0;
    }

    pub fn draw_calls(&self) -> u32 {
        self.draw_calls
    }

    pub fn add_draw_call(&mut self) {
        self.draw_calls += 1;
    }

    pub fn push_shape(&mut self, verts: &[f32; SHAPE_FLOATS_PER_QUAD]) {
        self.shape_buf.extend_from_slice(verts);
    }

    pub fn push_sprite(&mut self, texture_id: u64, verts: &[f32; SPRITE_FLOATS_PER_QUAD]) {
        self.sprite_runs
            .entry(texture_id)
            .or_insert_with(|| Vec::with_capacity(SPRITE_FLOATS_PER_QUAD * 64))
            .extend_from_slice(verts);
    }
}
