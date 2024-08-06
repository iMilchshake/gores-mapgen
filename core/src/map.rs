use crate::{config::MapConfig, kernel::Kernel, position::Position, walker::CuteWalker};
use ndarray::{s, Array2};

const CHUNK_SIZE: usize = 5;

#[derive(Debug, PartialEq)]
pub enum GameTile {
    Hookable,
    Freeze,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockType {
    Empty,
    /// Empty Block that should not be overwritten
    EmptyReserved,
    Hookable,
    Freeze,
    Spawn,
    Start,
    Finish,
    Platform,
}

impl BlockType {
    /// maps BlockType to tw game layer id for map export
    pub fn to_ingame_id(&self) -> u8 {
        match self {
            BlockType::Empty | BlockType::EmptyReserved => 0,
            BlockType::Hookable | BlockType::Platform => 1,
            BlockType::Freeze => 9,
            BlockType::Spawn => 192,
            BlockType::Start => 33,
            BlockType::Finish => 34,
        }
    }

    pub fn to_game_tile(&self) -> GameTile {
        match self {
            BlockType::Platform | BlockType::Hookable => GameTile::Hookable,
            BlockType::Empty | BlockType::EmptyReserved => GameTile::Empty,
            BlockType::Freeze => GameTile::Freeze,

            // every other block is just mapped to empty
            _ => GameTile::Empty,
        }
    }

    pub fn is_solid(&self) -> bool {
        matches!(self, BlockType::Hookable | BlockType::Platform)
    }
    pub fn is_freeze(&self) -> bool {
        matches!(self, BlockType::Freeze)
    }
}

#[derive(Clone, Copy)]
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
    pub height: usize,
    pub width: usize,
    pub chunks_edited: Array2<bool>, // TODO: make this optional in case editor is not used!
    pub chunk_size: usize,
    pub config: MapConfig
}

impl Map {
    pub fn new(config: MapConfig, default: BlockType) -> Map {
        let width = config.width;
        let height = config.height;

        Map {
            grid: Array2::from_elem((width, height), default),
            width,
            height,
            chunks_edited: Array2::from_elem(
                (width.div_ceil(CHUNK_SIZE), height.div_ceil(CHUNK_SIZE)),
                false,
            ),
            chunk_size: CHUNK_SIZE,
            config
        }
    }

    pub fn apply_kernel(
        &mut self,
        walker: &CuteWalker,
        kernel: &Kernel,
        block_type: BlockType,
    ) -> Result<(), &'static str> {
        let offset: usize = kernel.size / 2; // offset of kernel wrt. position (top/left)
        let extend: usize = kernel.size - offset; // how much kernel extends position (bot/right)

        let exceeds_left_bound = walker.pos.x < offset;
        let exceeds_upper_bound = walker.pos.y < offset;
        let exceeds_right_bound = (walker.pos.x + extend) > self.width;
        let exceeds_lower_bound = (walker.pos.y + extend) > self.height;

        if exceeds_left_bound || exceeds_upper_bound || exceeds_right_bound || exceeds_lower_bound {
            return Err("Kernel out of bounds");
        }

        let root_pos = Position::new(walker.pos.x - offset, walker.pos.y - offset);
        for ((kernel_x, kernel_y), kernel_active) in kernel.vector.indexed_iter() {
            let absolute_pos = Position::new(root_pos.x + kernel_x, root_pos.y + kernel_y);
            if *kernel_active {
                let current_type = &self.grid[absolute_pos.as_index()];

                let new_type = match current_type {
                    BlockType::Hookable | BlockType::Freeze => Some(block_type.clone()),
                    _ => None,
                };

                if let Some(new_type) = new_type {
                    self.grid[absolute_pos.as_index()] = new_type;
                }

                let chunk_pos = self.pos_to_chunk_pos(absolute_pos);
                self.chunks_edited[chunk_pos.as_index()] = true;
            }
        }

        Ok(())
    }

    fn pos_to_chunk_pos(&self, pos: Position) -> Position {
        Position::new(pos.x / self.chunk_size, pos.y / self.chunk_size)
    }
    
    pub fn pos_in_bounds(&self, pos: &Position) -> bool {
        // we dont have to check for lower bound, because of usize
        pos.x < self.width && pos.y < self.height
    }

    pub fn check_area_exists(
        &self,
        top_left: Position,
        bot_right: Position,
        value: BlockType,
    ) -> Result<bool, &'static str> {
        if !self.pos_in_bounds(&top_left) || !self.pos_in_bounds(&bot_right) {
            return Err("checking area out of bounds");
        }

        let area = self
            .grid
            .slice(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

        Ok(area.iter().any(|&block| block == value))
    }

    pub fn check_area_all(
        &self,
        top_left: Position,
        bot_right: Position,
        value: BlockType,
    ) -> Result<bool, &'static str> {
        if !self.pos_in_bounds(&top_left) || !self.pos_in_bounds(&bot_right) {
            return Err("checking area out of bounds");
        }
        let area = self
            .grid
            .slice(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

        Ok(area.iter().all(|&block| block == value))
    }

    pub fn count_occurence_in_area(
        &self,
        top_left: Position,
        bot_right: Position,
        value: BlockType,
    ) -> Result<usize, &'static str> {
        if !self.pos_in_bounds(&top_left) || !self.pos_in_bounds(&bot_right) {
            return Err("checking area out of bounds");
        }
        let area = self
            .grid
            .slice(s![top_left.x..=bot_right.x, top_left.y..=bot_right.y]);

        Ok(area.iter().filter(|&&block| block == value).count())
    }

    pub fn set_area(
        &mut self,
        top_left: Position,
        bot_right: Position,
        value: BlockType,
        overide: Overwrite,
    ) {
        if !self.pos_in_bounds(&top_left) || !self.pos_in_bounds(&bot_right) {
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
                self.chunks_edited[chunk_pos.as_index()] = true;
            }
        }
    }

    /// sets the outline of an area define by two positions
    pub fn set_area_border(
        &mut self,
        top_left: Position,
        bot_right: Position,
        value: BlockType,
        overwrite: Overwrite,
    ) {
        let top_right = Position::new(bot_right.x, top_left.y);
        let bot_left = Position::new(top_left.x, bot_right.y);

        self.set_area(top_left, top_right, value, overwrite);
        self.set_area(top_right, bot_right, value, overwrite);
        self.set_area(top_left, bot_left, value, overwrite);
        self.set_area(bot_left, bot_right, value, overwrite);
    }
}
