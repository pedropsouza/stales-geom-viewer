use stales_geom_viewer::{
    common_traits::*,
    geom::Vertex,
    point::Point,
};
use euclid::default::Vector2D;
use macroquad::prelude::*;

#[derive(Debug)]
pub struct Grid {
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
            let xi = idx % self.size.0;
            let x = xi as f64 * self.strides.0 + self.topleft.x();
            let y = ((idx - xi) as f64 * self.strides.1) / self.size.0 as f64 + self.topleft.y();
            Some((x,y))
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

    pub fn cell_dims(&self) -> (f64, f64) {
        self.strides
    }

    pub fn neighbourhood(&self, idx: usize) -> Vec<(usize, bool)> {
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
                .and_then(|occupied| Some((cur, *occupied)))
        }).collect()
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

