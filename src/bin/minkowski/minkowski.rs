use chrono::Timelike;
use macroquad::{miniquad::native::linux_x11::libx11::Display, prelude::*};
use genmap::GenMap;

use stales_geom_viewer::{
    utils,
    common_traits::*,
    geom::{self, *, Vertex},
    point::Point,
};
use euclid::default::Vector2D;

use std::{
    cmp::{Ord, Ordering}, collections::HashMap, default::Default, fmt::Debug, fs::File, io::Write, iter::{self, Iterator}, time::Instant
};

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



struct State {
    pub objects: GenMap<Object>,
    pub clear_color: Color,
    pub startup: Instant,
    pub prev_mouse_pos: (f32, f32),
    pub logfile: std::fs::File,
    pub ship_handle: genmap::Handle,
}

impl Default for State {
    fn default() -> Self {
        use chrono;
        let mut objects = GenMap::<Object>::with_capacity(1000);
        let chph = objects.insert(Object::PolyObj(Polygon::default()));
        let cur_time = chrono::Local::now();
        let log_name = format!("./log-{}-{}-{}.txt", cur_time.hour(), cur_time.minute(), cur_time.second());
        Self {
            objects: objects,
            clear_color: BLACK,
            startup: Instant::now(),
            prev_mouse_pos: mouse_position(),
            logfile: std::fs::File::create(log_name).expect("can't create \"./log.txt\" log file!"),
            ship_handle: 
        }
    }
}

impl State {
    fn add_line(&mut self, l: geom::Line2D) -> genmap::Handle {
        self.objects.insert(Object::LineObj(l))

    }
    fn add_circle(&mut self, c: geom::Circle) -> genmap::Handle {
        self.objects.insert(Object::CircleObj(c))
    }
    fn add_poly(&mut self, p: geom::Polygon) -> genmap::Handle {
        self.objects.insert(Object::PolyObj(p))
    }
    fn all_elements(&self) -> impl Iterator<Item = (genmap::Handle, &dyn Element)> {
        self.objects.iter().flat_map(|x| {
            self.objects.get(x).and_then(|obj|
                match obj {
                    Object::Point(_p) => None,
                    Object::CircleObj(c) => Some(c as &dyn Element),
                    Object::LineObj(l) => Some(l as &dyn Element),
                    Object::PolyObj(_p) => None,
                }).map(|elem| (x, elem))
            })
    }
    fn text_digest(&self) -> String {
        let line_cnt = self.objects
                           .iter().flat_map(|x| self.objects.get(x))
                           .filter(|x| matches!(x, Object::LineObj(_))).count();
        let circle_cnt = self.objects
                             .iter().flat_map(|x| self.objects.get(x))
                             .filter(|x| matches!(x, Object::CircleObj(_))).count();
        let vertex_cnt = self.objects
                             .iter().flat_map(|x| self.objects.get(x))
                                    .map(Draw::vertices).fold(0usize, |acc,verts| acc + verts.len());
        let frametime = get_frame_time();
        let convex_hull_info = self.convex_hull_report();
        format!(r"
num. of lines: {line_cnt}
num. of circles: {circle_cnt}
num. of vertices: {vertex_cnt}
frametime: {frametime}
hull info: {convex_hull_info}
")
    }

    pub fn default_ship_poly() -> Polygon {
        Polygon { verts: , edges: (), edge_thickness: (), faces: () }
    }
}

fn glamVec2_from_point(p: Point) -> glam::Vec2 {
    glam::Vec2::new(p.x() as f32, p.y() as f32)
}

#[derive(Debug)]
enum LogTag {
    Mouse, FrameTime, Select, Timing,
}

#[macroquad::main("convex-hull")]
async fn main() {
    const WIDTH: f32 = 1800.0;
    const HEIGHT: f32 = 1000.0;
    request_new_screen_size(WIDTH, HEIGHT);
    let state = std::rc::Rc::new(std::sync::RwLock::new(State::default()));

    #[cfg(debug_assertions)]
    {
    stderrlog::new()
        .module(module_path!())
        .verbosity(log::LevelFilter::Trace)
        .init().unwrap();
    }

    let log_line = |state: &mut State, time: &std::time::Duration, tag: LogTag, msg: &str| {
        let secs = time.as_secs();
        let nanosecs = time.subsec_nanos();
        writeln!(&mut state.logfile, "({tag:?}) [{}s{}ns]: {msg}", secs, nanosecs).expect("couldn't write log line")
    };
    
    const CIRCLE_RADIUS: f32 = 4.0;

    let bounds = (1.0*WIDTH/4.0..3.0*WIDTH/4.0,
                  1.0*HEIGHT/4.0..3.0*HEIGHT/4.0);
    let random_float_points = utils::random_points(10, bounds.clone());

    {
        let mut state = state.write().unwrap();
        for p in &random_float_points {
            state.add_circle(geom::Circle {
                center: Vertex::new(p.x, p.y,
                                    Some(utils::random_color())),
                radius: CIRCLE_RADIUS,
            });
        }

        let circles = &[
            (10, 80.0, RED, Vector2D::new(200.0, 200.0)),
            (100, 100.0, YELLOW, Vector2D::new(300.0, 400.0)),
            (6, 40.0, GREEN, Vector2D::new(400.0, 100.0)),
        ];

        for (vcount, radius, clr, center) in circles {
            let mut circle_poly = geom::Polygon::circle(*vcount, *radius, *clr);
            for vert in circle_poly.verts.iter_mut() {
                vert.pos = vert.pos + center;
            }
            state.add_poly(circle_poly);
        }

        let rectangles = &[(Vector2D::new(450.0, 100.0), Vector2D::new(500.0, 300.0))];

        for (a,b) in rectangles {
            state.add_poly(geom::Polygon::rectangle(*a, *b, WHITE, YELLOW));
        }
    }

    let mut state = state.write().unwrap();

    { // calculate initial convex hull with timing
        let before = Instant::now();
        let after = Instant::now();

        let d = after - before;
        log_line(&mut state, &std::time::Duration::from_secs(0), LogTag::Timing,
                 &format!("recalc_convex_hull took {}s{}ns", d.as_secs(), d.subsec_nanos()));
    }
        
    loop {
        let tick_time = Instant::now().duration_since(state.startup);

        let log_line = |state: &mut State, tag: LogTag, msg: &str| {
            log_line(state, &tick_time, tag, msg);
        };

        log_line(&mut state, LogTag::FrameTime, &format!("{} / {}", get_frame_time(), get_fps()));

        if is_quit_requested() { break }
        clear_background(state.clear_color);

        for handle in state.objects.iter() {
            let object = state.objects.get(handle).unwrap();
            object.draw();
        }

        // enumerate input points if requested
        // if is_key_down(KeyCode::N) {
        //     for (i,point) in state.sorted_pts.iter().enumerate() {
        //         draw_text(&i.to_string(), point.x() as f32, point.y() as f32, 20.0, WHITE);
        //     }
        // }

        { // Mouse handling
            let mouse_pos = mouse_position();
            if mouse_pos != state.prev_mouse_pos {
                log_line(&mut state, LogTag::Mouse, &format!("pos {},{}", mouse_pos.0, mouse_pos.1));
                state.prev_mouse_pos = mouse_pos;
            }

            if is_mouse_button_pressed(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right) {
                log_line(&mut state, LogTag::Mouse, &format!("clicked {},{}", mouse_pos.0, mouse_pos.1));

                let mut hit_elem = None;
                let delete = is_mouse_button_pressed(MouseButton::Right);
                for (handle,element) in state.all_elements() {
                    if element.contains_point(&Vector2D::new(mouse_pos.0, mouse_pos.1)) {
                        hit_elem = Some(handle);
                        break;
                    }
                }
                if let Some(elem) = hit_elem {
                    log_line(&mut state, LogTag::Select, &format!("selected {:?}", elem));
                    if delete {
                        state.objects.remove(elem);
                    }
                } else {
                    state.add_circle(geom::Circle {
                        center: Vertex::new(mouse_pos.0, mouse_pos.1, Some(utils::random_color())),
                        radius: CIRCLE_RADIUS,
                    });
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
