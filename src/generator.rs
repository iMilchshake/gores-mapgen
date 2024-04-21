use crate::{
    config::GenerationConfig,
    kernel::Kernel,
    map::{BlockType, Map},
    position::Position,
    random::Random,
    walker::CuteWalker,
};

use dt::{dt, dt_bool, dt_int};
use ndarray::{arr2, s, Array, Array2, Ix2, IxDyn};

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,
    pub rnd: Random,
}

impl Generator {
    /// derive a initial generator state based on a GenerationConfig
    pub fn new(config: &GenerationConfig, seed: u64) -> Generator {
        let spawn = Position::new(50, 250);
        let map = Map::new(300, 300, BlockType::Hookable, spawn.clone());
        let init_inner_kernel = Kernel::new(config.inner_size_bounds.1, 0.0);
        let init_outer_kernel = Kernel::new(config.outer_size_bounds.1, 0.1);
        let walker = CuteWalker::new(spawn, init_inner_kernel, init_outer_kernel, config);
        let rnd = Random::new(seed, config.step_weights.clone());

        Generator { walker, map, rnd }
    }

    pub fn step(&mut self, config: &GenerationConfig) -> Result<(), &'static str> {
        // check if walker has reached goal position
        if self.walker.is_goal_reached() == Some(true) {
            self.walker.next_waypoint();
        }

        if !self.walker.finished {
            // randomly mutate kernel
            self.walker.mutate_kernel(&config, &mut self.rnd);

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

    pub fn fill_area(&mut self, max_distance: &f32) -> Array2<f32> {
        let grid = self.map.grid.map(|val| *val != BlockType::Empty);

        let distance = dt_bool::<f32>(&grid.into_dyn())
            .into_dimensionality::<Ix2>()
            .unwrap();

        // let max = distance.fold(0.0, |v1, v2| f32::max(v1, *v2));

        self.map
            .grid
            .zip_mut_with(&distance, |block_type, distance| {
                if *block_type == BlockType::Empty && *distance > *max_distance {
                    *block_type = BlockType::Freeze;
                }
            });

        distance
    }

    pub fn post_processing(&mut self, config: &GenerationConfig) {
        self.fix_edge_bugs().expect("fix edge bugs failed");
        self.map
            .generate_room(&self.map.spawn.clone(), 4, Some(&BlockType::Start))
            .expect("start room generation failed");
        self.map
            .generate_room(&self.walker.pos.clone(), 4, Some(&BlockType::Finish))
            .expect("start finish room generation");

        self.fill_area(&config.max_distance);
    }

    /// Generates an entire map with a single function call. This function is used by the CLI.
    /// It is important to keep this function up to date with the editor generation, so that
    /// fixed seed map generations result in the same map.
    pub fn generate_map(
        max_steps: usize,
        seed: u64,
        config: &GenerationConfig,
    ) -> Result<Map, &'static str> {
        let mut gen = Generator::new(&config, seed);

        for _ in 0..max_steps {
            if gen.walker.finished {
                break;
            }
            gen.step(&config)?;
        }

        gen.post_processing(config);

        Ok(gen.map)
    }
}
