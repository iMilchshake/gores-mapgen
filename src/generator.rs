use crate::{
    editor::GenerationConfig,
    kernel::Kernel,
    map::{BlockType, Map},
    position::Position,
    random::Random,
    walker::CuteWalker,
};

use ndarray::{s, Array2};

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,
    pub rnd: Random,
}

impl Generator {
    /// derive a initial generator state based on a GenerationConfig
    pub fn new(config: &GenerationConfig, seed: u64) -> Generator {
        let spawn = Position::new(50, 50);
        let map = Map::new(300, 300, BlockType::Hookable, spawn.clone());
        let init_inner_kernel = Kernel::new(config.inner_size.1, 0.0);
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

                            let neighbor_x = x + dx - 1; // TODO: deal with overflow?
                            let neighbor_y = y + dy - 1;
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

        edge_bug
    }

    pub fn generate_room(&mut self, pos: &Position, margin: usize) {
        let start_x = pos.x.saturating_sub(margin);
        let start_y = pos.y.saturating_sub(margin);
        let end_x = (pos.x + margin + 1).min(self.map.width);
        let end_y = (pos.y + margin + 1).min(self.map.height);

        let valid = start_x < end_x && start_y < end_y;

        if valid {
            let mut view = self.map.grid.slice_mut(s![start_x..end_x, start_y..end_y]);
            view.map_inplace(|elem| *elem = BlockType::Empty);

            let platform = margin.saturating_sub(1); // also corresponds to a 'margin'

            dbg!(&platform);

            let mut view = self.map.grid.slice_mut(s![
                pos.x - platform..pos.x + platform + 1,
                pos.y + 1..pos.y + 2
            ]);

            dbg!(&view);
            view.map_inplace(|elem| *elem = BlockType::Hookable);
        }
    }
}
