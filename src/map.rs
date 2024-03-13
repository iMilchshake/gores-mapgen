use crate::CuteWalker;
use crate::Position;
use egui::epaint::ahash::HashMap;
use egui::epaint::ahash::HashMapExt;
use itertools::Itertools;
use ndarray::Array2;

#[derive(Debug, Clone, Copy)]
pub enum BlockType {
    Empty,
    Hookable,
    Freeze,
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
}

impl Map {
    pub fn new(width: usize, height: usize, default: BlockType) -> Map {
        Map {
            grid: Array2::from_elem((width, height), default),
            width,
            height,
        }
    }

    pub fn update(
        &mut self,
        walker: &CuteWalker,
        kernel_type: KernelType,
    ) -> Result<(), &'static str> {
        let offset: usize = walker.kernel.size / 2; // offset of kernel wrt. position (top/left)
        let extend: usize = walker.kernel.size - offset; // how much kernel extends position (bot/right)

        let exceeds_left_bound = walker.pos.x < offset;
        let exceeds_upper_bound = walker.pos.y < offset;
        let exceeds_right_bound = (walker.pos.x + extend) > self.width;
        let exceeds_lower_bound = (walker.pos.y + extend) > self.height;

        if exceeds_left_bound || exceeds_upper_bound || exceeds_right_bound || exceeds_lower_bound {
            return Err("kernel out of bounds");
        }

        let root_pos = Position::new(walker.pos.x - offset, walker.pos.y - offset);
        for ((kernel_x, kernel_y), kernel_active) in walker.kernel.vector.indexed_iter() {
            let absolute_pos = Position::new(root_pos.x + kernel_x, root_pos.y + kernel_y);
            if *kernel_active {
                let current_type = self.grid[absolute_pos.as_index()];
                let new_type = match (&kernel_type, current_type) {
                    // inner kernel removes everything
                    (KernelType::Inner, _) => BlockType::Empty,

                    // outer kernel will turn hookables to freeze
                    (KernelType::Outer, BlockType::Hookable) => BlockType::Freeze,
                    (KernelType::Outer, BlockType::Freeze) => BlockType::Freeze,
                    (KernelType::Outer, BlockType::Empty) => BlockType::Empty,
                };
                self.grid[absolute_pos.as_index()] = new_type;
            }
        }

        Ok(())
    }

    fn is_pos_in_bounds(&self, pos: Position) -> bool {
        // we dont have to check for lower bound, because of usize
        pos.x < self.width && pos.y < self.height
    }
}
