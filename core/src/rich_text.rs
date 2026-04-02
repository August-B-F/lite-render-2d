use crate::font_atlas::{FontSystem, GlyphQuad};
use crate::text::{FontHandle, TextAlign};
use crate::types::{Color, Vec2};

pub struct RichTextSpan {
    pub text: String,
    pub font: FontHandle,
    pub size: f32,
    pub color: Color,
    pub bold: bool,
    pub italic: bool,
}

pub struct RichText {
    pub spans: Vec<RichTextSpan>,
    pub align: TextAlign,
    pub max_width: Option<f32>,
    pub line_height: Option<f32>,
    pub position: Vec2,
}

// internal: a single char with its style info
struct StyledChar {
    ch: char,
    font_id: u64,
    size: f32,
    color: Color,
}

// layout rich text into glyph quads using the font system
pub fn layout_rich_text(rich: &RichText, font_sys: &mut FontSystem) -> Vec<GlyphQuad> {
    if rich.spans.is_empty() {
        return Vec::new();
    }

    // flatten spans into styled chars
    let mut chars: Vec<StyledChar> = Vec::new();
    for span in &rich.spans {
        let fid = span.font.id();
        for ch in span.text.chars() {
            chars.push(StyledChar {
                ch,
                font_id: fid,
                size: span.size,
                color: span.color,
            });
        }
    }

    if chars.is_empty() {
        return Vec::new();
    }

    // ensure all glyphs rasterized
    for sc in &chars {
        font_sys.ensure_glyph_pub(sc.font_id, sc.ch, sc.size);
    }

    // figure out default line height from first span
    let default_lh = rich.line_height.unwrap_or(rich.spans[0].size);
    let max_w = rich.max_width.unwrap_or(f32::MAX);

    // break into lines
    let mut lines: Vec<Vec<usize>> = vec![vec![]]; // indices into chars
    let mut cur_line_w = 0.0f32;
    let mut word_start = 0usize;
    let mut word_w = 0.0f32;
    let mut in_word = false;

    for (i, sc) in chars.iter().enumerate() {
        if sc.ch == '\n' {
            // flush word
            if in_word {
                lines.last_mut().unwrap().extend(word_start..i);
                in_word = false;
            }
            lines.push(vec![]);
            cur_line_w = 0.0;
            word_w = 0.0;
            continue;
        }

        let adv = font_sys.glyph_advance(sc.font_id, sc.ch, sc.size);

        if sc.ch == ' ' {
            // flush word
            if in_word {
                // check if word fits
                if cur_line_w + word_w > max_w && cur_line_w > 0.0 {
                    lines.push(vec![]);
                    cur_line_w = 0.0;
                }
                lines.last_mut().unwrap().extend(word_start..i);
                cur_line_w += word_w;
                word_w = 0.0;
                in_word = false;
            }
            // add space
            lines.last_mut().unwrap().push(i);
            cur_line_w += adv;
        } else {
            if !in_word {
                word_start = i;
                word_w = 0.0;
                in_word = true;
            }
            word_w += adv;
        }
    }

    // flush last word
    if in_word {
        if cur_line_w + word_w > max_w && cur_line_w > 0.0 {
            lines.push(vec![]);
        }
        lines.last_mut().unwrap().extend(word_start..chars.len());
    }

    // emit quads per line with alignment
    let mut quads = Vec::with_capacity(chars.len());

    for (li, line_indices) in lines.iter().enumerate() {
        // measure line width
        let line_w: f32 = line_indices.iter().map(|&i| {
            let sc = &chars[i];
            font_sys.glyph_advance(sc.font_id, sc.ch, sc.size)
        }).sum();

        let align_w = if max_w < f32::MAX { max_w } else { 0.0 };
        let x_off = match rich.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => if align_w > 0.0 { (align_w - line_w) * 0.5 } else { -line_w * 0.5 },
            TextAlign::Right => if align_w > 0.0 { align_w - line_w } else { -line_w },
        };

        let mut cx = rich.position.x + x_off;
        let by = rich.position.y + li as f32 * default_lh;

        for &idx in line_indices {
            let sc = &chars[idx];
            if let Some(q) = font_sys.glyph_quad(sc.font_id, sc.ch, sc.size, cx, by, sc.color) {
                quads.push(q);
            }
            cx += font_sys.glyph_advance(sc.font_id, sc.ch, sc.size);
        }
    }

    quads
}

// measure bounding box of rich text
pub fn measure_rich_text(rich: &RichText, font_sys: &mut FontSystem) -> Vec2 {
    let quads = layout_rich_text(rich, font_sys);
    if quads.is_empty() {
        return Vec2::ZERO;
    }

    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;

    for q in &quads {
        min_x = min_x.min(q.pos.x);
        min_y = min_y.min(q.pos.y);
        max_x = max_x.max(q.pos.x + q.size.x);
        max_y = max_y.max(q.pos.y + q.size.y);
    }

    Vec2::new(max_x - min_x, max_y - min_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_font_sys() -> (FontSystem, FontHandle) {
        let mut fs = FontSystem::new();
        // use embedded test font
        let data = include_bytes!("../../NotoSansMeroitic-Regular.ttf");
        let fh = fs.load_font(data).expect("load font");
        (fs, fh)
    }

    #[test]
    fn test_single_span_produces_quads() {
        let (mut fs, fh) = make_font_sys();
        let rt = RichText {
            spans: vec![RichTextSpan {
                text: "Hello".to_string(),
                font: fh,
                size: 24.0,
                color: Color::WHITE,
                bold: false,
                italic: false,
            }],
            align: TextAlign::Left,
            max_width: None,
            line_height: None,
            position: Vec2::ZERO,
        };
        let quads = layout_rich_text(&rt, &mut fs);
        assert!(!quads.is_empty(), "should produce glyph quads");
    }

    #[test]
    fn test_empty_spans_produce_no_quads() {
        let (mut fs, fh) = make_font_sys();
        let rt = RichText {
            spans: vec![RichTextSpan {
                text: "".to_string(),
                font: fh,
                size: 24.0,
                color: Color::WHITE,
                bold: false,
                italic: false,
            }],
            align: TextAlign::Left,
            max_width: None,
            line_height: None,
            position: Vec2::ZERO,
        };
        let quads = layout_rich_text(&rt, &mut fs);
        assert!(quads.is_empty());
    }

    #[test]
    fn test_multi_span_different_colors() {
        let (mut fs, fh) = make_font_sys();
        let rt = RichText {
            spans: vec![
                RichTextSpan {
                    text: "A".to_string(),
                    font: fh,
                    size: 24.0,
                    color: Color::RED,
                    bold: false,
                    italic: false,
                },
                RichTextSpan {
                    text: "B".to_string(),
                    font: fh,
                    size: 24.0,
                    color: Color::BLUE,
                    bold: false,
                    italic: false,
                },
            ],
            align: TextAlign::Left,
            max_width: None,
            line_height: None,
            position: Vec2::ZERO,
        };
        let quads = layout_rich_text(&rt, &mut fs);
        // should have quads with different colors
        if quads.len() >= 2 {
            // first quad from span1 should be red-ish
            assert_eq!(quads[0].color.r, 1.0);
            assert_eq!(quads[0].color.g, 0.0);
            // second quad from span2 should be blue-ish
            assert_eq!(quads[1].color.b, 1.0);
            assert_eq!(quads[1].color.r, 0.0);
        }
    }

    #[test]
    fn test_newline_creates_multiple_lines() {
        let (mut fs, fh) = make_font_sys();
        let rt = RichText {
            spans: vec![RichTextSpan {
                text: "A\nB".to_string(),
                font: fh,
                size: 24.0,
                color: Color::WHITE,
                bold: false,
                italic: false,
            }],
            align: TextAlign::Left,
            max_width: None,
            line_height: Some(30.0),
            position: Vec2::ZERO,
        };
        let quads = layout_rich_text(&rt, &mut fs);
        if quads.len() >= 2 {
            // second line should be offset by line_height
            let y_diff = quads[1].pos.y - quads[0].pos.y;
            assert!(y_diff.abs() > 10.0, "lines should be vertically separated, diff={}", y_diff);
        }
    }

    #[test]
    fn test_measure_returns_nonzero() {
        let (mut fs, fh) = make_font_sys();
        let rt = RichText {
            spans: vec![RichTextSpan {
                text: "Hello World".to_string(),
                font: fh,
                size: 24.0,
                color: Color::WHITE,
                bold: false,
                italic: false,
            }],
            align: TextAlign::Left,
            max_width: None,
            line_height: None,
            position: Vec2::ZERO,
        };
        let sz = measure_rich_text(&rt, &mut fs);
        assert!(sz.x > 0.0, "width should be > 0");
        assert!(sz.y > 0.0, "height should be > 0");
    }
}
