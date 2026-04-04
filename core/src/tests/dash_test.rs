use crate::dash::dash_polyline;
use crate::types::{Path, PathSegment, StrokeStyle, Vec2};

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

// -- solid passthrough --

#[test]
fn test_solid_returns_single_segment() {
    let points = vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)];
    let result = dash_polyline(&points, &StrokeStyle::Solid, 2.0);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 2);
}

// -- dashed --

#[test]
fn test_dashed_splits_into_segments() {
    let points = vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)];
    let result = dash_polyline(&points, &StrokeStyle::Dashed { dash_len: 20.0, gap_len: 10.0 }, 2.0);
    // 100 units: dash(20) gap(10) dash(20) gap(10) dash(20) gap(10) dash(10) = ~3-4 segments
    assert!(result.len() >= 3);
}

#[test]
fn test_dashed_segments_have_correct_length() {
    let points = vec![Vec2::new(0.0, 0.0), Vec2::new(60.0, 0.0)];
    let result = dash_polyline(&points, &StrokeStyle::Dashed { dash_len: 20.0, gap_len: 10.0 }, 2.0);
    // 60 units: dash(20) gap(10) dash(20) gap(10) = 2 dash segments
    assert_eq!(result.len(), 2);

    // each dash should span ~20 units
    let seg0_len = (result[0].last().unwrap().x - result[0].first().unwrap().x).abs();
    assert!(approx(seg0_len, 20.0));
}

#[test]
fn test_dashed_diagonal_line() {
    let points = vec![Vec2::new(0.0, 0.0), Vec2::new(30.0, 40.0)]; // length = 50
    let result = dash_polyline(&points, &StrokeStyle::Dashed { dash_len: 20.0, gap_len: 5.0 }, 2.0);
    assert_eq!(result.len(), 2); // 20 + 5 + 20 + 5 = 50
}

// -- dotted --

#[test]
fn test_dotted_produces_segments() {
    let points = vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)];
    let result = dash_polyline(&points, &StrokeStyle::Dotted { spacing: 10.0 }, 4.0);
    assert!(result.len() >= 3);
}

// -- edge cases --

#[test]
fn test_single_point_returns_single_segment() {
    let points = vec![Vec2::new(5.0, 5.0)];
    let result = dash_polyline(&points, &StrokeStyle::Dashed { dash_len: 10.0, gap_len: 5.0 }, 2.0);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 1);
}

#[test]
fn test_empty_points() {
    let points: Vec<Vec2> = vec![];
    let result = dash_polyline(&points, &StrokeStyle::Dashed { dash_len: 10.0, gap_len: 5.0 }, 2.0);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 0);
}

#[test]
fn test_zero_dash_len_passthrough() {
    let points = vec![Vec2::new(0.0, 0.0), Vec2::new(50.0, 0.0)];
    let result = dash_polyline(&points, &StrokeStyle::Dashed { dash_len: 0.0, gap_len: 5.0 }, 2.0);
    // zero dash_len should return original points
    assert_eq!(result.len(), 1);
}

// -- multi-segment polyline --

#[test]
fn test_dashed_multi_segment_polyline() {
    let points = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(50.0, 0.0),
        Vec2::new(50.0, 50.0),
    ]; // total length = 100
    let result = dash_polyline(&points, &StrokeStyle::Dashed { dash_len: 30.0, gap_len: 10.0 }, 2.0);
    // 100 units with 30+10=40 cycle: should get 2-3 dashes
    assert!(result.len() >= 2);
}

// -- dash_path (bezier flattening + dashing) --

#[test]
fn test_dash_path_line_segments() {
    let path = Path {
        segments: vec![
            PathSegment::MoveTo(Vec2::new(0.0, 0.0)),
            PathSegment::LineTo(Vec2::new(100.0, 0.0)),
        ],
    };
    let result = crate::dash::dash_path(&path, &StrokeStyle::Dashed { dash_len: 25.0, gap_len: 10.0 }, 2.0, 1.0);
    assert!(result.len() >= 2);
}

#[test]
fn test_dash_path_quad_bezier() {
    let path = Path {
        segments: vec![
            PathSegment::MoveTo(Vec2::new(0.0, 0.0)),
            PathSegment::QuadTo { ctrl: Vec2::new(50.0, -50.0), to: Vec2::new(100.0, 0.0) },
        ],
    };
    let result = crate::dash::dash_path(&path, &StrokeStyle::Dashed { dash_len: 20.0, gap_len: 10.0 }, 2.0, 1.0);
    assert!(result.len() >= 2);
}

#[test]
fn test_dash_path_empty() {
    let path = Path { segments: vec![] };
    let result = crate::dash::dash_path(&path, &StrokeStyle::Dashed { dash_len: 10.0, gap_len: 5.0 }, 2.0, 1.0);
    assert!(result.is_empty());
}
