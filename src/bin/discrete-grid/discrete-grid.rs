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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DrawingState {
    Ground,
    Drawing,
    Erasing,
}

#[derive(Debug)]
struct Grid {
    pub topleft: Point,
    pub botright: Point,
    pub array: Vec<bool>,
    size: (usize, usize),
    strides: (f64, f64),
    verts: Vec<Vertex>,
    clr: Color,
}

impl Grid {
    pub fn new(topleft: Point, botright: Point, clr: Color, size: (usize, usize)) -> Self {
        let mut verts = Vec::with_capacity(size.0*2 + size.1*2 + 2);
        let deltas = (botright.x() - topleft.x(), botright.y() - topleft.y());
        let strides = (deltas.0/(size.0 as f64), deltas.1/(size.1 as f64));
        for i in 0..(size.0 + 1) {
            let x = topleft.x() + (i as f64) * strides.0;
            let y1 = topleft.y();
            let y2 = botright.y();
            verts.push(Point::new(x,y1));
            verts.push(Point::new(x,y2));
        }
        for i in 0..(size.1 + 1) {
            let y = topleft.y() + (i as f64) * strides.1;
            let x1 = topleft.x();
            let x2 = botright.x();
            verts.push(Point::new(x1,y));
            verts.push(Point::new(x2,y));
        }

        Self {
            topleft, botright, clr, size, strides,
            verts: verts.into_iter().map(|p| Vertex::new(p.x() as f32, p.y() as f32, None)).collect(),
            array: [false].into_iter().cycle().take(size.0*size.1).collect(),
        }
    }

    pub fn quantize_xy(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let xu = x;
        let yu = y;

        if xu < 0.0 || yu < 0.0 { return None; }
        
        let xi = (xu / self.strides.0) as usize;
        let yi = (yu / self.strides.1) as usize;

        println!("{x}x{y} quantizes to {xi}x{yi}");
        Some((xi, yi))
    }

    pub fn xy_idx(&self, x: f64, y: f64) -> Option<usize> {
        let relx = x - self.topleft.x();
        let rely = y - self.topleft.y();
        let quantized = self.quantize_xy(relx,rely);
        quantized.and_then(|(xi,yi)| {
            if xi >= self.size.0
            || yi >= self.size.1
            { return None; }

            Some(xi + yi*self.size.0)
        })
    }

    pub fn idx_xy(&self, idx: usize) -> Option<(f64,f64)> {
        if idx < self.array.len() {
            Some((
                (idx as f64 % self.size.0 as f64) * self.strides.0 + self.topleft.x(),
                (idx as f64 / self.size.0 as f64) * self.strides.1 + self.topleft.y()
            ))
        } else { None }
    }
    pub fn probe_xy(&self, x: f64, y: f64) -> Option<bool> {
        self.xy_idx(x,y).map(|idx| self.array[idx])
    }

    pub fn idx_box(&self, idx: usize) -> Option<(Point, Point)> {
        self.idx_xy(idx).and_then(|(x,y)| self.xy_box(x,y))
    }

    pub fn xy_box(&self, x: f64, y: f64) -> Option<(Point, Point)> {
        let relx = x - self.topleft.x();
        let rely = y - self.topleft.y();
        let quantized = self.quantize_xy(relx,rely);
        quantized.and_then(|(xi,yi)| {
            if xi >= self.size.0 || yi >= self.size.1 {
                return None;
            }
            Some((
                Point::new((xi as f64)*self.strides.0 + self.topleft.x(), (yi as f64)*self.strides.1 + self.topleft.y()),
                Point::new(((xi + 1) as f64)*self.strides.0 + self.topleft.x(), ((yi + 1) as f64)*self.strides.1 + self.topleft.y()),
            ))
        })
    }
}

impl Draw for Grid {
    fn draw(&self) {
        for vs in self.verts.chunks(2) {
            draw_line(vs[0].pos.x, vs[0].pos.y,
                      vs[1].pos.x, vs[1].pos.y,
                      1.0, self.clr);
        }

        for (idx, _) in self.array.iter().enumerate().filter(|(_,x)| **x) {
            if let Some((topl,botr)) = self.idx_box(idx) {
                let s = botr - topl;
                let (w,h) = (s.x() as f32, s.y() as f32);
                draw_rectangle(topl.x() as f32, topl.y() as f32, w, h, self.clr);
            }
        }
    }

    fn vertices(&self) -> Vec<Vertex> {
        self.verts.clone()
    }
}

impl Select for Grid {
    fn compute_aabb(&self) -> euclid::default::Box2D<f32> {
        euclid::default::Box2D::new(
            euclid::default::Point2D::new(self.topleft.x() as f32, self.topleft.y() as f32),
            euclid::default::Point2D::new(self.botright.x() as f32, self.botright.y() as f32),
            )
    }

    fn sample_signed_distance_field(&self, global_sample_point: &Vector2D<f32>) -> f32 {
        // Rectangle SDF from iq (of course)
        // float sdBox( in vec2 p, in vec2 b )
        // {
        //     vec2 d = abs(p)-b;
        //     return length(max(d,0.0)) + min(max(d.x,d.y),0.0);
        // }
        let midpoint  = (self.topleft + self.botright) / 2.0;
        let point_rel = Point::new(global_sample_point.x as f64, global_sample_point.y as f64) - midpoint;
        let halfdelta = (self.botright - self.topleft) / 2.0;
        let d = Point::new(point_rel.x().abs(), point_rel.y().abs()) - halfdelta;
        let length = Point::new(d.x().max(0.0), d.y().max(0.0)).magnitude();
        let dist = (length as f32) + (d.x().max(d.y())).min(0.0) as f32;
        dist
    }
}

type Color = macroquad::color::Color;

enum Object {
    Point(geom::Vertex),
    CircleObj(geom::Circle),
    LineObj(geom::Line2D),
    PolyObj(geom::Polygon),
    GridObj(Grid)
}

impl Draw for Object {
    fn draw(&self) {
        match self {
            Object::Point(p) => p.draw(),
            Object::CircleObj(c) => c.draw(),
            Object::LineObj(l) => l.draw(),
            Object::PolyObj(p) => p.draw(),
            Object::GridObj(g) => g.draw(),
        }
    }
    fn vertices(&self) -> Vec<geom::Vertex> {
        match self {
            Object::Point(p) => p.vertices(),
            Object::CircleObj(c) => c.vertices(),
            Object::LineObj(l) => l.vertices(),
            Object::PolyObj(p) => p.vertices(),
            Object::GridObj(g) => g.vertices(),
        }
    }
}

struct State {
    pub objects: GenMap<Object>,
    pub clear_color: Color,
    pub startup: Instant,
    pub prev_mouse_pos: (f32, f32),
    pub logfile: std::fs::File,

    pub grids: Vec<genmap::Handle>,
    pub drawing_state: DrawingState,
}

impl Default for State {
    fn default() -> Self {
        use chrono;
        let mut objects = GenMap::<Object>::with_capacity(1000);
        let cur_time = chrono::Local::now();
        let log_name = format!("./log-{}-{}-{}.txt", cur_time.hour(), cur_time.minute(), cur_time.second());
        Self {
            objects: objects,
            clear_color: BLACK,
            startup: Instant::now(),
            prev_mouse_pos: mouse_position(),
            logfile: std::fs::File::create(log_name).expect("can't create \"./log.txt\" log file!"),
            grids: vec![],
            drawing_state: DrawingState::Ground,
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
                    Object::GridObj(g) => Some(g as &dyn Element),
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
        format!(r"
num. of lines: {line_cnt}
num. of circles: {circle_cnt}
num. of vertices: {vertex_cnt}
frametime: {frametime}
")
    }

}

fn glamVec2_from_point(p: Point) -> glam::Vec2 {
    glam::Vec2::new(p.x() as f32, p.y() as f32)
}

#[derive(Debug)]
enum LogTag {
    Mouse, FrameTime, Select, Timing,
}

#[macroquad::main("discrete-grid")]
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
    
    {
        let mut state = state.write().unwrap();
        let grid = state.objects.insert(Object::GridObj(
            Grid::new(
                Point::new(WIDTH as f64/8.0, HEIGHT as f64/8.0),
                Point::new(WIDTH as f64 * 7.0/8.0, HEIGHT as f64 * 7.0/8.0),
                WHITE,
                (30, 30)
            )));
        state.grids.push(grid);
    }

    let mut state = state.write().unwrap();

    { // calculate initial convex hull with timing
        let before = Instant::now();
        let after = Instant::now();

        // let d = after - before;
        // let point_count = state.sorted_pts.len();
        // log_line(&mut state, &std::time::Duration::from_secs(0), LogTag::Timing,
                 // &format!("recalc_convex_hull took {}s{}ns for {} points", d.as_secs(), d.subsec_nanos(), point_count));
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

        { // Mouse handling
            let mouse_pos = mouse_position();
            if mouse_pos != state.prev_mouse_pos {
                log_line(&mut state, LogTag::Mouse, &format!("pos {},{}", mouse_pos.0, mouse_pos.1));
                state.prev_mouse_pos = mouse_pos;
            }

            state.drawing_state
                = match (is_mouse_button_down(MouseButton::Left),
                         is_mouse_button_down(MouseButton::Right))
            {
                (true, _) => DrawingState::Drawing,
                (_, true) => DrawingState::Erasing,
                (false, false) => DrawingState::Ground,
            };

            if state.drawing_state != DrawingState::Ground {
                log_line(&mut state, LogTag::Mouse, &format!("clicked {},{}", mouse_pos.0, mouse_pos.1));

                let mut hit_elem = None;
                for (handle,element) in state.all_elements() {
                    if element.contains_point(&Vector2D::new(mouse_pos.0, mouse_pos.1)) {
                        hit_elem = Some(handle);
                        break;
                    }
                }
                
                if let Some(elem) = hit_elem {
                    log_line(&mut state, LogTag::Select, &format!("selected {:?}", elem));

                    let grid_handle = state.grids.iter().find(|x| **x == elem).cloned();
                    if let Some(grid_handle) = grid_handle {
                        let dstate = state.drawing_state;
                        let grid = state.objects.get_mut(grid_handle);
                        if let Some(Object::GridObj(ref mut grid)) = grid {
                            match grid.xy_idx(mouse_pos.0 as f64, mouse_pos.1 as f64) {
                                Some(idx) => grid.array[idx] = dstate == DrawingState::Drawing,
                                None => (),
                            }
                        }
                        else { unreachable!(); }
                    }
                } else {
                    // nothing
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
