use macroquad::prelude::*;
use std::default::Default;
use std::iter::Iterator;
use std::ops::Range;

pub mod common_traits;
pub use common_traits::*;
use euclid::{default::{Box2D, Vector2D}, *};

type Color = macroquad::color::Color;

pub mod geom;
pub use geom::{*, Vertex};

enum Object {
    Point(geom::Vertex),
    CircleObj(geom::Circle),
    LineObj(geom::Line2D),
    PolyObj(geom::Polygon),
}

impl Draw for Object {
    fn draw(&self) {
        match self {
            Object::Point(p) => p.draw(),
            Object::CircleObj(c) => c.draw(),
            Object::LineObj(l) => l.draw(),
            Object::PolyObj(p) => p.draw(),
        }
    }
    fn vertices(&self) -> Vec<geom::Vertex> {
        match self {
            Object::Point(p) => p.vertices(),
            Object::CircleObj(c) => c.vertices(),
            Object::LineObj(l) => l.vertices(),
            Object::PolyObj(p) => p.vertices(),
        }
    }
}

#[derive(Default)]
struct State {
    objects: Vec<Object>,
    clear_color: Color,
}

impl State {
    fn add_line(&mut self, l: geom::Line2D) {
        self.objects.push(Object::LineObj(l));
    }
    fn add_circle(&mut self, c: geom::Circle) {
        self.objects.push(Object::CircleObj(c));
    }
    fn add_poly(&mut self, p: geom::Polygon) {
        self.objects.push(Object::PolyObj(p))
    }
    fn all_drawables(&self) -> impl Iterator<Item = &Object> {
        self.objects.iter()
    }
    fn text_digest(&self) -> String {
        let line_cnt = self.objects.iter().filter(|x| matches!(x, Object::LineObj(_))).count();
        let circle_cnt = self.objects.iter().filter(|x| matches!(x, Object::CircleObj(_))).count();
        let vertex_cnt = self.objects.iter().map(Draw::vertices).fold(0usize, |acc,verts| acc + verts.len());
        let frametime = get_frame_time();
        format!(r"
num. of lines: {line_cnt}
num. of circles: {circle_cnt}
num. of vertices: {vertex_cnt}
frametime: {frametime}
")
    }
}

fn square_grid(spacing: f32, breadth: (Range<f32>, Range<f32>)) -> Vec<Line2D> {
    let mut x = breadth.0.start;
    let mut y = breadth.1.start;

    let mut lines = vec![];
    while breadth.1.contains(&y) {
        let ny = y + spacing;
        while breadth.0.contains(&x) {
            let nx = x + spacing;
            if breadth.0.contains(&nx) {
                lines.push(Line2D {
                    a: Vertex::new(x,y, Some(RED)),
                    b: Vertex::new(nx, y, Some(RED)),
                    thickness: 1.0,
                });
            }
            if breadth.1.contains(&ny) {
                lines.push(Line2D {
                    a: Vertex::new(x, y, Some(YELLOW.with_alpha(0.1))),
                    b: Vertex::new(x, ny, Some(YELLOW)),
                    thickness: 2.0,
                });
            }
            x = nx;
        }
        x = breadth.0.start;
        y = ny;
    };
    lines
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let mut state: State = Default::default();

    state.add_line(Line2D {
        a: Vertex::new(40.0, 40.0, Some(BLUE)),
        b: Vertex::new(100.0, 100.0, None),
        thickness: 1.0,
    });

    for line in square_grid(20.0, ((0.0..screen_width()), (0.0..screen_height()))) {
        state.add_line(line)
    }
    state.add_circle(geom::Circle { center: Vertex::new(screen_width() - 30.0, screen_height() - 30.0, Some(YELLOW)), radius: 15.0 });
    state.add_poly(random_polar_poly(Vertex::new(screen_width()/2.0, screen_height()/2.0, None), 50, 20.0..60.0));
    loop {
        if is_quit_requested() { break }
        clear_background(state.clear_color);

        for drawable in state.all_drawables() {
            drawable.draw();
        }

        if let Object::LineObj(line) = state.objects.get_mut(0).unwrap() {
            line.a.pos += Vector2D::new(10.0*get_frame_time(), 0.0);
        }

        draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        if is_key_released(KeyCode::R) {
            println!("{}", state.text_digest())
        }
        next_frame().await
    }
}

fn random_polar_poly(origin: Vertex, vert_count: usize, dist_range: Range<f32>) -> Polygon {
    let mut verts = Vec::with_capacity(vert_count);
    let mut edges = vec![];
    let mut faces = vec![];

    verts.push(origin.clone());
    for i in 1..vert_count {
        let a = (i as f32)*std::f32::consts::TAU/(vert_count as f32);
        let d = rand::gen_range(dist_range.start, dist_range.end);
        verts.push(Vertex::new(
            origin.pos.x + a.cos()*d,
            origin.pos.y + a.sin()*d,
            Some(Color::from_vec(glam::Vec4::from_array(
                random_color::RandomColor::new().to_f32_rgba_array())))
                ));
        if i > 1 && i < vert_count {
            edges.push((i-1, i, Color::from_vec(glam::Vec4::from_array(
                random_color::RandomColor::new().to_f32_rgba_array()))));
            faces.push((0, i-1, i, Color::from_vec(glam::Vec4::from_array(
                random_color::RandomColor::new().to_f32_rgba_array()))));
        }
    }

    Polygon { verts, edges, edge_thickness: 1.0, faces }
}
