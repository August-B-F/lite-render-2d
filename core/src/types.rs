use crate::texture::TextureHandle;

/// rgba color, components 0.0–1.0
#[derive(Clone, Copy, Debug, PartialEq)]
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
    pub const YELLOW: Self = Self { r: 1.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const CYAN: Self = Self { r: 0.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const MAGENTA: Self = Self { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const GRAY: Self = Self { r: 0.5, g: 0.5, b: 0.5, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    /// opaque color from rgb (alpha = 1.0)
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// return copy with alpha replaced
    pub const fn with_alpha(self, a: f32) -> Self {
        Self { r: self.r, g: self.g, b: self.b, a }
    }

    /// parse hex string like "#FF8800" or "FF8800FF", returns None on bad input
    pub fn from_hex_str(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        let hex = u32::from_str_radix(s, 16).ok()?;
        match s.len() {
            6 | 8 => Some(Self::from_hex(hex)),
            _ => None,
        }
    }

    /// construct from 0xRRGGBB or 0xRRGGBBAA hex value
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

    /// convert srgb gamma-encoded values to linear
    pub fn from_srgb(r: f32, g: f32, b: f32, a: f32) -> Self {
        fn to_linear(v: f32) -> f32 {
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        }
        Self { r: to_linear(r), g: to_linear(g), b: to_linear(b), a }
    }

    /// convert linear color back to srgb
    pub fn to_srgb(self) -> Self {
        fn to_gamma(v: f32) -> f32 {
            if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
        }
        Self { r: to_gamma(self.r), g: to_gamma(self.g), b: to_gamma(self.b), a: self.a }
    }

    /// construct from hsl (h in 0..360, s and l in 0..1)
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

    /// construct from hsv (h in 0..360, s and v in 0..1)
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

    /// linear interpolation between two colors
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

/// 2d vector / point, y-down screen coordinates
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
    /// points up on screen (negative y)
    pub const UP: Self = Self { x: 0.0, y: -1.0 };
    /// points down on screen (positive y)
    pub const DOWN: Self = Self { x: 0.0, y: 1.0 };
    pub const LEFT: Self = Self { x: -1.0, y: 0.0 };
    pub const RIGHT: Self = Self { x: 1.0, y: 0.0 };

    /// length of the vector
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// squared length (avoids sqrt, useful for comparisons)
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// unit vector, or ZERO if length is ~0
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len < 1e-10 { Self::ZERO } else { Self { x: self.x / len, y: self.y / len } }
    }

    /// dot product
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// 2d cross product (returns scalar z-component)
    pub fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    /// distance to another point
    pub fn distance_to(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// unit vector from angle in radians (0 = right, PI/2 = down in y-down coords)
    pub fn from_angle(radians: f32) -> Self {
        Self { x: radians.cos(), y: radians.sin() }
    }

    /// angle of the vector in radians via atan2(y, x)
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    /// linear interpolation between self and other
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

/// axis-aligned rectangle, top-left origin
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// top-left corner
    pub pos: Vec2,
    /// width and height
    pub size: Vec2,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { pos: Vec2::new(x, y), size: Vec2::new(w, h) }
    }

    /// create rect centered on (cx, cy) with given size
    pub fn from_center(cx: f32, cy: f32, w: f32, h: f32) -> Self {
        Self {
            pos: Vec2::new(cx - w * 0.5, cy - h * 0.5),
            size: Vec2::new(w, h),
        }
    }

    pub fn width(self) -> f32 { self.size.x }
    pub fn height(self) -> f32 { self.size.y }
    pub fn left(self) -> f32 { self.pos.x }
    pub fn top(self) -> f32 { self.pos.y }
    pub fn right(self) -> f32 { self.pos.x + self.size.x }
    pub fn bottom(self) -> f32 { self.pos.y + self.size.y }
    pub fn center(self) -> Vec2 {
        Vec2::new(self.pos.x + self.size.x * 0.5, self.pos.y + self.size.y * 0.5)
    }
}

/// 2d transform: position + scale + rotation
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    pub pos: Vec2,
    pub scale: Vec2,
    /// rotation in radians
    pub rotation: f32,
}

impl Transform2D {
    pub const IDENTITY: Self = Self { pos: Vec2::ZERO, scale: Vec2::ONE, rotation: 0.0 };

    /// position only, scale 1, no rotation
    pub const fn new(x: f32, y: f32) -> Self {
        Self { pos: Vec2::new(x, y), scale: Vec2::ONE, rotation: 0.0 }
    }

    pub fn with_scale(mut self, sx: f32, sy: f32) -> Self {
        self.scale = Vec2::new(sx, sy);
        self
    }

    pub fn with_uniform_scale(mut self, s: f32) -> Self {
        self.scale = Vec2::new(s, s);
        self
    }

    pub fn with_rotation(mut self, radians: f32) -> Self {
        self.rotation = radians;
        self
    }

    pub fn with_rotation_deg(mut self, degrees: f32) -> Self {
        self.rotation = degrees * std::f32::consts::PI / 180.0;
        self
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// texture sampling filter
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    /// sharp pixels, good for pixel art
    Nearest,
    /// smooth interpolation
    Linear,
}

/// texture edge wrapping
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapMode {
    /// clamp to edge color
    Clamp,
    /// tile / repeat
    Repeat,
}

/// rectangle with rounded corners
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

    /// different radius per corner
    pub fn with_radii(rect: Rect, tl: f32, tr: f32, bl: f32, br: f32) -> Self {
        Self { rect, radius: 0.0, radius_tl: tl, radius_tr: tr, radius_bl: bl, radius_br: br }
    }
}

/// line stroke pattern
#[derive(Clone, Copy, Debug, Default)]
pub enum StrokeStyle {
    #[default]
    Solid,
    Dashed { dash_len: f32, gap_len: f32 },
    Dotted { spacing: f32 },
}

/// how line endpoints are drawn
#[derive(Clone, Copy, Debug, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

/// how line corners are joined
#[derive(Clone, Copy, Debug, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// parameters for stroking shapes and paths
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

/// a color stop for multi-stop gradients
#[derive(Clone, Copy, Debug)]
pub struct GradientStop {
    /// position along the gradient, 0.0 to 1.0
    pub offset: f32,
    pub color: Color,
}

/// how a shape is filled or stroked
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

impl DrawStyle {
    pub fn fill(color: Color) -> Self {
        Self::Fill(color)
    }

    pub fn stroke(color: Color, thickness: f32) -> Self {
        Self::Stroke(StrokeParams::new(color, thickness))
    }
}

impl From<DrawStyle> for DrawParams {
    fn from(style: DrawStyle) -> Self {
        Self {
            style,
            blend: BlendMode::Alpha,
            z_index: 0,
            opacity: 1.0,
        }
    }
}

impl From<Color> for DrawParams {
    fn from(color: Color) -> Self {
        Self::fill(color)
    }
}

/// compositing mode for draw calls
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

/// full draw parameters for shapes: style + blend + z-index + opacity
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

/// parameters for drawing a textured sprite
#[derive(Clone, Copy, Debug)]
pub struct SpriteParams {
    pub transform: Transform2D,
    pub tint: Color,
    /// sub-region of the texture in pixels, None = full texture
    pub src_rect: Option<Rect>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub blend: BlendMode,
    pub z_index: i32,
    pub opacity: f32,
    /// normalized origin/pivot point (0..1). (0,0) = top-left, (0.5,0.5) = center.
    pub origin: Vec2,
}

impl SpriteParams {
    /// create sprite params positioned at (x, y) with default settings
    pub fn at(x: f32, y: f32) -> Self {
        Self::new(Transform2D::new(x, y))
    }

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
            origin: Vec2::ZERO,
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

    /// set the origin/pivot point (normalized 0..1). (0.5, 0.5) = center.
    pub fn with_origin(mut self, origin: Vec2) -> Self {
        self.origin = origin;
        self
    }

    /// set origin to center (0.5, 0.5) — sprite rotates/scales around its center
    pub fn centered(mut self) -> Self {
        self.origin = Vec2::new(0.5, 0.5);
        self
    }
}

/// per-instance data for instanced sprite drawing
#[derive(Clone, Copy, Debug)]
pub struct SpriteInstance {
    pub transform: Transform2D,
    pub tint: Color,
    pub opacity: f32,
    pub src_rect: Option<Rect>,
    pub flip_x: bool,
    pub flip_y: bool,
    /// normalized origin/pivot point (0..1). (0,0) = top-left, (0.5,0.5) = center.
    pub origin: Vec2,
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
            origin: Vec2::ZERO,
        }
    }
}

/// segment of a bezier path
#[derive(Clone, Copy, Debug)]
pub enum PathSegment {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadTo { ctrl: Vec2, to: Vec2 },
    CubicTo { ctrl1: Vec2, ctrl2: Vec2, to: Vec2 },
    Close,
}

/// a bezier path built with a fluent api
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

/// parameters for drawing lines and polylines
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

/// opaque handle to a compiled custom shader material
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialHandle(pub(crate) u64);

impl MaterialHandle {
    pub fn new(id: u64) -> Self { Self(id) }
    pub fn id(&self) -> u64 { self.0 }
}

/// uniform value for custom materials
#[derive(Clone, Debug)]
pub enum UniformValue {
    Float(f32),
    Vec2(Vec2),
    Vec4(Color),
    Int(i32),
}

/// nine-slice sprite for scalable UI elements
pub struct NineSlice {
    pub texture: TextureHandle,
    pub border_left: f32,
    pub border_right: f32,
    pub border_top: f32,
    pub border_bottom: f32,
}

/// per-frame perf stats returned from end_frame
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

// -- From/Into conversions --

impl From<[f32; 2]> for Vec2 {
    fn from([x, y]: [f32; 2]) -> Self { Self { x, y } }
}

impl From<(f32, f32)> for Vec2 {
    fn from((x, y): (f32, f32)) -> Self { Self { x, y } }
}

impl From<Vec2> for [f32; 2] {
    fn from(v: Vec2) -> Self { [v.x, v.y] }
}

impl From<Vec2> for (f32, f32) {
    fn from(v: Vec2) -> Self { (v.x, v.y) }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self { Self { r, g, b, a } }
}

impl From<[f32; 3]> for Color {
    fn from([r, g, b]: [f32; 3]) -> Self { Self { r, g, b, a: 1.0 } }
}

impl From<[u8; 4]> for Color {
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
}

impl From<u32> for Color {
    fn from(hex: u32) -> Self { Self::from_hex(hex) }
}

// -- Vec2 operators --

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Self { x: self.x + rhs.x, y: self.y + rhs.y } }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Self { x: self.x - rhs.x, y: self.y - rhs.y } }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self { Self { x: self.x * s, y: self.y * s } }
}

impl std::ops::Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self { Self { x: -self.x, y: -self.y } }
}

impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) { self.x += rhs.x; self.y += rhs.y; }
}

impl std::ops::SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) { self.x -= rhs.x; self.y -= rhs.y; }
}

impl std::ops::MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, s: f32) { self.x *= s; self.y *= s; }
}
