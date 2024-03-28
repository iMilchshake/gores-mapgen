use crate::{
    editor::GenerationConfig,
    kernel::Kernel,
    map::{BlockType, Map},
    position::Position,
    random::Random,
    walker::CuteWalker,
};

use ndarray::Array2;

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,
    pub rnd: Random,
}

impl Generator {
    /// derive a initial generator state based on a GenerationConfig
    pub fn new(config: &GenerationConfig, seed: u64) -> Generator {
        let spawn = Position::new(50, 50);
        let map = Map::new(900, 900, BlockType::Hookable, spawn.clone());
        let init_inner_kernel = Kernel::new(config.max_inner_size, 0.0);
        let init_outer_kernel = Kernel::new(config.max_outer_size, 0.1);
        let walker = CuteWalker::new(spawn, init_inner_kernel, init_outer_kernel, config);
        let rnd = Random::new(seed, config.step_weights.clone());

        Generator { walker, map, rnd }
    }

    pub fn step(&mut self, config: &GenerationConfig) -> Result<(), &'static str> {
        // check if walker has reached goal position
        if self.walker.is_goal_reached() == Some(true) {
            self.walker.next_waypoint();
        }

        // randomly mutate kernel
        self.walker.mutate_kernel(config, &mut self.rnd);

        // perform one step
        self.walker
            .probabilistic_step(&mut self.map, &mut self.rnd)?;

        Ok(())
    }

    /// Post processing step to fix all existing edge-bugs, as certain inner/outer kernel
    /// configurations do not ensure a min. 1-block freeze padding consistently.
    pub fn fix_edge_bugs(&mut self) -> Array2<bool> {
        let mut edge_bug = Array2::from_elem((self.map.width, self.map.height), false);

        for ((x, y), value) in self.map.grid.indexed_iter() {
            if *value == BlockType::Empty {
                for dx in 0..=2 {
                    for dy in 0..=2 {
                        if dx == 1 || dy == 1 {
                            continue;
                        }

                        let neighbor_x = x + dx - 1;
                        let neighbor_y = y + dy - 1;
                        if let Some(neighbor_value) = self.map.grid.get((neighbor_x, neighbor_y)) {
                            if *neighbor_value == BlockType::Hookable {
                                edge_bug[[x, y]] = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        edge_bug
    }
}
