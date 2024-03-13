use crate::{kernel::ValidKernelTable, Kernel, KernelType, Map, Position, Random};

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
}

pub struct GenerationConfig {
    pub max_kernel_size: usize,
    pub inner_rad_mut_prob: f32,
    pub outer_rad_mut_prob: f32,
    pub inner_size_mut_prob: f32,
    pub outer_size_mut_prob: f32,
}

impl GenerationConfig {
    pub fn new(max_kernel_size: usize, mut_prob: f32) -> GenerationConfig {
        GenerationConfig {
            max_kernel_size,
            inner_rad_mut_prob: mut_prob,
            outer_rad_mut_prob: mut_prob,
            inner_size_mut_prob: mut_prob,
            outer_size_mut_prob: mut_prob,
        }
    }
}

impl CuteWalker {
    pub fn new(
        initial_pos: Position,
        waypoints: Vec<Position>,
        inner_kernel: Kernel,
        outer_kernel: Kernel,
    ) -> CuteWalker {
        CuteWalker {
            pos: initial_pos,
            steps: 0,
            inner_kernel,
            outer_kernel,
            goal: Some(waypoints.first().unwrap().clone()),
            goal_index: 0,
            waypoints,
        }
    }

    pub fn is_goal_reached(&self) -> Option<bool> {
        self.goal.as_ref().map(|goal| self.pos.eq(goal))
    }

    pub fn next_waypoint(&mut self) -> Result<(), ()> {
        if let Some(next_goal) = self.waypoints.get(self.goal_index + 1) {
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

    pub fn mutate_kernel(
        &mut self,
        config: &GenerationConfig,
        rnd: &mut Random,
        kernel_table: &ValidKernelTable,
    ) {
        // mutate both kernels
        if rnd.with_probability(config.inner_size_mut_prob) {
            self.inner_kernel.size = rnd.random_size(config.max_kernel_size);
        }
        if rnd.with_probability(config.outer_size_mut_prob) {
            self.outer_kernel.size = rnd.random_size(config.max_kernel_size);
        }

        // enforce valid configuration
        if self.outer_kernel.size < self.inner_kernel.size + 2 {
            self.outer_kernel.size = self.inner_kernel.size + 2;
        }

        // very ugly - enforce maximum radius
        self.inner_kernel = Kernel::new(
            self.inner_kernel.size,
            *kernel_table
                .valid_radii_per_size
                .get(&self.inner_kernel.size)
                .unwrap()
                .last()
                .unwrap(),
        );
        self.outer_kernel = Kernel::new(
            self.outer_kernel.size,
            *kernel_table
                .valid_radii_per_size
                .get(&self.outer_kernel.size)
                .unwrap()
                .last()
                .unwrap(),
        );
    }
}
