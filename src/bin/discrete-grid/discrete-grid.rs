use chrono::Timelike;
use macroquad::prelude::*;
use genmap::GenMap;

use stales_geom_viewer::{
    common_traits::*,
    geom::{self, *},
    point::Point, utils::random_color,
};
use euclid::{default::Vector2D, num::Floor};

use std::{
    cmp::{Ord, Ordering},
    collections::HashMap,
    default::Default,
    fmt::Debug,
    fs::File, io::Write,
    iter::{self, Iterator},
    time::Instant,
    cell::RefCell,
    env,
};

mod bot;
mod grid;
mod obstacle;
use bot::Bot;
use grid::{Grid, SquareGrid, HexGrid};
use obstacle::Factory;

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
    GridObj(Box<dyn Grid>),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Obstacles(DrawingState),
    Bots(Option<usize>),
    Run((Instant, Instant)),
}

#[derive(Debug)]
struct State {
    pub objects: GenMap<Object>,
    pub clear_color: Color,
    pub startup: Instant,
    pub prev_mouse_pos: (f32, f32),
    pub logfile: std::fs::File,

    pub input_mode: InputMode,
    pub sel_cell: Option<usize>,
    pub bots: Vec<genmap::Handle>,
    pub grid: Box<dyn Grid>,
}

impl State {
    fn new(grid: Box<dyn Grid>) -> Self {
        use chrono;
        let objects = GenMap::<Object>::with_capacity(1000);
        let cur_time = chrono::Local::now();
        let log_name = format!("./log-{}-{}-{}-{}.txt", cur_time.hour(), cur_time.minute(), cur_time.second(), cur_time.nanosecond());
        Self {
            objects: objects,
            clear_color: BLACK,
            startup: Instant::now(),
            prev_mouse_pos: mouse_position(),
            logfile: std::fs::File::create(log_name).expect("can't create \"./log.txt\" log file!"),
            input_mode: InputMode::Obstacles(DrawingState::Ground),
            sel_cell: None,
            bots: vec![],
            grid,
        }
    }


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
                    //Object::GridObj(ref g) => Some(g as &dyn Element),
                    Object::GridObj(_g) => None,
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

    let args: Vec<String> = env::args().collect();
    let grid_dims = (args.get(1).map_or(30, |x| x.parse::<usize>().unwrap()), args.get(2).map_or(30, |x| x.parse::<usize>().unwrap()));
    let bot_count = args.get(3).map_or(10, |x| x.parse::<usize>().unwrap());

    let state = {
        let grid = HexGrid::new(
            Point::new(WIDTH as f64/8.0, HEIGHT as f64/8.0),
            Point::new(WIDTH as f64 * 7.0/8.0, HEIGHT as f64 * 7.0/8.0),
            WHITE,
            grid_dims,
            vec![
                (02, Box::new(obstacle::factories::RandomBoulder::new())),
            ]
        );
        std::rc::Rc::new(std::sync::RwLock::new(State::new(Box::from(grid))))
    };

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
        let mut bots = vec![];
        for _ in 0..bot_count {
            use bot::PathfinderDecorator;
            bots.push(Bot::random_inside(&state.grid, bot::DebugPathFinder::wrap(Box::new(bot::BasePathfinder {}))));
        }
        for bot in bots {
            let bot_handle = state.objects.insert(Object::BotObj(RefCell::new(bot)));
            state.bots.push(bot_handle);
        }
    }

    let mut state = state.write().unwrap();

    {
        let tick_time = Instant::now().duration_since(state.startup);

        let log_line = |state: &mut State, tag: LogTag, msg: &str| {
            log_line(state, &tick_time, tag, msg);
        };


        let before = Instant::now();

        let bots = state.bots.clone();
        let bot_count = bots.len();
        let cell_count = state.grid.size().0 * state.grid.size().1;
        for bot in bots {
            if let Object::BotObj(bot) = state.objects.get(bot).unwrap() {
                bot.borrow_mut().recalc_path(&state.grid);
            }
        }

        let after = Instant::now();
        let d = after - before;
        log_line(&mut state, LogTag::Timing,
                 &format!("recalculating the bot paths took {}s{}ns for {bot_count} bots on a grid with {cell_count}",
                          d.as_secs(), d.subsec_nanos()));

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

        {
            state.grid.draw();
            let a = (WIDTH as f32/8.0, HEIGHT as f32/8.0);
            let b = (WIDTH as f32 * 7.0/8.0, HEIGHT as f32 * 7.0/8.0);
            draw_rectangle_lines(a.0, a.1, b.0-a.0, b.1-a.1, 2.0, WHITE);
        }

        let mut recalc = false;
        { // input handling
            let mouse_pos = mouse_position();
            if mouse_pos != state.prev_mouse_pos {
                log_line(&mut state, LogTag::Mouse, &format!("pos {},{}", mouse_pos.0, mouse_pos.1));
                state.prev_mouse_pos = mouse_pos;
            }

            let grid_interact = |mut state: &mut State, mut grid_func: Box<dyn FnMut(&mut dyn Grid, usize) -> usize>| -> Option<usize> {
                let mut hit_elem = None;
                for (handle,element) in state.all_elements() {
                    if element.contains_point(&Vector2D::new(mouse_pos.0, mouse_pos.1)) {
                        hit_elem = Some(handle);
                        break;
                    }
                }
                
                if let Some(elem) = hit_elem {
                    log_line(&mut state, LogTag::Select, &format!("selected {:?}", elem));
                    None
                } else {
                    // grid is no longer a registered element
                    state.grid.xy_idx(mouse_pos.0 as f64, mouse_pos.1 as f64).map(|idx| grid_func(&mut *state.grid, idx))
                }
            };

            let obstacle_paint = |grid: &mut dyn Grid, idx: usize| {
                match obstacle::factories::Wall::new(idx).new_object(grid) {
                    Ok(obstacle) => { grid.push_obstacle(obstacle); },
                    Err(_e) => {},
                }
                idx
            };
            
            let obstacle_erase = |grid: &mut dyn Grid, idx: usize| {
                if let Some(oidx) = grid.obstacle_idx(idx) {
                    grid.remove_obstacle(oidx);
                }
                idx
            };

            let mut cur_mode =
                match
                (state.input_mode.clone(),
                 is_mouse_button_down(MouseButton::Left), is_mouse_button_down(MouseButton::Right),
                 is_key_down(KeyCode::D), is_key_down(KeyCode::B), is_key_down(KeyCode::Space)) {
                    (InputMode::Obstacles(_), left, right, _, _, _) if left || right =>
                        InputMode::Obstacles(
                            if left { DrawingState::Drawing }
                            else { DrawingState::Erasing }
                        ),
                    (_, _, _, true, _, _) => InputMode::Obstacles(DrawingState::Ground),
                    (_, _, _, _, true, _) => InputMode::Bots(None),
                    (_, _, _, _, _, true) => {
                        let now = Instant::now();
                        InputMode::Run((now, now))
                    },
                    (prev, _,_,_,_,_) => prev,
                };
            let mut bot_creation_info: Option<(usize, usize)> = None;
            
            match cur_mode {
                InputMode::Obstacles(ref mut drawing_state) => {
                    if !is_mouse_button_down(MouseButton::Left) && !is_mouse_button_down(MouseButton::Right) {
                        *drawing_state = DrawingState::Ground;
                        // return could go here, but that skips over important bookkeeping later
                    }

                    if *drawing_state != DrawingState::Ground {
                        log_line(&mut state, LogTag::Mouse, &format!("clicked {},{}", mouse_pos.0, mouse_pos.1));
                        
                        state.sel_cell = grid_interact(
                            &mut state,
                            if *drawing_state == DrawingState::Drawing {
                                Box::new(obstacle_paint)
                            } else {
                                Box::new(obstacle_erase)
                            });
                        recalc = true;
                    }

                    // if let Some(sel_cell) = state.sel_cell {
                    //     if let Object::GridObj(grid) = state.objects.get(state.grids[0]).unwrap() {
                    //         for i in 0..grid.array.len() {
                    //             let dist = grid.chebyshev_distance(sel_cell, i);
                    //             let pos = grid.idx_xy(i).unwrap();
                    //             let pos = (pos.0 as f32, pos.1 as f32);
                    //             draw_text(&dist.to_string(), pos.0, pos.1, 24.0, WHITE);
                    //             let pos = (pos.0 + grid.cell_dims().0 as f32/2.0, pos.1 + grid.cell_dims().1 as f32/2.0);
                    //             draw_circle(pos.0, pos.1, 2.0, WHITE);
                    //         }
                    //     } else {
                    //         unreachable!();
                    //     }
                    // }

                    if recalc {
                        let before = Instant::now();

                        let bots = state.bots.clone();
                        let bot_count = bots.len();
                        let cell_count = state.grid.size().0 * state.grid.size().1;
                        for bot in bots {
                            if let Object::BotObj(bot) = state.objects.get(bot).unwrap() {
                                bot.borrow_mut().recalc_path(&state.grid);
                            }
                        }

                        let after = Instant::now();
                        let d = after - before;
                        log_line(&mut state, LogTag::Timing,
                                 &format!("recalculating the bot paths took {}s{}ns for {bot_count} bots on a grid with {cell_count}",
                                          d.as_secs(), d.subsec_nanos()));

                    }
                },
                InputMode::Bots(ref mut origin_opt) => {
                    match origin_opt {
                        None => {
                            if is_mouse_button_released(MouseButton::Left) {
                                grid_interact(&mut state, Box::new(|_grid: &mut dyn Grid, idx: usize| {
                                    *origin_opt = Some(idx);
                                    idx
                                }));
                            }
                        },
                        Some(origin_idx) => {
                            {
                                let grid = &state.grid;
                                let mut pos = grid.idx_xy(*origin_idx).unwrap();
                                pos.0 += grid.cell_dims().0/2.0;
                                pos.1 += grid.cell_dims().1/2.0;
                                
                                draw_circle_lines(pos.0 as f32, pos.1 as f32, (grid.cell_dims().0.min(grid.cell_dims().1)/2.0) as f32, 2.0, WHITE);
                            }
                            if is_mouse_button_released(MouseButton::Left) {
                                grid_interact(&mut state, Box::new(|_grid: &mut dyn Grid, idx: usize| {
                                    idx
                                })).inspect(|dest_idx| {
                                    bot_creation_info = Some((*origin_idx, *dest_idx));
                                });
                                *origin_opt = None;
                            }
                        },
                    }
                },
                InputMode::Run(ref mut tick) => {
                    tick.1 = Instant::now();
                    for bot_handle in &state.bots {
                        if let Object::BotObj(bot) = state.objects.get(*bot_handle).unwrap() {
                            bot.borrow_mut().anim_step = tick.1.duration_since(tick.0).as_secs_f32();
                        }
                    }
                }
            }
            state.input_mode = cur_mode;

            if let Some((origin_idx, dest_idx)) = bot_creation_info {
                use bot::PathfinderDecorator;
                let pathfinder = bot::DebugPathFinder::wrap(Box::new(bot::BasePathfinder {}));
                let bot = Bot::new(&state.grid, pathfinder, origin_idx, dest_idx, random_color());
                let bot_handle = state.objects.insert(Object::BotObj(RefCell::new(bot)));
                state.bots.push(bot_handle);
            }
        }

        draw_text(&format!("Input mode: {:?}", state.input_mode), 20.0, 20.0, 30.0, DARKGRAY);

        if is_key_released(KeyCode::R) {
            println!("{}", state.text_digest())
        }
        next_frame().await
    }
}
