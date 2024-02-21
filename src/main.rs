mod grid_test;
use core::panic;
use std::isize;

use grid_test::*;

use ndarray::Array2;

use egui::{epaint::Shadow, widgets, Color32, Frame, Label, Margin, Rect};
use macroquad::{miniquad::native::egl::EGL_HEIGHT, prelude::*};

const LEVEL_SIZE: usize = 100;

fn window_frame() -> Frame {
    Frame {
        fill: Color32::from_gray(0),
        inner_margin: Margin::same(5.0),
        shadow: Shadow::NONE,
        ..Default::default()
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "egui with macroquad".to_owned(),
        ..Default::default()
    }
}

#[derive(Debug)]
enum ShiftDirection {
    Up,
    Right,
    Down,
    Left,
}

// using my own position vector to meet ndarray's indexing standard using usize
// while glam has nice performance benefits, the amount of expensive operations
// on the position vector will be very limited, so this should be fine..
#[derive(Debug, Default, PartialEq)]
struct Position {
    x: usize,
    y: usize,
}

impl Position {
    fn as_index(&self) -> [usize; 2] {
        [self.x, self.y]
    }

    fn get_greedy_dir(&self, goal: &Position) -> ShiftDirection {
        let x_diff = goal.x as isize - self.x as isize;
        let x_abs_diff = x_diff.abs();
        let y_diff = goal.y as isize - self.y as isize;
        let y_abs_diff = y_diff.abs();

        // check whether x or y is dominant
        if x_abs_diff > y_abs_diff {
            if x_diff.is_positive() {
                return ShiftDirection::Right;
            } else {
                return ShiftDirection::Left;
            }
        } else {
            if y_diff.is_positive() {
                return ShiftDirection::Down;
            } else {
                return ShiftDirection::Up;
            }
        }
    }
}

#[derive(Debug)]
struct Map {
    grid: Array2<BlockType>,
    height: usize,
    width: usize,
}

impl Map {
    fn new(width: usize, height: usize) -> Map {
        Map {
            grid: Array2::from_elem((LEVEL_SIZE, LEVEL_SIZE), BlockType::Empty),
            width,
            height,
        }
    }

    fn is_pos_in_bounds(&self, pos: Position) -> bool {
        // we dont have to check for lower bound, because of usize
        pos.x < self.width && pos.y < self.height
    }
}

// this walker is indeed very cute
#[derive(Debug)]
struct CuteWalker {
    pos: Position,
}

impl CuteWalker {
    pub fn cuddle(&self) {
        println!("Cute walker was cuddled!");
    }

    fn shift_pos(&mut self, shift: ShiftDirection) {
        match shift {
            ShiftDirection::Up => self.pos.y -= 1,
            ShiftDirection::Right => self.pos.x += 1,
            ShiftDirection::Down => self.pos.y += 1,
            ShiftDirection::Left => self.pos.x -= 1,
        }
    }

    fn is_shift_valid(&self, shift: &ShiftDirection, map: &Map) -> bool {
        match shift {
            ShiftDirection::Up => self.pos.y > 0,
            ShiftDirection::Right => self.pos.x < map.width - 1,
            ShiftDirection::Down => self.pos.y < map.height - 1,
            ShiftDirection::Left => self.pos.x > 0,
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // this Rect will hold the available space left after drawing egui
    let mut canvas: Rect = Rect::EVERYTHING;

    let mut map = Map::new(LEVEL_SIZE, LEVEL_SIZE);

    let mut walker = CuteWalker {
        pos: Position { x: 0, y: 0 },
    };

    let goal: Position = Position { x: 101, y: 5 };

    loop {
        clear_background(WHITE);

        if walker.pos.ne(&goal) {
            let shift = walker.pos.get_greedy_dir(&goal);
            if walker.is_shift_valid(&shift, &map) {
                walker.shift_pos(shift);
                map.grid[walker.pos.as_index()] = BlockType::Filled;
            } else {
                eprintln!("Error: Shift out of bounds!");
                std::process::exit(1)
            }
        }

        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label("hello world");
                ui.separator();
            });

            egui::Window::new("DEBUG")
                .frame(window_frame())
                .show(egui_ctx, |ui| {
                    ui.add(Label::new(get_fps().to_string()));
                    ui.add(Label::new(format!("{:?}", walker)));
                });

            canvas = egui_ctx.available_rect();
        });

        // draw grid
        let available_length = f32::min(canvas.width(), canvas.height()); // TODO: assumes square
        let display_factor = available_length / LEVEL_SIZE as f32;
        draw_grid_blocks(&mut map.grid, display_factor, vec2(0.0, 0.0));

        // draw GUI
        egui_macroquad::draw();

        next_frame().await
    }
}
