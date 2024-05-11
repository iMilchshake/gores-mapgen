use crate::{
    generator::Generator,
    map::{BlockType, Map, Overwrite},
    position::{Position, ShiftDirection},
};

use std::{f32::consts::SQRT_2, marker, usize};

use dt::dt_bool;
use ndarray::{s, Array2, ArrayBase, Dim, Ix2, ViewRepr};

pub fn is_freeze(block_type: &&BlockType) -> bool {
    **block_type == BlockType::Freeze
}

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
                if shape.iter().all(is_freeze) {
                    candidates.push((Position::new(window_x, window_y), dir));
                }
            }
        }
    }

    Ok(candidates)
}

/// if a skip has been found, this returns the end position
pub fn check_corner_skip(
    gen: &Generator,
    init_pos: &Position,
    shift: &ShiftDirection,
    tunnel_bounds: (usize, usize),
) -> Option<(Position, usize)> {
    let mut pos = init_pos.clone();

    let mut skip_length = 0;
    let mut stage = 0;
    while stage != 4 && skip_length < tunnel_bounds.1 {
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

        skip_length += 1;
    }

    if stage == 4 && skip_length > tunnel_bounds.0 {
        Some((pos, skip_length))
    } else {
        None
    }
}

pub fn generate_skip(
    gen: &mut Generator,
    start_pos: &Position,
    end_pos: &Position,
    shift: &ShiftDirection,
) {
    let top_left = Position::new(
        usize::min(start_pos.x, end_pos.x),
        usize::min(start_pos.y, end_pos.y),
    );
    let bot_right = Position::new(
        usize::max(start_pos.x, end_pos.x),
        usize::max(start_pos.y, end_pos.y),
    );

    gen.map.set_area(
        &top_left,
        &bot_right,
        &BlockType::Empty,
        &Overwrite::ReplaceSolidFreeze,
    );

    match shift {
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

pub fn generate_all_skips(
    gen: &mut Generator,
    length_bounds: (usize, usize),
    min_spacing_sqr: usize,
) {
    // get corner candidates
    let corner_candidates = find_corners(gen).expect("corner detection failed");

    // get possible skips
    let mut skips: Vec<(Position, Position, ShiftDirection, usize)> = Vec::new();
    for (start_pos, shift) in corner_candidates {
        if let Some((end_pos, length)) = check_corner_skip(gen, &start_pos, &shift, length_bounds) {
            skips.push((start_pos.clone(), end_pos, shift.clone(), length));
        }
    }

    // pick final selection of skips
    skips.sort_unstable_by(|s1, s2| usize::cmp(&s1.3, &s2.3)); // sort by length
    let mut valid_skips = vec![true; skips.len()];
    for skip_index in 0..skips.len() {
        // skip if already invalidated
        if !valid_skips[skip_index] {
            continue;
        }

        // skip is valid -> invalidate all following conflicting skips
        // TODO: right now skips can still cross each other
        let (start, end, _, _) = &skips[skip_index];
        for other_index in (skip_index + 1)..skips.len() {
            let (other_start, other_end, _, _) = &skips[other_index];

            if start.distance_squared(other_start) < min_spacing_sqr
                || start.distance_squared(other_end) < min_spacing_sqr
                || end.distance_squared(other_start) < min_spacing_sqr
                || end.distance_squared(other_start) < min_spacing_sqr
            {
                valid_skips[other_index] = false;
            }
        }
    }

    // generate all remaining valid skips
    for skip_index in 0..skips.len() {
        if valid_skips[skip_index] {
            let (start, end, shift, _) = &skips[skip_index];
            generate_skip(gen, start, end, shift);
        }
    }

    // set debug layer for valid skips
    let debug_skips = &mut gen.debug_layers.get_mut("skips").unwrap().grid;
    for ((start, end, _, _), valid) in skips.iter().zip(valid_skips.iter()) {
        if *valid {
            debug_skips[start.as_index()] = true;
            debug_skips[end.as_index()] = true;
        }
    }

    // set debug layer for invalid skips
    let debug_skips_invalid = &mut gen.debug_layers.get_mut("skips_invalid").unwrap().grid;
    for ((start, end, _, _), valid) in skips.iter().zip(valid_skips.iter()) {
        if !*valid {
            debug_skips_invalid[start.as_index()] = true;
            debug_skips_invalid[end.as_index()] = true;
        }
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

            // skip if not a freeze block
            if gen.map.grid[[x, y]] != BlockType::Freeze {
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
                for ((win_x, win_y), block_type) in window.indexed_iter() {
                    // skip current block
                    if win_x == 1 && win_y == 1 {
                        continue;
                    }

                    // blob is not unconnected -> abort
                    if block_type.is_solid() {
                        blob_unconnected = false;
                        break;
                    }

                    // queue neighboring unmarked & freeze blocks for visit
                    let abs_pos = Position::new(pos.x + win_x - 1, pos.y + win_y - 1);

                    // only consider freeze blocks
                    if !block_type.is_freeze() {
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

            if blob_unconnected {
                // dbg!(
                //     "found blob",
                //     &blob_visited,
                //     &blob_visit_next,
                //     &blob_size,
                //     &blob_visited.len()
                // );
                for visited_pos in blob_visited {
                    gen.debug_layers.get_mut("blobs").unwrap().grid[visited_pos.as_index()] = true;
                }
            }

            // gen.debug_layers.get_mut("blob_valid").unwrap().grid =
            //     invalid.map(|v| v.is_some_and(|v| !v));
            // gen.debug_layers.get_mut("blob_invalid").unwrap().grid =
            //     invalid.map(|v| v.is_some_and(|v| v));
            // gen.debug_layers.get_mut("blob_none").unwrap().grid = invalid.map(|v| v.is_none());
        }
    }
}
