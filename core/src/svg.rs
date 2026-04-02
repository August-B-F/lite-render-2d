use crate::types::{
    Color, DrawParams, LineCap, LineJoin, Path, PathSegment, StrokeParams, StrokeStyle,
    Transform2D, Vec2,
};

pub struct SvgImage {
    tree: usvg::Tree,
    width: f32,
    height: f32,
}

// intermediate draw commands from svg tree
pub enum SvgDrawCommand {
    FillPath {
        path: Path,
        color: Color,
        opacity: f32,
    },
    StrokePath {
        path: Path,
        params: StrokeParams,
        opacity: f32,
    },
    PushTransform(Transform2D),
    PopTransform,
}

impl SvgImage {
    pub fn from_data(data: &[u8]) -> Result<Self, String> {
        let tree = usvg::Tree::from_data(data, &usvg::Options::default())
            .map_err(|e| e.to_string())?;
        let sz = tree.size();
        Ok(Self {
            width: sz.width(),
            height: sz.height(),
            tree,
        })
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn to_commands(&self) -> Vec<SvgDrawCommand> {
        let mut cmds = Vec::new();
        walk_group(self.tree.root(), &mut cmds);
        cmds
    }
}

fn walk_group(group: &usvg::Group, cmds: &mut Vec<SvgDrawCommand>) {
    for child in group.children() {
        match child {
            usvg::Node::Group(ref g) => {
                let t = g.transform();
                let transform = Transform2D {
                    pos: Vec2::new(t.tx as f32, t.ty as f32),
                    scale: Vec2::new(t.sx as f32, t.sy as f32),
                    rotation: 0.0, // usvg decomposes rotation into the matrix
                };
                cmds.push(SvgDrawCommand::PushTransform(transform));
                walk_group(g, cmds);
                cmds.push(SvgDrawCommand::PopTransform);
            }
            usvg::Node::Path(ref p) => {
                convert_path(p, cmds);
            }
            usvg::Node::Image(_) => {
                // embedded images not supported yet
            }
            usvg::Node::Text(_) => {
                // text elements not supported yet
            }
        }
    }
}

fn convert_path(upath: &usvg::Path, cmds: &mut Vec<SvgDrawCommand>) {
    let path = convert_usvg_segments(upath.data());

    // handle fill
    if let Some(ref fill) = upath.fill() {
        if let Some(color) = paint_to_color(&fill.paint(), fill.opacity().get() as f32) {
            cmds.push(SvgDrawCommand::FillPath {
                path: path.clone(),
                color,
                opacity: fill.opacity().get() as f32,
            });
        }
    }

    // handle stroke
    if let Some(ref stroke) = upath.stroke() {
        if let Some(color) = paint_to_color(&stroke.paint(), stroke.opacity().get() as f32) {
            let cap = match stroke.linecap() {
                usvg::LineCap::Butt => LineCap::Butt,
                usvg::LineCap::Round => LineCap::Round,
                usvg::LineCap::Square => LineCap::Square,
            };
            let join = match stroke.linejoin() {
                usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => LineJoin::Miter,
                usvg::LineJoin::Round => LineJoin::Round,
                usvg::LineJoin::Bevel => LineJoin::Bevel,
            };
            cmds.push(SvgDrawCommand::StrokePath {
                path: path.clone(),
                params: StrokeParams {
                    color,
                    thickness: stroke.width().get() as f32,
                    style: StrokeStyle::Solid,
                    cap,
                    join,
                },
                opacity: stroke.opacity().get() as f32,
            });
        }
    }
}

fn convert_usvg_segments(data: &usvg::tiny_skia_path::Path) -> Path {
    let mut path = Path::new();
    for seg in data.segments() {
        match seg {
            usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                path.segments.push(PathSegment::MoveTo(Vec2::new(p.x, p.y)));
            }
            usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                path.segments.push(PathSegment::LineTo(Vec2::new(p.x, p.y)));
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(p1, p) => {
                path.segments.push(PathSegment::QuadTo {
                    ctrl: Vec2::new(p1.x, p1.y),
                    to: Vec2::new(p.x, p.y),
                });
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p) => {
                path.segments.push(PathSegment::CubicTo {
                    ctrl1: Vec2::new(p1.x, p1.y),
                    ctrl2: Vec2::new(p2.x, p2.y),
                    to: Vec2::new(p.x, p.y),
                });
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                path.segments.push(PathSegment::Close);
            }
        }
    }
    path
}

fn paint_to_color(paint: &usvg::Paint, opacity: f32) -> Option<Color> {
    match paint {
        usvg::Paint::Color(c) => Some(Color::new(
            c.red as f32 / 255.0,
            c.green as f32 / 255.0,
            c.blue as f32 / 255.0,
            opacity,
        )),
        usvg::Paint::LinearGradient(_) | usvg::Paint::RadialGradient(_) | usvg::Paint::Pattern(_) => {
            // gradients/patterns not yet supported, use fallback
            None
        }
    }
}

// render svg using existing renderer draw calls
pub fn draw_svg(
    renderer: &mut dyn crate::renderer::Renderer,
    svg: &SvgImage,
    position: Vec2,
    scale: f32,
) {
    let cmds = svg.to_commands();
    renderer.push_transform(Transform2D {
        pos: position,
        scale: Vec2::new(scale, scale),
        rotation: 0.0,
    });

    for cmd in &cmds {
        match cmd {
            SvgDrawCommand::FillPath { path, color, opacity } => {
                renderer.draw_path(path, DrawParams::fill(*color).with_opacity(*opacity));
            }
            SvgDrawCommand::StrokePath { path, params, .. } => {
                renderer.stroke_path(path, params.clone());
            }
            SvgDrawCommand::PushTransform(t) => {
                renderer.push_transform(*t);
            }
            SvgDrawCommand::PopTransform => {
                renderer.pop_transform();
            }
        }
    }

    renderer.pop_transform();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_svg() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect x="10" y="10" width="80" height="80" fill="red"/>
        </svg>"#;
        let svg = SvgImage::from_data(svg_data).expect("parse svg");
        assert_eq!(svg.width(), 100.0);
        assert_eq!(svg.height(), 100.0);
    }

    #[test]
    fn test_rect_produces_fill_command() {
        let svg_data = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect x="0" y="0" width="50" height="50" fill="#ff0000"/>
        </svg>"##;
        let svg = SvgImage::from_data(svg_data).expect("parse svg");
        let cmds = svg.to_commands();
        let has_fill = cmds.iter().any(|c| matches!(c, SvgDrawCommand::FillPath { .. }));
        assert!(has_fill, "should produce a FillPath command for rect");
    }

    #[test]
    fn test_stroke_params_mapping() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <line x1="0" y1="0" x2="100" y2="100" stroke="blue" stroke-width="3"/>
        </svg>"#;
        let svg = SvgImage::from_data(svg_data).expect("parse svg");
        let cmds = svg.to_commands();
        let has_stroke = cmds.iter().any(|c| matches!(c, SvgDrawCommand::StrokePath { .. }));
        assert!(has_stroke, "should produce a StrokePath command for stroked line");
    }

    #[test]
    fn test_group_transform() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <g transform="translate(10,20)">
                <rect x="0" y="0" width="10" height="10" fill="green"/>
            </g>
        </svg>"#;
        let svg = SvgImage::from_data(svg_data).expect("parse svg");
        let cmds = svg.to_commands();
        let has_push = cmds.iter().any(|c| matches!(c, SvgDrawCommand::PushTransform(_)));
        let has_pop = cmds.iter().any(|c| matches!(c, SvgDrawCommand::PopTransform));
        assert!(has_push, "should have PushTransform");
        assert!(has_pop, "should have PopTransform");
    }

    #[test]
    fn test_invalid_svg_returns_error() {
        let result = SvgImage::from_data(b"not valid svg");
        assert!(result.is_err());
    }
}
