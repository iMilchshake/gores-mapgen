use crate::config::MapConfig;
use crate::position::Position;
use crate::random::Seed;
use crate::{config::GenerationConfig, map::Map};
use ndarray::{s, Array2, ArrayView2, ArrayViewMut2};

pub fn safe_slice_mut<'a, T>(
    grid: &'a mut Array2<T>,
    top_left: &Position,
    bot_right: &Position,
    map: &Map,
) -> Result<ArrayViewMut2<'a, T>, &'static str> {
    if !map.pos_in_bounds(top_left) || !map.pos_in_bounds(bot_right) {
        return Err("safe_slice_mut accessed area out of bounds");
    }

    let area = grid.slice_mut(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

    Ok(area)
}

pub fn safe_slice<'a, T>(
    grid: &'a Array2<T>,
    top_left: &Position,
    bot_right: &Position,
    map: &Map,
) -> Result<ArrayView2<'a, T>, &'static str> {
    if !map.pos_in_bounds(top_left) || !map.pos_in_bounds(bot_right) {
        return Err("safe_slice accessed area out of bounds");
    }

    let area = grid.slice(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

    Ok(area)
}

/// Generate default filename
/// `<gen_config>-<map_config>-<seed>.map`
pub fn get_default_file_name(
    gen_config: &GenerationConfig,
    map_config: &MapConfig,
    seed: &Seed,
) -> String {
    format!(
        "{}-{}-{}.map",
        gen_config.name,
        map_config.name,
        seed.to_base64()
    )
}
