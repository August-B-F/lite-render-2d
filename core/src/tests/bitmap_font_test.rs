use crate::bitmap_font::BitmapFont;
use crate::texture::TextureHandle;
use crate::types::{Color, Rect, Vec2};

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn make_font() -> BitmapFont {
    BitmapFont::from_grid(
        TextureHandle::new(1),
        8.0,  // cell_w
        16.0, // cell_h
        16,   // columns
        32,   // first_char = ' '
        95,   // printable ASCII
    )
}

// -- from_grid --

#[test]
fn test_from_grid_creates_glyphs() {
    let font = make_font();
    assert!(font.get_glyph(' ').is_some());
    assert!(font.get_glyph('A').is_some());
    assert!(font.get_glyph('~').is_some());
}

#[test]
fn test_from_grid_glyph_positions() {
    let font = make_font();
    // ' ' is char 32, first_char is 32, so index 0 => col=0, row=0
    let space = font.get_glyph(' ').unwrap();
    assert!(approx(space.src_rect.pos.x, 0.0));
    assert!(approx(space.src_rect.pos.y, 0.0));

    // 'A' is char 65, index 33 => col=1, row=2 (33 / 16 = 2, 33 % 16 = 1)
    let a = font.get_glyph('A').unwrap();
    assert!(approx(a.src_rect.pos.x, 1.0 * 8.0));
    assert!(approx(a.src_rect.pos.y, 2.0 * 16.0));
}

#[test]
fn test_from_grid_advance() {
    let font = make_font();
    let g = font.get_glyph('X').unwrap();
    assert!(approx(g.advance, 8.0)); // cell_w
}

#[test]
fn test_line_height() {
    let font = make_font();
    assert!(approx(font.line_height, 16.0));
}

// -- measure --

#[test]
fn test_measure_single_line() {
    let font = make_font();
    let size = font.measure("Hello");
    assert!(approx(size.x, 5.0 * 8.0)); // 5 chars * 8.0 advance
    assert!(approx(size.y, 16.0));       // 1 line
}

#[test]
fn test_measure_multiline() {
    let font = make_font();
    let size = font.measure("AB\nCDE");
    assert!(approx(size.x, 3.0 * 8.0)); // max line is "CDE" = 3 chars
    assert!(approx(size.y, 2.0 * 16.0)); // 2 lines
}

#[test]
fn test_measure_empty() {
    let font = make_font();
    let size = font.measure("");
    assert!(approx(size.x, 0.0));
    assert!(approx(size.y, 16.0)); // 1 line even if empty
}

#[test]
fn test_measure_unknown_chars_skipped() {
    let font = make_font();
    // chars outside the 32..126 range won't have glyphs
    let size = font.measure("\x01\x02\x03");
    assert!(approx(size.x, 0.0)); // no recognized glyphs
}

// -- layout --

#[test]
fn test_layout_positions() {
    let font = make_font();
    let quads = font.layout("AB", Vec2::new(10.0, 20.0), Color::WHITE);
    assert_eq!(quads.len(), 2);
    assert!(approx(quads[0].pos.x, 10.0));
    assert!(approx(quads[0].pos.y, 20.0));
    assert!(approx(quads[1].pos.x, 18.0)); // 10 + 8 advance
    assert!(approx(quads[1].pos.y, 20.0));
}

#[test]
fn test_layout_newline() {
    let font = make_font();
    let quads = font.layout("A\nB", Vec2::new(0.0, 0.0), Color::WHITE);
    assert_eq!(quads.len(), 2);
    assert!(approx(quads[0].pos.y, 0.0));
    assert!(approx(quads[1].pos.x, 0.0));  // reset to start x
    assert!(approx(quads[1].pos.y, 16.0)); // next line
}

#[test]
fn test_layout_empty() {
    let font = make_font();
    let quads = font.layout("", Vec2::ZERO, Color::WHITE);
    assert!(quads.is_empty());
}

// -- set_glyph --

#[test]
fn test_set_glyph_overrides() {
    let mut font = make_font();
    let custom = crate::bitmap_font::BitmapGlyph {
        src_rect: Rect { pos: Vec2::new(100.0, 200.0), size: Vec2::new(12.0, 16.0) },
        offset: Vec2::new(1.0, 2.0),
        advance: 12.0,
    };
    font.set_glyph('A', custom);
    let g = font.get_glyph('A').unwrap();
    assert!(approx(g.advance, 12.0));
    assert!(approx(g.src_rect.pos.x, 100.0));
}
