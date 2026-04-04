use crate::tessellation::*;
use crate::transform_stack::TransformStack;
use crate::types::{Color, DrawStyle, GradientStop, LineParams, RoundedRect, Rect, StrokeParams, Transform2D, Vec2};

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

const FLOATS_PER_VERT: usize = 12;

// -- linear_gradient_color --

#[test]
fn test_linear_gradient_at_start() {
    let c = linear_gradient_color(
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Color::RED,
        Color::BLUE,
    );
    assert!(approx(c.r, 1.0));
    assert!(approx(c.b, 0.0));
}

#[test]
fn test_linear_gradient_at_end() {
    let c = linear_gradient_color(
        Vec2::new(100.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Color::RED,
        Color::BLUE,
    );
    assert!(approx(c.r, 0.0));
    assert!(approx(c.b, 1.0));
}

#[test]
fn test_linear_gradient_midpoint() {
    let c = linear_gradient_color(
        Vec2::new(50.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Color::BLACK,
        Color::WHITE,
    );
    assert!(approx(c.r, 0.5));
    assert!(approx(c.g, 0.5));
    assert!(approx(c.b, 0.5));
}

#[test]
fn test_linear_gradient_zero_length() {
    let c = linear_gradient_color(
        Vec2::new(50.0, 50.0),
        Vec2::new(10.0, 10.0),
        Vec2::new(10.0, 10.0), // start == end
        Color::RED,
        Color::BLUE,
    );
    // should return start color
    assert!(approx(c.r, 1.0));
}

// -- radial_gradient_color --

#[test]
fn test_radial_gradient_at_center() {
    let c = radial_gradient_color(
        Vec2::new(50.0, 50.0),
        Vec2::new(50.0, 50.0),
        100.0,
        Color::RED,
        Color::BLUE,
    );
    assert!(approx(c.r, 1.0));
    assert!(approx(c.b, 0.0));
}

#[test]
fn test_radial_gradient_at_edge() {
    let c = radial_gradient_color(
        Vec2::new(150.0, 50.0),
        Vec2::new(50.0, 50.0),
        100.0,
        Color::RED,
        Color::BLUE,
    );
    assert!(approx(c.r, 0.0));
    assert!(approx(c.b, 1.0));
}

#[test]
fn test_radial_gradient_zero_radius() {
    let c = radial_gradient_color(
        Vec2::new(10.0, 10.0),
        Vec2::new(0.0, 0.0),
        0.0,
        Color::RED,
        Color::BLUE,
    );
    assert!(approx(c.r, 1.0)); // should return inner color
}

// -- tessellate_triangle --

#[test]
fn test_triangle_produces_3_verts() {
    let verts = tessellate_triangle(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(5.0, 10.0),
        Color::RED,
    );
    assert_eq!(verts.len(), FLOATS_PER_VERT * 3);
}

#[test]
fn test_triangle_positions() {
    let verts = tessellate_triangle(
        Vec2::new(1.0, 2.0),
        Vec2::new(3.0, 4.0),
        Vec2::new(5.0, 6.0),
        Color::WHITE,
    );
    assert!(approx(verts[0], 1.0)); // x of first vertex
    assert!(approx(verts[1], 2.0)); // y of first vertex
    assert!(approx(verts[FLOATS_PER_VERT], 3.0)); // x of second vertex
}

#[test]
fn test_triangle_color() {
    let verts = tessellate_triangle(
        Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0),
        Color::new(0.2, 0.4, 0.6, 0.8),
    );
    assert!(approx(verts[4], 0.2)); // r
    assert!(approx(verts[5], 0.4)); // g
    assert!(approx(verts[6], 0.6)); // b
    assert!(approx(verts[7], 0.8)); // a
}

// -- tessellate_triangle_stroke --

#[test]
fn test_triangle_stroke_produces_verts() {
    let verts = tessellate_triangle_stroke(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(5.0, 10.0),
        &StrokeParams::new(Color::RED, 2.0),
    );
    assert!(verts.len() > 0);
    assert_eq!(verts.len() % FLOATS_PER_VERT, 0);
}

// -- tessellate_convex_polygon --

#[test]
fn test_polygon_quad() {
    let points = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(10.0, 10.0),
        Vec2::new(0.0, 10.0),
    ];
    let verts = tessellate_convex_polygon(&points, Color::GREEN);
    // quad = 2 triangles = 6 verts
    assert_eq!(verts.len(), FLOATS_PER_VERT * 6);
}

#[test]
fn test_polygon_pentagon() {
    let points = vec![
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(2.0, 0.5),
        Vec2::new(2.0, 1.5),
        Vec2::new(0.0, 1.5),
    ];
    let verts = tessellate_convex_polygon(&points, Color::WHITE);
    // 5 points = 3 triangles = 9 verts
    assert_eq!(verts.len(), FLOATS_PER_VERT * 9);
}

#[test]
fn test_polygon_too_few_points() {
    assert!(tessellate_convex_polygon(&[], Color::RED).is_empty());
    assert!(tessellate_convex_polygon(&[Vec2::ZERO], Color::RED).is_empty());
    assert!(tessellate_convex_polygon(&[Vec2::ZERO, Vec2::new(1.0, 0.0)], Color::RED).is_empty());
}

#[test]
fn test_polygon_stroke_too_few_points() {
    let params = StrokeParams::new(Color::RED, 2.0);
    assert!(tessellate_polygon_stroke(&[], &params).is_empty());
    assert!(tessellate_polygon_stroke(&[Vec2::ZERO], &params).is_empty());
}

// -- tessellate_ellipse_fill --

#[test]
fn test_ellipse_fill_produces_fan() {
    let verts = tessellate_ellipse_fill(Vec2::new(50.0, 50.0), Vec2::new(30.0, 20.0), Color::CYAN);
    assert!(verts.len() > 0);
    assert_eq!(verts.len() % (FLOATS_PER_VERT * 3), 0); // all triangles
}

#[test]
fn test_ellipse_center_vertex_at_center() {
    let center = Vec2::new(100.0, 200.0);
    let verts = tessellate_ellipse_fill(center, Vec2::new(50.0, 50.0), Color::WHITE);
    // first triangle's first vertex should be the center
    assert!(approx(verts[0], 100.0));
    assert!(approx(verts[1], 200.0));
}

// -- tessellate_ellipse_stroke --

#[test]
fn test_ellipse_stroke_produces_verts() {
    let params = StrokeParams::new(Color::RED, 2.0);
    let verts = tessellate_ellipse_stroke(Vec2::new(50.0, 50.0), Vec2::new(30.0, 20.0), &params);
    assert!(verts.len() > 0);
}

// -- tessellate_arc_fill --

#[test]
fn test_arc_fill_quarter() {
    let verts = tessellate_arc_fill(
        Vec2::new(0.0, 0.0),
        50.0,
        0.0,
        std::f32::consts::FRAC_PI_2,
        Color::RED,
    );
    assert!(verts.len() > 0);
    assert_eq!(verts.len() % (FLOATS_PER_VERT * 3), 0);
}

#[test]
fn test_arc_fill_full_circle() {
    let verts = tessellate_arc_fill(
        Vec2::new(0.0, 0.0),
        50.0,
        0.0,
        std::f32::consts::TAU,
        Color::RED,
    );
    assert!(verts.len() >= FLOATS_PER_VERT * 3 * 4); // at least 4 triangles
}

// -- tessellate_arc_stroke --

#[test]
fn test_arc_stroke_produces_verts() {
    let params = StrokeParams::new(Color::RED, 2.0);
    let verts = tessellate_arc_stroke(
        Vec2::new(0.0, 0.0),
        50.0,
        0.0,
        std::f32::consts::PI,
        &params,
    );
    assert!(verts.len() > 0);
}

// -- tessellate_rounded_rect_fill --

#[test]
fn test_rounded_rect_fill_produces_verts() {
    let rrect = RoundedRect {
        rect: Rect { pos: Vec2::new(10.0, 10.0), size: Vec2::new(100.0, 80.0) },
        radius: 10.0,
        radius_tl: 10.0,
        radius_tr: 10.0,
        radius_bl: 10.0,
        radius_br: 10.0,
    };
    let verts = tessellate_rounded_rect_fill(rrect, Color::WHITE);
    assert!(verts.len() > 0);
    assert_eq!(verts.len() % FLOATS_PER_VERT, 0);
}

#[test]
fn test_rounded_rect_zero_radius() {
    let rrect = RoundedRect {
        rect: Rect { pos: Vec2::ZERO, size: Vec2::new(50.0, 50.0) },
        radius: 0.0,
        radius_tl: 0.0,
        radius_tr: 0.0,
        radius_bl: 0.0,
        radius_br: 0.0,
    };
    let verts = tessellate_rounded_rect_fill(rrect, Color::WHITE);
    assert!(verts.len() > 0); // still produces the rectangular body
}

// -- tessellate_rounded_rect_stroke --

#[test]
fn test_rounded_rect_stroke_produces_verts() {
    let rrect = RoundedRect {
        rect: Rect { pos: Vec2::new(10.0, 10.0), size: Vec2::new(100.0, 80.0) },
        radius: 10.0,
        radius_tl: 10.0,
        radius_tr: 10.0,
        radius_bl: 10.0,
        radius_br: 10.0,
    };
    let params = StrokeParams::new(Color::RED, 2.0);
    let verts = tessellate_rounded_rect_stroke(rrect, &params);
    assert!(verts.len() > 0);
    assert_eq!(verts.len() % FLOATS_PER_VERT, 0);
}

// -- tessellate_polyline --

#[test]
fn test_polyline_two_points() {
    let points = vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)];
    let params = LineParams::new(Color::WHITE, 4.0);
    let verts = tessellate_polyline(&points, &params);
    assert!(verts.len() > 0);
    assert_eq!(verts.len() % FLOATS_PER_VERT, 0);
}

#[test]
fn test_polyline_single_point_empty() {
    let points = vec![Vec2::new(0.0, 0.0)];
    let params = LineParams::new(Color::WHITE, 4.0);
    let verts = tessellate_polyline(&points, &params);
    assert!(verts.is_empty());
}

#[test]
fn test_polyline_empty_empty() {
    let points: Vec<Vec2> = vec![];
    let params = LineParams::new(Color::WHITE, 4.0);
    let verts = tessellate_polyline(&points, &params);
    assert!(verts.is_empty());
}

#[test]
fn test_polyline_many_points() {
    let points: Vec<Vec2> = (0..100).map(|i| Vec2::new(i as f32, (i as f32).sin() * 50.0)).collect();
    let params = LineParams::new(Color::WHITE, 2.0);
    let verts = tessellate_polyline(&points, &params);
    assert!(verts.len() > 0);
}

// -- apply_transform --

#[test]
fn test_apply_transform_identity() {
    let stack = TransformStack::new();
    let mut verts = vec![0.0; FLOATS_PER_VERT * 2];
    verts[0] = 10.0; verts[1] = 20.0;
    verts[FLOATS_PER_VERT] = 30.0; verts[FLOATS_PER_VERT + 1] = 40.0;
    let orig = verts.clone();
    apply_transform(&mut verts, &stack);
    assert_eq!(verts, orig); // identity => no change
}

#[test]
fn test_apply_transform_translate() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D { pos: Vec2::new(10.0, 20.0), scale: Vec2::ONE, rotation: 0.0 });
    let mut verts = vec![0.0; FLOATS_PER_VERT];
    verts[0] = 5.0; verts[1] = 5.0;
    apply_transform(&mut verts, &stack);
    assert!(approx(verts[0], 15.0));
    assert!(approx(verts[1], 25.0));
}

#[test]
fn test_apply_transform_scale() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D { pos: Vec2::ZERO, scale: Vec2::new(2.0, 3.0), rotation: 0.0 });
    let mut verts = vec![0.0; FLOATS_PER_VERT];
    verts[0] = 10.0; verts[1] = 10.0;
    apply_transform(&mut verts, &stack);
    assert!(approx(verts[0], 20.0));
    assert!(approx(verts[1], 30.0));
}

#[test]
fn test_apply_transform_empty_verts() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D { pos: Vec2::new(10.0, 10.0), scale: Vec2::ONE, rotation: 0.0 });
    let mut verts: Vec<f32> = vec![];
    apply_transform(&mut verts, &stack); // should not panic
    assert!(verts.is_empty());
}

// -- apply_gradient --

fn make_test_verts(positions: &[[f32; 2]], color: Color) -> Vec<f32> {
    let mut out = Vec::new();
    for pos in positions {
        // [x, y, u, v, r, g, b, a, ...]
        out.extend_from_slice(&[
            pos[0], pos[1], 0.0, 0.0,
            color.r, color.g, color.b, color.a,
            0.0, 0.0, 0.0, 0.0,
        ]);
    }
    out
}

#[test]
fn test_apply_gradient_fill_noop() {
    let mut verts = make_test_verts(&[[0.0, 0.0], [100.0, 0.0]], Color::RED);
    let orig = verts.clone();
    apply_gradient(&mut verts, &DrawStyle::Fill(Color::RED));
    assert_eq!(verts, orig);
}

#[test]
fn test_apply_gradient_linear() {
    let mut verts = make_test_verts(&[[0.0, 0.0], [100.0, 0.0], [50.0, 0.0]], Color::WHITE);
    apply_gradient(&mut verts, &DrawStyle::LinearGradient {
        start: Vec2::new(0.0, 0.0),
        end: Vec2::new(100.0, 0.0),
        color_start: Color::RED,
        color_end: Color::BLUE,
    });
    // vertex at x=0 should be red
    assert!(approx(verts[4], 1.0)); // r
    assert!(approx(verts[6], 0.0)); // b
    // vertex at x=100 should be blue
    assert!(approx(verts[FLOATS_PER_VERT + 4], 0.0)); // r
    assert!(approx(verts[FLOATS_PER_VERT + 6], 1.0)); // b
    // vertex at x=50 should be midpoint
    assert!(approx(verts[2 * FLOATS_PER_VERT + 4], 0.5)); // r
    assert!(approx(verts[2 * FLOATS_PER_VERT + 6], 0.5)); // b
}

#[test]
fn test_apply_gradient_radial() {
    let mut verts = make_test_verts(&[[50.0, 50.0], [150.0, 50.0]], Color::WHITE);
    apply_gradient(&mut verts, &DrawStyle::RadialGradient {
        center: Vec2::new(50.0, 50.0),
        radius: 100.0,
        color_inner: Color::RED,
        color_outer: Color::BLUE,
    });
    // center vertex should be red (inner)
    assert!(approx(verts[4], 1.0));
    assert!(approx(verts[6], 0.0));
    // edge vertex should be blue (outer)
    assert!(approx(verts[FLOATS_PER_VERT + 4], 0.0));
    assert!(approx(verts[FLOATS_PER_VERT + 6], 1.0));
}

#[test]
fn test_apply_gradient_linear_stops() {
    let mut verts = make_test_verts(&[[0.0, 0.0], [50.0, 0.0], [100.0, 0.0]], Color::WHITE);
    apply_gradient(&mut verts, &DrawStyle::LinearGradientStops {
        start: Vec2::new(0.0, 0.0),
        end: Vec2::new(100.0, 0.0),
        stops: vec![
            GradientStop { offset: 0.0, color: Color::RED },
            GradientStop { offset: 0.5, color: Color::GREEN },
            GradientStop { offset: 1.0, color: Color::BLUE },
        ],
    });
    // vertex 0 at x=0 -> stop 0.0 = RED
    assert!(approx(verts[4], 1.0)); // r
    assert!(approx(verts[5], 0.0)); // g
    // vertex 1 at x=50 -> stop 0.5 = GREEN
    assert!(approx(verts[FLOATS_PER_VERT + 4], 0.0)); // r
    assert!(approx(verts[FLOATS_PER_VERT + 5], 1.0)); // g (was failing before: GREEN.g = 0.5, not 1.0)
    // vertex 2 at x=100 -> stop 1.0 = BLUE
    assert!(approx(verts[2 * FLOATS_PER_VERT + 6], 1.0)); // b
}

#[test]
fn test_apply_gradient_radial_stops() {
    let mut verts = make_test_verts(&[[50.0, 50.0], [100.0, 50.0]], Color::WHITE);
    apply_gradient(&mut verts, &DrawStyle::RadialGradientStops {
        center: Vec2::new(50.0, 50.0),
        radius: 100.0,
        stops: vec![
            GradientStop { offset: 0.0, color: Color::RED },
            GradientStop { offset: 1.0, color: Color::BLUE },
        ],
    });
    // center vertex at distance 0 -> red
    assert!(approx(verts[4], 1.0));
    assert!(approx(verts[6], 0.0));
    // vertex at distance 50 (half radius) -> midpoint
    assert!(approx(verts[FLOATS_PER_VERT + 4], 0.5));
    assert!(approx(verts[FLOATS_PER_VERT + 6], 0.5));
}
