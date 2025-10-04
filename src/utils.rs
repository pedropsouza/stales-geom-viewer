use macroquad::prelude::*;
use euclid::default::Vector2D;
use std::ops::Range;

pub fn random_color() -> macroquad::color::Color {
    use random_color::RandomColor;
    use macroquad::color::Color;
    
    Color::from_vec(glam::Vec4::from_array(
        RandomColor::new().to_f32_rgba_array()))
}

pub fn random_points(count: usize, bounds: (Range<f32>,Range<f32>)) -> Vec<Vector2D<f32>> {
    let mut acc = Vec::with_capacity(count);
    for _ in 0..count {
        acc.push(Vector2D::<f32>::new(
            rand::gen_range(bounds.0.start, bounds.0.end),
            rand::gen_range(bounds.1.start, bounds.1.end),
        ));
    }
    acc
}

pub fn quantize_points(
    float_points: &Vec<Vector2D<f32>>,
    bounds: (Range<f32>,Range<f32>))
    -> Vec<Vector2D<u64>> {
    float_points.into_iter()
        .map(|p| {
            let x_norm = (p.x - bounds.0.start)/(bounds.0.end - bounds.0.start);
            let x = (x_norm*(std::u64::MAX as f32)) as u64;
            let y_norm = (p.y - bounds.1.start)/(bounds.1.end - bounds.1.start);
            let y = (y_norm*(std::u64::MAX as f32)) as u64;
            Vector2D::<u64>::new(x,y)
        })
        .collect()
}

