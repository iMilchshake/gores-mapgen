use crate::{
    config::GenerationConfig,
    debug::DebugLayer,
    generator::Generator,
    map::{BlockType, Map, Overwrite},
    position::{Position, ShiftDirection},
};

use std::{
    collections::{HashMap, VecDeque},
    f32::consts::SQRT_2,
    usize,
};

use dt::dt_bool;
use ndarray::{s, Array2, ArrayBase, Dim, Ix2, ViewRepr};

/// Post processing step to fix all existing edge-bugs, as certain inner/outer kernel
/// configurations do not ensure a min. 1-block freeze padding consistently.
pub fn fix_edge_bugs(gen: &mut Generator) -> Result<Array2<bool>, &'static str> {
    let mut edge_bug = Array2::from_elem((gen.map.width, gen.map.height), false);
    let width = gen.map.width;
    let height = gen.map.height;

    for x in 0..width {
        for y in 0..height {
            let value = &gen.map.grid[[x, y]];
            if *value == BlockType::Empty {
                for dx in 0..=2 {
                    for dy in 0..=2 {
                        if dx == 1 && dy == 1 {
                            continue;
                        }

                        let neighbor_x = (x + dx)
                            .checked_sub(1)
                            .ok_or("fix edge bug out of bounds")?;
                        let neighbor_y = (y + dy)
                            .checked_sub(1)
                            .ok_or("fix edge bug out of bounds")?;
                        if neighbor_x < width && neighbor_y < height {
                            let neighbor_value = &gen.map.grid[[neighbor_x, neighbor_y]];
                            if *neighbor_value == BlockType::Hookable {
                                edge_bug[[x, y]] = true;
                                // break;
                                // TODO: this should be easy to optimize
                            }
                        }
                    }
                }

                if edge_bug[[x, y]] {
                    gen.map.grid[[x, y]] = BlockType::Freeze;
                }
            }
        }
    }

    Ok(edge_bug)
}

/// Using a distance transform this function will fill up all empty blocks that are too far
/// from the next solid/non-empty block
pub fn fill_open_areas(gen: &mut Generator, max_distance: &f32) -> Array2<f32> {
    let grid = gen.map.grid.map(|val| *val != BlockType::Empty);

    // euclidean distance transform
    let distance = dt_bool::<f32>(&grid.into_dyn())
        .into_dimensionality::<Ix2>()
        .unwrap();

    gen.map
        .grid
        .zip_mut_with(&distance, |block_type, distance| {
            // only modify empty blocks
            if *block_type != BlockType::Empty {
                return;
            }

            if *distance > *max_distance + SQRT_2 {
                *block_type = BlockType::Hookable;
            } else if *distance > *max_distance {
                *block_type = BlockType::Freeze;
            }
        });

    distance
}

// returns a vec of corner candidates and their respective direction to the wall
pub fn find_corners(gen: &Generator) -> Result<Vec<(Position, ShiftDirection)>, &'static str> {
    let mut candidates: Vec<(Position, ShiftDirection)> = Vec::new();

    let width = gen.map.width;
    let height = gen.map.height;

    let window_size = 2; // 2 -> 5x5 windows

    for window_x in window_size..(width - window_size) {
        for window_y in window_size..(height - window_size) {
            let window = &gen.map.grid.slice(s![
                window_x - window_size..=window_x + window_size,
                window_y - window_size..=window_y + window_size
            ]);

            if window[[2, 2]] != BlockType::Empty {
                continue;
            }

            let shapes = [
                // R1
                (
                    [
                        &window[[2, 3]],
                        &window[[3, 0]],
                        &window[[3, 1]],
                        &window[[3, 2]],
                        &window[[3, 3]],
                    ],
                    ShiftDirection::Right,
                ),
                // R2
                (
                    [
                        &window[[2, 1]],
                        &window[[3, 1]],
                        &window[[3, 2]],
                        &window[[3, 3]],
                        &window[[3, 4]],
                    ],
                    ShiftDirection::Right,
                ),
                // L1
                (
                    [
                        &window[[2, 3]],
                        &window[[1, 0]],
                        &window[[1, 1]],
                        &window[[1, 2]],
                        &window[[1, 3]],
                    ],
                    ShiftDirection::Left,
                ),
                // L2
                (
                    [
                        &window[[2, 1]],
                        &window[[1, 1]],
                        &window[[1, 2]],
                        &window[[1, 3]],
                        &window[[1, 4]],
                    ],
                    ShiftDirection::Left,
                ),
                // U1
                (
                    [
                        &window[[3, 2]],
                        &window[[0, 1]],
                        &window[[1, 1]],
                        &window[[2, 1]],
                        &window[[3, 1]],
                    ],
                    ShiftDirection::Up,
                ),
                // U2
                (
                    [
                        &window[[1, 2]],
                        &window[[1, 1]],
                        &window[[2, 1]],
                        &window[[3, 1]],
                        &window[[4, 1]],
                    ],
                    ShiftDirection::Up,
                ),
                // D1
                (
                    [
                        &window[[3, 2]],
                        &window[[0, 3]],
                        &window[[1, 3]],
                        &window[[2, 3]],
                        &window[[3, 3]],
                    ],
                    ShiftDirection::Down,
                ),
                // D2
                (
                    [
                        &window[[1, 2]],
                        &window[[1, 3]],
                        &window[[2, 3]],
                        &window[[3, 3]],
                        &window[[4, 3]],
                    ],
                    ShiftDirection::Down,
                ),
            ];

            for (shape, dir) in shapes {
                if shape.iter().all(|b| b.is_freeze()) {
                    candidates.push((Position::new(window_x, window_y), dir));
                }
            }
        }
    }

    Ok(candidates)
}

/// Replace all map blocks with empty, that were not locked in the generation
pub fn remove_unused_blocks(map: &mut Map, position_lock: &Array2<bool>) {
    for (map_block, lock_status) in map.grid.iter_mut().zip(position_lock.iter()) {
        if !lock_status {
            *map_block = BlockType::Empty;
        }
    }
}

pub struct Skip {
    start_pos: Position,
    end_pos: Position,
    length: usize,
    direction: ShiftDirection,
}

/// if a skip has been found, this returns the end position and length
pub fn check_corner_skip(
    gen: &Generator,
    init_pos: &Position,
    shift: &ShiftDirection,
    tunnel_bounds: (usize, usize),
) -> Option<Skip> {
    let mut pos = init_pos.clone();

    let mut length = 0;
    let mut stage = 0;
    while stage != 4 && length < tunnel_bounds.1 {
        // shift into given direction, abort if invalid shift
        if pos.shift_in_direction(shift, &gen.map).is_err() {
            return None;
        };
        let curr_block_type = gen.map.grid.get(pos.as_index()).unwrap();

        stage = match (stage, curr_block_type) {
            // proceed to / or stay in stage 1 if freeze is found
            (0 | 1, BlockType::Freeze) => 1,

            // proceed to / or stay in stage 2 if hookable is found
            (1 | 2, BlockType::Hookable) => 2,

            // proceed to / or stay in stage 2 if freeze is found
            (2 | 3, BlockType::Freeze) => 3,

            // proceed to final state if (first) empty block is found
            (3, BlockType::Empty) => 4,

            // no match -> invalid sequence, abort!
            _ => return None,
        };

        length += 1;
    }

    if stage == 4 && length > tunnel_bounds.0 {
        Some(Skip {
            start_pos: init_pos.clone(),
            end_pos: pos,
            length,
            direction: shift.clone(),
        })
    } else {
        None
    }
}

pub fn count_skip_neighbours(
    gen: &mut Generator,
    skip: &Skip,
    offset: usize,
) -> Result<usize, &'static str> {
    let top_left = Position::new(
        usize::min(skip.start_pos.x, skip.end_pos.x),
        usize::min(skip.start_pos.y, skip.end_pos.y),
    );
    let bot_right = Position::new(
        usize::max(skip.start_pos.x, skip.end_pos.x),
        usize::max(skip.start_pos.y, skip.end_pos.y),
    );

    let offset: i32 = offset as i32;

    match skip.direction {
        ShiftDirection::Left | ShiftDirection::Right => {
            let bot_count = gen.map.count_occurence_in_area(
                &top_left.shifted_by(0, offset)?,
                &bot_right.shifted_by(0, offset)?,
                &BlockType::Hookable,
            )?;
            let top_count = gen.map.count_occurence_in_area(
                &top_left.shifted_by(0, -offset)?,
                &bot_right.shifted_by(0, -offset)?,
                &BlockType::Hookable,
            )?;

            Ok(usize::min(bot_count, top_count))
        }
        ShiftDirection::Up | ShiftDirection::Down => {
            let left_count = gen.map.count_occurence_in_area(
                &top_left.shifted_by(-offset, 0)?,
                &bot_right.shifted_by(-offset, 0)?,
                &BlockType::Hookable,
            )?;
            let right_count = gen.map.count_occurence_in_area(
                &top_left.shifted_by(offset, 0)?,
                &bot_right.shifted_by(offset, 0)?,
                &BlockType::Hookable,
            )?;

            Ok(usize::min(left_count, right_count))
        }
    }
}

pub fn generate_skip(gen: &mut Generator, skip: &Skip, block_type: &BlockType) {
    let top_left = Position::new(
        usize::min(skip.start_pos.x, skip.end_pos.x),
        usize::min(skip.start_pos.y, skip.end_pos.y),
    );
    let bot_right = Position::new(
        usize::max(skip.start_pos.x, skip.end_pos.x),
        usize::max(skip.start_pos.y, skip.end_pos.y),
    );

    gen.map.set_area(
        &top_left,
        &bot_right,
        block_type,
        &Overwrite::ReplaceSolidFreeze,
    );

    if block_type.is_freeze() {
        return;
    }

    match skip.direction {
        ShiftDirection::Left | ShiftDirection::Right => {
            gen.map.set_area(
                &top_left.shifted_by(0, -1).unwrap(),
                &bot_right.shifted_by(0, -1).unwrap(),
                &BlockType::Freeze,
                &Overwrite::ReplaceSolidOnly,
            );
            gen.map.set_area(
                &top_left.shifted_by(0, 1).unwrap(),
                &bot_right.shifted_by(0, 1).unwrap(),
                &BlockType::Freeze,
                &Overwrite::ReplaceSolidOnly,
            );
        }
        ShiftDirection::Up | ShiftDirection::Down => {
            gen.map.set_area(
                &top_left.shifted_by(-1, 0).unwrap(),
                &bot_right.shifted_by(-1, 0).unwrap(),
                &BlockType::Freeze,
                &Overwrite::ReplaceSolidOnly,
            );
            gen.map.set_area(
                &top_left.shifted_by(1, 0).unwrap(),
                &bot_right.shifted_by(1, 0).unwrap(),
                &BlockType::Freeze,
                &Overwrite::ReplaceSolidOnly,
            );
        }
    }
}

#[derive(Clone, PartialEq)]
enum SkipStatus {
    Invalid,
    ValidFreezeSkipOnly,
    Valid,
}

pub fn generate_all_skips(
    gen: &mut Generator,
    length_bounds: (usize, usize),
    min_spacing_sqr: usize,
    max_level_skip: usize,
    flood_fill: &Array2<Option<usize>>,
) {
    // get corner candidates
    let corner_candidates = find_corners(gen).expect("corner detection failed");

    // get possible skips
    let mut skips: Vec<Skip> = Vec::new();
    for (start_pos, shift) in corner_candidates {
        if let Some(skip) = check_corner_skip(gen, &start_pos, &shift, length_bounds) {
            skips.push(skip);
        }
    }

    // pick final selection of skips
    skips.sort_unstable_by(|s1, s2| usize::cmp(&s1.length, &s2.length)); // sort by length
    let mut skip_status = vec![SkipStatus::Valid; skips.len()];
    for skip_index in 0..skips.len() {
        // skip if already invalidated
        if skip_status[skip_index] == SkipStatus::Invalid {
            continue;
        }

        let skip = &skips[skip_index];

        // check if too much of the level would be skipped
        let level_distance_start = flood_fill[skip.start_pos.as_index()].unwrap();
        let level_distance_end = flood_fill[skip.end_pos.as_index()].unwrap();
        let level_skip_distance = usize::abs_diff(level_distance_start, level_distance_end);
        if level_skip_distance > max_level_skip {
            skip_status[skip_index] = SkipStatus::Invalid;
            continue;
        }

        // invalidate if skip would have no neighboring blocks
        if count_skip_neighbours(gen, skip, 2).unwrap_or(0) <= 0 {
            // if yes, test if freeze skip would have neighboring blocks
            if count_skip_neighbours(gen, skip, 1).unwrap_or(0) >= 1 {
                skip_status[skip_index] = SkipStatus::ValidFreezeSkipOnly;
            } else {
                // if both are not the case -> invalidate
                skip_status[skip_index] = SkipStatus::Invalid;
                continue;
            }
        }

        // skip is valid -> invalidate all following conflicting skips
        // TODO: right now skips can still cross each other
        // TODO: i feel like i need a config seperation between skips and freeze skips
        //       would be nice to not have freeze invalidate actual skips, and have different
        //       length
        for other_index in (skip_index + 1)..skips.len() {
            let skip_other = &skips[other_index];

            // check if skips are too close
            if skip.start_pos.distance_squared(&skip_other.start_pos) < min_spacing_sqr
                || skip.start_pos.distance_squared(&skip_other.end_pos) < min_spacing_sqr
                || skip.end_pos.distance_squared(&skip_other.start_pos) < min_spacing_sqr
                || skip.end_pos.distance_squared(&skip_other.end_pos) < min_spacing_sqr
            {
                skip_status[other_index] = SkipStatus::Invalid;
            }
        }
    }

    // generate all remaining valid skips
    for skip_index in 0..skips.len() {
        match skip_status[skip_index] {
            SkipStatus::Valid => generate_skip(gen, &skips[skip_index], &BlockType::Empty),
            SkipStatus::ValidFreezeSkipOnly => {
                generate_skip(gen, &skips[skip_index], &BlockType::Freeze)
            }
            _ => (),
        }
    }

    // add debug visualizations
    for (skip, status) in skips.iter().zip(skip_status.iter()) {
        let debug_layer = match *status {
            SkipStatus::Valid => gen.debug_layers.get_mut("skips").unwrap(),
            SkipStatus::Invalid => gen.debug_layers.get_mut("skips_invalid").unwrap(),
            SkipStatus::ValidFreezeSkipOnly => gen.debug_layers.get_mut("freeze_skips").unwrap(),
        };

        debug_layer.grid[skip.start_pos.as_index()] = true;
        debug_layer.grid[skip.end_pos.as_index()] = true;
    }
}

pub fn get_window<T>(
    grid: &Array2<T>,
    x: usize,
    y: usize,
    window_size: usize,
) -> ArrayBase<ViewRepr<&T>, Dim<[usize; 2]>> {
    grid.slice(s![
        x - window_size..=x + window_size,
        y - window_size..=y + window_size
    ])
}

/// removes unconnected/isolated that are smaller in size than given minimal threshold
pub fn remove_freeze_blobs(gen: &mut Generator, min_freeze_size: usize) {
    let width = gen.map.width;
    let height = gen.map.height;

    // keeps track of which blocks are (in)valid. Valid blocks are isolated freeze block that are
    // not directly connected to any solid blocks. Invalid blocks are (in)directly connected to
    // solid blocks. None just means, that we dont know yet.
    let mut invalid = Array2::<Option<bool>>::from_elem(gen.map.grid.dim(), None);

    let window_size = 1; // 1 -> 3x3 windows
    for x in window_size..(width - window_size) {
        for y in window_size..(height - window_size) {
            // skip if already processed
            if invalid[[x, y]].is_some() {
                continue;
            }

            // invalidate neighboring blocks to hookables
            let block_type = &gen.map.grid[[x, y]];

            // invalidate freeze blocks next to hookable so they arent checked
            // TODO: In theory this should be a nice speedup, but in pracise i should replace this with a
            // much better two sweep approach. Idea: Do a post processing step which detects
            // 'wall'-freezes. this information can then be used in various other steps.
            if *block_type == BlockType::Hookable {
                invalid
                    .slice_mut(s![x - 1..=x + 1, y - 1..=y + 1])
                    .fill(Some(true));
                continue;
            }

            // skip if not a freeze block
            if *block_type != BlockType::Freeze {
                continue;
            }

            // check all freeze blocks that are connected to the current block
            let mut blob_visited = Vec::<Position>::new();
            let mut blob_visit_next = vec![Position::new(x, y)];
            let mut blob_unconnected = true; // for now we assume that the current blob is unconnected
            let mut blob_size = 0;
            while blob_unconnected && !blob_visit_next.is_empty() {
                let pos = blob_visit_next.pop().unwrap();
                invalid[pos.as_index()] = Some(false); // for now we assume that current block is valid

                // check neighborhood
                let window = get_window(&gen.map.grid, pos.x, pos.y, window_size);
                for ((win_x, win_y), other_block_type) in window.indexed_iter() {
                    // skip current block
                    if win_x == 1 && win_y == 1 {
                        continue;
                    }

                    // blob is not unconnected -> abort
                    if other_block_type.is_solid() {
                        blob_unconnected = false;
                        break;
                    }

                    // queue neighboring unmarked & freeze blocks for visit
                    let abs_pos = Position::new(pos.x + win_x - 1, pos.y + win_y - 1);

                    // only consider freeze blocks
                    if !other_block_type.is_freeze() {
                        continue;
                    }

                    // check if block has already been checked
                    if let Some(invalid) = invalid[abs_pos.as_index()] {
                        if invalid {
                            // block has already been invalidated -> abort
                            blob_unconnected = false;
                            break;
                        } else {
                            // block has already been validated -> skip
                            continue;
                        }
                    }

                    // queue block for visit
                    blob_visit_next.push(abs_pos);
                }

                // valid block, finalize
                blob_visited.push(pos);
                blob_size += 1;
            }

            // if blob is connected, invalidate all visited and future blocks that would have
            // been part of the blob
            if !blob_unconnected {
                for pos in &blob_visited {
                    invalid[pos.as_index()] = Some(true);
                }
                for pos in &blob_visit_next {
                    invalid[pos.as_index()] = Some(true);
                }
            }

            // unconnected blob has been found
            if blob_unconnected {
                for visited_pos in blob_visited {
                    gen.debug_layers.get_mut("blobs").unwrap().grid[visited_pos.as_index()] = true;

                    // remove small blobs
                    if blob_size < min_freeze_size {
                        gen.map.grid[visited_pos.as_index()] = BlockType::Empty;
                    }
                }
            }
        }
    }
}

pub fn get_flood_fill(gen: &Generator, start_pos: &Position) -> Array2<Option<usize>> {
    let width = gen.map.width;
    let height = gen.map.height;
    let mut distance = Array2::from_elem((width, height), None);
    let mut queue = VecDeque::new();

    let solid = gen.map.grid.map(|val| val.is_solid() || val.is_freeze());

    // TODO: error
    if solid[start_pos.as_index()] {
        return distance;
    }

    queue.push_back((start_pos.clone(), 0));
    distance[start_pos.as_index()] = Some(0);

    while let Some((pos, dist)) = queue.pop_front() {
        let neighbors = [
            pos.shifted_by(-1, 0),
            pos.shifted_by(1, 0),
            pos.shifted_by(0, -1),
            pos.shifted_by(0, 1),
        ];

        for neighbor in neighbors.iter() {
            if let Ok(neighbor_pos) = neighbor {
                if gen.map.pos_in_bounds(&neighbor_pos) {
                    if !solid[neighbor_pos.as_index()]
                        && distance[neighbor_pos.as_index()].is_none()
                    {
                        distance[neighbor_pos.as_index()] = Some(dist + 1);
                        queue.push_back((neighbor_pos.clone(), dist + 1));
                    }
                }
            }
        }
    }

    distance
}

/// stores all relevant information about platform candidates
#[derive(Debug, Clone)]
pub struct Platform {
    /// how total height is available for platform generation
    pub available_height: usize,

    /// how much platform extends to the left
    pub width_left: usize,

    /// how much platform extends to the right
    pub width_right: usize,

    /// lowest center position of platform
    pub pos: Position,
}

pub fn get_optimal_greedy_platform_candidate(
    pos: &Position,
    map: &Map,
    gen_config: &GenerationConfig,
) -> Result<Platform, &'static str> {
    // how far empty box has been extended
    let mut left_limit = 0;
    let mut right_limit = 0;
    let mut up_limit = 0;

    // which directions are already locked due to hitting a limit
    let mut left_locked = false;
    let mut right_locked = false;
    let mut up_locked = false;

    while !left_locked || !right_locked || !up_locked {
        // try to expand upwards
        if !up_locked {
            let next_limit_valid = map.check_area_all(
                &pos.shifted_by(-left_limit, -(up_limit + 1))?,
                &pos.shifted_by(right_limit, -(up_limit + 1))?,
                &BlockType::Empty,
            )?;

            if next_limit_valid {
                up_limit += 1;
            } else {
                up_locked = true;
            }
        }

        // try to expand left
        if !left_locked {
            // check if platform has no overhang
            let no_overhang =
                map.check_position_crit(&pos.shifted_by(-(left_limit + 1), 1)?, |b| {
                    match (gen_config.plat_soft_overhang, b) {
                        (true, block) => !block.is_empty(),
                        (false, block) => block.is_solid(),
                    }
                });

            let next_limit_valid = map.check_area_all(
                &pos.shifted_by(-(left_limit + 1), -up_limit)?,
                &pos.shifted_by(-(left_limit + 1), -1)?, // dont check y=0 as freeze expected
                &BlockType::Empty,
            )?;

            if no_overhang && next_limit_valid {
                left_limit += 1;
            } else {
                left_locked = true;
            }
        }

        // try to expand right
        if !right_locked {
            let no_overhang = map.check_position_crit(&pos.shifted_by(right_limit + 1, 1)?, |b| {
                match (gen_config.plat_soft_overhang, b) {
                    (true, block) => !block.is_empty(),
                    (false, block) => block.is_solid(),
                }
            });
            let next_limit_valid = map.check_area_all(
                &pos.shifted_by(right_limit + 1, -up_limit)?,
                &pos.shifted_by(right_limit + 1, -1)?, // dont check y=0 as freeze expected
                &BlockType::Empty,
            )?;

            if no_overhang && next_limit_valid {
                right_limit += 1;
            } else {
                right_locked = true;
            }
        }

        // early abort if x or y dimension is already locked, but lower bound isnt reached
        if up_locked
            && (((up_limit + 1) as usize)
                < gen_config.plat_height_bounds.0 + gen_config.plat_min_empty_height)
        {
            return Err("not enough y space");
        } else if left_locked
            && right_locked
            && (((left_limit + right_limit + 1) as usize) < gen_config.plat_width_bounds.0)
        {
            return Err("not enough x space");
        }
        if ((up_limit + 1) as usize)
            >= (gen_config.plat_height_bounds.1 + gen_config.plat_min_empty_height)
        {
            up_locked = true;
        }
        if ((left_limit + right_limit + 1) as usize) >= gen_config.plat_width_bounds.1 {
            left_locked = true;
            right_locked = true;
        }
    }

    Ok(Platform {
        pos: pos.clone(),
        width_left: left_limit as usize,
        width_right: right_limit as usize,
        available_height: (up_limit + 1) as usize,
    })
}

pub fn gen_all_platform_candidates(
    walker_pos_history: &Vec<Position>,
    flood_fill: &Array2<Option<usize>>,
    map: &mut Map,
    gen_config: &GenerationConfig,
    debug_layers: &mut HashMap<&'static str, DebugLayer>,
) {
    let mut platform_candidates: Vec<Platform> = Vec::new();
    let mut last_platform_level_distance = 0;

    for pos_index in 0..walker_pos_history.len() {
        let pos = &walker_pos_history[pos_index];

        // skip if initial walker pos is non empty
        if map.grid[pos.as_index()] != BlockType::Empty {
            continue;
        }

        // skip if previous platform is still to close
        let level_distance = flood_fill[pos.as_index()].unwrap();
        if level_distance.saturating_sub(last_platform_level_distance)
            < gen_config.plat_min_distance
        {
            continue;
        }

        // skip if floor pos coulnt be determined
        let floor_pos = map.shift_pos_until(pos, ShiftDirection::Down, |b| b.is_solid());
        if floor_pos.is_none() {
            continue;
        }
        let floor_pos = floor_pos.unwrap();

        // try to get optimal platform candidate
        let platform_pos = floor_pos.shifted_by(0, -1).unwrap();
        let result = get_optimal_greedy_platform_candidate(&platform_pos, map, gen_config);
        if let Ok(platform_candidate) = result {
            // draw debug
            let platforms_walker_pos = debug_layers.get_mut("platforms_walker_pos").unwrap();
            platforms_walker_pos.grid[pos.as_index()] = true;
            let platforms_floor_pos = debug_layers.get_mut("platforms_floor_pos").unwrap();
            platforms_floor_pos.grid[floor_pos.as_index()] = true;
            let platforms_pos = debug_layers.get_mut("platforms_pos").unwrap();
            platforms_pos.grid[platform_pos.as_index()] = true;
            let platform_debug_layer = debug_layers.get_mut("platforms").unwrap();
            let mut area = platform_debug_layer.grid.slice_mut(s![
                platform_pos.x - platform_candidate.width_left
                    ..=platform_pos.x + platform_candidate.width_right,
                platform_pos.y - (platform_candidate.available_height - 1)..=platform_pos.y
            ]);
            area.fill(true);

            // save platform
            platform_candidates.push(platform_candidate);

            // update last level distance
            last_platform_level_distance = level_distance;
        }
    }

    // generate platforms
    for platform_candidate in platform_candidates {
        let platform_height =
            platform_candidate.available_height - gen_config.plat_min_empty_height;

        if platform_height > 0 {
            map.set_area(
                &platform_candidate
                    .pos
                    .shifted_by(
                        -(platform_candidate.width_left as i32),
                        -(platform_height as i32),
                    )
                    .unwrap(),
                &platform_candidate
                    .pos
                    .shifted_by(platform_candidate.width_right as i32, 0)
                    .unwrap(),
                &BlockType::Platform,
                &Overwrite::Force,
            );
        }

        map.set_area(
            &platform_candidate
                .pos
                .shifted_by(
                    -(platform_candidate.width_left as i32),
                    -((platform_candidate.available_height - 1) as i32),
                )
                .unwrap(),
            &platform_candidate
                .pos
                .shifted_by(
                    platform_candidate.width_right as i32,
                    -(platform_height as i32),
                )
                .unwrap(),
            &BlockType::EmptyReserved,
            &Overwrite::Force,
        );
    }
}
