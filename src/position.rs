use dt::num::ToPrimitive;
use serde::{Deserialize, Serialize};

use crate::{map::Map, random::Random};
use std::f32::consts::PI;

/// smallest difference between two angles in degrees
pub fn angle_difference_deg(angle1: f32, angle2: f32) -> f32 {
    let diff = (angle1 - angle2).abs() % 360.0;
    if diff > 180.0 {
        360.0 - diff
    } else {
        diff
    }
}

// using my own position vector to meet ndarray's indexing standard using usize
//
// while glam has nice performance benefits, the amount of expensive operations
// on the position vector will be very limited, so this should be fine..
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
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
        shift: &ShiftDirection,
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

    /// will return a randomly shifted
    pub fn random_shift(
        &self,
        rnd: &mut Random,
        max_distance: f32,
    ) -> Result<Position, &'static str> {
        let direction_radians = rnd.get_unit_ratio() * 2.0 * PI;
        let distance = rnd.get_unit_ratio() * max_distance;

        let delta_x = distance * direction_radians.cos();
        let delta_y = distance * direction_radians.sin();

        self.shifted_by(delta_x.round() as i32, delta_y.round() as i32)
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

    /// squared euclidean distance between two Positions
    pub fn distance_squared(&self, rhs: &Position) -> usize {
        self.x.abs_diff(rhs.x).saturating_pow(2) + self.y.abs_diff(rhs.y).saturating_pow(2)
    }

    /// euclidean distance between two Positions
    pub fn distance(&self, rhs: &Position) -> f32 {
        (self.x.abs_diff(rhs.x).saturating_pow(2) + self.y.abs_diff(rhs.y).saturating_pow(2))
            .to_f32()
            .unwrap()
            .sqrt()
    }

    /// angle to other position in degrees
    pub fn angle_deg(&self, to: &Position) -> f32 {
        let dx = to.x as f32 - self.x as f32;
        let dy = to.y as f32 - self.y as f32;
        let angle_rad = dy.atan2(dx);
        let angle_deg = angle_rad * 180.0 / PI;
        angle_deg
    }

    /// linear interpolation with another point
    pub fn lerp(&self, other: &Position, weight: f32) -> Position {
        let lerp_x = (self.x as f32 * (1.0 - weight) + other.x as f32 * weight).round() as usize;
        let lerp_y = (self.y as f32 * (1.0 - weight) + other.y as f32 * weight).round() as usize;

        Position {
            x: lerp_x,
            y: lerp_y,
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

        shifts.sort_by_cached_key(|shift| {
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
