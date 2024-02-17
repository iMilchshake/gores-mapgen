use std::fmt::{self, Display};
use std::fmt::Formatter;
use std::ops::Mul;

use array2d::Array2D;
use macroquad::prelude::*;

const LEVEL_SIZE: usize = 100;
const SHIFT_FACTOR: f32 = 250.0;
const ZOOM_FACTOR: f32 = 1.1;

#[derive(Debug, Clone)]
pub enum BlockType {
    Empty,
    Filled,
    Reserved,
}

#[derive(Debug)]
pub struct Vec2D {
    pub x: usize,
    pub y: usize,
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            BlockType::Empty => write!(f, "E"),
            BlockType::Filled => write!(f, "F"),
            BlockType::Reserved => write!(f, "R"),
        }
    }
}

impl Vec2D {
    pub fn random_pos() -> Vec2D {
        let x = rand::gen_range(0, LEVEL_SIZE);
        let y = rand::gen_range(0, LEVEL_SIZE);
        Vec2D { x, y }
    }
}

pub fn handle_mouse_inputs(display_factor: &mut f32, display_shift: &mut Vec2) {
    let mouse_wheel_y = mouse_wheel().1;

    if mouse_wheel_y > 0.0 {
        *display_factor *= ZOOM_FACTOR;
    } else if mouse_wheel_y < 0.0 {
        *display_factor /= ZOOM_FACTOR;
    }

    let mouse_delta = mouse_delta_position();
    if is_mouse_button_down(MouseButton::Left) {
        *display_shift -= mouse_delta.mul(SHIFT_FACTOR);
    }
}



pub fn draw_grid_blocks(grid: &Array2D<BlockType>, display_factor: f32, display_shift: Vec2) {
    for x in 0..LEVEL_SIZE {
        for y in 0..LEVEL_SIZE {
            if let Some(value) = grid.get(x, y) {
                draw_rectangle(
                    (x as f32) * display_factor + display_shift.x,
                    (y as f32) * display_factor + display_shift.y,
                    display_factor,
                    display_factor,
                    match value {
                        BlockType::Filled => LIME,
                        _ => DARKGRAY,
                    },
                );
            }
        }
    }
}

#[macroquad::main("testing")]
async fn main() {
    let mut display_factor: f32 = 1.0;
    let mut display_shift: Vec2 = vec2(10.0, 10.0);

    let mut grid: Array2D<BlockType> =
        Array2D::filled_with(BlockType::Empty, LEVEL_SIZE, LEVEL_SIZE);

    for _ in 1..5500 {
        let point = Vec2D::random_pos();
        grid.set(point.x, point.y, BlockType::Filled).unwrap();
    }

    loop {
        clear_background(LIGHTGRAY);

        handle_mouse_inputs(&mut display_factor, &mut display_shift);
        draw_grid_blocks(&grid, display_factor, display_shift);
        macroquad::models::draw_grid(10, 10.0, BLACK, GREEN);

        draw_text(&get_fps().to_string(), 60.0, 20.0, 30.0, RED);
        next_frame().await;
    }
}
