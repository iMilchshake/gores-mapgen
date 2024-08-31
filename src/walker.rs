use std::fmt;

use ndarray::{s, Array2};

use crate::{
    config::GenerationConfig,
    debug::DebugLayers,
    kernel::Kernel,
    map::{BlockType, Map, Overwrite},
    position::{Position, ShiftDirection},
    random::Random,
};

// this walker is indeed very cute
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

    /// keeps track of how many steps ago the last platorm has been placed
    pub steps_since_platform: usize,

    /// keeps track of the last shift direction
    pub last_shift: Option<ShiftDirection>,

    /// counts how many steps the pulse constraints have been fulfilled
    pub pulse_counter: usize,

    /// keeps track on which positions can no longer be visited
    pub locked_positions: Array2<bool>,

    /// keeps track of all positions the walker has visited so far
    pub position_history: Vec<Position>,

    /// keeps track of current position locking step,
    pub locked_position_step: usize,
}

const NUM_SHIFT_SAMPLE_RETRIES: usize = 25;

impl fmt::Debug for CuteWalker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CuteWalker")
            .field("pos", &self.pos)
            .field("steps", &self.steps)
            // .field("inner_kernel", &self.inner_kernel)
            // .field("outer_kernel", &self.outer_kernel)
            .field("goal", &self.goal)
            .field("goal_index", &self.goal_index)
            // .field("waypoints", &self.waypoints)
            .field("finished", &self.finished)
            .field("steps_since_platform", &self.steps_since_platform)
            .field("last_shift", &self.last_shift)
            .field("pulse_counter", &self.pulse_counter)
            // .field("locked_positions", &self.locked_positions)
            // .field("position_history", &self.position_history)
            .field("locked_position_step", &self.locked_position_step)
            .finish()
    }
}

impl CuteWalker {
    pub fn new(
        initial_pos: Position,
        inner_kernel: Kernel,
        outer_kernel: Kernel,
        waypoints: Vec<Position>,
        map: &Map,
    ) -> CuteWalker {
        CuteWalker {
            pos: initial_pos,
            steps: 0,
            inner_kernel,
            outer_kernel,
            goal: Some(waypoints.first().unwrap().clone()),
            goal_index: 0,
            waypoints,
            finished: false,
            steps_since_platform: 0,
            last_shift: None,
            pulse_counter: 0,
            locked_positions: Array2::from_elem((map.width, map.height), false),
            locked_position_step: 0,
            position_history: Vec::new(),
        }
    }

    pub fn is_goal_reached(&self, waypoint_reached_dist: &usize) -> Option<bool> {
        self.goal
            .as_ref()
            .map(|goal| goal.distance_squared(&self.pos) <= *waypoint_reached_dist)
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

    pub fn check_platform_at_walker(
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

        // Case 2: max distance has been exceeded -> force platform using a room
        if self.steps_since_platform > max_distance {
            // generator::generate_room(map, &walker_pos.shifted_by(0, 6)?, 5, 3, None)?;
            // self.steps_since_platform = 0;
            // return Ok(());

            // try to place floor platform
            let mut pos = self.position_history[self.steps.saturating_sub(50)].clone();
            let mut reached_floor = false;
            while !reached_floor {
                if pos.shift_in_direction(&ShiftDirection::Down, map).is_err() {
                    break; // fail while shifting down -> abort!
                }

                if map.grid[pos.as_index()] == BlockType::Hookable {
                    reached_floor = true;
                    break;
                }
            }

            let platform_height = 2;
            let platform_free_height = 3;
            let platform_width = 2; //

            // check if area above platform is valid
            if reached_floor
                && map.check_area_all(
                    &pos.shifted_by(-platform_width, -(platform_height + platform_free_height))?,
                    &pos.shifted_by(platform_width, -2)?,
                    &BlockType::Empty,
                )?
            {
                map.set_area(
                    &pos.shifted_by(-platform_width, -2)?,
                    &pos.shifted_by(platform_width, -1)?,
                    &BlockType::Platform,
                    &Overwrite::ReplaceNonSolid,
                );

                self.steps_since_platform = 0;
            }

            return Ok(());
        }

        // Case 3: min distance has been exceeded -> Try to place platform, but only if possible
        let area_empty = map.check_area_all(
            &self.pos.shifted_by(-3, -3)?,
            &self.pos.shifted_by(3, 2)?,
            &BlockType::Empty,
        )?;
        if area_empty {
            map.set_area(
                &self.pos.shifted_by(-1, 0)?,
                &self.pos.shifted_by(1, 0)?,
                &BlockType::Platform,
                &Overwrite::ReplaceEmptyOnly,
            );
            self.steps_since_platform = 0;
        }

        Ok(())
    }

    pub fn probabilistic_step(
        &mut self,
        map: &mut Map,
        gen_config: &GenerationConfig,
        rnd: &mut Random,
        debug_layers: &mut Option<DebugLayers>,
    ) -> Result<(), &'static str> {
        if self.finished {
            return Err("Walker is finished");
        }

        // save position to history before its updated
        self.position_history.push(self.pos.clone());

        // sample next shift
        let goal = self.goal.as_ref().ok_or("Error: Goal is None")?;
        let shifts = self.pos.get_rated_shifts(goal, map);

        let mut current_shift = rnd.sample_shift(&shifts);

        // Momentum: re-use last shift direction with certain probability
        if let Some(last_shift) = self.last_shift {
            if rnd.with_probability(gen_config.momentum_prob) {
                current_shift = last_shift;
            }
        }

        let mut current_target_pos = self.pos.clone();
        current_target_pos.shift_in_direction(&current_shift, map)?;

        // if target pos is locked, re-sample until a valid one is found
        let mut invalid = false;
        for _ in 0..NUM_SHIFT_SAMPLE_RETRIES {
            invalid = self.locked_positions[current_target_pos.as_index()];

            if invalid {
                current_shift = rnd.sample_shift(&shifts);
                current_target_pos = self.pos.clone();
                current_target_pos.shift_in_direction(&current_shift, map)?;
            }
        }

        if invalid {
            return Err("number of shift sample retries exceeded, walker stuck?");
        }

        // determine if direction changed from last shift
        let same_dir = match self.last_shift {
            Some(last_shift) => current_shift == last_shift,
            None => false,
        };

        // apply selected shift
        self.pos.shift_in_direction(&current_shift, map)?;
        self.steps += 1;

        // lock old position
        self.lock_previous_location(map, gen_config, false)?;

        // TODO: this is so imperformant, i dont wanna do this all the time, hmm
        if let Some(debug_layers) = debug_layers {
            debug_layers.bool_layers.get_mut("lock").unwrap().grid = self.locked_positions.clone();
        }

        // perform pulse if config constraints allows it
        let perform_pulse = gen_config.enable_pulse
            && ((same_dir && self.pulse_counter > gen_config.pulse_straight_delay)
                || (!same_dir && self.pulse_counter > gen_config.pulse_corner_delay));

        // apply kernels
        if perform_pulse {
            self.pulse_counter = 0; // reset pulse counter
            map.apply_kernel(
                &self.pos,
                &Kernel::new(&self.inner_kernel.size + 4, 0.0),
                BlockType::Freeze,
            )?;
            map.apply_kernel(
                &self.pos,
                &Kernel::new(&self.inner_kernel.size + 2, 0.0),
                BlockType::Empty,
            )?;
        } else {
            map.apply_kernel(&self.pos, &self.outer_kernel, BlockType::Freeze)?;

            let empty = if self.steps < gen_config.fade_steps {
                BlockType::EmptyReserved
            } else {
                BlockType::Empty
            };
            map.apply_kernel(&self.pos, &self.inner_kernel, empty)?;
        };

        if same_dir && self.inner_kernel.size <= gen_config.pulse_max_kernel_size {
            self.pulse_counter += 1;
        } else {
            self.pulse_counter = 0;
        };

        self.last_shift = Some(current_shift);

        Ok(())
    }

    pub fn cuddle(&self) {
        println!("Cute walker was cuddled!");
    }

    /// fades kernel size from max_size to min_size for fade_steps
    pub fn set_fade_kernel(
        &mut self,
        step: usize,
        min_size: usize,
        max_size: usize,
        fade_steps: usize,
    ) {
        let slope = (min_size as f32 - max_size as f32) / fade_steps as f32;
        let kernel_size_f = (step as f32) * slope + max_size as f32;
        let kernel_size = kernel_size_f.floor() as usize;
        self.inner_kernel = Kernel::new(kernel_size, 0.0);
        self.outer_kernel = Kernel::new(kernel_size + 2, 0.0);
    }

    pub fn mutate_kernel(&mut self, config: &GenerationConfig, rnd: &mut Random) {
        let mut inner_size = self.inner_kernel.size;
        let mut inner_circ = self.inner_kernel.circularity;
        let mut outer_size = self.outer_kernel.size;
        let mut outer_circ = self.outer_kernel.circularity;
        let mut outer_margin = outer_size - inner_size;
        let mut modified = false;

        if rnd.with_probability(config.inner_size_mut_prob) {
            inner_size = rnd.sample_inner_kernel_size();
            modified = true;
        } else {
            rnd.skip_n(2); // for some reason sampling requires two values?
        }

        if rnd.with_probability(config.outer_size_mut_prob) {
            outer_margin = rnd.sample_outer_kernel_margin();
            modified = true;
        } else {
            rnd.skip_n(2);
        }

        if rnd.with_probability(config.inner_rad_mut_prob) {
            inner_circ = rnd.sample_circularity();
            modified = true;
        } else {
            rnd.skip_n(2);
        }

        if rnd.with_probability(config.outer_rad_mut_prob) {
            outer_circ = rnd.sample_circularity();
            modified = true;
        } else {
            rnd.skip_n(2);
        }

        outer_size = inner_size + outer_margin;

        // constraint 1: small circles must be fully rect
        if inner_size <= 3 {
            inner_circ = 0.0;
        }
        if outer_size <= 3 {
            outer_circ = 0.0;
        }

        // constraint 2: outer size cannot be smaller than inner
        assert!(outer_size >= inner_size); // this shoulnt happen -> crash!

        if modified {
            self.inner_kernel = Kernel::new(inner_size, inner_circ);
            self.outer_kernel = Kernel::new(outer_size, outer_circ);
        }
    }

    pub fn lock_previous_location(
        &mut self,
        map: &Map,
        gen_config: &GenerationConfig,
        ignore_distance: bool,
    ) -> Result<(), &'static str> {
        while self.locked_position_step < self.steps {
            if self.position_history.len() <= self.locked_position_step + 1 {
                return Ok(()); // history not long enough yet to lock another step
            }

            // get position of the next step to lock
            let next_lock_pos = &self.position_history[self.locked_position_step + 1];

            // check if locking lacks too far behind -> walker most likely stuck
            if self.steps - self.locked_position_step > gen_config.pos_lock_max_delay {
                return Err("pos_lock_max_delay exceeded, walker stuck");
            }

            // check if walker is far enough to lock next position
            if !ignore_distance && next_lock_pos.distance(&self.pos) < gen_config.pos_lock_max_dist
            {
                return Ok(());
            }

            // TODO: rework this by reusing functionality -> lock possible cells
            let offset: usize = gen_config.lock_kernel_size; // offset of kernel wrt. position (top/left)
            let extend: usize = (gen_config.lock_kernel_size * 2) - offset; // how much kernel extends position (bot/right)
            let top_left = next_lock_pos.shifted_by(-(offset as i32), -(offset as i32))?;
            let bot_right = next_lock_pos.shifted_by(extend as i32, extend as i32)?;

            // check if operation valid
            if !map.pos_in_bounds(&top_left) || !map.pos_in_bounds(&bot_right) {
                return Err("kill zone out of bounds");
            }

            // lock all
            let mut view = self
                .locked_positions
                .slice_mut(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);
            for lock_status in view.iter_mut() {
                *lock_status = true;
            }

            self.locked_position_step += 1;
        }

        Ok(())
    }
}
