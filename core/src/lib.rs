//! # lite-render-2d
//!
//! lightweight 2d rendering in rust. 100k sprites, 1 draw call, 30 MB RAM.
//!
//! ## quick start
//!
//! ```rust,ignore
//! use lite_render_2d_core::prelude::*;
//! use lite_render_2d_glow::GlowRenderer;
//!
//! let mut ren = GlowRenderer::new(&window)?;
//! ren.set_clear_color(Color::rgb(0.15, 0.15, 0.2));
//!
//! ren.begin_frame()?;
//! ren.draw_rect(Rect::new(50.0, 50.0, 200.0, 100.0), DrawParams::fill(Color::RED));
//! ren.draw_circle(Vec2::new(400.0, 300.0), 60.0, DrawParams::fill(Color::CYAN));
//! let stats = ren.end_frame()?;
//! ```
//!
//! ## coordinate system
//!
//! y-down: (0, 0) is top-left, x goes right, y goes down.
//!
//! ## backends
//!
//! - `lite-render-2d-glow` — OpenGL ES 3.0 (default, lightweight)
//! - `lite-render-2d-wgpu` — wgpu (heavier, broader gpu support)

pub mod atlas;
#[cfg(feature = "audio")]
pub mod audio;
pub mod bitmap_font;
pub mod camera;
pub mod collision;
pub mod dash;
#[cfg(feature = "text")]
pub mod font_atlas;
#[cfg(feature = "input")]
pub mod input;
#[cfg(feature = "paths")]
pub mod path_tessellation;
pub mod particle;
pub mod post_process;
pub mod prelude;
pub mod renderer;
#[cfg(feature = "text")]
pub mod rich_text;
#[cfg(feature = "text")]
pub mod sdf_font;
pub mod sprite_sheet;
#[cfg(feature = "svg")]
pub mod svg;
pub mod tessellation;
pub mod text;
pub mod tilemap;
pub mod trail;
pub mod transform_stack;
pub mod texture;
pub mod types;

#[cfg(test)]
mod tests;

pub use atlas::{AtlasRegion, TextureAtlas};
#[cfg(feature = "audio")]
pub use audio::{AudioManager, PlaybackHandle, SoundHandle};
#[cfg(feature = "input")]
pub use input::{ActionBinding, AxisBinding, ButtonBinding, InputManager};
#[cfg(feature = "text")]
pub use rich_text::{RichText, RichTextSpan};
#[cfg(feature = "text")]
pub use sdf_font::SdfFontSystem;
#[cfg(feature = "svg")]
pub use svg::{SvgDrawCommand, SvgImage};
pub use bitmap_font::{BitmapFont, BitmapGlyph, BitmapGlyphQuad};
pub use camera::Camera2D;
pub use collision::{circle_contains, circle_intersects_rect, line_intersects_line, point_in_polygon};
pub use renderer::{Renderer, RendererError};
pub use sprite_sheet::{PlaybackMode, SpriteAnimation, SpriteSheet};
pub use text::{FontHandle, GlyphPosition, TextAlign, TextLayout, TextParams};
pub use texture::{AtlasHandle, RenderTargetHandle, TextureHandle, TextureParams};
pub use particle::{ParticleConfig, ParticleEmitter, ParticleSystem};
pub use post_process::PostEffect;
pub use tilemap::{AnimatedTile, Tilemap, TilemapProjection, TilesetInfo, TILE_FLIP_H, TILE_FLIP_V, TILE_ID_MASK};
pub use trail::TrailRenderer;
pub use types::{
    BlendMode, Color, DrawParams, DrawStyle, FilterMode, FrameStats, GradientStop, LineCap,
    LineJoin, LineParams, MaterialHandle, NineSlice, Path, PathSegment, Rect, RoundedRect,
    SpriteInstance, SpriteParams, StrokeParams, StrokeStyle, Transform2D, UniformValue, Vec2,
    WrapMode,
};
