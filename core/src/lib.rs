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
pub use text::{FontHandle, TextAlign, TextParams};
pub use texture::{RenderTargetHandle, TextureHandle, TextureParams};
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
