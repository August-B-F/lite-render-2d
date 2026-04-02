use crate::texture::TextureHandle;

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const RED: Self = Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Self = Self { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Self = Self { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };

    // construct from 0xRRGGBB or 0xRRGGBBAA hex value
    pub fn from_hex(hex: u32) -> Self {
        if hex > 0xFFFFFF {
            // 0xRRGGBBAA format
            Self {
                r: ((hex >> 24) & 0xFF) as f32 / 255.0,
                g: ((hex >> 16) & 0xFF) as f32 / 255.0,
                b: ((hex >> 8) & 0xFF) as f32 / 255.0,
                a: (hex & 0xFF) as f32 / 255.0,
            }
        } else {
            // 0xRRGGBB format, alpha = 1
            Self {
                r: ((hex >> 16) & 0xFF) as f32 / 255.0,
                g: ((hex >> 8) & 0xFF) as f32 / 255.0,
                b: (hex & 0xFF) as f32 / 255.0,
                a: 1.0,
            }
        }
    }

    // convert srgb gamma-encoded values to linear
    pub fn from_srgb(r: f32, g: f32, b: f32, a: f32) -> Self {
        fn to_linear(v: f32) -> f32 {
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        }
        Self { r: to_linear(r), g: to_linear(g), b: to_linear(b), a }
    }

    // convert linear color back to srgb
    pub fn to_srgb(self) -> Self {
        fn to_gamma(v: f32) -> f32 {
            if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
        }
        Self { r: to_gamma(self.r), g: to_gamma(self.g), b: to_gamma(self.b), a: self.a }
    }

    // construct from hsl (h in 0..360, s and l in 0..1)
    pub fn hsl(h: f32, s: f32, l: f32) -> Self {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let h2 = h / 60.0;
        let x = c * (1.0 - (h2 % 2.0 - 1.0).abs());
        let (r1, g1, b1) = match h2 as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = l - c * 0.5;
        Self { r: r1 + m, g: g1 + m, b: b1 + m, a: 1.0 }
    }

    // construct from hsv (h in 0..360, s and v in 0..1)
    pub fn hsv(h: f32, s: f32, v: f32) -> Self {
        let c = v * s;
        let h2 = h / 60.0;
        let x = c * (1.0 - (h2 % 2.0 - 1.0).abs());
        let (r1, g1, b1) = match h2 as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = v - c;
        Self { r: r1 + m, g: g1 + m, b: b1 + m, a: 1.0 }
    }

    // linear interpolation between two colors
    pub fn lerp(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub pos: Vec2,
    pub size: Vec2,
}

#[derive(Clone, Copy, Debug)]
pub struct Transform2D {
    pub pos: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FilterMode {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapMode {
    Clamp,
    Repeat,
}

#[derive(Clone, Copy, Debug)]
pub struct RoundedRect {
    pub rect: Rect,
    pub radius: f32,
    // per-corner radii, if set these override the single radius
    pub radius_tl: f32,
    pub radius_tr: f32,
    pub radius_bl: f32,
    pub radius_br: f32,
}

impl RoundedRect {
    pub fn new(rect: Rect, radius: f32) -> Self {
        Self { rect, radius, radius_tl: radius, radius_tr: radius, radius_bl: radius, radius_br: radius }
    }

    pub fn with_radii(rect: Rect, tl: f32, tr: f32, bl: f32, br: f32) -> Self {
        Self { rect, radius: 0.0, radius_tl: tl, radius_tr: tr, radius_bl: bl, radius_br: br }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum StrokeStyle {
    #[default]
    Solid,
    Dashed { dash_len: f32, gap_len: f32 },
    Dotted { spacing: f32 },
}

#[derive(Clone, Copy, Debug, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[derive(Clone, Copy, Debug)]
pub struct StrokeParams {
    pub color: Color,
    pub thickness: f32,
    pub style: StrokeStyle,
    pub cap: LineCap,
    pub join: LineJoin,
}

impl StrokeParams {
    pub fn new(color: Color, thickness: f32) -> Self {
        Self {
            color,
            thickness,
            style: StrokeStyle::Solid,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GradientStop {
    pub offset: f32,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub enum DrawStyle {
    Fill(Color),
    Stroke(StrokeParams),
    LinearGradient { start: Vec2, end: Vec2, color_start: Color, color_end: Color },
    RadialGradient { center: Vec2, radius: f32, color_inner: Color, color_outer: Color },
    // multi-stop gradient variants
    LinearGradientStops { start: Vec2, end: Vec2, stops: Vec<GradientStop> },
    RadialGradientStops { center: Vec2, radius: f32, stops: Vec<GradientStop> },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BlendMode {
    #[default]
    Alpha,
    Additive,
    Multiply,
    Screen,
    PremultipliedAlpha,
}

#[derive(Clone, Debug)]
pub struct DrawParams {
    pub style: DrawStyle,
    pub blend: BlendMode,
    pub z_index: i32,
    pub opacity: f32,
}

impl DrawParams {
    pub fn fill(color: Color) -> Self {
        Self {
            style: DrawStyle::Fill(color),
            blend: BlendMode::Alpha,
            z_index: 0,
            opacity: 1.0,
        }
    }

    pub fn stroke(color: Color, thickness: f32) -> Self {
        Self {
            style: DrawStyle::Stroke(StrokeParams::new(color, thickness)),
            blend: BlendMode::Alpha,
            z_index: 0,
            opacity: 1.0,
        }
    }

    pub fn with_z(mut self, z: i32) -> Self {
        self.z_index = z;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_blend(mut self, blend: BlendMode) -> Self {
        self.blend = blend;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SpriteParams {
    pub transform: Transform2D,
    pub tint: Color,
    pub src_rect: Option<Rect>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub blend: BlendMode,
    pub z_index: i32,
    pub opacity: f32,
}

impl SpriteParams {
    pub fn new(transform: Transform2D) -> Self {
        Self {
            transform,
            tint: Color::WHITE,
            src_rect: None,
            flip_x: false,
            flip_y: false,
            blend: BlendMode::Alpha,
            z_index: 0,
            opacity: 1.0,
        }
    }

    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }

    pub fn with_src_rect(mut self, rect: Rect) -> Self {
        self.src_rect = Some(rect);
        self
    }

    pub fn with_flip(mut self, x: bool, y: bool) -> Self {
        self.flip_x = x;
        self.flip_y = y;
        self
    }

    pub fn with_z(mut self, z: i32) -> Self {
        self.z_index = z;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_blend(mut self, blend: BlendMode) -> Self {
        self.blend = blend;
        self
    }
}

// per-instance data for instanced sprite drawing
#[derive(Clone, Copy, Debug)]
pub struct SpriteInstance {
    pub transform: Transform2D,
    pub tint: Color,
    pub opacity: f32,
    pub src_rect: Option<Rect>,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl SpriteInstance {
    pub fn new(transform: Transform2D) -> Self {
        Self {
            transform,
            tint: Color::WHITE,
            opacity: 1.0,
            src_rect: None,
            flip_x: false,
            flip_y: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PathSegment {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadTo { ctrl: Vec2, to: Vec2 },
    CubicTo { ctrl1: Vec2, ctrl2: Vec2, to: Vec2 },
    Close,
}

#[derive(Clone, Debug, Default)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

impl Path {
    pub fn new() -> Self {
        Self { segments: Vec::new() }
    }

    pub fn move_to(mut self, p: Vec2) -> Self {
        self.segments.push(PathSegment::MoveTo(p));
        self
    }

    pub fn line_to(mut self, p: Vec2) -> Self {
        self.segments.push(PathSegment::LineTo(p));
        self
    }

    pub fn quad_to(mut self, ctrl: Vec2, to: Vec2) -> Self {
        self.segments.push(PathSegment::QuadTo { ctrl, to });
        self
    }

    pub fn cubic_to(mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) -> Self {
        self.segments.push(PathSegment::CubicTo { ctrl1, ctrl2, to });
        self
    }

    pub fn close(mut self) -> Self {
        self.segments.push(PathSegment::Close);
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LineParams {
    pub thickness: f32,
    pub color: Color,
    pub cap: LineCap,
    pub join: LineJoin,
    pub style: StrokeStyle,
    pub blend: BlendMode,
    pub z_index: i32,
    pub opacity: f32,
}

impl LineParams {
    pub fn new(color: Color, thickness: f32) -> Self {
        Self {
            thickness,
            color,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            style: StrokeStyle::Solid,
            blend: BlendMode::Alpha,
            z_index: 0,
            opacity: 1.0,
        }
    }
}

// opaque handle to a compiled custom shader material
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialHandle(pub(crate) u64);

impl MaterialHandle {
    pub fn new(id: u64) -> Self { Self(id) }
    pub fn id(&self) -> u64 { self.0 }
}

// unifrom value for custom materials
#[derive(Clone, Debug)]
pub enum UniformValue {
    Float(f32),
    Vec2(Vec2),
    Vec4(Color),
    Int(i32),
}

pub struct NineSlice {
    pub texture: TextureHandle,
    pub border_left: f32,
    pub border_right: f32,
    pub border_top: f32,
    pub border_bottom: f32,
}

// per-frame perf stats returned from end_frame
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    pub frame_time_ms: f64,
    pub draw_calls: u32,
    pub vertices: u32,
    pub texture_binds: u32,
    pub batch_flushes: u32,
    pub ram_bytes: u64,
    pub fps: f64,
}
