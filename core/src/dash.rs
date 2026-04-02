use crate::types::{Path, PathSegment, StrokeStyle, Vec2};

// walk along a polyline and split it into sub-segments based on dash/dot pattern
pub fn dash_polyline(points: &[Vec2], style: &StrokeStyle, thickness: f32) -> Vec<Vec<Vec2>> {
    match style {
        StrokeStyle::Solid => {
            vec![points.to_vec()]
        }
        StrokeStyle::Dashed { dash_len, gap_len } => {
            split_by_pattern(points, *dash_len, *gap_len)
        }
        StrokeStyle::Dotted { spacing } => {
            // dots are very short dashes (thickness-length) with the given spacing
            let dot_len = thickness.max(1.0);
            split_by_pattern(points, dot_len, *spacing)
        }
    }
}

// flatten a bezier path to a polyline then apply dashing
pub fn dash_path(path: &Path, style: &StrokeStyle, thickness: f32, tolerance: f32) -> Vec<Vec<Vec2>> {
    let flattened = flatten_path(path, tolerance);
    if flattened.is_empty() {
        return Vec::new();
    }
    dash_polyline(&flattened, style, thickness)
}

fn split_by_pattern(points: &[Vec2], dash_len: f32, gap_len: f32) -> Vec<Vec<Vec2>> {
    if points.len() < 2 || dash_len <= 0.0 {
        return vec![points.to_vec()];
    }

    let cycle = dash_len + gap_len;
    if cycle <= 0.0 {
        return vec![points.to_vec()];
    }

    let mut result: Vec<Vec<Vec2>> = Vec::new();
    let mut current_seg: Vec<Vec2> = Vec::new();
    let mut pattern_offset = 0.0_f32; // how far into the current dash/gap cycle
    let mut in_dash = true;

    current_seg.push(points[0]);

    for i in 0..points.len() - 1 {
        let a = points[i];
        let b = points[i + 1];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len < 1e-6 {
            continue;
        }

        let mut consumed = 0.0_f32;

        while consumed < seg_len - 1e-6 {
            let remaining_in_phase = if in_dash {
                dash_len - pattern_offset
            } else {
                gap_len - pattern_offset
            };

            let remaining_in_seg = seg_len - consumed;
            let advance = remaining_in_phase.min(remaining_in_seg);

            consumed += advance;
            pattern_offset += advance;

            let t = consumed / seg_len;
            let pt = Vec2::new(a.x + dx * t, a.y + dy * t);

            if in_dash {
                current_seg.push(pt);
            }

            // check if we completed the current phase
            let phase_limit = if in_dash { dash_len } else { gap_len };
            if pattern_offset >= phase_limit - 1e-6 {
                if in_dash && current_seg.len() >= 2 {
                    result.push(std::mem::take(&mut current_seg));
                } else if !in_dash {
                    // starting a new dash, begin fresh segment
                    current_seg.clear();
                    current_seg.push(pt);
                }
                in_dash = !in_dash;
                pattern_offset = 0.0;
            }
        }
    }

    // flush remaining dash segment
    if in_dash && current_seg.len() >= 2 {
        result.push(current_seg);
    }

    result
}

// flatten path curves into a polyline by recursive subdivision
fn flatten_path(path: &Path, tolerance: f32) -> Vec<Vec2> {
    let mut out = Vec::new();
    let mut current = Vec2::ZERO;

    for seg in &path.segments {
        match *seg {
            PathSegment::MoveTo(p) => {
                current = p;
                out.push(p);
            }
            PathSegment::LineTo(p) => {
                current = p;
                out.push(p);
            }
            PathSegment::QuadTo { ctrl, to } => {
                flatten_quad(&mut out, current, ctrl, to, tolerance);
                current = to;
            }
            PathSegment::CubicTo { ctrl1, ctrl2, to } => {
                flatten_cubic(&mut out, current, ctrl1, ctrl2, to, tolerance);
                current = to;
            }
            PathSegment::Close => {
                if let Some(&first) = out.first() {
                    if (current.x - first.x).abs() > 1e-6 || (current.y - first.y).abs() > 1e-6 {
                        out.push(first);
                        current = first;
                    }
                }
            }
        }
    }

    out
}

fn flatten_quad(out: &mut Vec<Vec2>, p0: Vec2, p1: Vec2, p2: Vec2, tol: f32) {
    // check if flat enough
    let mid_x = (p0.x + p2.x) * 0.5;
    let mid_y = (p0.y + p2.y) * 0.5;
    let dx = p1.x - mid_x;
    let dy = p1.y - mid_y;
    if dx * dx + dy * dy <= tol * tol {
        out.push(p2);
        return;
    }
    let q0 = Vec2::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
    let q1 = Vec2::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
    let r = Vec2::new((q0.x + q1.x) * 0.5, (q0.y + q1.y) * 0.5);
    flatten_quad(out, p0, q0, r, tol);
    flatten_quad(out, r, q1, p2, tol);
}

fn flatten_cubic(out: &mut Vec<Vec2>, p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, tol: f32) {
    let d1x = p1.x - (p0.x * 2.0 + p3.x) / 3.0;
    let d1y = p1.y - (p0.y * 2.0 + p3.y) / 3.0;
    let d2x = p2.x - (p0.x + p3.x * 2.0) / 3.0;
    let d2y = p2.y - (p0.y + p3.y * 2.0) / 3.0;
    let d = (d1x * d1x + d1y * d1y).max(d2x * d2x + d2y * d2y);
    if d <= tol * tol {
        out.push(p3);
        return;
    }
    let q0 = Vec2::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
    let q1 = Vec2::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
    let q2 = Vec2::new((p2.x + p3.x) * 0.5, (p2.y + p3.y) * 0.5);
    let r0 = Vec2::new((q0.x + q1.x) * 0.5, (q0.y + q1.y) * 0.5);
    let r1 = Vec2::new((q1.x + q2.x) * 0.5, (q1.y + q2.y) * 0.5);
    let s = Vec2::new((r0.x + r1.x) * 0.5, (r0.y + r1.y) * 0.5);
    flatten_cubic(out, p0, q0, r0, s, tol);
    flatten_cubic(out, s, r1, q2, p3, tol);
}
