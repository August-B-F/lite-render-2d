use crate::transform_stack::TransformStack;
use crate::types::{Transform2D, Vec2};

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn approx_vec(a: Vec2, b: Vec2) -> bool {
    approx(a.x, b.x) && approx(a.y, b.y)
}

// -- new / identity --

#[test]
fn test_new_is_identity() {
    let stack = TransformStack::new();
    assert!(stack.is_identity());
}

#[test]
fn test_identity_apply_passthrough() {
    let stack = TransformStack::new();
    let p = Vec2::new(42.0, 99.0);
    let result = stack.apply(p);
    assert!(approx_vec(result, p));
}

// -- push / apply --

#[test]
fn test_push_translation() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D { pos: Vec2::new(10.0, 20.0), scale: Vec2::ONE, rotation: 0.0 });
    assert!(!stack.is_identity());
    let result = stack.apply(Vec2::ZERO);
    assert!(approx_vec(result, Vec2::new(10.0, 20.0)));
}

#[test]
fn test_push_scale() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D { pos: Vec2::ZERO, scale: Vec2::new(2.0, 3.0), rotation: 0.0 });
    let result = stack.apply(Vec2::new(5.0, 10.0));
    assert!(approx_vec(result, Vec2::new(10.0, 30.0)));
}

#[test]
fn test_push_rotation_90() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D {
        pos: Vec2::ZERO,
        scale: Vec2::ONE,
        rotation: std::f32::consts::FRAC_PI_2,
    });
    let result = stack.apply(Vec2::new(1.0, 0.0));
    assert!(approx_vec(result, Vec2::new(0.0, 1.0)));
}

#[test]
fn test_push_combined_translate_scale() {
    let mut stack = TransformStack::new();
    // first push translate, then scale — scale is applied first (inner), translate second (outer)
    stack.push(Transform2D { pos: Vec2::new(100.0, 0.0), scale: Vec2::ONE, rotation: 0.0 });
    stack.push(Transform2D { pos: Vec2::ZERO, scale: Vec2::new(2.0, 2.0), rotation: 0.0 });
    let result = stack.apply(Vec2::new(5.0, 5.0));
    assert!(approx_vec(result, Vec2::new(110.0, 10.0)));
}

// -- pop --

#[test]
fn test_pop_restores_previous() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D { pos: Vec2::new(10.0, 0.0), scale: Vec2::ONE, rotation: 0.0 });
    stack.push(Transform2D { pos: Vec2::new(0.0, 20.0), scale: Vec2::ONE, rotation: 0.0 });
    stack.pop();
    let result = stack.apply(Vec2::ZERO);
    assert!(approx_vec(result, Vec2::new(10.0, 0.0)));
}

#[test]
fn test_pop_to_identity() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D { pos: Vec2::new(10.0, 20.0), scale: Vec2::ONE, rotation: 0.0 });
    stack.pop();
    assert!(stack.is_identity());
    let result = stack.apply(Vec2::new(1.0, 2.0));
    assert!(approx_vec(result, Vec2::new(1.0, 2.0)));
}

#[test]
fn test_pop_empty_is_noop() {
    let mut stack = TransformStack::new();
    stack.pop(); // should not panic
    assert!(stack.is_identity());
}

// -- reset --

#[test]
fn test_reset_clears_all() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D { pos: Vec2::new(1.0, 2.0), scale: Vec2::new(3.0, 4.0), rotation: 0.5 });
    stack.push(Transform2D { pos: Vec2::new(5.0, 6.0), scale: Vec2::ONE, rotation: 0.0 });
    stack.reset();
    assert!(stack.is_identity());
    let result = stack.apply(Vec2::new(7.0, 8.0));
    assert!(approx_vec(result, Vec2::new(7.0, 8.0)));
}

// -- multiple push/pop --

#[test]
fn test_nested_push_pop() {
    let mut stack = TransformStack::new();
    stack.push(Transform2D { pos: Vec2::new(10.0, 0.0), scale: Vec2::ONE, rotation: 0.0 });
    stack.push(Transform2D { pos: Vec2::new(0.0, 10.0), scale: Vec2::ONE, rotation: 0.0 });
    stack.push(Transform2D { pos: Vec2::new(0.0, 0.0), scale: Vec2::new(2.0, 2.0), rotation: 0.0 });

    let result = stack.apply(Vec2::new(1.0, 1.0));
    assert!(approx_vec(result, Vec2::new(12.0, 12.0)));

    stack.pop();
    let result = stack.apply(Vec2::new(1.0, 1.0));
    assert!(approx_vec(result, Vec2::new(11.0, 11.0)));

    stack.pop();
    let result = stack.apply(Vec2::new(1.0, 1.0));
    assert!(approx_vec(result, Vec2::new(11.0, 1.0)));

    stack.pop();
    assert!(stack.is_identity());
}
