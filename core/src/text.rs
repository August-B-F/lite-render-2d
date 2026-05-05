use crate::types::{Color, Vec2};

/// opaque handle to a loaded font
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontHandle(pub(crate) u64);

impl FontHandle {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

/// horizontal text alignment
#[derive(Clone, Copy, Debug, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// position information for a single character in laid-out text
#[derive(Clone, Debug)]
pub struct GlyphPosition {
    pub byte_offset: usize,
    pub char_index: usize,
    pub line: usize,
    pub x: f32,
    pub y: f32,
    pub advance: f32,
    pub line_height: f32,
}

/// result of text layout including per-character positions
#[derive(Clone, Debug)]
pub struct TextLayout {
    pub glyphs: Vec<GlyphPosition>,
    pub size: Vec2,
    pub line_count: usize,
    pub line_offsets: Vec<f32>,
}

/// parameters for drawing text
#[derive(Clone, Debug)]
pub struct TextParams {
    pub font: FontHandle,
    pub size: f32,
    pub color: Color,
    pub align: TextAlign,
    pub position: Vec2,
    /// when set, wrap text at word boundaries within this width
    pub max_width: Option<f32>,
    /// override line height (defaults to font size if None)
    pub line_height: Option<f32>,
    /// z-index for draw ordering (default 0)
    pub z: i32,
    /// extra spacing added to each character's advance (pixels). Negative to tighten.
    pub letter_spacing: Option<f32>,
    pub underline: bool,
    pub strikethrough: bool,
}
