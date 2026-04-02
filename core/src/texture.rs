use crate::types::{FilterMode, WrapMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub(crate) u64);

impl TextureHandle {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextureParams {
    pub filter: FilterMode,
    pub wrap: WrapMode,
}

impl Default for TextureParams {
    fn default() -> Self {
        Self {
            filter: FilterMode::Linear,
            wrap: WrapMode::Clamp,
        }
    }
}
