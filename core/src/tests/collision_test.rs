use crate::collision::*;
use crate::types::{Rect, Vec2};

// Rect::contains

#[test]
fn test_rect_contains_point_inside() {
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    assert!(r.contains(Vec2::new(50.0, 50.0)));
}

#[test]
fn test_rect_contains_point_outside() {
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    assert!(!r.contains(Vec2::new(150.0, 50.0)));
    assert!(!r.contains(Vec2::new(-1.0, 50.0)));
    assert!(!r.contains(Vec2::new(50.0, -1.0)));
    assert!(!r.contains(Vec2::new(50.0, 101.0)));
}

#[test]
fn test_rect_contains_point_on_edge() {
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    // edges are inclusive
    assert!(r.contains(Vec2::new(0.0, 0.0)));
    assert!(r.contains(Vec2::new(100.0, 100.0)));
    assert!(r.contains(Vec2::new(100.0, 0.0)));
    assert!(r.contains(Vec2::new(0.0, 100.0)));
}

// Rect::intersects

#[test]
fn test_rect_intersects_overlapping() {
    let a = Rect::new(0.0, 0.0, 100.0, 100.0);
    let b = Rect::new(50.0, 50.0, 100.0, 100.0);
    assert!(a.intersects(&b));
    assert!(b.intersects(&a));
}

#[test]
fn test_rect_intersects_separate() {
    let a = Rect::new(0.0, 0.0, 50.0, 50.0);
    let b = Rect::new(100.0, 100.0, 50.0, 50.0);
    assert!(!a.intersects(&b));
}

#[test]
fn test_rect_intersects_adjacent_not_overlapping() {
    // exactly touching edges - strict < means no overlap
    let a = Rect::new(0.0, 0.0, 50.0, 50.0);
    let b = Rect::new(50.0, 0.0, 50.0, 50.0);
    assert!(!a.intersects(&b));
}

#[test]
fn test_rect_intersects_contained() {
    let outer = Rect::new(0.0, 0.0, 200.0, 200.0);
    let inner = Rect::new(50.0, 50.0, 20.0, 20.0);
    assert!(outer.intersects(&inner));
    assert!(inner.intersects(&outer));
}

// circle_contains

#[test]
fn test_circle_contains_center() {
    assert!(circle_contains(Vec2::new(50.0, 50.0), 25.0, Vec2::new(50.0, 50.0)));
}

#[test]
fn test_circle_contains_inside() {
    assert!(circle_contains(Vec2::new(0.0, 0.0), 10.0, Vec2::new(3.0, 4.0)));
}

#[test]
fn test_circle_contains_outside() {
    assert!(!circle_contains(Vec2::new(0.0, 0.0), 10.0, Vec2::new(11.0, 0.0)));
}

#[test]
fn test_circle_contains_on_edge() {
    // exactly on boundary should be inside (<=)
    assert!(circle_contains(Vec2::new(0.0, 0.0), 5.0, Vec2::new(5.0, 0.0)));
}

// circle_intersects_rect

#[test]
fn test_circle_rect_overlap() {
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    // circle centered inside the rect
    assert!(circle_intersects_rect(Vec2::new(50.0, 50.0), 10.0, &r));
}

#[test]
fn test_circle_rect_outside() {
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    // circle far away
    assert!(!circle_intersects_rect(Vec2::new(200.0, 200.0), 10.0, &r));
}

#[test]
fn test_circle_rect_edge_touching() {
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    // circle just touching the right edge
    assert!(circle_intersects_rect(Vec2::new(110.0, 50.0), 10.0, &r));
}

#[test]
fn test_circle_rect_corner_case() {
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    // circle near corner but not quite touching
    assert!(!circle_intersects_rect(Vec2::new(108.0, 108.0), 10.0, &r));
}

// point_in_polygon

#[test]
fn test_point_in_convex_polygon() {
    // square polygon
    let poly = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(100.0, 100.0),
        Vec2::new(0.0, 100.0),
    ];
    assert!(point_in_polygon(Vec2::new(50.0, 50.0), &poly));
}

#[test]
fn test_point_outside_convex_polygon() {
    let poly = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(100.0, 100.0),
        Vec2::new(0.0, 100.0),
    ];
    assert!(!point_in_polygon(Vec2::new(150.0, 50.0), &poly));
}

#[test]
fn test_point_in_concave_polygon() {
    // L-shaped polygon
    let poly = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(100.0, 50.0),
        Vec2::new(50.0, 50.0),
        Vec2::new(50.0, 100.0),
        Vec2::new(0.0, 100.0),
    ];
    // inside the L
    assert!(point_in_polygon(Vec2::new(25.0, 75.0), &poly));
    // in the cutout area
    assert!(!point_in_polygon(Vec2::new(75.0, 75.0), &poly));
}

#[test]
fn test_point_in_polygon_too_few_verts() {
    assert!(!point_in_polygon(Vec2::new(0.0, 0.0), &[]));
    assert!(!point_in_polygon(Vec2::new(0.0, 0.0), &[Vec2::ZERO]));
    assert!(!point_in_polygon(Vec2::new(0.0, 0.0), &[Vec2::ZERO, Vec2::ONE]));
}

// line_intersects_line

#[test]
fn test_lines_crossing() {
    let result = line_intersects_line(
        Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0),
        Vec2::new(10.0, 0.0), Vec2::new(0.0, 10.0),
    );
    assert!(result.is_some());
    let p = result.unwrap();
    assert!((p.x - 5.0).abs() < 1e-3);
    assert!((p.y - 5.0).abs() < 1e-3);
}

#[test]
fn test_lines_parallel() {
    let result = line_intersects_line(
        Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0),
        Vec2::new(0.0, 5.0), Vec2::new(10.0, 5.0),
    );
    assert!(result.is_none());
}

#[test]
fn test_lines_collinear() {
    let result = line_intersects_line(
        Vec2::new(0.0, 0.0), Vec2::new(5.0, 0.0),
        Vec2::new(3.0, 0.0), Vec2::new(8.0, 0.0),
    );
    // collinear treated as parallel (denom ~0)
    assert!(result.is_none());
}

#[test]
fn test_lines_not_reaching() {
    // segments that would cross if extended but dont actually reach
    let result = line_intersects_line(
        Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0),
        Vec2::new(5.0, 0.0), Vec2::new(6.0, 1.0),
    );
    assert!(result.is_none());
}

#[test]
fn test_lines_endpoint_touching() {
    let result = line_intersects_line(
        Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0),
        Vec2::new(5.0, 5.0), Vec2::new(10.0, 0.0),
    );
    assert!(result.is_some());
    let p = result.unwrap();
    assert!((p.x - 5.0).abs() < 1e-3);
    assert!((p.y - 5.0).abs() < 1e-3);
}
