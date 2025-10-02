use macroquad::prelude::*;
use std::{
    default::Default, time::Instant,
    iter::Iterator,
    io::Write,
};

use stales_geom_viewer::common_traits::*;
use euclid::{default::{Box2D, Vector2D}, *};

type Color = macroquad::color::Color;

pub use stales_geom_viewer::geom::{self, *, Vertex};

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
    pub objects: Vec<Object>,
    pub clear_color: Color,
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
    fn all_elements(&self) -> impl Iterator<Item = &dyn Element> {
        self.objects.iter().flat_map(|x| {
            match x {
                Object::Point(_p) => None,
                Object::CircleObj(c) => Some(c as &dyn Element),
                Object::LineObj(l) => Some(l as &dyn Element),
                Object::PolyObj(_p) => None,
            }
        })
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

#[derive(Debug)]
enum LogTag {
    Mouse, FrameTime, Select,
}

#[macroquad::main("Voronoi")]
async fn main() {
    let mut state: State = Default::default();
    let startup = Instant::now();
    let mut prev_mouse_pos = mouse_position();

    let mut logfile = std::fs::File::create("./log.txt").expect("can't create \"./log.txt\" log file!");

    loop {
        let tick_time = {
            let t = Instant::now().duration_since(startup);
            (t.as_secs(), t.subsec_nanos())
        };

        let mut log_line = |tag: LogTag, msg: &str| {
            writeln!(&mut logfile, "({tag:?}) [{}s{}ns]: {msg}", tick_time.0, tick_time.1).expect("couldn't write log line")
        };

        log_line(LogTag::FrameTime, &format!("{} / {}", get_frame_time(), get_fps()));

        if is_quit_requested() { break }
        clear_background(state.clear_color);

        for object in &state.objects {
            object.draw();
        }

        { // Mouse handling
            let mouse_pos = mouse_position();
            if mouse_pos != prev_mouse_pos {
                log_line(LogTag::Mouse, &format!("pos {},{}", mouse_pos.0, mouse_pos.1));
                prev_mouse_pos = mouse_pos;
            }

            if is_mouse_button_pressed(MouseButton::Left) {
                log_line(LogTag::Mouse, &format!("clicked {},{}", mouse_pos.0, mouse_pos.1));

                for element in state.all_elements() {
                    if element.contains_point(&Vector2D::new(mouse_pos.0, mouse_pos.1)) {
                        log_line(LogTag::Select, &format!("selected {:?}", element));
                    }
                }

            }
        }

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        if is_key_released(KeyCode::R) {
            println!("{}", state.text_digest())
        }
        next_frame().await
    }
}
