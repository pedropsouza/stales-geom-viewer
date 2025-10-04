use macroquad::prelude::*;
use std::{
    default::Default, time::Instant,
    iter::Iterator,
    io::Write,
    ops::Range,
    cmp::{Ordering, Ord}
};

use stales_geom_viewer::{
    utils,
    common_traits::*,
    geom::{self, *, Vertex},
};
use euclid::{default::{Box2D, Vector2D}, *};

type Color = macroquad::color::Color;

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

#[derive(PartialEq, Eq, Ord, Debug)]
struct PointPrio<T: Ord + Eq> {
    pub x: T,
    pub y: T,
}

impl<T: Ord + Eq> PartialOrd for PointPrio<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.y.partial_cmp(&other.y) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        self.x.partial_cmp(&other.x)
    }
}

fn voronoi(points: &Vec<Vector2D<u64>>) -> Vec<Object> {
    use priority_queue::PriorityQueue;

    let item_prio_iter = points
        .into_iter()
        .cloned()
        .map(|p| { (p, PointPrio { x: p.x, y: p.y }) });
    let queue = PriorityQueue::<Vector2D<u64>, PointPrio<u64>>::from_iter(item_prio_iter);
    println!("debug {:?}", queue);
    vec![]
}

fn cheating(points: &Vec<Vector2D<f32>>) -> Vec<Object> {
    let dcel = voronoi::voronoi(
        points.iter().map(|v| voronoi::Point::new(v.x.into(), v.y.into())).collect(),
        1000.0,
    );
    let lines = voronoi::make_line_segments(&dcel);
    lines.into_iter().map(|pts| {
        Object::LineObj(geom::Line2D {
            a: geom::Vertex::new(
                pts[0].x.0 as f32, pts[0].y.0 as f32,
                Some(utils::random_color())),
            b: geom::Vertex::new(
                pts[1].x.0 as f32, pts[1].y.0 as f32,
                None),
            thickness: 1.0,
        })
    }).collect()
}

#[macroquad::main("Voronoi")]
async fn main() {
    let mut state: State = Default::default();
    let startup = Instant::now();
    let mut prev_mouse_pos = mouse_position();

    let mut logfile = std::fs::File::create("./log.txt").expect("can't create \"./log.txt\" log file!");

    let bounds = (0.0..screen_width(), 0.0..screen_height());
    let random_float_points = utils::random_points(100, bounds.clone());
    let random_quant_points = utils::quantize_points(&random_float_points, bounds.clone());
    //let voronoi_lines = voronoi(&random_quant_points);
    let voronoi_lines = cheating(&random_float_points);
    state.objects.extend(voronoi_lines.into_iter());

    for p in &random_float_points {
        state.add_circle(geom::Circle {
            center: Vertex::new(p.x, p.y,
                                Some(utils::random_color())),
            radius: 5.0,
        });
    }

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
