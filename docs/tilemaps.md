# Tilemaps

## Creating a tilemap

A tilemap needs a tileset texture (sprite sheet of tiles) and info about its grid layout:

```rust
use lite_render_2d_core::tilemap::{Tilemap, TilesetInfo, AnimatedTile};

// Load the tileset texture
let tileset_tex = renderer.load_texture_from_file(
    "assets/tileset.png".as_ref(),
    TextureParams::nearest(),   // pixel art tiles should use nearest filtering
)?;

// Describe the tileset layout: each tile is 16x16 pixels, 16 columns in the texture
let tileset = TilesetInfo {
    tile_width: 16.0,
    tile_height: 16.0,
    columns: 16,
};

// Create a 100x100 tile map with 16px world-space tile size
let mut map = Tilemap::new(100, 100, 16.0, tileset_tex, tileset);
```

## Setting tiles

Tile IDs start at 1. Tile 0 means "empty" (not drawn).

```rust
// Set tile on layer 0 (default layer)
map.set_tile(5, 3, 1);     // column 5, row 3, tile ID 1
map.set_tile(6, 3, 2);     // tile ID 2

// Get a tile
let id = map.get_tile(5, 3);  // returns 1
```

Tile IDs map to positions in the tileset texture: tile 1 is the first tile (column 0, row 0), tile 2 is the second (column 1, row 0), and so on, wrapping at `columns`.

## Multiple layers

```rust
// The map starts with one layer (index 0)
// Add more layers for foreground/decoration:
let fg_layer = map.add_layer();   // returns 1
let decor_layer = map.add_layer(); // returns 2

// Set tiles on specific layers
map.set_tile_layer(fg_layer, 5, 3, 42);
map.set_tile_layer(decor_layer, 5, 3, 99);

// Get tile from a layer
let id = map.get_tile_layer(fg_layer, 5, 3);

// Check layer count
let n = map.layer_count();  // 3
```

Layers are drawn back-to-front (layer 0 first, higher layers on top).

## Animated tiles

Register animated tiles that cycle through frames automatically:

```rust
// When tile ID 7 is encountered, cycle through frames 7, 8, 9, 10
map.add_animated_tile(7, AnimatedTile {
    frames: vec![7, 8, 9, 10],
    frame_duration: 0.2,        // seconds per frame
});

// Water animation
map.add_animated_tile(20, AnimatedTile {
    frames: vec![20, 21, 22, 23],
    frame_duration: 0.15,
});
```

You must call `map.update(dt)` each frame to advance animations:

```rust
map.update(dt);     // dt = frame delta time in seconds
```

## Tile flip flags

Flip flags are packed into the upper bits of tile IDs:

```rust
use lite_render_2d_core::tilemap::{TILE_FLIP_H, TILE_FLIP_V, TILE_ID_MASK};

// Set a horizontally flipped tile
map.set_tile(5, 3, 1 | TILE_FLIP_H);

// Set a vertically flipped tile
map.set_tile(6, 3, 2 | TILE_FLIP_V);

// Both flipped
map.set_tile(7, 3, 3 | TILE_FLIP_H | TILE_FLIP_V);

// Check flags on a raw tile ID
let raw = map.get_tile(5, 3);
let is_flipped_h = Tilemap::tile_flip_h(raw);  // true
let is_flipped_v = Tilemap::tile_flip_v(raw);  // false
let base_id = raw & TILE_ID_MASK;              // 1
```

Constants:
- `TILE_FLIP_H = 0x8000` — horizontal flip flag
- `TILE_FLIP_V = 0x4000` — vertical flip flag
- `TILE_ID_MASK = 0x3FFF` — mask to extract the base tile ID

## Tilemap projection

### Orthogonal (default)

Standard top-down grid:

```rust
map.projection = TilemapProjection::Orthogonal;
```

### Isometric

Diamond-shaped isometric grid:

```rust
use lite_render_2d_core::tilemap::TilemapProjection;

map.projection = TilemapProjection::Isometric;
```

In isometric mode, `grid_to_world` converts grid coordinates to the diamond layout automatically.

### Grid to world conversion

```rust
let world_pos = map.grid_to_world(col, row, Vec2::new(offset_x, offset_y));
```

This converts grid coordinates (col, row) to world-space position, accounting for the current projection and an offset.

## Drawing the tilemap

```rust
renderer.draw_tilemap(&map, Vec2::new(0.0, 0.0), 0);
// position: world-space offset for the entire tilemap
// z_index: draw order (layers are drawn at z_index, z_index+1, z_index+2, etc.)
```

### Automatic frustum culling

The renderer only draws tiles that are visible within the current camera viewport. Off-screen tiles are skipped entirely. You don't need to do anything to enable this — it's automatic.

To verify culling is working, check `FrameStats.draw_calls` — it should stay roughly constant regardless of map size, as long as the visible area is the same.

## Utility methods

```rust
// Resolve animated tile ID for the current time
let resolved = map.resolve_tile_id(raw_id);

// Get pixel-space source rect for a tile ID in the tileset
let src_rect = map.tile_src_rect(tile_id);
```

## Complete example

```rust
// Setup
let tileset_tex = renderer.load_texture_from_file("tileset.png".as_ref(), TextureParams::nearest())?;
let tileset = TilesetInfo { tile_width: 16.0, tile_height: 16.0, columns: 16 };
let mut map = Tilemap::new(50, 50, 16.0, tileset_tex, tileset);

// Fill with grass (tile 1), place some water (tile 7)
for y in 0..50 {
    for x in 0..50 {
        map.set_tile(x, y, 1);
    }
}
map.set_tile(10, 10, 7);
map.set_tile(11, 10, 7);

// Animate water
map.add_animated_tile(7, AnimatedTile {
    frames: vec![7, 8, 9, 10],
    frame_duration: 0.2,
});

// Add foreground layer with a tree
let fg = map.add_layer();
map.set_tile_layer(fg, 15, 15, 50);

// Game loop
map.update(dt);
renderer.draw_tilemap(&map, Vec2::ZERO, 0);
```
