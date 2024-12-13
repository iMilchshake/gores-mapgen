use crate::{
    kernel::Kernel,
    position::{Position, ShiftDirection},
    twmap_export::TwExport,
};
use ndarray::{s, Array2};

use std::{char, path::PathBuf};

const CHUNK_SIZE: usize = 5;
const MAX_SHIFT_UNTIL_STEPS: usize = 25;

#[derive(PartialEq)]
pub enum BlockTypeTW {
    Hookable,
    Freeze,
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Empty,
    /// Empty Block that should not be overwritten
    EmptyReserved,
    Hookable,
    Platform,
    Freeze,
    Spawn,
    Start,
    Finish,
}

impl BlockType {
    /// maps BlockType to tw game layer id for map export
    pub fn to_tw_game_id(&self) -> u8 {
        match self {
            BlockType::Empty | BlockType::EmptyReserved => 0,
            BlockType::Hookable | BlockType::Platform => 1,
            BlockType::Freeze => 9,
            BlockType::Spawn => 192,
            BlockType::Start => 33,
            BlockType::Finish => 34,
        }
    }

    pub fn to_tw_block_type(&self) -> BlockTypeTW {
        match self {
            BlockType::Platform | BlockType::Hookable => BlockTypeTW::Hookable,
            BlockType::Empty | BlockType::EmptyReserved => BlockTypeTW::Empty,
            BlockType::Freeze => BlockTypeTW::Freeze,

            // every other block is just mapped to empty
            _ => BlockTypeTW::Empty,
        }
    }

    pub fn is_solid(&self) -> bool {
        matches!(self, BlockType::Hookable | BlockType::Platform)
    }

    pub fn is_freeze(&self) -> bool {
        matches!(self, BlockType::Freeze)
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, BlockType::Empty)
    }
}

pub enum Overwrite {
    /// Replace EVERYTHING
    Force,

    /// Replace Hookable+Freeze
    ReplaceSolidFreeze,

    /// Replace Hookable
    ReplaceSolidOnly,

    /// Replace Empty
    ReplaceEmptyOnly,

    /// Replace Freeze+Empty
    ReplaceNonSolid,

    /// Replace Freeze+Empty+EmptyReserved
    ReplaceNonSolidForce,
}

impl Overwrite {
    fn will_override(&self, btype: &BlockType) -> bool {
        match self {
            Overwrite::Force => true,
            Overwrite::ReplaceSolidFreeze => {
                matches!(&btype, BlockType::Hookable | BlockType::Freeze)
            }
            Overwrite::ReplaceSolidOnly => matches!(&btype, BlockType::Hookable),
            Overwrite::ReplaceEmptyOnly => matches!(&btype, BlockType::Empty),
            Overwrite::ReplaceNonSolid => matches!(&btype, BlockType::Freeze | BlockType::Empty),
            Overwrite::ReplaceNonSolidForce => matches!(
                &btype,
                BlockType::Freeze | BlockType::Empty | BlockType::EmptyReserved
            ),
        }
    }
}

pub enum KernelType {
    Outer,
    Inner,
}

#[derive(Debug)]
pub struct Map {
    pub grid: Array2<BlockType>,
    pub font_layer: Array2<char>,
    pub noise_overlay: Array2<bool>,
    pub noise_background: Array2<bool>,
    pub height: usize,
    pub width: usize,
    pub chunk_edited: Array2<bool>, // TODO: make this optional in case editor is not used!
    pub chunk_size: usize,
}

impl Map {
    pub fn new(width: usize, height: usize, default: BlockType) -> Map {
        Map {
            grid: Array2::from_elem((width, height), default),
            font_layer: Array2::from_elem((width, height), ' '),
            noise_overlay: Array2::from_elem((width, height), false),
            noise_background: Array2::from_elem((width, height), false),
            width,
            height,
            chunk_edited: Array2::from_elem(
                (width.div_ceil(CHUNK_SIZE), height.div_ceil(CHUNK_SIZE)),
                false,
            ),
            chunk_size: CHUNK_SIZE,
        }
    }

    pub fn apply_kernel(
        &mut self,
        pos: &Position,
        kernel: &Kernel,
        new_block_type: BlockType,
    ) -> Result<(), &'static str> {
        let offset: usize = kernel.size / 2; // offset of kernel wrt. position (top/left)
        let extend: usize = kernel.size - offset; // how much kernel extends position (bot/right)

        let exceeds_left_bound = pos.x < offset;
        let exceeds_upper_bound = pos.y < offset;
        let exceeds_right_bound = (pos.x + extend) > self.width;
        let exceeds_lower_bound = (pos.y + extend) > self.height;

        if exceeds_left_bound || exceeds_upper_bound || exceeds_right_bound || exceeds_lower_bound {
            return Err("Kernel out of bounds");
        }

        let root_pos = Position::new(pos.x - offset, pos.y - offset);
        for ((kernel_x, kernel_y), kernel_active) in kernel.vector.indexed_iter() {
            let absolute_pos = Position::new(root_pos.x + kernel_x, root_pos.y + kernel_y);
            if *kernel_active {
                let current_type = &self.grid[absolute_pos.as_index()];

                let new_type = match current_type {
                    BlockType::Hookable | BlockType::Freeze => Some(new_block_type.clone()),
                    _ => None,
                };

                if let Some(new_type) = new_type {
                    self.grid[absolute_pos.as_index()] = new_type;
                }

                let chunk_pos = self.pos_to_chunk_pos(absolute_pos);
                self.chunk_edited[chunk_pos.as_index()] = true;
            }
        }

        Ok(())
    }

    fn pos_to_chunk_pos(&self, pos: Position) -> Position {
        Position::new(pos.x / self.chunk_size, pos.y / self.chunk_size)
    }

    pub fn export(&self, path: &PathBuf) {
        TwExport::export(self, path)
    }

    pub fn pos_in_bounds(&self, pos: &Position) -> bool {
        // we dont have to check for lower bound, because of usize
        pos.x < self.width && pos.y < self.height
    }

    pub fn check_area_exists(
        &self,
        top_left: &Position,
        bot_right: &Position,
        value: &BlockType,
    ) -> Result<bool, &'static str> {
        if !self.pos_in_bounds(top_left) || !self.pos_in_bounds(bot_right) {
            return Err("checking area out of bounds");
        }

        let area = self
            .grid
            .slice(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

        Ok(area.iter().any(|block| block == value))
    }

    pub fn check_area_all(
        &self,
        top_left: &Position,
        bot_right: &Position,
        value: &BlockType,
    ) -> Result<bool, &'static str> {
        if !self.pos_in_bounds(top_left) || !self.pos_in_bounds(bot_right) {
            return Err("checking area out of bounds");
        }
        let area = self
            .grid
            .slice(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

        Ok(area.iter().all(|block| block == value))
    }

    pub fn count_occurence_in_area(
        &self,
        top_left: &Position,
        bot_right: &Position,
        value: &BlockType,
    ) -> Result<usize, &'static str> {
        if !self.pos_in_bounds(top_left) || !self.pos_in_bounds(bot_right) {
            return Err("checking area out of bounds");
        }
        let area = self
            .grid
            .slice(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

        Ok(area.iter().filter(|&block| block == value).count())
    }

    pub fn check_position_type(&self, pos: &Position, block_type: BlockType) -> bool {
        match self.grid.get(pos.as_index()) {
            Some(value) => *value == block_type,
            None => false,
        }
    }

    pub fn check_position_crit<F>(&self, pos: &Position, criterion: F) -> bool
    where
        F: Fn(&BlockType) -> bool,
    {
        match self.grid.get(pos.as_index()) {
            Some(value) => criterion(value),
            None => false,
        }
    }

    pub fn set_area(
        &mut self,
        top_left: &Position,
        bot_right: &Position,
        value: &BlockType,
        overide: &Overwrite,
    ) {
        if !self.pos_in_bounds(top_left) || !self.pos_in_bounds(bot_right) {
            return;
        }

        let chunk_size = self.chunk_size;

        let mut view = self
            .grid
            .slice_mut(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

        for ((x, y), current_value) in view.indexed_iter_mut() {
            if overide.will_override(current_value) {
                *current_value = value.clone();

                let chunk_pos =
                    Position::new((top_left.x + x) / chunk_size, (top_left.y + y) / chunk_size);
                self.chunk_edited[chunk_pos.as_index()] = true;
            }
        }
    }

    /// sets the outline of an area define by two positions
    pub fn set_area_border(
        &mut self,
        top_left: &Position,
        bot_right: &Position,
        value: &BlockType,
        overwrite: &Overwrite,
    ) {
        let top_right = Position::new(bot_right.x, top_left.y);
        let bot_left = Position::new(top_left.x, bot_right.y);

        self.set_area(top_left, &top_right, value, overwrite);
        self.set_area(&top_right, bot_right, value, overwrite);
        self.set_area(top_left, &bot_left, value, overwrite);
        self.set_area(&bot_left, bot_right, value, overwrite);
    }

    /// shifts position in given direction until block fulfills criterion
    pub fn shift_pos_until<F>(
        &self,
        pos: &Position,
        dir: ShiftDirection,
        criterion: F,
    ) -> Option<Position>
    where
        F: Fn(&BlockType) -> bool,
    {
        let mut shift_pos = pos.clone();
        for _ in 0..MAX_SHIFT_UNTIL_STEPS {
            // shift in given direction
            if shift_pos.shift_in_direction(&dir, self).is_err() {
                return None; // fail while shifting -> abort
            } else if criterion(&self.grid[shift_pos.as_index()]) {
                return Some(shift_pos); // criterion fulfilled -> return current position
            }
        }

        None // criterion was never fulfilled
    }
}
