use stales_geom_viewer::{
    common_traits::*,
    geom::Vertex,
    point::Point,
};
use euclid::default::Vector2D;
use macroquad::prelude::*;
use crate::{observer, obstacle};
use crate::observer::{HasObserverList, Observable};
use std::fmt::Debug;

pub trait ObservableGrid: Grid + Observable {}

#[derive(Debug)]
pub struct ObservableGridDecorator {
    grid: Box<dyn Grid>,
    observers: observer::ObserverList,
}

impl ObservableGridDecorator {
    pub fn new(grid: Box<dyn Grid>) -> Self {
        Self { grid, observers: Default::default() }
    }
}

impl ObservableGrid for ObservableGridDecorator {}

impl Grid for ObservableGridDecorator {
    fn xy(&self, x: f64, y: f64) -> Option<&Box<dyn obstacle::Obstacle>> {
        self.grid.xy(x, y)
    }

    fn idx(&self, idx: usize) -> Option<&Box<dyn obstacle::Obstacle>> {
        self.grid.idx(idx)
    }

    fn obstacle_idx(&self, idx: usize) -> Option<usize> {
        self.grid.obstacle_idx(idx)
    }

    fn xy_idx(&self, x: f64, y: f64) -> Option<usize> {
        self.grid.xy_idx(x, y)
    }

    fn idx_xy(&self, idx: usize) -> Option<(f64,f64)> {
        self.grid.idx_xy(idx)
    }

    fn probe_xy(&self, x: f64, y: f64) -> Option<bool> {
        self.grid.probe_xy(x, y)
    }

    fn cell_dims(&self) -> (f64, f64) {
        self.grid.cell_dims()
    }

    fn neighbourhood(&self, idx: usize) -> Vec<(usize, bool)> {
        self.grid.neighbourhood(idx)
    }

    fn size(&self) -> (usize, usize) {
        self.grid.size()
    }

    fn push_obstacle(&mut self, obstacle: Box<dyn obstacle::Obstacle>) -> usize {
        let tmp = self.grid.push_obstacle(obstacle);
        self.invalidate_for_observers();
        tmp
    }

    fn remove_obstacle(&mut self, idx: usize) -> Box<dyn obstacle::Obstacle> {
        let tmp = self.grid.remove_obstacle(idx);
        self.invalidate_for_observers();
        tmp
    }
}

impl Select for ObservableGridDecorator {
    fn compute_aabb(&self) -> euclid::default::Box2D<f32> {
        self.grid.compute_aabb()
    }

    fn sample_signed_distance_field(&self, global_sample_point: &Vector2D<f32>) -> f32 {
        self.grid.sample_signed_distance_field(global_sample_point)
    }
}

impl Draw for ObservableGridDecorator {
    fn draw(&self) {
        self.grid.draw();
    }

    fn vertices(&self) -> Vec<Vertex> {
        self.grid.vertices()
    }
}

impl HasObserverList for ObservableGridDecorator {
    fn get_observer_list(&self) -> &observer::ObserverList {
        &self.observers
    }

    fn get_mut_observer_list(&mut self) -> &mut observer::ObserverList {
        &mut self.observers
    }
}

pub trait Grid: Debug + Draw + Select + Element {
    fn xy(&self, x: f64, y: f64) -> Option<&Box<dyn obstacle::Obstacle>>;
    fn idx(&self, idx: usize) -> Option<&Box<dyn obstacle::Obstacle>>;
    fn obstacle_idx(&self, idx: usize) -> Option<usize>;
    fn xy_idx(&self, x: f64, y: f64) -> Option<usize>;
    fn idx_xy(&self, idx: usize) -> Option<(f64,f64)>;
    fn probe_xy(&self, x: f64, y: f64) -> Option<bool>;
    fn cell_dims(&self) -> (f64, f64);

    fn neighbourhood(&self, idx: usize) -> Vec<(usize, bool)>;
    fn size(&self) -> (usize, usize);

    fn push_obstacle(&mut self, obstacle: Box<dyn obstacle::Obstacle>) -> usize;
    fn remove_obstacle(&mut self, idx: usize) -> Box<dyn obstacle::Obstacle>;

    fn coords(&self, pos: (usize, usize)) -> Option<&Box<dyn obstacle::Obstacle>> {
        self.idx(self.coords_idx(pos)?)
    }

    fn idx_coords(&self, idx: usize) -> Option<(usize, usize)> {
        let col = idx % self.size().0;
        let row = (idx - col) / self.size().0;
        if row > self.size().1 { None }
        else { Some((col, row)) }
    }

    fn coords_idx(&self, coords: (usize, usize)) -> Option<usize> {
        let (col, row) = coords;
        let idx = col + row * self.size().0;
        if idx > self.size().0 * self.size().1 { None }
        else { Some(idx) }
    }
}

#[derive(Debug)]
pub struct HexGrid {
    topleft: Point,
    botright: Point,
    array: Vec<Option<usize>>,
    obstacles: Vec<Box<dyn obstacle::Obstacle>>,
    size: (usize, usize),
    strides: (f64, f64), // distances between adjacent cell centers
    cell_wh: (f64,f64), // hex cell sizes
    verts: Vec<Vertex>,
    clr: Color,
}

// Hexagonal grid width flat-top orientation
impl HexGrid {
    pub fn new(topleft: Point, botright: Point, clr: Color, size: (usize, usize), obstacle_factories: Vec<(usize, Box<dyn obstacle::Factory>)>) -> Self {
        // figuring out vert count:
        // 6 + (6 - 2)
        // (6 - 2) + (6 - 3)
        // 6/4*square size oughta be enough
        let vert_count_heuristic = (6.0/4.0 * (size.0*size.1) as f64) as usize;
        let mut verts = Vec::with_capacity(vert_count_heuristic);
        let deltas = (botright.x() - topleft.x(), botright.y() - topleft.y());
        // rect -> w*n = W
        // hex -> (3/4*w)*n = W
        // rect -> h*n = H
        let cell_wh = (
            4.0*deltas.0/(3.0*((size.0 as f64) + 1.0/3.0)),
            deltas.1/((size.1 as f64)),
        );
        let strides = (
            3.0/4.0 * cell_wh.0,
            cell_wh.1,
        );
            
        for (i, p1) in (0..size.0).zip([0,1].iter().cycle()) {
            for j in 0..size.1 {
                for p2 in [0,1,2] {
                    for k in 0..2 {
                        let x = topleft.x()
                            + (i as f64) * strides.0
                            + if p2 != 1 {
                                (1.0/4.0*cell_wh.0) // even positive offset
                                    + (k as f64) * cell_wh.0/2.0 // second vertice of line
                            } else {
                                (k as f64) * cell_wh.0 // full step for the odd ones
                            };
                        let y = topleft.y()
                            + (j as f64) * strides.1 + (p2 as f64) * cell_wh.1/2.0
                            + if *p1 == 1 {
                                cell_wh.1/2.0
                            } else {
                                0.0
                            };
                        verts.push(Point::new(x,y));
                    }
                }
            }
        }
        let mut grid = Self {
            topleft, botright, clr, size, strides,
            verts: verts.into_iter().map(|p| Vertex::new(p.x() as f32, p.y() as f32, None)).collect(),
            array: [None].into_iter().cycle().take(size.0*size.1).collect(),
            obstacles: vec![],
            cell_wh,
        };

        for (count, ref mut factory) in obstacle_factories {
            for _ in 0..count {
                let obstacle = factory.new_object(&grid).unwrap();
                grid.push_obstacle(obstacle);
            }
        }

        grid
    }

    pub fn quantize_xy(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let xu = x;
        let yu = y;

        if xu < 0.0 || yu < 0.0 { return None; }
        
        let xi = (xu / self.strides.0) as usize;
        let yi = (yu / self.strides.1) as usize;

        Some((xi, yi))
    }

    pub fn taxicab_distance(&self, a: usize, b: usize) -> usize {
        let y_dist = (a/self.size.0).abs_diff(b/self.size.0);
        let x_dist = (a%self.size.0).abs_diff(b%self.size.0);
        x_dist + y_dist
    }

    pub fn chebyshev_distance(&self, a: usize, b: usize) -> usize {
        let y_dist = (a/self.size.0).abs_diff(b/self.size.0);
        let x_dist = (a%self.size.0).abs_diff(b%self.size.0);
        x_dist.max(y_dist)
    }

}

impl Grid for HexGrid {
    fn xy(&self, x: f64, y: f64) -> Option<&Box<dyn obstacle::Obstacle>> {
        self.xy_idx(x, y).and_then(|i| self.idx(i))
    }

    fn idx(&self, idx: usize) -> Option<&Box<dyn obstacle::Obstacle>> {
        self.array.get(idx)
            .and_then(|i| i.map(|iobst| &self.obstacles[iobst]))
    }
    
    fn xy_idx(&self, x: f64, y: f64) -> Option<usize> {
        fn axial_to_oddq(x: isize, y: isize) -> Option<(usize, usize)> {
            let parity = x&1;
            let col = x;
            let row = y + (x - parity) / 2;
            match (col,row) {
                (col, row) if col >= 0 && row >= 0 => Some((col as usize, row as usize)),
                _ => None
            }
        }

        fn oddq_to_axial(x: usize, y: usize) -> (isize, isize){
           let  parity = x&1;
            let q = x;
            let r = y - (x - parity) / 2;
            return (q as isize, r as isize)
        }
        
        let relx = x - self.topleft.x();
        let rely = y - self.topleft.y();

        let ux = relx/self.strides.0;
        let uy = rely/self.cell_wh.1;

        let q = ux;
        let r = -1.0/2.0 * ux + 2.0*3.0_f64.sqrt()/3.0 * uy;
        
        let (x,y) = axial_to_oddq(q as isize, r as isize)?;
        let idx = x + y*self.size.0;
        
        Some(idx)
        
    }

    fn idx_xy(&self, idx: usize) -> Option<(f64,f64)> {
        if idx < self.array.len() {
            let xi = idx % self.size.0;
            // hex to cartesian
            let x =        3.0/4.0 * (xi as f64);
            let y = (idx-xi) as f64/(self.size.0 as f64) + 0.5 * (xi&1) as f64;
            // scale cartesian coordinates
            let x = self.topleft.x() + x * self.cell_wh.0;
            let y = self.topleft.y() + y * self.cell_wh.1;
            Some((x,y))
        } else { None }
    }

    fn obstacle_idx(&self, idx: usize) -> Option<usize> {
        self.array.get(idx).and_then(|oidx| *oidx)
    }

    fn probe_xy(&self, x: f64, y: f64) -> Option<bool> {
        self.xy_idx(x,y).map(|idx| self.array[idx].is_some())
    }

    fn cell_dims(&self) -> (f64, f64) {
        self.cell_wh
    }

    fn neighbourhood(&self, idx: usize) -> Vec<(usize, bool)> {
        let xrun = self.size.0 as isize;
        let oddq_direction_differences = &[
            // even cols 
            &[(1,  0), (1, -1), ( 0, -1),
              (-1, -1), (-1,  0), ( 0, 1)],
            // odd cols 
            &[(1, 1), (1,  0), ( 0, -1),
              (-1,  0), (-1, 1), ( 0, 1)],
        ];

        let parity = (idx % xrun as usize)&1;
        oddq_direction_differences[parity].iter().cloned().filter_map(move |off| {
            let off = off.0 + xrun*off.1;
            let cur = (idx as isize).checked_add(off).unwrap();
            if cur < 0 { return None }
            let cur = cur as usize;
            if self.chebyshev_distance(idx, cur) != 1 { return None; }
            self.array
                .get(cur as usize)
                .and_then(|occupied| Some((cur, occupied.is_some())))
        }).collect()
    }

    fn size(&self) -> (usize, usize) { self.size }

    fn push_obstacle(&mut self, obstacle: Box<dyn obstacle::Obstacle>) -> usize {
        let l = self.obstacles.len();
        for cell in obstacle.cells() {
            self.array[*cell] = Some(l);
        }
        self.obstacles.push(obstacle);
        l
    }

    fn remove_obstacle(&mut self, idx: usize) -> Box<dyn obstacle::Obstacle> {
        let mut obstacle = Box::new(obstacle::NullObstacle) as Box<dyn obstacle::Obstacle>;
        std::mem::swap(&mut self.obstacles[idx], &mut obstacle);
        for cell in obstacle.cells() {
            self.array[*cell] = None;
        };
        obstacle
        // TODO: drop these null obstacle boxes
    }
}

impl Draw for HexGrid {
    fn draw(&self) {
        for vs in self.verts.chunks(6) {
            let pos: Vec<_> = vs.iter().map(|v| v.pos).collect();
            const LUT: [(usize, usize); 6] = [(0,1),(0,2),(1,3),(2,4),(3,5),(4,5)];
            for (a,b) in LUT {
                draw_line(pos[a].x, pos[a].y, pos[b].x, pos[b].y, 2.0, self.clr);
            }
        }
        for idx in 0..self.size.0*self.size.1 {
            let pos = self.idx_xy(idx);
            let pos = pos.unwrap();
            let r = self.array[idx].map(|_| 10.0);
            if let Some(r) = r {
                draw_circle((pos.0 + self.cell_wh.0/2.0) as f32, (pos.1 + self.cell_wh.1/2.0) as f32, r, self.clr);
            };
        }
    }

    fn vertices(&self) -> Vec<Vertex> {
        self.verts.clone()
    }
}

impl Select for HexGrid {
    fn compute_aabb(&self) -> euclid::default::Box2D<f32> {
        euclid::default::Box2D::new(
            euclid::default::Point2D::new(self.topleft.x()  as f32, self.topleft.y()  as f32),
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

#[derive(Debug)]
pub struct SquareGrid {
    topleft: Point,
    botright: Point,
    array: Vec<Option<usize>>,
    obstacles: Vec<Box<dyn obstacle::Obstacle>>,
    size: (usize, usize),
    strides: (f64, f64),
    verts: Vec<Vertex>,
    clr: Color,
}

impl SquareGrid {
    pub fn new(topleft: Point, botright: Point, clr: Color, size: (usize, usize), obstacle_factories: Vec<(usize, Box<dyn obstacle::Factory>)>) -> Self {
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

        let mut grid = Self {
            topleft, botright, clr, size, strides,
            verts: verts.into_iter().map(|p| Vertex::new(p.x() as f32, p.y() as f32, None)).collect(),
            array: [None].into_iter().cycle().take(size.0*size.1).collect(),
            obstacles: vec![],
        };

        for (count, ref mut factory) in obstacle_factories {
            for _ in 0..count {
                let obstacle = factory.new_object(&grid).unwrap();
                grid.push_obstacle(obstacle);
            }
        }

        grid
    }

    pub fn quantize_xy(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let xu = x;
        let yu = y;

        if xu < 0.0 || yu < 0.0 { return None; }
        
        let xi = (xu / self.strides.0) as usize;
        let yi = (yu / self.strides.1) as usize;

        Some((xi, yi))
    }

    pub fn taxicab_distance(&self, a: usize, b: usize) -> usize {
        let y_dist = (a/self.size.0).abs_diff(b/self.size.0);
        let x_dist = (a%self.size.0).abs_diff(b%self.size.0);
        x_dist + y_dist
    }

    pub fn chebyshev_distance(&self, a: usize, b: usize) -> usize {
        let y_dist = (a/self.size.0).abs_diff(b/self.size.0);
        let x_dist = (a%self.size.0).abs_diff(b%self.size.0);
        x_dist.max(y_dist)
    }

}

impl Grid for SquareGrid {
    fn xy(&self, x: f64, y: f64) -> Option<&Box<dyn obstacle::Obstacle>> {
        self.xy_idx(x, y).and_then(|i| self.idx(i))
    }

    fn idx(&self, idx: usize) -> Option<&Box<dyn obstacle::Obstacle>> {
        self.array.get(idx)
            .and_then(|i| i.map(|iobst| &self.obstacles[iobst]))
    }
    
    fn xy_idx(&self, x: f64, y: f64) -> Option<usize> {
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

    fn idx_xy(&self, idx: usize) -> Option<(f64,f64)> {
        if idx < self.array.len() {
            let xi = idx % self.size.0;
            let x = xi as f64 * self.strides.0 + self.topleft.x();
            let y = ((idx - xi) as f64 * self.strides.1) / self.size.0 as f64 + self.topleft.y();
            Some((x,y))
        } else { None }
    }
    fn obstacle_idx(&self, idx: usize) -> Option<usize> {
        self.array.get(idx).and_then(|oidx| *oidx)
    }

    fn probe_xy(&self, x: f64, y: f64) -> Option<bool> {
        self.xy_idx(x,y).map(|idx| self.array[idx].is_some())
    }

    fn cell_dims(&self) -> (f64, f64) {
        self.strides
    }

    fn neighbourhood(&self, idx: usize) -> Vec<(usize, bool)> {
        let xrun = self.size.0 as isize;
        let offsets = Box::new([
            -xrun-1,-xrun,-xrun+1,
                 -1,            1,
             xrun-1, xrun, xrun+1,
            //-xrun, -1, 1, xrun,
        ]);
        offsets.iter().cloned().filter_map(move |off| {
            let cur = (idx as isize).checked_add(off).unwrap();
            if cur < 0 { return None }
            let cur = cur as usize;
            if self.chebyshev_distance(idx, cur) != 1 { return None; }
            self.array
                .get(cur as usize)
                .and_then(|occupied| Some((cur, occupied.is_some())))
        }).collect()
    }

    fn size(&self) -> (usize, usize) { self.size }

    fn push_obstacle(&mut self, obstacle: Box<dyn obstacle::Obstacle>) -> usize {
        let l = self.obstacles.len();
        for cell in obstacle.cells() {
            self.array[*cell] = Some(l);
        }
        self.obstacles.push(obstacle);
        l
    }

    fn remove_obstacle(&mut self, idx: usize) -> Box<dyn obstacle::Obstacle> {
        let mut obstacle = Box::new(obstacle::NullObstacle) as Box<dyn obstacle::Obstacle>;
        std::mem::swap(&mut self.obstacles[idx], &mut obstacle);
        for cell in obstacle.cells() {
            self.array[*cell] = None;
        };
        obstacle
        // TODO: drop these null obstacle boxes
    }
}

impl Draw for SquareGrid {
    fn draw(&self) {
        for vs in self.verts.chunks(2) {
            draw_line(vs[0].pos.x, vs[0].pos.y,
                      vs[1].pos.x, vs[1].pos.y,
                      1.0, self.clr);
        }

        for (idx, _) in self.array.iter().enumerate().filter(|(_,x)| x.is_some()) {
            let xy_box = |x: f64, y: f64| -> Option<(Point, Point)> {
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
            };

            let idx_box = |idx: usize| -> Option<(Point, Point)> {
                self.idx_xy(idx).and_then(|(x,y)| xy_box(x,y))
            };

            if let Some((topl,botr)) = idx_box(idx) {
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

impl Select for SquareGrid {
    fn compute_aabb(&self) -> euclid::default::Box2D<f32> {
        euclid::default::Box2D::new(
            euclid::default::Point2D::new(self.topleft.x()  as f32, self.topleft.y()  as f32),
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

