# Text rendering

lite-render-2d supports three text rendering approaches: TTF/OTF fonts (via fontdue), SDF fonts (crisp at any scale), and bitmap fonts (grid-based sprite sheets).

## Loading fonts

### TTF/OTF fonts

`load_font` takes raw font bytes, not a file path. Read the file yourself:

```rust
let font_data = std::fs::read("assets/font.ttf").expect("read font file");
let font = renderer.load_font(&font_data)?;
```

Or embed at compile time:

```rust
let font = renderer.load_font(include_bytes!("../assets/font.ttf"))?;
```

### Unloading fonts

```rust
renderer.unload_font(font);
```

`FontHandle` is `Copy` and cheap to pass around.

## Drawing text

```rust
renderer.draw_text(text: &str, params: &TextParams)
```

All text parameters are bundled into `TextParams`:

```rust
renderer.draw_text("Hello, world!", &TextParams {
    font,                               // FontHandle from load_font
    size: 24.0,                         // font size in pixels
    color: Color::WHITE,                // text color
    align: TextAlign::Left,             // Left, Center, or Right
    position: Vec2::new(10.0, 10.0),    // top-left of text
    max_width: None,                    // None = no wrap
    line_height: None,                  // None = defaults to font size
});
```

### TextParams fields

| Field | Type | Description |
|-------|------|-------------|
| `font` | `FontHandle` | Font to use |
| `size` | `f32` | Font size in pixels |
| `color` | `Color` | Text color |
| `align` | `TextAlign` | Horizontal alignment |
| `position` | `Vec2` | Position of text origin |
| `max_width` | `Option<f32>` | Word wrap width (None = no wrapping) |
| `line_height` | `Option<f32>` | Line spacing (None = defaults to font size) |

## Text alignment

```rust
TextAlign::Left     // text starts at position.x (default)
TextAlign::Center   // text is centered around position.x
TextAlign::Right    // text ends at position.x
```

Alignment is relative to `max_width` when set, otherwise relative to the text's own width.

## Multiline text

Newlines (`\n`) in the text string create line breaks:

```rust
renderer.draw_text("Line one\nLine two\nLine three", &TextParams {
    font,
    size: 20.0,
    color: Color::WHITE,
    align: TextAlign::Left,
    position: Vec2::new(10.0, 10.0),
    max_width: None,
    line_height: Some(24.0),  // custom line spacing
});
```

## Word wrapping

Set `max_width` to automatically wrap text at word boundaries:

```rust
renderer.draw_text(
    "This is a long paragraph that will automatically wrap at word boundaries when it exceeds the maximum width.",
    &TextParams {
        font,
        size: 16.0,
        color: Color::WHITE,
        align: TextAlign::Left,
        position: Vec2::new(10.0, 10.0),
        max_width: Some(300.0),     // wrap at 300 pixels
        line_height: None,
    },
);
```

## Measuring text

Get the bounding box of text without drawing it:

```rust
let size: Vec2 = renderer.measure_text("Hello, world!", &TextParams {
    font,
    size: 24.0,
    color: Color::WHITE,
    align: TextAlign::Left,
    position: Vec2::ZERO,       // position doesn't affect measurement
    max_width: None,
    line_height: None,
});

// size.x = total width in pixels
// size.y = total height in pixels
```

### Centering text on screen

```rust
let text = "Game Over";
let params = TextParams {
    font,
    size: 48.0,
    color: Color::RED,
    align: TextAlign::Left,
    position: Vec2::ZERO,
    max_width: None,
    line_height: None,
};
let size = renderer.measure_text(text, &params);

renderer.draw_text(text, &TextParams {
    position: Vec2::new(
        (window_w - size.x) / 2.0,
        (window_h - size.y) / 2.0,
    ),
    ..params
});
```

## SDF fonts [feature: "text"]

SDF (Signed Distance Field) fonts render text that stays crisp at any size — no blurring when zoomed in. Best for text that changes size or is viewed through a zooming camera.

```rust
let sdf_font = renderer.load_sdf_font(&font_data)?;

renderer.draw_sdf_text("Crisp at any zoom!", &TextParams {
    font: sdf_font,
    size: 48.0,
    color: Color::WHITE,
    align: TextAlign::Left,
    position: Vec2::new(10.0, 10.0),
    max_width: None,
    line_height: None,
});

let size = renderer.measure_sdf_text("Crisp!", &TextParams {
    font: sdf_font,
    size: 48.0,
    color: Color::WHITE,
    align: TextAlign::Left,
    position: Vec2::ZERO,
    max_width: None,
    line_height: None,
});

renderer.unload_sdf_font(sdf_font);
```

SDF fonts are rendered at a base size of 64px with 6px SDF spread internally, then scaled to the requested size. This means they stay sharp even at very large sizes.

## Rich text [feature: "text"]

Rich text lets you mix fonts, sizes, colors, and styles within a single text block:

```rust
use lite_render_2d_core::{RichText, RichTextSpan};

renderer.draw_rich_text(&RichText {
    spans: vec![
        RichTextSpan {
            text: "Bold red ".into(),
            font,
            size: 24.0,
            color: Color::RED,
            bold: true,
            italic: false,
        },
        RichTextSpan {
            text: "then normal white ".into(),
            font,
            size: 24.0,
            color: Color::WHITE,
            bold: false,
            italic: false,
        },
        RichTextSpan {
            text: "then small blue".into(),
            font,
            size: 16.0,
            color: Color::BLUE,
            bold: false,
            italic: false,
        },
    ],
    align: TextAlign::Left,
    max_width: Some(400.0),    // word wraps at 400px
    line_height: None,
    position: Vec2::new(10.0, 10.0),
});

// Measure rich text bounding box:
let size = renderer.measure_rich_text(&rich_text);
```

### RichTextSpan fields

| Field | Type | Description |
|-------|------|-------------|
| `text` | `String` | The text content |
| `font` | `FontHandle` | Font for this span |
| `size` | `f32` | Font size for this span |
| `color` | `Color` | Color for this span |
| `bold` | `bool` | Bold flag (font must support it) |
| `italic` | `bool` | Italic flag (font must support it) |

## Bitmap fonts

Grid-based bitmap fonts use a sprite sheet where characters are laid out in a grid:

```rust
use lite_render_2d_core::{BitmapFont, BitmapGlyph};

// Create from a grid texture:
// cell_w=8, cell_h=16, 16 columns, starting at ASCII 32 (space), 96 chars
let bfont = BitmapFont::from_grid(font_texture, 8.0, 16.0, 16, 32, 96);

// Measure text:
let size = bfont.measure("Hello World");

// Layout into quads for manual rendering:
let quads = bfont.layout("Hello World", Vec2::new(10.0, 10.0), Color::WHITE);
for quad in &quads {
    renderer.draw_sprite(bfont.texture, SpriteParams::new(
        Transform2D::new(quad.pos.x, quad.pos.y)
    ).with_src_rect(quad.src_rect));
}
```

### Custom glyph metrics

For variable-width bitmap fonts, set individual glyph metrics:

```rust
bfont.set_glyph('W', BitmapGlyph {
    src_rect: Rect::new(0.0, 0.0, 12.0, 16.0),
    offset: Vec2::ZERO,
    advance: 12.0,     // wider than other characters
});
```

### BitmapGlyph fields

| Field | Type | Description |
|-------|------|-------------|
| `src_rect` | `Rect` | Source rectangle in the texture |
| `offset` | `Vec2` | Offset from cursor position |
| `advance` | `f32` | How far to move cursor after this glyph |
