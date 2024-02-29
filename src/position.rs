use std::{ops::Mul, usize};

use crate::{Map, ShiftDirection};

// using my own position vector to meet ndarray's indexing standard using usize
//
// while glam has nice performance benefits, the amount of expensive operations
// on the position vector will be very limited, so this should be fine..
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Position {
        Position { x, y }
    }

    pub fn as_index(&self) -> [usize; 2] {
        [self.x, self.y]
    }

    pub fn shift(&mut self, shift: ShiftDirection, map: &Map) -> Result<(), &'static str> {
        if !self.is_shift_valid(&shift, map) {
            return Err("invalid shift");
        }

        match shift {
            ShiftDirection::Up => self.y -= 1,
            ShiftDirection::Right => self.x += 1,
            ShiftDirection::Down => self.y += 1,
            ShiftDirection::Left => self.x -= 1,
        }

        Ok(())
    }

    pub fn is_shift_valid(&self, shift: &ShiftDirection, map: &Map) -> bool {
        match shift {
            ShiftDirection::Up => self.y > 0,
            ShiftDirection::Right => self.x < map.width - 1,
            ShiftDirection::Down => self.y < map.height - 1,
            ShiftDirection::Left => self.x > 0,
        }
    }

    pub fn get_greedy_shift(&self, goal: &Position) -> ShiftDirection {
        let x_diff = goal.x as isize - self.x as isize;
        let x_abs_diff = x_diff.abs();
        let y_diff = goal.y as isize - self.y as isize;
        let y_abs_diff = y_diff.abs();

        // check whether x or y is dominant
        if x_abs_diff > y_abs_diff {
            if x_diff.is_positive() {
                ShiftDirection::Right
            } else {
                ShiftDirection::Left
            }
        } else if y_diff.is_positive() {
            ShiftDirection::Down
        } else {
            ShiftDirection::Up
        }
    }

    /// returns a Vec with all possible shifts, sorted by how close they get
    /// towards the goal position
    pub fn get_rated_shifts(&self, goal: &Position, map: &Map) -> [ShiftDirection; 4] {
        let mut shifts = [
            ShiftDirection::Left,
            ShiftDirection::Up,
            ShiftDirection::Right,
            ShiftDirection::Down,
        ];

        // TODO: make this safe XD
        shifts.sort_by_cached_key(|shift| {
            let mut shifted_pos = self.clone();

            if let Ok(()) = shifted_pos.shift(*shift, map) {
                let x_diff = shifted_pos.x.abs_diff(goal.x);
                let y_diff = shifted_pos.y.abs_diff(goal.y);
                let squared_dist =
                    usize::saturating_mul(x_diff, x_diff) + usize::saturating_mul(y_diff, y_diff);
                squared_dist
            } else {
                usize::MAX
            }
        });

        shifts
    }
}
