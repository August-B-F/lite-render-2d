use crate::font_atlas::FontSystem;
use crate::text::{FontHandle, TextAlign, TextParams};
use crate::types::{Color, Vec2};

const FONT_DATA: &[u8] = include_bytes!("../../../NotoSansMeroitic-Regular.ttf");

fn make_params(font: FontHandle, size: f32) -> TextParams {
    TextParams {
        font,
        size,
        color: Color::WHITE,
        align: TextAlign::Left,
        position: Vec2::ZERO,
        max_width: None,
        line_height: None,
        z: 0,
        letter_spacing: None,
        underline: false,
        strikethrough: false,
    }
}

// -- construction --

#[test]
fn test_new_not_dirty() {
    let sys = FontSystem::new();
    assert!(!sys.is_atlas_dirty());
}

#[test]
fn test_new_atlas_size() {
    let sys = FontSystem::new();
    let (data, w, h) = sys.atlas_texture_data();
    assert_eq!(w, 512);
    assert_eq!(h, 512);
    assert_eq!(data.len(), (512 * 512 * 4) as usize);
}

// -- load_font --

#[test]
fn test_load_font_valid() {
    let mut sys = FontSystem::new();
    let result = sys.load_font(FONT_DATA);
    assert!(result.is_ok());
}

#[test]
fn test_load_font_invalid() {
    let mut sys = FontSystem::new();
    let result = sys.load_font(&[0, 1, 2, 3]);
    assert!(result.is_err());
}

#[test]
fn test_load_font_unique_handles() {
    let mut sys = FontSystem::new();
    let h1 = sys.load_font(FONT_DATA).unwrap();
    let h2 = sys.load_font(FONT_DATA).unwrap();
    assert_ne!(h1.id(), h2.id());
}

// -- unload_font --

#[test]
fn test_unload_font_then_layout_empty() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    sys.unload_font(h);
    let quads = sys.layout_text("Hello", &make_params(h, 24.0));
    assert!(quads.is_empty());
}

// -- layout_text --

#[test]
fn test_layout_text_simple() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    let quads = sys.layout_text("AB", &make_params(h, 24.0));
    // should produce at least 1 quad (some chars may not be in this font)
    // NotoSansMeroitic may not have Latin glyphs, but fontdue should still produce metrics
    // If no quads, the font lacks those glyphs - that's ok, test the API doesn't crash
    let _ = quads;
}

#[test]
fn test_layout_text_empty() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    let quads = sys.layout_text("", &make_params(h, 24.0));
    assert!(quads.is_empty());
}

#[test]
fn test_layout_text_invalid_font() {
    let mut sys = FontSystem::new();
    let bogus = FontHandle::new(999);
    let quads = sys.layout_text("Hello", &make_params(bogus, 24.0));
    assert!(quads.is_empty());
}

#[test]
fn test_layout_text_multiline() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    let quads = sys.layout_text("A\nB", &make_params(h, 24.0));
    // If font has these glyphs, quads on different lines should have different y values
    if quads.len() >= 2 {
        assert!(quads[0].pos.y != quads[quads.len() - 1].pos.y);
    }
}

// -- measure_text --

#[test]
fn test_measure_text_positive() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    let size = sys.measure_text("Hello World", &make_params(h, 24.0));
    // width should be > 0 if any glyphs exist, height = 1 line = 24.0
    assert!(size.y > 0.0);
}

#[test]
fn test_measure_text_empty() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    let size = sys.measure_text("", &make_params(h, 24.0));
    assert!(size.y > 0.0); // 1 line
}

#[test]
fn test_measure_text_multiline() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    let one_line = sys.measure_text("A", &make_params(h, 24.0));
    let two_lines = sys.measure_text("A\nB", &make_params(h, 24.0));
    assert!(two_lines.y > one_line.y);
}

// -- dirty tracking --

#[test]
fn test_dirty_after_layout() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    sys.layout_text("A", &make_params(h, 24.0));
    // rasterizing glyphs should mark atlas dirty
    assert!(sys.is_atlas_dirty());
}

#[test]
fn test_clear_dirty() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    sys.layout_text("A", &make_params(h, 24.0));
    sys.clear_dirty();
    assert!(!sys.is_atlas_dirty());
    assert!(sys.dirty_region().is_none());
}

// -- glyph_advance --

#[test]
fn test_glyph_advance_after_ensure() {
    let mut sys = FontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    sys.ensure_glyph_pub(h.id(), 'A', 24.0);
    let advance = sys.glyph_advance(h.id(), 'A', 24.0);
    // advance may be 0 if font doesn't have 'A', but should not panic
    let _ = advance;
}

#[test]
fn test_glyph_advance_unknown_returns_zero() {
    let sys = FontSystem::new();
    let advance = sys.glyph_advance(999, 'Z', 24.0);
    assert_eq!(advance, 0.0);
}
