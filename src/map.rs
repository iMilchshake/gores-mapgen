use crate::draw_grid_blocks;
use crate::CuteWalker;
use crate::Position;
use itertools::Itertools;
use ndarray::Array2;
use rand_distr::num_traits::Pow;

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

#[derive(Debug)]
pub struct Kernel {
    pub size: usize,
    pub radius_sqr: usize,
    pub vector: Array2<bool>,
    pub max_distance_sqr: usize,
}

// TODO: getting max_radius or the kernel_vector involves sqrt()'s. In the future i should at least
// replace the comparison in get_kernel() with squared radii.

impl Kernel {
    pub fn new(size: usize, radius_sqr: usize) -> Kernel {
        let (vector, max_distance_sqr) = Kernel::get_kernel_vector(size, radius_sqr);
        Kernel {
            size,
            radius_sqr,
            vector,
            max_distance_sqr,
        }
    }

    fn get_kernel_center(size: usize) -> usize {
        (size - 1) / 2
    }

    pub fn get_valid_radius_bounds(size: usize) -> (usize, usize) {
        // TODO: center and min_radius are actually the same value
        let center = Kernel::get_kernel_center(size);

        let min_radius = ((size - 1) / 2).pow(2); // min radius is from center to border
        let max_radius = center * center + center * center; // max radius is from center to corner

        (min_radius, max_radius)
    }

    pub fn is_valid_radius(size: usize, radius_sqr: usize) -> bool {
        let (min_radius, max_radius) = Kernel::get_valid_radius_bounds(size);
        let is_valid = min_radius <= radius_sqr && radius_sqr <= max_radius;

        is_valid
    }

    // pub fn get_min_circularity(size: usize, radius_limit: f32) -> f32 {
    //     let center = Kernel::get_kernel_center(size);
    //
    //     let min_radius = (size - 1) as f32 / 2.0;
    //     let max_radius = f32::sqrt(center * center + center * center);
    //
    //     let actual_max_radius = f32::min(max_radius, radius_limit); // get LOWER bound
    //
    //     // calculate circularity which results in actual max radius by linear combination of min
    //     // and max radius
    //     // a=xb+(1-x)c => x = (a-c)/(b-c)
    //
    //     let min_circularity = (actual_max_radius - max_radius) / (min_radius - max_radius);
    //
    //     min_circularity
    // }

    /// TODO: this could also be further optimized by using the kernels symmetry
    fn get_kernel_vector(size: usize, radius_sqr: usize) -> (Array2<bool>, usize) {
        let center = Kernel::get_kernel_center(size);
        let mut max_distance_sqr = 0;
        dbg!(&center);

        let mut kernel = Array2::from_elem((size, size), false);
        for ((x, y), value) in kernel.indexed_iter_mut() {
            let distance = x.abs_diff(center).pow(2) + y.abs_diff(center).pow(2);
            if distance <= radius_sqr {
                *value = true;

                if distance >= max_distance_sqr {
                    max_distance_sqr = distance;
                }
            }
        }

        (kernel, max_distance_sqr)
    }

    /// iterate over all possible distances from center to valid positions within the kernel bounds
    /// to get all possible squared radii. This returns a Vec of all possible squared radii that
    /// limit at least one possible location in the kernel, so each results in a unique kernel
    pub fn get_unique_radii_sqr(size: usize) -> Vec<usize> {
        let mut valid_sqr_distances: Vec<usize> = Vec::new();
        let center = Kernel::get_kernel_center(size);
        let max_offset = size - center - 1;
        let min_radius_sqr = Kernel::get_valid_radius_bounds(size).0;

        for x in 0..=max_offset {
            // due to symmetry only look at values >= x
            for y in x..=max_offset {
                let distance_sqr = x * x + y * y;

                // ensure that each radius is only added once
                if distance_sqr >= min_radius_sqr
                    && !valid_sqr_distances.iter().contains(&distance_sqr)
                {
                    valid_sqr_distances.push(distance_sqr);
                }
            }
        }

        dbg!((&center, &max_offset, &min_radius_sqr, &valid_sqr_distances));

        valid_sqr_distances
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
