use chrono::Timelike;
use macroquad::prelude::*;
use genmap::GenMap;

use stales_geom_viewer::{
    common_traits::*,
    geom::{self, *},
    point::Point,
};
use euclid::default::Vector2D;

use std::{
    cmp::{Ord, Ordering},
    collections::HashMap,
    default::Default,
    fmt::Debug,
    fs::File, io::Write,
    iter::{self, Iterator},
    time::Instant,
    cell::RefCell,
};

mod bot;
mod grid;
use bot::Bot;
use grid::Grid;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DrawingState {
    Ground,
    Drawing,
    Erasing,
}

type Color = macroquad::color::Color;

#[derive(Debug)]
enum Object {
    Point(geom::Vertex),
    CircleObj(geom::Circle),
    LineObj(geom::Line2D),
    PolyObj(geom::Polygon),
    GridObj(Grid),
    BotObj(RefCell<Bot>),
}

impl Draw for Object {
    fn draw(&self) {
        match self {
            Object::Point(p) => p.draw(),
            Object::CircleObj(c) => c.draw(),
            Object::LineObj(l) => l.draw(),
            Object::PolyObj(p) => p.draw(),
            Object::GridObj(g) => g.draw(),
            Object::BotObj(b) => b.borrow().draw(),
        }
    }
    fn vertices(&self) -> Vec<geom::Vertex> {
        match self {
            Object::Point(p) => p.vertices(),
            Object::CircleObj(c) => c.vertices(),
            Object::LineObj(l) => l.vertices(),
            Object::PolyObj(p) => p.vertices(),
            Object::GridObj(g) => g.vertices(),
            Object::BotObj(b) => b.borrow().vertices(),
        }
    }
}

#[derive(Debug)]
struct State {
    pub objects: GenMap<Object>,
    pub clear_color: Color,
    pub startup: Instant,
    pub prev_mouse_pos: (f32, f32),
    pub logfile: std::fs::File,

    pub grids: Vec<genmap::Handle>,
    pub drawing_state: DrawingState,
    pub sel_cell: Option<usize>,
    pub bots: Vec<genmap::Handle>,
}

impl Default for State {
    fn default() -> Self {
        use chrono;
        let objects = GenMap::<Object>::with_capacity(1000);
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
            sel_cell: None,
            bots: vec![],
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
                    Object::BotObj(_b) => None, //Some(b.into_inner().clone() as &dyn Element),
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
        let bots = format!("bots: {:?}", self.bots.iter().map(|b| self.objects.get(*b)).collect::<Vec<_>>());
        format!(r"
num. of lines: {line_cnt}
num. of circles: {circle_cnt}
num. of vertices: {vertex_cnt}
frametime: {frametime}
bots: {bots}
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
                (8, 8)
            )));
        state.grids.push(grid);
    }

    {
        let mut state = state.write().unwrap();
        if let Object::GridObj(grid) = state.objects.get(state.grids[0]).unwrap() {
            let bot = Bot::new(grid, 0, 8*8-1, RED);
            let bot_handle = state.objects.insert(Object::BotObj(RefCell::new(bot)));
            state.bots.push(bot_handle);
        }
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

        let mut recalc = false;
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
                                Some(idx) => {
                                    grid.array[idx] = dstate == DrawingState::Drawing;
                                    state.sel_cell = Some(idx);
                                    recalc = true;
                                },
                                None => (),
                            }
                        }
                        else { unreachable!(); }
                    }
                } else {
                    state.sel_cell = None;
                }
            }
        }

        if let Some(sel_cell) = state.sel_cell {
            if let Object::GridObj(grid) = state.objects.get(state.grids[0]).unwrap() {
                for i in 0..grid.array.len() {
                    let dist = grid.chebyshev_distance(sel_cell, i);
                    let pos = grid.idx_xy(i).unwrap();
                    let pos = (pos.0 as f32, pos.1 as f32);
                    draw_text(&dist.to_string(), pos.0, pos.1, 24.0, WHITE);
                    let pos = (pos.0 + grid.cell_dims().0 as f32/2.0, pos.1 + grid.cell_dims().1 as f32/2.0);
                    draw_circle(pos.0, pos.1, 2.0, WHITE);
                }
            } else {
                unreachable!();
            }
        }

        if recalc {
            let grid_handle = state.grids[0];
            for bot in state.bots.clone() {
                let objects = &mut state.objects;
                if let Object::BotObj(bot) = objects.get(bot).unwrap() {
                    if let Object::GridObj(grid) = objects.get(grid_handle).unwrap() {
                        bot.borrow_mut().recalc_path(grid);
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
