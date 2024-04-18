use crate::{
    config::GenerationConfig,
    kernel::Kernel,
    map::{KernelType, Map},
    position::Position,
    random::Random,
};

// this walker is indeed very cute
#[derive(Debug)]
pub struct CuteWalker {
    pub pos: Position,
    pub steps: usize,
    pub inner_kernel: Kernel,
    pub outer_kernel: Kernel,
    pub goal: Option<Position>,
    pub goal_index: usize,
    pub waypoints: Vec<Position>,

    /// indicates whether walker has reached the last waypoint
    pub finished: bool,
}

impl CuteWalker {
    pub fn new(
        initial_pos: Position,
        inner_kernel: Kernel,
        outer_kernel: Kernel,
        config: &GenerationConfig,
    ) -> CuteWalker {
        CuteWalker {
            pos: initial_pos,
            steps: 0,
            inner_kernel,
            outer_kernel,
            goal: Some(config.waypoints.first().unwrap().clone()),
            goal_index: 0,
            waypoints: config.waypoints.clone(),
            finished: false,
        }
    }

    pub fn is_goal_reached(&self) -> Option<bool> {
        self.goal.as_ref().map(|goal| self.pos.eq(goal))
    }

    pub fn next_waypoint(&mut self) {
        if let Some(next_goal) = self.waypoints.get(self.goal_index + 1) {
            self.goal_index += 1;
            self.goal = Some(next_goal.clone());
        } else {
            self.finished = true;
            self.goal = None;
        }
    }

    pub fn probabilistic_step(
        &mut self,
        map: &mut Map,
        rnd: &mut Random,
    ) -> Result<(), &'static str> {
        if self.finished {
            return Err("Walker is finished");
        }

        let goal = self.goal.as_ref().ok_or("Error: Goal is None")?;
        let shifts = self.pos.get_rated_shifts(goal, map);
        let sampled_shift = rnd.sample_move(shifts);

        // apply that shift
        self.pos.shift(sampled_shift, map)?;
        self.steps += 1;

        // remove blocks using a kernel at current position
        map.update(self, KernelType::Outer)?;
        map.update(self, KernelType::Inner)?;

        Ok(())
    }

    pub fn cuddle(&self) {
        println!("Cute walker was cuddled!");
    }

    pub fn mutate_kernel(&mut self, config: &GenerationConfig, rnd: &mut Random) {
        let mut inner_size = self.inner_kernel.size;
        let mut inner_circ = self.inner_kernel.circularity;
        let mut outer_size = self.outer_kernel.size;
        let mut outer_circ = self.outer_kernel.circularity;

        let mut modified = false;

        // mutate inner kernel
        if rnd.with_probability(config.inner_size_mut_prob) {
            inner_size =
                rnd.in_range_inclusive(config.inner_size_bounds.0, config.inner_size_bounds.1);
            modified = true;
        } else {
            rnd.skip();
        }

        if rnd.with_probability(config.outer_size_mut_prob) {
            outer_size =
                rnd.in_range_inclusive(config.outer_size_bounds.0, config.outer_size_bounds.1);
            modified = true;
        } else {
            rnd.skip();
        }

        if rnd.with_probability(config.inner_rad_mut_prob) {
            inner_circ = *rnd.pick_element(&vec![0.0, 0.1, 0.2, 0.6, 0.8]); // TODO: also, this is
                                                                            // terrible
            modified = true;
        } else {
            rnd.skip();
        }

        if rnd.with_probability(config.outer_rad_mut_prob) {
            outer_circ = *rnd.pick_element(&vec![0.0, 0.1, 0.2, 0.6, 0.8]);
            modified = true;
        } else {
            rnd.skip();
        }

        // constraint 1: small circles must be fully rect
        if inner_size <= 3 {
            inner_circ = 0.0;
        }
        if outer_size <= 3 {
            outer_circ = 0.0;
        }

        // constraint 2: outer size cannot be smaller than inner
        outer_size = usize::max(outer_size, inner_size);

        // constraint 3: both sizes should be either odd or even
        // TODO: this can (iteratively) lead to outer_size > max_outer_size
        if (outer_size - inner_size) % 2 == 1 {
            outer_size += 1;
        }

        if modified {
            self.inner_kernel = Kernel::new(inner_size, inner_circ);
            self.outer_kernel = Kernel::new(outer_size, outer_circ);
        }
    }
}
