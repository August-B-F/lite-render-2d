use crate::types::{Color, Vec2};

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

#[derive(Clone, Copy, Debug)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl Default for TextAlign {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Clone, Debug)]
pub struct TextParams {
    pub font: FontHandle,
    pub size: f32,
    pub color: Color,
    pub align: TextAlign,
    pub position: Vec2,
}
