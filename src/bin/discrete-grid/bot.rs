use ::rand::random_range;
use stales_geom_viewer::{
    common_traits::*, geom::{self, Vertex, *}, point::Point, utils::{self, random_color}
};
use euclid::default::Vector2D;
use macroquad::prelude::*;
use crate::{State, Object, grid::Grid};
use std::{collections::{ HashMap, HashSet }, sync::RwLock};
use std::rc::Rc;
use dyn_clone::DynClone;

#[derive(Debug)]
pub struct Bot {
    pub pos: Point,
    pub dest_idx: usize,
    pub origin_idx: usize,
    pub path: Result<Vec<(usize,(f32,f32))>, String>,
    pub pathfinder: Box<dyn Pathfinder>,
    pub anim_step: f32,
    clr: Color,
    radius: f32,
}

impl Clone for Bot {
    fn clone(&self) -> Self {
        Self {
            pos: self.pos.clone(),
            dest_idx: self.dest_idx.clone(),
            origin_idx: self.origin_idx.clone(),
            path: self.path.clone(),
            pathfinder: dyn_clone::clone_box(&*self.pathfinder),
            anim_step: self.anim_step.clone(),
            clr: self.clr.clone(),
            radius: self.radius.clone(),
        }
    }
}

impl Bot {
    pub fn new(grid: &Box<dyn Grid>, mut pathfinder: Box<dyn Pathfinder>, origin_idx: usize, dest_idx: usize, clr: Color) -> Self {
        let cdims = grid.cell_dims();
        let radius = cdims.0.min(cdims.1)/2.0;
        let pos_xy = grid.idx_xy(origin_idx).unwrap();
        let pos = Point::new(pos_xy.0 + cdims.0/2.0, pos_xy.1 + cdims.1/2.0);
        let path = pathfinder.pathfind(grid, origin_idx, dest_idx);

        Self {
            pos, origin_idx, dest_idx, clr, radius: radius as f32, path, pathfinder, anim_step: 0.0,
        }
    }

    pub fn random_inside(grid: &Box<dyn Grid>, pathfinder: Box<dyn Pathfinder>) -> Self {
        let origin_idx = random_range(0..grid.size().0*grid.size().1);
        let dest_idx = random_range(0..grid.size().0*grid.size().1);
        let clr = random_color();
        Self::new(grid, pathfinder, origin_idx, dest_idx, clr)
    }

        pub fn recalc_path(&mut self, grid: &Box<dyn Grid>) {
        self.path = self.pathfinder.pathfind(grid, self.origin_idx, self.dest_idx);
    }
}

impl Draw for Bot {
    fn draw(&self) {
        let (a,b) = match &self.path {
            Ok(path) => {
                let a_i = (self.anim_step.floor() as usize) % path.len();
                let b_i = ((self.anim_step + 1.0).floor() as usize) % path.len();
                let xy_a = path[a_i].1;
                let xy_b = path[b_i].1;
                (Point::new(xy_a.0.into(), xy_a.1.into()),
                 Point::new(xy_b.0.into(), xy_b.1.into()))
            },
            Err(_) => (self.pos, self.pos),
        };
        let pos = a * (1.0 - self.anim_step.fract() as f64) + b * (self.anim_step.fract() as f64);
        draw_circle(pos.x() as f32, pos.y() as f32, self.radius, self.clr);

        match self.path {
            Ok(ref path) => {
                let verts = path.iter().map(|v| v.1);
                let verts_len = verts.len();
                let segments = verts.clone().zip(verts.skip(1)).take(verts_len - 1);
                for (a,b) in segments {
                    draw_line(a.0, a.1, b.0, b.1, 2.0, self.clr);
                    //draw_circle_lines(b.0, b.1, self.radius/4.0, 2.0, self.clr);
                }
            },
            Err(_) => (),
        }
    }

    fn vertices(&self) -> Vec<Vertex> {
        vec![Vertex::new(self.pos.x() as f32, self.pos.y() as f32, Some(self.clr))]
    }
}

impl Select for Bot {
    fn compute_aabb(&self) -> euclid::default::Box2D<f32> {
        let topleft  = euclid::default::Point2D::new(self.pos.x() as f32 - self.radius, self.pos.y() as f32 - self.radius);
        let botright = euclid::default::Point2D::new(self.pos.x() as f32 + self.radius, self.pos.y() as f32 + self.radius);
        euclid::default::Box2D::new(topleft, botright)
    }

    fn sample_signed_distance_field(&self, global_sample_point: &Vector2D<f32>) -> f32 {
        (Point::new(global_sample_point.x as f64, global_sample_point.y as f64) - self.pos).magnitude() as f32 - self.radius
    }
}

#[derive(Debug, Clone)]
pub struct BasePathfinder {}

pub trait Pathfinder: std::fmt::Debug + DynClone {
    fn pathfind(&mut self, grid: &Box<dyn Grid>, from: usize, to: usize) -> Result<Vec<(usize, (f32,f32))>, String> {
        let mut space: Vec<(usize, usize, Vec<usize>)> = vec![(from, 0usize, vec![from])];
        let mut needle = 0usize;
        let mut seen = HashSet::new();
        while needle < space.len() {
            let neighs = grid.neighbourhood(space[needle].0);
            let mut closed = true;
            for (idx, occupied) in neighs {
                closed &= occupied;
                if !occupied && !seen.contains(&idx) {
                    let dist = space[needle].1+1;
                    if idx == to {
                        let mut path = space[needle].2.clone();
                        path.push(idx);
                        
                        return Ok(path.into_iter().map(|idx| {
                            let mut pos = grid.idx_xy(idx).unwrap();
                            pos.0 += grid.cell_dims().0/2.0;
                            pos.1 += grid.cell_dims().1/2.0;
                            (idx, (pos.0 as f32, pos.1 as f32))
                        }).collect());
                    } else {
                        let mut path = space[needle].2.clone();
                        path.push(idx);
                        space.push((idx, dist, path));
                    }
                }
                seen.insert(idx);
            }
            if closed {
                return Err("no path from end to start".to_owned());
            }
            needle += 1;
        }
        return Err("exhausted search space".to_owned());
    }
}

impl Pathfinder for BasePathfinder {}

#[derive(Debug)]
pub struct DebugPathFinder(Box<dyn Pathfinder>);

impl Clone for DebugPathFinder {
    fn clone(&self) -> Self {
        Self { 0: dyn_clone::clone_box(&*self.0) }
    }
}

pub trait PathfinderDecorator: Pathfinder {
    fn wrap(other: Box<dyn Pathfinder>) -> Box<dyn Pathfinder>;
}

impl Pathfinder for DebugPathFinder {
    fn pathfind(&mut self, grid: &Box<dyn Grid>, from: usize, to: usize) -> Result<Vec<(usize, (f32,f32))>, String> {
        let inner_res = self.0.pathfind(grid, from, to);
        if let Ok(path) = &inner_res {
            for place in path {
                let idx = place.0;
                let (x,y) = place.1;
                let info = format!("{idx}: {x}x{y}");
                draw_text(&info, x, y, 10.0, WHITE);
            }
        }
        inner_res
    }
}

impl PathfinderDecorator for DebugPathFinder {
    fn wrap(other: Box<dyn Pathfinder>) -> Box<dyn Pathfinder> {
        Box::new(Self { 0: other })
    }
}


