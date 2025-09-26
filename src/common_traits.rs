use crate::Vector2D;
use crate::geom::Vertex;

pub trait Draw {
    fn draw(&self);
    fn vertices(&self) -> Vec<Vertex>;
}

pub trait Select {
    fn compute_aabb(&self) -> crate::Box2D<f32>;
    fn sample_signed_distance_field(&self, global_sample_point: &Vector2D<f32>) -> f32;
    fn contains_point(&self, point: &Vector2D<f32>) -> bool {
        self.sample_signed_distance_field(point) <= 0.0
    }
}

pub trait Element: Draw + Select + std::fmt::Debug {}

impl<T: Draw + Select + std::fmt::Debug> Element for T {

}
