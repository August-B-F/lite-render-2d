use crate::text::{FontHandle, GlyphPosition, TextAlign, TextLayout, TextParams};
use crate::types::{Color, Rect, Vec2};

use std::collections::HashMap;

const ATLAS_SIZE: u32 = 512;
const MAX_ATLAS_SIZE: u32 = 4096;

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
    dirty_rect: Option<(u32, u32, u32, u32)>, // (x, y, w, h) bounding box of changed pixels
    glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    // shelf packer state
    shelf_x: u32,
    shelf_y: u32,
    shelf_row_height: u32,
    next_font_id: u64,
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FontSystem {
    pub fn new() -> Self {
        let size = (ATLAS_SIZE * ATLAS_SIZE * 4) as usize;
        Self {
            fonts: HashMap::new(),
            atlas_data: vec![0u8; size],
            atlas_width: ATLAS_SIZE,
            atlas_height: ATLAS_SIZE,
            dirty_rect: None,
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
            if !self.grow_atlas() {
                return;
            }
            // after growing, recalculate shelf position
            if self.shelf_x + gw + pad > self.atlas_width {
                self.shelf_y += self.shelf_row_height + pad;
                self.shelf_x = 0;
                self.shelf_row_height = 0;
            }
            if self.shelf_y + gh + pad > self.atlas_height {
                return;
            }
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

    fn grow_atlas(&mut self) -> bool {
        let new_w = (self.atlas_width * 2).min(MAX_ATLAS_SIZE);
        let new_h = (self.atlas_height * 2).min(MAX_ATLAS_SIZE);
        if new_w == self.atlas_width && new_h == self.atlas_height {
            return false;
        }

        let old_w = self.atlas_width;
        let old_h = self.atlas_height;
        let mut new_data = vec![0u8; (new_w * new_h * 4) as usize];

        for row in 0..old_h {
            let src_start = (row * old_w * 4) as usize;
            let src_end = src_start + (old_w * 4) as usize;
            let dst_start = (row * new_w * 4) as usize;
            new_data[dst_start..dst_start + (old_w * 4) as usize]
                .copy_from_slice(&self.atlas_data[src_start..src_end]);
        }

        let nw = new_w as f32;
        let nh = new_h as f32;
        let ow = old_w as f32;
        let oh = old_h as f32;
        for info in self.glyph_cache.values_mut() {
            info.uv.pos.x = info.uv.pos.x * ow / nw;
            info.uv.pos.y = info.uv.pos.y * oh / nh;
            info.uv.size.x = info.uv.size.x * ow / nw;
            info.uv.size.y = info.uv.size.y * oh / nh;
        }

        self.atlas_data = new_data;
        self.atlas_width = new_w;
        self.atlas_height = new_h;
        self.dirty_rect = Some((0, 0, new_w, new_h));
        true
    }

    fn split_lines(&self, text: &str, params: &TextParams) -> Vec<String> {
        let font_id = params.font.id();
        let size_key = (params.size * 10.0) as u32;
        let ls = params.letter_spacing.unwrap_or(0.0);

        let mut lines = Vec::new();
        for raw_line in text.split('\n') {
            let max_w = match params.max_width {
                Some(w) if w > 0.0 => w,
                _ => {
                    lines.push(raw_line.to_string());
                    continue;
                }
            };

            let words: Vec<&str> = raw_line.split(' ').collect();
            let mut current = String::new();
            let mut cur_w = 0.0f32;
            let space_advance = {
                let key = GlyphKey { font_id, ch: ' ', size_key };
                self.glyph_cache.get(&key).map(|i| i.advance).unwrap_or(params.size * 0.25)
            } + ls;

            for word in words.iter() {
                let word_w = self.measure_str_width(word, font_id, size_key, ls);

                if word_w > max_w && current.is_empty() {
                    let mut char_line = String::new();
                    let mut cw = 0.0f32;
                    for ch in word.chars() {
                        let key = GlyphKey { font_id, ch, size_key };
                        let adv = self.glyph_cache.get(&key).map(|i| i.advance).unwrap_or(0.0) + ls;
                        if cw + adv > max_w && !char_line.is_empty() {
                            lines.push(char_line);
                            char_line = String::new();
                            cw = 0.0;
                        }
                        char_line.push(ch);
                        cw += adv;
                    }
                    current = char_line;
                    cur_w = cw;
                    continue;
                }

                let needed = if current.is_empty() { word_w } else { space_advance + word_w };
                if cur_w + needed > max_w && !current.is_empty() {
                    lines.push(current);
                    current = String::new();
                    cur_w = 0.0;
                }

                if !current.is_empty() {
                    current.push(' ');
                    cur_w += space_advance;
                }
                current.push_str(word);
                cur_w += word_w;
            }
            lines.push(current);
        }
        lines
    }

    fn measure_str_width(&self, s: &str, font_id: u64, size_key: u32, letter_spacing: f32) -> f32 {
        let mut w = 0.0f32;
        for ch in s.chars() {
            let key = GlyphKey { font_id, ch, size_key };
            if let Some(info) = self.glyph_cache.get(&key) {
                w += info.advance + letter_spacing;
            }
        }
        w
    }

    pub fn layout_text(&mut self, text: &str, params: &TextParams) -> Vec<GlyphQuad> {
        let font_id = params.font.id();
        if !self.fonts.contains_key(&font_id) {
            return Vec::new();
        }

        for ch in text.chars() {
            self.ensure_glyph(font_id, ch, params.size);
        }

        let line_h = params.line_height.unwrap_or(params.size);
        let ls = params.letter_spacing.unwrap_or(0.0);
        let lines = self.split_lines(text, params);
        let size_key = (params.size * 10.0) as u32;

        let mut quads = Vec::with_capacity(text.len());
        let align_width = params.max_width.unwrap_or(0.0);

        for (li, line) in lines.iter().enumerate() {
            let line_w = self.measure_str_width(line, font_id, size_key, ls);

            let x_offset = match params.align {
                TextAlign::Left => 0.0,
                TextAlign::Center => {
                    if align_width > 0.0 { (align_width - line_w) * 0.5 }
                    else { -line_w * 0.5 }
                }
                TextAlign::Right => {
                    if align_width > 0.0 { align_width - line_w }
                    else { -line_w }
                }
            };

            let mut cursor_x = params.position.x + x_offset;
            let baseline_y = params.position.y + params.size + li as f32 * line_h;

            for ch in line.chars() {
                let key = GlyphKey { font_id, ch, size_key };
                if let Some(info) = self.glyph_cache.get(&key) {
                    if info.width > 0 && info.height > 0 {
                        let gx = cursor_x + info.offset.x;
                        let gy = baseline_y - info.offset.y - info.height as f32;
                        quads.push(GlyphQuad {
                            pos: Vec2::new(gx, gy),
                            size: Vec2::new(info.width as f32, info.height as f32),
                            uv: info.uv,
                            color: params.color,
                        });
                    }
                    cursor_x += info.advance + ls;
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
            self.ensure_glyph(font_id, ch, params.size);
        }

        let line_h = params.line_height.unwrap_or(params.size);
        let ls = params.letter_spacing.unwrap_or(0.0);
        let lines = self.split_lines(text, params);
        let size_key = (params.size * 10.0) as u32;

        let mut max_w = 0.0f32;
        for line in &lines {
            let w = self.measure_str_width(line, font_id, size_key, ls);
            if w > max_w { max_w = w; }
        }

        Vec2::new(max_w, lines.len() as f32 * line_h)
    }

    pub fn compute_text_layout(&mut self, text: &str, params: &TextParams) -> TextLayout {
        let font_id = params.font.id();
        if !self.fonts.contains_key(&font_id) {
            return TextLayout { glyphs: vec![], size: Vec2::ZERO, line_count: 0, line_offsets: vec![] };
        }

        for ch in text.chars() {
            self.ensure_glyph(font_id, ch, params.size);
        }

        let line_h = params.line_height.unwrap_or(params.size);
        let ls = params.letter_spacing.unwrap_or(0.0);
        let lines = self.split_lines(text, params);
        let size_key = (params.size * 10.0) as u32;
        let align_width = params.max_width.unwrap_or(0.0);

        let mut glyphs = Vec::with_capacity(text.len());
        let mut line_offsets = Vec::with_capacity(lines.len());
        let mut max_w = 0.0f32;

        // map wrapped lines back to byte offsets in the original string
        // split_lines produces wrapped lines from the original text split by \n then by words
        // we walk the original text in parallel
        let mut byte_cursor = 0usize;
        let mut char_cursor = 0usize;

        for (li, line) in lines.iter().enumerate() {
            let line_y = params.position.y + li as f32 * line_h;
            line_offsets.push(line_y);

            let line_w = self.measure_str_width(line, font_id, size_key, ls);
            if line_w > max_w { max_w = line_w; }

            let x_offset = match params.align {
                TextAlign::Left => 0.0,
                TextAlign::Center => {
                    if align_width > 0.0 { (align_width - line_w) * 0.5 }
                    else { -line_w * 0.5 }
                }
                TextAlign::Right => {
                    if align_width > 0.0 { align_width - line_w }
                    else { -line_w }
                }
            };

            let mut cursor_x = params.position.x + x_offset;

            for ch in line.chars() {
                let key = GlyphKey { font_id, ch, size_key };
                let adv = self.glyph_cache.get(&key).map(|i| i.advance).unwrap_or(0.0) + ls;

                // find byte_offset in original string: skip whitespace between wrapped lines
                while byte_cursor < text.len() {
                    let orig_ch = text[byte_cursor..].chars().next().unwrap();
                    if orig_ch == ch {
                        break;
                    }
                    // skip \n that was consumed by line splitting or spaces that got dropped
                    if orig_ch == '\n' || orig_ch == ' ' {
                        byte_cursor += orig_ch.len_utf8();
                        char_cursor += 1;
                    } else {
                        break;
                    }
                }

                glyphs.push(GlyphPosition {
                    byte_offset: byte_cursor,
                    char_index: char_cursor,
                    line: li,
                    x: cursor_x,
                    y: line_y,
                    advance: adv,
                    line_height: line_h,
                });

                cursor_x += adv;
                byte_cursor += ch.len_utf8();
                char_cursor += 1;
            }

            // after each wrapped line, skip the \n or space delimiter in the original text
            if li < lines.len() - 1 && byte_cursor < text.len() {
                let next_ch = text[byte_cursor..].chars().next().unwrap();
                if next_ch == '\n' || next_ch == ' ' {
                    byte_cursor += next_ch.len_utf8();
                    char_cursor += 1;
                }
            }
        }

        TextLayout {
            glyphs,
            size: Vec2::new(max_w, lines.len() as f32 * line_h),
            line_count: lines.len(),
            line_offsets,
        }
    }

    pub fn ensure_glyph_pub(&mut self, font_id: u64, ch: char, size: f32) {
        self.ensure_glyph(font_id, ch, size);
    }

    // get horizontal advance for a glyph
    pub fn glyph_advance(&self, font_id: u64, ch: char, size: f32) -> f32 {
        let size_key = (size * 10.0) as u32;
        let key = GlyphKey { font_id, ch, size_key };
        self.glyph_cache.get(&key).map(|i| i.advance).unwrap_or(0.0)
    }

    // get a positioned glyph quad (used by rich text layout)
    pub fn glyph_quad(&self, font_id: u64, ch: char, size: f32, cx: f32, by: f32, color: Color) -> Option<GlyphQuad> {
        let size_key = (size * 10.0) as u32;
        let key = GlyphKey { font_id, ch, size_key };
        let info = self.glyph_cache.get(&key)?;
        if info.width == 0 || info.height == 0 {
            return None;
        }
        let gx = cx + info.offset.x;
        let gy = by - info.offset.y - info.height as f32;
        Some(GlyphQuad {
            pos: Vec2::new(gx, gy),
            size: Vec2::new(info.width as f32, info.height as f32),
            uv: info.uv,
            color,
        })
    }

    pub fn font_ascent(&self, font_id: u64, size: f32) -> f32 {
        self.fonts
            .get(&font_id)
            .and_then(|f| f.horizontal_line_metrics(size))
            .map(|m| m.ascent)
            .unwrap_or(size * 0.8)
    }

    pub fn atlas_texture_data(&self) -> (&[u8], u32, u32) {
        (&self.atlas_data, self.atlas_width, self.atlas_height)
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
            let src = (((y + row) * self.atlas_width + x) * 4) as usize;
            out.extend_from_slice(&self.atlas_data[src..src + (w * 4) as usize]);
        }
        out
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_rect = None;
    }
}
