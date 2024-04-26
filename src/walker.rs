use crate::{
    config::GenerationConfig,
    kernel::Kernel,
    map::{BlockType, KernelType, Map},
    position::{Position, ShiftDirection},
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

    pub steps_since_platform: usize,

    pub last_direction: Option<ShiftDirection>,
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
            steps_since_platform: 0,
            last_direction: None,
        }
    }

    pub fn is_goal_reached(&self, waypoint_reached_dist: &usize) -> Option<bool> {
        self.goal
            .as_ref()
            .map(|goal| goal.distance_squared(&self.pos) < *waypoint_reached_dist)
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

    /// will try to place a platform at the walkers position.
    /// If force is true it will enforce a platform.
    pub fn check_platform(
        &mut self,
        map: &mut Map,
        min_distance: usize,
        max_distance: usize,
    ) -> Result<(), &'static str> {
        self.steps_since_platform += 1;

        // Case 1: min distance is not reached -> skip
        if self.steps_since_platform < min_distance {
            return Ok(());
        }

        let walker_pos = self.pos.clone();

        // Case 2: max distance has been exceeded -> force platform using a room
        if self.steps_since_platform > max_distance {
            // TODO: for now this is hardcoded so that platform is shifted down by 7 blocks.
            map.generate_room(&walker_pos.shifted_by(0, 6)?, 5, 3, None)?;
            self.steps_since_platform = 0;
            return Ok(());
        }

        // Case 3: min distance has been exceeded -> Try to place platform, but only if possible
        let area_empty = map.check_area_all(
            &walker_pos.shifted_by(-3, -3)?,
            &walker_pos.shifted_by(3, 2)?,
            &BlockType::Empty,
        )?;
        if area_empty {
            map.set_area(
                &walker_pos.shifted_by(-1, 0)?,
                &walker_pos.shifted_by(1, 0)?,
                &BlockType::Platform,
                true,
            );
            self.steps_since_platform = 0;
        }

        Ok(())
    }

    pub fn probabilistic_step(
        &mut self,
        map: &mut Map,
        config: &GenerationConfig,
        rnd: &mut Random,
    ) -> Result<(), &'static str> {
        if self.finished {
            return Err("Walker is finished");
        }

        let goal = self.goal.as_ref().ok_or("Error: Goal is None")?;
        let shifts = self.pos.get_rated_shifts(goal, map);
        let mut sampled_shift = &rnd.sample_move(&shifts);

        // with a certain probabiliy re-use last direction instead
        if rnd.with_probability(config.momentum_prob) && self.last_direction.is_some() {
            sampled_shift = self.last_direction.as_ref().unwrap();
        }

        // apply that shift
        self.pos.shift_in_direction(sampled_shift, map)?;
        self.steps += 1;
        self.last_direction = Some(sampled_shift.clone());

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
            inner_circ = *rnd.pick_element(&[0.0, 0.1, 0.2, 0.6, 0.8]); // TODO: also, this is
                                                                        // terrible
            modified = true;
        } else {
            rnd.skip();
        }

        if rnd.with_probability(config.outer_rad_mut_prob) {
            outer_circ = *rnd.pick_element(&[0.0, 0.1, 0.2, 0.6, 0.8]);
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
