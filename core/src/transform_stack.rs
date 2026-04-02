use crate::types::{Transform2D, Vec2};

/// 2D affine transform as [a, b, c, d, tx, ty]
/// Represents the matrix:
///   | a  c  tx |
///   | b  d  ty |
///   | 0  0   1 |
type Affine = [f32; 6];

const IDENTITY: Affine = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn multiply(a: &Affine, b: &Affine) -> Affine {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

fn from_transform2d(t: &Transform2D) -> Affine {
    let cos = t.rotation.cos();
    let sin = t.rotation.sin();
    [
        cos * t.scale.x,
        sin * t.scale.x,
        -sin * t.scale.y,
        cos * t.scale.y,
        t.pos.x,
        t.pos.y,
    ]
}

pub struct TransformStack {
    stack: Vec<Affine>,
    current: Affine,
    identity: bool, // cached flag, avoids 6-float compare per draw call
}

impl TransformStack {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            current: IDENTITY,
            identity: true,
        }
    }

    pub fn push(&mut self, transform: Transform2D) {
        self.stack.push(self.current);
        let m = from_transform2d(&transform);
        self.current = multiply(&self.current, &m);
        self.identity = false;
    }

    pub fn pop(&mut self) {
        if let Some(prev) = self.stack.pop() {
            self.current = prev;
            self.identity = self.stack.is_empty() && self.current == IDENTITY;
        }
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.current = IDENTITY;
        self.identity = true;
    }

    pub fn is_identity(&self) -> bool {
        self.identity
    }

    pub fn apply(&self, p: Vec2) -> Vec2 {
        Vec2::new(
            self.current[0] * p.x + self.current[2] * p.y + self.current[4],
            self.current[1] * p.x + self.current[3] * p.y + self.current[5],
        )
    }
}
