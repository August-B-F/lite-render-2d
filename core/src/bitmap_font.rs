use std::collections::HashMap;
use crate::types::{Color, Rect, Vec2};
use crate::texture::TextureHandle;

// glyph metrics for a single character in a bitmap font
#[derive(Clone, Copy, Debug)]
pub struct BitmapGlyph {
    pub src_rect: Rect,
    pub offset: Vec2,
    pub advance: f32,
}

// bitmap font loaded from a grid-based sprite sheet or .fnt metrics
pub struct BitmapFont {
    pub texture: TextureHandle,
    pub line_height: f32,
    glyphs: HashMap<char, BitmapGlyph>,
}

impl BitmapFont {
    // create a grid-based bitmap font where chars are layed out in a grid
    // first_char is the ascii code of the top-left character (usually ' ' = 32)
    pub fn from_grid(
        texture: TextureHandle,
        cell_w: f32,
        cell_h: f32,
        columns: u32,
        first_char: u32,
        char_count: u32,
    ) -> Self {
        let mut glyphs = HashMap::new();
        for i in 0..char_count {
            let ch = char::from_u32(first_char + i);
            if let Some(ch) = ch {
                let col = i % columns;
                let row = i / columns;
                glyphs.insert(ch, BitmapGlyph {
                    src_rect: Rect {
                        pos: Vec2::new(col as f32 * cell_w, row as f32 * cell_h),
                        size: Vec2::new(cell_w, cell_h),
                    },
                    offset: Vec2::ZERO,
                    advance: cell_w,
                });
            }
        }
        Self {
            texture,
            line_height: cell_h,
            glyphs,
        }
    }

    // set custom glyph metrics (for variable-width bitmap fonts)
    pub fn set_glyph(&mut self, ch: char, glyph: BitmapGlyph) {
        self.glyphs.insert(ch, glyph);
    }

    pub fn get_glyph(&self, ch: char) -> Option<&BitmapGlyph> {
        self.glyphs.get(&ch)
    }

    // measure text width
    pub fn measure(&self, text: &str) -> Vec2 {
        let mut w = 0.0f32;
        let mut max_w = 0.0f32;
        let mut lines = 1;
        for ch in text.chars() {
            if ch == '\n' {
                max_w = max_w.max(w);
                w = 0.0;
                lines += 1;
                continue;
            }
            if let Some(g) = self.glyphs.get(&ch) {
                w += g.advance;
            }
        }
        max_w = max_w.max(w);
        Vec2::new(max_w, lines as f32 * self.line_height)
    }

    // layout text and return quads for rendering via sprite pipeline
    pub fn layout(&self, text: &str, pos: Vec2, _color: Color) -> Vec<BitmapGlyphQuad> {
        let mut quads = Vec::new();
        let mut cx = pos.x;
        let mut cy = pos.y;

        for ch in text.chars() {
            if ch == '\n' {
                cx = pos.x;
                cy += self.line_height;
                continue;
            }
            if let Some(g) = self.glyphs.get(&ch) {
                quads.push(BitmapGlyphQuad {
                    src_rect: g.src_rect,
                    pos: Vec2::new(cx + g.offset.x, cy + g.offset.y),
                    size: g.src_rect.size,
                });
                cx += g.advance;
            }
        }
        quads
    }
}

// output quad for bitmap font rendering
pub struct BitmapGlyphQuad {
    pub src_rect: Rect,
    pub pos: Vec2,
    pub size: Vec2,
}
