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
    cell::RefCell, cmp::{Ord, Ordering}, collections::{BTreeMap, HashMap}, default::Default, env, fmt::Debug, fs::File, io::Write, iter::{self, Iterator}, ops::DerefMut, sync::{Arc, RwLock}, time::{self, Instant}
};

mod bot;
mod grid;
mod obstacle;
mod command;
use bot::Bot;
use grid::{Grid, SquareGrid, HexGrid};
use command::Command;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DrawingState {
    Ground,
    Drawing,
    Erasing,
}

type Color = macroquad::color::Color;

#[derive(Debug)]
pub enum Object {
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
pub enum InputMode {
    Obstacles(DrawingState),
    Bots(Option<usize>),
    Run((Instant, Instant)),
}

type CommandHistoryEntry = (Box<dyn Command<State>>, Option<Box<dyn Command<State>>>);

#[derive(Debug, Default)]
pub struct CommandHistory {
    data: BTreeMap<time::Instant, CommandHistoryEntry>,
    time_needle: Option<time::Instant>,
}

impl CommandHistory {
    fn take_undo_command(&mut self) -> Option<Box<dyn Command<State>>> {
        let mut rev_order = self.data.iter().rev();
        let entry = match self.time_needle {
            Some(t_ref) => rev_order.filter(|entry| *entry.0 < t_ref).next(),
            None => rev_order.next(),
        }?;
        let cmd = entry.1.1.clone()?;
        self.time_needle = Some(*entry.0);
        Some(cmd)
    }

    fn take_redo_command(&mut self) -> Option<Box<dyn Command<State>>> {
        let mut order = self.data.iter();
        let entry = match self.time_needle {
            Some(t_ref) => order.filter(|entry| *entry.0 >= t_ref).next(),
            None => order.next(),
        }?;
        let cmd = entry.1.1.clone()?;
        self.time_needle = Some(*entry.0);
        Some(cmd)
    }

    fn push_entry(&mut self, entry: CommandHistoryEntry) {
        if let Some(t_ref) = self.time_needle {
            // invalidate redos
            let first_invalid = self.keys().skip_while(|t| **t <= t_ref).cloned().next();
            if let Some(fi) = first_invalid {
                let _ = self.split_off(&fi);
            }
            self.time_needle = None;
        }
        self.insert(Instant::now(), entry);
    }
}

impl std::ops::Deref for CommandHistory {
    type Target = BTreeMap<time::Instant, CommandHistoryEntry>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for CommandHistory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Debug)]
pub struct State {
    pub objects: GenMap<Object>,
    pub clear_color: Color,
    pub startup: Instant,
    pub prev_mouse_pos: (f32, f32),
    pub logfile: std::fs::File,

    pub input_mode: InputMode,
    pub sel_cell: Option<usize>,
    pub bots: Vec<genmap::Handle>,
    pub grid: Box<dyn Grid>,
    pub command_history: CommandHistory,
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
            command_history: CommandHistory::default(),
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
        let bots = format!("{:#?}", self.bots.iter().map(|b| self.objects.get(*b)).collect::<Vec<_>>());
        let command_history = format!("{:#?}", self.command_history);
        format!(r"
num. of lines: {line_cnt}
num. of circles: {circle_cnt}
num. of vertices: {vertex_cnt}
frametime: {frametime}
bots: {bots}
command history: {command_history}
")
    }

    pub fn log_line(&mut self, tag: LogTag, msg: &str) {
        let tick_time = Instant::now().duration_since(self.startup);
        let secs = tick_time.as_secs();
        let nanosecs = tick_time.subsec_nanos();

        writeln!(&mut self.logfile, "({tag:?}) [{}s{}ns]: {msg}", secs, nanosecs).expect("couldn't write log line")
    }

    fn run_command_inner(&mut self, cmd: &Box<dyn Command<State>>, forget: bool) {
        match cmd.run(self) {
            Ok(undo) => {
                self.log_line(LogTag::Command, &format!("executing command {cmd:?}"));
                if ! forget {
                    self.command_history.push_entry((cmd.clone(), undo));
                }
            },
            Err(e) => {
                self.log_line(LogTag::Error, &format!("couldn't execute command {cmd:?}, reason: {e}"));
            }
        }
    }

    fn run_command(&mut self, cmd: &Box<dyn Command<State>>) {
        self.run_command_inner(cmd, false);
    }

    pub fn undo(&mut self) {
        if let Some(cmd) = self.command_history.take_undo_command() {
            self.run_command_inner(&cmd, true);
        }
    }

    pub fn redo(&mut self) {
        if let Some(cmd) = self.command_history.take_redo_command() {
            self.run_command_inner(&cmd, true);
        }
    }
}

fn glamVec2_from_point(p: Point) -> glam::Vec2 {
    glam::Vec2::new(p.x() as f32, p.y() as f32)
}

#[derive(Debug)]
pub enum LogTag {
    Mouse, FrameTime, Select, Timing,
    Command, Error,
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
        state.log_line(LogTag::Timing,
                 &format!("recalculating the bot paths took {}s{}ns for {bot_count} bots on a grid with {cell_count}",
                          d.as_secs(), d.subsec_nanos()));

    }

    loop {
        state.log_line(LogTag::FrameTime, &format!("{} / {}", get_frame_time(), get_fps()));

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

            if is_key_released(KeyCode::Z)
                && (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
            {
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    state.redo();
                } else {
                    state.undo();
                }
            }
            
            let command: Arc<RwLock<Option<Box<dyn Command<State>>>>> = Arc::new(RwLock::new(None));
            let mouse_pos = mouse_position();
            if mouse_pos != state.prev_mouse_pos {
                state.log_line(LogTag::Mouse, &format!("pos {},{}", mouse_pos.0, mouse_pos.1));
                state.prev_mouse_pos = mouse_pos;
            }

            let grid_interact = |state: &mut State, mut grid_func: Box<dyn FnMut(&mut dyn Grid, usize) -> Option<usize>>| -> Option<usize> {
                let mut hit_elem = None;
                for (handle,element) in state.all_elements() {
                    if element.contains_point(&Vector2D::new(mouse_pos.0, mouse_pos.1)) {
                        hit_elem = Some(handle);
                        break;
                    }
                }
                
                if let Some(elem) = hit_elem {
                    state.log_line(LogTag::Select, &format!("selected {:?}", elem));
                    None
                } else {
                    // grid is no longer a registered element
                    state.grid.xy_idx(mouse_pos.0 as f64, mouse_pos.1 as f64).map(|idx| grid_func(&mut *state.grid, idx)).flatten()
                }
            };

            let obstacle_paint_cmd = command.clone();
            let obstacle_paint = |grid: &mut dyn Grid, idx: usize| {
                if idx > grid.size().0 * grid.size().1 { return None; }
                let mut command = obstacle_paint_cmd.write().unwrap();
                *command = Some(Box::new(command::AddObstacle::new(grid.idx_coords(idx).unwrap())));
                Some(idx)
            };
            
            let obstacle_erase_cmd = command.clone();
            let obstacle_erase = |grid: &mut dyn Grid, idx: usize| {
                if idx > grid.size().0 * grid.size().1 { return None; }
                if let Some(oidx) = grid.obstacle_idx(idx) {
                    let mut command = obstacle_erase_cmd.write().unwrap();
                    *command = Some(Box::new(command::RemoveObstacle::new(oidx)));
                }
                Some(idx)
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
                        state.log_line(LogTag::Mouse, &format!("clicked {},{}", mouse_pos.0, mouse_pos.1));

                        if state.grid.compute_aabb().contains((mouse_pos.0, mouse_pos.1).into()) {
                            state.sel_cell = grid_interact(
                                &mut state,
                                if *drawing_state == DrawingState::Drawing {
                                    Box::new(obstacle_paint)
                                } else {
                                    Box::new(obstacle_erase)
                                });
                            recalc = true;
                        }
                    }

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
                        state.log_line(LogTag::Timing,
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
                                    Some(idx)
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
                                    Some(idx)
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
                let mut command = command.write().unwrap();
                let (origin, dest) =
                    (state.grid.idx_coords(origin_idx).unwrap(),
                     state.grid.idx_coords(dest_idx).unwrap());
                *command = Some(Box::new(command::AddBot::new(origin, dest)));
            }

            let apply_command = command.clone();
            if let Some(ref command) = &*apply_command.write().unwrap() {
                state.run_command(command);
            };
        }


        draw_text(&format!("Input mode: {:?}", state.input_mode), 20.0, 20.0, 30.0, DARKGRAY);

        if is_key_released(KeyCode::R) {
            println!("{}", state.text_digest())
        }
        next_frame().await
    }
}
