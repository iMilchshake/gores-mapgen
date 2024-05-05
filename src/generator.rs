use std::f32::consts::SQRT_2;

use std::collections::BTreeMap;

use crate::{
    config::GenerationConfig,
    debug::DebugLayer,
    kernel::Kernel,
    map::{BlockType, Map},
    position::{Position, ShiftDirection},
    random::{Random, Seed},
    walker::CuteWalker,
};

use dt::dt_bool;
use macroquad::color::colors;
use ndarray::{s, Array, Array2, IndexLonger, Ix2};

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,
    pub rnd: Random,
    pub debug_layers: BTreeMap<&'static str, DebugLayer>,
}

impl Generator {
    /// derive a initial generator state based on a GenerationConfig
    pub fn new(config: &GenerationConfig, seed: Seed) -> Generator {
        let spawn = Position::new(50, 250);
        let map = Map::new(300, 300, BlockType::Hookable, spawn.clone());
        let init_inner_kernel = Kernel::new(5, 0.0);
        let init_outer_kernel = Kernel::new(7, 0.0);
        let walker = CuteWalker::new(spawn, init_inner_kernel, init_outer_kernel, config);
        let rnd = Random::new(seed, config);

        let debug_layers = BTreeMap::from([
            ("edge_bugs", DebugLayer::new(true, colors::RED, &map)),
            ("corners", DebugLayer::new(true, colors::BLUE, &map)),
        ]);

        Generator {
            walker,
            map,
            rnd,
            debug_layers,
        }
    }

    pub fn step(&mut self, config: &GenerationConfig) -> Result<(), &'static str> {
        // check if walker has reached goal position
        if self.walker.is_goal_reached(&config.waypoint_reached_dist) == Some(true) {
            self.walker.next_waypoint();
        }

        if !self.walker.finished {
            config.validate()?;

            // randomly mutate kernel
            self.walker.mutate_kernel(config, &mut self.rnd);

            // perform one step
            self.walker
                .probabilistic_step(&mut self.map, config, &mut self.rnd)?;

            // handle platforms
            self.walker.check_platform(
                &mut self.map,
                config.platform_distance_bounds.0,
                config.platform_distance_bounds.1,
            )?;
        }

        Ok(())
    }

    /// Post processing step to fix all existing edge-bugs, as certain inner/outer kernel
    /// configurations do not ensure a min. 1-block freeze padding consistently.
    fn fix_edge_bugs(&mut self) -> Result<Array2<bool>, &'static str> {
        let mut edge_bug = Array2::from_elem((self.map.width, self.map.height), false);
        let width = self.map.width;
        let height = self.map.height;

        for x in 0..width {
            for y in 0..height {
                let value = &self.map.grid[[x, y]];
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
                                let neighbor_value = &self.map.grid[[neighbor_x, neighbor_y]];
                                if *neighbor_value == BlockType::Hookable {
                                    edge_bug[[x, y]] = true;
                                    // break;
                                    // TODO: this should be easy to optimize
                                }
                            }
                        }
                    }

                    if edge_bug[[x, y]] {
                        self.map.grid[[x, y]] = BlockType::Freeze;
                    }
                }
            }
        }

        Ok(edge_bug)
    }

    /// Using a distance transform this function will fill up all empty blocks that are too far
    /// from the next solid/non-empty block
    pub fn fill_area(&mut self, max_distance: &f32) -> Array2<f32> {
        let grid = self.map.grid.map(|val| *val != BlockType::Empty);

        // euclidean distance transform
        let distance = dt_bool::<f32>(&grid.into_dyn())
            .into_dimensionality::<Ix2>()
            .unwrap();

        self.map
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

    pub fn find_corners(&self) -> Result<Array2<bool>, &'static str> {
        let mut corners_grid = Array2::from_elem((self.map.width, self.map.height), false);

        // stores a list of corner candidates and the respective direction to the wall
        let mut corner_candidates: Vec<(Position, ShiftDirection)> = Vec::new();

        let width = self.map.width;
        let height = self.map.height;

        let window_size = 2; // 2 -> 5x5 windows

        for window_x in window_size..(width - window_size) {
            for window_y in window_size..(height - window_size) {
                let window = &self.map.grid.slice(s![
                    window_x - window_size..=window_x + window_size,
                    window_y - window_size..=window_y + window_size
                ]);

                if window[[2, 2]] != BlockType::Empty {
                    continue;
                }

                if [
                    &window[[2, 3]],
                    &window[[3, 0]],
                    &window[[3, 1]],
                    &window[[3, 2]],
                    &window[[3, 3]],
                ]
                .iter()
                .all(|&val| *val == BlockType::Freeze)
                {
                    corners_grid[[window_x, window_y]] = true;
                    corner_candidates
                        .push((Position::new(window_x, window_y), ShiftDirection::Left));
                    continue;
                }

                if [
                    &window[[2, 3]],
                    &window[[1, 0]],
                    &window[[1, 1]],
                    &window[[1, 2]],
                    &window[[1, 3]],
                ]
                .iter()
                .all(|&val| *val == BlockType::Freeze)
                {
                    corners_grid[[window_x, window_y]] = true;
                    continue;
                }
            }
        }

        Ok(corners_grid)
    }

    pub fn post_processing(&mut self, config: &GenerationConfig) {
        let edge_bugs = self.fix_edge_bugs().expect("fix edge bugs failed");
        self.map
            .generate_room(&self.map.spawn.clone(), 4, 3, Some(&BlockType::Start))
            .expect("start room generation failed");
        self.map
            .generate_room(&self.walker.pos.clone(), 4, 3, Some(&BlockType::Finish))
            .expect("start finish room generation");

        self.fill_area(&config.max_distance);

        let corners = self.find_corners().expect("corners failed");

        let corner_count = corners.iter().filter(|&&val| val).count();
        dbg!(corner_count);

        // set debug layers
        self.debug_layers.get_mut("edge_bugs").unwrap().grid = edge_bugs;
        self.debug_layers.get_mut("corners").unwrap().grid = corners;
    }

    /// Generates an entire map with a single function call. This function is used by the CLI.
    /// It is important to keep this function up to date with the editor generation, so that
    /// fixed seed map generations result in the same map.
    pub fn generate_map(
        max_steps: usize,
        seed: &Seed,
        config: &GenerationConfig,
    ) -> Result<Map, &'static str> {
        let mut gen = Generator::new(config, seed.clone());

        for _ in 0..max_steps {
            if gen.walker.finished {
                break;
            }
            gen.step(config)?;
        }

        gen.post_processing(config);

        Ok(gen.map)
    }
}
