# Camera

The `Camera2D` controls what portion of the world is visible. It handles panning, zooming, screen-to-world coordinate conversion, smooth following, and screen shake.

## Setup

```rust
let mut camera = Camera2D::new(window_width as f32, window_height as f32);
renderer.set_camera(&camera);
```

By default, the camera is positioned at (0, 0) with zoom 1.0. The viewport covers from (0, 0) to (window_width, window_height) in world space.

### Centered camera

`centered` is identical to `new` — both start at position (0, 0):

```rust
let camera = Camera2D::centered(window_width as f32, window_height as f32);
```

### Builder pattern

```rust
let camera = Camera2D::new(800.0, 600.0)
    .with_position(Vec2::new(400.0, 300.0))
    .with_zoom(2.0);
```

## Panning

Snap the camera to a position immediately:

```rust
camera.look_at(Vec2::new(player_x, player_y));
```

This sets `camera.position` directly. The position is the **center** of the viewport in world space.

## Zooming

```rust
camera = camera.with_zoom(2.0);  // 2x zoom in (things appear bigger)
camera = camera.with_zoom(0.5);  // 2x zoom out (things appear smaller)
```

Zoom affects the visible area: `visible_width = viewport_width / zoom`.

## Screen to world coordinates

Essential for mouse interaction. Converts screen pixel coordinates (e.g., mouse position) to world coordinates:

```rust
let world_pos = camera.screen_to_world(Vec2::new(mouse_x, mouse_y));
```

### Complete example: click to place objects

```rust
WindowEvent::CursorMoved { position, .. } => {
    let screen_pos = Vec2::new(position.x as f32, position.y as f32);
    let world_pos = camera.screen_to_world(screen_pos);
    // world_pos is now in world coordinates
    // Use it to check what was clicked, place objects, etc.
}
```

## World to screen coordinates

The inverse — convert world position to screen pixels:

```rust
let screen_pos = camera.world_to_screen(enemy_position);
// Useful for drawing HUD elements that track world objects
```

## Smooth camera follow

Smoothly move the camera toward a target position. Call every frame:

```rust
// smoothing: higher = faster follow (5.0 is responsive, 1.0 is sluggish)
// dt: frame delta time in seconds
camera.follow(player_position, 5.0, dt);
```

The camera lerps toward the target each frame. At smoothing = 1.0 and dt = 0.016 (60fps), the camera covers about 1.6% of the remaining distance per frame.

## Camera shake

Trigger a screen shake that decays over a duration:

```rust
// intensity: maximum pixel offset
// duration: time in seconds until shake stops
camera.shake(10.0, 0.3);
```

You **must** call `camera.update(dt)` every frame for shake to work:

```rust
// In your game loop:
camera.update(dt);          // step shake decay
renderer.set_camera(&camera);
```

The shake uses a cheap PRNG to generate random offsets that are applied to the camera's projection matrix. The intensity decays linearly over the duration.

## Setting the camera

```rust
renderer.set_camera(&camera);
```

This affects all subsequent draw calls until the next `set_camera` call or the end of the frame. You can switch cameras mid-frame (e.g., world camera for gameplay, then identity camera for HUD):

```rust
renderer.begin_frame()?;

// Draw world with game camera
let game_cam = Camera2D::new(800.0, 600.0).with_position(player_pos).with_zoom(2.0);
renderer.set_camera(&game_cam);
renderer.draw_sprite(world_tex, world_params);

// Draw HUD with default camera (no pan/zoom)
let hud_cam = Camera2D::new(800.0, 600.0);
renderer.set_camera(&hud_cam);
renderer.draw_text("Score: 100", &text_params);

renderer.end_frame()?;
```

## Getting the current camera

```rust
let current = renderer.camera();
// Returns &Camera2D
```

## Camera2D fields

```rust
Camera2D {
    pub position: Vec2,     // center of viewport in world space
    pub zoom: f32,          // 1.0 = normal, 2.0 = zoomed in
    pub viewport: Vec2,     // viewport size in pixels (set from window size)
    // shake state (private)
}
```

## Projection matrix

If you need the raw projection matrix (e.g., for custom shaders):

```rust
let matrix: [f32; 16] = camera.projection_matrix();
```

This returns a column-major 4x4 orthographic projection matrix with y-down coordinates, including any active shake offset.
