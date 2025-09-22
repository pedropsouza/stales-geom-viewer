use macroquad::prelude::*;
use std::default::Default;
use std::iter::Iterator;
use std::ops::Range;

mod common_traits;
pub use common_traits::*;
pub use euclid::{*, default::Point2D};

type Color = macroquad::color::Color;

struct Line2D {
    a: Point2D<f32>,
    b: Point2D<f32>,
    clr: Color,
}

impl Draw for Line2D {
    fn draw(&self) {
        draw_line(self.a.x, self.a.y,
                  self.b.x, self.b.y,
                  1.0, self.clr);
    }
}

struct Circle {
    center: Point2D<f32>,
    radius: f32,
    clr: Color,
}

impl Draw for Circle {
    fn draw(&self) {
        draw_circle(self.center.x, self.center.y,
                    self.radius, self.clr);
    }
}

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

// fn square_grid(spacing: f32, breadth: (Range<f32>, Range<f32>)) -> Vec<Line2d> {
//     let mut x = breadth.0.start;
//     let mut y = breadth.1.start;

//     let mut xseq = (0..((breadth.0.end - x)/spacing) as usize)
//         .into_iter()
//         .map(|x| (x as f32)*spacing)
//         .peekable()
//         .into_iter();
//     let mut yseq = (0..((breadth.1.end - y)/spacing) as usize)
//         .into_iter()
//         .map(|y| (y as f32)*spacing)
//         .peekable()
//         .into_iter();

//     let mut lines = vec![];
//     for y in yseq {
//         let ny = yseq.peek();
//         let xseq_copy = xseq.clone();
//         for x in xseq {
//             let nx = xseq.peek();
//             if let Some(nx) = nx {
//                 lines.push(Line2d {
//                     a: Point2D<f32> {x, y},
//                     b: Point2D<f32> { x: *nx, y},
//                     clr: RED,
//                 });
//             }
//             if let Some(ny) = ny {
//                 lines.push(Line2d {
//                     a: Point2D<f32> {x, y},
//                     b: Point2D<f32> { x, y: *ny },
//                     clr: RED,
//                 });
//             }
//         }
//     };
//     lines
// }

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
                    a: Point2D::<f32>::new(x,y),
                    b: Point2D::<f32>::new(nx, y),
                    clr: RED,
                });
            }
            if breadth.1.contains(&ny) {
                lines.push(Line2D {
                    a: Point2D::<f32>::new(x, y),
                    b: Point2D::<f32>::new(x, ny),
                    clr: RED,
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
        a: Point2D::<f32>::new(40.0, 40.0),
        b: Point2D::<f32>::new(100.0, 100.0),
        clr: BLUE
    });

    for line in square_grid(20.0, ((0.0..screen_width()), (0.0..screen_height()))) {
        state.add_line(line)
    }
    state.add_circle(Circle { center: Point2D::<f32>::new(screen_width() - 30.0, screen_height() - 30.0), radius: 15.0, clr: YELLOW });
    loop {
        if is_quit_requested() { break }
        clear_background(state.clear_color);

        for drawable in state.all_drawables() {
            drawable.draw();
        }

        let line = state.lines.get_mut(0).unwrap();
        (*line).a = line.a + default::Size2D::<f32>::new(10.0*get_frame_time(), 0.0);

        draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        if is_key_released(KeyCode::R) {
            println!("{}", state.text_digest())
        }
        next_frame().await
    }
}
