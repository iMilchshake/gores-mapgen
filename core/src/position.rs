use serde::{Deserialize, Serialize};

use crate::map::Map;
use std::usize;

// using my own position vector to meet ndarray's indexing standard using usize
//
// while glam has nice performance benefits, the amount of expensive operations
// on the position vector will be very limited, so this should be fine..
#[derive(Debug, Default, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Serialize, Deserialize, Default)]
pub enum ShiftDirection {
    #[default]
    Up = 0,
    Right = 1,
    Down = 2,
    Left = 3,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Position {
        Position { x, y }
    }

    pub fn as_index(&self) -> [usize; 2] {
        [self.x, self.y]
    }

    /// returns a new position shifted by some x and y value
    pub fn shifted_by(&self, x_shift: i32, y_shift: i32) -> Result<Position, &'static str> {
        let new_x = match x_shift >= 0 {
            true => self.x + (x_shift as usize),
            false => self
                .x
                .checked_sub((-x_shift) as usize)
                .ok_or("invalid shift")?,
        };

        let new_y = match y_shift >= 0 {
            true => self.y + y_shift as usize,
            false => self
                .y
                .checked_sub((-y_shift) as usize)
                .ok_or("invalid shift")?,
        };

        Ok(Position::new(new_x, new_y))
    }

    pub fn shift_in_direction(
        &mut self,
        shift: ShiftDirection,
        map: &Map,
    ) -> Result<(), &'static str> {
        if !self.is_shift_valid(shift, map) {
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

    pub fn is_shift_valid(&self, shift: ShiftDirection, map: &Map) -> bool {
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

    /// squared euclidean distance between two Positions
    pub fn distance_squared(&self, rhs: &Position) -> usize {
        self.x.abs_diff(rhs.x).saturating_pow(2) + self.y.abs_diff(rhs.y).saturating_pow(2)
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

        shifts.sort_by_cached_key(|&shift| {
            let mut shifted_pos = self.clone();
            if let Ok(()) = shifted_pos.shift_in_direction(shift, map) {
                shifted_pos.distance_squared(goal)
            } else {
                // assign maximum distance to invalid shifts
                // TODO: i could also return a vec and completly remove invalid moves?
                usize::MAX
            }
        });

        shifts
    }
}
