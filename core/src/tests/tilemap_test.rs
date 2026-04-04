use crate::texture::TextureHandle;
use crate::tilemap::*;
use crate::types::Vec2;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn make_tilemap() -> Tilemap {
    Tilemap::new(
        10, 8, 16.0,
        TextureHandle::new(1),
        TilesetInfo { tile_width: 16.0, tile_height: 16.0, columns: 8 },
    )
}

// -- construction --

#[test]
fn test_new_tilemap_defaults() {
    let tm = make_tilemap();
    assert_eq!(tm.width, 10);
    assert_eq!(tm.height, 8);
    assert_eq!(tm.layer_count(), 1);
    assert_eq!(tm.projection, TilemapProjection::Orthogonal);
}

#[test]
fn test_new_tiles_are_zero() {
    let tm = make_tilemap();
    for y in 0..tm.height {
        for x in 0..tm.width {
            assert_eq!(tm.get_tile(x, y), 0);
        }
    }
}

// -- set/get tile --

#[test]
fn test_set_get_tile() {
    let mut tm = make_tilemap();
    tm.set_tile(3, 4, 42);
    assert_eq!(tm.get_tile(3, 4), 42);
}

#[test]
fn test_set_tile_out_of_bounds_noop() {
    let mut tm = make_tilemap();
    tm.set_tile(100, 100, 5);
    // no panic, and in-bounds tiles unchanged
    assert_eq!(tm.get_tile(0, 0), 0);
}

#[test]
fn test_get_tile_out_of_bounds_returns_zero() {
    let tm = make_tilemap();
    assert_eq!(tm.get_tile(999, 999), 0);
}

// -- layers --

#[test]
fn test_add_layer() {
    let mut tm = make_tilemap();
    let idx = tm.add_layer();
    assert_eq!(idx, 1);
    assert_eq!(tm.layer_count(), 2);
}

#[test]
fn test_set_get_tile_layer() {
    let mut tm = make_tilemap();
    tm.add_layer();
    tm.set_tile_layer(1, 2, 3, 99);
    assert_eq!(tm.get_tile_layer(1, 2, 3), 99);
    assert_eq!(tm.get_tile_layer(0, 2, 3), 0); // other layer unaffected
}

#[test]
fn test_get_tile_invalid_layer() {
    let tm = make_tilemap();
    assert_eq!(tm.get_tile_layer(5, 0, 0), 0);
}

// -- tile_src_rect --

#[test]
fn test_tile_src_rect_first_tile() {
    let tm = make_tilemap();
    let r = tm.tile_src_rect(0);
    assert!(approx(r.pos.x, 0.0));
    assert!(approx(r.pos.y, 0.0));
    assert!(approx(r.size.x, 16.0));
    assert!(approx(r.size.y, 16.0));
}

#[test]
fn test_tile_src_rect_tile_in_second_row() {
    let tm = make_tilemap();
    let r = tm.tile_src_rect(9); // col=1, row=1 (8 columns)
    assert!(approx(r.pos.x, 16.0));
    assert!(approx(r.pos.y, 16.0));
}

#[test]
fn test_tile_src_rect_strips_flip_flags() {
    let tm = make_tilemap();
    let r = tm.tile_src_rect(5 | TILE_FLIP_H | TILE_FLIP_V);
    let expected = tm.tile_src_rect(5);
    assert!(approx(r.pos.x, expected.pos.x));
    assert!(approx(r.pos.y, expected.pos.y));
}

// -- flip flags --

#[test]
fn test_tile_flip_h() {
    assert!(Tilemap::tile_flip_h(TILE_FLIP_H | 5));
    assert!(!Tilemap::tile_flip_h(5));
}

#[test]
fn test_tile_flip_v() {
    assert!(Tilemap::tile_flip_v(TILE_FLIP_V | 5));
    assert!(!Tilemap::tile_flip_v(5));
}

#[test]
fn test_tile_flip_both() {
    let raw = TILE_FLIP_H | TILE_FLIP_V | 10;
    assert!(Tilemap::tile_flip_h(raw));
    assert!(Tilemap::tile_flip_v(raw));
    assert_eq!(raw & TILE_ID_MASK, 10);
}

// -- grid_to_world orthogonal --

#[test]
fn test_grid_to_world_ortho_origin() {
    let tm = make_tilemap();
    let pos = tm.grid_to_world(0, 0, Vec2::ZERO);
    assert!(approx(pos.x, 0.0));
    assert!(approx(pos.y, 0.0));
}

#[test]
fn test_grid_to_world_ortho_offset() {
    let tm = make_tilemap();
    let pos = tm.grid_to_world(3, 2, Vec2::new(10.0, 20.0));
    assert!(approx(pos.x, 10.0 + 3.0 * 16.0));
    assert!(approx(pos.y, 20.0 + 2.0 * 16.0));
}

// -- grid_to_world isometric --

#[test]
fn test_grid_to_world_iso() {
    let mut tm = make_tilemap();
    tm.projection = TilemapProjection::Isometric;
    let pos = tm.grid_to_world(1, 0, Vec2::ZERO);
    assert!(approx(pos.x, 16.0 * 0.5));
    assert!(approx(pos.y, 16.0 * 0.25));
}

#[test]
fn test_grid_to_world_iso_symmetric() {
    let mut tm = make_tilemap();
    tm.projection = TilemapProjection::Isometric;
    let p1 = tm.grid_to_world(2, 2, Vec2::ZERO);
    // (2-2)*ts*0.5 = 0, (2+2)*ts*0.25 = ts
    assert!(approx(p1.x, 0.0));
    assert!(approx(p1.y, 16.0));
}

// -- animated tiles --

#[test]
fn test_resolve_tile_id_no_animation() {
    let tm = make_tilemap();
    assert_eq!(tm.resolve_tile_id(5), 5);
}

#[test]
fn test_resolve_tile_id_with_animation() {
    let mut tm = make_tilemap();
    tm.add_animated_tile(5, AnimatedTile {
        frames: vec![10, 11, 12],
        frame_duration: 0.1,
    });
    // at time 0, should resolve to first frame
    assert_eq!(tm.resolve_tile_id(5), 10);
}

#[test]
fn test_resolve_tile_id_animation_cycles() {
    let mut tm = make_tilemap();
    tm.add_animated_tile(5, AnimatedTile {
        frames: vec![10, 11, 12],
        frame_duration: 0.1,
    });
    tm.update(0.15); // should be in frame 1 (11)
    assert_eq!(tm.resolve_tile_id(5), 11);
}

#[test]
fn test_resolve_tile_id_strips_flip_flags() {
    let mut tm = make_tilemap();
    tm.add_animated_tile(5, AnimatedTile {
        frames: vec![10, 11],
        frame_duration: 0.1,
    });
    // tile id 5 with flip flags should still resolve through animation
    let result = tm.resolve_tile_id(5 | TILE_FLIP_H);
    assert_eq!(result, 10);
}
