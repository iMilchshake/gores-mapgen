use crate::{
    config::{GenerationConfig, ThemeConfig},
    debug::DebugLayers,
    generator::Generator,
    map::{BlockType, Map, Overwrite},
    noise,
    position::{Position, ShiftDirection},
    random::Random,
    utils::{safe_slice, safe_slice_mut},
};

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, VecDeque},
    f32::consts::SQRT_2,
};

use dt::dt_bool;
use ndarray::{s, Array2, ArrayBase, Dim, Ix2, ViewRepr};

/// Post processing step to fix all existing edge-bugs, as certain inner/outer kernel
/// configurations do not ensure a min. 1-block freeze padding consistently.
/// This function replaces all empty blocks that have neighbor hookable blocks with freeze,
/// so it kind of "expands" the existing freeze to ensure that there are no edge bugs.
pub fn fix_edge_bugs_expanding(gen: &mut Generator) -> Result<Array2<bool>, &'static str> {
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
                    // this doesnt break chunking, as we only consider
                    // Empty (and therefore already edited) cells
                    gen.map.grid[[x, y]] = BlockType::Freeze;
                }
            }
        }
    }

    Ok(edge_bug)
}

/// given an empty rectangle, ensure that there are no adjacent hookable blocks.
/// This is relevant when modifying the map by setting Empty, as it can remove the
/// layer of freeze blocks around hookable blocks. This function replaces those
/// hookable blocks with freeze, to retain the empty-area and freeze padding.
fn fix_local_edge_bugs(map: &mut Map, top_left: &Position, bot_right: &Position) {
    // vertical borders
    for y in top_left.y..=bot_right.y {
        // left neighbour
        if top_left.x > 0 {
            let x = top_left.x - 1;
            if map.grid[[x, y]] == BlockType::Hookable {
                map.grid[[x, y]] = BlockType::Freeze;
            }
        }
        // right neighbour
        let x = bot_right.x + 1;
        if map.grid[[x, y]] == BlockType::Hookable {
            map.grid[[x, y]] = BlockType::Freeze;
        }
    }

    // horizontal border above
    if top_left.y > 0 {
        let y_above = top_left.y - 1;
        let start_x = top_left.x.saturating_sub(1);
        let end_x = bot_right.x + 1;
        for x in start_x..=end_x {
            if map.grid[[x, y_above]] == BlockType::Hookable {
                map.grid[[x, y_above]] = BlockType::Freeze;
            }
        }
    }

    // TODO: function directly changes Hookable blocks outside of playable baths, CAN break chunked
    // TODO: currently i dont check below
}

/// Using a distance transform this function will fill up all empty blocks that are too far
/// from the next solid/non-empty block
pub fn fill_open_areas(
    gen: &mut Generator,
    max_distance: &f32,
    debug_layers: &mut Option<DebugLayers>,
) -> Array2<f32> {
    let grid = gen.map.grid.map(|val| *val != BlockType::Empty);

    // euclidean distance transform
    let distance = dt_bool::<f32>(&grid.into_dyn())
        .into_dimensionality::<Ix2>()
        .unwrap();

    if let Some(debug_layers) = debug_layers {
        debug_layers.float_layers.get_mut("dt").unwrap().grid =
            distance.map(|v| if *v > 0.0 { Some(*v) } else { None });
    }

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

#[derive(Debug)]
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
        if pos.shift_inplace(shift, &gen.map).is_err() {
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
            direction: *shift,
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
        &Overwrite::ReplaceHookableFreeze,
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
                &Overwrite::ReplaceHookableOnly,
            );
            gen.map.set_area(
                &top_left.shifted_by(0, 1).unwrap(),
                &bot_right.shifted_by(0, 1).unwrap(),
                &BlockType::Freeze,
                &Overwrite::ReplaceHookableOnly,
            );
        }
        ShiftDirection::Up | ShiftDirection::Down => {
            gen.map.set_area(
                &top_left.shifted_by(-1, 0).unwrap(),
                &bot_right.shifted_by(-1, 0).unwrap(),
                &BlockType::Freeze,
                &Overwrite::ReplaceHookableOnly,
            );
            gen.map.set_area(
                &top_left.shifted_by(1, 0).unwrap(),
                &bot_right.shifted_by(1, 0).unwrap(),
                &BlockType::Freeze,
                &Overwrite::ReplaceHookableOnly,
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
    debug_layers: &mut Option<DebugLayers>,
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
        if count_skip_neighbours(gen, skip, 2).unwrap_or(0) == 0 {
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
    if let Some(debug_layers) = debug_layers {
        for (skip, status) in skips.iter().zip(skip_status.iter()) {
            let debug_layer = match *status {
                SkipStatus::Valid => debug_layers.bool_layers.get_mut("skips").unwrap(),
                SkipStatus::Invalid => debug_layers.bool_layers.get_mut("skips_invalid").unwrap(),
                SkipStatus::ValidFreezeSkipOnly => {
                    debug_layers.bool_layers.get_mut("freeze_skips").unwrap()
                }
            };

            debug_layer.grid[skip.start_pos.as_index()] = true;
            debug_layer.grid[skip.end_pos.as_index()] = true;
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
pub fn remove_freeze_blobs(
    gen: &mut Generator,
    min_freeze_size: usize,
    debug_layers: &mut Option<DebugLayers>,
) {
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
                if let Some(debug_layers) = debug_layers {
                    for visited_pos in blob_visited {
                        debug_layers.bool_layers.get_mut("blobs").unwrap().grid
                            [visited_pos.as_index()] = true;

                        // remove small blobs
                        if blob_size < min_freeze_size {
                            gen.map.grid[visited_pos.as_index()] = BlockType::Empty;
                        }
                    }
                }
            }
        }
    }
}

pub struct FloodFillResult {
    /// tracks distance from each cell to `start_pos`
    pub distance: Array2<Option<usize>>,

    /// tracks from which adjacent cell each cell was visited, only if `end_pos` is set
    pub come_from: Option<Array2<Option<ShiftDirection>>>,

    /// reconstruct path from `start_pos` to `end_pos` using `come_from`, only if `end_pos` is set
    pub path: Option<Vec<Position>>,
}

/// flood fill implementation with optional multi-start or direction tracking.
/// to enable direction tracking, just provide an `end_pos`
pub fn flood_fill(
    gen: &Generator,
    start_pos: &[Position],
    end_pos: Option<&Position>,
    fill_freeze: bool,
) -> Result<FloodFillResult, &'static str> {
    let width = gen.map.width;
    let height = gen.map.height;
    let mut distance = Array2::from_elem((width, height), None);
    let mut queue = VecDeque::new();

    // track from where a cell was visited first
    let mut come_from: Option<Array2<Option<ShiftDirection>>> = if end_pos.is_some() {
        Some(Array2::from_elem((width, height), None))
    } else {
        None
    };

    let blocked_positions = gen
        .map
        .grid
        .map(|val| val.is_solid() || (!fill_freeze && val.is_freeze()));

    // initialize all start positions
    for pos in start_pos {
        if blocked_positions[pos.as_index()] {
            return Err("floodfill started on blocked position");
        }
        queue.push_back((pos.clone(), 0));
        distance[pos.as_index()] = Some(0);
    }

    while let Some((pos, dist)) = queue.pop_front() {
        let shifts = [
            ShiftDirection::Right,
            ShiftDirection::Down,
            ShiftDirection::Up,
            ShiftDirection::Left,
        ];

        for shift in shifts.iter() {
            let pos_neighbor = pos.shifted(shift, &gen.map)?;
            if gen.map.pos_in_bounds(&pos_neighbor)
                && !blocked_positions[pos_neighbor.as_index()]
                && distance[pos_neighbor.as_index()].is_none()
            {
                distance[pos_neighbor.as_index()] = Some(dist + 1);

                if let Some(from) = come_from.as_mut() {
                    from[pos_neighbor.as_index()] = Some(*shift);
                }

                queue.push_back((pos_neighbor, dist + 1));
            }
        }
    }

    // get fastest path from start to finish
    let path = if let Some(end_pos) = end_pos {
        let mut pos = end_pos.clone();
        let num_steps = distance[pos.as_index()].unwrap();
        let from = come_from.as_ref().unwrap();
        let mut path_grid: Array2<bool> = Array2::from_elem((gen.map.width, gen.map.height), false);
        let mut path: Vec<Position> = vec![end_pos.clone()];

        for _ in 0..num_steps {
            let shift = from[pos.as_index()].unwrap().get_opposite();
            pos.shift_inplace(&shift, &gen.map)?;
            path_grid[pos.as_index()] = true;
            path.push(pos.clone());
        }

        Some(path)
    } else {
        None
    };

    Ok(FloodFillResult {
        distance,
        come_from,
        path,
    })
}

pub fn dijkstra(
    map: &Map,
    start: &Position,
    end: &Position,
    debug_layers: &mut Option<DebugLayers>,
) -> Result<(), &'static str> {
    let mut open_cells: BinaryHeap<Reverse<(u32, Position)>> = BinaryHeap::new();
    open_cells.push(Reverse((0, start.clone())));
    let mut best_dist: HashMap<Position, u32> = HashMap::new();
    best_dist.insert(start.clone(), 0);

    while let Some(Reverse((g, pos))) = open_cells.pop() {
        if let Some(&current) = best_dist.get(&pos) {
            if g > current {
                continue;
            }
        }

        for shift in [
            ShiftDirection::Right,
            ShiftDirection::Up,
            ShiftDirection::Left,
            ShiftDirection::Down,
        ] {
            let neighbor = pos.shifted(&shift, map)?;

            if neighbor == *end {
                println!("goal found :) ");
                return Ok(());
            }

            // we only consider "playable" blocks aka. parts the player can pass (except freeze)
            if !matches!(
                map.grid[neighbor.as_index()],
                BlockType::Empty
                    | BlockType::EmptyRoom
                    | BlockType::EmptyFade
                    | BlockType::EmptyPlatform
                    | BlockType::Start
                    | BlockType::Finish
            ) {
                continue;
            }

            let new_cost = g + 1;
            if best_dist
                .get(&neighbor)
                .map_or(true, |&cost| new_cost < cost)
            {
                best_dist.insert(neighbor.clone(), new_cost);
                open_cells.push(Reverse((new_cost, neighbor)));
            }
        }

        if let Some(debug_layers) = debug_layers {
            debug_layers.bool_layers.get_mut("dijkstra").unwrap().grid[pos.as_index()] = true;
        }
    }

    Ok(())
}

pub fn generate_noise_layers(
    map: &mut Map,
    rnd: &mut Random,
    thm_config: &ThemeConfig,
    debug_layers: &mut Option<DebugLayers>,
) {
    map.noise_overlay = Some(noise::generate_noise_array(
        map,
        thm_config.overlay_noise_scale,
        thm_config.overlay_noise_invert,
        thm_config.overlay_noise_threshold,
        thm_config.overlay_noise_type,
        true,
        false,
        rnd.get_u32(),
    ));
    let noise_background = noise::generate_noise_array(
        map,
        thm_config.background_noise_scale,
        thm_config.background_noise_invert,
        thm_config.background_noise_threshold,
        thm_config.background_noise_type,
        false,
        true,
        rnd.get_u32(),
    );
    map.noise_background = Some(noise::opening(&noise::closing(&noise_background)));

    if let Some(debug_layers) = debug_layers {
        debug_layers.bool_layers.get_mut("noise_o").unwrap().grid =
            map.noise_overlay.clone().unwrap();
    }
    if let Some(debug_layers) = debug_layers {
        debug_layers.bool_layers.get_mut("noise_b").unwrap().grid =
            map.noise_background.clone().unwrap();
    }
}

/// prototype for general purpose pattern detection. I dont need this right now, but i'll leave it
/// here for future me :)
// pub fn detect_pattern(map: &mut Map) {
//     todo!();
//     type BlockTypePredicate = fn(&BlockType) -> bool;
//     #[rustfmt::skip]
//     const PATTERN: [[BlockTypePredicate; 3]; 3] = [
//         [BlockType::is_empty, BlockType::is_solid, BlockType::is_empty],
//         [BlockType::is_solid, BlockType::is_empty, BlockType::is_solid],
//         [BlockType::is_empty, BlockType::is_solid, BlockType::is_empty],
//     ];
// }

/// Fix diagonal staircase patterns
///
/// using cityblock distance based floodfill for dead-end removal results in 'perfect' staircases,
/// this is a consistent pattern that collides with the generators philosophy of generating no
/// recognizable patterns.
/// This function should detect and fix these staircase artifacts.
/// A staircase looks like this, X's being solid and _ empty.
/// X X X
/// X X _
/// X _ _
/// to not introduce new too recognizable patterns, stairs are fixed by either removing the center
/// block, or all hookable blocks.
pub fn fix_stairs(map: &mut Map, filled_positions: Vec<Position>, rnd: &mut Random) {
    for pos in filled_positions.iter() {
        let stair = detect_stair(map, pos);

        if stair.is_some() {
            if rnd.get_bool_with_prob(0.5) {
                // = 50%
                // remove center block
                map.grid[pos.as_index()] = BlockType::Empty;
            } else if rnd.get_bool_with_prob(0.5) {
                // = 25%
                // remove all hookable blocks
                map.set_area(
                    &pos.shifted_by(-1, -1).unwrap(),
                    &pos.shifted_by(1, 1).unwrap(),
                    &BlockType::Empty,
                    &Overwrite::ReplaceHookableOnly,
                );
            }

            // = 25%
            // do nothing :)
        }
    }
}

/// helper function for `fix_stairs`
/// checks if stair pattern is present at given position.
/// if yes, returns the empty corner.
pub fn detect_stair(map: &Map, pos: &Position) -> Option<(i32, i32)> {
    let mut corner = None;

    // check if center block is solid
    if !map.grid[pos.as_index()].is_solid() {
        return None; // no stair
    }

    // check if exactly one corner is empty
    for x_shift in [-1, 1] {
        for y_shift in [-1, 1] {
            let corner_pos = pos.shifted_by(x_shift, y_shift).unwrap();
            let corner_block_type = &map.grid[corner_pos.as_index()];

            if corner_block_type.is_empty() {
                if corner.is_none() {
                    corner = Some((x_shift, y_shift));
                } else {
                    // second empty corner was found!
                    return None; // no stair
                }
            } else if !corner_block_type.is_solid() {
                // a non solid/empty block occured
                return None; // no stair
            }
        }
    }

    // check if no empty corner found
    corner?;
    let corner = corner.unwrap();

    // ensure that neighboring non diagonal cells are also empty
    let neighbor_pos1 = pos.shifted_by(corner.0, 0).unwrap();
    let neighbor_pos2 = pos.shifted_by(0, corner.1).unwrap();
    if !map.grid[neighbor_pos1.as_index()].is_empty()
        || !map.grid[neighbor_pos2.as_index()].is_empty()
    {
        return None; // no stair
    }

    // ensure that opposite non diagonal cells are solid
    let opposite_pos1 = pos.shifted_by(-corner.0, 0).unwrap();
    let opposite_pos2 = pos.shifted_by(0, -corner.1).unwrap();
    if !map.grid[opposite_pos1.as_index()].is_solid()
        || !map.grid[opposite_pos2.as_index()].is_solid()
    {
        return None; // no stair
    }

    // all checks passes, this is a stair!
    Some(corner)
}

pub fn generate_finish_room(
    pos: &Position,
    map: &mut Map,
    locked_positions: &Array2<bool>,
    ff_dist: &Array2<Option<usize>>,
    room_size: usize,
) -> Result<(), &'static str> {
    let room_size: i32 = room_size as i32;

    let top_left = pos.shifted_by(-room_size, -room_size)?;
    let bot_right = pos.shifted_by(room_size, room_size)?;

    // check if area already locked
    let area_locked = safe_slice(locked_positions, &top_left, &bot_right, &map)?;
    let locked = area_locked.iter().any(|v| *v);
    if locked {
        // if its locked, we ensure that we actually overlap with playable parts

        let flood_fill_area = safe_slice(
            ff_dist,
            &top_left.shifted_by(-1, -1)?,
            &bot_right.shifted_by(1, 1)?,
            &map,
        )?;
        let min_ff_dist = flood_fill_area.iter().filter_map(|v| *v).min().unwrap();
        let goal_ff_dist = ff_dist[pos.as_index()].unwrap();
        let ff_diff = goal_ff_dist.saturating_sub(min_ff_dist);

        // TODO: introduce constant?
        if ff_diff > 40 {
            return Err("Cant place finish room, overlapping with playable part!");
        }
    }

    // carve room
    map.set_area(
        &top_left,
        &bot_right,
        &BlockType::EmptyRoom,
        &Overwrite::Force,
    );

    // set start/finish line
    map.set_area_border(
        &top_left.shifted_by(-1, -1)?,
        &bot_right.shifted_by(1, 1)?,
        &BlockType::Finish,
        &Overwrite::ReplaceNonSolid,
    );

    map.write_text(&pos.shifted_by(-2, 0)?, "GG :0");

    Ok(())
}

pub fn fill_dead_ends(
    map: &mut Map,
    gen_config: &GenerationConfig,
    main_path_distance: &Array2<Option<usize>>,
) -> Result<Vec<Position>, &'static str> {
    let mut filled_blocks = Vec::new();

    for x in 0..map.width {
        for y in 0..map.height {
            let block = &map.grid[(x, y)];

            if block != &BlockType::Empty && block != &BlockType::Freeze {
                continue;
            }

            if map.check_area_exists(
                &Position::new(x - 1, y - 1),
                &Position::new(x + 1, y + 1),
                &BlockType::EmptyFade,
            )? {
                continue;
            }

            // if too far, fill up with hookables.
            if let Some(dist) = main_path_distance[[x, y]] {
                if dist > gen_config.dead_end_threshold {
                    map.grid[(x, y)] = BlockType::Hookable;
                    filled_blocks.push(Position::new(x, y));
                }
            }
        }
    }

    Ok(filled_blocks)
}

#[derive(Debug, Clone, PartialEq)]
enum PlatformPosCandidate {
    /// location is not platform candidate
    None,
    /// location is platform candidate, not used yet in a platform group. stores individual empty_height.
    Candidate(usize),
    /// location is platform candidate and already used for platform group. stores minumum
    /// empty_height of group.
    Grouped(usize),
}

#[derive(Debug)]
pub struct FloorPosition {
    pub pos: Position,
    pub empty_height: usize,
    pub freeze_height: usize,
}

#[derive(Debug, Clone)]
pub struct PlatformCandidate {
    /// 'center' position of platform
    pub pos: Position,

    /// inclusive offset of left platform position
    pub offset_left: usize,

    /// inclusive offset of right platform position
    pub offset_right: usize,

    /// reserved height above platform
    pub reserved_height: usize,

    /// flood fill distance in the map
    pub flood_fill_dist: usize,
}

impl PlatformCandidate {
    pub fn total_width(&self) -> usize {
        self.offset_left + self.offset_right + 1
    }

    /// Re-centers the platform around its middle point
    /// When total width is odd, biases right position, so higher offset_left
    pub fn re_center(&mut self) {
        let total_width = self.total_width();
        let center = self.pos.x - self.offset_left + (total_width / 2);
        self.offset_left = total_width / 2;
        self.offset_right = total_width - self.offset_left - 1;
        self.pos.x = center;
    }

    /// Shrinks the platform to specified width
    /// Prefers reducing left, to counterbalance the bias in re_center
    pub fn shrink(&mut self, shrink_to: usize) {
        let total_width = self.total_width();
        if total_width <= shrink_to {
            return;
        }

        let shrink_by = total_width - shrink_to;
        let shrink_left = (shrink_by + 1) / 2; // ceiling division
        let shrink_right = shrink_by / 2; // floor division

        self.offset_left = self.offset_left.saturating_sub(shrink_left);
        self.offset_right = self.offset_right.saturating_sub(shrink_right);
    }
}

pub fn find_floor_positions(
    map: &Map,
    gen_config: &GenerationConfig,
) -> Result<Vec<FloorPosition>, &'static str> {
    let mut floor_pos: Vec<FloorPosition> = Vec::new();
    for x in 0..map.width {
        for y in 1..map.height {
            if map.grid[[x, y]] != BlockType::Hookable {
                continue; // current block must be hookable
            }

            if map.grid[[x, y - 1]] != BlockType::Freeze {
                continue; // block above must be freeze
            }

            // shift upwards to find first non freeze block
            let base_pos = Position::new(x, y);
            if let Some(non_freeze_pos) = map.shift_pos_until(
                &base_pos,
                ShiftDirection::Up,
                |b| !b.is_freeze(),
                Some(gen_config.plat_max_freeze + 1),
            ) {
                if map.grid[non_freeze_pos.as_index()] != BlockType::Empty {
                    continue; // SKIP: above N freeze blocks there must be an empty block
                }

                let freeze_height = base_pos.y - (non_freeze_pos.y + 1);
                if freeze_height > gen_config.plat_max_freeze {
                    continue;
                }

                // we scan 10 more blocks than required, as reserving more emtpy space later,
                // should result in nicer platforms especially for larger ones
                let empty_scan_height = gen_config.plat_height + 10;
                let empty_height = match map.shift_pos_until(
                    &non_freeze_pos,
                    ShiftDirection::Up,
                    |b| !b.is_empty(),
                    Some(empty_scan_height),
                ) {
                    // found some non-empty block, measure height
                    Some(first_non_empty_pos) => non_freeze_pos.y - first_non_empty_pos.y,
                    // never reached non-empty,
                    None => empty_scan_height, // so we just fall back to maximum scan height
                };

                if empty_height + freeze_height < gen_config.plat_height {
                    continue;
                }

                floor_pos.push(FloorPosition {
                    pos: base_pos,
                    empty_height,
                    freeze_height,
                });
            }
        }
    }
    return Ok(floor_pos);
}

pub fn generate_platform_candidates(
    map: &Map,
    floor_pos: &[FloorPosition],
    flood_fill: &Array2<Option<usize>>,
    gen_config: &GenerationConfig,
    debug_layers: &mut Option<DebugLayers>,
) -> Result<Vec<PlatformCandidate>, &'static str> {
    // fill candidates
    let mut candidates = Array2::from_elem((map.width, map.height), PlatformPosCandidate::None);
    for floor in floor_pos.iter() {
        for freeze_offset in 0..floor.freeze_height {
            candidates[[floor.pos.x, floor.pos.y - freeze_offset]] =
                PlatformPosCandidate::Candidate(
                    floor.empty_height + floor.freeze_height - freeze_offset,
                );
        }
    }

    let mut platforms = Vec::new();

    // group candidates based on floor positions. So floor positions can be grouped with
    // freeze_offset candidates, but a group must start at a floor position!
    for floor in floor_pos.iter() {
        let mut min_empty_height: usize;

        // check current position, skip if already grouped
        if let PlatformPosCandidate::Candidate(height) = candidates[[floor.pos.x, floor.pos.y]] {
            min_empty_height = height;
        } else {
            continue;
        }

        // group to the left
        let mut offset_left = 1;
        while let PlatformPosCandidate::Candidate(height) =
            candidates[[floor.pos.x - offset_left, floor.pos.y]]
        {
            if height < gen_config.plat_height {
                break;
            }
            min_empty_height = min_empty_height.min(height);
            offset_left += 1;
        }
        offset_left -= 1;

        // group to the right
        let mut offset_right = 1;
        while let PlatformPosCandidate::Candidate(empty_height) =
            candidates[[floor.pos.x + offset_right, floor.pos.y]]
        {
            if empty_height < gen_config.plat_height {
                break;
            }
            min_empty_height = min_empty_height.min(empty_height);
            offset_right += 1;
        }
        offset_right -= 1;

        // group all candidates
        let mut view = safe_slice_mut(
            &mut candidates,
            &floor.pos.shifted_by(-(offset_left as i32), 0)?,
            &floor.pos.shifted_by(offset_right as i32, 0)?,
            map,
        )?;
        view.fill(PlatformPosCandidate::Grouped(min_empty_height));

        // skip if platform too narrow
        if offset_left + offset_right + 1 < gen_config.plat_min_width {
            continue;
        }

        // derive platform
        if let Some(flood_fill_dist) =
            flood_fill[[floor.pos.x, floor.pos.y - (gen_config.plat_max_freeze + 1)]]
        {
            let mut platform_cand = PlatformCandidate {
                pos: floor.pos.clone(), // TODO: remove clone
                offset_left,
                offset_right,
                reserved_height: min_empty_height,
                flood_fill_dist,
            };
            platform_cand.re_center();
            platform_cand.shrink(gen_config.plat_max_width);
            platforms.push(platform_cand);
        }
    }

    if let Some(debug_layers) = debug_layers {
        debug_layers.float_layers.get_mut("plat_cand").unwrap().grid =
            candidates.mapv(|v| match v {
                PlatformPosCandidate::Candidate(empty_height) => Some(empty_height as f32),
                _ => None,
            });

        debug_layers
            .float_layers
            .get_mut("plat_group")
            .unwrap()
            .grid = candidates.mapv(|v| match v {
            PlatformPosCandidate::Grouped(empty_height) => Some(empty_height as f32),
            _ => None,
        });
    }

    return Ok(platforms);
}

/// Greedily selects platforms that dont violate the minimum gap constraint.
/// Algorithm considers `platforms` in the order as provided.
/// Defaults to only floodfill distance, but `use_euclidean` can be enabled,
/// to add a double constraint that also adds a euclidean based gap check.
pub fn greedy_select_platforms(
    platforms: &[PlatformCandidate],
    min_gap: usize,
    use_euclidean: bool,
) -> Result<Vec<PlatformCandidate>, &'static str> {
    let platforms_count = platforms.len();
    let mut platform_blocked = vec![false; platforms_count];
    let mut selected_platforms: Vec<PlatformCandidate> = Vec::new();

    for idx in 0..platforms.len() {
        if platform_blocked[idx] {
            continue;
        }

        // not blocked so we place platform
        let plat = &platforms[idx];

        // but block all near platforms
        for idx_other in idx..platforms_count {
            if platform_blocked[idx_other] {
                continue; // skip if already blocked
            }

            let plat_other = &platforms[idx_other];

            let ff_gap = plat.flood_fill_dist.abs_diff(plat_other.flood_fill_dist);
            if ff_gap < min_gap {
                if use_euclidean {
                    let euclidean_dist = plat.pos.distance(&plat_other.pos);
                    if euclidean_dist > min_gap as f32 {
                        continue; // euclidean constraint is not violated, so we DONT BLOCK
                    }
                }
                platform_blocked[idx_other] = true;
            }
        }

        selected_platforms.push(plat.clone());
    }

    Ok(selected_platforms)
}

/// DP solver to minimize total sum of deviations from platform gap to target gap
pub fn select_platforms_dp(
    mut plats: Vec<PlatformCandidate>,
    keep: usize,
    target_gap: usize,
    ff_map_length: usize,
) -> Result<(Vec<usize>, f32), &'static str> {
    if plats.is_empty() || keep == 0 || keep > plats.len() {
        return Err("no feasible selection");
    }

    plats.sort_unstable_by(|a, b| a.flood_fill_dist.cmp(&b.flood_fill_dist));
    let n = plats.len();
    let layers = keep;

    // dp[layer][idx] -> (best_cost, predecessor)
    let mut dp = vec![vec![(f32::INFINITY, None); n]; layers];

    // layer 0: only one platform
    for idx in 0..n {
        let start_gap = plats[idx].flood_fill_dist;
        dp[0][idx].0 = (start_gap as f32 - target_gap as f32).abs();
    }

    // fill further layers
    for chosen in 1..layers {
        for cur in chosen..n {
            for prev in (chosen - 1)..cur {
                let gap = plats[cur].flood_fill_dist - plats[prev].flood_fill_dist;
                let cost = dp[chosen - 1][prev].0 + (gap as f32 - target_gap as f32).abs();
                if cost < dp[chosen][cur].0 {
                    dp[chosen][cur] = (cost, Some(prev));
                }
            }
        }
    }

    // pick best tail: add end-gap penalty
    let (best_tail_idx, best_total) = dp[layers - 1]
        .iter()
        .enumerate()
        .map(|(idx, &(cost, _))| {
            // There can be platforms with ff_dist higher than the goal, so we saturate to zero.
            // TODO: this means that such a platform adds zero cost -> problem?
            let end_gap = ff_map_length.saturating_sub(plats[idx].flood_fill_dist);
            (idx, cost + (end_gap as f32 - target_gap as f32).abs())
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .ok_or("no feasible selection")?;

    // back-track indices
    let mut indices = Vec::with_capacity(layers);
    let mut idx = best_tail_idx;
    for layer in (0..layers).rev() {
        indices.push(idx);
        if let Some(prev) = dp[layer][idx].1 {
            idx = prev;
        }
    }
    indices.reverse();

    Ok((indices, best_total))
}

/// Pick selection of platforms whose worst-gap deviation from target_gap is minimal.
pub fn select_best_platform_config(
    mut plats: Vec<PlatformCandidate>,
    target_gap: usize,
    ff_map_length: usize,
) -> Result<Vec<PlatformCandidate>, &'static str> {
    if plats.is_empty() {
        return Err("no candidates");
    }

    plats.sort_unstable_by(|a, b| a.flood_fill_dist.cmp(&b.flood_fill_dist));
    let n = plats.len();
    let mut best_indices = Vec::<usize>::new();
    let mut best_max_dev: i32 = i32::MAX;
    let mut best_sum_cost = f32::INFINITY;

    // TODO: in practise the score/worst dev is following the trend of a hyperble. So a
    // possible optimization would be to start with an approximation, and search minimum
    // from there, as it'd skip many of the runs
    for k in 1..=n {
        let (idxs, sum_cost) = select_platforms_dp(plats.clone(), k, target_gap, ff_map_length)?;

        // compute worst gap deviation
        let mut worst_dev: i32 = 0;
        let mut prev_pos = 0;
        for &idx in &idxs {
            let gap = plats[idx].flood_fill_dist.saturating_sub(prev_pos);
            worst_dev = worst_dev.max((gap as i32 - target_gap as i32).abs());
            prev_pos = plats[idx].flood_fill_dist;
        }
        let end_gap = ff_map_length.saturating_sub(prev_pos);
        worst_dev = worst_dev.max((end_gap as i32 - target_gap as i32).abs());
        // debug!(
        //     "k={:<2}  worst_dev={:<4}  sum_cost={}",
        //     k, worst_dev, sum_cost,
        // );

        // keep best according to worst deviation from target gap
        // Tie-breaker: Fall back to the total summed gap cost
        if worst_dev < best_max_dev || (worst_dev == best_max_dev && sum_cost < best_sum_cost) {
            best_indices = idxs;
            best_max_dev = worst_dev;
            best_sum_cost = sum_cost;
            // debug!(
            //     "   ↳ new best (k={}, worst_dev={}, sum_cost={})",
            //     k, worst_dev, sum_cost
            // );
        }
    }

    // info!(
    //     "Chosen k={}, worst_dev={}, total_cost={}",
    //     best_indices.len(),
    //     best_max_dev,
    //     best_sum_cost
    // );

    Ok(best_indices.into_iter().map(|i| plats[i].clone()).collect())
}

pub fn generate_platforms(
    map: &mut Map,
    gen_config: &GenerationConfig,
    flood_fill: &Array2<Option<usize>>,
    ff_map_length: usize,
    debug_layers: &mut Option<DebugLayers>,
) -> Result<Vec<FloorPosition>, &'static str> {
    // find potential floor positions
    let floor_pos = find_floor_positions(map, gen_config)?;

    // generate all valid platform candidates and order by their size
    let mut all_platforms =
        generate_platform_candidates(map, &floor_pos, flood_fill, gen_config, debug_layers)?;
    all_platforms.sort_unstable_by(|a, b| {
        (a.offset_left + a.offset_right)
            .cmp(&(b.offset_left + b.offset_right))
            .reverse()
    });

    // first greedy filter: prioritize large platforms, use only fraction of the target gap
    let selected_platforms =
        greedy_select_platforms(&all_platforms, gen_config.plat_target_distance / 5, false)?;

    // now find optimal configuration of platforms
    let mut final_platforms = select_best_platform_config(
        selected_platforms,
        gen_config.plat_target_distance,
        ff_map_length,
    )?;

    // generate final selection of platforms
    for plat in final_platforms.iter() {
        set_platform(map, plat)?;
    }

    // check that no platform gap is too large
    // TODO: this doesnt yet consider multi-path maps
    final_platforms.sort_unstable_by(|a, b| a.flood_fill_dist.cmp(&b.flood_fill_dist));
    let ff_gaps: Vec<usize> = final_platforms
        .windows(2)
        .map(|a| a[1].flood_fill_dist - a[0].flood_fill_dist)
        .collect();
    // TODO: introduce these as a parameter?
    // dbg!(&ff_gaps);
    let max_valid_gap = (gen_config.plat_target_distance as f32 * 1.50) as usize;
    let min_valid_gap = (gen_config.plat_target_distance as f32 / 2.00) as usize;
    let max_gap = *ff_gaps.iter().max().unwrap();
    let min_gap = *ff_gaps.iter().min().unwrap();
    if max_gap > max_valid_gap {
        // dbg!(min_valid_gap, max_valid_gap, min_gap, max_gap);
        return Err("maximum plat gap constrain not fulfilled");
    } else if min_gap < min_valid_gap {
        // dbg!(min_valid_gap, max_valid_gap, min_gap, max_gap);
        return Err("minimum plat gap constrain not fulfilled");
    }

    Ok(floor_pos)
}

pub fn set_platform(map: &mut Map, plat: &PlatformCandidate) -> Result<(), &'static str> {
    let left = plat.pos.x - plat.offset_left;
    let right = plat.pos.x + plat.offset_right;
    let top = plat.pos.y - plat.reserved_height;

    let top_left = Position::new(left, top);
    let bot_right = Position::new(right, plat.pos.y - 1);

    map.set_area(
        &Position::new(left, plat.pos.y),
        &Position::new(right, plat.pos.y),
        &BlockType::Platform,
        &Overwrite::Force,
    );

    map.set_area(
        &top_left,
        &bot_right,
        &BlockType::EmptyPlatform,
        &Overwrite::Force,
    );

    // fix edge bugs
    // due to improved platform detection we dont need this anymore, but maybe i'll decide to
    // add a feature that expands a platform, in that case i'd need this again.
    // fix_local_edge_bugs(map, &top_left, &bot_right);

    // check "soft blocked" parts
    let part_offset = 2;

    // consider left and right
    for &dir in &[-1, 1] {
        let entry_x = if dir == 1 { right + 1 } else { left - 1 };
        let part_entry = Position::new(entry_x, plat.pos.y - 1);
        let part_exit = part_entry.shifted_by(part_offset * dir, 0)?;

        // ensure correct order for position access
        let (left, right) = if dir == 1 {
            (part_entry, part_exit)
        } else {
            (part_exit, part_entry)
        };

        // check if part is empty = "playable"
        if map.check_area_all(&left, &right, &BlockType::Empty)? {
            let above_left = left.shifted_by(0, -1)?;
            let above_right = right.shifted_by(0, -1)?;

            // check if playable path is a one tiler
            if map.check_area_exists(&above_left, &above_right, &BlockType::Freeze)? {
                // if yes, remove one block above
                map.set_area(
                    &above_left,
                    &above_right,
                    &BlockType::Empty,
                    &Overwrite::ReplaceNonSolid,
                );

                // and fix potential resulting edge bugs
                fix_local_edge_bugs(map, &above_left, &above_right);
            }
        }
    }

    Ok(())
}
