//! Convenience re-exports for the most commonly used types.
//!
//! ```
//! use lite_render_2d_core::prelude::*;
//! ```

pub use crate::camera::Camera2D;
pub use crate::renderer::{Renderer, RendererError};
pub use crate::text::{FontHandle, GlyphPosition, TextAlign, TextLayout, TextParams};
pub use crate::atlas::AtlasRegion;
pub use crate::texture::{AtlasHandle, TextureHandle, TextureParams};
pub use crate::types::{
    BlendMode, Color, DrawParams, DrawStyle, FilterMode, FrameStats, LineParams, Rect,
    SpriteParams, Transform2D, Vec2, WrapMode,
};
