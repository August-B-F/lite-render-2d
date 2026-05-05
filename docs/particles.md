# Particles & Trails

## Particle system

The particle system manages multiple emitters, spawning and updating particles each frame.

### Setup

```rust
use lite_render_2d_core::particle::{ParticleSystem, ParticleConfig};

let mut particles = ParticleSystem::new();
```

### Adding emitters

```rust
let fire_emitter = particles.add_emitter(
    ParticleConfig {
        spawn_rate: 50.0,                                       // 50 particles per second
        lifetime: (0.5, 1.5),                                   // each particle lives 0.5-1.5 seconds
        velocity: (Vec2::new(-20.0, -80.0), Vec2::new(20.0, -30.0)),  // upward spread
        size: (6.0, 1.0),                                       // starts at 6px, shrinks to 1px
        color_start: Color::YELLOW,
        color_end: Color::new(1.0, 0.0, 0.0, 0.0),            // fades to transparent red
        gravity: Vec2::new(0.0, -20.0),                         // slight upward pull for fire
        texture: None,                                           // None = circles, Some(tex) = sprites
    },
    Vec2::new(400.0, 500.0),    // emitter position
);
// Returns emitter index (usize)
```

### ParticleConfig fields

| Field | Type | Description |
|-------|------|-------------|
| `spawn_rate` | `f32` | Particles per second |
| `lifetime` | `(f32, f32)` | (min, max) seconds each particle lives |
| `velocity` | `(Vec2, Vec2)` | (min, max) initial velocity — randomized per particle |
| `size` | `(f32, f32)` | (start, end) size in pixels — interpolated over lifetime |
| `color_start` | `Color` | Color at birth |
| `color_end` | `Color` | Color at death — interpolated over lifetime |
| `gravity` | `Vec2` | Acceleration applied each frame |
| `texture` | `Option<TextureHandle>` | None = draw circles, Some = draw sprites |

Default values: spawn_rate=10, lifetime=(1.0, 2.0), velocity=((-20, -50), (20, -10)), size=(4, 1), gravity=(0, 98).

### Update and draw

Call these every frame:

```rust
particles.update(dt);               // spawn new particles, move existing, remove dead
particles.draw(&mut renderer);      // render all alive particles
```

### Moving emitters

```rust
particles.set_emitter_position(fire_emitter, new_position);
```

### Removing emitters

```rust
particles.remove_emitter(fire_emitter);
// Note: existing particles from this emitter continue until they die
```

### Querying

```rust
let count = particles.particle_count();  // total alive particles across all emitters
```

### Textured particles

Pass a texture handle to draw sprites instead of circles:

```rust
let smoke_tex = renderer.load_texture_from_file("smoke.png".as_ref(), TextureParams::default())?;

particles.add_emitter(ParticleConfig {
    texture: Some(smoke_tex),
    spawn_rate: 20.0,
    lifetime: (1.0, 3.0),
    velocity: (Vec2::new(-10.0, -40.0), Vec2::new(10.0, -20.0)),
    size: (8.0, 24.0),          // grows over lifetime
    color_start: Color::new(0.8, 0.8, 0.8, 0.8),
    color_end: Color::TRANSPARENT,
    gravity: Vec2::new(0.0, -5.0),
    ..ParticleConfig::default()
}, position);
```

## Trail rendering

Trails draw a fading ribbon behind a moving object.

### Setup

```rust
use lite_render_2d_core::trail::TrailRenderer;

let mut trail = TrailRenderer::new(
    100,    // max points in the trail
    4.0,    // ribbon width in pixels
    1.0,    // lifetime in seconds (points fade and die)
);

// Customize colors
trail.color_start = Color::WHITE;
trail.color_end = Color::TRANSPARENT;

// Optional: use a texture instead of solid color
trail.texture = Some(trail_tex);
```

### Update loop

Call these every frame:

```rust
// Add the current position of the moving object
trail.add_point(object_position);

// Age points and remove expired ones
trail.update(dt);

// Draw the trail
trail.draw(&mut renderer);
```

### How it works

- `add_point` adds a new point at the object's current position. Points too close together (< 1 pixel) are skipped to avoid degenerate geometry.
- `update` ages all points by `dt` seconds. Points older than `lifetime` are removed.
- `draw` renders the trail as a ribbon of quads connecting consecutive points. The ribbon width tapers from `width` at age 0 to 0 at age = lifetime. Color interpolates from `color_start` to `color_end`.

### TrailRenderer fields

| Field | Type | Description |
|-------|------|-------------|
| `points` | `Vec<TrailPoint>` | Current trail points |
| `max_points` | `usize` | Maximum points (oldest evicted when exceeded) |
| `width` | `f32` | Ribbon width at age 0 |
| `lifetime` | `f32` | Seconds before a point expires |
| `color_start` | `Color` | Color at age 0 |
| `color_end` | `Color` | Color at age = lifetime |
| `texture` | `Option<TextureHandle>` | Optional texture for the ribbon |

### Querying

```rust
let n = trail.point_count();  // current number of points
```

## Example: fire with trail

```rust
let mut particles = ParticleSystem::new();
particles.add_emitter(ParticleConfig {
    spawn_rate: 80.0,
    lifetime: (0.3, 0.8),
    velocity: (Vec2::new(-15.0, -60.0), Vec2::new(15.0, -30.0)),
    size: (5.0, 1.0),
    color_start: Color::YELLOW,
    color_end: Color::new(1.0, 0.0, 0.0, 0.0),
    gravity: Vec2::new(0.0, -50.0),
    texture: None,
}, torch_position);

let mut trail = TrailRenderer::new(50, 3.0, 0.5);
trail.color_start = Color::new(1.0, 0.5, 0.0, 0.8);
trail.color_end = Color::TRANSPARENT;

// Each frame:
particles.update(dt);
trail.add_point(moving_fire_position);
trail.update(dt);

renderer.begin_frame()?;
trail.draw(&mut renderer);
particles.draw(&mut renderer);
renderer.end_frame()?;
```
