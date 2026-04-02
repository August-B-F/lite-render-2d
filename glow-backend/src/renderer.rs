use std::collections::HashMap;
use std::num::NonZeroU32;
use std::time::Instant;

use glow::HasContext;
use glutin::surface::GlSurface;

use lite_render_2d_core::{
    BlendMode, Camera2D, Color, DrawParams, DrawStyle, FilterMode, FontHandle, FrameStats,
    LineParams, Path, PostEffect, Rect, RenderTargetHandle, Renderer, RendererError, RoundedRect,
    SpriteInstance, SpriteParams, StrokeParams, TextParams, TextureHandle, TextureParams,
    Transform2D, Vec2, WrapMode,
};

use crate::batch::{Batcher, FlushContext};
use crate::context::{self, GlContext, Surface};
use crate::shaders;

struct TextureInfo {
    gl_tex: glow::Texture,
    width: u32,
    height: u32,
}

struct RenderTargetInfo {
    fbo: glow::Framebuffer,
    color_tex: glow::Texture,
    texture_handle_id: u64,
    width: u32,
    height: u32,
}

pub struct GlowRenderer {
    gl: glow::Context,
    surface: Surface,
    gl_ctx: GlContext,
    clear_color: Color,
    w: u32,
    h: u32,
    proj: [f32; 16],
    cam: Camera2D,
    // batched shape pipeline
    shape_prog: glow::Program,
    shape_vaos: [glow::VertexArray; 2],
    shape_loc_proj: Option<glow::UniformLocation>,
    // batched sprite pipeline
    sprite_prog: glow::Program,
    sprite_vaos: [glow::VertexArray; 2],
    sprite_loc_proj: Option<glow::UniformLocation>,
    sprite_loc_tex: Option<glow::UniformLocation>,
    // double-buffered vbos (ping-pong to avoid gpu stalls)
    shape_vbos: [glow::Buffer; 2],
    sprite_vbos: [glow::Buffer; 2],
    shape_vbo_caps: [usize; 2],
    sprite_vbo_caps: [usize; 2],
    vbo_frame_idx: usize,
    // shared quad index buffer (bound to vaos, kept alive)
    #[allow(dead_code)]
    quad_ibo: glow::Buffer,
    // batcher
    batcher: Batcher,
    // texture storage
    textures: HashMap<u64, TextureInfo>,
    next_tex_id: u64,
    // font system
    font_system: lite_render_2d_core::font_atlas::FontSystem,
    font_atlas_tex_id: Option<u64>,
    font_atlas_gl_tex: Option<glow::Texture>,
    // transform stack
    transform_stack: lite_render_2d_core::transform_stack::TransformStack,
    // clip rect stack (pixel coords)
    clip_stack: Vec<[u32; 4]>,
    current_clip: Option<[u32; 4]>,
    // blend mode
    current_blend: BlendMode,
    // dpi scale factor
    scale_factor: f32,
    // post-processing effect programs
    effect_grayscale_prog: glow::Program,
    effect_invert_prog: glow::Program,
    effect_brightness_prog: glow::Program,
    effect_brightness_loc: Option<glow::UniformLocation>,
    effect_vignette_prog: glow::Program,
    effect_vao: glow::VertexArray,
    #[allow(dead_code)]
    effect_vbo: glow::Buffer,
    // blur + bloom effect programs
    effect_blur_h_prog: glow::Program,
    effect_blur_h_radius_loc: Option<glow::UniformLocation>,
    effect_blur_v_prog: glow::Program,
    effect_blur_v_radius_loc: Option<glow::UniformLocation>,
    effect_bloom_threshold_prog: glow::Program,
    effect_bloom_threshold_loc: Option<glow::UniformLocation>,
    effect_bloom_composite_prog: glow::Program,
    effect_bloom_intensity_loc: Option<glow::UniformLocation>,
    effect_bloom_tex_loc: Option<glow::UniformLocation>,
    // render targets
    render_targets: HashMap<u64, RenderTargetInfo>,
    next_rt_id: u64,
    active_render_target: Option<u64>,
    saved_viewport: Option<(u32, u32)>,
    saved_proj: Option<[f32; 16]>,
    // custom materials
    materials: HashMap<u64, glow::Program>,
    next_material_id: u64,
    // sdf text pipeline
    sdf_prog: glow::Program,
    sdf_loc_proj: Option<glow::UniformLocation>,
    sdf_loc_tex: Option<glow::UniformLocation>,
    // sdf font system
    sdf_font_system: lite_render_2d_core::sdf_font::SdfFontSystem,
    sdf_atlas_tex_id: Option<u64>,
    sdf_atlas_gl_tex: Option<glow::Texture>,
    // sprite texture atlas for small textures
    sprite_atlas: lite_render_2d_core::atlas::TextureAtlas,
    sprite_atlas_gl_tex: Option<glow::Texture>,
    sprite_atlas_tex_id: Option<u64>,
    atlas_region_map: HashMap<u64, lite_render_2d_core::atlas::AtlasRegion>,
    atlas_dirty: bool,
    atlas_image_indices: HashMap<u64, usize>, // tex_id -> image index for regrow
    // instanced sprite pipeline
    inst_sprite_prog: glow::Program,
    inst_sprite_vao: glow::VertexArray,
    inst_sprite_loc_proj: Option<glow::UniformLocation>,
    inst_sprite_loc_tex: Option<glow::UniformLocation>,
    #[allow(dead_code)]
    inst_quad_vbo: glow::Buffer,
    #[allow(dead_code)]
    inst_quad_ibo: glow::Buffer,
    inst_data_vbo: glow::Buffer,
    inst_data_vbo_cap: usize,
    // perf stats
    frame_start: Instant,
    fps_samples: [f64; 60],
    fps_idx: usize,
    gpu_ram_bytes: u64,
    stats_enabled: bool,
    // cached texture map to avoid per-frame alloc
    cached_gl_tex_map: HashMap<u64, glow::Texture>,
    tex_map_dirty: bool,
}

fn intersect_rects(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = (a[0] + a[2]).min(b[0] + b[2]);
    let y1 = (a[1] + a[3]).min(b[1] + b[3]);
    if x1 <= x0 || y1 <= y0 {
        return [0, 0, 0, 0];
    }
    [x0, y0, x1 - x0, y1 - y0]
}

// y-down screen ortho, origin top-left
fn screen_ortho(w: u32, h: u32) -> [f32; 16] {
    let w = w as f32;
    let h = h as f32;
    [
        2.0 / w, 0.0,      0.0,  0.0,
        0.0,    -2.0 / h,  0.0,  0.0,
        0.0,     0.0,      -1.0, 0.0,
       -1.0,     1.0,       0.0, 1.0,
    ]
}

impl GlowRenderer {
    // rebuild cached texture map if dirty
    fn rebuild_tex_map_if_dirty(&mut self) {
        if self.tex_map_dirty {
            self.cached_gl_tex_map.clear();
            for (&id, info) in &self.textures {
                self.cached_gl_tex_map.insert(id, info.gl_tex);
            }
            self.tex_map_dirty = false;
        }
    }

    pub fn draw_calls(&self) -> u32 {
        self.batcher.draw_calls()
    }

    fn upload_sprite_atlas_if_dirty(&mut self) {
        if !self.atlas_dirty {
            return;
        }
        self.atlas_dirty = false;

        if self.sprite_atlas_gl_tex.is_none() {
            // first time: create gl texture with full atlas data
            let (data, aw, ah) = self.sprite_atlas.texture_data();
            let tex = unsafe {
                let t = self.gl.create_texture().expect("create atlas tex");
                self.gl.bind_texture(glow::TEXTURE_2D, Some(t));
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D, 0, glow::RGBA8 as i32,
                    aw as i32, ah as i32, 0,
                    glow::RGBA, glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(data)),
                );
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                self.gl.bind_texture(glow::TEXTURE_2D, None);
                t
            };
            self.sprite_atlas_gl_tex = Some(tex);
            self.gpu_ram_bytes += (aw as u64) * (ah as u64) * 4;

            // register as a texture so the batcher can look it up
            let atlas_tex_id = self.next_tex_id;
            self.next_tex_id += 1;
            self.textures.insert(atlas_tex_id, TextureInfo { gl_tex: tex, width: aw, height: ah });
            self.sprite_atlas_tex_id = Some(atlas_tex_id);
            self.sprite_atlas.clear_dirty();
        } else if let Some((dx, dy, dw, dh)) = self.sprite_atlas.dirty_region() {
            // partial upload: only the changed region
            let sub = self.sprite_atlas.atlas_sub_data(dx, dy, dw, dh);
            let tex = self.sprite_atlas_gl_tex.unwrap();
            unsafe {
                self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                self.gl.tex_sub_image_2d(
                    glow::TEXTURE_2D, 0,
                    dx as i32, dy as i32, dw as i32, dh as i32,
                    glow::RGBA, glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&sub)),
                );
                self.gl.bind_texture(glow::TEXTURE_2D, None);
            }
            self.sprite_atlas.clear_dirty();
        }

        // update all atlas-packed textures to point to the atlas gl tex
        let gl_tex = match self.sprite_atlas_gl_tex {
            Some(t) => t,
            None => return,
        };
        let atlas_tex_id = self.sprite_atlas_tex_id.unwrap();
        for (&id, _region) in &self.atlas_region_map {
            if let Some(info) = self.textures.get_mut(&id) {
                info.gl_tex = gl_tex;
            }
        }
        if let Some(info) = self.textures.get_mut(&atlas_tex_id) {
            info.gl_tex = gl_tex;
        }
        self.tex_map_dirty = true;
    }

    fn flush_batch(&mut self) {
        self.upload_sprite_atlas_if_dirty();
        self.rebuild_tex_map_if_dirty();
        let viewport_h = if let Some(rt_id) = self.active_render_target {
            self.render_targets.get(&rt_id).map(|rt| rt.height).unwrap_or(self.h)
        } else {
            self.h
        };
        let fi = self.vbo_frame_idx;
        let ctx = FlushContext {
            gl: &self.gl,
            proj: &self.proj,
            shape_vbo: self.shape_vbos[fi],
            sprite_vbo: self.sprite_vbos[fi],
            shape_vbo_cap: self.shape_vbo_caps[fi],
            sprite_vbo_cap: self.sprite_vbo_caps[fi],
            shape_prog: self.shape_prog,
            shape_vao: self.shape_vaos[fi],
            shape_loc_proj: &self.shape_loc_proj,
            sprite_prog: self.sprite_prog,
            sprite_vao: self.sprite_vaos[fi],
            sprite_loc_proj: &self.sprite_loc_proj,
            sprite_loc_tex: &self.sprite_loc_tex,
            textures: &self.cached_gl_tex_map,
            viewport_h,
            sdf_prog: self.sdf_prog,
            sdf_loc_proj: &self.sdf_loc_proj,
            sdf_loc_tex: &self.sdf_loc_tex,
            inst_sprite_prog: self.inst_sprite_prog,
            inst_sprite_vao: self.inst_sprite_vao,
            inst_sprite_loc_proj: &self.inst_sprite_loc_proj,
            inst_sprite_loc_tex: &self.inst_sprite_loc_tex,
            inst_data_vbo: self.inst_data_vbo,
            inst_data_vbo_cap: self.inst_data_vbo_cap,
        };
        let (sc, spc, ic) = self.batcher.flush(&ctx);
        self.shape_vbo_caps[fi] = sc;
        self.sprite_vbo_caps[fi] = spc;
        self.inst_data_vbo_cap = ic;
    }
}

impl GlowRenderer {
    // create a temp fbo + texture for blur ping-pong
    fn create_temp_fbo(&self, w: u32, h: u32) -> (glow::Framebuffer, glow::Texture) {
        unsafe {
            let tex = self.gl.create_texture().expect("create temp tex");
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D, 0, glow::RGBA8 as i32,
                w as i32, h as i32, 0,
                glow::RGBA, glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);

            let fbo = self.gl.create_framebuffer().expect("create temp fbo");
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            self.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0,
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            (fbo, tex)
        }
    }

    fn destroy_temp_fbo(&self, fbo: glow::Framebuffer, tex: glow::Texture) {
        unsafe {
            self.gl.delete_framebuffer(fbo);
            self.gl.delete_texture(tex);
        }
    }

    // run fullscreen pass with given program, binding src_tex to texture unit 0
    fn fullscreen_pass(&self, prog: glow::Program, src_tex: glow::Texture) {
        unsafe {
            self.gl.use_program(Some(prog));
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(src_tex));
            self.gl.bind_vertex_array(Some(self.effect_vao));
            self.gl.draw_arrays(glow::TRIANGLES, 0, 6);
            self.gl.bind_vertex_array(None);
        }
    }

    // 2-pass separable gaussian blur: horiz then vert, writes back to source fbo
    fn apply_blur(&self, src_tex: glow::Texture, dst_fbo: glow::Framebuffer, w: u32, h: u32, radius: u32) {
        let (temp_fbo, temp_tex) = self.create_temp_fbo(w, h);
        let r = radius.max(1) as f32;

        unsafe {
            self.gl.viewport(0, 0, w as i32, h as i32);

            // pass 1: horizontal blur from src -> temp
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(temp_fbo));
            self.gl.use_program(Some(self.effect_blur_h_prog));
            if let Some(ref loc) = self.effect_blur_h_radius_loc {
                self.gl.uniform_1_f32(Some(loc), r);
            }
            self.fullscreen_pass(self.effect_blur_h_prog, src_tex);

            // pass 2: vertical blur from temp -> dst
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(dst_fbo));
            self.gl.use_program(Some(self.effect_blur_v_prog));
            if let Some(ref loc) = self.effect_blur_v_radius_loc {
                self.gl.uniform_1_f32(Some(loc), r);
            }
            self.fullscreen_pass(self.effect_blur_v_prog, temp_tex);

            // restore framebuffer
            if let Some(rt_id) = self.active_render_target {
                if let Some(rt) = self.render_targets.get(&rt_id) {
                    self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(rt.fbo));
                }
            } else {
                self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            }
            self.gl.viewport(0, 0, self.w as i32, self.h as i32);
        }

        self.destroy_temp_fbo(temp_fbo, temp_tex);
    }

    // bloom: threshold -> blur -> additive composite
    fn apply_bloom(&self, src_tex: glow::Texture, dst_fbo: glow::Framebuffer, w: u32, h: u32,
                   threshold: f32, intensity: f32, radius: u32) {
        let (thresh_fbo, thresh_tex) = self.create_temp_fbo(w, h);
        let (blur_fbo, blur_tex) = self.create_temp_fbo(w, h);

        unsafe {
            self.gl.viewport(0, 0, w as i32, h as i32);

            // pass 1: extract bright pixels
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(thresh_fbo));
            self.gl.use_program(Some(self.effect_bloom_threshold_prog));
            if let Some(ref loc) = self.effect_bloom_threshold_loc {
                self.gl.uniform_1_f32(Some(loc), threshold);
            }
            self.fullscreen_pass(self.effect_bloom_threshold_prog, src_tex);

            // pass 2: horizontal blur on thresholded
            let r = radius.max(1) as f32;
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(blur_fbo));
            self.gl.use_program(Some(self.effect_blur_h_prog));
            if let Some(ref loc) = self.effect_blur_h_radius_loc {
                self.gl.uniform_1_f32(Some(loc), r);
            }
            self.fullscreen_pass(self.effect_blur_h_prog, thresh_tex);

            // pass 3: vertical blur
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(thresh_fbo));
            self.gl.use_program(Some(self.effect_blur_v_prog));
            if let Some(ref loc) = self.effect_blur_v_radius_loc {
                self.gl.uniform_1_f32(Some(loc), r);
            }
            self.fullscreen_pass(self.effect_blur_v_prog, blur_tex);

            // pass 4: composite original + blurred bloom
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(dst_fbo));
            self.gl.use_program(Some(self.effect_bloom_composite_prog));
            if let Some(ref loc) = self.effect_bloom_intensity_loc {
                self.gl.uniform_1_f32(Some(loc), intensity);
            }
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(src_tex));
            self.gl.active_texture(glow::TEXTURE1);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(thresh_tex));
            if let Some(ref loc) = self.effect_bloom_tex_loc {
                self.gl.uniform_1_i32(Some(loc), 1);
            }
            self.gl.bind_vertex_array(Some(self.effect_vao));
            self.gl.draw_arrays(glow::TRIANGLES, 0, 6);
            self.gl.bind_vertex_array(None);

            // restore
            if let Some(rt_id) = self.active_render_target {
                if let Some(rt) = self.render_targets.get(&rt_id) {
                    self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(rt.fbo));
                }
            } else {
                self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            }
            self.gl.viewport(0, 0, self.w as i32, self.h as i32);
        }

        self.destroy_temp_fbo(thresh_fbo, thresh_tex);
        self.destroy_temp_fbo(blur_fbo, blur_tex);
    }

    // aabb vs camera viewport, true if entirly outside
    fn is_offscreen(&self, min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> bool {
        let hw = self.cam.viewport.x / (2.0 * self.cam.zoom);
        let hh = self.cam.viewport.y / (2.0 * self.cam.zoom);
        let cam_left = self.cam.position.x - hw;
        let cam_right = self.cam.position.x + hw;
        let cam_top = self.cam.position.y - hh;
        let cam_bottom = self.cam.position.y + hh;
        max_x < cam_left || min_x > cam_right || max_y < cam_top || min_y > cam_bottom
    }

    fn apply_transform_quad(&self, verts: &mut [u8; crate::batch::SHAPE_BYTES_PER_QUAD]) {
        if self.transform_stack.is_identity() {
            return;
        }
        for i in 0..4 {
            let base = i * crate::batch::SHAPE_BYTES_PER_VERT;
            let x = f32::from_le_bytes([verts[base], verts[base+1], verts[base+2], verts[base+3]]);
            let y = f32::from_le_bytes([verts[base+4], verts[base+5], verts[base+6], verts[base+7]]);
            let p = self.transform_stack.apply(Vec2::new(x, y));
            verts[base..base+4].copy_from_slice(&p.x.to_le_bytes());
            verts[base+4..base+8].copy_from_slice(&p.y.to_le_bytes());
        }
    }

    fn push_shape_raw_transformed(&mut self, verts: &mut Vec<f32>, z_index: i32, blend: BlendMode) {
        lite_render_2d_core::tessellation::apply_transform(verts, &self.transform_stack);
        // convert f32 tessellation output (12 floats/vert) to packed bytes (36 bytes/vert)
        let bytes = Self::f32_verts_to_shape_bytes(verts);
        self.batcher.push_shape_raw(&bytes, z_index, blend, self.current_clip);
    }

    // convert core tessellation f32 verts (12 floats each) to packed shape bytes
    fn f32_verts_to_shape_bytes(src: &[f32]) -> Vec<u8> {
        let vert_count = src.len() / 12;
        let mut dst = Vec::with_capacity(vert_count * crate::batch::SHAPE_BYTES_PER_VERT);
        for chunk in src.chunks_exact(12) {
            dst.extend_from_slice(&chunk[0].to_le_bytes()); // pos.x
            dst.extend_from_slice(&chunk[1].to_le_bytes()); // pos.y
            dst.extend_from_slice(&chunk[2].to_le_bytes()); // local.x / uv.u
            dst.extend_from_slice(&chunk[3].to_le_bytes()); // local.y / uv.v
            // color f32x4 -> u8x4
            dst.push((chunk[4].clamp(0.0, 1.0) * 255.0) as u8);
            dst.push((chunk[5].clamp(0.0, 1.0) * 255.0) as u8);
            dst.push((chunk[6].clamp(0.0, 1.0) * 255.0) as u8);
            dst.push((chunk[7].clamp(0.0, 1.0) * 255.0) as u8);
            dst.extend_from_slice(&chunk[8].to_le_bytes());  // mode
            dst.extend_from_slice(&chunk[9].to_le_bytes());  // stroke_w
            dst.extend_from_slice(&chunk[10].to_le_bytes()); // size.x
            dst.extend_from_slice(&chunk[11].to_le_bytes()); // size.y
        }
        dst
    }

    fn apply_transform_sprite(&self, verts: &mut [u8; crate::batch::SPRITE_BYTES_PER_QUAD]) {
        if self.transform_stack.is_identity() {
            return;
        }
        for i in 0..4 {
            let base = i * crate::batch::SPRITE_BYTES_PER_VERT;
            let x = f32::from_le_bytes([verts[base], verts[base+1], verts[base+2], verts[base+3]]);
            let y = f32::from_le_bytes([verts[base+4], verts[base+5], verts[base+6], verts[base+7]]);
            let p = self.transform_stack.apply(Vec2::new(x, y));
            verts[base..base+4].copy_from_slice(&p.x.to_le_bytes());
            verts[base+4..base+8].copy_from_slice(&p.y.to_le_bytes());
        }
    }

    // pack 4 shape verts into byte array with u8 color
    #[inline]
    fn pack_shape_quad(
        positions: [(f32, f32); 4],
        locals: [(f32, f32); 4],
        color: Color,
        mode: f32,
        stroke_w: f32,
        size: (f32, f32),
    ) -> [u8; crate::batch::SHAPE_BYTES_PER_QUAD] {
        let mut buf = [0u8; crate::batch::SHAPE_BYTES_PER_QUAD];
        let rb = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
        let gb = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
        let bb = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
        let ab = (color.a.clamp(0.0, 1.0) * 255.0) as u8;
        for i in 0..4 {
            let off = i * crate::batch::SHAPE_BYTES_PER_VERT;
            buf[off..off+4].copy_from_slice(&positions[i].0.to_le_bytes());
            buf[off+4..off+8].copy_from_slice(&positions[i].1.to_le_bytes());
            buf[off+8..off+12].copy_from_slice(&locals[i].0.to_le_bytes());
            buf[off+12..off+16].copy_from_slice(&locals[i].1.to_le_bytes());
            buf[off+16] = rb; buf[off+17] = gb; buf[off+18] = bb; buf[off+19] = ab;
            buf[off+20..off+24].copy_from_slice(&mode.to_le_bytes());
            buf[off+24..off+28].copy_from_slice(&stroke_w.to_le_bytes());
            buf[off+28..off+32].copy_from_slice(&size.0.to_le_bytes());
            buf[off+32..off+36].copy_from_slice(&size.1.to_le_bytes());
        }
        buf
    }

    // pack shape quad with per-vertex colors (for gradients)
    #[inline]
    fn pack_shape_quad_gradient(
        positions: [(f32, f32); 4],
        locals: [(f32, f32); 4],
        colors: [Color; 4],
        mode: f32,
        stroke_w: f32,
        size: (f32, f32),
    ) -> [u8; crate::batch::SHAPE_BYTES_PER_QUAD] {
        let mut buf = [0u8; crate::batch::SHAPE_BYTES_PER_QUAD];
        for i in 0..4 {
            let off = i * crate::batch::SHAPE_BYTES_PER_VERT;
            buf[off..off+4].copy_from_slice(&positions[i].0.to_le_bytes());
            buf[off+4..off+8].copy_from_slice(&positions[i].1.to_le_bytes());
            buf[off+8..off+12].copy_from_slice(&locals[i].0.to_le_bytes());
            buf[off+12..off+16].copy_from_slice(&locals[i].1.to_le_bytes());
            buf[off+16] = (colors[i].r.clamp(0.0, 1.0) * 255.0) as u8;
            buf[off+17] = (colors[i].g.clamp(0.0, 1.0) * 255.0) as u8;
            buf[off+18] = (colors[i].b.clamp(0.0, 1.0) * 255.0) as u8;
            buf[off+19] = (colors[i].a.clamp(0.0, 1.0) * 255.0) as u8;
            buf[off+20..off+24].copy_from_slice(&mode.to_le_bytes());
            buf[off+24..off+28].copy_from_slice(&stroke_w.to_le_bytes());
            buf[off+28..off+32].copy_from_slice(&size.0.to_le_bytes());
            buf[off+32..off+36].copy_from_slice(&size.1.to_le_bytes());
        }
        buf
    }

    // pack 4 sprite verts into byte array: pos(f32x2) + uv(f32x2) + tint_rgba(u8x4) per vert
    #[inline]
    fn pack_sprite_quad(
        positions: [(f32, f32); 4],
        uvs: [(f32, f32); 4],
        r: f32, g: f32, b: f32, a: f32,
    ) -> [u8; crate::batch::SPRITE_BYTES_PER_QUAD] {
        let mut buf = [0u8; crate::batch::SPRITE_BYTES_PER_QUAD];
        let rb = (r.clamp(0.0, 1.0) * 255.0) as u8;
        let gb = (g.clamp(0.0, 1.0) * 255.0) as u8;
        let bb = (b.clamp(0.0, 1.0) * 255.0) as u8;
        let ab = (a.clamp(0.0, 1.0) * 255.0) as u8;
        for i in 0..4 {
            let off = i * crate::batch::SPRITE_BYTES_PER_VERT;
            buf[off..off+4].copy_from_slice(&positions[i].0.to_le_bytes());
            buf[off+4..off+8].copy_from_slice(&positions[i].1.to_le_bytes());
            buf[off+8..off+12].copy_from_slice(&uvs[i].0.to_le_bytes());
            buf[off+12..off+16].copy_from_slice(&uvs[i].1.to_le_bytes());
            buf[off+16] = rb;
            buf[off+17] = gb;
            buf[off+18] = bb;
            buf[off+19] = ab;
        }
        buf
    }
}

impl GlowRenderer {
    /// create renderer with explicit vsync control
    pub fn new_with_vsync(window: &winit::window::Window, vsync: bool) -> Result<Self, RendererError> {
        let (gl, surface, gl_ctx) = context::create_gl_context_with_vsync(window, vsync)?;
        Self::init(gl, surface, gl_ctx, window)
    }

    fn init(
        gl: glow::Context,
        surface: context::Surface,
        gl_ctx: context::GlContext,
        window: &winit::window::Window,
    ) -> Result<Self, RendererError> {
        let size = window.inner_size();
        let scale = window.scale_factor() as f32;

        use crate::batch::MAX_IBO_QUADS;
        const INIT_VBO_CAP: usize = 65536; // 64kb initial

        // double-buffered vbos (2 each for shape and sprite)
        let shape_vbos: [glow::Buffer; 2] = unsafe {
            let mk = || {
                let buf = gl.create_buffer().expect("create shape vbo");
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(buf));
                gl.buffer_data_size(glow::ARRAY_BUFFER, INIT_VBO_CAP as i32, glow::DYNAMIC_DRAW);
                buf
            };
            [mk(), mk()]
        };
        let sprite_vbos: [glow::Buffer; 2] = unsafe {
            let mk = || {
                let buf = gl.create_buffer().expect("create sprite vbo");
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(buf));
                gl.buffer_data_size(glow::ARRAY_BUFFER, INIT_VBO_CAP as i32, glow::DYNAMIC_DRAW);
                buf
            };
            [mk(), mk()]
        };

        // shared quad index buffer: [0,1,2, 2,3,0] repeated
        let quad_ibo = unsafe {
            let buf = gl.create_buffer().expect("create quad ibo");
            let mut indices = Vec::with_capacity(MAX_IBO_QUADS * 6);
            for q in 0..MAX_IBO_QUADS as u16 {
                let base = q * 4;
                indices.push(base);
                indices.push(base + 1);
                indices.push(base + 2);
                indices.push(base + 2);
                indices.push(base + 3);
                indices.push(base);
            }
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(buf));
            let bytes = std::slice::from_raw_parts(
                indices.as_ptr() as *const u8,
                indices.len() * 2,
            );
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
            buf
        };

        // helper to wire shape vao attribs
        let wire_shape_vao = |gl: &glow::Context, vbo: glow::Buffer| -> glow::VertexArray { unsafe {
            let vao = gl.create_vertex_array().expect("create shape vao");
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = 36;
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, glow::UNSIGNED_BYTE, true, stride, 16);
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 1, glow::FLOAT, false, stride, 20);
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(4, 1, glow::FLOAT, false, stride, 24);
            gl.enable_vertex_attrib_array(5);
            gl.vertex_attrib_pointer_f32(5, 2, glow::FLOAT, false, stride, 28);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(quad_ibo));
            gl.bind_vertex_array(None);
            vao
        }};

        // helper to wire sprite vao attribs
        let wire_sprite_vao = |gl: &glow::Context, vbo: glow::Buffer| -> glow::VertexArray { unsafe {
            let vao = gl.create_vertex_array().expect("create sprite vao");
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = 20;
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, glow::UNSIGNED_BYTE, true, stride, 16);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(quad_ibo));
            gl.bind_vertex_array(None);
            vao
        }};

        // -- batched shape pipeline (2 vaos for double buffering) --
        let (shape_prog, shape_vaos, shape_loc_proj) = unsafe {
            let prog = shaders::compile_program(&gl, shaders::BATCH_SHAPE_VERT, shaders::BATCH_SHAPE_FRAG)?;
            let loc_proj = gl.get_uniform_location(prog, "u_proj");
            let vaos = [wire_shape_vao(&gl, shape_vbos[0]), wire_shape_vao(&gl, shape_vbos[1])];
            (prog, vaos, loc_proj)
        };

        // -- batched sprite pipeline (2 vaos for double buffering) --
        let (sprite_prog, sprite_vaos, sprite_loc_proj, sprite_loc_tex) = unsafe {
            let prog = shaders::compile_program(&gl, shaders::BATCH_SPRITE_VERT, shaders::BATCH_SPRITE_FRAG)?;
            let loc_proj = gl.get_uniform_location(prog, "u_proj");
            let loc_tex = gl.get_uniform_location(prog, "u_tex");
            let vaos = [wire_sprite_vao(&gl, sprite_vbos[0]), wire_sprite_vao(&gl, sprite_vbos[1])];
            (prog, vaos, loc_proj, loc_tex)
        };

        // -- sdf text pipeline (reuses sprite vao layout) --
        let (sdf_prog, sdf_loc_proj, sdf_loc_tex) = unsafe {
            let prog = shaders::compile_program(&gl, shaders::SDF_TEXT_VERT, shaders::SDF_TEXT_FRAG)?;
            let loc_proj = gl.get_uniform_location(prog, "u_proj");
            let loc_tex = gl.get_uniform_location(prog, "u_tex");
            (prog, loc_proj, loc_tex)
        };

        // -- instanced sprite pipeline --
        let (inst_sprite_prog, inst_sprite_vao, inst_sprite_loc_proj, inst_sprite_loc_tex,
             inst_quad_vbo, inst_quad_ibo, inst_data_vbo) = unsafe {
            let prog = shaders::compile_program(&gl, shaders::INSTANCED_SPRITE_VERT, shaders::BATCH_SPRITE_FRAG)?;
            let loc_proj = gl.get_uniform_location(prog, "u_proj");
            let loc_tex = gl.get_uniform_location(prog, "u_tex");

            // static unit quad vbo: 4 verts of vec2
            let qvbo = gl.create_buffer().expect("create inst quad vbo");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(qvbo));
            #[rustfmt::skip]
            let quad_verts: [f32; 8] = [
                0.0, 0.0,
                1.0, 0.0,
                1.0, 1.0,
                0.0, 1.0,
            ];
            let bytes = std::slice::from_raw_parts(quad_verts.as_ptr() as *const u8, 32);
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);

            // ibo for the unit quad
            let qibo = gl.create_buffer().expect("create inst quad ibo");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(qibo));
            let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];
            let ibytes = std::slice::from_raw_parts(indices.as_ptr() as *const u8, 12);
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, ibytes, glow::STATIC_DRAW);

            // per-instance data vbo (dynamic)
            let dvbo = gl.create_buffer().expect("create inst data vbo");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(dvbo));
            gl.buffer_data_size(glow::ARRAY_BUFFER, INIT_VBO_CAP as i32, glow::DYNAMIC_DRAW);

            // vao wiring
            let vao = gl.create_vertex_array().expect("create inst sprite vao");
            gl.bind_vertex_array(Some(vao));

            // loc 0: a_corner from static quad vbo
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(qvbo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

            // per-instance attribs from data vbo (stride = 40 bytes)
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(dvbo));
            let ist = 40;
            // loc 1: a_pos vec2 offset 0
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, ist, 0);
            gl.vertex_attrib_divisor(1, 1);
            // loc 2: a_scale vec2 offset 8
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, ist, 8);
            gl.vertex_attrib_divisor(2, 1);
            // loc 3: a_rot float offset 16
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 1, glow::FLOAT, false, ist, 16);
            gl.vertex_attrib_divisor(3, 1);
            // loc 4: a_uv_min vec2 offset 20
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(4, 2, glow::FLOAT, false, ist, 20);
            gl.vertex_attrib_divisor(4, 1);
            // loc 5: a_uv_max vec2 offset 28
            gl.enable_vertex_attrib_array(5);
            gl.vertex_attrib_pointer_f32(5, 2, glow::FLOAT, false, ist, 28);
            gl.vertex_attrib_divisor(5, 1);
            // loc 6: a_tint u8x4 normalized offset 36
            gl.enable_vertex_attrib_array(6);
            gl.vertex_attrib_pointer_f32(6, 4, glow::UNSIGNED_BYTE, true, ist, 36);
            gl.vertex_attrib_divisor(6, 1);

            // bind ibo
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(qibo));
            gl.bind_vertex_array(None);

            (prog, vao, loc_proj, loc_tex, qvbo, qibo, dvbo)
        };

        // -- post-processing effect pipelines --
        let (fx_gray, fx_inv, fx_bright, fx_bright_loc, fx_vign, fx_vao, fx_vbo) = unsafe {
            let gray = shaders::compile_program(&gl, shaders::EFFECT_VERT, shaders::EFFECT_GRAYSCALE_FRAG)?;
            let inv = shaders::compile_program(&gl, shaders::EFFECT_VERT, shaders::EFFECT_INVERT_FRAG)?;
            let bright = shaders::compile_program(&gl, shaders::EFFECT_VERT, shaders::EFFECT_BRIGHTNESS_FRAG)?;
            let bright_loc = gl.get_uniform_location(bright, "u_brightness");
            let vign = shaders::compile_program(&gl, shaders::EFFECT_VERT, shaders::EFFECT_VIGNETTE_FRAG)?;

            // fullscreen quad vao: 2 tris, pos + uv
            let vao = gl.create_vertex_array().expect("create effect vao");
            let vbo = gl.create_buffer().expect("create effect vbo");
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            // ndc fullscreen quad
            #[rustfmt::skip]
            let quad: [f32; 24] = [
                -1.0, -1.0,  0.0, 0.0,
                 1.0, -1.0,  1.0, 0.0,
                 1.0,  1.0,  1.0, 1.0,
                -1.0, -1.0,  0.0, 0.0,
                 1.0,  1.0,  1.0, 1.0,
                -1.0,  1.0,  0.0, 1.0,
            ];
            let bytes = std::slice::from_raw_parts(quad.as_ptr() as *const u8, quad.len() * 4);
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytes,
                glow::STATIC_DRAW,
            );

            let stride = 16; // 4 floats
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);

            gl.bind_vertex_array(None);

            (gray, inv, bright, bright_loc, vign, vao, vbo)
        };

        // -- blur + bloom effect programs --
        let (fx_blur_h, fx_blur_h_rloc, fx_blur_v, fx_blur_v_rloc,
             fx_bloom_thresh, fx_bloom_thresh_loc, fx_bloom_comp, fx_bloom_int_loc, fx_bloom_tex_loc) = unsafe {
            let blur_h = shaders::compile_program(&gl, shaders::EFFECT_VERT, shaders::EFFECT_BLUR_H_FRAG)?;
            let blur_h_r = gl.get_uniform_location(blur_h, "u_radius");
            let blur_v = shaders::compile_program(&gl, shaders::EFFECT_VERT, shaders::EFFECT_BLUR_V_FRAG)?;
            let blur_v_r = gl.get_uniform_location(blur_v, "u_radius");
            let bloom_t = shaders::compile_program(&gl, shaders::EFFECT_VERT, shaders::EFFECT_BLOOM_THRESHOLD_FRAG)?;
            let bloom_t_loc = gl.get_uniform_location(bloom_t, "u_threshold");
            let bloom_c = shaders::compile_program(&gl, shaders::EFFECT_VERT, shaders::EFFECT_BLOOM_COMPOSITE_FRAG)?;
            let bloom_i_loc = gl.get_uniform_location(bloom_c, "u_intensity");
            let bloom_tex_loc = gl.get_uniform_location(bloom_c, "u_bloom");
            (blur_h, blur_h_r, blur_v, blur_v_r, bloom_t, bloom_t_loc, bloom_c, bloom_i_loc, bloom_tex_loc)
        };

        unsafe {
            gl.viewport(0, 0, size.width as i32, size.height as i32);
        }

        Ok(Self {
            gl,
            surface,
            gl_ctx,
            clear_color: Color::BLACK,
            w: size.width,
            h: size.height,
            proj: screen_ortho(
                (size.width as f32 / scale) as u32,
                (size.height as f32 / scale) as u32,
            ),
            cam: Camera2D::new(size.width as f32 / scale, size.height as f32 / scale),
            shape_prog,
            shape_vaos,
            shape_loc_proj,
            sprite_prog,
            sprite_vaos,
            sprite_loc_proj,
            sprite_loc_tex,
            shape_vbos,
            sprite_vbos,
            shape_vbo_caps: [INIT_VBO_CAP; 2],
            sprite_vbo_caps: [INIT_VBO_CAP; 2],
            vbo_frame_idx: 0,
            quad_ibo,
            batcher: Batcher::new(),
            textures: HashMap::new(),
            next_tex_id: 1,
            font_system: lite_render_2d_core::font_atlas::FontSystem::new(),
            font_atlas_tex_id: None,
            font_atlas_gl_tex: None,
            transform_stack: lite_render_2d_core::transform_stack::TransformStack::new(),
            clip_stack: Vec::new(),
            current_clip: None,
            current_blend: BlendMode::Alpha,
            scale_factor: scale,
            effect_grayscale_prog: fx_gray,
            effect_invert_prog: fx_inv,
            effect_brightness_prog: fx_bright,
            effect_brightness_loc: fx_bright_loc,
            effect_vignette_prog: fx_vign,
            effect_vao: fx_vao,
            effect_vbo: fx_vbo,
            effect_blur_h_prog: fx_blur_h,
            effect_blur_h_radius_loc: fx_blur_h_rloc,
            effect_blur_v_prog: fx_blur_v,
            effect_blur_v_radius_loc: fx_blur_v_rloc,
            effect_bloom_threshold_prog: fx_bloom_thresh,
            effect_bloom_threshold_loc: fx_bloom_thresh_loc,
            effect_bloom_composite_prog: fx_bloom_comp,
            effect_bloom_intensity_loc: fx_bloom_int_loc,
            effect_bloom_tex_loc: fx_bloom_tex_loc,
            render_targets: HashMap::new(),
            next_rt_id: 1,
            active_render_target: None,
            saved_viewport: None,
            saved_proj: None,
            materials: HashMap::new(),
            next_material_id: 1,
            sdf_prog,
            sdf_loc_proj,
            sdf_loc_tex,
            sdf_font_system: lite_render_2d_core::sdf_font::SdfFontSystem::new(),
            sdf_atlas_tex_id: None,
            sdf_atlas_gl_tex: None,
            inst_sprite_prog,
            inst_sprite_vao,
            inst_sprite_loc_proj,
            inst_sprite_loc_tex,
            inst_quad_vbo,
            inst_quad_ibo,
            inst_data_vbo,
            inst_data_vbo_cap: INIT_VBO_CAP,
            sprite_atlas: lite_render_2d_core::atlas::TextureAtlas::new(2048, 2048),
            sprite_atlas_gl_tex: None,
            sprite_atlas_tex_id: None,
            atlas_region_map: HashMap::new(),
            atlas_dirty: false,
            atlas_image_indices: HashMap::new(),
            frame_start: Instant::now(),
            fps_samples: [0.0; 60],
            fps_idx: 0,
            gpu_ram_bytes: 0,
            stats_enabled: std::env::var("LITE_RENDER_STATS").map(|v| v == "1").unwrap_or(false),
            cached_gl_tex_map: HashMap::new(),
            tex_map_dirty: true,
        })
    }
}

impl Renderer for GlowRenderer {
    fn new(window: &winit::window::Window) -> Result<Self, RendererError>
    where
        Self: Sized,
    {
        let (gl, surface, gl_ctx) = context::create_gl_context(window)?;
        Self::init(gl, surface, gl_ctx, window)
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.w = width;
        self.h = height;
        // projection uses logical pixels, viewport uses physical
        let lw = (width as f32 / self.scale_factor) as u32;
        let lh = (height as f32 / self.scale_factor) as u32;
        self.proj = screen_ortho(lw, lh);
        self.surface.resize(
            &self.gl_ctx,
            NonZeroU32::new(width.max(1)).unwrap(),
            NonZeroU32::new(height.max(1)).unwrap(),
        );
        unsafe {
            self.gl.viewport(0, 0, width as i32, height as i32);
        }
    }

    fn set_camera(&mut self, camera: &Camera2D) {
        self.cam = *camera;
        self.proj = camera.projection_matrix();
    }

    fn camera(&self) -> &Camera2D {
        &self.cam
    }

    fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    fn set_blend_mode(&mut self, mode: BlendMode) {
        self.current_blend = mode;
    }

    fn begin_frame(&mut self) -> Result<(), RendererError> {
        self.frame_start = Instant::now();
        self.vbo_frame_idx = 1 - self.vbo_frame_idx;
        self.batcher.clear();
        let c = self.clear_color;
        unsafe {
            self.gl.clear_color(c.r, c.g, c.b, c.a);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        Ok(())
    }

    fn push_transform(&mut self, transform: Transform2D) {
        self.transform_stack.push(transform);
    }
    fn pop_transform(&mut self) {
        self.transform_stack.pop();
    }
    fn reset_transform(&mut self) {
        self.transform_stack.reset();
    }

    fn push_clip_rect(&mut self, rect: Rect) {
        let s = self.scale_factor;
        let new_clip = [
            (rect.pos.x.max(0.0) * s) as u32,
            (rect.pos.y.max(0.0) * s) as u32,
            (rect.size.x.max(0.0) * s) as u32,
            (rect.size.y.max(0.0) * s) as u32,
        ];
        // intersec with parent clip if any
        let clipped = match self.current_clip {
            Some(parent) => intersect_rects(parent, new_clip),
            None => new_clip,
        };
        self.clip_stack.push(clipped);
        self.current_clip = Some(clipped);
    }

    fn pop_clip_rect(&mut self) {
        self.clip_stack.pop();
        self.current_clip = self.clip_stack.last().copied();
    }

    fn draw_rect(&mut self, rect: Rect, params: DrawParams) {
        let (color, mode, stroke_w) = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                (c, 0.0_f32, 0.0_f32)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut c = sp.color;
                c.a *= params.opacity;
                (c, 1.0, sp.thickness)
            }
            DrawStyle::LinearGradient { color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                (c, 0.0, 0.0)
            }
            DrawStyle::RadialGradient { color_inner, .. } => {
                let mut c = color_inner;
                c.a *= params.opacity;
                (c, 0.0, 0.0)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                (c, 0.0, 0.0)
            }
        };

        let x = rect.pos.x;
        let y = rect.pos.y;
        let w = rect.size.x;
        let h = rect.size.y;

        // skip if fully offscreen
        if self.transform_stack.is_identity() && self.is_offscreen(x, y, x + w, y + h) {
            return;
        }

        // compute per-vertex colors (handles gradients)
        let positions = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
        let locals = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
        let mut verts = if matches!(params.style, DrawStyle::Fill(_) | DrawStyle::Stroke(_)) {
            Self::pack_shape_quad(positions, locals, color, mode, stroke_w, (w, h))
        } else {
            // build temp f32 quad, apply gradient, convert to bytes
            #[rustfmt::skip]
            let mut tmp: [f32; 48] = [
                x,     y,     0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
                x + w, y,     w,   0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
                x + w, y + h, w,   h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
                x,     y + h, 0.0, h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            ];
            lite_render_2d_core::tessellation::apply_gradient(&mut tmp, &params.style);
            // extract per-vertex colors from the gradient result
            let vc: [Color; 4] = std::array::from_fn(|i| {
                let b = i * 12;
                Color { r: tmp[b+4], g: tmp[b+5], b: tmp[b+6], a: tmp[b+7] }
            });
            Self::pack_shape_quad_gradient(positions, locals, vc, mode, stroke_w, (w, h))
        };

        self.apply_transform_quad(&mut verts);
        self.batcher.push_shape(&verts, params.z_index, params.blend, self.current_clip);
    }

    fn draw_rounded_rect(&mut self, rrect: RoundedRect, params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_rounded_rect_fill(rrect, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_rounded_rect_stroke(rrect, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_rounded_rect_fill(rrect, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_rounded_rect_fill(rrect, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_circle(&mut self, center: Vec2, radius: f32, params: DrawParams) {
        let (color, mode, stroke_w_norm) = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                (c, 2.0_f32, 0.0_f32)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut c = sp.color;
                c.a *= params.opacity;
                // normalzied inner edge for sdf ring
                let norm = 1.0 - sp.thickness / radius;
                (c, 3.0, norm)
            }
            DrawStyle::LinearGradient { color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                (c, 2.0, 0.0)
            }
            DrawStyle::RadialGradient { color_inner, .. } => {
                let mut c = color_inner;
                c.a *= params.opacity;
                (c, 2.0, 0.0)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                (c, 2.0, 0.0)
            }
        };

        // pad quad slightly for aa fringe
        let pad = 2.0;
        let ext = radius + pad;
        let cx = center.x;
        let cy = center.y;
        let ln = ext / radius;

        // skip if fully offscreen
        if self.transform_stack.is_identity() && self.is_offscreen(cx - ext, cy - ext, cx + ext, cy + ext) {
            return;
        }

        let positions = [(cx - ext, cy - ext), (cx + ext, cy - ext), (cx + ext, cy + ext), (cx - ext, cy + ext)];
        let locals = [(-ln, -ln), (ln, -ln), (ln, ln), (-ln, ln)];
        let mut verts = if matches!(params.style, DrawStyle::Fill(_) | DrawStyle::Stroke(_)) {
            Self::pack_shape_quad(positions, locals, color, mode, stroke_w_norm, (0.0, 0.0))
        } else {
            #[rustfmt::skip]
            let mut tmp: [f32; 48] = [
                cx - ext, cy - ext, -ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
                cx + ext, cy - ext,  ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
                cx + ext, cy + ext,  ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
                cx - ext, cy + ext, -ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            ];
            lite_render_2d_core::tessellation::apply_gradient(&mut tmp, &params.style);
            let vc: [Color; 4] = std::array::from_fn(|i| {
                let b = i * 12;
                Color { r: tmp[b+4], g: tmp[b+5], b: tmp[b+6], a: tmp[b+7] }
            });
            Self::pack_shape_quad_gradient(positions, locals, vc, mode, stroke_w_norm, (0.0, 0.0))
        };

        self.apply_transform_quad(&mut verts);
        self.batcher.push_shape(&verts, params.z_index, params.blend, self.current_clip);
    }

    fn draw_ellipse(&mut self, center: Vec2, radii: Vec2, params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_ellipse_fill(center, radii, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_ellipse_stroke(center, radii, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_ellipse_fill(center, radii, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_ellipse_fill(center, radii, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_arc(
        &mut self,
        center: Vec2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        params: DrawParams,
    ) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_arc_fill(center, radius, start_angle, end_angle, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_arc_stroke(center, radius, start_angle, end_angle, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_arc_fill(center, radius, start_angle, end_angle, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_arc_fill(center, radius, start_angle, end_angle, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_polygon(&mut self, points: &[Vec2], params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_convex_polygon(points, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_polygon_stroke(points, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_convex_polygon(points, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_convex_polygon(points, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_complex_polygon(&mut self, outer: &[Vec2], holes: &[&[Vec2]], params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::path_tessellation::tessellate_complex_polygon(outer, holes, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::path_tessellation::tessellate_complex_polygon_stroke(outer, holes, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::path_tessellation::tessellate_complex_polygon(outer, holes, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::path_tessellation::tessellate_complex_polygon(outer, holes, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(col) => {
                let mut col = col;
                col.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_triangle(a, b, c, col)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_triangle_stroke(a, b, c, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut col = color_start;
                col.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_triangle(a, b, c, col)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut col = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                col.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_triangle(a, b, c, col)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_line(&mut self, from: Vec2, to: Vec2, params: LineParams) {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }

        let half = params.thickness * 0.5;
        // perpendiculr direction
        let nx = -dy / len * half;
        let ny = dx / len * half;

        let mut color = params.color;
        color.a *= params.opacity;
        let mode = 4.0_f32;

        // skip if fully offscreen
        {
            let lx_min = (from.x + nx).min(to.x + nx).min(to.x - nx).min(from.x - nx);
            let lx_max = (from.x + nx).max(to.x + nx).max(to.x - nx).max(from.x - nx);
            let ly_min = (from.y + ny).min(to.y + ny).min(to.y - ny).min(from.y - ny);
            let ly_max = (from.y + ny).max(to.y + ny).max(to.y - ny).max(from.y - ny);
            if self.transform_stack.is_identity() && self.is_offscreen(lx_min, ly_min, lx_max, ly_max) {
                return;
            }
        }

        let positions = [
            (from.x + nx, from.y + ny),
            (to.x + nx, to.y + ny),
            (to.x - nx, to.y - ny),
            (from.x - nx, from.y - ny),
        ];
        let zeros = [(0.0, 0.0); 4];
        let mut verts = Self::pack_shape_quad(positions, zeros, color, mode, 0.0, (0.0, 0.0));

        self.apply_transform_quad(&mut verts);
        self.batcher.push_shape(&verts, params.z_index, params.blend, self.current_clip);
    }

    fn draw_polyline(&mut self, points: &[Vec2], params: LineParams) {
        let z_index = params.z_index;
        let blend = params.blend;
        let mut params = params;
        params.color.a *= params.opacity;
        let mut verts = lite_render_2d_core::tessellation::tessellate_polyline(points, &params);
        self.push_shape_raw_transformed(&mut verts, z_index, blend);
    }

    fn draw_path(&mut self, path: &Path, params: DrawParams) {
        let color = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                c
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                let mut verts = lite_render_2d_core::path_tessellation::tessellate_path_stroke(path, &sp);
                self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
                return;
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                c
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                c
            }
        };
        let mut verts = lite_render_2d_core::path_tessellation::tessellate_path_fill(path, color);
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn stroke_path(&mut self, path: &Path, params: StrokeParams) {
        let mut params = params;
        params.color.a *= 1.0;
        let mut verts = lite_render_2d_core::path_tessellation::tessellate_path_stroke(path, &params);
        self.push_shape_raw_transformed(&mut verts, 0, BlendMode::Alpha);
    }

    fn load_texture(
        &mut self,
        data: &[u8],
        params: TextureParams,
    ) -> Result<TextureHandle, RendererError> {
        let img = image::load_from_memory(data)
            .map_err(|e| RendererError::Texture(e.to_string()))?
            .into_rgba8();
        let (w, h) = img.dimensions();
        let pixels = img.into_raw();

        // try packing small textures into sprite atlas
        if w <= 256 && h <= 256 && params.wrap == WrapMode::Clamp {
            let mut region = self.sprite_atlas.add_image(&pixels, w, h);

            // if atlas full, try to grow and repack
            if region.is_none() {
                if let Some(new_regions) = self.sprite_atlas.grow() {
                    // update all existing atlas region mappings with new positions
                    for (&tex_id, &img_idx) in &self.atlas_image_indices {
                        if let Some(r) = new_regions.get(img_idx) {
                            self.atlas_region_map.insert(tex_id, *r);
                        }
                    }
                    // delete old gl texture, will be recreated on next upload
                    if let Some(old_tex) = self.sprite_atlas_gl_tex.take() {
                        let old_w = self.sprite_atlas.width;
                        let old_h = self.sprite_atlas.height;
                        self.gpu_ram_bytes = self.gpu_ram_bytes.saturating_sub((old_w as u64 / 2) * (old_h as u64) * 4);
                        unsafe { self.gl.delete_texture(old_tex); }
                    }
                    // retry after grow
                    region = self.sprite_atlas.add_image(&pixels, w, h);
                }
            }

            if let Some(region) = region {
                let id = self.next_tex_id;
                self.next_tex_id += 1;
                let img_idx = self.sprite_atlas.region_count() - 1;
                self.atlas_image_indices.insert(id, img_idx);
                self.atlas_region_map.insert(id, region);
                self.atlas_dirty = true;
                // store a textureinfo with original dims so texture_size() works
                let placeholder_tex = unsafe {
                    self.sprite_atlas_gl_tex.unwrap_or_else(|| {
                        self.gl.create_texture().expect("placeholder")
                    })
                };
                self.textures.insert(id, TextureInfo { gl_tex: placeholder_tex, width: w, height: h });
                self.tex_map_dirty = true;
                return Ok(TextureHandle::new(id));
            }
        }

        let gl_tex = unsafe {
            let tex = self.gl.create_texture().map_err(|e| RendererError::Texture(e))?;
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                w as i32,
                h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&pixels)),
            );

            let filter = match params.filter {
                FilterMode::Nearest => glow::NEAREST as i32,
                FilterMode::Linear => glow::LINEAR as i32,
            };
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter);

            let wrap = match params.wrap {
                WrapMode::Clamp => glow::CLAMP_TO_EDGE as i32,
                WrapMode::Repeat => glow::REPEAT as i32,
            };
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap);

            self.gl.bind_texture(glow::TEXTURE_2D, None);
            tex
        };

        let id = self.next_tex_id;
        self.next_tex_id += 1;
        self.textures.insert(id, TextureInfo { gl_tex, width: w, height: h });
        self.tex_map_dirty = true;
        self.gpu_ram_bytes += (w as u64) * (h as u64) * 4;
        Ok(TextureHandle::new(id))
    }

    fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)> {
        self.textures.get(&handle.id()).map(|i| (i.width, i.height))
    }

    fn unload_texture(&mut self, handle: TextureHandle) {
        if let Some(info) = self.textures.remove(&handle.id()) {
            unsafe {
                self.gl.delete_texture(info.gl_tex);
            }
            self.tex_map_dirty = true;
            self.gpu_ram_bytes = self.gpu_ram_bytes.saturating_sub((info.width as u64) * (info.height as u64) * 4);
        }
    }

    fn draw_sprite(&mut self, handle: TextureHandle, params: SpriteParams) {
        let info = match self.textures.get(&handle.id()) {
            Some(i) => i,
            None => return,
        };

        let tw = info.width as f32;
        let th = info.height as f32;

        // use src_rect dimensions for world-space size (not full texture)
        let (display_w, display_h) = match params.src_rect {
            Some(r) => (r.size.x, r.size.y),
            None => (tw, th),
        };

        let t = &params.transform;
        let sx = t.scale.x * display_w;
        let sy = t.scale.y * display_h;

        // compute uv bounds
        let (mut uv_min_x, mut uv_min_y, mut uv_max_x, mut uv_max_y) = match params.src_rect {
            Some(r) => (r.pos.x / tw, r.pos.y / th, (r.pos.x + r.size.x) / tw, (r.pos.y + r.size.y) / th),
            None => (0.0, 0.0, 1.0, 1.0),
        };
        if params.flip_x { std::mem::swap(&mut uv_min_x, &mut uv_max_x); }
        if params.flip_y { std::mem::swap(&mut uv_min_y, &mut uv_max_y); }

        // remap uvs to atlas space if this texture is atlas-packed
        let tex_id = if let Some(region) = self.atlas_region_map.get(&handle.id()) {
            if let Some(atlas_id) = self.sprite_atlas_tex_id {
                let aw = self.sprite_atlas.width as f32;
                let ah = self.sprite_atlas.height as f32;
                let ox = region.x as f32 / aw;
                let oy = region.y as f32 / ah;
                let sw = region.width as f32 / aw;
                let sh = region.height as f32 / ah;
                uv_min_x = ox + uv_min_x * sw;
                uv_min_y = oy + uv_min_y * sh;
                uv_max_x = ox + uv_max_x * sw;
                uv_max_y = oy + uv_max_y * sh;
                atlas_id
            } else {
                handle.id()
            }
        } else {
            handle.id()
        };

        let a = params.tint.a * params.opacity;

        // use instanced path when no transform stack (common case)
        if self.transform_stack.is_identity() {
            // frustum cull using aabb of the transformed quad
            let (cos, sin) = if t.rotation == 0.0 { (1.0_f32, 0.0_f32) } else { (t.rotation.cos(), t.rotation.sin()) };
            // compute aabb from rotated quad extents
            let abs_cos = cos.abs();
            let abs_sin = sin.abs();
            let half_w = (abs_cos * sx + abs_sin * sy) * 0.5;
            let half_h = (abs_sin * sx + abs_cos * sy) * 0.5;
            let cx = t.pos.x + (cos * sx - sin * sy) * 0.5;
            let cy = t.pos.y + (sin * sx + cos * sy) * 0.5;
            if self.is_offscreen(cx - half_w, cy - half_h, cx + half_w, cy + half_h) {
                return;
            }

            // pack 40 bytes of instance data
            let mut inst = [0u8; crate::batch::SPRITE_INST_SIZE];
            inst[0..4].copy_from_slice(&t.pos.x.to_le_bytes());
            inst[4..8].copy_from_slice(&t.pos.y.to_le_bytes());
            inst[8..12].copy_from_slice(&sx.to_le_bytes());
            inst[12..16].copy_from_slice(&sy.to_le_bytes());
            inst[16..20].copy_from_slice(&t.rotation.to_le_bytes());
            inst[20..24].copy_from_slice(&uv_min_x.to_le_bytes());
            inst[24..28].copy_from_slice(&uv_min_y.to_le_bytes());
            inst[28..32].copy_from_slice(&uv_max_x.to_le_bytes());
            inst[32..36].copy_from_slice(&uv_max_y.to_le_bytes());
            inst[36] = (params.tint.r.clamp(0.0, 1.0) * 255.0) as u8;
            inst[37] = (params.tint.g.clamp(0.0, 1.0) * 255.0) as u8;
            inst[38] = (params.tint.b.clamp(0.0, 1.0) * 255.0) as u8;
            inst[39] = (a.clamp(0.0, 1.0) * 255.0) as u8;

            self.batcher.push_sprite_inst(tex_id, &inst, params.z_index, params.blend, self.current_clip);
            return;
        }

        // fallback: quad path for non-identity transform stack
        let (cos, sin) = if t.rotation == 0.0 { (1.0_f32, 0.0_f32) } else { (t.rotation.cos(), t.rotation.sin()) };
        let transform = |px: f32, py: f32| -> (f32, f32) {
            let x = cos * sx * px + (-sin * sy) * py + t.pos.x;
            let y = sin * sx * px + cos * sy * py + t.pos.y;
            (x, y)
        };

        let (x0, y0) = transform(0.0, 0.0);
        let (x1, y1) = transform(1.0, 0.0);
        let (x2, y2) = transform(1.0, 1.0);
        let (x3, y3) = transform(0.0, 1.0);

        // bake uv corners from min/max
        let (u0, v0) = (uv_min_x, uv_min_y);
        let (u1, v1) = (uv_max_x, uv_min_y);
        let (u2, v2) = (uv_max_x, uv_max_y);
        let (u3, v3) = (uv_min_x, uv_max_y);

        let mut verts = Self::pack_sprite_quad(
            [(x0, y0), (x1, y1), (x2, y2), (x3, y3)],
            [(u0, v0), (u1, v1), (u2, v2), (u3, v3)],
            params.tint.r, params.tint.g, params.tint.b, a,
        );

        self.apply_transform_sprite(&mut verts);
        self.batcher.push_sprite(tex_id, &verts, params.z_index, params.blend, self.current_clip);
    }

    fn load_font(&mut self, data: &[u8]) -> Result<FontHandle, RendererError> {
        self.font_system.load_font(data).map_err(|e| RendererError::Font(e))
    }

    fn unload_font(&mut self, handle: FontHandle) {
        self.font_system.unload_font(handle);
    }

    fn draw_text(&mut self, text: &str, params: &TextParams) {
        let quads = self.font_system.layout_text(text, params);
        if quads.is_empty() {
            return;
        }

        // ensure atlas texture is uploaded
        if self.font_atlas_gl_tex.is_none() {
            // first time: create gl texture with full atlas data
            let (data, w, h) = self.font_system.atlas_texture_data();
            let gl_tex = unsafe {
                let tex = self.gl.create_texture().expect("create font atlas tex");
                self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D, 0, glow::RGBA8 as i32,
                    w as i32, h as i32, 0,
                    glow::RGBA, glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(data)),
                );
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                self.gl.bind_texture(glow::TEXTURE_2D, None);
                tex
            };
            self.font_atlas_gl_tex = Some(gl_tex);
            let id = self.next_tex_id;
            self.next_tex_id += 1;
            self.textures.insert(id, TextureInfo { gl_tex, width: w, height: h });
            self.font_atlas_tex_id = Some(id);
            self.tex_map_dirty = true;
            self.gpu_ram_bytes += (w as u64) * (h as u64) * 4;
            self.font_system.clear_dirty();
        } else if let Some((dx, dy, dw, dh)) = self.font_system.dirty_region() {
            // partial upload: only the changed region
            let sub = self.font_system.atlas_sub_data(dx, dy, dw, dh);
            let gl_tex = self.font_atlas_gl_tex.unwrap();
            unsafe {
                self.gl.bind_texture(glow::TEXTURE_2D, Some(gl_tex));
                self.gl.tex_sub_image_2d(
                    glow::TEXTURE_2D, 0,
                    dx as i32, dy as i32, dw as i32, dh as i32,
                    glow::RGBA, glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&sub)),
                );
                self.gl.bind_texture(glow::TEXTURE_2D, None);
            }
            self.font_system.clear_dirty();
        }

        let atlas_id = self.font_atlas_tex_id.unwrap();

        // emit each glyph as a sprite quad
        for q in &quads {
            let x = q.pos.x;
            let y = q.pos.y;
            let w = q.size.x;
            let h = q.size.y;
            let u0 = q.uv.pos.x;
            let v0 = q.uv.pos.y;
            let u1 = u0 + q.uv.size.x;
            let v1 = v0 + q.uv.size.y;
            let verts = Self::pack_sprite_quad(
                [(x, y), (x + w, y), (x + w, y + h), (x, y + h)],
                [(u0, v0), (u1, v0), (u1, v1), (u0, v1)],
                q.color.r, q.color.g, q.color.b, q.color.a,
            );

            self.batcher.push_sprite(atlas_id, &verts, 0, BlendMode::Alpha, self.current_clip);
        }
    }

    fn measure_text(&mut self, text: &str, params: &TextParams) -> Vec2 {
        self.font_system.measure_text(text, params)
    }

    fn end_frame(&mut self) -> Result<FrameStats, RendererError> {
        // if still rendering to texture, auto-end it
        if self.active_render_target.is_some() {
            self.end_render_to_texture();
        }

        self.flush_batch();

        unsafe { self.gl.flush(); }

        self.surface
            .swap_buffers(&self.gl_ctx)
            .expect("swap buffers");

        // compute frame stats
        let frame_time_ms = self.frame_start.elapsed().as_secs_f64() * 1000.0;
        self.fps_samples[self.fps_idx] = frame_time_ms;
        self.fps_idx = (self.fps_idx + 1) % 60;
        let avg_ms = self.fps_samples.iter().sum::<f64>() / 60.0;
        let fps = if avg_ms > 0.0 { 1000.0 / avg_ms } else { 0.0 };

        let stats = FrameStats {
            frame_time_ms,
            draw_calls: self.batcher.draw_calls(),
            vertices: self.batcher.vertices(),
            texture_binds: self.batcher.texture_binds(),
            batch_flushes: 1,
            ram_bytes: self.gpu_ram_bytes,
            fps,
        };

        if self.stats_enabled {
            println!(
                "[stats] {:.1}ms | {:.0}fps | draws:{} verts:{} tex_binds:{} ram:{}KB",
                stats.frame_time_ms, stats.fps, stats.draw_calls,
                stats.vertices, stats.texture_binds, stats.ram_bytes / 1024
            );
        }

        Ok(stats)
    }

    fn create_render_target(&mut self, width: u32, height: u32) -> Result<RenderTargetHandle, RendererError> {
        unsafe {
            let color_tex = self.gl.create_texture().map_err(|e| RendererError::Texture(e))?;
            self.gl.bind_texture(glow::TEXTURE_2D, Some(color_tex));
            let empty = vec![0u8; (width * height * 4) as usize];
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&empty)),
            );
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);

            let fbo = self.gl.create_framebuffer().map_err(|e| RendererError::Other(e))?;
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            self.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(color_tex),
                0,
            );

            let status = self.gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                self.gl.delete_framebuffer(fbo);
                self.gl.delete_texture(color_tex);
                return Err(RendererError::Other(format!("framebuffer incomplete: {status}")));
            }

            // unbind fbo
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl.bind_texture(glow::TEXTURE_2D, None);

            // register the texture so draw_sprite can use it
            let tex_id = self.next_tex_id;
            self.next_tex_id += 1;
            self.textures.insert(tex_id, TextureInfo {
                gl_tex: color_tex,
                width,
                height,
            });
            self.tex_map_dirty = true;
            self.gpu_ram_bytes += (width as u64) * (height as u64) * 4;

            let rt_id = self.next_rt_id;
            self.next_rt_id += 1;
            self.render_targets.insert(rt_id, RenderTargetInfo {
                fbo,
                color_tex,
                texture_handle_id: tex_id,
                width,
                height,
            });

            Ok(RenderTargetHandle::new(rt_id))
        }
    }

    fn destroy_render_target(&mut self, target: RenderTargetHandle) {
        if let Some(rt) = self.render_targets.remove(&target.id()) {
            self.textures.remove(&rt.texture_handle_id);
            unsafe {
                self.gl.delete_framebuffer(rt.fbo);
                self.gl.delete_texture(rt.color_tex);
            }
            self.tex_map_dirty = true;
            self.gpu_ram_bytes = self.gpu_ram_bytes.saturating_sub((rt.width as u64) * (rt.height as u64) * 4);
        }
    }

    fn begin_render_to_texture(&mut self, target: RenderTargetHandle) -> Result<(), RendererError> {
        // flush pending draws to current target first
        self.flush_batch();
        self.batcher.clear();

        let rt_id = target.id();
        let rt = self.render_targets.get(&rt_id)
            .ok_or_else(|| RendererError::Other("invalid render target".into()))?;

        let (rt_w, rt_h, fbo) = (rt.width, rt.height, rt.fbo);

        // save current state
        self.saved_viewport = Some((self.w, self.h));
        self.saved_proj = Some(self.proj);

        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            self.gl.viewport(0, 0, rt_w as i32, rt_h as i32);
            self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }

        self.proj = screen_ortho(rt_w, rt_h);
        self.active_render_target = Some(rt_id);
        Ok(())
    }

    fn end_render_to_texture(&mut self) {
        if self.active_render_target.is_none() {
            return;
        }

        // flush draws to the fbo
        self.flush_batch();
        self.batcher.clear();

        // restore default framebuffer
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }

        if let Some((w, h)) = self.saved_viewport.take() {
            self.w = w;
            self.h = h;
            unsafe {
                self.gl.viewport(0, 0, w as i32, h as i32);
            }
        }

        if let Some(proj) = self.saved_proj.take() {
            self.proj = proj;
        }

        // restore clear color
        let c = self.clear_color;
        unsafe {
            self.gl.clear_color(c.r, c.g, c.b, c.a);
        }

        self.active_render_target = None;
    }

    fn render_target_texture(&self, target: RenderTargetHandle) -> Option<TextureHandle> {
        self.render_targets.get(&target.id())
            .map(|rt| TextureHandle::new(rt.texture_handle_id))
    }

    fn apply_post_effect(&mut self, effect: &PostEffect, source: RenderTargetHandle) {
        // flush any pending draws first
        self.flush_batch();

        let rt_info = match self.render_targets.get(&source.id()) {
            Some(rt) => (rt.texture_handle_id, rt.width, rt.height, rt.fbo),
            None => return,
        };
        let (tex_id, rt_w, rt_h, rt_fbo) = rt_info;
        let gl_tex = match self.textures.get(&tex_id) {
            Some(info) => info.gl_tex,
            None => return,
        };

        match effect {
            PostEffect::Blur(radius) => {
                self.apply_blur(gl_tex, rt_fbo, rt_w, rt_h, *radius);
                return;
            }
            PostEffect::Bloom { threshold, intensity, radius } => {
                self.apply_bloom(gl_tex, rt_fbo, rt_w, rt_h, *threshold, *intensity, *radius);
                return;
            }
            _ => {}
        }

        let prog = match effect {
            PostEffect::Grayscale => self.effect_grayscale_prog,
            PostEffect::Invert => self.effect_invert_prog,
            PostEffect::Brightness(_) => self.effect_brightness_prog,
            PostEffect::Vignette => self.effect_vignette_prog,
            _ => return,
        };

        unsafe {
            self.gl.use_program(Some(prog));

            // set brightness uniform if needed
            if let PostEffect::Brightness(val) = effect {
                if let Some(ref loc) = self.effect_brightness_loc {
                    self.gl.uniform_1_f32(Some(loc), *val);
                }
            }

            // bind source texture
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(gl_tex));

            // draw fullscreen quad
            self.gl.bind_vertex_array(Some(self.effect_vao));
            self.gl.draw_arrays(glow::TRIANGLES, 0, 6);
            self.gl.bind_vertex_array(None);
        }
    }

    fn create_material(&mut self, frag_src: &str) -> Result<lite_render_2d_core::types::MaterialHandle, RendererError> {
        let prog = unsafe {
            shaders::compile_program(&self.gl, shaders::BATCH_SPRITE_VERT, frag_src)?
        };
        let id = self.next_material_id;
        self.next_material_id += 1;
        self.materials.insert(id, prog);
        Ok(lite_render_2d_core::types::MaterialHandle::new(id))
    }

    fn destroy_material(&mut self, material: lite_render_2d_core::types::MaterialHandle) {
        if let Some(prog) = self.materials.remove(&material.id()) {
            unsafe { self.gl.delete_program(prog); }
        }
    }

    fn draw_sprite_with_material(
        &mut self,
        handle: TextureHandle,
        material: &lite_render_2d_core::types::MaterialHandle,
        uniforms: &[(&str, lite_render_2d_core::types::UniformValue)],
        params: SpriteParams,
    ) {
        // flush current batch, draw this sprite with custom shader
        self.flush_batch();
        let prog = match self.materials.get(&material.id()) {
            Some(p) => *p,
            None => return,
        };
        let info = match self.textures.get(&handle.id()) {
            Some(i) => i,
            None => return,
        };

        unsafe {
            self.gl.use_program(Some(prog));
            // set projection
            let loc = self.gl.get_uniform_location(prog, "u_proj");
            if let Some(ref loc) = loc {
                self.gl.uniform_matrix_4_f32_slice(Some(loc), false, &self.proj);
            }
            // set texture
            let tex_loc = self.gl.get_uniform_location(prog, "u_tex");
            if let Some(ref loc) = tex_loc {
                self.gl.uniform_1_i32(Some(loc), 0);
            }
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(info.gl_tex));

            // set custom uniforms
            for (name, val) in uniforms {
                let uloc = self.gl.get_uniform_location(prog, name);
                if let Some(ref uloc) = uloc {
                    match val {
                        lite_render_2d_core::types::UniformValue::Float(v) => self.gl.uniform_1_f32(Some(uloc), *v),
                        lite_render_2d_core::types::UniformValue::Vec2(v) => self.gl.uniform_2_f32(Some(uloc), v.x, v.y),
                        lite_render_2d_core::types::UniformValue::Vec4(c) => self.gl.uniform_4_f32(Some(uloc), c.r, c.g, c.b, c.a),
                        lite_render_2d_core::types::UniformValue::Int(v) => self.gl.uniform_1_i32(Some(uloc), *v),
                    }
                }
            }
        }

        // draw the sprite with the material's shader (reuse sprite draw logic)
        self.draw_sprite(handle, params);
        self.flush_batch();
    }

    fn read_pixels(&self, target: RenderTargetHandle) -> Result<Vec<u8>, RendererError> {
        let rt = self.render_targets.get(&target.id())
            .ok_or_else(|| RendererError::Other("invalid render target".into()))?;
        let (w, h) = (rt.width, rt.height);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(rt.fbo));
            self.gl.read_pixels(
                0, 0, w as i32, h as i32,
                glow::RGBA, glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut buf)),
            );
            // restore fbo
            if let Some(active_id) = self.active_render_target {
                if let Some(art) = self.render_targets.get(&active_id) {
                    self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(art.fbo));
                }
            } else {
                self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            }
        }
        // gl reads bottom-up, flip rows
        let stride = (w * 4) as usize;
        let mut flipped = vec![0u8; buf.len()];
        for row in 0..h as usize {
            let src = row * stride;
            let dst = (h as usize - 1 - row) * stride;
            flipped[dst..dst + stride].copy_from_slice(&buf[src..src + stride]);
        }
        Ok(flipped)
    }

    fn begin_stencil_write(&mut self) {
        self.flush_batch();
        unsafe {
            self.gl.enable(glow::STENCIL_TEST);
            self.gl.clear(glow::STENCIL_BUFFER_BIT);
            self.gl.stencil_func(glow::ALWAYS, 1, 0xFF);
            self.gl.stencil_op(glow::KEEP, glow::KEEP, glow::REPLACE);
            self.gl.stencil_mask(0xFF);
            // dont write to color buffer while building mask
            self.gl.color_mask(false, false, false, false);
        }
    }

    fn end_stencil_write(&mut self) {
        self.flush_batch();
        unsafe {
            // now draw only where stencil == 1
            self.gl.color_mask(true, true, true, true);
            self.gl.stencil_func(glow::EQUAL, 1, 0xFF);
            self.gl.stencil_mask(0x00); // dont modify stencil anymore
        }
    }

    fn pop_stencil_mask(&mut self) {
        self.flush_batch();
        unsafe {
            self.gl.disable(glow::STENCIL_TEST);
        }
    }

    fn draw_sprite_instanced(
        &mut self,
        handle: TextureHandle,
        instances: &[SpriteInstance],
        blend: BlendMode,
        _z_index: i32,
    ) {
        if instances.is_empty() {
            return;
        }

        // fallback to individual draws when transform stack is active
        if !self.transform_stack.is_identity() {
            for inst in instances {
                self.draw_sprite(handle, SpriteParams {
                    transform: inst.transform,
                    tint: inst.tint,
                    opacity: inst.opacity,
                    src_rect: inst.src_rect,
                    flip_x: inst.flip_x,
                    flip_y: inst.flip_y,
                    blend,
                    z_index: _z_index,
                });
            }
            return;
        }

        let info = match self.textures.get(&handle.id()) {
            Some(i) => (i.width, i.height),
            None => return,
        };
        let (tw, th) = (info.0 as f32, info.1 as f32);

        // flush current batch to maintain draw order
        self.flush_batch();

        // resolve atlas mapping once for all instances
        let (tex_id, atlas_remap) = if let Some(region) = self.atlas_region_map.get(&handle.id()) {
            if let Some(atlas_id) = self.sprite_atlas_tex_id {
                let aw = self.sprite_atlas.width as f32;
                let ah = self.sprite_atlas.height as f32;
                (atlas_id, Some((region.x as f32 / aw, region.y as f32 / ah,
                                 region.width as f32 / aw, region.height as f32 / ah)))
            } else {
                (handle.id(), None)
            }
        } else {
            (handle.id(), None)
        };

        let gl_tex = match self.cached_gl_tex_map.get(&tex_id) {
            Some(t) => *t,
            None => {
                self.rebuild_tex_map_if_dirty();
                match self.cached_gl_tex_map.get(&tex_id) {
                    Some(t) => *t,
                    None => return,
                }
            }
        };

        // build per-instance data (40 bytes each)
        const INST_SIZE: usize = 40;
        let mut data = Vec::with_capacity(instances.len() * INST_SIZE);

        for inst in instances {
            let (dw, dh) = match inst.src_rect {
                Some(r) => (r.size.x, r.size.y),
                None => (tw, th),
            };

            let t = &inst.transform;
            let sx = t.scale.x * dw;
            let sy = t.scale.y * dh;

            // uv bounds
            let (mut uv_min_x, mut uv_min_y, mut uv_max_x, mut uv_max_y) = match inst.src_rect {
                Some(r) => (r.pos.x / tw, r.pos.y / th, (r.pos.x + r.size.x) / tw, (r.pos.y + r.size.y) / th),
                None => (0.0, 0.0, 1.0, 1.0),
            };

            // apply flip
            if inst.flip_x { std::mem::swap(&mut uv_min_x, &mut uv_max_x); }
            if inst.flip_y { std::mem::swap(&mut uv_min_y, &mut uv_max_y); }

            // remap to atlas space if needed
            if let Some((ox, oy, sw, sh)) = atlas_remap {
                uv_min_x = ox + uv_min_x * sw;
                uv_min_y = oy + uv_min_y * sh;
                uv_max_x = ox + uv_max_x * sw;
                uv_max_y = oy + uv_max_y * sh;
            }

            let a = inst.tint.a * inst.opacity;

            // pack 40 bytes: pos(8) + scale(8) + rot(4) + uv_min(8) + uv_max(8) + tint(4)
            data.extend_from_slice(&t.pos.x.to_le_bytes());
            data.extend_from_slice(&t.pos.y.to_le_bytes());
            data.extend_from_slice(&sx.to_le_bytes());
            data.extend_from_slice(&sy.to_le_bytes());
            data.extend_from_slice(&t.rotation.to_le_bytes());
            data.extend_from_slice(&uv_min_x.to_le_bytes());
            data.extend_from_slice(&uv_min_y.to_le_bytes());
            data.extend_from_slice(&uv_max_x.to_le_bytes());
            data.extend_from_slice(&uv_max_y.to_le_bytes());
            data.push((inst.tint.r.clamp(0.0, 1.0) * 255.0) as u8);
            data.push((inst.tint.g.clamp(0.0, 1.0) * 255.0) as u8);
            data.push((inst.tint.b.clamp(0.0, 1.0) * 255.0) as u8);
            data.push((a.clamp(0.0, 1.0) * 255.0) as u8);
        }

        // upload instance data
        let needed = data.len();
        unsafe {
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.inst_data_vbo));
            if needed > self.inst_data_vbo_cap {
                let mut new_cap = self.inst_data_vbo_cap.max(65536);
                while new_cap < needed { new_cap *= 2; }
                self.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &data, glow::DYNAMIC_DRAW);
                self.inst_data_vbo_cap = new_cap;
            } else {
                self.gl.buffer_data_size(glow::ARRAY_BUFFER, self.inst_data_vbo_cap as i32, glow::DYNAMIC_DRAW);
                self.gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, &data);
            }

            // set up gl state and draw
            self.gl.enable(glow::BLEND);
            match blend {
                BlendMode::Alpha => self.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA),
                BlendMode::Additive => self.gl.blend_func(glow::SRC_ALPHA, glow::ONE),
                BlendMode::Multiply => self.gl.blend_func(glow::DST_COLOR, glow::ONE_MINUS_SRC_ALPHA),
                BlendMode::Screen => self.gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_COLOR),
                BlendMode::PremultipliedAlpha => self.gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA),
            }

            self.gl.use_program(Some(self.inst_sprite_prog));
            if let Some(ref loc) = self.inst_sprite_loc_proj {
                self.gl.uniform_matrix_4_f32_slice(Some(loc), false, &self.proj);
            }
            if let Some(ref loc) = self.inst_sprite_loc_tex {
                self.gl.uniform_1_i32(Some(loc), 0);
            }
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(gl_tex));
            self.gl.bind_vertex_array(Some(self.inst_sprite_vao));
            self.gl.draw_elements_instanced(
                glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0, instances.len() as i32,
            );
            self.gl.bind_vertex_array(None);
        }
    }

    // -- sdf text --

    fn load_sdf_font(&mut self, data: &[u8]) -> Result<FontHandle, RendererError> {
        self.sdf_font_system.load_font(data).map_err(|e| RendererError::Font(e))
    }

    fn unload_sdf_font(&mut self, handle: FontHandle) {
        self.sdf_font_system.unload_font(handle);
    }

    fn draw_sdf_text(&mut self, text: &str, params: &TextParams) {
        let quads = self.sdf_font_system.layout_text(text, params);
        if quads.is_empty() {
            return;
        }

        // upload sdf atlas if dirty
        if self.sdf_atlas_gl_tex.is_none() {
            // first time: create gl texture with full atlas
            let (data, w, h) = self.sdf_font_system.atlas_texture_data();
            let gl_tex = unsafe {
                let tex = self.gl.create_texture().expect("create sdf atlas");
                self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D, 0, glow::RGBA8 as i32,
                    w as i32, h as i32, 0,
                    glow::RGBA, glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(data)),
                );
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                self.gl.bind_texture(glow::TEXTURE_2D, None);
                tex
            };
            self.sdf_atlas_gl_tex = Some(gl_tex);
            let id = self.next_tex_id;
            self.next_tex_id += 1;
            self.textures.insert(id, TextureInfo { gl_tex, width: w, height: h });
            self.sdf_atlas_tex_id = Some(id);
            self.tex_map_dirty = true;
            self.gpu_ram_bytes += (w as u64) * (h as u64) * 4;
            self.sdf_font_system.clear_dirty();
        } else if let Some((dx, dy, dw, dh)) = self.sdf_font_system.dirty_region() {
            // partial upload: only the changed region
            let sub = self.sdf_font_system.atlas_sub_data(dx, dy, dw, dh);
            let gl_tex = self.sdf_atlas_gl_tex.unwrap();
            unsafe {
                self.gl.bind_texture(glow::TEXTURE_2D, Some(gl_tex));
                self.gl.tex_sub_image_2d(
                    glow::TEXTURE_2D, 0,
                    dx as i32, dy as i32, dw as i32, dh as i32,
                    glow::RGBA, glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&sub)),
                );
                self.gl.bind_texture(glow::TEXTURE_2D, None);
            }
            self.sdf_font_system.clear_dirty();
        }

        let atlas_id = self.sdf_atlas_tex_id.unwrap();

        for q in &quads {
            let x = q.pos.x;
            let y = q.pos.y;
            let w = q.size.x;
            let h = q.size.y;
            let u0 = q.uv.pos.x;
            let v0 = q.uv.pos.y;
            let u1 = u0 + q.uv.size.x;
            let v1 = v0 + q.uv.size.y;
            let verts = Self::pack_sprite_quad(
                [(x, y), (x + w, y), (x + w, y + h), (x, y + h)],
                [(u0, v0), (u1, v0), (u1, v1), (u0, v1)],
                q.color.r, q.color.g, q.color.b, q.color.a,
            );

            self.batcher.push_sdf_sprite(atlas_id, &verts, 0, BlendMode::Alpha, self.current_clip);
        }
    }

    fn measure_sdf_text(&mut self, text: &str, params: &TextParams) -> Vec2 {
        self.sdf_font_system.measure_text(text, params)
    }

    // -- rich text --

    fn draw_rich_text(&mut self, rich: &lite_render_2d_core::rich_text::RichText) {
        let quads = lite_render_2d_core::rich_text::layout_rich_text(rich, &mut self.font_system);
        if quads.is_empty() {
            return;
        }

        // ensure font atlas is uploaded (same as draw_text)
        if self.font_atlas_gl_tex.is_none() {
            let (data, w, h) = self.font_system.atlas_texture_data();
            let gl_tex = unsafe {
                let tex = self.gl.create_texture().expect("create font atlas tex");
                self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D, 0, glow::RGBA8 as i32,
                    w as i32, h as i32, 0,
                    glow::RGBA, glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(data)),
                );
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                self.gl.bind_texture(glow::TEXTURE_2D, None);
                tex
            };
            self.font_atlas_gl_tex = Some(gl_tex);
            let id = self.next_tex_id;
            self.next_tex_id += 1;
            self.textures.insert(id, TextureInfo { gl_tex, width: w, height: h });
            self.font_atlas_tex_id = Some(id);
            self.tex_map_dirty = true;
            self.gpu_ram_bytes += (w as u64) * (h as u64) * 4;
            self.font_system.clear_dirty();
        } else if let Some((dx, dy, dw, dh)) = self.font_system.dirty_region() {
            let sub = self.font_system.atlas_sub_data(dx, dy, dw, dh);
            let gl_tex = self.font_atlas_gl_tex.unwrap();
            unsafe {
                self.gl.bind_texture(glow::TEXTURE_2D, Some(gl_tex));
                self.gl.tex_sub_image_2d(
                    glow::TEXTURE_2D, 0,
                    dx as i32, dy as i32, dw as i32, dh as i32,
                    glow::RGBA, glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&sub)),
                );
                self.gl.bind_texture(glow::TEXTURE_2D, None);
            }
            self.font_system.clear_dirty();
        }

        let atlas_id = self.font_atlas_tex_id.unwrap();

        for q in &quads {
            let x = q.pos.x;
            let y = q.pos.y;
            let w = q.size.x;
            let h = q.size.y;
            let u0 = q.uv.pos.x;
            let v0 = q.uv.pos.y;
            let u1 = u0 + q.uv.size.x;
            let v1 = v0 + q.uv.size.y;
            let verts = Self::pack_sprite_quad(
                [(x, y), (x + w, y), (x + w, y + h), (x, y + h)],
                [(u0, v0), (u1, v0), (u1, v1), (u0, v1)],
                q.color.r, q.color.g, q.color.b, q.color.a,
            );

            self.batcher.push_sprite(atlas_id, &verts, 0, BlendMode::Alpha, self.current_clip);
        }
    }

    fn measure_rich_text(&mut self, rich: &lite_render_2d_core::rich_text::RichText) -> Vec2 {
        lite_render_2d_core::rich_text::measure_rich_text(rich, &mut self.font_system)
    }
}
