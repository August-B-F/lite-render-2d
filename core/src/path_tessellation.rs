use lyon_tessellation::geom::point;
use lyon_tessellation::path::Path as LyonPath;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};

use crate::types::{Color, LineCap, LineJoin, Path, PathSegment, StrokeParams, StrokeStyle, Vec2};

const FLOATS_PER_VERT: usize = 12;

fn solid_vertex(x: f32, y: f32, color: Color) -> [f32; FLOATS_PER_VERT] {
    [x, y, 0.0, 0.0, color.r, color.g, color.b, color.a, 0.0, 0.0, 0.0, 0.0]
}

fn to_lyon_path(path: &Path) -> LyonPath {
    let mut builder = LyonPath::builder();
    let mut needs_end = false;
    for seg in &path.segments {
        match *seg {
            PathSegment::MoveTo(p) => {
                if needs_end {
                    builder.end(false);
                }
                builder.begin(point(p.x, p.y));
                needs_end = true;
            }
            PathSegment::LineTo(p) => {
                builder.line_to(point(p.x, p.y));
            }
            PathSegment::QuadTo { ctrl, to } => {
                builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(to.x, to.y));
            }
            PathSegment::CubicTo { ctrl1, ctrl2, to } => {
                builder.cubic_bezier_to(
                    point(ctrl1.x, ctrl1.y),
                    point(ctrl2.x, ctrl2.y),
                    point(to.x, to.y),
                );
            }
            PathSegment::Close => {
                builder.close();
                needs_end = false;
            }
        }
    }
    if needs_end {
        builder.end(false);
    }
    builder.build()
}

fn buffers_to_floats(buffers: &VertexBuffers<[f32; 2], u32>, color: Color) -> Vec<f32> {
    let mut out = Vec::with_capacity(buffers.indices.len() * FLOATS_PER_VERT);
    for &idx in &buffers.indices {
        let [x, y] = buffers.vertices[idx as usize];
        out.extend_from_slice(&solid_vertex(x, y, color));
    }
    out
}

pub fn tessellate_path_fill(path: &Path, color: Color) -> Vec<f32> {
    let lyon_path = to_lyon_path(path);
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    let result = tessellator.tessellate_path(
        &lyon_path,
        &FillOptions::default(),
        &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex| {
            let p = vertex.position();
            [p.x, p.y]
        }),
    );

    if result.is_err() {
        return Vec::new();
    }

    buffers_to_floats(&buffers, color)
}

pub fn tessellate_path_stroke(path: &Path, params: &StrokeParams) -> Vec<f32> {
    // for dashed/dotted styles, flatten the path and use polyline tessellation
    if !matches!(params.style, StrokeStyle::Solid) {
        let segments = crate::dash::dash_path(path, &params.style, params.thickness, 0.5);
        let mut out = Vec::new();
        for seg in &segments {
            out.extend(crate::tessellation::tessellate_polyline(
                seg,
                &crate::types::LineParams {
                    thickness: params.thickness,
                    color: params.color,
                    cap: params.cap,
                    join: params.join,
                    style: StrokeStyle::Solid, // already dashed
                    blend: crate::types::BlendMode::Alpha,
                    z_index: 0,
                    opacity: 1.0,
                },
            ));
        }
        return out;
    }

    let lyon_path = to_lyon_path(path);
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();

    let lyon_cap = match params.cap {
        LineCap::Butt => lyon_tessellation::LineCap::Butt,
        LineCap::Round => lyon_tessellation::LineCap::Round,
        LineCap::Square => lyon_tessellation::LineCap::Square,
    };
    let lyon_join = match params.join {
        LineJoin::Miter => lyon_tessellation::LineJoin::Miter,
        LineJoin::Round => lyon_tessellation::LineJoin::Round,
        LineJoin::Bevel => lyon_tessellation::LineJoin::Bevel,
    };
    let options = StrokeOptions::default()
        .with_line_width(params.thickness)
        .with_line_cap(lyon_cap)
        .with_line_join(lyon_join);

    let result = tessellator.tessellate_path(
        &lyon_path,
        &options,
        &mut BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex| {
            let p = vertex.position();
            [p.x, p.y]
        }),
    );

    if result.is_err() {
        return Vec::new();
    }

    buffers_to_floats(&buffers, params.color)
}

// tessellate concave polygon with holes using lyon
pub fn tessellate_complex_polygon(outer: &[Vec2], holes: &[&[Vec2]], color: Color) -> Vec<f32> {
    if outer.len() < 3 {
        return Vec::new();
    }

    let mut builder = LyonPath::builder();

    // outer boundary
    builder.begin(point(outer[0].x, outer[0].y));
    for p in &outer[1..] {
        builder.line_to(point(p.x, p.y));
    }
    builder.close();

    // hole boundaries (wound opposite direction)
    for hole in holes {
        if hole.len() < 3 { continue; }
        builder.begin(point(hole[0].x, hole[0].y));
        for p in &hole[1..] {
            builder.line_to(point(p.x, p.y));
        }
        builder.close();
    }

    let lyon_path = builder.build();
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    let result = tessellator.tessellate_path(
        &lyon_path,
        &FillOptions::default(),
        &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex| {
            let p = vertex.position();
            [p.x, p.y]
        }),
    );

    if result.is_err() {
        return Vec::new();
    }

    buffers_to_floats(&buffers, color)
}

// stroke a complex polygon outline (outer + holes)
pub fn tessellate_complex_polygon_stroke(
    outer: &[Vec2],
    holes: &[&[Vec2]],
    params: &StrokeParams,
) -> Vec<f32> {
    // stroke each boundary as a closed polyline
    let mut out = Vec::new();

    let line_params = crate::types::LineParams {
        thickness: params.thickness,
        color: params.color,
        cap: params.cap,
        join: params.join,
        style: params.style,
        blend: crate::types::BlendMode::Alpha,
        z_index: 0,
        opacity: 1.0,
    };

    // close outer loop and stroke
    if outer.len() >= 3 {
        let mut closed = outer.to_vec();
        closed.push(outer[0]);
        out.extend(crate::tessellation::tessellate_polyline(&closed, &line_params));
    }

    for hole in holes {
        if hole.len() < 3 { continue; }
        let mut closed = hole.to_vec();
        closed.push(hole[0]);
        out.extend(crate::tessellation::tessellate_polyline(&closed, &line_params));
    }

    out
}
