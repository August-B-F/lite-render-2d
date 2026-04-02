/// post-processing effects applied to render targets
#[derive(Clone, Debug)]
pub enum PostEffect {
    Grayscale,
    Invert,
    Brightness(f32),
    Vignette,
    Blur(u32),
    Bloom { threshold: f32, intensity: f32, radius: u32 },
}
