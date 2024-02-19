use std::fmt::Formatter;
use std::fmt::{self, Display};
use std::ops::Mul;

use macroquad::prelude::*;
use ndarray::{prelude, Array, Array2};

const SHIFT_FACTOR: f32 = 250.0;
const ZOOM_FACTOR: f32 = 1.1;

#[derive(Debug, Clone)]
pub enum BlockType {
    Empty,
    Filled,
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
        }
    }
}

impl Vec2D {
    pub fn random_pos(level_size: usize) -> Vec2D {
        let x = rand::gen_range(0, level_size);
        let y = rand::gen_range(0, level_size);
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

pub fn draw_grid_blocks(grid: &Array2<BlockType>, display_factor: f32, display_shift: Vec2) {
    let width = grid.dim().0;
    let height = grid.dim().1;

    for x in 0..width {
        for y in 0..height {
            draw_rectangle(
                (x as f32) * display_factor + display_shift.x,
                (y as f32) * display_factor + display_shift.y,
                display_factor,
                display_factor,
                match grid[[x, y]] {
                    BlockType::Filled => LIME,
                    _ => DARKGRAY,
                },
            );
        }
    }
}
