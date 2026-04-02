use lyon_tessellation::geom::point;
use lyon_tessellation::path::Path as LyonPath;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};

use crate::types::{Color, Path, PathSegment, StrokeParams};

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
    let lyon_path = to_lyon_path(path);
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();

    let options = StrokeOptions::default().with_line_width(params.thickness);

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
