use crate::dash;
use crate::transform_stack::TransformStack;
use crate::types::{Color, DrawStyle, GradientStop, LineCap, LineJoin, LineParams, RoundedRect, StrokeParams, StrokeStyle, Vec2};

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

// interpolate color from sorted gradient stops at parameter t (0..1)
fn color_from_stops(stops: &[GradientStop], t: f32) -> Color {
    if stops.is_empty() { return Color::WHITE; }
    if stops.len() == 1 || t <= stops[0].offset { return stops[0].color; }
    let last = stops.len() - 1;
    if t >= stops[last].offset { return stops[last].color; }
    for i in 0..last {
        if t >= stops[i].offset && t <= stops[i + 1].offset {
            let range = stops[i + 1].offset - stops[i].offset;
            if range < 1e-6 { return stops[i].color; }
            let local_t = (t - stops[i].offset) / range;
            return stops[i].color.lerp(stops[i + 1].color, local_t);
        }
    }
    stops[last].color
}

// rewrite per-vertex colors based on gradient style
// no-op for Fill and Stroke
pub fn apply_gradient(verts: &mut [f32], style: &DrawStyle) {
    match style {
        DrawStyle::LinearGradient { start, end, color_start, color_end } => {
            for chunk in verts.chunks_exact_mut(FLOATS_PER_VERT) {
                let pos = Vec2::new(chunk[0], chunk[1]);
                let c = linear_gradient_color(pos, *start, *end, *color_start, *color_end);
                chunk[4] = c.r;
                chunk[5] = c.g;
                chunk[6] = c.b;
                chunk[7] = c.a;
            }
        }
        DrawStyle::RadialGradient { center, radius, color_inner, color_outer } => {
            for chunk in verts.chunks_exact_mut(FLOATS_PER_VERT) {
                let pos = Vec2::new(chunk[0], chunk[1]);
                let c = radial_gradient_color(pos, *center, *radius, *color_inner, *color_outer);
                chunk[4] = c.r;
                chunk[5] = c.g;
                chunk[6] = c.b;
                chunk[7] = c.a;
            }
        }
        DrawStyle::LinearGradientStops { start, end, stops } => {
            let ax = end.x - start.x;
            let ay = end.y - start.y;
            let dot_aa = ax * ax + ay * ay;
            for chunk in verts.chunks_exact_mut(FLOATS_PER_VERT) {
                let pos = Vec2::new(chunk[0], chunk[1]);
                let t = if dot_aa < 1e-10 { 0.0 } else {
                    let dx = pos.x - start.x;
                    let dy = pos.y - start.y;
                    ((dx * ax + dy * ay) / dot_aa).clamp(0.0, 1.0)
                };
                let c = color_from_stops(stops, t);
                chunk[4] = c.r;
                chunk[5] = c.g;
                chunk[6] = c.b;
                chunk[7] = c.a;
            }
        }
        DrawStyle::RadialGradientStops { center, radius, stops } => {
            let r = *radius;
            for chunk in verts.chunks_exact_mut(FLOATS_PER_VERT) {
                let pos = Vec2::new(chunk[0], chunk[1]);
                let dx = pos.x - center.x;
                let dy = pos.y - center.y;
                let t = if r < 1e-6 { 0.0 } else {
                    ((dx * dx + dy * dy).sqrt() / r).clamp(0.0, 1.0)
                };
                let c = color_from_stops(stops, t);
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
    match params.style {
        StrokeStyle::Solid => {
            polyline_inner(points, params.thickness, params.color, params.cap, params.join, false)
        }
        _ => {
            let segments = dash::dash_polyline(points, &params.style, params.thickness);
            let mut out = Vec::new();
            for seg in &segments {
                out.extend(polyline_inner(seg, params.thickness, params.color, params.cap, params.join, false));
            }
            out
        }
    }
}

// closed polyline for stroke outlines (no caps, joins at every vertex)
fn tessellate_polyline_closed(points: &[Vec2], thickness: f32, color: Color) -> Vec<f32> {
    if points.len() < 2 {
        return Vec::new();
    }
    polyline_inner(points, thickness, color, LineCap::Butt, LineJoin::Miter, true)
}

fn seg_normal(a: Vec2, b: Vec2) -> (Vec2, Vec2) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return (Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0));
    }
    let dir = Vec2::new(dx / len, dy / len);
    let normal = Vec2::new(-dir.y, dir.x);
    (dir, normal)
}

fn polyline_inner(
    points: &[Vec2],
    thickness: f32,
    color: Color,
    cap: LineCap,
    join: LineJoin,
    closed: bool,
) -> Vec<f32> {
    let n = points.len();
    let half = thickness * 0.5;
    let seg_count = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(FLOATS_PER_VERT * 6 * (seg_count + n));

    // precompute directions and normals for each segment
    let mut dirs = Vec::with_capacity(seg_count);
    let mut normals = Vec::with_capacity(seg_count);
    for i in 0..seg_count {
        let next = if closed { (i + 1) % n } else { i + 1 };
        let (d, nm) = seg_normal(points[i], points[next]);
        dirs.push(d);
        normals.push(nm);
    }

    // emit segments with proper offsets at each vertex
    for i in 0..seg_count {
        let next = if closed { (i + 1) % n } else { i + 1 };
        let p0 = points[i];
        let p1 = points[next];
        let nm = normals[i];

        // offset corners of this segment
        let a = [p0.x + nm.x * half, p0.y + nm.y * half];
        let b = [p0.x - nm.x * half, p0.y - nm.y * half];
        let c = [p1.x - nm.x * half, p1.y - nm.y * half];
        let d = [p1.x + nm.x * half, p1.y + nm.y * half];

        push_tri(&mut out, a, b, c, color);
        push_tri(&mut out, a, c, d, color);
    }

    // emit joins at interior (or all for closed) vertices
    let join_start = if closed { 0 } else { 1 };
    let join_end = if closed { n } else { n - 1 };

    for i in join_start..join_end {
        let seg_prev = if closed { (i + seg_count - 1) % seg_count } else { i - 1 };
        let seg_next = if closed { i % seg_count } else { i };
        if seg_next >= normals.len() || seg_prev >= normals.len() {
            continue;
        }

        let n0 = normals[seg_prev];
        let n1 = normals[seg_next];
        let p = points[i];

        // cross product to determine turn direction
        let cross = n0.x * n1.y - n0.y * n1.x;
        if cross.abs() < 1e-6 {
            continue; // colinear, no join needed
        }

        match join {
            LineJoin::Bevel => {
                push_bevel_join(&mut out, p, n0, n1, half, cross, color);
            }
            LineJoin::Round => {
                push_round_join(&mut out, p, n0, n1, half, cross, color);
            }
            LineJoin::Miter => {
                push_miter_join(&mut out, p, n0, n1, half, cross, color);
            }
        }
    }

    // emit caps for open polylines
    if !closed && n >= 2 {
        match cap {
            LineCap::Butt => {}
            LineCap::Round => {
                push_round_cap(&mut out, points[0], dirs[0], normals[0], half, color, false);
                push_round_cap(&mut out, points[n - 1], dirs[seg_count - 1], normals[seg_count - 1], half, color, true);
            }
            LineCap::Square => {
                push_square_cap(&mut out, points[0], dirs[0], normals[0], half, color, false);
                push_square_cap(&mut out, points[n - 1], dirs[seg_count - 1], normals[seg_count - 1], half, color, true);
            }
        }
    }

    out
}

fn push_bevel_join(
    out: &mut Vec<f32>,
    p: Vec2,
    n0: Vec2,
    n1: Vec2,
    half: f32,
    cross: f32,
    color: Color,
) {
    // fill the gap between the two offset edges with a triangle
    if cross > 0.0 {
        // left turn - gap is on the positive normal side
        let a = [p.x + n0.x * half, p.y + n0.y * half];
        let b = [p.x + n1.x * half, p.y + n1.y * half];
        push_tri(out, [p.x, p.y], a, b, color);
    } else {
        // right turn - gap is on the negative normal side
        let a = [p.x - n0.x * half, p.y - n0.y * half];
        let b = [p.x - n1.x * half, p.y - n1.y * half];
        push_tri(out, [p.x, p.y], b, a, color);
    }
}

fn push_round_join(
    out: &mut Vec<f32>,
    p: Vec2,
    n0: Vec2,
    n1: Vec2,
    half: f32,
    cross: f32,
    color: Color,
) {
    let (start_n, end_n) = if cross > 0.0 { (n0, n1) } else { (Vec2::new(-n0.x, -n0.y), Vec2::new(-n1.x, -n1.y)) };

    let a0 = start_n.y.atan2(start_n.x);
    let mut a1 = end_n.y.atan2(end_n.x);

    // ensure we go the short way around
    if cross > 0.0 {
        while a1 < a0 { a1 += std::f32::consts::TAU; }
    } else {
        while a1 > a0 { a1 -= std::f32::consts::TAU; }
    }

    let angle_span = (a1 - a0).abs();
    let segs = ((angle_span * half * 2.0).ceil() as usize).clamp(3, 16);
    let step = (a1 - a0) / segs as f32;

    let center = [p.x, p.y];
    for s in 0..segs {
        let ang0 = a0 + step * s as f32;
        let ang1 = a0 + step * (s + 1) as f32;
        let v0 = [p.x + ang0.cos() * half, p.y + ang0.sin() * half];
        let v1 = [p.x + ang1.cos() * half, p.y + ang1.sin() * half];
        push_tri(out, center, v0, v1, color);
    }
}

fn push_miter_join(
    out: &mut Vec<f32>,
    p: Vec2,
    n0: Vec2,
    n1: Vec2,
    half: f32,
    cross: f32,
    color: Color,
) {
    // compute miter offset
    let nx = n0.x + n1.x;
    let ny = n0.y + n1.y;
    let dot = n0.x * n1.x + n0.y * n1.y;
    let miter_scale = 1.0 / (1.0 + dot);

    // miter limit: if the miter length exceeds 4x the half-width, fall back to bevel
    if miter_scale > 4.0 {
        push_bevel_join(out, p, n0, n1, half, cross, color);
        return;
    }

    let mx = nx * half * miter_scale;
    let my = ny * half * miter_scale;

    if cross > 0.0 {
        // fill gap on positive side with miter point
        let miter_pt = [p.x + mx, p.y + my];
        let a = [p.x + n0.x * half, p.y + n0.y * half];
        let b = [p.x + n1.x * half, p.y + n1.y * half];
        push_tri(out, [p.x, p.y], a, miter_pt, color);
        push_tri(out, [p.x, p.y], miter_pt, b, color);
    } else {
        let miter_pt = [p.x - mx, p.y - my];
        let a = [p.x - n0.x * half, p.y - n0.y * half];
        let b = [p.x - n1.x * half, p.y - n1.y * half];
        push_tri(out, [p.x, p.y], b, miter_pt, color);
        push_tri(out, [p.x, p.y], miter_pt, a, color);
    }
}

fn push_round_cap(
    out: &mut Vec<f32>,
    point: Vec2,
    _dir: Vec2,
    normal: Vec2,
    half: f32,
    color: Color,
    is_end: bool,
) {
    // semicircle centered at point, oriented by the normal
    let start_angle = normal.y.atan2(normal.x);
    let sweep = std::f32::consts::PI;
    let base_angle = if is_end { start_angle } else { start_angle + std::f32::consts::PI };

    let segs = (half * 2.0).ceil().clamp(4.0, 16.0) as usize;
    let step = sweep / segs as f32;
    let center = [point.x, point.y];

    for s in 0..segs {
        let a0 = base_angle + step * s as f32;
        let a1 = base_angle + step * (s + 1) as f32;
        let v0 = [point.x + a0.cos() * half, point.y + a0.sin() * half];
        let v1 = [point.x + a1.cos() * half, point.y + a1.sin() * half];
        push_tri(out, center, v0, v1, color);
    }
}

fn push_square_cap(
    out: &mut Vec<f32>,
    point: Vec2,
    dir: Vec2,
    normal: Vec2,
    half: f32,
    color: Color,
    is_end: bool,
) {
    // extend the line by half-thickness beyond the endpoint
    let extend = if is_end { 1.0 } else { -1.0 };
    let ex = point.x + dir.x * half * extend;
    let ey = point.y + dir.y * half * extend;

    let a = [point.x + normal.x * half, point.y + normal.y * half];
    let b = [point.x - normal.x * half, point.y - normal.y * half];
    let c = [ex - normal.x * half, ey - normal.y * half];
    let d = [ex + normal.x * half, ey + normal.y * half];

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
    let x = rrect.rect.pos.x;
    let y = rrect.rect.pos.y;
    let w = rrect.rect.size.x;
    let h = rrect.rect.size.y;
    let half_min = (w * 0.5).min(h * 0.5);

    // per-corner radii clamped independently
    let rtl = rrect.radius_tl.min(half_min);
    let rtr = rrect.radius_tr.min(half_min);
    let rbl = rrect.radius_bl.min(half_min);
    let rbr = rrect.radius_br.min(half_min);

    let max_r = rtl.max(rtr).max(rbl).max(rbr);
    let arc_segs = (max_r * 4.0).clamp(4.0, 32.0) as usize;
    let mut out = Vec::with_capacity(FLOATS_PER_VERT * 3 * (10 + 4 * arc_segs));

    // inset corners
    let lx = x + rtl.max(rbl);
    let rx = x + w - rtr.max(rbr);
    let ty = y + rtl.max(rtr);
    let by = y + h - rbl.max(rbr);

    // center rect
    push_tri(&mut out, [lx, ty], [rx, ty], [rx, by], color);
    push_tri(&mut out, [lx, ty], [rx, by], [lx, by], color);

    // top side
    let tl_x = x + rtl;
    let tr_x = x + w - rtr;
    let tl_y = y + rtl;
    let tr_y = y + rtr;
    push_tri(&mut out, [tl_x, y], [tr_x, y], [rx, ty], color);
    push_tri(&mut out, [tl_x, y], [rx, ty], [lx, ty], color);

    // bottom side
    let bl_x = x + rbl;
    let br_x = x + w - rbr;
    let bl_y = y + h - rbl;
    let br_y = y + h - rbr;
    push_tri(&mut out, [lx, by], [rx, by], [br_x, y + h], color);
    push_tri(&mut out, [lx, by], [br_x, y + h], [bl_x, y + h], color);

    // left side
    push_tri(&mut out, [x, tl_y], [lx, ty], [lx, by], color);
    push_tri(&mut out, [x, tl_y], [lx, by], [x, bl_y], color);

    // right side
    push_tri(&mut out, [rx, ty], [x + w, tr_y], [x + w, br_y], color);
    push_tri(&mut out, [rx, ty], [x + w, br_y], [rx, by], color);

    // corner arcs: (center_x, center_y, radius, start_angle, end_angle)
    let corners = [
        (tr_x, tr_y, rtr, -std::f32::consts::FRAC_PI_2, 0.0),           // top-right
        (tl_x, tl_y, rtl, -std::f32::consts::PI, -std::f32::consts::FRAC_PI_2), // top-left
        (bl_x, bl_y, rbl, std::f32::consts::FRAC_PI_2, std::f32::consts::PI),   // bottom-left
        (br_x, br_y, rbr, 0.0, std::f32::consts::FRAC_PI_2),            // bottom-right
    ];

    for (cx, cy, cr, a_start, a_end) in corners {
        if cr < 0.5 { continue; }
        let segs = ((cr * 4.0) as usize).clamp(4, 32);
        let step = (a_end - a_start) / segs as f32;
        for i in 0..segs {
            let a0 = a_start + step * i as f32;
            let a1 = a_start + step * (i + 1) as f32;
            push_tri(
                &mut out,
                [cx, cy],
                [cx + a0.cos() * cr, cy + a0.sin() * cr],
                [cx + a1.cos() * cr, cy + a1.sin() * cr],
                color,
            );
        }
    }

    out
}

pub fn tessellate_rounded_rect_stroke(rrect: RoundedRect, params: &StrokeParams) -> Vec<f32> {
    let x = rrect.rect.pos.x;
    let y = rrect.rect.pos.y;
    let w = rrect.rect.size.x;
    let h = rrect.rect.size.y;
    let half_min = (w * 0.5).min(h * 0.5);

    let rtl = rrect.radius_tl.min(half_min);
    let rtr = rrect.radius_tr.min(half_min);
    let rbl = rrect.radius_bl.min(half_min);
    let rbr = rrect.radius_br.min(half_min);

    let max_r = rtl.max(rtr).max(rbl).max(rbr);
    let arc_segs = (max_r * 4.0).clamp(4.0, 32.0) as usize;

    let mut points = Vec::with_capacity(4 * arc_segs + 8);

    // top edge (left to right)
    points.push(Vec2::new(x + rtl, y));
    points.push(Vec2::new(x + w - rtr, y));

    // top-right corner arc
    if rtr > 0.5 {
        push_arc_points(&mut points, x + w - rtr, y + rtr, rtr, -std::f32::consts::FRAC_PI_2, 0.0, arc_segs);
    }

    // right edge
    points.push(Vec2::new(x + w, y + rtr));
    points.push(Vec2::new(x + w, y + h - rbr));

    // bottom-right corner arc
    if rbr > 0.5 {
        push_arc_points(&mut points, x + w - rbr, y + h - rbr, rbr, 0.0, std::f32::consts::FRAC_PI_2, arc_segs);
    }

    // bottom edge (right to left)
    points.push(Vec2::new(x + w - rbr, y + h));
    points.push(Vec2::new(x + rbl, y + h));

    // bottom-left corner arc
    if rbl > 0.5 {
        push_arc_points(&mut points, x + rbl, y + h - rbl, rbl, std::f32::consts::FRAC_PI_2, std::f32::consts::PI, arc_segs);
    }

    // left edge
    points.push(Vec2::new(x, y + h - rbl));
    points.push(Vec2::new(x, y + rtl));

    // top-left corner arc
    if rtl > 0.5 {
        push_arc_points(&mut points, x + rtl, y + rtl, rtl, -std::f32::consts::PI, -std::f32::consts::FRAC_PI_2, arc_segs);
    }

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
