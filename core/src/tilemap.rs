use crate::texture::TextureHandle;
use crate::types::Vec2;

#[derive(Clone, Debug)]
pub struct TilesetInfo {
    pub tile_width: f32,
    pub tile_height: f32,
    pub columns: u32,
}

// tile flip flags packed into upper bits of tile id
pub const TILE_FLIP_H: u16 = 0x8000;
pub const TILE_FLIP_V: u16 = 0x4000;
pub const TILE_ID_MASK: u16 = 0x3FFF;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TilemapProjection {
    #[default]
    Orthogonal,
    Isometric,
}

// animated tile definition — cycles through frames
#[derive(Clone, Debug)]
pub struct AnimatedTile {
    pub frames: Vec<u16>,      // tile ids to cycle through
    pub frame_duration: f32,   // seconds per frame
}

#[derive(Clone, Debug)]
pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub tile_size: f32,
    pub layers: Vec<Vec<u16>>,
    pub texture: TextureHandle,
    pub tileset: TilesetInfo,
    pub projection: TilemapProjection,
    // animated tiles: maps a tile_id to its animation def
    pub animated_tiles: Vec<(u16, AnimatedTile)>,
    anim_time: f32,
}

impl Tilemap {
    pub fn new(
        width: u32,
        height: u32,
        tile_size: f32,
        texture: TextureHandle,
        tileset: TilesetInfo,
    ) -> Self {
        Self {
            width,
            height,
            tile_size,
            layers: vec![vec![0; (width * height) as usize]],
            texture,
            tileset,
            projection: TilemapProjection::Orthogonal,
            animated_tiles: Vec::new(),
            anim_time: 0.0,
        }
    }

    // add a new empty layer, returns layer index
    pub fn add_layer(&mut self) -> usize {
        self.layers.push(vec![0; (self.width * self.height) as usize]);
        self.layers.len() - 1
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn set_tile(&mut self, x: u32, y: u32, tile_id: u16) {
        self.set_tile_layer(0, x, y, tile_id);
    }

    pub fn set_tile_layer(&mut self, layer: usize, x: u32, y: u32, tile_id: u16) {
        if layer < self.layers.len() && x < self.width && y < self.height {
            self.layers[layer][(y * self.width + x) as usize] = tile_id;
        }
    }

    pub fn get_tile(&self, x: u32, y: u32) -> u16 {
        self.get_tile_layer(0, x, y)
    }

    pub fn get_tile_layer(&self, layer: usize, x: u32, y: u32) -> u16 {
        if layer < self.layers.len() && x < self.width && y < self.height {
            self.layers[layer][(y * self.width + x) as usize]
        } else {
            0
        }
    }

    // register an animated tile
    pub fn add_animated_tile(&mut self, tile_id: u16, anim: AnimatedTile) {
        self.animated_tiles.push((tile_id, anim));
    }

    // step animation timer
    pub fn update(&mut self, dt: f32) {
        self.anim_time += dt;
    }

    // resolve a tile id through animation if applicable
    pub fn resolve_tile_id(&self, raw_id: u16) -> u16 {
        let base_id = raw_id & TILE_ID_MASK;
        for (trigger_id, anim) in &self.animated_tiles {
            if base_id == *trigger_id && !anim.frames.is_empty() {
                let total = anim.frame_duration * anim.frames.len() as f32;
                if total <= 0.0 { return anim.frames[0]; }
                let t = self.anim_time % total;
                let idx = (t / anim.frame_duration) as usize;
                return anim.frames[idx.min(anim.frames.len() - 1)];
            }
        }
        base_id
    }

    // check flip flags on a raw tile id
    pub fn tile_flip_h(raw_id: u16) -> bool {
        raw_id & TILE_FLIP_H != 0
    }

    pub fn tile_flip_v(raw_id: u16) -> bool {
        raw_id & TILE_FLIP_V != 0
    }

    // compute src rect in pixel coords for a tile id
    pub fn tile_src_rect(&self, tile_id: u16) -> crate::types::Rect {
        let id = tile_id & TILE_ID_MASK;
        let col = (id as u32) % self.tileset.columns;
        let row = (id as u32) / self.tileset.columns;
        crate::types::Rect {
            pos: Vec2::new(
                col as f32 * self.tileset.tile_width,
                row as f32 * self.tileset.tile_height,
            ),
            size: Vec2::new(self.tileset.tile_width, self.tileset.tile_height),
        }
    }

    // convert grid coords to world position based on projection
    pub fn grid_to_world(&self, col: u32, row: u32, offset: Vec2) -> Vec2 {
        let ts = self.tile_size;
        match self.projection {
            TilemapProjection::Orthogonal => {
                Vec2::new(offset.x + col as f32 * ts, offset.y + row as f32 * ts)
            }
            TilemapProjection::Isometric => {
                let x = (col as f32 - row as f32) * ts * 0.5;
                let y = (col as f32 + row as f32) * ts * 0.25;
                Vec2::new(offset.x + x, offset.y + y)
            }
        }
    }
}
