pub trait Draw {
    fn draw(&self);
}

pub trait Selectable {
    fn compute_aabb(&self) ;
    fn sample_signed_distance_field(&self, global_sample_point: &crate::Point2D<f32>);
}
