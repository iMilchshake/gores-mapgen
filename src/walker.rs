use crate::{Kernel, KernelType, Map, Position, Random};

// this walker is indeed very cute
#[derive(Debug)]
pub struct CuteWalker {
    pub pos: Position,
    pub steps: usize,
    pub kernel: Kernel,

    pub goal: Option<Position>,
    pub goal_index: usize,
    pub waypoints: Vec<Position>,
}

impl CuteWalker {
    pub fn new(initial_pos: Position, waypoints: Vec<Position>, kernel: Kernel) -> CuteWalker {
        CuteWalker {
            pos: initial_pos,
            steps: 0,
            kernel,
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
        self.kernel = Kernel::new(9, Position::new(4, 1));
        map.update(self, KernelType::Outer)?;

        self.kernel = Kernel::new(7, Position::new(3, 0));
        map.update(self, KernelType::Inner)?;

        Ok(())
    }

    pub fn cuddle(&self) {
        println!("Cute walker was cuddled!");
    }
}
