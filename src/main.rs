mod grid_test;
use std::isize;

use grid_test::*;

use ndarray::Array2;

use egui::{epaint::Shadow, Color32, Frame, Label, Margin, Rect};
use macroquad::prelude::*;

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
    UP,
    RIGHT,
    DOWN,
    LEFT
}

// using my own position vector to meet ndarray's indexing standard using usize
// while glam has nice performance benefits, the amount of expensive operations
// on the position vector will be very limited, so this should be fine..
#[derive(Debug, Default, PartialEq)]
struct Position {
    x: usize, 
    y: usize
}

impl Position {
    fn as_index(&self) -> [usize;2] {
        [self.x, self.y]
    }  

    fn shift(&mut self, shift: ShiftDirection) {
        match shift {
            ShiftDirection::UP => {self.y -= 1},
            ShiftDirection::RIGHT => {self.x += 1},
            ShiftDirection::DOWN => {self.y += 1},
            ShiftDirection::LEFT => {self.x -= 1}
        }
    }

    fn get_greedy_dir(&self, goal: &Position) -> ShiftDirection {
        let x_diff = goal.x as isize - self.x as isize;
        let x_abs_diff = x_diff.abs();
        let y_diff = goal.y as isize - self.y as isize;
        let y_abs_diff = y_diff.abs();
        
        // check whether x or y is dominant
        if x_abs_diff > y_abs_diff {
            if x_diff.is_positive() {
                return ShiftDirection::RIGHT;
            } else {
                return ShiftDirection::LEFT;
            }
        } else {
            if y_diff.is_positive() {
                return ShiftDirection::DOWN;
            } else {
                return ShiftDirection::UP;
            }
        }

    }
}

// this walker is indeed very cute
#[derive(Default, Debug)]
struct CuteWalker {
    pos: Position,
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut main_rect: Rect = Rect::EVERYTHING;
    let mut grid = Array2::from_elem((LEVEL_SIZE, LEVEL_SIZE), BlockType::Empty);
    let mut walker = CuteWalker::default();

    let goal: Position = Position {x: 90, y: 90};

    loop {
        clear_background(WHITE);

        // TODO: add proper mouse input xd
        // if main_rect.contains(mouse_position().into()) {
        //     handle_mouse_inputs(&mut display_factor, &mut display_shift);
        // }

        if walker.pos.ne(&goal) {
            walker.pos.shift(walker.pos.get_greedy_dir(&goal));
            grid[walker.pos.as_index()] = BlockType::Filled;
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

            main_rect = egui_ctx.available_rect();
        });

        // draw grid
        let available_length = f32::min(main_rect.width(), main_rect.height()); // TODO: assumes square
        let display_factor = available_length / LEVEL_SIZE as f32;
        draw_grid_blocks(&grid, display_factor, vec2(0.0, 0.0));

        // draw GUI
        egui_macroquad::draw();

        next_frame().await
    }
}
