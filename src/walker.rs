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
    pub max_inner_size: usize,
    pub max_outer_size: usize,
    pub inner_rad_mut_prob: f32,
    pub inner_size_mut_prob: f32,
}

impl GenerationConfig {
    pub fn new(
        max_inner_size: usize,
        max_outer_size: usize,
        inner_rad_mut_prob: f32,
        inner_size_mut_prob: f32,
    ) -> GenerationConfig {
        assert!(
            max_outer_size - 2 >= max_inner_size,
            "max_outer_size needs to be +2 of max_inner_size"
        );
        GenerationConfig {
            max_inner_size,
            max_outer_size,
            inner_rad_mut_prob,
            inner_size_mut_prob,
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
        let mut inner_size = self.inner_kernel.size;
        let mut inner_radius = self.inner_kernel.radius;
        let mut modified = false;

        // mutate inner kernel
        if rnd.with_probability(config.inner_size_mut_prob) {
            inner_size = rnd.random_kernel_size(config.max_inner_size);
            modified = true;
        }
        if rnd.with_probability(config.inner_rad_mut_prob) {
            inner_radius = rnd.pick_element(&kernel_table.get_valid_radii(&inner_size));
            modified = true;
        }

        if modified {
            self.inner_kernel = Kernel::new(inner_size, inner_radius);
            self.outer_kernel = kernel_table.get_min_valid_outer_kernel(&self.inner_kernel);
        }
    }
}
