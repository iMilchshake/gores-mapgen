use crate::CuteWalker;
use crate::{Draw, Vec2};
use ndarray::Array2;
use notan::draw::DrawShapes;
use notan::graphics::color::Color;

#[derive(Debug, Clone, Copy)]
pub enum BlockType {
    Empty,
    Filled,
}

pub fn draw_grid_blocks(
    draw: &mut Draw,
    grid: &Array2<BlockType>,
    display_factor: f32,
    display_shift: Vec2,
) {
    let width = grid.dim().0;
    let height = grid.dim().1;

    for x in 0..width {
        for y in 0..height {
            draw.rect(
                (
                    (x as f32) * display_factor + display_shift.x,
                    (y as f32) * display_factor + display_shift.y,
                ),
                (display_factor, display_factor),
            )
            .fill_color(match grid[[x, y]] {
                BlockType::Filled => Color::GREEN,
                _ => Color::GRAY,
            });
        }
    }
}

// pub fn draw_walker(walker: &CuteWalker, display_factor: f32, display_shift: Vec2) {
//     draw_rectangle_lines(
//         (walker.pos.x as f32) * display_factor + display_shift.x,
//         (walker.pos.y as f32) * display_factor + display_shift.y,
//         display_factor,
//         display_factor,
//         2.0,
//         YELLOW,
//     );
//     draw_circle(
//         walker.pos.x as f32 * display_factor + (display_factor / 2.),
//         walker.pos.y as f32 * display_factor + (display_factor / 2.),
//         display_factor * 0.25,
//         BLUE,
//     )
// }
