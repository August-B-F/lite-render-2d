use crate::camera::Camera2D;
use crate::types::Vec2;

#[test]
fn test_camera_new_defaults() {
    let cam = Camera2D::new(800.0, 600.0);
    assert_eq!(cam.position, Vec2::ZERO);
    assert_eq!(cam.zoom, 1.0);
    assert_eq!(cam.viewport, Vec2::new(800.0, 600.0));
}

#[test]
fn test_camera_centered_same_as_new() {
    let a = Camera2D::new(800.0, 600.0);
    let b = Camera2D::centered(800.0, 600.0);
    assert_eq!(a.position, b.position);
    assert_eq!(a.zoom, b.zoom);
    assert_eq!(a.viewport, b.viewport);
}

#[test]
fn test_camera_with_position() {
    let cam = Camera2D::new(800.0, 600.0).with_position(Vec2::new(100.0, 200.0));
    assert_eq!(cam.position, Vec2::new(100.0, 200.0));
}

#[test]
fn test_camera_with_zoom() {
    let cam = Camera2D::new(800.0, 600.0).with_zoom(2.0);
    assert_eq!(cam.zoom, 2.0);
}

#[test]
fn test_camera_look_at() {
    let mut cam = Camera2D::new(800.0, 600.0);
    cam.look_at(Vec2::new(50.0, 75.0));
    assert_eq!(cam.position, Vec2::new(50.0, 75.0));
}

// -- projection matrix --

#[test]
fn test_camera_projection_matrix_valid() {
    let cam = Camera2D::new(800.0, 600.0);
    let m = cam.projection_matrix();
    // should produce finite values
    for &v in &m {
        assert!(v.is_finite(), "matrix element is not finite: {}", v);
    }
}

#[test]
fn test_camera_projection_matrix_ortho_diagonal() {
    // at zoom 1, position 0, viewport 800x600:
    // left=-400, right=400, top=-300, bottom=300
    // m[0] = 2/(right-left) = 2/800
    // m[5] = 2/(top-bottom) = 2/(-600) (y-down)
    let cam = Camera2D::new(800.0, 600.0);
    let m = cam.projection_matrix();
    assert!((m[0] - 2.0 / 800.0).abs() < 1e-6);
    assert!((m[5] - 2.0 / -600.0).abs() < 1e-6);
}

// -- screen_to_world / world_to_screen --

#[test]
fn test_screen_to_world_center() {
    // center of viewport should map to camera position
    let cam = Camera2D::new(800.0, 600.0);
    let world = cam.screen_to_world(Vec2::new(400.0, 300.0));
    assert!((world.x).abs() < 1e-3);
    assert!((world.y).abs() < 1e-3);
}

#[test]
fn test_screen_to_world_top_left() {
    let cam = Camera2D::new(800.0, 600.0);
    let world = cam.screen_to_world(Vec2::new(0.0, 0.0));
    assert!((world.x - (-400.0)).abs() < 1e-3);
    assert!((world.y - (-300.0)).abs() < 1e-3);
}

#[test]
fn test_screen_to_world_bottom_right() {
    let cam = Camera2D::new(800.0, 600.0);
    let world = cam.screen_to_world(Vec2::new(800.0, 600.0));
    assert!((world.x - 400.0).abs() < 1e-3);
    assert!((world.y - 300.0).abs() < 1e-3);
}

#[test]
fn test_world_to_screen_roundtrip() {
    let cam = Camera2D::new(800.0, 600.0).with_position(Vec2::new(100.0, 50.0));
    let original = Vec2::new(200.0, 150.0);
    let screen = cam.world_to_screen(original);
    let back = cam.screen_to_world(screen);
    assert!((back.x - original.x).abs() < 1e-3);
    assert!((back.y - original.y).abs() < 1e-3);
}

#[test]
fn test_screen_to_world_roundtrip_with_zoom() {
    let cam = Camera2D::new(800.0, 600.0)
        .with_zoom(2.0)
        .with_position(Vec2::new(50.0, -30.0));
    let original = Vec2::new(123.0, 456.0);
    let screen = cam.world_to_screen(original);
    let back = cam.screen_to_world(screen);
    assert!((back.x - original.x).abs() < 1e-3);
    assert!((back.y - original.y).abs() < 1e-3);
}

// -- zoom affects visible area --

#[test]
fn test_zoom_2x_halves_visible_area() {
    let cam1 = Camera2D::new(800.0, 600.0);
    let cam2 = Camera2D::new(800.0, 600.0).with_zoom(2.0);

    // top-left corner in world space
    let tl1 = cam1.screen_to_world(Vec2::ZERO);
    let tl2 = cam2.screen_to_world(Vec2::ZERO);

    // at 2x zoom, visible range should be half
    assert!((tl2.x - tl1.x / 2.0).abs() < 1e-3);
    assert!((tl2.y - tl1.y / 2.0).abs() < 1e-3);
}

// -- pan affects world coordinates --

#[test]
fn test_pan_offsets_world() {
    let cam = Camera2D::new(800.0, 600.0).with_position(Vec2::new(100.0, 50.0));
    let world = cam.screen_to_world(Vec2::new(400.0, 300.0));
    // center of screen should map to camera position
    assert!((world.x - 100.0).abs() < 1e-3);
    assert!((world.y - 50.0).abs() < 1e-3);
}

// -- follow --

#[test]
fn test_follow_moves_toward_target() {
    let mut cam = Camera2D::new(800.0, 600.0);
    cam.follow(Vec2::new(100.0, 0.0), 5.0, 0.016);
    // should have moved toward target but not arrived
    assert!(cam.position.x > 0.0);
    assert!(cam.position.x < 100.0);
}

// -- shake --

#[test]
fn test_shake_produces_offset_after_update() {
    let mut cam = Camera2D::new(800.0, 600.0);
    cam.shake(10.0, 1.0);
    cam.update(0.1);
    // after update, the projection should include shake offset
    // we just verify the matrix is still finite
    let m = cam.projection_matrix();
    for &v in &m {
        assert!(v.is_finite());
    }
}

#[test]
fn test_shake_decays_to_zero() {
    let mut cam = Camera2D::new(800.0, 600.0);
    cam.shake(10.0, 0.5);
    // step past the duration
    cam.update(1.0);
    // after duration expired, shake offset should be zero
    // get projection with and without shake - they should match a no-shake cam
    let ref_cam = Camera2D::new(800.0, 600.0);
    let m1 = cam.projection_matrix();
    let m2 = ref_cam.projection_matrix();
    for i in 0..16 {
        assert!((m1[i] - m2[i]).abs() < 1e-3, "matrix[{}] differs: {} vs {}", i, m1[i], m2[i]);
    }
}

// -- edge cases --

#[test]
fn test_zero_viewport_no_panic() {
    let cam = Camera2D::new(0.0, 0.0);
    let _m = cam.projection_matrix();
    // just verifying no panic/crash
}

#[test]
fn test_extreme_zoom_small() {
    let cam = Camera2D::new(800.0, 600.0).with_zoom(0.001);
    let m = cam.projection_matrix();
    for &v in &m {
        assert!(v.is_finite());
    }
}

#[test]
fn test_extreme_zoom_large() {
    let cam = Camera2D::new(800.0, 600.0).with_zoom(10000.0);
    let m = cam.projection_matrix();
    for &v in &m {
        assert!(v.is_finite());
    }
}
