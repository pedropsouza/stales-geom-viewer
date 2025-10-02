use macroquad::prelude::*;
use std::{
    default::Default, time::Instant,
    iter::Iterator,
    ops::Range,
    io::Write,
    fmt::Display,
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

#[derive(Debug)]
enum LogTag {
    Mouse, FrameTime, Select,
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let mut state: State = Default::default();
    let startup = Instant::now();
    let mut prev_mouse_pos = mouse_position();

    state.add_line(Line2D {
        a: Vertex::new(40.0, 40.0, Some(BLUE)),
        b: Vertex::new(100.0, 100.0, None),
        thickness: 1.0,
    });

    state.add_circle(geom::Circle { center: Vertex::new(screen_width() - 30.0, screen_height() - 30.0, Some(YELLOW)), radius: 15.0 });
    state.add_poly(random_polar_poly(Vertex::new(screen_width()/2.0, screen_height()/2.0, None), 50, 20.0..60.0));
    state.add_poly(rect_poly(
        Vector2D::new(screen_width()/2.0 - 60.0, screen_height()/2.0 - 30.0),
        Vector2D::new(screen_width()/2.0 + 60.0, screen_height()/2.0 + 30.0),
        GREEN,
        YELLOW,
        4.0));
    let rect_idx = state.objects.len() - 1;

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

        if let Object::LineObj(line) = state.objects.get_mut(0).unwrap() {
            line.a.pos += Vector2D::new(10.0*get_frame_time(), 0.0);
        }

        if let Object::PolyObj(rect) = state.objects.get_mut(rect_idx).unwrap() {
            let center = rect.verts.iter().fold(Vector2D::zero(), |lhs,rhs| lhs + rhs.pos).component_div(Vector2D::splat(rect.verts.len() as f32));
            let delta = Vector2D::from(mouse_position()) - center;
            for vert in rect.verts.iter_mut() {
                vert.pos += delta * 2.0 * get_frame_time()
            }
        }

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        if is_key_released(KeyCode::R) {
            println!("{}", state.text_digest())
        }
        next_frame().await
    }
}

fn rect_poly(ne: Vector2D<f32>, sw: Vector2D<f32>, fill_clr: Color, stroke_clr: Color, stroke_thickness: f32) -> Polygon {
    Polygon {
        verts: vec![
            Vertex::new(ne.x, ne.y, None),
            Vertex::new(ne.x, sw.y, None),
            Vertex::new(sw.x, sw.y, None),
            Vertex::new(sw.x, ne.y, None),
        ],
        edges: vec![
            (0,1,stroke_clr),
            (1,2,stroke_clr),
            (2,3,stroke_clr),
            (3,0,stroke_clr),
        ],
        edge_thickness: stroke_thickness,
        faces: vec![
            (0,1,2,fill_clr),
            (0,2,3,fill_clr),
        ],
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
