use std::collections::HashMap;

use crate::grid::Grid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Surface, Sky,
}

pub trait Obstacle: std::fmt::Debug {
    // fn into_obstacle(self) -> Box<dyn Obstacle> {
    //     Box::new(self)
    // }
    fn blocks(&self, level: &Level, pos: usize) -> bool;
    fn levels_at(&self, pos: usize) -> &'static [Level];
    fn cells(&self) -> &[usize];
    // TODO: fn dynamic_p() -> bool; // doesn't take self, it's here for the type
}

pub trait HeterogenousObstacle: Obstacle {
    fn occlusion_map(&self) -> &HashMap<usize, &'static [Level]>;
    fn levels_at(&self, _: usize) -> &'static [Level] {
        self.occlusion_map().iter().next().unwrap().1
    }
    fn blocks(&self, level: &Level, pos: usize) -> bool {
        self.occlusion_map().get(&pos).map_or(false, |p| p.contains(level))
    }
}

#[derive(Clone, Debug)]
pub enum FactoryError {
    Occluded(Vec<usize>),
    OutOfBounds,
    Unsupported(&'static str),
    Etc(String),
}

pub type FactoryResult = Result<Box<dyn Obstacle>, FactoryError>;

pub trait Factory {
    fn new_object(&mut self, grid: &dyn Grid) -> FactoryResult;
}

#[derive(Debug)]
pub struct Boulder {
    pos: usize,
    occlusion_map: HashMap<usize, &'static [Level]>,
}

impl Boulder {
    pub fn new(pos: usize) -> Self {
        let mut occlusion_map = HashMap::new();
        const LEVELS: &'static [Level] = &[Level::Surface];
        occlusion_map.insert(pos, LEVELS);
        Self { pos, occlusion_map, }
    }
}

impl Obstacle for Boulder {
    fn cells(&self) -> &[usize] {
        std::slice::from_ref(&self.pos)
    }
    fn levels_at(&self, pos: usize) -> &'static [Level] {
        <Self as HeterogenousObstacle>::levels_at(self, pos)
    }
    fn blocks(&self, level: &Level, pos: usize) -> bool {
        <Self as HeterogenousObstacle>::blocks(self, level, pos)
    }
}

impl HeterogenousObstacle for Boulder {
    fn occlusion_map(&self) -> &HashMap<usize, &'static [Level]> {
        &self.occlusion_map
    }
}

#[derive(Debug)]
pub struct Wall {
    start: usize,
    end: usize,
    occlusion_map: HashMap<usize, &'static [Level]>,
    cells: Vec<usize>,
}

impl Wall {
    pub fn new(grid: &dyn Grid, start: usize, end: usize) -> FactoryResult {
        let start_xy = grid.idx_xy(start);
        let end_xy = grid.idx_xy(end);

        match (start_xy, end_xy) {
            (Some(start_xy), Some(end_xy)) => {
                let start_xy = (start_xy.0 as usize, start_xy.1 as usize);
                let end_xy = (end_xy.0 as usize, end_xy.1 as usize);
                let x_eq = start_xy.0 == end_xy.0;
                let y_eq = start_xy.1 == end_xy.1;
                if !x_eq && !y_eq {
                    Err(FactoryError::Unsupported("off-grid walls"))
                } else {
                    const LEVELS: &'static [Level] = &[Level::Surface, Level::Sky];
                    let pos_levels = match (x_eq,y_eq) {
                        (true,true) => { vec![(start, LEVELS)] },
                        (true,false) =>
                            (start_xy.1..end_xy.1+1)
                                .into_iter()
                                .map(|y| (y, LEVELS)).collect(),
                        (false,true) =>
                            (start_xy.0..end_xy.0+1)
                                .into_iter()
                                .map(|x| (x,LEVELS))
                                .collect(),
                        (false,false) => unreachable!(),
                    };

                    let mut occlusion_map = HashMap::new();
                    let mut cells = vec![];
                    for (pos, level) in pos_levels {
                        occlusion_map.insert(pos, level);
                        cells.push(pos);
                    }
                    
                    Ok(Box::new(Self { start, end, occlusion_map, cells }))
                }
            },
            (None, _) | (_, None) => Err(FactoryError::OutOfBounds),
        }
    }
}

impl Obstacle for Wall {
    fn blocks(&self, level: &Level, pos: usize) -> bool {
        <Self as HeterogenousObstacle>::blocks(&self, level, pos)
    }

    fn levels_at(&self, pos: usize) -> &'static [Level] {
        <Self as HeterogenousObstacle>::levels_at(self, pos)
    }

    fn cells(&self) -> &[usize] {
        &self.cells
    }
}

impl HeterogenousObstacle for Wall {
    fn occlusion_map(&self) -> &HashMap<usize, &'static [Level]> {
       &self.occlusion_map
    }
}

pub mod factories {
    use super::{Factory, Grid, FactoryError, FactoryResult};
    use rand::random_range;

    pub struct RandomBoulder();
    
    impl RandomBoulder {
        pub fn new() -> Self { Self() }
        fn new_object_inner(&mut self, grid: &dyn Grid, depth: usize) -> FactoryResult {
            use super::Boulder;
            let idx = random_range(0..grid.size().0*grid.size().1);
            if let Some(_) = grid.idx(idx) {
                if depth > 10 { Err(FactoryError::Occluded(vec![idx])) }
                else { Self::new_object_inner(self, grid, depth+1) }
            }
            else {
                Ok(Box::new(Boulder::new(idx)))
            }
        }
    }
    
    impl Factory for RandomBoulder {
        fn new_object(&mut self, grid: &dyn Grid) -> FactoryResult {
            self.new_object_inner(grid, 0)
        }
    }

    pub struct Wall(usize);
    
    impl Wall {
        pub fn new(pos: usize) -> Self { Self(pos) }
        
    }
    
    impl Factory for Wall {
        fn new_object(&mut self, grid: &dyn Grid) -> FactoryResult {
            use super::Wall;
            Wall::new(grid, self.0, self.0)
        }
    }
}

#[derive(Debug)]
pub struct NullObstacle;

impl Obstacle for NullObstacle {
    fn blocks(&self, _level: &Level, _pos: usize) -> bool {
        false
    }

    fn levels_at(&self, _pos: usize) -> &'static [Level] {
        &[]
    }

    fn cells(&self) -> &[usize] {
        &[]
    }
}
