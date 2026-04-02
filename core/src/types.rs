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

#[derive(Clone, Copy, Debug)]
pub enum WrapMode {
    Clamp,
    Repeat,
}

#[derive(Clone, Copy, Debug)]
pub struct RoundedRect {
    pub rect: Rect,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum StrokeStyle {
    Solid,
    Dashed { dash_len: f32, gap_len: f32 },
    Dotted { spacing: f32 },
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self::Solid
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

impl Default for LineCap {
    fn default() -> Self {
        Self::Butt
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

impl Default for LineJoin {
    fn default() -> Self {
        Self::Miter
    }
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

#[derive(Clone, Copy, Debug)]
pub enum DrawStyle {
    Fill(Color),
    Stroke(StrokeParams),
    LinearGradient { start: Vec2, end: Vec2, color_start: Color, color_end: Color },
    RadialGradient { center: Vec2, radius: f32, color_inner: Color, color_outer: Color },
}

#[derive(Clone, Copy, Debug)]
pub enum BlendMode {
    Alpha,
    Additive,
    Multiply,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Alpha
    }
}

#[derive(Clone, Copy, Debug)]
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
            blend: BlendMode::Alpha,
            z_index: 0,
            opacity: 1.0,
        }
    }
}
