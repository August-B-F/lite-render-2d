use crate::types::{FilterMode, WrapMode};

/// opaque handle to a loaded gpu texture, returned by load_texture
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

/// texture sampling parameters (filter + wrap mode)
#[derive(Clone, Copy, Debug)]
pub struct TextureParams {
    pub filter: FilterMode,
    pub wrap: WrapMode,
}

/// opaque handle to an offscreen render target
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderTargetHandle(pub(crate) u64);

impl RenderTargetHandle {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

impl TextureParams {
    /// pixel-art friendly: nearest filter, clamp wrap
    pub fn nearest() -> Self {
        Self { filter: FilterMode::Nearest, wrap: WrapMode::Clamp }
    }
}

impl Default for TextureParams {
    fn default() -> Self {
        Self {
            filter: FilterMode::Linear,
            wrap: WrapMode::Clamp,
        }
    }
}
