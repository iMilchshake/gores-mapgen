use crate::BlockType;
use crate::CuteWalker;
use crate::Position;
use ndarray::Array2;

#[derive(Debug)]
pub struct Map {
    pub grid: Array2<BlockType>,
    pub height: usize,
    pub width: usize,
}

#[derive(Debug)]
pub struct Kernel {
    pub size: usize,
    pub circularity: f32,
    pub vector: Array2<bool>,
}

impl Kernel {
    pub fn new(size: usize, circularity: f32) -> Kernel {
        Kernel {
            size,
            circularity,
            vector: Kernel::get_kernel(size, circularity),
        }
    }

    fn get_kernel(size: usize, circularity: f32) -> Array2<bool> {
        let mut kernel = Array2::from_elem((size, size), false);
        let center = (size - 1) as f32 / 2.0;

        // calculate radii based on the size and circularity
        let min_radius = (size - 1) as f32 / 2.0; // min radius is from center to border
        let max_radius = f32::sqrt(center * center + center * center); // max radius is from center to corner
        let radius = circularity * min_radius + (1.0 - circularity) * max_radius;

        for ((x, y), value) in kernel.indexed_iter_mut() {
            let distance = f32::sqrt(
                (x as f32 - center) * (x as f32 - center)
                    + (y as f32 - center) * (y as f32 - center),
            );
            if distance <= radius {
                *value = true;
            }
        }

        kernel
    }
}

impl Map {
    pub fn new(width: usize, height: usize, default: BlockType) -> Map {
        Map {
            grid: Array2::from_elem((width, height), default),
            width,
            height,
        }
    }

    pub fn update(&mut self, walker: &CuteWalker, value: BlockType) -> Result<(), &'static str> {
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
        for ((x, y), kernel_active) in walker.kernel.vector.indexed_iter() {
            if *kernel_active {
                self.grid[[root_pos.x + x, root_pos.y + y]] = value;
            }
        }

        Ok(())
    }

    fn is_pos_in_bounds(&self, pos: Position) -> bool {
        // we dont have to check for lower bound, because of usize
        pos.x < self.width && pos.y < self.height
    }
}
