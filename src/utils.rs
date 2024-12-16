use crate::map::Map;
use crate::position::Position;
use ndarray::{s, Array2, ArrayViewMut2};

pub fn safe_slice_mut<'a, T>(
    grid: &'a mut Array2<T>,
    top_left: &Position,
    bot_right: &Position,
    map: &Map,
) -> Result<ArrayViewMut2<'a, T>, &'static str> {
    if !map.pos_in_bounds(top_left) || !map.pos_in_bounds(bot_right) {
        return Err("area out of bounds");
    }

    let area = grid.slice_mut(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

    Ok(area)
}
