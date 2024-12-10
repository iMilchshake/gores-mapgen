use crate::map_camera::MapCamera;
use crate::{map::BlockType, map::KernelType, walker::CuteWalker};
use macroquad::color::colors;
use macroquad::color::Color;
use macroquad::shapes::*;
use macroquad::text::{draw_text_ex, TextParams};
use ndarray::Array2;

fn blocktype_to_color(value: &BlockType) -> Color {
    match value {
        BlockType::Hookable => Color::new(0.76, 0.48, 0.29, 0.8),
        BlockType::Freeze => Color::new(0.0, 0.0, 0.0, 0.8),
        BlockType::Empty => Color::new(0.0, 0.0, 0.0, 0.0),
        BlockType::EmptyReserved => Color::new(0.3, 0.0, 0.0, 0.1),
        BlockType::Finish => Color::new(1.0, 0.1, 0.1, 0.8),
        BlockType::Start => Color::new(0.1, 1.0, 0.1, 0.8),
        BlockType::Platform => Color::new(0.83, 0.64, 0.51, 0.8),
        BlockType::Spawn => Color::new(0.2, 0.2, 0.7, 0.8),
    }
}

/// Unoptimized drawing of a grid with dynamic colormap.
pub fn draw_grid<T, F>(grid: &Array2<T>, to_color: F)
where
    F: Fn(&T) -> Color,
{
    for ((x, y), value) in grid.indexed_iter() {
        draw_rectangle(x as f32, y as f32, 1.0, 1.0, to_color(value));
    }
}

/// Drawing of a boolean grid. Only draw cells with true values. Useful for debugging.
pub fn draw_bool_grid(grid: &Array2<bool>, color: &Color, outline: &bool) {
    for ((x, y), value) in grid.indexed_iter() {
        if *value {
            if *outline {
                draw_rectangle_lines(x as f32, y as f32, 1.0, 1.0, 0.1, *color);
            } else {
                draw_rectangle(x as f32, y as f32, 1.0, 1.0, *color);
            }
        }
    }
}

/// Drawing of a float grid.
pub fn draw_opt_float_grid(grid: &Array2<Option<f32>>, color_min: &Color, color_max: &Color) {
    let max_value = grid
        .iter()
        .filter_map(|&x| x)
        .max_by(|a, b| a.partial_cmp(b).unwrap());

    let min_value = grid
        .iter()
        .filter_map(|&x| x)
        .min_by(|a, b| a.partial_cmp(b).unwrap());

    if min_value.is_none() || max_value.is_none() {
        return;
    }

    for ((x, y), value) in grid.indexed_iter() {
        if value.is_none() {
            continue;
        }

        let relative_value = (value.unwrap() - min_value.unwrap()) / max_value.unwrap();

        let lerp_color = Color::new(
            (color_min.r * relative_value) + (color_max.r * (1. - relative_value)),
            (color_min.g * relative_value) + (color_max.g * (1. - relative_value)),
            (color_min.b * relative_value) + (color_max.b * (1. - relative_value)),
            (color_min.a * relative_value) + (color_max.a * (1. - relative_value)),
        );

        draw_rectangle(x as f32, y as f32, 1.0, 1.0, lerp_color);
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
                    let value = &grid[[x, y]];
                    draw_rectangle(x as f32, y as f32, 1.0, 1.0, blocktype_to_color(value));
                }
            }
        } else {
            let mut color = blocktype_to_color(&BlockType::Hookable); // assumed that initial value is hookable
            color.a *= 0.95;
            draw_rectangle(
                (x_chunk * chunk_size) as f32,
                (y_chunk * chunk_size) as f32,
                chunk_size as f32,
                chunk_size as f32,
                color,
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
        colors::YELLOW,
    );
    draw_circle(
        walker.pos.x as f32 + 0.5,
        walker.pos.y as f32 + 0.5,
        0.25,
        colors::BLUE,
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

pub fn draw_waypoints(walker: &CuteWalker, color: Color, color_next: Color) {
    for (waypoint_index, waypoint_pos) in walker.waypoints.iter().enumerate() {
        let color = if waypoint_index == walker.goal_index {
            color_next
        } else {
            color
        };

        draw_circle(
            waypoint_pos.x as f32 + 0.5,
            waypoint_pos.y as f32 + 0.5,
            0.5,
            color,
        )
    }
}

pub fn draw_mouse_map_cell_pos(map_cam: &MapCamera) {
    let mouse_map_pos = map_cam.get_map_mouse_pos();
    draw_rectangle_lines(
        mouse_map_pos.x.floor(),
        mouse_map_pos.y.floor(),
        1.0,
        1.0,
        0.15,
        Color::new(1.0, 0.8, 0.2, 0.95),
    );
}

pub fn draw_font_layer(font_layer: &Array2<char>) {
    let text_params = TextParams {
        font_size: 100,
        font_scale: 0.01,
        color: colors::BLACK,
        ..Default::default()
    };
    // draw_text_ex("HELLO", 20.0, 20.0, text_params);
    for ((x, y), ch) in font_layer.indexed_iter() {
        if *ch == ' ' {
            continue;
        }
        draw_text_ex(
            &ch.to_string(),
            x as f32 + 0.25,
            y as f32 + 0.75,
            text_params,
        );
    }
}
