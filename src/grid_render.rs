use crate::{BlockType, CuteWalker, Position};
use macroquad::color::*;
use macroquad::shapes::*;
use ndarray::Array2;

pub fn draw_grid_blocks(grid: &Array2<BlockType>) {
    for ((x, y), value) in grid.indexed_iter() {
        draw_rectangle(
            x as f32,
            y as f32,
            1.0,
            1.0,
            match value {
                BlockType::Hookable => BROWN,
                BlockType::Freeze => Color::new(0.0, 0.0, 0.0, 0.8),
                _ => Color::new(0.0, 0.0, 0.0, 0.1),
            },
        );
    }
}

pub fn draw_walker(walker: &CuteWalker) {
    draw_rectangle_lines(
        walker.pos.x as f32,
        walker.pos.y as f32,
        1.0,
        1.0,
        2.0,
        YELLOW,
    );
    draw_circle(
        walker.pos.x as f32 + 0.5,
        walker.pos.y as f32 + 0.5,
        0.25,
        BLUE,
    )
}

pub fn draw_walker_kernel(walker: &CuteWalker) {
    let offset: usize = walker.kernel.size / 2; // offset of kernel wrt. position (top/left)

    let root_pos = Position::new(walker.pos.x - offset, walker.pos.y - offset);
    for ((x, y), kernel_active) in walker.kernel.vector.indexed_iter() {
        if *kernel_active {
            draw_rectangle_lines(
                (root_pos.x + x) as f32,
                (root_pos.y + y) as f32,
                1.0,
                1.0,
                0.05,
                Color::new(0.1, 0.1, 1.0, 0.5),
            );
        }
    }
}

pub fn draw_waypoints(waypoints: &Vec<Position>) {
    for pos in waypoints.iter() {
        draw_circle(pos.x as f32 + 0.5, pos.y as f32 + 0.5, 1.0, RED)
    }
}
