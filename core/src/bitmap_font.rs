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

// Built-in 4x6 pixel font (Tom Thumb, public domain).
// 95 glyphs for ASCII 32-126, each 4 pixels wide x 6 pixels tall.
// Packed 1 bit per pixel, 4 bits per row, 3 bytes per glyph (6 rows x 4 bits = 24 bits).
// Row bits are stored MSB-first within the upper nibble then lower nibble of each byte.
// Byte layout per glyph: [row0_row1, row2_row3, row4_row5] where each row is 4 bits (MSB = leftmost).
const BUILTIN_FONT_4X6: [u8; 285] = [
    // ' ' (32)
    0x00, 0x00, 0x00,
    // '!' (33)
    0x44, 0x40, 0x40,
    // '"' (34)
    0xAA, 0x00, 0x00,
    // '#' (35)
    0xAF, 0xAF, 0xA0,
    // '$' (36)
    0x6E, 0xC7, 0xC0,
    // '%' (37)
    0x92, 0x44, 0x90,
    // '&' (38)
    0x4A, 0x4A, 0x50,
    // '\'' (39)
    0x44, 0x00, 0x00,
    // '(' (40)
    0x24, 0x44, 0x20,
    // ')' (41)
    0x48, 0x88, 0x40,
    // '*' (42)
    0x0A, 0x4A, 0x00,
    // '+' (43)
    0x04, 0xE4, 0x00,
    // ',' (44)
    0x00, 0x04, 0x48,
    // '-' (45)
    0x00, 0xE0, 0x00,
    // '.' (46)
    0x00, 0x00, 0x40,
    // '/' (47)
    0x22, 0x44, 0x80,
    // '0' (48)
    0x4A, 0xAA, 0x40,
    // '1' (49)
    0x4C, 0x44, 0xE0,
    // '2' (50)
    0xC2, 0x48, 0xE0,
    // '3' (51)
    0xC2, 0x42, 0xC0,
    // '4' (52)
    0xAA, 0xE2, 0x20,
    // '5' (53)
    0xE8, 0xC2, 0xC0,
    // '6' (54)
    0x68, 0xEA, 0xE0,
    // '7' (55)
    0xE2, 0x44, 0x40,
    // '8' (56)
    0xEA, 0xEA, 0xE0,
    // '9' (57)
    0xEA, 0xE2, 0xC0,
    // ':' (58)
    0x04, 0x04, 0x00,
    // ';' (59)
    0x04, 0x04, 0x48,
    // '<' (60)
    0x24, 0x84, 0x20,
    // '=' (61)
    0x0E, 0x0E, 0x00,
    // '>' (62)
    0x84, 0x24, 0x80,
    // '?' (63)
    0xE2, 0x40, 0x40,
    // '@' (64)
    0x4A, 0xE8, 0x60,
    // 'A' (65)
    0x4A, 0xEA, 0xA0,
    // 'B' (66)
    0xCA, 0xCA, 0xC0,
    // 'C' (67)
    0x68, 0x88, 0x60,
    // 'D' (68)
    0xCA, 0xAA, 0xC0,
    // 'E' (69)
    0xE8, 0xC8, 0xE0,
    // 'F' (70)
    0xE8, 0xC8, 0x80,
    // 'G' (71)
    0x68, 0xAA, 0x60,
    // 'H' (72)
    0xAA, 0xEA, 0xA0,
    // 'I' (73)
    0xE4, 0x44, 0xE0,
    // 'J' (74)
    0x22, 0x2A, 0x40,
    // 'K' (75)
    0xAA, 0xCA, 0xA0,
    // 'L' (76)
    0x88, 0x88, 0xE0,
    // 'M' (77)
    0xAE, 0xEA, 0xA0,
    // 'N' (78)
    0xAE, 0xEE, 0xA0,
    // 'O' (79)
    0x4A, 0xAA, 0x40,
    // 'P' (80)
    0xCA, 0xC8, 0x80,
    // 'Q' (81)
    0x4A, 0xAE, 0x60,
    // 'R' (82)
    0xCA, 0xCA, 0xA0,
    // 'S' (83)
    0x68, 0x42, 0xC0,
    // 'T' (84)
    0xE4, 0x44, 0x40,
    // 'U' (85)
    0xAA, 0xAA, 0x60,
    // 'V' (86)
    0xAA, 0xAA, 0x40,
    // 'W' (87)
    0xAA, 0xEE, 0xA0,
    // 'X' (88)
    0xAA, 0x4A, 0xA0,
    // 'Y' (89)
    0xAA, 0x44, 0x40,
    // 'Z' (90)
    0xE2, 0x48, 0xE0,
    // '[' (91)
    0x64, 0x44, 0x60,
    // '\\' (92)
    0x88, 0x44, 0x20,
    // ']' (93)
    0x62, 0x22, 0x60,
    // '^' (94)
    0x4A, 0x00, 0x00,
    // '_' (95)
    0x00, 0x00, 0xE0,
    // '`' (96)
    0x84, 0x00, 0x00,
    // 'a' (97)
    0x06, 0xAA, 0x60,
    // 'b' (98)
    0x8C, 0xAA, 0xC0,
    // 'c' (99)
    0x06, 0x88, 0x60,
    // 'd' (100)
    0x26, 0xAA, 0x60,
    // 'e' (101)
    0x06, 0xAC, 0x60,
    // 'f' (102)
    0x24, 0xE4, 0x40,
    // 'g' (103)
    0x06, 0xA6, 0x2C,
    // 'h' (104)
    0x8C, 0xAA, 0xA0,
    // 'i' (105)
    0x40, 0x44, 0x40,
    // 'j' (106)
    0x20, 0x22, 0xA4,
    // 'k' (107)
    0x8A, 0xCA, 0xA0,
    // 'l' (108)
    0xC4, 0x44, 0xE0,
    // 'm' (109)
    0x0E, 0xEA, 0xA0,
    // 'n' (110)
    0x0C, 0xAA, 0xA0,
    // 'o' (111)
    0x04, 0xAA, 0x40,
    // 'p' (112)
    0x0C, 0xAC, 0x80,
    // 'q' (113)
    0x06, 0xA6, 0x20,
    // 'r' (114)
    0x06, 0x88, 0x80,
    // 's' (115)
    0x06, 0xC2, 0xC0,
    // 't' (116)
    0x4E, 0x44, 0x20,
    // 'u' (117)
    0x0A, 0xAA, 0x60,
    // 'v' (118)
    0x0A, 0xAA, 0x40,
    // 'w' (119)
    0x0A, 0xAE, 0xE0,
    // 'x' (120)
    0x0A, 0x44, 0xA0,
    // 'y' (121)
    0x0A, 0xA6, 0x2C,
    // 'z' (122)
    0x0E, 0x24, 0xE0,
    // '{' (123)
    0x64, 0xC4, 0x60,
    // '|' (124)
    0x44, 0x44, 0x40,
    // '}' (125)
    0xC4, 0x64, 0xC0,
    // '~' (126)
    0x05, 0xA0, 0x00,
];

impl BitmapFont {
    /// create the built-in 4x6 debug font. requires a renderer to upload the texture.
    pub fn builtin_debug<R: crate::renderer::Renderer>(renderer: &mut R) -> Result<Self, crate::renderer::RendererError> {
        use crate::texture::TextureParams;

        let columns = 16u32;
        let glyph_count = 95u32;
        let rows = (glyph_count + columns - 1) / columns; // 6
        let cell_w = 4u32;
        let cell_h = 6u32;
        let atlas_w = columns * cell_w; // 64
        let atlas_h = rows * cell_h;    // 36
        let mut rgba = vec![0u8; (atlas_w * atlas_h * 4) as usize];

        for glyph_idx in 0..glyph_count {
            let byte_offset = (glyph_idx as usize) * 3;
            let grid_col = glyph_idx % columns;
            let grid_row = glyph_idx / columns;

            for row in 0..6u32 {
                // each row is 4 bits (a nibble)
                let byte = BUILTIN_FONT_4X6[byte_offset + (row as usize / 2)];
                let nibble = if row % 2 == 0 { byte >> 4 } else { byte & 0x0F };

                for col in 0..4u32 {
                    let bit = (nibble >> (3 - col)) & 1;
                    if bit == 1 {
                        let px = grid_col * cell_w + col;
                        let py = grid_row * cell_h + row;
                        let idx = ((py * atlas_w + px) * 4) as usize;
                        rgba[idx]     = 255; // r
                        rgba[idx + 1] = 255; // g
                        rgba[idx + 2] = 255; // b
                        rgba[idx + 3] = 255; // a
                    }
                }
            }
        }

        let handle = renderer.load_texture_raw(&rgba, atlas_w, atlas_h, TextureParams::nearest())?;
        Ok(BitmapFont::from_grid(handle, cell_w as f32, cell_h as f32, columns, 32, glyph_count))
    }
}
