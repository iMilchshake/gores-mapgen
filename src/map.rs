use std::f64::consts::SQRT_2;



use crate::CuteWalker;
use crate::Position;
use egui::emath::Numeric;
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

#[derive(Debug)]
pub struct Kernel {
    pub size: usize,
    pub radius_sqr: usize,
    pub vector: Array2<bool>,
}

impl Kernel {
    pub fn new(size: usize, radius_sqr: usize) -> Kernel {
        let vector = Kernel::get_kernel_vector(size, radius_sqr);
        Kernel {
            size,
            radius_sqr,
            vector,
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
        

        min_radius <= radius_sqr && radius_sqr <= max_radius
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

    /// TODO: this could also be further optimized by using the kernels symmetry, but instead of
    /// optimizing this function it would make sense to replace the entire kernel
    fn get_kernel_vector(size: usize, radius_sqr: usize) -> Array2<bool> {
        let center = Kernel::get_kernel_center(size);

        let mut kernel = Array2::from_elem((size, size), false);
        for ((x, y), value) in kernel.indexed_iter_mut() {
            let distance = x.abs_diff(center).pow(2) + y.abs_diff(center).pow(2);
            if distance <= radius_sqr {
                *value = true;
            }
        }

        kernel
    }

    /// iterate over all possible distances from center to valid positions within the kernel bounds
    /// to get all possible squared radii. This returns a Vec of all possible squared radii that
    /// limit at least one possible location in the kernel, so each results in a unique kernel
    pub fn get_unique_radii_sqr(size: usize, check_min: bool) -> Vec<usize> {
        let mut valid_sqr_distances: Vec<usize> = Vec::new();
        let center = Kernel::get_kernel_center(size);
        let max_offset = size - center - 1;
        let min_radius_sqr = Kernel::get_valid_radius_bounds(size).0;

        for x in 0..=max_offset {
            // due to symmetry only look at values >= x
            for y in x..=max_offset {
                let distance_sqr = x * x + y * y;

                let min_check: bool = !check_min || distance_sqr >= min_radius_sqr;
                if min_check && !valid_sqr_distances.iter().contains(&distance_sqr) {
                    valid_sqr_distances.push(distance_sqr);
                }
            }
        }

        valid_sqr_distances.sort(); // TODO: do i need this?

        valid_sqr_distances
    }

    pub fn evaluate_kernels(max_kernel_size: usize) {
        let all_valid_radii_sqr = Kernel::get_unique_radii_sqr(max_kernel_size, false);

        // TODO: use two hashmaps to achieve bidirectional mapping, not sure if i actually need
        // this, but might come in handy
        let mut max_inner_radius_for_outer: HashMap<usize, usize> = HashMap::new();
        let mut max_outer_radius_for_inner: HashMap<usize, usize> = HashMap::new();

        for outer_radius_index in 0..all_valid_radii_sqr.len() {
            let outer_radius = *all_valid_radii_sqr.get(outer_radius_index).unwrap();

            for inner_radius_index in (0..outer_radius_index).rev() {
                let inner_radius = *all_valid_radii_sqr.get(inner_radius_index).unwrap();

                // validate if inner radius is valid TODO: replace this with an error free method!
                let factor: f64 = 2.0 * SQRT_2 * outer_radius.to_f64().sqrt();
                let kernel_is_valid = inner_radius.to_f64() <= outer_radius.to_f64() - factor + 2.0;

                if kernel_is_valid {
                    println!("outer: {:} \t inner: {:}", outer_radius, inner_radius);
                    max_inner_radius_for_outer.insert(outer_radius, inner_radius); // always unique entry
                    max_outer_radius_for_inner.insert(inner_radius, outer_radius); // will override
                    break;
                }
            }
        }

        dbg!(max_inner_radius_for_outer);
        dbg!(max_outer_radius_for_inner);
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
