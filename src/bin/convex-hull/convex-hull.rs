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
    pub sorted_pts: Vec<Point>,
    pub convex_hull_poly: genmap::Handle,
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
            convex_hull_poly: chph,
            sorted_pts: vec![],
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

    pub fn recalc_sorted_pts(&mut self) -> &Vec<Point> {
        self.sorted_pts = self.objects.iter().flat_map(|x| {
            self.objects.get(x).and_then(|obj| {
                match obj {
                    Object::CircleObj(c) => Some(vec![Point::new(c.center.pos.x as f64, c.center.pos.y as f64)]),
                    Object::PolyObj(p) => {
                        if x == self.convex_hull_poly { None } // skip the convex hull points
                        else { Some(p.verts.iter().map(|v| {
                            Point::new(v.pos.x as f64, v.pos.y as f64)
                        }).collect()) }
                    },
                    _ => None,
                }
            })
        }).flatten().collect();
        self.sorted_pts.sort();
        &self.sorted_pts
    }

    pub fn convex_hull_report(&self) -> String {
        let convex_hull = self.objects.get(self.convex_hull_poly).unwrap();
        if let Object::PolyObj(convex_hull) = convex_hull {
            let points_on_hull = convex_hull.edges.len()+1;
            let points_inside_hull = self.sorted_pts.len() - points_on_hull;
            let hull_centroid = convex_hull.edges.iter()
                                                 .map(|e| convex_hull.verts[e.0].pos)
                                                 .sum::<Vector2D<f32>>()
                                                 .component_div(Vector2D::splat(points_on_hull as f32));
            let points_centroid = {
                let t = self.sorted_pts
                            .iter()
                            .fold(Point::new(0.0,0.0), |acc, v| Point::new(acc.x() + v.x(), acc.y() + v.y()));
                Vector2D::new(t.x() as f32 / self.sorted_pts.len() as f32, t.y() as f32 / self.sorted_pts.len() as f32)
            };

            draw_circle(hull_centroid.x, hull_centroid.y, 10.0, MAGENTA);
            draw_circle(points_centroid.x, points_centroid.y, 10.0, RED);
            format!("convex hull has {} points on the hull perimeter and {} points inside.
The geometric center of the hull perimeter is {:?}, while the geometric center of all points is {:?}",
                    points_on_hull, points_inside_hull, hull_centroid, points_centroid)
        } else {
            panic!();
        }
    }

    pub fn recalc_convex_hull(&mut self) {
        let input = self.recalc_sorted_pts().clone();
        let out_poly = self.objects.get_mut(self.convex_hull_poly).unwrap();
        if let Object::PolyObj(ref mut out_poly) = out_poly {
            let ref mut out_verts = out_poly.verts;
            let ref mut out_edges = out_poly.edges;
            // wipe previous result;
            out_verts.clear();
            out_edges.clear();

            if input.len() < 3 {
                return;
            }

            let mut upper = vec![0, 1];

            for i in 2..input.len() {
                upper.push(i);

                loop {
                    if upper.len() < 3 { break; }
                    let p_lll = input[upper[upper.len()-3]];
                    let p_ll = input[upper[upper.len()-2]];
                    let p_l = input[upper[upper.len()-1]];
                    let v_a = glamVec2_from_point(p_lll - p_ll);
                    let v_b = glamVec2_from_point(p_l - p_ll);
                    let last_3_make_right = (v_a.x*v_b.y - v_b.x*v_a.y) > 0.0;
                    if last_3_make_right {
                        break;
                    }
                    upper.remove(upper.len()-2);
                }
            }

            let mut lower = vec![input.len()-1, input.len()-2];

            for i in (0..input.len()-2).rev() {
                lower.push(i);

                loop {
                    if lower.len() < 3 { break; }
                    let p_lll = input[lower[lower.len()-3]];
                    let p_ll = input[lower[lower.len()-2]];
                    let p_l = input[lower[lower.len()-1]];
                    let v_a = glamVec2_from_point(p_lll - p_ll);
                    let v_b = glamVec2_from_point(p_l - p_ll);
                    let last_3_make_right = (v_a.x*v_b.y - v_b.x*v_a.y) > 0.0;
                    if last_3_make_right {
                        break;
                    }
                    lower.remove(lower.len()-2);
                }
            }

            *out_verts = input.iter().map(|pos| {
                Vertex::new(pos.x() as f32, pos.y() as f32, None)
            }).collect();


            let lower_edges = lower
                .iter()
                .cloned()
                .skip(1) // dedup the lowermost vert
                .take(lower.len().saturating_sub(2 + 1)) // and the uppermost
                .zip(lower.iter().cloned().skip(1 + 1).take(lower.len() - 2))
                .chain(iter::once((lower[lower.len()-2], upper[0])));
            let upper_edges = upper
                .iter()
                .cloned()
                .take(upper.len()-1)
                .zip(upper.iter().cloned().skip(1).take(upper.len()-1))
                .chain(iter::once((upper[upper.len()-1], lower[1])));

            *out_edges = upper_edges.chain(lower_edges).map(|e| (e.0, e.1, utils::random_color())).collect();
        } else {
            panic!();
        }
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
        state.recalc_convex_hull();
        let after = Instant::now();

        let d = after - before;
        let point_count = state.sorted_pts.len();
        log_line(&mut state, &std::time::Duration::from_secs(0), LogTag::Timing,
                 &format!("recalc_convex_hull took {}s{}ns for {} points", d.as_secs(), d.subsec_nanos(), point_count));
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
        if is_key_down(KeyCode::N) {
            for (i,point) in state.sorted_pts.iter().enumerate() {
                draw_text(&i.to_string(), point.x() as f32, point.y() as f32, 20.0, WHITE);
            }
        }

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
                state.recalc_convex_hull();
            }
        }

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        if is_key_released(KeyCode::R) {
            println!("{}", state.text_digest())
        }
        next_frame().await
    }
}
