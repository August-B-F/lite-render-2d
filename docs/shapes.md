# Drawing shapes

All shape methods take a `DrawParams` which controls fill/stroke style, blend mode, z-ordering, and opacity. Draw calls can happen in any order between `begin_frame()` and `end_frame()` — the renderer sorts them.

## DrawParams

The parameter struct for all shape draw calls:

```rust
// Quick constructors:
DrawParams::fill(Color::RED)                        // solid fill
DrawParams::stroke(Color::WHITE, 2.0)               // solid stroke, 2px thick

// Builder chain:
DrawParams::fill(Color::RED)
    .with_z(5)                                      // draw order (higher = on top)
    .with_opacity(0.5)                              // 0.0-1.0
    .with_blend(BlendMode::Additive)                // compositing mode
```

Under the hood, `DrawParams` wraps a `DrawStyle` plus blend/z/opacity:

```rust
DrawParams {
    style: DrawStyle,       // Fill, Stroke, or Gradient
    blend: BlendMode,       // Alpha (default), Additive, Multiply, Screen, PremultipliedAlpha
    z_index: i32,           // draw order
    opacity: f32,           // 0.0-1.0
}
```

## Rectangles

```rust
renderer.draw_rect(rect: Rect, params: DrawParams)
```

- `rect` — position and size. Use `Rect::new(x, y, w, h)` for top-left origin or `Rect::from_center(cx, cy, w, h)` for centered.
- `params` — fill, stroke, or gradient.

```rust
// Filled red rectangle
renderer.draw_rect(Rect::new(10.0, 10.0, 200.0, 100.0), DrawParams::fill(Color::RED));

// Stroked white rectangle, 2px border
renderer.draw_rect(Rect::new(10.0, 10.0, 200.0, 100.0), DrawParams::stroke(Color::WHITE, 2.0));

// Semi-transparent
renderer.draw_rect(Rect::new(10.0, 10.0, 200.0, 100.0), DrawParams::fill(Color::BLUE).with_opacity(0.5));

// Centered rectangle
renderer.draw_rect(Rect::from_center(400.0, 300.0, 200.0, 100.0), DrawParams::fill(Color::GREEN));
```

Edge cases: zero-size rects draw nothing. Negative width/height are treated as-is (may produce inverted geometry).

## Rounded rectangles

```rust
renderer.draw_rounded_rect(rrect: RoundedRect, params: DrawParams)
```

- `rrect` — rect plus corner radii. Uniform or per-corner.

```rust
// Uniform radius
renderer.draw_rounded_rect(
    RoundedRect::new(Rect::new(10.0, 10.0, 200.0, 100.0), 15.0),
    DrawParams::fill(Color::CYAN),
);

// Per-corner radii: top-left, top-right, bottom-left, bottom-right
renderer.draw_rounded_rect(
    RoundedRect::with_radii(Rect::new(10.0, 10.0, 200.0, 100.0), 5.0, 10.0, 15.0, 20.0),
    DrawParams::fill(Color::YELLOW),
);

// Stroked rounded rect
renderer.draw_rounded_rect(
    RoundedRect::new(Rect::new(10.0, 10.0, 200.0, 100.0), 10.0),
    DrawParams::stroke(Color::WHITE, 2.0),
);
```

If a radius exceeds half the rect dimension, it's clamped internally to avoid artifacts.

## Circles

```rust
renderer.draw_circle(center: Vec2, radius: f32, params: DrawParams)
```

```rust
// Filled circle
renderer.draw_circle(Vec2::new(200.0, 200.0), 50.0, DrawParams::fill(Color::BLUE));

// Stroked circle
renderer.draw_circle(Vec2::new(200.0, 200.0), 50.0, DrawParams::stroke(Color::WHITE, 2.0));
```

Zero or negative radius draws nothing.

## Ellipses

```rust
renderer.draw_ellipse(center: Vec2, radii: Vec2, params: DrawParams)
```

- `radii` — `Vec2(rx, ry)` where rx is horizontal radius, ry is vertical radius.

```rust
renderer.draw_ellipse(
    Vec2::new(300.0, 200.0),
    Vec2::new(80.0, 40.0),     // wider than tall
    DrawParams::fill(Color::MAGENTA),
);
```

## Arcs

```rust
renderer.draw_arc(center: Vec2, radius: f32, start_angle: f32, end_angle: f32, params: DrawParams)
```

- Angles are in **radians**. 0 is rightward (3 o'clock), increasing clockwise.
- Draws a pie/wedge shape when filled, or an arc segment when stroked.

```rust
use std::f32::consts::PI;

// Quarter circle (top-right quadrant)
renderer.draw_arc(
    Vec2::new(200.0, 200.0),
    60.0,
    0.0,            // start: 3 o'clock
    PI * 0.5,       // end: 6 o'clock
    DrawParams::fill(Color::YELLOW),
);

// Stroked arc
renderer.draw_arc(
    Vec2::new(200.0, 200.0),
    60.0,
    0.0,
    PI,
    DrawParams::stroke(Color::WHITE, 2.0),
);
```

## Triangles

```rust
renderer.draw_triangle(a: Vec2, b: Vec2, c: Vec2, params: DrawParams)
```

```rust
renderer.draw_triangle(
    Vec2::new(100.0, 50.0),
    Vec2::new(50.0, 150.0),
    Vec2::new(150.0, 150.0),
    DrawParams::fill(Color::GREEN),
);
```

## Polygons

```rust
renderer.draw_polygon(points: &[Vec2], params: DrawParams)
```

For **convex** polygons. Points should be in winding order (clockwise or counter-clockwise).

```rust
// Pentagon
let points = [
    Vec2::new(200.0, 100.0),
    Vec2::new(250.0, 150.0),
    Vec2::new(230.0, 210.0),
    Vec2::new(170.0, 210.0),
    Vec2::new(150.0, 150.0),
];
renderer.draw_polygon(&points, DrawParams::fill(Color::CYAN));
```

### Complex polygons (concave, with holes)

```rust
renderer.draw_complex_polygon(outer: &[Vec2], holes: &[&[Vec2]], params: DrawParams)
```

Uses tessellation to handle concave shapes and holes. Requires the `paths` feature.

```rust
// Concave polygon with a hole
let outer = [
    Vec2::new(100.0, 100.0), Vec2::new(300.0, 100.0),
    Vec2::new(300.0, 300.0), Vec2::new(200.0, 200.0),
    Vec2::new(100.0, 300.0),
];
let hole = [
    Vec2::new(180.0, 150.0), Vec2::new(220.0, 150.0),
    Vec2::new(220.0, 190.0), Vec2::new(180.0, 190.0),
];
renderer.draw_complex_polygon(&outer, &[&hole], DrawParams::fill(Color::RED));
```

## Lines

```rust
renderer.draw_line(from: Vec2, to: Vec2, params: LineParams)
```

```rust
// Basic line
renderer.draw_line(
    Vec2::new(10.0, 10.0),
    Vec2::new(200.0, 150.0),
    LineParams::new(Color::WHITE, 2.0),
);
```

### LineParams

```rust
LineParams {
    color: Color,
    thickness: f32,
    cap: LineCap,       // Butt (default), Round, Square
    join: LineJoin,      // Miter (default), Round, Bevel
    style: StrokeStyle,  // Solid (default), Dashed, Dotted
    blend: BlendMode,
    z_index: i32,
    opacity: f32,
}
```

### Line caps

- **Butt** — line ends exactly at the endpoint (default)
- **Round** — semicircle cap at each end
- **Square** — extends half-thickness beyond endpoint

### Line joins

- **Miter** — sharp corner (default)
- **Round** — rounded corner
- **Bevel** — flat corner

### Dashed and dotted lines

```rust
// Dashed
renderer.draw_line(from, to, LineParams {
    color: Color::WHITE,
    thickness: 2.0,
    style: StrokeStyle::Dashed { dash_len: 10.0, gap_len: 5.0 },
    cap: LineCap::Butt,
    join: LineJoin::Miter,
    blend: BlendMode::Alpha,
    z_index: 0,
    opacity: 1.0,
});

// Dotted
renderer.draw_line(from, to, LineParams {
    color: Color::YELLOW,
    thickness: 3.0,
    style: StrokeStyle::Dotted { spacing: 8.0 },
    ..LineParams::new(Color::YELLOW, 3.0)
});
```

## Polylines

```rust
renderer.draw_polyline(points: &[Vec2], params: LineParams)
```

Connected line strip through all points. Same `LineParams` as single lines.

```rust
let path_points = [
    Vec2::new(50.0, 200.0),
    Vec2::new(100.0, 100.0),
    Vec2::new(200.0, 250.0),
    Vec2::new(300.0, 150.0),
];
renderer.draw_polyline(&path_points, LineParams::new(Color::GREEN, 3.0));
```

## Bezier paths

Build paths with the fluent `Path` API, then fill or stroke them. Requires the `paths` feature.

```rust
// Build a path
let path = Path::new()
    .move_to(Vec2::new(50.0, 200.0))
    .cubic_to(
        Vec2::new(100.0, 50.0),    // control point 1
        Vec2::new(200.0, 350.0),   // control point 2
        Vec2::new(250.0, 200.0),   // end point
    )
    .close();

// Fill path
renderer.draw_path(&path, DrawParams::fill(Color::RED));

// Stroke path
renderer.stroke_path(&path, StrokeParams::new(Color::WHITE, 2.0));
```

### Path segments

- `move_to(p)` — move pen without drawing
- `line_to(p)` — straight line to point
- `quad_to(ctrl, to)` — quadratic bezier curve
- `cubic_to(ctrl1, ctrl2, to)` — cubic bezier curve
- `close()` — close the path back to the last `move_to`

### StrokeParams

For `stroke_path`, you use `StrokeParams` instead of `DrawParams`:

```rust
StrokeParams {
    color: Color,
    thickness: f32,
    style: StrokeStyle,   // Solid, Dashed, Dotted
    cap: LineCap,         // Butt, Round, Square
    join: LineJoin,        // Miter, Round, Bevel
}

// Quick constructor
StrokeParams::new(Color::WHITE, 2.0)  // Solid, Butt cap, Miter join
```

## Gradients

Any shape can be filled with a gradient by using `DrawStyle` variants inside `DrawParams`:

### Linear gradient (two colors)

```rust
renderer.draw_rect(Rect::new(10.0, 10.0, 200.0, 100.0), DrawParams {
    style: DrawStyle::LinearGradient {
        start: Vec2::new(10.0, 10.0),
        end: Vec2::new(210.0, 10.0),
        color_start: Color::RED,
        color_end: Color::BLUE,
    },
    blend: BlendMode::Alpha,
    z_index: 0,
    opacity: 1.0,
});
```

### Radial gradient (two colors)

```rust
renderer.draw_circle(Vec2::new(200.0, 200.0), 80.0, DrawParams {
    style: DrawStyle::RadialGradient {
        center: Vec2::new(200.0, 200.0),
        radius: 80.0,
        color_inner: Color::WHITE,
        color_outer: Color::BLUE,
    },
    blend: BlendMode::Alpha,
    z_index: 0,
    opacity: 1.0,
});
```

### Multi-stop gradients

```rust
renderer.draw_rect(rect, DrawParams {
    style: DrawStyle::LinearGradientStops {
        start: Vec2::new(0.0, 0.0),
        end: Vec2::new(300.0, 0.0),
        stops: vec![
            GradientStop { offset: 0.0, color: Color::RED },
            GradientStop { offset: 0.5, color: Color::YELLOW },
            GradientStop { offset: 1.0, color: Color::GREEN },
        ],
    },
    blend: BlendMode::Alpha,
    z_index: 0,
    opacity: 1.0,
});

// Radial multi-stop:
DrawStyle::RadialGradientStops {
    center: Vec2::new(200.0, 200.0),
    radius: 100.0,
    stops: vec![
        GradientStop { offset: 0.0, color: Color::WHITE },
        GradientStop { offset: 0.7, color: Color::CYAN },
        GradientStop { offset: 1.0, color: Color::TRANSPARENT },
    ],
}
```

`GradientStop.offset` ranges from 0.0 (start) to 1.0 (end).
