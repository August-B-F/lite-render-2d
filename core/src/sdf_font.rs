use crate::text::{FontHandle, TextAlign, TextParams};
use crate::types::{Rect, Vec2};
use crate::font_atlas::GlyphQuad;

use std::collections::HashMap;

const SDF_ATLAS_SIZE: u32 = 512;
const BASE_RENDER_SIZE: f32 = 64.0;
const SDF_SPREAD: f32 = 6.0;

// sdf glyphs are size-independnt, keyed only by font+char
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct SdfGlyphKey {
    font_id: u64,
    ch: char,
}

struct SdfGlyphInfo {
    uv: Rect,
    // metrics normalized to 1.0 = base render size
    offset_norm: Vec2,
    advance_norm: f32,
    em_w: f32,
    em_h: f32,
}

pub struct SdfFontSystem {
    fonts: HashMap<u64, fontdue::Font>,
    atlas_data: Vec<u8>,
    atlas_w: u32,
    atlas_h: u32,
    dirty_rect: Option<(u32, u32, u32, u32)>,
    glyph_cache: HashMap<SdfGlyphKey, SdfGlyphInfo>,
    shelf_x: u32,
    shelf_y: u32,
    shelf_row_h: u32,
    next_font_id: u64,
}

impl Default for SdfFontSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SdfFontSystem {
    pub fn new() -> Self {
        let sz = (SDF_ATLAS_SIZE * SDF_ATLAS_SIZE * 4) as usize;
        Self {
            fonts: HashMap::new(),
            atlas_data: vec![0u8; sz],
            atlas_w: SDF_ATLAS_SIZE,
            atlas_h: SDF_ATLAS_SIZE,
            dirty_rect: None,
            glyph_cache: HashMap::new(),
            shelf_x: 0,
            shelf_y: 0,
            shelf_row_h: 0,
            next_font_id: 1,
        }
    }

    pub fn load_font(&mut self, data: &[u8]) -> Result<FontHandle, String> {
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())
            .map_err(|e| e.to_string())?;
        let id = self.next_font_id;
        self.next_font_id += 1;
        self.fonts.insert(id, font);
        Ok(FontHandle::new(id))
    }

    pub fn unload_font(&mut self, handle: FontHandle) {
        self.fonts.remove(&handle.id());
    }

    fn ensure_glyph(&mut self, font_id: u64, ch: char) {
        let key = SdfGlyphKey { font_id, ch };
        if self.glyph_cache.contains_key(&key) {
            return;
        }

        let font = match self.fonts.get(&font_id) {
            Some(f) => f,
            None => return,
        };

        let (metrics, bitmap) = font.rasterize(ch, BASE_RENDER_SIZE);
        if metrics.width == 0 || metrics.height == 0 {
            // whitespace
            self.glyph_cache.insert(key, SdfGlyphInfo {
                uv: Rect { pos: Vec2::ZERO, size: Vec2::ZERO },
                offset_norm: Vec2::new(
                    metrics.xmin as f32 / BASE_RENDER_SIZE,
                    metrics.ymin as f32 / BASE_RENDER_SIZE,
                ),
                advance_norm: metrics.advance_width / BASE_RENDER_SIZE,
                em_w: 0.0,
                em_h: 0.0,
            });
            return;
        }

        // compute sdf from rasterized bitmap
        let sdf = compute_sdf(&bitmap, metrics.width, metrics.height, SDF_SPREAD);
        let gw = metrics.width as u32;
        let gh = metrics.height as u32;
        let pad = 2u32;

        // shelf pack into atlas
        if self.shelf_x + gw + pad > self.atlas_w {
            self.shelf_y += self.shelf_row_h + pad;
            self.shelf_x = 0;
            self.shelf_row_h = 0;
        }

        if self.shelf_y + gh + pad > self.atlas_h {
            return; // atlas full
        }

        let ax = self.shelf_x;
        let ay = self.shelf_y;

        // copy sdf into atlas (white + sdf in alpha)
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let src = row * metrics.width + col;
                let dst_x = ax + col as u32;
                let dst_y = ay + row as u32;
                let dst = ((dst_y * self.atlas_w + dst_x) * 4) as usize;
                self.atlas_data[dst] = 255;
                self.atlas_data[dst + 1] = 255;
                self.atlas_data[dst + 2] = 255;
                self.atlas_data[dst + 3] = sdf[src];
            }
        }

        let aw = self.atlas_w as f32;
        let ah = self.atlas_h as f32;

        self.glyph_cache.insert(key, SdfGlyphInfo {
            uv: Rect {
                pos: Vec2::new(ax as f32 / aw, ay as f32 / ah),
                size: Vec2::new(gw as f32 / aw, gh as f32 / ah),
            },
            offset_norm: Vec2::new(
                metrics.xmin as f32 / BASE_RENDER_SIZE,
                metrics.ymin as f32 / BASE_RENDER_SIZE,
            ),
            advance_norm: metrics.advance_width / BASE_RENDER_SIZE,
            em_w: metrics.width as f32 / BASE_RENDER_SIZE,
            em_h: metrics.height as f32 / BASE_RENDER_SIZE,
        });

        self.shelf_x = ax + gw + pad;
        self.shelf_row_h = self.shelf_row_h.max(gh);
        // expand dirty rect to include this glyph
        self.dirty_rect = Some(match self.dirty_rect {
            Some((rx, ry, rw, rh)) => {
                let min_x = rx.min(ax);
                let min_y = ry.min(ay);
                let max_x = (rx + rw).max(ax + gw);
                let max_y = (ry + rh).max(ay + gh);
                (min_x, min_y, max_x - min_x, max_y - min_y)
            }
            None => (ax, ay, gw, gh),
        });
    }

    pub fn layout_text(&mut self, text: &str, params: &TextParams) -> Vec<GlyphQuad> {
        let font_id = params.font.id();
        if !self.fonts.contains_key(&font_id) {
            return Vec::new();
        }

        for ch in text.chars() {
            self.ensure_glyph(font_id, ch);
        }

        let scale = params.size;
        let line_h = params.line_height.unwrap_or(scale);

        let lines: Vec<&str> = text.split('\n').collect();
        let mut quads = Vec::with_capacity(text.len());

        for (li, line) in lines.iter().enumerate() {
            let line_w: f32 = line.chars().map(|ch| {
                let key = SdfGlyphKey { font_id, ch };
                self.glyph_cache.get(&key).map(|g| g.advance_norm).unwrap_or(0.0) * scale
            }).sum();

            let max_w = params.max_width.unwrap_or(0.0);
            let x_off = match params.align {
                TextAlign::Left => 0.0,
                TextAlign::Center => if max_w > 0.0 { (max_w - line_w) * 0.5 } else { -line_w * 0.5 },
                TextAlign::Right => if max_w > 0.0 { max_w - line_w } else { -line_w },
            };

            let mut cx = params.position.x + x_off;
            // position.y is the top of the text line, shift baseline down by font size
            let by = params.position.y + scale + li as f32 * line_h;

            for ch in line.chars() {
                let key = SdfGlyphKey { font_id, ch };
                if let Some(info) = self.glyph_cache.get(&key) {
                    if info.em_w > 0.0 && info.em_h > 0.0 {
                        let gx = cx + info.offset_norm.x * scale;
                        let gy = by - info.offset_norm.y * scale - info.em_h * scale;
                        quads.push(GlyphQuad {
                            pos: Vec2::new(gx, gy),
                            size: Vec2::new(info.em_w * scale, info.em_h * scale),
                            uv: info.uv,
                            color: params.color,
                        });
                    }
                    cx += info.advance_norm * scale;
                }
            }
        }

        quads
    }

    pub fn measure_text(&mut self, text: &str, params: &TextParams) -> Vec2 {
        let font_id = params.font.id();
        if !self.fonts.contains_key(&font_id) {
            return Vec2::ZERO;
        }

        for ch in text.chars() {
            self.ensure_glyph(font_id, ch);
        }

        let scale = params.size;
        let line_h = params.line_height.unwrap_or(scale);
        let lines: Vec<&str> = text.split('\n').collect();

        let mut max_w = 0.0f32;
        for line in &lines {
            let w: f32 = line.chars().map(|ch| {
                let key = SdfGlyphKey { font_id, ch };
                self.glyph_cache.get(&key).map(|g| g.advance_norm).unwrap_or(0.0) * scale
            }).sum();
            if w > max_w { max_w = w; }
        }

        Vec2::new(max_w, lines.len() as f32 * line_h)
    }

    pub fn atlas_texture_data(&self) -> (&[u8], u32, u32) {
        (&self.atlas_data, self.atlas_w, self.atlas_h)
    }

    pub fn is_atlas_dirty(&self) -> bool {
        self.dirty_rect.is_some()
    }

    pub fn dirty_region(&self) -> Option<(u32, u32, u32, u32)> {
        self.dirty_rect
    }

    // extract tightly-packed sub-rect from atlas (for glTexSubImage2D)
    pub fn atlas_sub_data(&self, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for row in 0..h {
            let src = (((y + row) * self.atlas_w + x) * 4) as usize;
            out.extend_from_slice(&self.atlas_data[src..src + (w * 4) as usize]);
        }
        out
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_rect = None;
    }
}

// compute signed distance field from a glyph bitmap
// uses brute-force approach: for each pixel find nearest boundary
pub fn compute_sdf(bitmap: &[u8], w: usize, h: usize, spread: f32) -> Vec<u8> {
    let len = w * h;
    let mut sdf = vec![128u8; len];

    let spread_i = spread.ceil() as i32;

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let inside = bitmap[idx] > 127;
            let mut min_dist_sq = (spread * spread) + 1.0;

            // search in a window around this pixel for nearest boundary
            let y0 = (y as i32 - spread_i).max(0) as usize;
            let y1 = (y as i32 + spread_i).min(h as i32 - 1) as usize;
            let x0 = (x as i32 - spread_i).max(0) as usize;
            let x1 = (x as i32 + spread_i).min(w as i32 - 1) as usize;

            for sy in y0..=y1 {
                for sx in x0..=x1 {
                    let other = bitmap[sy * w + sx] > 127;
                    if other != inside {
                        let dx = x as f32 - sx as f32;
                        let dy = y as f32 - sy as f32;
                        let d = dx * dx + dy * dy;
                        if d < min_dist_sq {
                            min_dist_sq = d;
                        }
                    }
                }
            }

            let dist = min_dist_sq.sqrt();
            // normalize: 0 = far outside, 128 = boundary, 255 = far inside
            let norm = if inside {
                128.0 + (dist / spread * 127.0).min(127.0)
            } else {
                128.0 - (dist / spread * 128.0).min(128.0)
            };

            sdf[idx] = norm.clamp(0.0, 255.0) as u8;
        }
    }

    sdf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdf_boundary_at_midpoint() {
        // 8x8 bitmap with a filled 4x4 square in center
        let mut bitmap = vec![0u8; 64];
        for y in 2..6 {
            for x in 2..6 {
                bitmap[y * 8 + x] = 255;
            }
        }
        let sdf = compute_sdf(&bitmap, 8, 8, 4.0);
        // center pixel should be inside (> 128)
        assert!(sdf[4 * 8 + 4] > 128, "center should be inside, got {}", sdf[4 * 8 + 4]);
        // corner pixel (0,0) should be outside (< 128)
        assert!(sdf[0] < 128, "corner should be outside, got {}", sdf[0]);
    }

    #[test]
    fn test_sdf_empty_bitmap() {
        let bitmap = vec![0u8; 16];
        let sdf = compute_sdf(&bitmap, 4, 4, 4.0);
        // all pixels outside, all should be <= 128
        for v in &sdf {
            assert!(*v <= 128, "empty bitmap pixel should be <= 128, got {}", v);
        }
    }

    #[test]
    fn test_sdf_full_bitmap() {
        let bitmap = vec![255u8; 16];
        let sdf = compute_sdf(&bitmap, 4, 4, 4.0);
        // all inside, no boundary -> all should be >= 128
        for v in &sdf {
            assert!(*v >= 128, "full bitmap pixel should be >= 128, got {}", v);
        }
    }

    #[test]
    fn test_sdf_symmetry() {
        // symmetric input: centered 2x2 block in 6x6
        let mut bitmap = vec![0u8; 36];
        for y in 2..4 {
            for x in 2..4 {
                bitmap[y * 6 + x] = 255;
            }
        }
        let sdf = compute_sdf(&bitmap, 6, 6, 4.0);
        // top-left and top-right mirror should have same value
        assert_eq!(sdf[0 * 6 + 1], sdf[0 * 6 + 4], "horizontal symmetry broken");
    }
}
