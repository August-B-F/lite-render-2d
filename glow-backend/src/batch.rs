use std::collections::HashMap;

use glow::HasContext;
use lite_render_2d_core::BlendMode;

// floats per shape vertex (pos2 + local2 + color4 + mode1 + stroke_w1 + size2)
const SHAPE_FLOATS_PER_VERT: usize = 12;
const SHAPE_FLOATS_PER_QUAD: usize = SHAPE_FLOATS_PER_VERT * 6;

// floats per sprite vertex (pos2 + uv2 + tint4 + opacity1)
const SPRITE_FLOATS_PER_VERT: usize = 9;
const SPRITE_FLOATS_PER_QUAD: usize = SPRITE_FLOATS_PER_VERT * 6;

// one draw comand with its gpu state
#[derive(Clone, Copy)]
struct DrawCmd {
    kind: CmdKind,
    // float offset into shape_buf or sprite_buf
    vert_start: u32,
    // number of floats
    vert_len: u32,
    z_index: i32,
    blend: BlendMode,
    // scissor rect in pixels or none for full viewport
    clip: Option<[u32; 4]>,
}

#[derive(Clone, Copy, PartialEq)]
enum CmdKind {
    Shape,
    Sprite { texture_id: u64 },
}

pub struct Batcher {
    shape_buf: Vec<f32>,
    sprite_buf: Vec<f32>,
    commands: Vec<DrawCmd>,
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
        debug_assert!(verts.len() % SHAPE_FLOATS_PER_VERT == 0);
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

    // flush all batched geometry with sorting and state changes
    pub fn flush(&mut self, ctx: &FlushContext) {
        self.draw_calls = 0;
        if self.commands.is_empty() {
            return;
        }

        // stabble sort so same z keeps submision order
        self.commands.sort_by_key(|cmd| cmd.z_index);

        // upload shape and sprite bufs once
        if !self.shape_buf.is_empty() {
            unsafe {
                ctx.gl.bind_vertex_array(Some(ctx.shape_vao));
                ctx.gl.bind_buffer(glow::ARRAY_BUFFER, Some(ctx.vbo));
                ctx.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    f32_as_bytes(&self.shape_buf),
                    glow::DYNAMIC_DRAW,
                );
            }
        }

        let mut cur_blend = BlendMode::Alpha;
        let mut cur_clip: Option<[u32; 4]> = None;

        // set initial gl state
        unsafe {
            ctx.gl.enable(glow::BLEND);
            apply_blend(&ctx.gl, cur_blend);
            ctx.gl.disable(glow::SCISSOR_TEST);
        }

        // coalesce and draw
        let mut i = 0;
        while i < self.commands.len() {
            let cmd = self.commands[i];

            // coalesce adjascent cmds with same state
            let mut end = i + 1;
            while end < self.commands.len() {
                let next = &self.commands[end];
                if next.kind != cmd.kind
                    || next.blend != cmd.blend
                    || next.clip != cmd.clip
                    || next.z_index != cmd.z_index
                {
                    // check contiguous for same-kind merging
                    break;
                }
                // check contiguous verts
                let prev = &self.commands[end - 1];
                let expected_start = prev.vert_start + prev.vert_len;
                if next.vert_start != expected_start {
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
                    let vert_count = total_vert_len as usize / SHAPE_FLOATS_PER_VERT;
                    let first = vert_start as usize / SHAPE_FLOATS_PER_VERT;
                    unsafe {
                        ctx.gl.use_program(Some(ctx.shape_prog));
                        if let Some(loc) = &ctx.shape_loc_proj {
                            ctx.gl.uniform_matrix_4_f32_slice(Some(loc), false, ctx.proj);
                        }
                        ctx.gl.bind_vertex_array(Some(ctx.shape_vao));
                        ctx.gl.draw_arrays(glow::TRIANGLES, first as i32, vert_count as i32);
                    }
                    self.draw_calls += 1;
                }
                CmdKind::Sprite { texture_id } => {
                    let gl_tex = match ctx.textures.get(&texture_id) {
                        Some(t) => *t,
                        None => { i = end; continue; }
                    };
                    let vert_count = total_vert_len as usize / SPRITE_FLOATS_PER_VERT;
                    unsafe {
                        ctx.gl.use_program(Some(ctx.sprite_prog));
                        if let Some(loc) = &ctx.sprite_loc_proj {
                            ctx.gl.uniform_matrix_4_f32_slice(Some(loc), false, ctx.proj);
                        }
                        if let Some(loc) = &ctx.sprite_loc_tex {
                            ctx.gl.uniform_1_i32(Some(loc), 0);
                        }
                        ctx.gl.active_texture(glow::TEXTURE0);
                        ctx.gl.bind_texture(glow::TEXTURE_2D, Some(gl_tex));
                        ctx.gl.bind_vertex_array(Some(ctx.sprite_vao));
                        ctx.gl.bind_buffer(glow::ARRAY_BUFFER, Some(ctx.vbo));
                        // upload just this sprite range
                        let slice = &self.sprite_buf[vert_start as usize..(vert_start + total_vert_len) as usize];
                        ctx.gl.buffer_data_u8_slice(
                            glow::ARRAY_BUFFER,
                            f32_as_bytes(slice),
                            glow::DYNAMIC_DRAW,
                        );
                        ctx.gl.draw_arrays(glow::TRIANGLES, 0, vert_count as i32);
                    }
                    self.draw_calls += 1;
                }
            }

            i = end;
        }

        // restore scissor state
        unsafe {
            ctx.gl.disable(glow::SCISSOR_TEST);
        }
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
        }
    }
}

// all the gl handles the batcher needs to issue draws
pub struct FlushContext<'a> {
    pub gl: &'a glow::Context,
    pub proj: &'a [f32; 16],
    pub vbo: glow::Buffer,
    pub shape_prog: glow::Program,
    pub shape_vao: glow::VertexArray,
    pub shape_loc_proj: &'a Option<glow::UniformLocation>,
    pub sprite_prog: glow::Program,
    pub sprite_vao: glow::VertexArray,
    pub sprite_loc_proj: &'a Option<glow::UniformLocation>,
    pub sprite_loc_tex: &'a Option<glow::UniformLocation>,
    pub textures: &'a HashMap<u64, glow::Texture>,
    pub viewport_h: u32,
}

fn f32_as_bytes(data: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}
