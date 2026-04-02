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
}
