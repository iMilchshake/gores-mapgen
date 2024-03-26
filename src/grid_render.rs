use crate::{map::BlockType, map::KernelType, position::Position, walker::CuteWalker};
use macroquad::color::*;
use macroquad::shapes::*;
use ndarray::Array2;

/// Unoptimized drawing of a grid. Usage of draw_chunked_grid is preferred.
pub fn draw_grid(grid: &Array2<BlockType>) {
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

/// Optimized variant of draw_grid using chunking. If a chunk has not been edited after
/// initialization, the entire chunk is drawn using a single rectangle. Otherwise, each block is
/// drawn individually as in the unoptimized variant.
pub fn draw_chunked_grid(
    grid: &Array2<BlockType>,
    chunks_edited: &Array2<bool>,
    chunk_size: usize,
) {
    for ((x_chunk, y_chunk), chunk_edited) in chunks_edited.indexed_iter() {
        if *chunk_edited {
            let x_start = x_chunk * chunk_size;
            let y_start = y_chunk * chunk_size;
            let x_end = usize::min((x_chunk + 1) * chunk_size, grid.shape()[0]);
            let y_end = usize::min((y_chunk + 1) * chunk_size, grid.shape()[1]);

            for x in x_start..x_end {
                for y in y_start..y_end {
                    let value = grid[[x, y]];
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
        } else {
            draw_rectangle(
                (x_chunk * chunk_size) as f32,
                (y_chunk * chunk_size) as f32,
                chunk_size as f32,
                chunk_size as f32,
                BROWN,
            );
        }
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

pub fn draw_walker_kernel(walker: &CuteWalker, kernel_type: KernelType) {
    let kernel = match kernel_type {
        KernelType::Inner => &walker.inner_kernel,
        KernelType::Outer => &walker.outer_kernel,
    };
    let offset: usize = kernel.size / 2; // offset of kernel wrt. position (top/left)

    let root_x = walker.pos.x.checked_sub(offset);
    let root_y = walker.pos.y.checked_sub(offset);

    if root_x.is_none() || root_y.is_none() {
        return; // dont draw as the following draw operation would fail
                // TODO: do this for each cell individually!
    }

    for ((x, y), kernel_active) in kernel.vector.indexed_iter() {
        if *kernel_active {
            draw_rectangle(
                (root_x.unwrap() + x) as f32,
                (root_y.unwrap() + y) as f32,
                1.0,
                1.0,
                match kernel_type {
                    KernelType::Inner => Color::new(0.0, 0.0, 1.0, 0.1),
                    KernelType::Outer => Color::new(0.0, 1.0, 0.0, 0.1),
                },
            );
        }
    }
}

pub fn draw_waypoints(waypoints: &Vec<Position>) {
    for pos in waypoints.iter() {
        draw_circle(pos.x as f32 + 0.5, pos.y as f32 + 0.5, 1.0, RED)
    }
}
