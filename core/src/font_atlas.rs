use crate::text::{FontHandle, TextAlign, TextParams};
use crate::types::{Color, Rect, Vec2};

use std::collections::HashMap;

const ATLAS_SIZE: u32 = 1024;

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct GlyphKey {
    font_id: u64,
    ch: char,
    size_key: u32, // font size * 10 to allow one decimal
}

struct GlyphInfo {
    uv: Rect,     // normalized UV rect in atlas
    offset: Vec2,  // bearing offset from cursor
    advance: f32,  // horizontal advance
    width: u32,
    height: u32,
}

pub struct GlyphQuad {
    pub pos: Vec2,
    pub size: Vec2,
    pub uv: Rect,
    pub color: Color,
}

pub struct FontSystem {
    fonts: HashMap<u64, fontdue::Font>,
    atlas_data: Vec<u8>,
    atlas_width: u32,
    atlas_height: u32,
    atlas_dirty: bool,
    glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    // shelf packer state
    shelf_x: u32,
    shelf_y: u32,
    shelf_row_height: u32,
    next_font_id: u64,
}

impl FontSystem {
    pub fn new() -> Self {
        let size = (ATLAS_SIZE * ATLAS_SIZE * 4) as usize;
        Self {
            fonts: HashMap::new(),
            atlas_data: vec![0u8; size],
            atlas_width: ATLAS_SIZE,
            atlas_height: ATLAS_SIZE,
            atlas_dirty: false,
            glyph_cache: HashMap::new(),
            shelf_x: 0,
            shelf_y: 0,
            shelf_row_height: 0,
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

    fn ensure_glyph(&mut self, font_id: u64, ch: char, size: f32) {
        let size_key = (size * 10.0) as u32;
        let key = GlyphKey { font_id, ch, size_key };
        if self.glyph_cache.contains_key(&key) {
            return;
        }

        let font = match self.fonts.get(&font_id) {
            Some(f) => f,
            None => return,
        };

        let (metrics, bitmap) = font.rasterize(ch, size);
        if metrics.width == 0 || metrics.height == 0 {
            // whitespace — store metrics only
            self.glyph_cache.insert(key, GlyphInfo {
                uv: Rect { pos: Vec2::ZERO, size: Vec2::ZERO },
                offset: Vec2::new(metrics.xmin as f32, metrics.ymin as f32),
                advance: metrics.advance_width,
                width: 0,
                height: 0,
            });
            return;
        }

        let gw = metrics.width as u32;
        let gh = metrics.height as u32;
        let pad = 1u32; // 1px padding between glyphs

        // shelf pack
        if self.shelf_x + gw + pad > self.atlas_width {
            // next row
            self.shelf_y += self.shelf_row_height + pad;
            self.shelf_x = 0;
            self.shelf_row_height = 0;
        }

        if self.shelf_y + gh + pad > self.atlas_height {
            // atlas full — for now just skip
            return;
        }

        let ax = self.shelf_x;
        let ay = self.shelf_y;

        // copy glyph bitmap into atlas (white + alpha)
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let src = row * metrics.width + col;
                let dst_x = ax + col as u32;
                let dst_y = ay + row as u32;
                let dst = ((dst_y * self.atlas_width + dst_x) * 4) as usize;
                let alpha = bitmap[src];
                self.atlas_data[dst] = 255;     // R
                self.atlas_data[dst + 1] = 255; // G
                self.atlas_data[dst + 2] = 255; // B
                self.atlas_data[dst + 3] = alpha; // A
            }
        }

        let aw = self.atlas_width as f32;
        let ah = self.atlas_height as f32;

        self.glyph_cache.insert(key, GlyphInfo {
            uv: Rect {
                pos: Vec2::new(ax as f32 / aw, ay as f32 / ah),
                size: Vec2::new(gw as f32 / aw, gh as f32 / ah),
            },
            offset: Vec2::new(metrics.xmin as f32, metrics.ymin as f32),
            advance: metrics.advance_width,
            width: gw,
            height: gh,
        });

        self.shelf_x = ax + gw + pad;
        self.shelf_row_height = self.shelf_row_height.max(gh);
        self.atlas_dirty = true;
    }

    pub fn layout_text(&mut self, text: &str, params: &TextParams) -> Vec<GlyphQuad> {
        let font_id = params.font.id();
        if !self.fonts.contains_key(&font_id) {
            return Vec::new();
        }

        // ensure all glyphs are rasterized
        for ch in text.chars() {
            self.ensure_glyph(font_id, ch, params.size);
        }

        // measure total width for alignment
        let total_width = self.measure_width(text, params);

        let x_offset = match params.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => -total_width * 0.5,
            TextAlign::Right => -total_width,
        };

        let mut cursor_x = params.position.x + x_offset;
        let baseline_y = params.position.y;
        let mut quads = Vec::with_capacity(text.len());

        let size_key = (params.size * 10.0) as u32;

        for ch in text.chars() {
            let key = GlyphKey { font_id, ch, size_key };
            if let Some(info) = self.glyph_cache.get(&key) {
                if info.width > 0 && info.height > 0 {
                    // fontdue ymin is distance from baseline (positive = above)
                    let gx = cursor_x + info.offset.x;
                    let gy = baseline_y - info.offset.y - info.height as f32;

                    quads.push(GlyphQuad {
                        pos: Vec2::new(gx, gy),
                        size: Vec2::new(info.width as f32, info.height as f32),
                        uv: info.uv,
                        color: params.color,
                    });
                }
                cursor_x += info.advance;
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
            self.ensure_glyph(font_id, ch, params.size);
        }

        Vec2::new(self.measure_width(text, params), params.size)
    }

    fn measure_width(&self, text: &str, params: &TextParams) -> f32 {
        let font_id = params.font.id();
        let size_key = (params.size * 10.0) as u32;
        let mut width = 0.0f32;
        for ch in text.chars() {
            let key = GlyphKey { font_id, ch, size_key };
            if let Some(info) = self.glyph_cache.get(&key) {
                width += info.advance;
            }
        }
        width
    }

    pub fn atlas_texture_data(&self) -> (&[u8], u32, u32) {
        (&self.atlas_data, self.atlas_width, self.atlas_height)
    }

    pub fn is_atlas_dirty(&self) -> bool {
        self.atlas_dirty
    }

    pub fn clear_dirty(&mut self) {
        self.atlas_dirty = false;
    }
}
