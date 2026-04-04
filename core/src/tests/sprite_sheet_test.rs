use crate::sprite_sheet::{PlaybackMode, SpriteAnimation, SpriteSheet};

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

// -- SpriteSheet --

#[test]
fn test_frame_rect_first_frame() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 8);
    let r = sheet.frame_rect(0);
    assert!(approx(r.pos.x, 0.0));
    assert!(approx(r.pos.y, 0.0));
    assert!(approx(r.size.x, 32.0));
    assert!(approx(r.size.y, 32.0));
}

#[test]
fn test_frame_rect_second_row() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 8);
    let r = sheet.frame_rect(5); // col=1, row=1
    assert!(approx(r.pos.x, 32.0));
    assert!(approx(r.pos.y, 32.0));
}

#[test]
fn test_frame_rect_last_frame() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 8);
    let r = sheet.frame_rect(7); // col=3, row=1
    assert!(approx(r.pos.x, 96.0));
    assert!(approx(r.pos.y, 32.0));
}

#[test]
fn test_frame_rect_clamps_out_of_bounds() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 8);
    let r = sheet.frame_rect(100); // should clamp to frame 7
    let expected = sheet.frame_rect(7);
    assert!(approx(r.pos.x, expected.pos.x));
    assert!(approx(r.pos.y, expected.pos.y));
}

#[test]
fn test_frame_rect_single_frame() {
    let sheet = SpriteSheet::new(64.0, 64.0, 1, 1);
    let r = sheet.frame_rect(0);
    assert!(approx(r.pos.x, 0.0));
    assert!(approx(r.pos.y, 0.0));
    assert!(approx(r.size.x, 64.0));
}

// -- SpriteAnimation Loop --

#[test]
fn test_animation_loop_advances() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Loop);
    assert_eq!(anim.current_frame(), 0);

    anim.update(0.1);
    assert_eq!(anim.current_frame(), 1);

    anim.update(0.1);
    assert_eq!(anim.current_frame(), 2);
}

#[test]
fn test_animation_loop_wraps() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Loop);
    anim.update(0.4); // advance 4 frames, should wrap to 0
    assert_eq!(anim.current_frame(), 0);
}

#[test]
fn test_animation_loop_never_finishes() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Loop);
    anim.update(10.0); // many cycles
    assert!(!anim.is_finished());
}

// -- SpriteAnimation Once --

#[test]
fn test_animation_once_stops_at_end() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Once);
    anim.update(0.5); // more than enough to reach end
    assert_eq!(anim.current_frame(), 3);
    assert!(anim.is_finished());
}

#[test]
fn test_animation_once_does_not_advance_after_finish() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Once);
    anim.update(1.0);
    assert!(anim.is_finished());
    let frame = anim.current_frame();
    anim.update(1.0);
    assert_eq!(anim.current_frame(), frame);
}

// -- SpriteAnimation PingPong --

#[test]
fn test_animation_pingpong_reverses() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::PingPong);

    // forward: 0 -> 1 -> 2 -> 3, then reverse: 2 -> 1 -> 0, then forward again
    anim.update(0.1); assert_eq!(anim.current_frame(), 1);
    anim.update(0.1); assert_eq!(anim.current_frame(), 2);
    anim.update(0.1); assert_eq!(anim.current_frame(), 3);
    anim.update(0.1); assert_eq!(anim.current_frame(), 2); // reversed
    anim.update(0.1); assert_eq!(anim.current_frame(), 1);
    anim.update(0.1); assert_eq!(anim.current_frame(), 0);
    anim.update(0.1); assert_eq!(anim.current_frame(), 1); // forward again
}

#[test]
fn test_animation_pingpong_never_finishes() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::PingPong);
    anim.update(10.0);
    assert!(!anim.is_finished());
}

// -- reset / set_frame --

#[test]
fn test_animation_reset() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Once);
    anim.update(1.0);
    assert!(anim.is_finished());
    anim.reset();
    assert_eq!(anim.current_frame(), 0);
    assert!(!anim.is_finished());
}

#[test]
fn test_animation_set_frame() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Loop);
    anim.set_frame(2);
    assert_eq!(anim.current_frame(), 2);
}

#[test]
fn test_animation_set_frame_clamps() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 4);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Loop);
    anim.set_frame(100);
    assert_eq!(anim.current_frame(), 3);
}

// -- current_src_rect --

#[test]
fn test_current_src_rect_matches_frame() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 8);
    let mut anim = SpriteAnimation::new(sheet.clone(), 0.1, PlaybackMode::Loop);
    anim.update(0.2); // frame 2
    let r = anim.current_src_rect();
    let expected = sheet.frame_rect(2);
    assert!(approx(r.pos.x, expected.pos.x));
    assert!(approx(r.pos.y, expected.pos.y));
}

// -- zero frame count edge case --

#[test]
fn test_animation_zero_frames_no_panic() {
    let sheet = SpriteSheet::new(32.0, 32.0, 4, 0);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Loop);
    anim.update(1.0); // should not panic
    assert_eq!(anim.current_frame(), 0);
}

#[test]
fn test_animation_single_frame_no_advance() {
    let sheet = SpriteSheet::new(32.0, 32.0, 1, 1);
    let mut anim = SpriteAnimation::new(sheet, 0.1, PlaybackMode::Loop);
    anim.update(1.0);
    assert_eq!(anim.current_frame(), 0);
}
