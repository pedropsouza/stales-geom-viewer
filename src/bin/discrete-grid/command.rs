use std::fmt::{self, Debug, Display};
use dyn_clone::DynClone;
use crate::{obstacle::{self, Factory}, State};

#[derive(Debug)]
pub struct CommandError(String);

impl Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub type CommandResult<T> = Result<Option<Box<dyn Command<T>>>, CommandError>;

pub trait Command<T>: fmt::Debug + DynClone {
    fn run(&self, state: &mut T) -> CommandResult<T>;
}

dyn_clone::clone_trait_object!(Command<State>);

#[derive(Debug)]
pub enum ValueOrKey<V: Debug, K: Clone + Debug> {
    Value(V),
    Key(K),
}

#[derive(Debug, Clone)]
pub struct AddObstacle {
    coords: (usize, usize),
}

impl AddObstacle {
    pub fn new(coords: (usize, usize)) -> Self {
        Self { coords }
    }
}

impl Command<State> for AddObstacle {
    fn run(&self, state: &mut State) -> CommandResult<State> {
        let idx = state.grid.coords_idx(self.coords)
                            .ok_or(CommandError("invalid coordinates".to_string()))?;
        if state.grid.idx(idx).is_some() {
            return Err(CommandError("cell already occupied".to_string()));
        }
        let mut fact = obstacle::factories::Wall::new(idx);
        let obstacle = fact.new_object(&*state.grid)
                                   .map_err(|e| CommandError(format!("{e:?}")))?;
        let idx = state.grid.push_obstacle(obstacle);
        Ok(Some(Box::new(RemoveObstacle {
            obstacle_key: idx
        })))
    }
}

#[derive(Debug, Clone)]
pub struct RemoveObstacle {
    obstacle_key: usize,
}

impl RemoveObstacle {
    pub fn new(obstacle_key: usize) -> Self {
        Self { obstacle_key }
    }
}

impl Command<State> for RemoveObstacle {
    fn run(&self, state: &mut State) -> CommandResult<State> {
        let obstacle = state.grid.remove_obstacle(self.obstacle_key);
        let idx = obstacle.cells().get(0).ok_or(CommandError("obstacle doesn't occupy any cells".to_string()))?; // hardcoded for now
        Ok(Some(Box::new(AddObstacle {
            coords: state.grid.idx_coords(*idx).ok_or(CommandError("invalid coordinates".to_string()))?
        })))
    }
}

#[derive(Debug, Clone)]
pub struct StepForward {}

impl StepForward {
    pub fn new() -> Self {
        Self {  }
    }
}

impl Command<State> for StepForward {
    fn run(&self, state: &mut State) -> CommandResult<State> {
        //state.tick++;
        Ok(Some(Box::new(StepBackwards {})))
    }
}

#[derive(Debug, Clone)]
pub struct StepBackwards {}

impl StepBackwards {
    pub fn new() -> Self {
        Self {  }
    }
}

impl Command<State> for StepBackwards {
    fn run(&self, state: &mut State) -> CommandResult<State> {
        //state.tick--;
        Ok(Some(Box::new(StepForward {})))
    }
}

#[derive(Debug, Clone)]
pub struct AddBot {
    pos: (usize, usize),
    dest: (usize, usize),
}

impl AddBot {
    pub fn new(pos: (usize, usize), dest: (usize, usize)) -> Self {
        Self { pos, dest }
    }
}

impl Command<State> for AddBot {
    fn run(&self, state: &mut State) -> CommandResult<State> {
        use crate::{bot, Object};
        use stales_geom_viewer::utils::random_color;
        use std::cell::RefCell;
        use bot::PathfinderDecorator;
        let (origin_idx, dest_idx) =
            (state.grid.coords_idx(self.pos).ok_or(CommandError("invalid coordinates for origin".to_string()))?,
             state.grid.coords_idx(self.dest).ok_or(CommandError("invalid coordinates for origin".to_string()))?);
        let pathfinder = bot::DebugPathFinder::wrap(Box::new(bot::BasePathfinder {}));
        let bot = bot::Bot::new(&state.grid, pathfinder, origin_idx, dest_idx, random_color());
        let bot_handle = state.objects.insert(Object::BotObj(RefCell::new(bot)));
        state.bots.push(bot_handle);
        Ok(Some(Box::new(RemoveBot::new(bot_handle))))
    }
}

#[derive(Debug, Clone)]
pub struct RemoveBot {
    bot_handle: genmap::Handle,
}

impl RemoveBot {
    pub fn new(bot_handle: genmap::Handle) -> Self {
        Self { bot_handle }
    }
}

impl Command<State> for RemoveBot {
    fn run(&self, state: &mut State) -> CommandResult<State> {
        use crate::Object;
        let bot = state.objects.remove(self.bot_handle).ok_or(CommandError("no such bot".to_string()))?;
        state.bots.retain(|handle| *handle != self.bot_handle);
        if let Object::BotObj(bot) = bot {
            let bot = bot.borrow();
            let (origin, dest) =
                (state.grid.idx_coords(bot.origin_idx).ok_or(CommandError("invalid coordinates for origin".to_string()))?,
                 state.grid.idx_coords(bot.dest_idx).ok_or(CommandError("invalid coordinates for origin".to_string()))?); 
            Ok(Some(Box::new(
                AddBot::new(origin, dest)
            )))
        } else {
            panic!("bot handle doesn't point to bot object")
        }
    }
}

