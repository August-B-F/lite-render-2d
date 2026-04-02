use lite_render_2d_core::BlendMode;

// floats per shape vertex (pos2 + local2 + color4 + mode1 + stroke_w1 + size2)
const SHAPE_FLOATS_PER_VERT: usize = 12;
pub(crate) const SHAPE_FLOATS_PER_QUAD: usize = SHAPE_FLOATS_PER_VERT * 6;

// floats per sprite vertex (pos2 + uv2 + tint4 + opacity1)
const SPRITE_FLOATS_PER_VERT: usize = 9;
pub(crate) const SPRITE_FLOATS_PER_QUAD: usize = SPRITE_FLOATS_PER_VERT * 6;

// one draw comand with its gpu state
#[derive(Clone, Copy)]
pub(crate) struct DrawCmd {
    pub kind: CmdKind,
    // float offset into shape_buf or sprite_buf
    pub vert_start: u32,
    // number of floats
    pub vert_len: u32,
    pub z_index: i32,
    pub blend: BlendMode,
    // scissor rect in pixels or none for full viewport
    pub clip: Option<[u32; 4]>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum CmdKind {
    Shape,
    Sprite { texture_id: u64 },
    SdfSprite { texture_id: u64 },
}

pub(crate) struct Batcher {
    pub shape_buf: Vec<f32>,
    pub sprite_buf: Vec<f32>,
    pub commands: Vec<DrawCmd>,
    draw_calls: u32,
}

impl Batcher {
    pub fn new() -> Self {
        Self {
            shape_buf: Vec::with_capacity(SHAPE_FLOATS_PER_QUAD * 256),
            sprite_buf: Vec::with_capacity(SPRITE_FLOATS_PER_QUAD * 64),
            commands: Vec::with_capacity(256),
            draw_calls: 0,
        }
    }

    pub fn clear(&mut self) {
        self.shape_buf.clear();
        self.sprite_buf.clear();
        self.commands.clear();
        self.draw_calls = 0;
    }

    pub fn draw_calls(&self) -> u32 {
        self.draw_calls
    }

    pub fn add_draw_call(&mut self) {
        self.draw_calls += 1;
    }

    pub fn push_shape(
        &mut self,
        verts: &[f32; SHAPE_FLOATS_PER_QUAD],
        z_index: i32,
        blend: BlendMode,
        clip: Option<[u32; 4]>,
    ) {
        let start = self.shape_buf.len() as u32;
        self.shape_buf.extend_from_slice(verts);
        self.commands.push(DrawCmd {
            kind: CmdKind::Shape,
            vert_start: start,
            vert_len: SHAPE_FLOATS_PER_QUAD as u32,
            z_index,
            blend,
            clip,
        });
    }

    pub fn push_shape_raw(
        &mut self,
        verts: &[f32],
        z_index: i32,
        blend: BlendMode,
        clip: Option<[u32; 4]>,
    ) {
        debug_assert!(verts.len().is_multiple_of(SHAPE_FLOATS_PER_VERT));
        let start = self.shape_buf.len() as u32;
        self.shape_buf.extend_from_slice(verts);
        self.commands.push(DrawCmd {
            kind: CmdKind::Shape,
            vert_start: start,
            vert_len: verts.len() as u32,
            z_index,
            blend,
            clip,
        });
    }

    pub fn push_sprite(
        &mut self,
        texture_id: u64,
        verts: &[f32; SPRITE_FLOATS_PER_QUAD],
        z_index: i32,
        blend: BlendMode,
        clip: Option<[u32; 4]>,
    ) {
        let start = self.sprite_buf.len() as u32;
        self.sprite_buf.extend_from_slice(verts);
        self.commands.push(DrawCmd {
            kind: CmdKind::Sprite { texture_id },
            vert_start: start,
            vert_len: SPRITE_FLOATS_PER_QUAD as u32,
            z_index,
            blend,
            clip,
        });
    }

    pub fn push_sdf_sprite(
        &mut self,
        texture_id: u64,
        verts: &[f32; SPRITE_FLOATS_PER_QUAD],
        z_index: i32,
        blend: BlendMode,
        clip: Option<[u32; 4]>,
    ) {
        let start = self.sprite_buf.len() as u32;
        self.sprite_buf.extend_from_slice(verts);
        self.commands.push(DrawCmd {
            kind: CmdKind::SdfSprite { texture_id },
            vert_start: start,
            vert_len: SPRITE_FLOATS_PER_QUAD as u32,
            z_index,
            blend,
            clip,
        });
    }

    // stabble sort so same z keeps submision order
    pub fn sort_commands(&mut self) {
        self.commands.sort_by_key(|cmd| cmd.z_index);
    }
}
