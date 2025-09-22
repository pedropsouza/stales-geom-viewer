use macroquad::prelude::*;
use std::default::Default;
use std::iter::Iterator;
use std::ops::Range;

pub mod common_traits;
pub use common_traits::*;
use euclid::{default::{Point2D, Size2D, Vector2D}, *};

type Color = macroquad::color::Color;

pub mod geom;
pub use geom::{*, Circle, Vertex};

#[derive(Default)]
struct State {
    circles: Vec<Circle>,
    lines: Vec<Line2D>,
    clear_color: Color,
}

impl State {
    fn add_line(&mut self, l: Line2D) {
        self.lines.push(l);
    }
    fn add_circle(&mut self, c: Circle) {
        self.circles.push(c);
    }
    fn all_drawables(&self) -> impl Iterator<Item = Box<&dyn Draw>> {
        self.circles.iter()
                    .map(|x| { let b: Box<&dyn Draw> = Box::new(x); b })
                    .chain(self.lines.iter().map(|x| { let b: Box<&dyn Draw> = Box::new(x); b }))
    }
    fn text_digest(&self) -> String {
        let line_cnt = self.lines.len();
        let circle_cnt = self.circles.len();
        let frametime = get_frame_time();
        format!(r"
num. of lines: {line_cnt}
num. of circles: {circle_cnt}
frametime: {frametime}
")
    }
}

fn square_grid(spacing: f32, breadth: (Range<f32>, Range<f32>)) -> Vec<Line2D> {
    let mut x = breadth.0.start;
    let mut y = breadth.1.start;

    let mut lines = vec![];
    while breadth.1.contains(&y) {
        let ny = y + spacing;
        while breadth.0.contains(&x) {
            let nx = x + spacing;
            if breadth.0.contains(&nx) {
                lines.push(Line2D {
                    a: Vertex::new(x,y, Some(RED)),
                    b: Vertex::new(nx, y, Some(RED)),
                    thickness: 1.0,
                });
            }
            if breadth.1.contains(&ny) {
                lines.push(Line2D {
                    a: Vertex::new(x, y, Some(YELLOW.with_alpha(0.1))),
                    b: Vertex::new(x, ny, Some(YELLOW)),
                    thickness: 2.0,
                });
            }
            x = nx;
        }
        x = breadth.0.start;
        y = ny;
    };
    lines
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let mut state: State = Default::default();

    state.add_line(Line2D {
        a: Vertex::new(40.0, 40.0, Some(BLUE)),
        b: Vertex::new(100.0, 100.0, None),
        thickness: 1.0,
    });

    for line in square_grid(20.0, ((0.0..screen_width()), (0.0..screen_height()))) {
        state.add_line(line)
    }
    state.add_circle(Circle { center: Vertex::new(screen_width() - 30.0, screen_height() - 30.0, Some(YELLOW)), radius: 15.0 });
    loop {
        if is_quit_requested() { break }
        clear_background(state.clear_color);

        for drawable in state.all_drawables() {
            drawable.draw();
        }

        let line = state.lines.get_mut(0).unwrap();
        line.a.pos += Vector2D::new(10.0*get_frame_time(), 0.0);

        draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        if is_key_released(KeyCode::R) {
            println!("{}", state.text_digest())
        }
        next_frame().await
    }
}
