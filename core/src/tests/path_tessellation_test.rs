use crate::path_tessellation::*;
use crate::types::{Color, Path, PathSegment, StrokeParams, StrokeStyle, Vec2};

const FLOATS_PER_VERT: usize = 12;

// -- tessellate_path_fill --

#[test]
fn test_fill_triangle() {
    let path = Path {
        segments: vec![
            PathSegment::MoveTo(Vec2::new(0.0, 0.0)),
            PathSegment::LineTo(Vec2::new(100.0, 0.0)),
            PathSegment::LineTo(Vec2::new(50.0, 100.0)),
            PathSegment::Close,
        ],
    };
    let verts = tessellate_path_fill(&path, Color::RED);
    assert!(!verts.is_empty());
    assert_eq!(verts.len() % FLOATS_PER_VERT, 0);
}

#[test]
fn test_fill_empty_path() {
    let path = Path { segments: vec![] };
    let verts = tessellate_path_fill(&path, Color::RED);
    assert!(verts.is_empty());
}

#[test]
fn test_fill_single_moveto() {
    let path = Path {
        segments: vec![PathSegment::MoveTo(Vec2::new(10.0, 10.0))],
    };
    let verts = tessellate_path_fill(&path, Color::RED);
    assert!(verts.is_empty());
}

// -- tessellate_path_stroke --

#[test]
fn test_stroke_line() {
    let path = Path {
        segments: vec![
            PathSegment::MoveTo(Vec2::new(0.0, 0.0)),
            PathSegment::LineTo(Vec2::new(100.0, 0.0)),
        ],
    };
    let params = StrokeParams::new(Color::WHITE, 4.0);
    let verts = tessellate_path_stroke(&path, &params);
    assert!(!verts.is_empty());
    assert_eq!(verts.len() % FLOATS_PER_VERT, 0);
}

#[test]
fn test_stroke_dashed() {
    let path = Path {
        segments: vec![
            PathSegment::MoveTo(Vec2::new(0.0, 0.0)),
            PathSegment::LineTo(Vec2::new(100.0, 0.0)),
        ],
    };
    let mut params = StrokeParams::new(Color::WHITE, 4.0);
    params.style = StrokeStyle::Dashed { dash_len: 10.0, gap_len: 5.0 };
    let verts = tessellate_path_stroke(&path, &params);
    assert!(!verts.is_empty());
}

// -- tessellate_complex_polygon --

#[test]
fn test_complex_polygon_square() {
    let outer = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(100.0, 100.0),
        Vec2::new(0.0, 100.0),
    ];
    let verts = tessellate_complex_polygon(&outer, &[], Color::GREEN);
    assert!(!verts.is_empty());
    assert_eq!(verts.len() % FLOATS_PER_VERT, 0);
}

#[test]
fn test_complex_polygon_with_hole() {
    let outer = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(100.0, 100.0),
        Vec2::new(0.0, 100.0),
    ];
    let hole: Vec<Vec2> = vec![
        Vec2::new(25.0, 25.0),
        Vec2::new(75.0, 25.0),
        Vec2::new(75.0, 75.0),
        Vec2::new(25.0, 75.0),
    ];
    let verts_no_hole = tessellate_complex_polygon(&outer, &[], Color::GREEN);
    let verts_with_hole = tessellate_complex_polygon(&outer, &[hole.as_slice()], Color::GREEN);
    assert!(!verts_with_hole.is_empty());
    // with hole should have more triangles (to fill around the hole)
    assert!(verts_with_hole.len() > verts_no_hole.len());
}

#[test]
fn test_complex_polygon_empty_outer() {
    let verts = tessellate_complex_polygon(&[], &[], Color::RED);
    assert!(verts.is_empty());
}

// -- tessellate_complex_polygon_stroke --

#[test]
fn test_complex_polygon_stroke() {
    let outer = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(100.0, 100.0),
        Vec2::new(0.0, 100.0),
    ];
    let params = StrokeParams::new(Color::WHITE, 2.0);
    let verts = tessellate_complex_polygon_stroke(&outer, &[], &params);
    assert!(!verts.is_empty());
    assert_eq!(verts.len() % FLOATS_PER_VERT, 0);
}

#[test]
fn test_complex_polygon_stroke_empty() {
    let params = StrokeParams::new(Color::WHITE, 2.0);
    let verts = tessellate_complex_polygon_stroke(&[], &[], &params);
    assert!(verts.is_empty());
}
