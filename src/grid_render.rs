use std::fmt::Formatter;

use crate::CuteWalker;
use macroquad::prelude::*;
use ndarray::Array2;

#[derive(Debug, Clone, Copy)]
pub enum BlockType {
    Empty,
    Filled,
}

pub fn handle_mouse_inputs(display_factor: &mut f32, display_shift: &mut Vec2) {
    /* let mouse_wheel_y = mouse_wheel().1;

    if mouse_wheel_y > 0.0 {
        *display_factor *= ZOOM_FACTOR;
    } else if mouse_wheel_y < 0.0 {
        *display_factor /= ZOOM_FACTOR;
    }

    let mouse_delta = mouse_delta_position();
    if is_mouse_button_down(MouseButton::Left) {
        *display_shift -= mouse_delta.mul(SHIFT_FACTOR);
    } */
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

pub fn draw_walker(walker: &CuteWalker, display_factor: f32, display_shift: Vec2) {
    draw_rectangle_lines(
        (walker.pos.x as f32) * display_factor + display_shift.x,
        (walker.pos.y as f32) * display_factor + display_shift.y,
        display_factor,
        display_factor,
        2.0,
        YELLOW,
    );
    draw_circle(
        walker.pos.x as f32 * display_factor + (display_factor / 2.),
        walker.pos.y as f32 * display_factor + (display_factor / 2.),
        display_factor * 0.25,
        BLUE,
    )
}
