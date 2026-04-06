use crate::sdf_font::SdfFontSystem;
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
    }
}

// -- construction --

#[test]
fn test_new_not_dirty() {
    let sys = SdfFontSystem::new();
    assert!(!sys.is_atlas_dirty());
}

#[test]
fn test_new_atlas_size() {
    let sys = SdfFontSystem::new();
    let (data, w, h) = sys.atlas_texture_data();
    assert_eq!(w, 512);
    assert_eq!(h, 512);
    assert_eq!(data.len(), (512 * 512 * 4) as usize);
}

// -- load_font --

#[test]
fn test_load_font_valid() {
    let mut sys = SdfFontSystem::new();
    assert!(sys.load_font(FONT_DATA).is_ok());
}

#[test]
fn test_load_font_invalid() {
    let mut sys = SdfFontSystem::new();
    assert!(sys.load_font(&[0, 1, 2, 3]).is_err());
}

// -- unload_font --

#[test]
fn test_unload_font_then_layout_empty() {
    let mut sys = SdfFontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    sys.unload_font(h);
    let quads = sys.layout_text("Hello", &make_params(h, 24.0));
    assert!(quads.is_empty());
}

// -- layout_text --

#[test]
fn test_layout_text_simple() {
    let mut sys = SdfFontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    // just ensure no panic — font may not have Latin glyphs
    let _quads = sys.layout_text("AB", &make_params(h, 24.0));
}

#[test]
fn test_layout_text_empty() {
    let mut sys = SdfFontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    let quads = sys.layout_text("", &make_params(h, 24.0));
    assert!(quads.is_empty());
}

// -- measure_text --

#[test]
fn test_measure_text_positive() {
    let mut sys = SdfFontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    let size = sys.measure_text("Hello", &make_params(h, 24.0));
    assert!(size.y > 0.0);
}

// -- dirty tracking --

#[test]
fn test_dirty_after_layout() {
    let mut sys = SdfFontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    sys.layout_text("A", &make_params(h, 24.0));
    assert!(sys.is_atlas_dirty());
}

#[test]
fn test_clear_dirty() {
    let mut sys = SdfFontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    sys.layout_text("A", &make_params(h, 24.0));
    sys.clear_dirty();
    assert!(!sys.is_atlas_dirty());
    assert!(sys.dirty_region().is_none());
}

#[test]
fn test_atlas_sub_data_length() {
    let mut sys = SdfFontSystem::new();
    let h = sys.load_font(FONT_DATA).unwrap();
    sys.layout_text("A", &make_params(h, 24.0));
    if let Some((x, y, w, h)) = sys.dirty_region() {
        let sub = sys.atlas_sub_data(x, y, w, h);
        assert_eq!(sub.len(), (w * h * 4) as usize);
    }
}
