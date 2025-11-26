use euclid::{*, default::Vector2D, default::Box2D, vec2};
use macroquad::prelude::{*};
use crate::{
    point::Point,
    common_traits::*,
};

#[derive(Clone, Debug)]
pub struct Vertex {
    pub pos: Vector2D<f32>,
    pub clr: Option<Color>,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            pos: Vector2D::zero(),
            clr: None,
        }
    }
}

impl Vertex {
    pub fn new(x: f32, y: f32, clr: Option<Color>) -> Self {
        Self { pos: vec2(x,y), clr}
    }
}

impl Draw for Vertex {
    fn draw(&self) {
        draw_circle(self.pos.x, self.pos.y, 1.0, self.clr.unwrap_or(BLACK));
    }

    fn vertices(&self) -> Vec<Vertex> {
        vec![self.clone()]
    }
}

#[derive(Debug)]
pub struct Line2D {
    pub a: Vertex,
    pub b: Vertex,
    pub thickness: f32,
}

impl Draw for Line2D {
    fn draw(&self) {
        draw_line(self.a.pos.x, self.a.pos.y,
                  self.b.pos.x, self.b.pos.y,
                  self.thickness,
                  self.a.clr.or(self.b.clr).unwrap_or(BLACK));
        //FIXME: detect when a.clr != b.clr and mix colors accordingly
    }
    fn vertices(&self) -> Vec<Vertex> {
        vec![self.a.clone(), self.b.clone()]
    }
}

impl Select for Line2D {
    fn compute_aabb(&self) -> Box2D<f32> {
        let xs = {
            let mut xs = [self.a.pos.x, self.b.pos.x];
            if xs[0] > xs[1] { xs.reverse(); }
            xs
        };
        let ys = {
            let mut ys = [self.a.pos.y, self.b.pos.y];
            if ys[0] > ys[1] { ys.reverse(); }
            ys
        };
        Box2D::new(
            Point2D::new(xs[0],ys[0]),
            Point2D::new(xs[1],ys[1]),
        )
    }

    fn sample_signed_distance_field(&self, global_sample_point: &Vector2D<f32>) -> f32 {
        let xs = [self.a.pos.x, self.b.pos.x];
        let ys = [self.a.pos.y, self.b.pos.y];
        let tx = global_sample_point.x;
        let ty = global_sample_point.y;
        let numer = ((ys[1] - ys[0])*tx - (xs[1] - xs[0])*ty + xs[1]*ys[0] - ys[1]*xs[0]).abs();
        let denom = ((ys[1] - ys[0]).powi(2) + (xs[1] - xs[0]).powi(2)).sqrt();
        numer/denom - self.thickness
    }
}

#[derive(Debug)]
pub struct Circle {
    pub center: Vertex,
    pub radius: f32,
}

impl Draw for Circle {
    fn draw(&self) {
        draw_circle(self.center.pos.x, self.center.pos.y,
                    self.radius, self.center.clr.unwrap_or(BLACK));
    }
    fn vertices(&self) -> Vec<Vertex> {
        vec![self.center.clone()]
    }
}

impl Select for Circle {
    fn compute_aabb(&self) -> Box2D<f32> {
        Box2D::new(
            Point2D::new(self.center.pos.x - self.radius, self.center.pos.y - self.radius),
            Point2D::new(self.center.pos.x + self.radius, self.center.pos.y + self.radius),
        )
    }

    fn sample_signed_distance_field(&self, global_sample_point: &Vector2D<f32>) -> f32 {
        let offset = *global_sample_point - self.center.pos;
        let dist = (offset.x.powi(2) + offset.y.powi(2)).sqrt();
        dist - self.radius
    }
}

#[derive(Debug)]
pub struct Polygon {
    pub verts: Vec<Vertex>,
    pub edges: Vec<(usize, usize, Color)>,
    pub edge_thickness: f32,
    pub faces: Vec<(usize, usize, usize, Color)>,
}

impl Polygon {
    pub fn circle(vert_count: usize, radius: f32, edge_color: Color) -> Self {
        const TAU: f32 = std::f32::consts::PI * 2.0;
        let verts = (0..vert_count).map(|i| {
            Vertex::new(
                radius * ((i as f32)*TAU/(vert_count as f32)).cos(),
                radius * ((i as f32)*TAU/(vert_count as f32)).sin(),
                None,
            )
        }).collect();

        let edges = {
            let srcs = (0..vert_count).into_iter();
            let dsts = (1..vert_count).into_iter().chain(std::iter::once(0));
            srcs.zip(dsts).map(|e| (e.0, e.1, edge_color)).collect()
        };

        // TODO: faces

        Self {
            verts, edges, edge_thickness: 1.0, faces: vec![],
        }
    }

    pub fn rectangle(a: Vector2D<f32>, b: Vector2D<f32>, edge_color: Color, face_color: Color) -> Self {
        Self {
            verts: [a, Vector2D::new(b.x, a.y), b, Vector2D::new(a.x, b.y)]
                .iter()
                .map(|pos| Vertex::new(pos.x, pos.y, None)).collect(),
            edges: [(0,1),(1,2),(2,3),(3,0)].iter().map(|e| (e.0, e.1, edge_color)).collect(),
            faces: [(0,1,2), (0,2,3)].iter().map(|f| (f.0,f.1,f.2, face_color)).collect(),
            edge_thickness: 1.0,
        }
    }
}

impl Default for Polygon {
    fn default() -> Self {
        Self { verts: vec![], edges: vec![], faces: vec![], edge_thickness: 2.0 }
    }
}

impl Draw for Polygon {
    fn draw(&self) {
        for edge_data in &self.edges {
            let a = &self.verts[edge_data.0];
            let b = &self.verts[edge_data.1];
            let c = edge_data.2;
            draw_line(
                a.pos.x,a.pos.y,
                b.pos.x,b.pos.y,
                self.edge_thickness, c,
            )
        }
        for face_data in &self.faces {
            use glam::Vec2;
            let (a,b,c) = (
                Vec2::from(self.verts[face_data.0].pos.to_array()),
                Vec2::from(self.verts[face_data.1].pos.to_array()),
                Vec2::from(self.verts[face_data.2].pos.to_array()),
            );
            let clr = face_data.3;
            draw_triangle(a,b,c, clr);
        }

    }

    fn vertices(&self) -> Vec<Vertex> {
        self.verts.clone()
    }
}

use ordered_float::OrderedFloat;

type TripleSite = (Point, Point, Point);
pub type Segment = [Point; 2];

pub fn segment_intersection(seg1: Segment, seg2: Segment) -> Option<Point> {
    let a = seg1[0];
    let c = seg2[0];
    let r = seg1[1] - a;
    let s = seg2[1] - c;

    let denom = r.cross(s);
    if denom == 0.0 { return None; }

    let numer_a = (c - a).cross(s);
    let numer_c = (c - a).cross(r);

    let t = numer_a / denom;
    let u = numer_c / denom;

    if t < 0.0 || t > 1.0 || u < 0.0 || u > 1.0 { return None; }

    return Some(a + r * t);
}

pub fn circle_bottom(triple_site: TripleSite) -> Option<OrderedFloat<f64>> {
    let circle_center = circle_center(triple_site);
    if let None = circle_center { return None; }
    let circle_center = circle_center.unwrap();

    let (_, _, p3) = triple_site;
    let x3 = p3.x();
    let y3 = p3.y();
    let x_cen = circle_center.x();
    let y_cen = circle_center.y();

    let r = ((x3 - x_cen) * (x3 - x_cen) + (y3 - y_cen) * (y3 - y_cen)).sqrt();

    return Some(OrderedFloat::<f64>(y_cen - r));
}

pub fn circle_center(triple_site: TripleSite) -> Option<Point> {
    let (p1, p2, p3) = triple_site;
    let x1 = p1.x();
    let x2 = p2.x();
    let x3 = p3.x();
    let y1 = p1.y();
    let y2 = p2.y();
    let y3 = p3.y();

    let c1 = x3 * x3 + y3 * y3 - x1 * x1 - y1 * y1;
    let c2 = x3 * x3 + y3 * y3 - x2 * x2 - y2 * y2;
    let a1 = -2. * (x1 - x3);
    let a2 = -2. * (x2 - x3);
    let b1 = -2. * (y1 - y3);
    let b2 = -2. * (y2 - y3);

    let numer = c1 * a2 - c2 * a1;
    let denom = b1 * a2 - b2 * a1;

    if denom == 0.0 { return None; }
    let y_cen = numer / denom;


    let x_cen = if a2 != 0.0 {
        (c2 - b2 * y_cen) / a2
    } else {
        (c1 - b1 * y_cen) / a1
    };

    return Some(Point::new(x_cen, y_cen));
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_circle_center() {
        let circle_triple = (Point::new(-1.0, 0.0), Point::new(0.0, 1.0), Point::new(1.0, 0.0));
        assert_eq!(circle_center(circle_triple).unwrap(), Point::new(0.0, 0.0));
    }

    #[test]
    fn simple_circle_bottom() {
        let circle_triple = (Point::new(-1.0, 0.0), Point::new(0.0, 1.0), Point::new(1.0, 0.0));
        assert_eq!(circle_bottom(circle_triple).unwrap(), OrderedFloat(-1.0));
    }

    #[test]
    fn degenerate_circle() {
        let circle_triple = (Point::new(-1.0, 0.0), Point::new(1.0, 0.0), Point::new(0.0, 0.0));
        assert_eq!(circle_bottom(circle_triple), None);
    }

    #[test]
    fn simple_segments_intersect() {
        let line1 = [Point::new(-1.0, 0.0), Point::new(1.0, 0.0)];
        let line2 = [Point::new(0.0, -1.0), Point::new(0.0, 1.0)];
        assert_eq!(segment_intersection(line1, line2), Some(Point::new(0.0, 0.0)));
    }

    #[test]
    fn tee_segments_intersect() {
        let line1 = [Point::new(-1.0, 0.0), Point::new(1.0, 0.0)];
        let line2 = [Point::new(0.0, 0.0), Point::new(0.0, 1.0)];
        assert_eq!(segment_intersection(line1, line2), Some(Point::new(0.0, 0.0)));
    }

    #[test]
    fn simple_segments_nonintersect() {
        let line1 = [Point::new(-1.0, 10.0), Point::new(1.0, 10.0)];
        let line2 = [Point::new(0.0, -1.0), Point::new(0.0, 1.0)];
        assert_eq!(segment_intersection(line1, line2), None);
    }
}
