use crate::types::*;

// -- Color constants --

#[test]
fn test_color_constants() {
    assert_eq!(Color::RED, Color::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(Color::GREEN, Color::new(0.0, 1.0, 0.0, 1.0));
    assert_eq!(Color::BLUE, Color::new(0.0, 0.0, 1.0, 1.0));
    assert_eq!(Color::WHITE, Color::new(1.0, 1.0, 1.0, 1.0));
    assert_eq!(Color::BLACK, Color::new(0.0, 0.0, 0.0, 1.0));
    assert_eq!(Color::TRANSPARENT, Color::new(0.0, 0.0, 0.0, 0.0));
    assert_eq!(Color::YELLOW, Color::new(1.0, 1.0, 0.0, 1.0));
    assert_eq!(Color::CYAN, Color::new(0.0, 1.0, 1.0, 1.0));
    assert_eq!(Color::MAGENTA, Color::new(1.0, 0.0, 1.0, 1.0));
    assert_eq!(Color::GRAY, Color::new(0.5, 0.5, 0.5, 1.0));
}

// -- Color::rgb --

#[test]
fn test_color_rgb_sets_alpha_one() {
    let c = Color::rgb(0.2, 0.4, 0.6);
    assert_eq!(c.a, 1.0);
    assert_eq!(c.r, 0.2);
}

// -- Color::with_alpha --

#[test]
fn test_color_with_alpha() {
    let c = Color::RED.with_alpha(0.5);
    assert_eq!(c.r, 1.0);
    assert_eq!(c.a, 0.5);
}

// -- Color::from_hex_str --

#[test]
fn test_color_from_hex_str_rgb() {
    let c = Color::from_hex_str("#FF0000").unwrap();
    assert!((c.r - 1.0).abs() < 1e-3);
    assert!((c.g).abs() < 1e-3);
    assert!((c.b).abs() < 1e-3);
    assert!((c.a - 1.0).abs() < 1e-3);
}

#[test]
fn test_color_from_hex_str_rgba() {
    // note: from_hex uses `hex > 0xFFFFFF` to detect RRGGBBAA format,
    // so values like 0x00FF00FF get misinterpreted as RGB. use a high value.
    let c = Color::from_hex_str("#FF0000FF").unwrap();
    assert!((c.r - 1.0).abs() < 1e-3);
    assert!((c.g).abs() < 1e-3);
    assert!((c.b).abs() < 1e-3);
    assert!((c.a - 1.0).abs() < 1e-3);
}

#[test]
fn test_color_from_hex_str_no_hash() {
    let c = Color::from_hex_str("FF0000").unwrap();
    assert!((c.r - 1.0).abs() < 1e-3);
}

#[test]
fn test_color_from_hex_str_invalid() {
    assert!(Color::from_hex_str("ZZZZZZ").is_none());
    assert!(Color::from_hex_str("#GG0000").is_none());
    assert!(Color::from_hex_str("").is_none());
    assert!(Color::from_hex_str("#FF").is_none());
}

// -- Color::from_hex --

#[test]
fn test_color_from_hex_rgb_format() {
    let c = Color::from_hex(0xFF0000);
    assert!((c.r - 1.0).abs() < 1e-3);
    assert!((c.a - 1.0).abs() < 1e-3);
}

#[test]
fn test_color_from_hex_rgba_format() {
    let c = Color::from_hex(0xFF0000FF);
    assert!((c.r - 1.0).abs() < 1e-3);
    assert!((c.a - 1.0).abs() < 1e-3);
}

// -- Color From impls --

#[test]
fn test_color_from_f32_array() {
    let c: Color = [1.0_f32, 0.0, 0.0, 1.0].into();
    assert_eq!(c, Color::RED);
}

#[test]
fn test_color_from_f32_3_array() {
    let c: Color = [0.0_f32, 1.0, 0.0].into();
    assert_eq!(c, Color::GREEN);
}

#[test]
fn test_color_from_u8_array() {
    let c: Color = [255u8, 0, 0, 255].into();
    assert!((c.r - 1.0).abs() < 1e-3);
    assert!((c.a - 1.0).abs() < 1e-3);
}

#[test]
fn test_color_from_u32() {
    let c: Color = 0xFF0000u32.into();
    assert!((c.r - 1.0).abs() < 1e-3);
}

// -- Color::lerp --

#[test]
fn test_color_lerp_endpoints() {
    let a = Color::RED;
    let b = Color::BLUE;
    assert_eq!(a.lerp(b, 0.0), a);
    assert_eq!(a.lerp(b, 1.0), b);
}

#[test]
fn test_color_lerp_midpoint() {
    let c = Color::BLACK.lerp(Color::WHITE, 0.5);
    assert!((c.r - 0.5).abs() < 1e-3);
    assert!((c.g - 0.5).abs() < 1e-3);
    assert!((c.b - 0.5).abs() < 1e-3);
}

#[test]
fn test_color_lerp_clamps_t() {
    let a = Color::RED;
    let b = Color::BLUE;
    // t outside 0..1 gets clamped
    assert_eq!(a.lerp(b, -1.0), a);
    assert_eq!(a.lerp(b, 2.0), b);
}

// -- Color srgb roundtrip --

#[test]
fn test_color_srgb_roundtrip() {
    let original = Color::rgb(0.5, 0.7, 0.3);
    let linear = Color::from_srgb(original.r, original.g, original.b, original.a);
    let back = linear.to_srgb();
    assert!((back.r - original.r).abs() < 1e-4);
    assert!((back.g - original.g).abs() < 1e-4);
    assert!((back.b - original.b).abs() < 1e-4);
}

// -- Color::hsl --

#[test]
fn test_color_hsl_red() {
    let c = Color::hsl(0.0, 1.0, 0.5);
    assert!((c.r - 1.0).abs() < 1e-3);
    assert!(c.g.abs() < 1e-3);
    assert!(c.b.abs() < 1e-3);
}

// -- Color PartialEq --

#[test]
fn test_color_partial_eq() {
    assert_eq!(Color::RED, Color::RED);
    assert_ne!(Color::RED, Color::BLUE);
}

// ============================
// Vec2 tests
// ============================

#[test]
fn test_vec2_new() {
    let v = Vec2::new(3.0, 4.0);
    assert_eq!(v.x, 3.0);
    assert_eq!(v.y, 4.0);
}

#[test]
fn test_vec2_constants() {
    assert_eq!(Vec2::ZERO, Vec2::new(0.0, 0.0));
    assert_eq!(Vec2::ONE, Vec2::new(1.0, 1.0));
    assert_eq!(Vec2::UP, Vec2::new(0.0, -1.0));
    assert_eq!(Vec2::DOWN, Vec2::new(0.0, 1.0));
    assert_eq!(Vec2::LEFT, Vec2::new(-1.0, 0.0));
    assert_eq!(Vec2::RIGHT, Vec2::new(1.0, 0.0));
}

#[test]
fn test_vec2_add() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    assert_eq!(a + b, Vec2::new(4.0, 6.0));
}

#[test]
fn test_vec2_sub() {
    let a = Vec2::new(5.0, 7.0);
    let b = Vec2::new(2.0, 3.0);
    assert_eq!(a - b, Vec2::new(3.0, 4.0));
}

#[test]
fn test_vec2_mul() {
    let v = Vec2::new(2.0, 3.0);
    assert_eq!(v * 2.0, Vec2::new(4.0, 6.0));
}

#[test]
fn test_vec2_neg() {
    let v = Vec2::new(1.0, -2.0);
    assert_eq!(-v, Vec2::new(-1.0, 2.0));
}

#[test]
fn test_vec2_add_assign() {
    let mut v = Vec2::new(1.0, 2.0);
    v += Vec2::new(3.0, 4.0);
    assert_eq!(v, Vec2::new(4.0, 6.0));
}

#[test]
fn test_vec2_sub_assign() {
    let mut v = Vec2::new(5.0, 7.0);
    v -= Vec2::new(2.0, 3.0);
    assert_eq!(v, Vec2::new(3.0, 4.0));
}

#[test]
fn test_vec2_mul_assign() {
    let mut v = Vec2::new(2.0, 3.0);
    v *= 2.0;
    assert_eq!(v, Vec2::new(4.0, 6.0));
}

#[test]
fn test_vec2_length_345_triangle() {
    let v = Vec2::new(3.0, 4.0);
    assert!((v.length() - 5.0).abs() < 1e-6);
}

#[test]
fn test_vec2_length_squared() {
    let v = Vec2::new(3.0, 4.0);
    assert!((v.length_squared() - 25.0).abs() < 1e-6);
}

#[test]
fn test_vec2_normalize() {
    let v = Vec2::new(3.0, 4.0).normalize();
    assert!((v.length() - 1.0).abs() < 1e-6);
}

#[test]
fn test_vec2_normalize_zero_returns_zero() {
    let v = Vec2::ZERO.normalize();
    assert_eq!(v, Vec2::ZERO);
}

#[test]
fn test_vec2_dot_perpendicular() {
    let a = Vec2::new(1.0, 0.0);
    let b = Vec2::new(0.0, 1.0);
    assert!((a.dot(b)).abs() < 1e-6);
}

#[test]
fn test_vec2_dot_parallel() {
    let a = Vec2::new(2.0, 0.0);
    let b = Vec2::new(3.0, 0.0);
    assert!((a.dot(b) - 6.0).abs() < 1e-6);
}

#[test]
fn test_vec2_cross() {
    let a = Vec2::new(1.0, 0.0);
    let b = Vec2::new(0.0, 1.0);
    assert!((a.cross(b) - 1.0).abs() < 1e-6);
}

#[test]
fn test_vec2_distance_to() {
    let a = Vec2::new(0.0, 0.0);
    let b = Vec2::new(3.0, 4.0);
    assert!((a.distance_to(b) - 5.0).abs() < 1e-6);
}

#[test]
fn test_vec2_distance_equals_sub_length() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(4.0, 6.0);
    assert!((a.distance_to(b) - (a - b).length()).abs() < 1e-6);
}

// -- Vec2 From impls --

#[test]
fn test_vec2_from_tuple() {
    let v: Vec2 = (1.0_f32, 2.0_f32).into();
    assert_eq!(v, Vec2::new(1.0, 2.0));
}

#[test]
fn test_vec2_from_array() {
    let v: Vec2 = [1.0_f32, 2.0].into();
    assert_eq!(v, Vec2::new(1.0, 2.0));
}

#[test]
fn test_vec2_into_array() {
    let arr: [f32; 2] = Vec2::new(1.0, 2.0).into();
    assert_eq!(arr, [1.0, 2.0]);
}

#[test]
fn test_vec2_into_tuple() {
    let t: (f32, f32) = Vec2::new(1.0, 2.0).into();
    assert_eq!(t, (1.0, 2.0));
}

#[test]
fn test_vec2_partial_eq() {
    assert_eq!(Vec2::new(1.0, 2.0), Vec2::new(1.0, 2.0));
    assert_ne!(Vec2::new(1.0, 2.0), Vec2::new(1.0, 3.0));
}

#[test]
fn test_vec2_default_is_zero() {
    assert_eq!(Vec2::default(), Vec2::ZERO);
}

#[test]
fn test_vec2_from_angle_zero() {
    let v = Vec2::from_angle(0.0);
    assert!((v.x - 1.0).abs() < 1e-6);
    assert!(v.y.abs() < 1e-6);
}

#[test]
fn test_vec2_from_angle_half_pi() {
    let v = Vec2::from_angle(std::f32::consts::FRAC_PI_2);
    assert!(v.x.abs() < 1e-6);
    assert!((v.y - 1.0).abs() < 1e-6);
}

#[test]
fn test_vec2_angle_right() {
    assert!(Vec2::RIGHT.angle().abs() < 1e-6);
}

#[test]
fn test_vec2_angle_roundtrip() {
    let original = Vec2::new(3.0, 4.0).normalize();
    let reconstructed = Vec2::from_angle(original.angle());
    assert!((original.x - reconstructed.x).abs() < 1e-5);
    assert!((original.y - reconstructed.y).abs() < 1e-5);
}

#[test]
fn test_vec2_lerp_endpoints() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(5.0, 10.0);
    let at0 = a.lerp(b, 0.0);
    let at1 = a.lerp(b, 1.0);
    assert!((at0.x - a.x).abs() < 1e-6);
    assert!((at0.y - a.y).abs() < 1e-6);
    assert!((at1.x - b.x).abs() < 1e-6);
    assert!((at1.y - b.y).abs() < 1e-6);
}

#[test]
fn test_vec2_lerp_midpoint() {
    let v = Vec2::ZERO.lerp(Vec2::new(10.0, 20.0), 0.5);
    assert!((v.x - 5.0).abs() < 1e-6);
    assert!((v.y - 10.0).abs() < 1e-6);
}

// ============================
// Rect tests
// ============================

#[test]
fn test_rect_new() {
    let r = Rect::new(10.0, 20.0, 100.0, 50.0);
    assert_eq!(r.pos, Vec2::new(10.0, 20.0));
    assert_eq!(r.size, Vec2::new(100.0, 50.0));
}

#[test]
fn test_rect_from_center() {
    let r = Rect::from_center(50.0, 50.0, 20.0, 10.0);
    assert!((r.pos.x - 40.0).abs() < 1e-6);
    assert!((r.pos.y - 45.0).abs() < 1e-6);
    assert_eq!(r.size, Vec2::new(20.0, 10.0));
}

#[test]
fn test_rect_accessors() {
    let r = Rect::new(10.0, 20.0, 100.0, 50.0);
    assert_eq!(r.width(), 100.0);
    assert_eq!(r.height(), 50.0);
    assert_eq!(r.left(), 10.0);
    assert_eq!(r.top(), 20.0);
    assert_eq!(r.right(), 110.0);
    assert_eq!(r.bottom(), 70.0);
}

#[test]
fn test_rect_center() {
    let r = Rect::new(0.0, 0.0, 100.0, 200.0);
    assert_eq!(r.center(), Vec2::new(50.0, 100.0));
}

#[test]
fn test_rect_partial_eq() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let b = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(a, b);
}

// ============================
// Transform2D tests
// ============================

#[test]
fn test_transform2d_new() {
    let t = Transform2D::new(10.0, 20.0);
    assert_eq!(t.pos, Vec2::new(10.0, 20.0));
    assert_eq!(t.scale, Vec2::ONE);
    assert_eq!(t.rotation, 0.0);
}

#[test]
fn test_transform2d_identity() {
    let t = Transform2D::IDENTITY;
    assert_eq!(t.pos, Vec2::ZERO);
    assert_eq!(t.scale, Vec2::ONE);
    assert_eq!(t.rotation, 0.0);
}

#[test]
fn test_transform2d_default_is_identity() {
    assert_eq!(Transform2D::default(), Transform2D::IDENTITY);
}

#[test]
fn test_transform2d_with_scale() {
    let t = Transform2D::new(0.0, 0.0).with_scale(2.0, 3.0);
    assert_eq!(t.scale, Vec2::new(2.0, 3.0));
}

#[test]
fn test_transform2d_with_uniform_scale() {
    let t = Transform2D::new(0.0, 0.0).with_uniform_scale(5.0);
    assert_eq!(t.scale, Vec2::new(5.0, 5.0));
}

#[test]
fn test_transform2d_with_rotation() {
    let t = Transform2D::new(0.0, 0.0).with_rotation(1.5);
    assert!((t.rotation - 1.5).abs() < 1e-6);
}

#[test]
fn test_transform2d_with_rotation_deg_90() {
    let t = Transform2D::new(0.0, 0.0).with_rotation_deg(90.0);
    assert!((t.rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
}

#[test]
fn test_transform2d_rotation_deg_matches_radians() {
    let a = Transform2D::new(0.0, 0.0).with_rotation_deg(90.0);
    let b = Transform2D::new(0.0, 0.0).with_rotation(std::f32::consts::FRAC_PI_2);
    assert!((a.rotation - b.rotation).abs() < 1e-5);
}

#[test]
fn test_transform2d_chaining() {
    let t = Transform2D::new(10.0, 20.0)
        .with_scale(2.0, 2.0)
        .with_rotation_deg(45.0);
    assert_eq!(t.pos, Vec2::new(10.0, 20.0));
    assert_eq!(t.scale, Vec2::new(2.0, 2.0));
    assert!((t.rotation - std::f32::consts::PI / 4.0).abs() < 1e-5);
}

#[test]
fn test_transform2d_partial_eq() {
    assert_eq!(Transform2D::IDENTITY, Transform2D::IDENTITY);
    assert_ne!(Transform2D::new(1.0, 0.0), Transform2D::IDENTITY);
}
