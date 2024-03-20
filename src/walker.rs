use crate::{GenerationConfig, Kernel, KernelType, Map, Position, Random};

// this walker is indeed very cute
#[derive(Debug)]
pub struct CuteWalker {
    pub pos: Position,
    pub steps: usize,
    pub inner_kernel: Kernel,
    pub outer_kernel: Kernel,
    pub goal: Option<Position>,
    pub goal_index: usize,
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
        }
    }

    pub fn is_goal_reached(&self) -> Option<bool> {
        self.goal.as_ref().map(|goal| self.pos.eq(goal))
    }

    pub fn next_waypoint(&mut self, config: &GenerationConfig) -> Result<(), ()> {
        if let Some(next_goal) = config.waypoints.get(self.goal_index + 1) {
            self.goal_index += 1;
            self.goal = Some(next_goal.clone());
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn greedy_step(&mut self, map: &mut Map) -> Result<(), &'static str> {
        let goal = self.goal.as_ref().ok_or("Error: Goal is None")?;
        let greedy_shift = self.pos.get_greedy_shift(goal);

        // apply that shift
        self.pos.shift(greedy_shift, map)?;
        self.steps += 1;

        // remove blocks using a kernel at current position
        map.update(self, KernelType::Inner)?;

        Ok(())
    }

    pub fn probabilistic_step(
        &mut self,
        map: &mut Map,
        rnd: &mut Random,
    ) -> Result<(), &'static str> {
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
        let mut inner_circularity = self.inner_kernel.circularity;
        let mut modified = false;

        // mutate inner kernel
        if rnd.with_probability(config.inner_size_mut_prob) {
            inner_size = rnd.random_kernel_size(config.max_inner_size);
            modified = true;
        }
        if rnd.with_probability(config.inner_rad_mut_prob) {
            inner_circularity = *rnd.pick_element(&vec![0.0, 0.1, 0.2, 0.6, 0.8]);
            modified = true;
        }

        if inner_size <= 2 {
            inner_circularity = 0.0;
        }

        if modified {
            self.inner_kernel = Kernel::new(inner_size, inner_circularity);
            self.outer_kernel = Kernel::new(inner_size + 2, inner_circularity)
        }
    }
}
