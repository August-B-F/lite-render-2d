use crate::atlas::{AtlasRegion, TextureAtlas};

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn make_rgba_image(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
    vec![r, g, b, a].repeat((w * h) as usize)
}

// -- construction --

#[test]
fn test_new_dimensions() {
    let atlas = TextureAtlas::new(256, 256);
    assert_eq!(atlas.width, 256);
    assert_eq!(atlas.height, 256);
    let (data, w, h) = atlas.texture_data();
    assert_eq!(w, 256);
    assert_eq!(h, 256);
    assert_eq!(data.len(), (256 * 256 * 4) as usize);
}

#[test]
fn test_new_data_zeroed() {
    let atlas = TextureAtlas::new(64, 64);
    let (data, _, _) = atlas.texture_data();
    assert!(data.iter().all(|&b| b == 0));
}

#[test]
fn test_new_not_dirty() {
    let atlas = TextureAtlas::new(256, 256);
    assert!(!atlas.is_dirty());
    assert!(atlas.dirty_region().is_none());
}

// -- add_image --

#[test]
fn test_add_image_single() {
    let mut atlas = TextureAtlas::new(256, 256);
    let pixels = make_rgba_image(4, 4, 255, 0, 0, 255);
    let region = atlas.add_image(&pixels, 4, 4);
    assert!(region.is_some());
    let r = region.unwrap();
    assert_eq!(r.x, 0);
    assert_eq!(r.y, 0);
    assert_eq!(r.width, 4);
    assert_eq!(r.height, 4);
}

#[test]
fn test_add_image_pixel_data() {
    let mut atlas = TextureAtlas::new(256, 256);
    // 2x2 image: red, green, blue, white
    let pixels: Vec<u8> = vec![
        255, 0, 0, 255,   0, 255, 0, 255,
        0, 0, 255, 255,   255, 255, 255, 255,
    ];
    let region = atlas.add_image(&pixels, 2, 2).unwrap();
    let sub = atlas.atlas_sub_data(region.x, region.y, region.width, region.height);
    assert_eq!(sub, pixels);
}

#[test]
fn test_add_image_sets_dirty() {
    let mut atlas = TextureAtlas::new(256, 256);
    let pixels = make_rgba_image(4, 4, 255, 0, 0, 255);
    atlas.add_image(&pixels, 4, 4);
    assert!(atlas.is_dirty());
}

#[test]
fn test_add_image_dirty_region_bounds() {
    let mut atlas = TextureAtlas::new(256, 256);
    let pixels = make_rgba_image(8, 6, 255, 0, 0, 255);
    atlas.add_image(&pixels, 8, 6);
    let (dx, dy, dw, dh) = atlas.dirty_region().unwrap();
    assert_eq!(dx, 0);
    assert_eq!(dy, 0);
    assert!(dw >= 8);
    assert!(dh >= 6);
}

#[test]
fn test_add_image_multiple() {
    let mut atlas = TextureAtlas::new(256, 256);
    let img1 = make_rgba_image(10, 10, 255, 0, 0, 255);
    let img2 = make_rgba_image(10, 10, 0, 255, 0, 255);
    let r1 = atlas.add_image(&img1, 10, 10).unwrap();
    let r2 = atlas.add_image(&img2, 10, 10).unwrap();
    // second image should be next to the first (shelf packing)
    assert_eq!(r1.y, 0);
    assert_eq!(r2.y, 0);
    assert!(r2.x > r1.x);
}

#[test]
fn test_add_image_fills_atlas() {
    let mut atlas = TextureAtlas::new(20, 20);
    let img = make_rgba_image(8, 8, 255, 0, 0, 255);
    // 20x20 atlas, 1px padding: first at (0,0) shelf_x=9
    // second at (9,0) shelf_x=18, shelf_row_height=8
    // third: 18+8+1=27>20, new row y=9; (0,9) fits (9+8=17<20 height)
    // fourth: shelf_x=9 on row 9; 9+8+1=18<20 fits; y=9+8=17<20
    // fifth: new row y=18; 18+8+1=27>20, fails
    let r1 = atlas.add_image(&img, 8, 8);
    assert!(r1.is_some());
    let r2 = atlas.add_image(&img, 8, 8);
    assert!(r2.is_some());
    let r3 = atlas.add_image(&img, 8, 8);
    assert!(r3.is_some());
    let r4 = atlas.add_image(&img, 8, 8);
    assert!(r4.is_some());
    // fifth should fail
    let r5 = atlas.add_image(&img, 8, 8);
    assert!(r5.is_none());
}

#[test]
fn test_add_image_region_count() {
    let mut atlas = TextureAtlas::new(256, 256);
    for _ in 0..5 {
        let img = make_rgba_image(4, 4, 100, 100, 100, 255);
        atlas.add_image(&img, 4, 4);
    }
    assert_eq!(atlas.region_count(), 5);
}

// -- clear_dirty --

#[test]
fn test_clear_dirty() {
    let mut atlas = TextureAtlas::new(256, 256);
    let img = make_rgba_image(4, 4, 255, 0, 0, 255);
    atlas.add_image(&img, 4, 4);
    assert!(atlas.is_dirty());
    atlas.clear_dirty();
    assert!(!atlas.is_dirty());
}

#[test]
fn test_clear_dirty_region_none() {
    let mut atlas = TextureAtlas::new(256, 256);
    let img = make_rgba_image(4, 4, 255, 0, 0, 255);
    atlas.add_image(&img, 4, 4);
    atlas.clear_dirty();
    assert!(atlas.dirty_region().is_none());
}

// -- atlas_sub_data --

#[test]
fn test_sub_data_correct() {
    let mut atlas = TextureAtlas::new(256, 256);
    let pixels = make_rgba_image(3, 3, 42, 84, 126, 200);
    let region = atlas.add_image(&pixels, 3, 3).unwrap();
    let sub = atlas.atlas_sub_data(region.x, region.y, region.width, region.height);
    assert_eq!(sub.len(), (3 * 3 * 4) as usize);
    // every pixel should match
    for chunk in sub.chunks(4) {
        assert_eq!(chunk, &[42, 84, 126, 200]);
    }
}

// -- AtlasRegion uv_rect / src_rect --

#[test]
fn test_uv_rect_normalized() {
    let region = AtlasRegion { x: 10, y: 20, width: 30, height: 40 };
    let uv = region.uv_rect(256, 256);
    assert!(approx(uv.pos.x, 10.0 / 256.0));
    assert!(approx(uv.pos.y, 20.0 / 256.0));
    assert!(approx(uv.size.x, 30.0 / 256.0));
    assert!(approx(uv.size.y, 40.0 / 256.0));
}

#[test]
fn test_uv_rect_full_atlas() {
    let region = AtlasRegion { x: 0, y: 0, width: 256, height: 256 };
    let uv = region.uv_rect(256, 256);
    assert!(approx(uv.pos.x, 0.0));
    assert!(approx(uv.pos.y, 0.0));
    assert!(approx(uv.size.x, 1.0));
    assert!(approx(uv.size.y, 1.0));
}

#[test]
fn test_src_rect_values() {
    let region = AtlasRegion { x: 10, y: 20, width: 30, height: 40 };
    let r = region.src_rect();
    assert!(approx(r.pos.x, 10.0));
    assert!(approx(r.pos.y, 20.0));
    assert!(approx(r.size.x, 30.0));
    assert!(approx(r.size.y, 40.0));
}

// -- grow --

#[test]
fn test_grow_changes_dimensions() {
    let mut atlas = TextureAtlas::new(256, 256);
    let result = atlas.grow();
    assert!(result.is_some());
    // 256x256 (square) => doubles width first => 512x256
    assert_eq!(atlas.width, 512);
    assert_eq!(atlas.height, 256);
}

#[test]
fn test_grow_repacks_images() {
    let mut atlas = TextureAtlas::new(64, 64);
    let img1 = make_rgba_image(8, 8, 255, 0, 0, 255);
    let img2 = make_rgba_image(8, 8, 0, 255, 0, 255);
    atlas.add_image(&img1, 8, 8);
    atlas.add_image(&img2, 8, 8);
    let count_before = atlas.region_count();

    let new_regions = atlas.grow().unwrap();
    assert_eq!(atlas.region_count(), count_before);
    assert_eq!(new_regions.len(), count_before);

    // verify pixel data was preserved for the first region
    let r = new_regions[0];
    let sub = atlas.atlas_sub_data(r.x, r.y, r.width, r.height);
    for chunk in sub.chunks(4) {
        assert_eq!(chunk, &[255, 0, 0, 255]);
    }
}

#[test]
fn test_grow_at_max_returns_none() {
    let mut atlas = TextureAtlas::new(4096, 4096);
    assert!(atlas.grow().is_none());
}

#[test]
fn test_grow_clears_dirty_rect() {
    let mut atlas = TextureAtlas::new(256, 256);
    let img = make_rgba_image(4, 4, 255, 0, 0, 255);
    atlas.add_image(&img, 4, 4);
    atlas.grow();
    // grow needs full re-upload, dirty_rect should be None
    assert!(atlas.dirty_region().is_none());
}

#[test]
fn test_grow_allows_more_images() {
    let mut atlas = TextureAtlas::new(20, 20);
    let img = make_rgba_image(8, 8, 255, 0, 0, 255);
    // fill it up (4 images fit, 5th fails)
    for _ in 0..4 {
        atlas.add_image(&img, 8, 8).unwrap();
    }
    assert!(atlas.add_image(&img, 8, 8).is_none());
    // grow and retry
    atlas.grow();
    assert!(atlas.add_image(&img, 8, 8).is_some());
}
