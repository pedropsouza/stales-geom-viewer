use euclid::{*, default::Vector2D, vec2};
use macroquad::prelude::{*};
use crate::common_traits::*;

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
    fn compute_aabb(&self) -> crate::Box2D<f32> {
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
    fn compute_aabb(&self) -> crate::Box2D<f32> {
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
