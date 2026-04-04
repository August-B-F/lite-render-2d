use crate::text::FontHandle;
use crate::texture::{RenderTargetHandle, TextureHandle, TextureParams};
use crate::types::{FilterMode, WrapMode};

#[test]
fn test_texture_handle_new_id() {
    let h = TextureHandle::new(42);
    assert_eq!(h.id(), 42);
}

#[test]
fn test_texture_handle_equality() {
    assert_eq!(TextureHandle::new(1), TextureHandle::new(1));
    assert_ne!(TextureHandle::new(1), TextureHandle::new(2));
}

#[test]
fn test_render_target_handle_new_id() {
    let h = RenderTargetHandle::new(7);
    assert_eq!(h.id(), 7);
}

#[test]
fn test_font_handle_new_id() {
    let h = FontHandle::new(99);
    assert_eq!(h.id(), 99);
}

#[test]
fn test_texture_params_default() {
    let p = TextureParams::default();
    assert!(matches!(p.filter, FilterMode::Linear));
    assert!(matches!(p.wrap, WrapMode::Clamp));
}

#[test]
fn test_texture_params_nearest() {
    let p = TextureParams::nearest();
    assert!(matches!(p.filter, FilterMode::Nearest));
    assert!(matches!(p.wrap, WrapMode::Clamp));
}
