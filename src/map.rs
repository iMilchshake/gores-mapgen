use crate::BlockType;
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
        dbg!(&kernel);
        dbg!(&center);

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

    fn is_pos_in_bounds(&self, pos: Position) -> bool {
        // we dont have to check for lower bound, because of usize
        pos.x < self.width && pos.y < self.height
    }
}
