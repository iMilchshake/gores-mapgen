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
    for ((x, y), value) in grid.indexed_iter() {
        draw.rect(
            (
                (x as f32) * display_factor + display_shift.x,
                (y as f32) * display_factor + display_shift.y,
            ),
            (display_factor, display_factor),
        )
        .fill_color(match value {
            BlockType::Filled => Color::GREEN,
            _ => Color::GRAY,
        });
    }
}

pub fn draw_walker(draw: &mut Draw, walker: &CuteWalker, display_factor: f32, display_shift: Vec2) {
    draw.rect(
        (
            (walker.pos.x as f32) * display_factor + display_shift.x,
            (walker.pos.y as f32) * display_factor + display_shift.y,
        ),
        (display_factor, display_factor),
    )
    .stroke_color(Color::YELLOW)
    .stroke(f32::max(1.0, display_factor / 5.));

    // draw_circle(
    //     walker.pos.x as f32 * display_factor + (display_factor / 2.),
    //     walker.pos.y as f32 * display_factor + (display_factor / 2.),
    //     display_factor * 0.25,
    //     BLUE,
    // )
}
