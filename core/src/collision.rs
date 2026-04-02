use crate::types::{Rect, Vec2};

impl Rect {
    // check if point is inside this rect
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.pos.x
            && point.y >= self.pos.y
            && point.x <= self.pos.x + self.size.x
            && point.y <= self.pos.y + self.size.y
    }

    // check if this rect overlaps with another
    pub fn intersects(&self, other: &Rect) -> bool {
        self.pos.x < other.pos.x + other.size.x
            && self.pos.x + self.size.x > other.pos.x
            && self.pos.y < other.pos.y + other.size.y
            && self.pos.y + self.size.y > other.pos.y
    }
}

// check if point is inside circle
pub fn circle_contains(center: Vec2, radius: f32, point: Vec2) -> bool {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    dx * dx + dy * dy <= radius * radius
}

// check if circle overlaps rect
pub fn circle_intersects_rect(center: Vec2, radius: f32, rect: &Rect) -> bool {
    // find closest point on rect to circle center
    let cx = center.x.clamp(rect.pos.x, rect.pos.x + rect.size.x);
    let cy = center.y.clamp(rect.pos.y, rect.pos.y + rect.size.y);
    let dx = center.x - cx;
    let dy = center.y - cy;
    dx * dx + dy * dy <= radius * radius
}

// ray casting algorithm for arbitrary polygon
pub fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let n = polygon.len();
    if n < 3 { return false; }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let pi = polygon[i];
        let pj = polygon[j];
        if (pi.y > point.y) != (pj.y > point.y) {
            let x_intersect = (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x;
            if point.x < x_intersect {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

// find intersection point of two line segments, if any
pub fn line_intersects_line(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> Option<Vec2> {
    let d1x = a2.x - a1.x;
    let d1y = a2.y - a1.y;
    let d2x = b2.x - b1.x;
    let d2y = b2.y - b1.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-10 {
        return None; // paralell
    }

    let t = ((b1.x - a1.x) * d2y - (b1.y - a1.y) * d2x) / denom;
    let u = ((b1.x - a1.x) * d1y - (b1.y - a1.y) * d1x) / denom;

    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(Vec2::new(a1.x + t * d1x, a1.y + t * d1y))
    } else {
        None
    }
}
