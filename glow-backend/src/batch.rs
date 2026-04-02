use std::collections::BTreeMap;

use glow::HasContext;

// floats per shape vertex (pos2 + local2 + color4 + mode1 + stroke_w1 + size2)
const SHAPE_FLOATS_PER_VERT: usize = 12;
const SHAPE_FLOATS_PER_QUAD: usize = SHAPE_FLOATS_PER_VERT * 6;

// floats per sprite vertex (pos2 + uv2 + tint4 + opacity1)
const SPRITE_FLOATS_PER_VERT: usize = 9;
const SPRITE_FLOATS_PER_QUAD: usize = SPRITE_FLOATS_PER_VERT * 6;

pub struct Batcher {
    shape_buf: Vec<f32>,
    // btreemap so textures flush in deterministic order
    sprite_runs: BTreeMap<u64, Vec<f32>>,
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

    pub fn push_shape(&mut self, verts: &[f32; SHAPE_FLOATS_PER_QUAD]) {
        self.shape_buf.extend_from_slice(verts);
    }

    pub fn push_sprite(&mut self, texture_id: u64, verts: &[f32; SPRITE_FLOATS_PER_QUAD]) {
        self.sprite_runs
            .entry(texture_id)
            .or_insert_with(|| Vec::with_capacity(SPRITE_FLOATS_PER_QUAD * 64))
            .extend_from_slice(verts);
    }

    // flush all batched geometry
    pub fn flush(&mut self, ctx: &FlushContext) {
        self.draw_calls = 0;

        // -- shapes --
        if !self.shape_buf.is_empty() {
            let vert_count = self.shape_buf.len() / SHAPE_FLOATS_PER_VERT;
            unsafe {
                ctx.gl.use_program(Some(ctx.shape_prog));
                if let Some(loc) = &ctx.shape_loc_proj {
                    ctx.gl.uniform_matrix_4_f32_slice(Some(loc), false, ctx.proj);
                }
                ctx.gl.bind_vertex_array(Some(ctx.shape_vao));
                ctx.gl.bind_buffer(glow::ARRAY_BUFFER, Some(ctx.vbo));
                ctx.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    f32_as_bytes(&self.shape_buf),
                    glow::DYNAMIC_DRAW,
                );
                ctx.gl.draw_arrays(glow::TRIANGLES, 0, vert_count as i32);
            }
            self.draw_calls += 1;
        }

        // -- sprites, one draw per texture --
        for (&tex_id, buf) in &self.sprite_runs {
            if buf.is_empty() {
                continue;
            }
            let gl_tex = match ctx.textures.get(&tex_id) {
                Some(t) => *t,
                None => continue,
            };
            let vert_count = buf.len() / SPRITE_FLOATS_PER_VERT;
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
                ctx.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    f32_as_bytes(buf),
                    glow::DYNAMIC_DRAW,
                );
                ctx.gl.draw_arrays(glow::TRIANGLES, 0, vert_count as i32);
            }
            self.draw_calls += 1;
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
    pub textures: &'a std::collections::HashMap<u64, glow::Texture>,
}

fn f32_as_bytes(data: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}
