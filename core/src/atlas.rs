use crate::types::{Rect, Vec2};

// region returned when packing an image into the atlas
#[derive(Clone, Copy, Debug)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl AtlasRegion {
    // get normalized uv rect for this region within atlas of given size
    pub fn uv_rect(&self, atlas_w: u32, atlas_h: u32) -> Rect {
        Rect {
            pos: Vec2::new(self.x as f32 / atlas_w as f32, self.y as f32 / atlas_h as f32),
            size: Vec2::new(self.width as f32 / atlas_w as f32, self.height as f32 / atlas_h as f32),
        }
    }

    // get pixel-space src rect for use with draw_sprite
    pub fn src_rect(&self) -> Rect {
        Rect {
            pos: Vec2::new(self.x as f32, self.y as f32),
            size: Vec2::new(self.width as f32, self.height as f32),
        }
    }
}

// stored source image for atlas regrow
struct ImageEntry {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

// cpu-side texture atlas packer using shelf algorithm
pub struct TextureAtlas {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA8
    // shelf packer state
    shelf_x: u32,
    shelf_y: u32,
    shelf_row_height: u32,
    regions: Vec<AtlasRegion>,
    // stored images for regrow
    image_entries: Vec<ImageEntry>,
    // dirty rect for partial upload
    dirty_rect: Option<(u32, u32, u32, u32)>,
}

const MAX_ATLAS_DIM: u32 = 4096;

impl TextureAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0u8; (width * height * 4) as usize],
            shelf_x: 0,
            shelf_y: 0,
            shelf_row_height: 0,
            regions: Vec::new(),
            image_entries: Vec::new(),
            dirty_rect: None,
        }
    }

    // pack an rgba image into the atlas, returns region or None if full
    pub fn add_image(&mut self, pixels: &[u8], img_w: u32, img_h: u32) -> Option<AtlasRegion> {
        let pad = 1u32;

        // check if we need a new row
        if self.shelf_x + img_w + pad > self.width {
            self.shelf_y += self.shelf_row_height + pad;
            self.shelf_x = 0;
            self.shelf_row_height = 0;
        }

        // check if atlas is full
        if self.shelf_y + img_h + pad > self.height {
            return None;
        }

        let ax = self.shelf_x;
        let ay = self.shelf_y;

        // copy pixels into atlas
        for row in 0..img_h {
            for col in 0..img_w {
                let src = ((row * img_w + col) * 4) as usize;
                let dst_x = ax + col;
                let dst_y = ay + row;
                let dst = ((dst_y * self.width + dst_x) * 4) as usize;
                if src + 3 < pixels.len() && dst + 3 < self.data.len() {
                    self.data[dst] = pixels[src];
                    self.data[dst + 1] = pixels[src + 1];
                    self.data[dst + 2] = pixels[src + 2];
                    self.data[dst + 3] = pixels[src + 3];
                }
            }
        }

        self.shelf_x = ax + img_w + pad;
        self.shelf_row_height = self.shelf_row_height.max(img_h);

        // expand dirty rect to include this image
        self.dirty_rect = Some(match self.dirty_rect {
            Some((rx, ry, rw, rh)) => {
                let min_x = rx.min(ax);
                let min_y = ry.min(ay);
                let max_x = (rx + rw).max(ax + img_w);
                let max_y = (ry + rh).max(ay + img_h);
                (min_x, min_y, max_x - min_x, max_y - min_y)
            }
            None => (ax, ay, img_w, img_h),
        });

        let region = AtlasRegion { x: ax, y: ay, width: img_w, height: img_h };
        self.regions.push(region);
        // store source pixels for potentail regrow
        self.image_entries.push(ImageEntry { pixels: pixels.to_vec(), width: img_w, height: img_h });
        Some(region)
    }

    // grow atlas to larger dimensions and repack all images
    // returns new regions in same order as original add_image calls, or None if at max
    pub fn grow(&mut self) -> Option<Vec<AtlasRegion>> {
        // determien new size: double width first, then height
        let (new_w, new_h) = if self.width == self.height {
            if self.width >= MAX_ATLAS_DIM { return None; }
            (self.width * 2, self.height)
        } else {
            if self.height >= MAX_ATLAS_DIM { return None; }
            (self.width, self.height * 2)
        };

        // reset atlas state with new dimensions
        self.width = new_w;
        self.height = new_h;
        self.data = vec![0u8; (new_w * new_h * 4) as usize];
        self.shelf_x = 0;
        self.shelf_y = 0;
        self.shelf_row_height = 0;
        self.regions.clear();

        // repack all stored images
        let entries = std::mem::take(&mut self.image_entries);
        let mut new_regions = Vec::with_capacity(entries.len());
        for entry in &entries {
            // use internal pack logic (copy pixels into new data)
            let pad = 1u32;
            if self.shelf_x + entry.width + pad > self.width {
                self.shelf_y += self.shelf_row_height + pad;
                self.shelf_x = 0;
                self.shelf_row_height = 0;
            }
            let ax = self.shelf_x;
            let ay = self.shelf_y;
            for row in 0..entry.height {
                for col in 0..entry.width {
                    let src = ((row * entry.width + col) * 4) as usize;
                    let dst_x = ax + col;
                    let dst_y = ay + row;
                    let dst = ((dst_y * self.width + dst_x) * 4) as usize;
                    if src + 3 < entry.pixels.len() && dst + 3 < self.data.len() {
                        self.data[dst] = entry.pixels[src];
                        self.data[dst + 1] = entry.pixels[src + 1];
                        self.data[dst + 2] = entry.pixels[src + 2];
                        self.data[dst + 3] = entry.pixels[src + 3];
                    }
                }
            }
            self.shelf_x = ax + entry.width + pad;
            self.shelf_row_height = self.shelf_row_height.max(entry.height);
            let region = AtlasRegion { x: ax, y: ay, width: entry.width, height: entry.height };
            self.regions.push(region);
            new_regions.push(region);
        }
        self.image_entries = entries;
        self.dirty_rect = None; // grow needs full re-upload, not partial

        Some(new_regions)
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    pub fn texture_data(&self) -> (&[u8], u32, u32) {
        (&self.data, self.width, self.height)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty_rect.is_some()
    }

    pub fn dirty_region(&self) -> Option<(u32, u32, u32, u32)> {
        self.dirty_rect
    }

    // extract tightly-packed sub-rect for glTexSubImage2D
    pub fn atlas_sub_data(&self, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for row in 0..h {
            let src = (((y + row) * self.width + x) * 4) as usize;
            out.extend_from_slice(&self.data[src..src + (w * 4) as usize]);
        }
        out
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_rect = None;
    }
}
