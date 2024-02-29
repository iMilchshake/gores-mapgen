use crate::{BlockType, Kernel, Map, Position, ShiftDirection};

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
        // get greedy shift towards goal
        let shift = self.pos.get_greedy_dir(&self.curr_goal);

        // apply that shift
        self.shift_pos(shift, map)?;

        // remove blocks using a kernel at current position
        map.update(self, BlockType::Filled)?;

        Ok(())
    }

    pub fn cuddle(&self) {
        println!("Cute walker was cuddled!");
    }

    pub fn shift_pos(&mut self, shift: ShiftDirection, map: &Map) -> Result<(), &'static str> {
        if !self.is_shift_valid(&shift, map) {
            return Err("invalid shift");
        }

        match shift {
            ShiftDirection::Up => self.pos.y -= 1,
            ShiftDirection::Right => self.pos.x += 1,
            ShiftDirection::Down => self.pos.y += 1,
            ShiftDirection::Left => self.pos.x -= 1,
        }

        self.steps += 1;

        Ok(())
    }

    pub fn is_shift_valid(&self, shift: &ShiftDirection, map: &Map) -> bool {
        match shift {
            ShiftDirection::Up => self.pos.y > 0,
            ShiftDirection::Right => self.pos.x < map.width - 1,
            ShiftDirection::Down => self.pos.y < map.height - 1,
            ShiftDirection::Left => self.pos.x > 0,
        }
    }
}
