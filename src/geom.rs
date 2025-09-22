use euclid::{*, default::Vector2D, default::Size2D, vec2};
use macroquad::prelude::*;
use crate::common_traits::*;

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
}

pub struct Circle {
    pub center: Vertex,
    pub radius: f32,
}

impl Draw for Circle {
    fn draw(&self) {
        draw_circle(self.center.pos.x, self.center.pos.y,
                    self.radius, self.center.clr.unwrap_or(BLACK));
    }
}

pub struct Polygon {
    pub verts: Vec<Vertex>,
}
