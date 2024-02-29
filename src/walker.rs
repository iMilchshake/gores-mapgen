use crate::{BlockType, Kernel, Map, Position, Random, ShiftDirection};

// this walker is indeed very cute
#[derive(Debug)]
pub struct CuteWalker {
    pub pos: Position,
    pub steps: usize,
    pub curr_goal: Position,
    pub waypoints: Vec<Position>,
    pub kernel: Kernel,
}

impl CuteWalker {
    pub fn new(initial_pos: Position, mut waypoints: Vec<Position>, kernel: Kernel) -> CuteWalker {
        CuteWalker {
            pos: initial_pos,
            steps: 0,
            curr_goal: waypoints
                .pop()
                .expect("expect at least one waypoint on initialization"),
            waypoints,
            kernel,
        }
    }

    pub fn next_waypoint(&mut self) -> Result<(), ()> {
        if let Some(next_goal) = self.waypoints.pop() {
            self.curr_goal = next_goal;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn greedy_step(&mut self, map: &mut Map) -> Result<(), &'static str> {
        let greedy_shift = self.pos.get_greedy_shift(&self.curr_goal);

        // apply that shift
        self.pos.shift(greedy_shift, map)?;
        self.steps += 1;

        // remove blocks using a kernel at current position
        map.update(self, BlockType::Filled)?;

        Ok(())
    }

    pub fn probabilistic_step(
        &mut self,
        map: &mut Map,
        rnd: &mut Random,
    ) -> Result<(), &'static str> {
        let shifts = self.pos.get_rated_shifts(&self.curr_goal, map);
        let sampled_shift = rnd.sample_move(shifts);

        // apply that shift
        self.pos.shift(sampled_shift, map)?;
        self.steps += 1;

        // remove blocks using a kernel at current position
        map.update(self, BlockType::Filled)?;

        Ok(())
    }

    pub fn cuddle(&self) {
        println!("Cute walker was cuddled!");
    }
}
