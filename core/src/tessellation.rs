use crate::transform_stack::TransformStack;
use crate::types::{Color, DrawStyle, LineParams, RoundedRect, StrokeParams, Vec2};

const FLOATS_PER_VERT: usize = 12;

/// Apply current transform stack to shape vertex positions (every 12 floats, first 2 are x,y)
pub fn apply_transform(verts: &mut [f32], stack: &TransformStack) {
    if stack.is_identity() {
        return;
    }
    for chunk in verts.chunks_exact_mut(FLOATS_PER_VERT) {
        let p = stack.apply(Vec2::new(chunk[0], chunk[1]));
        chunk[0] = p.x;
        chunk[1] = p.y;
    }
}

// compute gradient color for a vertex positon
pub fn linear_gradient_color(
    pos: Vec2,
    start: Vec2,
    end: Vec2,
    color_start: Color,
    color_end: Color,
) -> Color {
    let ax = end.x - start.x;
    let ay = end.y - start.y;
    let dot_aa = ax * ax + ay * ay;
    if dot_aa < 1e-10 {
        return color_start;
    }
    let dx = pos.x - start.x;
    let dy = pos.y - start.y;
    let t = (dx * ax + dy * ay) / dot_aa;
    color_start.lerp(color_end, t)
}

pub fn radial_gradient_color(
    pos: Vec2,
    center: Vec2,
    radius: f32,
    color_inner: Color,
    color_outer: Color,
) -> Color {
    if radius < 1e-6 {
        return color_inner;
    }
    let dx = pos.x - center.x;
    let dy = pos.y - center.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let t = dist / radius;
    color_inner.lerp(color_outer, t)
}

// rewrite per-vertex colors based on gradient style
// no-op for Fill and Stroke
pub fn apply_gradient(verts: &mut [f32], style: &DrawStyle) {
    match *style {
        DrawStyle::LinearGradient { start, end, color_start, color_end } => {
            for chunk in verts.chunks_exact_mut(FLOATS_PER_VERT) {
                let pos = Vec2::new(chunk[0], chunk[1]);
                let c = linear_gradient_color(pos, start, end, color_start, color_end);
                chunk[4] = c.r;
                chunk[5] = c.g;
                chunk[6] = c.b;
                chunk[7] = c.a;
            }
        }
        DrawStyle::RadialGradient { center, radius, color_inner, color_outer } => {
            for chunk in verts.chunks_exact_mut(FLOATS_PER_VERT) {
                let pos = Vec2::new(chunk[0], chunk[1]);
                let c = radial_gradient_color(pos, center, radius, color_inner, color_outer);
                chunk[4] = c.r;
                chunk[5] = c.g;
                chunk[6] = c.b;
                chunk[7] = c.a;
            }
        }
        _ => {}
    }
}

fn solid_vertex(x: f32, y: f32, color: Color) -> [f32; FLOATS_PER_VERT] {
    [x, y, 0.0, 0.0, color.r, color.g, color.b, color.a, 0.0, 0.0, 0.0, 0.0]
}

fn push_tri(out: &mut Vec<f32>, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: Color) {
    out.extend_from_slice(&solid_vertex(a[0], a[1], color));
    out.extend_from_slice(&solid_vertex(b[0], b[1], color));
    out.extend_from_slice(&solid_vertex(c[0], c[1], color));
}

// ── Triangle ──

pub fn tessellate_triangle(a: Vec2, b: Vec2, c: Vec2, color: Color) -> Vec<f32> {
    let mut out = Vec::with_capacity(FLOATS_PER_VERT * 3);
    push_tri(&mut out, [a.x, a.y], [b.x, b.y], [c.x, c.y], color);
    out
}

pub fn tessellate_triangle_stroke(a: Vec2, b: Vec2, c: Vec2, params: &StrokeParams) -> Vec<f32> {
    tessellate_polyline_closed(&[a, b, c], params.thickness, params.color)
}

// ── Convex polygon ──

pub fn tessellate_convex_polygon(points: &[Vec2], color: Color) -> Vec<f32> {
    if points.len() < 3 {
        return Vec::new();
    }
    let tri_count = points.len() - 2;
    let mut out = Vec::with_capacity(FLOATS_PER_VERT * 3 * tri_count);
    let p0 = [points[0].x, points[0].y];
    for i in 1..points.len() - 1 {
        push_tri(
            &mut out,
            p0,
            [points[i].x, points[i].y],
            [points[i + 1].x, points[i + 1].y],
            color,
        );
    }
    out
}

pub fn tessellate_polygon_stroke(points: &[Vec2], params: &StrokeParams) -> Vec<f32> {
    if points.len() < 3 {
        return Vec::new();
    }
    tessellate_polyline_closed(points, params.thickness, params.color)
}

// ── Polyline (open) ──

pub fn tessellate_polyline(points: &[Vec2], params: &LineParams) -> Vec<f32> {
    if points.len() < 2 {
        return Vec::new();
    }
    let seg_count = points.len() - 1;
    let mut out = Vec::with_capacity(FLOATS_PER_VERT * 6 * seg_count);
    for i in 0..seg_count {
        push_thick_segment(&mut out, points[i], points[i + 1], params.thickness, params.color);
    }
    out
}

// closed polyline for stroke outlines
fn tessellate_polyline_closed(points: &[Vec2], thickness: f32, color: Color) -> Vec<f32> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(FLOATS_PER_VERT * 6 * n);
    for i in 0..n {
        let next = (i + 1) % n;
        push_thick_segment(&mut out, points[i], points[next], thickness, color);
    }
    out
}

fn push_thick_segment(out: &mut Vec<f32>, from: Vec2, to: Vec2, thickness: f32, color: Color) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return;
    }
    let nx = -dy / len * thickness * 0.5;
    let ny = dx / len * thickness * 0.5;

    let a = [from.x + nx, from.y + ny];
    let b = [from.x - nx, from.y - ny];
    let c = [to.x - nx, to.y - ny];
    let d = [to.x + nx, to.y + ny];

    push_tri(out, a, b, c, color);
    push_tri(out, a, c, d, color);
}

// ── Ellipse ──

fn adaptive_segments(radius: f32) -> usize {
    ((radius * 4.0) as usize).clamp(16, 128)
}

pub fn tessellate_ellipse_fill(center: Vec2, radii: Vec2, color: Color) -> Vec<f32> {
    let segments = adaptive_segments(radii.x.max(radii.y));
    let mut out = Vec::with_capacity(FLOATS_PER_VERT * 3 * segments);
    let step = std::f32::consts::TAU / segments as f32;

    for i in 0..segments {
        let a0 = step * i as f32;
        let a1 = step * (i + 1) as f32;
        push_tri(
            &mut out,
            [center.x, center.y],
            [center.x + a0.cos() * radii.x, center.y + a0.sin() * radii.y],
            [center.x + a1.cos() * radii.x, center.y + a1.sin() * radii.y],
            color,
        );
    }
    out
}

pub fn tessellate_ellipse_stroke(
    center: Vec2,
    radii: Vec2,
    params: &StrokeParams,
) -> Vec<f32> {
    let segments = adaptive_segments(radii.x.max(radii.y));
    let step = std::f32::consts::TAU / segments as f32;
    let mut points = Vec::with_capacity(segments);
    for i in 0..segments {
        let a = step * i as f32;
        points.push(Vec2::new(
            center.x + a.cos() * radii.x,
            center.y + a.sin() * radii.y,
        ));
    }
    tessellate_polyline_closed(&points, params.thickness, params.color)
}

// ── Arc ──

pub fn tessellate_arc_fill(
    center: Vec2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: Color,
) -> Vec<f32> {
    let sweep = end_angle - start_angle;
    let segments = ((sweep.abs() / std::f32::consts::TAU * adaptive_segments(radius) as f32) as usize).max(4);
    let step = sweep / segments as f32;
    let mut out = Vec::with_capacity(FLOATS_PER_VERT * 3 * segments);

    for i in 0..segments {
        let a0 = start_angle + step * i as f32;
        let a1 = start_angle + step * (i + 1) as f32;
        push_tri(
            &mut out,
            [center.x, center.y],
            [center.x + a0.cos() * radius, center.y + a0.sin() * radius],
            [center.x + a1.cos() * radius, center.y + a1.sin() * radius],
            color,
        );
    }
    out
}

pub fn tessellate_arc_stroke(
    center: Vec2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    params: &StrokeParams,
) -> Vec<f32> {
    let sweep = end_angle - start_angle;
    let segments = ((sweep.abs() / std::f32::consts::TAU * adaptive_segments(radius) as f32) as usize).max(4);
    let step = sweep / segments as f32;
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let a = start_angle + step * i as f32;
        points.push(Vec2::new(
            center.x + a.cos() * radius,
            center.y + a.sin() * radius,
        ));
    }
    tessellate_polyline(&points, &LineParams::new(params.color, params.thickness))
}

// ── Rounded rectangle ──

pub fn tessellate_rounded_rect_fill(rrect: RoundedRect, color: Color) -> Vec<f32> {
    let r = rrect.radius;
    let x = rrect.rect.pos.x;
    let y = rrect.rect.pos.y;
    let w = rrect.rect.size.x;
    let h = rrect.rect.size.y;

    // clamp radius to half the smallest dimension
    let r = r.min(w * 0.5).min(h * 0.5);

    let arc_segs = (r * 4.0).clamp(4.0, 32.0) as usize;
    // estimate: center rect + 4 side rects + 4 corner arcs
    let mut out = Vec::with_capacity(FLOATS_PER_VERT * 3 * (2 + 8 + 4 * arc_segs));

    // center rectangle (between the rounded corners)
    let cx0 = x + r;
    let cx1 = x + w - r;
    let cy0 = y + r;
    let cy1 = y + h - r;

    // center
    push_tri(&mut out, [cx0, cy0], [cx1, cy0], [cx1, cy1], color);
    push_tri(&mut out, [cx0, cy0], [cx1, cy1], [cx0, cy1], color);

    // top side
    push_tri(&mut out, [cx0, y], [cx1, y], [cx1, cy0], color);
    push_tri(&mut out, [cx0, y], [cx1, cy0], [cx0, cy0], color);

    // bottom side
    push_tri(&mut out, [cx0, cy1], [cx1, cy1], [cx1, y + h], color);
    push_tri(&mut out, [cx0, cy1], [cx1, y + h], [cx0, y + h], color);

    // left side
    push_tri(&mut out, [x, cy0], [cx0, cy0], [cx0, cy1], color);
    push_tri(&mut out, [x, cy0], [cx0, cy1], [x, cy1], color);

    // right side
    push_tri(&mut out, [cx1, cy0], [x + w, cy0], [x + w, cy1], color);
    push_tri(&mut out, [cx1, cy0], [x + w, cy1], [cx1, cy1], color);

    // four corner arcs (quarter circles)
    let corners = [
        (cx1, cy0, -std::f32::consts::FRAC_PI_2, 0.0),           // top-right
        (cx0, cy0, -std::f32::consts::PI, -std::f32::consts::FRAC_PI_2), // top-left
        (cx0, cy1, std::f32::consts::FRAC_PI_2, std::f32::consts::PI),   // bottom-left
        (cx1, cy1, 0.0, std::f32::consts::FRAC_PI_2),            // bottom-right
    ];

    for (cx, cy, a_start, a_end) in corners {
        let step = (a_end - a_start) / arc_segs as f32;
        for i in 0..arc_segs {
            let a0 = a_start + step * i as f32;
            let a1 = a_start + step * (i + 1) as f32;
            push_tri(
                &mut out,
                [cx, cy],
                [cx + a0.cos() * r, cy + a0.sin() * r],
                [cx + a1.cos() * r, cy + a1.sin() * r],
                color,
            );
        }
    }

    out
}

pub fn tessellate_rounded_rect_stroke(rrect: RoundedRect, params: &StrokeParams) -> Vec<f32> {
    let r = rrect.radius.min(rrect.rect.size.x * 0.5).min(rrect.rect.size.y * 0.5);
    let x = rrect.rect.pos.x;
    let y = rrect.rect.pos.y;
    let w = rrect.rect.size.x;
    let h = rrect.rect.size.y;

    let cx0 = x + r;
    let cx1 = x + w - r;
    let cy0 = y + r;
    let cy1 = y + h - r;

    let arc_segs = (r * 4.0).clamp(4.0, 32.0) as usize;

    // build outline path: straight edges + quarter-arc corners
    let mut points = Vec::with_capacity(4 * arc_segs + 4);

    // top edge (left to right)
    points.push(Vec2::new(cx0, y));
    points.push(Vec2::new(cx1, y));

    // top-right corner arc
    push_arc_points(&mut points, cx1, cy0, r, -std::f32::consts::FRAC_PI_2, 0.0, arc_segs);

    // right edge
    points.push(Vec2::new(x + w, cy0));
    points.push(Vec2::new(x + w, cy1));

    // bottom-right corner arc
    push_arc_points(&mut points, cx1, cy1, r, 0.0, std::f32::consts::FRAC_PI_2, arc_segs);

    // bottom edge (right to left)
    points.push(Vec2::new(cx1, y + h));
    points.push(Vec2::new(cx0, y + h));

    // bottom-left corner arc
    push_arc_points(&mut points, cx0, cy1, r, std::f32::consts::FRAC_PI_2, std::f32::consts::PI, arc_segs);

    // left edge
    points.push(Vec2::new(x, cy1));
    points.push(Vec2::new(x, cy0));

    // top-left corner arc
    push_arc_points(&mut points, cx0, cy0, r, -std::f32::consts::PI, -std::f32::consts::FRAC_PI_2, arc_segs);

    tessellate_polyline_closed(&points, params.thickness, params.color)
}

fn push_arc_points(
    points: &mut Vec<Vec2>,
    cx: f32,
    cy: f32,
    r: f32,
    a_start: f32,
    a_end: f32,
    segments: usize,
) {
    let step = (a_end - a_start) / segments as f32;
    for i in 0..=segments {
        let a = a_start + step * i as f32;
        points.push(Vec2::new(cx + a.cos() * r, cy + a.sin() * r));
    }
}
