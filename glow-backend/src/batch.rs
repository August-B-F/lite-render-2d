use std::collections::HashMap;

use glow::HasContext;
use lite_render_2d_core::BlendMode;

// bytes per shape vertex (pos2*f32 + local2*f32 + color4*u8x4 + mode1*f32 + stroke_w1*f32 + size2*f32 = 36)
pub const SHAPE_BYTES_PER_VERT: usize = 36;
pub const SHAPE_BYTES_PER_QUAD: usize = SHAPE_BYTES_PER_VERT * 4; // 4 verts, indexed

// bytes per sprite vertex (pos2*f32 + uv2*f32 + tint_rgba*u8x4 = 20)
pub const SPRITE_BYTES_PER_VERT: usize = 20;
pub const SPRITE_BYTES_PER_QUAD: usize = SPRITE_BYTES_PER_VERT * 4; // 4 verts, indexed

// bytes per sprite instance (pos2 + scale2 + rot + uv_min2 + uv_max2 + tint_u8x4 = 40)
pub const SPRITE_INST_SIZE: usize = 40;

// max quads in one ibo (u16 indices)
pub const MAX_IBO_QUADS: usize = 16384;

// one draw comand with its gpu state
#[derive(Clone, Copy)]
struct DrawCmd {
    kind: CmdKind,
    // byte offset into shape_buf or sprite_buf
    vert_start: u32,
    // number of bytes
    vert_len: u32,
    z_index: i32,
    blend: BlendMode,
    // scissor rect in pixels or none for full viewport
    clip: Option<[u32; 4]>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CmdKind {
    Shape,      // quad-based shape, uses ibo
    ShapeRaw,   // arbitrary tris, uses glDrawArrays
    Sprite { texture_id: u64 },      // legacy quad sprite (for transform stack)
    SpriteInst { texture_id: u64 },  // instanced sprite (1 instance per cmd)
    SdfSprite { texture_id: u64 },
}

impl CmdKind {
    // sort key: (type_order, texture_id)
    fn sort_key(&self) -> (u8, u64) {
        match self {
            CmdKind::Shape => (0, 0),
            CmdKind::ShapeRaw => (0, 1),
            CmdKind::Sprite { texture_id } => (1, *texture_id),
            CmdKind::SpriteInst { texture_id } => (1, *texture_id), // same sort bucket as sprite
            CmdKind::SdfSprite { texture_id } => (2, *texture_id),
        }
    }
}

pub struct Batcher {
    shape_buf: Vec<u8>,
    sprite_buf: Vec<u8>,
    inst_buf: Vec<u8>,   // per-instance data for instanced sprites
    // scratch buffers for reordering after sort
    shape_scratch: Vec<u8>,
    sprite_scratch: Vec<u8>,
    inst_scratch: Vec<u8>,
    commands: Vec<DrawCmd>,
    draw_calls: u32,
    texture_binds: u32,
    vertices: u32,
}

impl Batcher {
    pub fn new() -> Self {
        Self {
            shape_buf: Vec::with_capacity(SHAPE_BYTES_PER_QUAD * 256),
            sprite_buf: Vec::with_capacity(SPRITE_BYTES_PER_QUAD * 64),
            inst_buf: Vec::with_capacity(SPRITE_INST_SIZE * 256),
            shape_scratch: Vec::with_capacity(SHAPE_BYTES_PER_QUAD * 256),
            sprite_scratch: Vec::with_capacity(SPRITE_BYTES_PER_QUAD * 64),
            inst_scratch: Vec::with_capacity(SPRITE_INST_SIZE * 256),
            commands: Vec::with_capacity(256),
            draw_calls: 0,
            texture_binds: 0,
            vertices: 0,
        }
    }

    pub fn clear(&mut self) {
        self.shape_buf.clear();
        self.sprite_buf.clear();
        self.inst_buf.clear();
        self.commands.clear();
        self.draw_calls = 0;
        self.texture_binds = 0;
        self.vertices = 0;
    }

    pub fn draw_calls(&self) -> u32 {
        self.draw_calls
    }

    pub fn texture_binds(&self) -> u32 {
        self.texture_binds
    }

    pub fn vertices(&self) -> u32 {
        self.vertices
    }

    fn upload_one_vbo_u8(
        gl: &glow::Context,
        vbo: glow::Buffer,
        buf: &[u8],
        cap: usize,
    ) -> usize {
        Self::upload_one_vbo_raw(gl, vbo, buf, cap, buf.len())
    }

    fn upload_one_vbo_raw(
        gl: &glow::Context,
        vbo: glow::Buffer,
        data: &[u8],
        cap: usize,
        needed: usize,
    ) -> usize {
        if needed == 0 {
            return cap;
        }
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            if needed > cap {
                let mut new_cap = cap.max(65536);
                while new_cap < needed {
                    new_cap *= 2;
                }
                // alloc full capacity then write data (sub_data needs room for future frames)
                gl.buffer_data_size(
                    glow::ARRAY_BUFFER,
                    new_cap as i32,
                    glow::DYNAMIC_DRAW,
                );
                gl.buffer_sub_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    0,
                    data,
                );
                new_cap
            } else {
                // no orphan needed, gpu reads from the other buffer
                gl.buffer_sub_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    0,
                    data,
                );
                cap
            }
        }
    }

    // push a quad shape (4 verts, indexed via ibo)
    pub fn push_shape(
        &mut self,
        verts: &[u8; SHAPE_BYTES_PER_QUAD],
        z_index: i32,
        blend: BlendMode,
        clip: Option<[u32; 4]>,
    ) {
        let start = self.shape_buf.len() as u32;
        self.shape_buf.extend_from_slice(verts);
        self.commands.push(DrawCmd {
            kind: CmdKind::Shape,
            vert_start: start,
            vert_len: SHAPE_BYTES_PER_QUAD as u32,
            z_index,
            blend,
            clip,
        });
    }

    // push raw triangles (non-quad, uses glDrawArrays)
    pub fn push_shape_raw(
        &mut self,
        verts: &[u8],
        z_index: i32,
        blend: BlendMode,
        clip: Option<[u32; 4]>,
    ) {
        debug_assert!(verts.len() % SHAPE_BYTES_PER_VERT == 0);
        let start = self.shape_buf.len() as u32;
        self.shape_buf.extend_from_slice(verts);
        self.commands.push(DrawCmd {
            kind: CmdKind::ShapeRaw,
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
        verts: &[u8; SPRITE_BYTES_PER_QUAD],
        z_index: i32,
        blend: BlendMode,
        clip: Option<[u32; 4]>,
    ) {
        let start = self.sprite_buf.len() as u32;
        self.sprite_buf.extend_from_slice(verts);
        self.commands.push(DrawCmd {
            kind: CmdKind::Sprite { texture_id },
            vert_start: start,
            vert_len: SPRITE_BYTES_PER_QUAD as u32,
            z_index,
            blend,
            clip,
        });
    }

    pub fn push_sdf_sprite(
        &mut self,
        texture_id: u64,
        verts: &[u8; SPRITE_BYTES_PER_QUAD],
        z_index: i32,
        blend: BlendMode,
        clip: Option<[u32; 4]>,
    ) {
        let start = self.sprite_buf.len() as u32;
        self.sprite_buf.extend_from_slice(verts);
        self.commands.push(DrawCmd {
            kind: CmdKind::SdfSprite { texture_id },
            vert_start: start,
            vert_len: SPRITE_BYTES_PER_QUAD as u32,
            z_index,
            blend,
            clip,
        });
    }

    // push a single sprite instance (40 bytes) for instanced drawing
    pub fn push_sprite_inst(
        &mut self,
        texture_id: u64,
        data: &[u8; SPRITE_INST_SIZE],
        z_index: i32,
        blend: BlendMode,
        clip: Option<[u32; 4]>,
    ) {
        let start = self.inst_buf.len() as u32;
        self.inst_buf.extend_from_slice(data);
        self.commands.push(DrawCmd {
            kind: CmdKind::SpriteInst { texture_id },
            vert_start: start,
            vert_len: SPRITE_INST_SIZE as u32,
            z_index,
            blend,
            clip,
        });
    }

    // flush all batched geometry with sorting and state changes
    // returns updated vbo capacities (shape, sprite)
    pub fn flush(&mut self, ctx: &FlushContext) -> (usize, usize, usize) {
        self.draw_calls = 0;
        if self.commands.is_empty() {
            return (ctx.shape_vbo_cap, ctx.sprite_vbo_cap, ctx.inst_data_vbo_cap);
        }

        // check if commands are already sorted (common in structured scenes)
        let sort_key = |c: &DrawCmd| (c.z_index, c.kind.sort_key(), c.blend);
        let already_sorted = self.commands.windows(2).all(|w| sort_key(&w[0]) <= sort_key(&w[1]));

        if !already_sorted {
            // sort by z first, then by kind/texture, then blend for max coalescing
            self.commands.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));

            // reorder vertex data so sorted cmds have contiguous verts
            self.shape_scratch.clear();
            self.sprite_scratch.clear();
            self.inst_scratch.clear();

            for cmd in self.commands.iter_mut() {
                let start = cmd.vert_start as usize;
                let len = cmd.vert_len as usize;
                match cmd.kind {
                    CmdKind::Shape | CmdKind::ShapeRaw => {
                        let new_start = self.shape_scratch.len() as u32;
                        self.shape_scratch.extend_from_slice(&self.shape_buf[start..start + len]);
                        cmd.vert_start = new_start;
                    }
                    CmdKind::Sprite { .. } | CmdKind::SdfSprite { .. } => {
                        let new_start = self.sprite_scratch.len() as u32;
                        self.sprite_scratch.extend_from_slice(&self.sprite_buf[start..start + len]);
                        cmd.vert_start = new_start;
                    }
                    CmdKind::SpriteInst { .. } => {
                        let new_start = self.inst_scratch.len() as u32;
                        self.inst_scratch.extend_from_slice(&self.inst_buf[start..start + len]);
                        cmd.vert_start = new_start;
                    }
                }
            }

            // swap scratch into main buffers (reordered data now in main bufs)
            std::mem::swap(&mut self.shape_buf, &mut self.shape_scratch);
            std::mem::swap(&mut self.sprite_buf, &mut self.sprite_scratch);
            std::mem::swap(&mut self.inst_buf, &mut self.inst_scratch);
        }

        // upload reordered data to gpu
        let new_shape_cap = Self::upload_one_vbo_u8(ctx.gl, ctx.shape_vbo, &self.shape_buf, ctx.shape_vbo_cap);
        let new_sprite_cap = Self::upload_one_vbo_u8(ctx.gl, ctx.sprite_vbo, &self.sprite_buf, ctx.sprite_vbo_cap);
        // upload instance data if any
        let new_inst_cap = if !self.inst_buf.is_empty() {
            Self::upload_one_vbo_u8(ctx.gl, ctx.inst_data_vbo, &self.inst_buf, ctx.inst_data_vbo_cap)
        } else {
            ctx.inst_data_vbo_cap
        };

        let mut cur_blend = BlendMode::Alpha;
        let mut cur_clip: Option<[u32; 4]> = None;
        let mut cur_texture: Option<u64> = None;
        let mut cur_program: Option<glow::Program> = None;

        // set initial gl state + upload proj matrix to all programs once
        unsafe {
            ctx.gl.enable(glow::BLEND);
            apply_blend(&ctx.gl, cur_blend);
            ctx.gl.disable(glow::SCISSOR_TEST);
            // set projection on all programs upfront (doesnt change during flush)
            ctx.gl.use_program(Some(ctx.shape_prog));
            if let Some(loc) = &ctx.shape_loc_proj {
                ctx.gl.uniform_matrix_4_f32_slice(Some(loc), false, ctx.proj);
            }
            ctx.gl.use_program(Some(ctx.sprite_prog));
            if let Some(loc) = &ctx.sprite_loc_proj {
                ctx.gl.uniform_matrix_4_f32_slice(Some(loc), false, ctx.proj);
            }
            if let Some(loc) = &ctx.sprite_loc_tex {
                ctx.gl.uniform_1_i32(Some(loc), 0);
            }
            ctx.gl.use_program(Some(ctx.sdf_prog));
            if let Some(loc) = &ctx.sdf_loc_proj {
                ctx.gl.uniform_matrix_4_f32_slice(Some(loc), false, ctx.proj);
            }
            if let Some(loc) = &ctx.sdf_loc_tex {
                ctx.gl.uniform_1_i32(Some(loc), 0);
            }
            ctx.gl.use_program(Some(ctx.inst_sprite_prog));
            if let Some(loc) = &ctx.inst_sprite_loc_proj {
                ctx.gl.uniform_matrix_4_f32_slice(Some(loc), false, ctx.proj);
            }
            if let Some(loc) = &ctx.inst_sprite_loc_tex {
                ctx.gl.uniform_1_i32(Some(loc), 0);
            }
        }

        // coalesce and draw
        let mut i = 0;
        while i < self.commands.len() {
            let cmd = self.commands[i];

            // coalesce adjascent cmds with same state (verts now contiguous after reorder)
            let mut end = i + 1;
            while end < self.commands.len() {
                let next = &self.commands[end];
                if next.kind != cmd.kind
                    || next.blend != cmd.blend
                    || next.clip != cmd.clip
                    || next.z_index != cmd.z_index
                {
                    break;
                }
                end += 1;
            }

            // compute merged range
            let total_vert_len: u32 = self.commands[i..end].iter().map(|c| c.vert_len).sum();
            let vert_start = cmd.vert_start;

            // switch blend if needed
            if cmd.blend != cur_blend {
                cur_blend = cmd.blend;
                apply_blend(&ctx.gl, cur_blend);
            }

            // switch scissor if needed
            if cmd.clip != cur_clip {
                cur_clip = cmd.clip;
                unsafe {
                    match cur_clip {
                        Some([x, y, w, h]) => {
                            ctx.gl.enable(glow::SCISSOR_TEST);
                            // gl scissor is bottom-left origin, flip y
                            let flipped_y = ctx.viewport_h.saturating_sub(y + h);
                            ctx.gl.scissor(x as i32, flipped_y as i32, w as i32, h as i32);
                        }
                        None => {
                            ctx.gl.disable(glow::SCISSOR_TEST);
                        }
                    }
                }
            }

            // issue draw
            match cmd.kind {
                CmdKind::Shape => {
                    // indexed quads: 4 verts per quad, 6 indices per quad
                    let first_vert = vert_start as usize / SHAPE_BYTES_PER_VERT;
                    let vert_count = total_vert_len as usize / SHAPE_BYTES_PER_VERT;
                    let quad_count = vert_count / 4;
                    let index_count = quad_count * 6;
                    let first_quad = first_vert / 4;
                    let index_offset = first_quad * 6 * 2; // u16 indices
                    unsafe {
                        if cur_program != Some(ctx.shape_prog) {
                            ctx.gl.use_program(Some(ctx.shape_prog));
                            ctx.gl.bind_vertex_array(Some(ctx.shape_vao));
                            cur_program = Some(ctx.shape_prog);
                            cur_texture = None;
                        }
                        ctx.gl.draw_elements(glow::TRIANGLES, index_count as i32, glow::UNSIGNED_SHORT, index_offset as i32);
                    }
                    self.draw_calls += 1;
                    self.vertices += vert_count as u32;
                }
                CmdKind::ShapeRaw => {
                    // raw triangles, non-indexed
                    let first_vert = vert_start as usize / SHAPE_BYTES_PER_VERT;
                    let vert_count = total_vert_len as usize / SHAPE_BYTES_PER_VERT;
                    unsafe {
                        if cur_program != Some(ctx.shape_prog) {
                            ctx.gl.use_program(Some(ctx.shape_prog));
                            ctx.gl.bind_vertex_array(Some(ctx.shape_vao));
                            cur_program = Some(ctx.shape_prog);
                            cur_texture = None;
                        }
                        ctx.gl.draw_arrays(glow::TRIANGLES, first_vert as i32, vert_count as i32);
                    }
                    self.draw_calls += 1;
                    self.vertices += vert_count as u32;
                }
                CmdKind::Sprite { texture_id } => {
                    let gl_tex = match ctx.textures.get(&texture_id) {
                        Some(t) => *t,
                        None => { i = end; continue; }
                    };
                    let first_vert = vert_start as usize / SPRITE_BYTES_PER_VERT;
                    let vert_count = total_vert_len as usize / SPRITE_BYTES_PER_VERT;
                    let quad_count = vert_count / 4;
                    let index_count = quad_count * 6;
                    let first_quad = first_vert / 4;
                    let index_offset = first_quad * 6 * 2;
                    unsafe {
                        if cur_program != Some(ctx.sprite_prog) {
                            ctx.gl.use_program(Some(ctx.sprite_prog));
                            ctx.gl.bind_vertex_array(Some(ctx.sprite_vao));
                            cur_program = Some(ctx.sprite_prog);
                            cur_texture = None;
                        }
                        if cur_texture != Some(texture_id) {
                            ctx.gl.active_texture(glow::TEXTURE0);
                            ctx.gl.bind_texture(glow::TEXTURE_2D, Some(gl_tex));
                            cur_texture = Some(texture_id);
                            self.texture_binds += 1;
                        }
                        ctx.gl.draw_elements(glow::TRIANGLES, index_count as i32, glow::UNSIGNED_SHORT, index_offset as i32);
                    }
                    self.draw_calls += 1;
                    self.vertices += vert_count as u32;
                }
                CmdKind::SpriteInst { texture_id } => {
                    // instanced sprites: one draw call for all coalesced instances
                    let gl_tex = match ctx.textures.get(&texture_id) {
                        Some(t) => *t,
                        None => { i = end; continue; }
                    };
                    let instance_count = total_vert_len as usize / SPRITE_INST_SIZE;
                    unsafe {
                        if cur_program != Some(ctx.inst_sprite_prog) {
                            ctx.gl.use_program(Some(ctx.inst_sprite_prog));
                            ctx.gl.bind_vertex_array(Some(ctx.inst_sprite_vao));
                            cur_program = Some(ctx.inst_sprite_prog);
                            cur_texture = None;
                        }
                        if cur_texture != Some(texture_id) {
                            ctx.gl.active_texture(glow::TEXTURE0);
                            ctx.gl.bind_texture(glow::TEXTURE_2D, Some(gl_tex));
                            cur_texture = Some(texture_id);
                            self.texture_binds += 1;
                        }
                        ctx.gl.draw_elements_instanced(
                            glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0,
                            instance_count as i32,
                        );
                    }
                    self.draw_calls += 1;
                    self.vertices += instance_count as u32;
                }
                CmdKind::SdfSprite { texture_id } => {
                    let gl_tex = match ctx.textures.get(&texture_id) {
                        Some(t) => *t,
                        None => { i = end; continue; }
                    };
                    let first_vert = vert_start as usize / SPRITE_BYTES_PER_VERT;
                    let vert_count = total_vert_len as usize / SPRITE_BYTES_PER_VERT;
                    let quad_count = vert_count / 4;
                    let index_count = quad_count * 6;
                    let first_quad = first_vert / 4;
                    let index_offset = first_quad * 6 * 2;
                    unsafe {
                        if cur_program != Some(ctx.sdf_prog) {
                            ctx.gl.use_program(Some(ctx.sdf_prog));
                            ctx.gl.bind_vertex_array(Some(ctx.sprite_vao));
                            cur_program = Some(ctx.sdf_prog);
                            cur_texture = None;
                        }
                        if cur_texture != Some(texture_id) {
                            ctx.gl.active_texture(glow::TEXTURE0);
                            ctx.gl.bind_texture(glow::TEXTURE_2D, Some(gl_tex));
                            cur_texture = Some(texture_id);
                            self.texture_binds += 1;
                        }
                        ctx.gl.draw_elements(glow::TRIANGLES, index_count as i32, glow::UNSIGNED_SHORT, index_offset as i32);
                    }
                    self.draw_calls += 1;
                    self.vertices += vert_count as u32;
                }
            }

            i = end;
        }

        // restore scissor state
        unsafe {
            ctx.gl.disable(glow::SCISSOR_TEST);
        }

        (new_shape_cap, new_sprite_cap, new_inst_cap)
    }
}

fn apply_blend(gl: &glow::Context, mode: BlendMode) {
    unsafe {
        match mode {
            BlendMode::Alpha => {
                gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            }
            BlendMode::Additive => {
                gl.blend_func(glow::SRC_ALPHA, glow::ONE);
            }
            BlendMode::Multiply => {
                gl.blend_func(glow::DST_COLOR, glow::ONE_MINUS_SRC_ALPHA);
            }
            BlendMode::Screen => {
                gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_COLOR);
            }
            BlendMode::PremultipliedAlpha => {
                gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            }
        }
    }
}

// all the gl handles the batcher needs to issue draws
pub struct FlushContext<'a> {
    pub gl: &'a glow::Context,
    pub proj: &'a [f32; 16],
    pub shape_vbo: glow::Buffer,
    pub sprite_vbo: glow::Buffer,
    pub shape_vbo_cap: usize,
    pub sprite_vbo_cap: usize,
    pub shape_prog: glow::Program,
    pub shape_vao: glow::VertexArray,
    pub shape_loc_proj: &'a Option<glow::UniformLocation>,
    pub sprite_prog: glow::Program,
    pub sprite_vao: glow::VertexArray,
    pub sprite_loc_proj: &'a Option<glow::UniformLocation>,
    pub sprite_loc_tex: &'a Option<glow::UniformLocation>,
    pub textures: &'a HashMap<u64, glow::Texture>,
    pub viewport_h: u32,
    // sdf text shader handles
    pub sdf_prog: glow::Program,
    pub sdf_loc_proj: &'a Option<glow::UniformLocation>,
    pub sdf_loc_tex: &'a Option<glow::UniformLocation>,
    // instanced sprite pipeline
    pub inst_sprite_prog: glow::Program,
    pub inst_sprite_vao: glow::VertexArray,
    pub inst_sprite_loc_proj: &'a Option<glow::UniformLocation>,
    pub inst_sprite_loc_tex: &'a Option<glow::UniformLocation>,
    pub inst_data_vbo: glow::Buffer,
    pub inst_data_vbo_cap: usize,
}
