use euclid::{*, default::Vector2D, vec2};
use macroquad::prelude::{*};
use crate::common_traits::*;

#[derive(Clone)]
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
