use crate::trail::TrailRenderer;
use crate::types::Vec2;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

// -- construction --

#[test]
fn test_new_trail_empty() {
    let trail = TrailRenderer::new(100, 5.0, 1.0);
    assert_eq!(trail.point_count(), 0);
    assert_eq!(trail.max_points, 100);
    assert!(approx(trail.width, 5.0));
    assert!(approx(trail.lifetime, 1.0));
}

// -- add_point --

#[test]
fn test_add_point() {
    let mut trail = TrailRenderer::new(100, 5.0, 1.0);
    trail.add_point(Vec2::new(0.0, 0.0));
    assert_eq!(trail.point_count(), 1);
}

#[test]
fn test_add_point_skips_too_close() {
    let mut trail = TrailRenderer::new(100, 5.0, 1.0);
    trail.add_point(Vec2::new(0.0, 0.0));
    trail.add_point(Vec2::new(0.5, 0.0)); // distance < 1.0, should skip
    assert_eq!(trail.point_count(), 1);
}

#[test]
fn test_add_point_accepts_far_enough() {
    let mut trail = TrailRenderer::new(100, 5.0, 1.0);
    trail.add_point(Vec2::new(0.0, 0.0));
    trail.add_point(Vec2::new(5.0, 0.0)); // distance > 1.0, should add
    assert_eq!(trail.point_count(), 2);
}

#[test]
fn test_add_point_evicts_oldest() {
    let mut trail = TrailRenderer::new(3, 5.0, 10.0);
    trail.add_point(Vec2::new(0.0, 0.0));
    trail.add_point(Vec2::new(10.0, 0.0));
    trail.add_point(Vec2::new(20.0, 0.0));
    trail.add_point(Vec2::new(30.0, 0.0)); // evicts first
    assert_eq!(trail.point_count(), 3);
    // first point should now be (10, 0)
    assert!(approx(trail.points[0].position.x, 10.0));
}

// -- update --

#[test]
fn test_update_ages_points() {
    let mut trail = TrailRenderer::new(100, 5.0, 2.0);
    trail.add_point(Vec2::new(0.0, 0.0));
    trail.add_point(Vec2::new(10.0, 0.0));
    trail.update(0.5);
    assert!(approx(trail.points[0].age, 0.5));
    assert!(approx(trail.points[1].age, 0.5));
}

#[test]
fn test_update_removes_expired() {
    let mut trail = TrailRenderer::new(100, 5.0, 1.0);
    trail.add_point(Vec2::new(0.0, 0.0));
    trail.add_point(Vec2::new(10.0, 0.0));
    trail.update(1.5); // both points older than lifetime
    assert_eq!(trail.point_count(), 0);
}

#[test]
fn test_update_retains_young_points() {
    let mut trail = TrailRenderer::new(100, 5.0, 2.0);
    trail.add_point(Vec2::new(0.0, 0.0));
    trail.update(0.5);
    // add a new point after some aging
    trail.add_point(Vec2::new(10.0, 0.0));
    trail.update(1.0);
    // first point age = 1.5 (alive), second = 1.0 (alive)
    assert_eq!(trail.point_count(), 2);
    trail.update(0.6);
    // first point age = 2.1 (dead), second = 1.6 (alive)
    assert_eq!(trail.point_count(), 1);
}

// -- color defaults --

#[test]
fn test_default_colors() {
    let trail = TrailRenderer::new(10, 5.0, 1.0);
    assert!(approx(trail.color_start.a, 1.0));
    assert!(approx(trail.color_end.a, 0.0)); // fades out
}
