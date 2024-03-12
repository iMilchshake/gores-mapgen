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

#[derive(Debug, Clone)]
pub struct Kernel {
    pub size: usize,
    pub radius_sqr: usize,
    pub vector: Array2<bool>,
}

pub struct ValidKernelTable {
    pub all_valid_radii_sqr: Vec<usize>,
    pub valid_radii_per_size: HashMap<usize, Vec<usize>>,
    max_inner_radius_for_outer: HashMap<usize, usize>,
    max_outer_radius_for_inner: HashMap<usize, usize>,
}

impl ValidKernelTable {
    /// Precomputes all possible unique squared radii that can be used and their compatibility
    /// when used as inner and outer kernels. The algorithm that checks the validity is somewhat
    /// inefficient, so this function should be called once at startup, but not used at runtime.
    pub fn new(max_kernel_size: usize) -> ValidKernelTable {
        let all_valid_radii_sqr = ValidKernelTable::get_unique_radii_sqr(max_kernel_size, false);
        let mut max_inner_radius_for_outer: HashMap<usize, usize> = HashMap::new();
        let mut max_outer_radius_for_inner: HashMap<usize, usize> = HashMap::new();

        for outer_radius_index in 0..all_valid_radii_sqr.len() {
            let outer_radius = *all_valid_radii_sqr.get(outer_radius_index).unwrap();

            for inner_radius_index in (0..outer_radius_index).rev() {
                let inner_radius = *all_valid_radii_sqr.get(inner_radius_index).unwrap();

                // validate if inner radius is valid
                let inner_kernel = Kernel::new(max_kernel_size, inner_radius);
                let outer_kernel = Kernel::new(max_kernel_size, outer_radius);
                let kernel_valid = ValidKernelTable::check_kernels(&inner_kernel, &outer_kernel);

                // if it is optimal, store it as the upper bound and skip all possible smaller ones
                if kernel_valid {
                    println!("outer: {:} \t inner: {:}", outer_radius, inner_radius);
                    max_inner_radius_for_outer.insert(outer_radius, inner_radius); // always unique entry
                    max_outer_radius_for_inner.insert(inner_radius, outer_radius); // will override
                    break;
                }
            }
        }

        let mut valid_radii_per_size: HashMap<usize, Vec<usize>> = HashMap::new();
        for kernel_size in (1..max_kernel_size).step_by(2) {
            let valid_radii = ValidKernelTable::get_unique_radii_sqr(kernel_size, true);
            valid_radii_per_size.insert(kernel_size, valid_radii);
        }

        ValidKernelTable {
            all_valid_radii_sqr,
            max_inner_radius_for_outer,
            max_outer_radius_for_inner,
            valid_radii_per_size,
        }
    }

    pub fn get_max_valid_inner_radius(&self, outer_radius: &usize) -> usize {
        let max_radius = self
            .max_inner_radius_for_outer
            .get(outer_radius)
            .unwrap_or(&0);

        *max_radius
    }

    /// iterate over all possible distances from center to valid positions within the kernel bounds
    /// to get all possible squared radii. This returns a Vec of all possible squared radii that
    /// limit at least one possible location in the kernel, so each results in a unique kernel
    fn get_unique_radii_sqr(size: usize, check_min: bool) -> Vec<usize> {
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

        valid_sqr_distances.sort(); // validation algorithm expects that this is sorted
        valid_sqr_distances
    }

    fn check_kernels(inner_kernel: &Kernel, outer_kernel: &Kernel) -> bool {
        let max_offset = inner_kernel.max_offset();
        let inner_center = inner_kernel.center();
        let outer_center = outer_kernel.center();

        for x in (0..=max_offset).rev() {
            for y in (0..=max_offset).rev() {
                let inner_pos = (inner_center + x, inner_center + y);
                if inner_kernel.vector.get(inner_pos) != Some(&true) {
                    continue; // inner cell is not active, skip
                }

                // check adjacent neighboring cells in outer kernel
                for &offset in &[(1, 1), (0, 1), (0, 1)] {
                    let outer_pos = (outer_center + x + offset.0, outer_center + y + offset.1);
                    if outer_kernel.vector.get(outer_pos) != Some(&true) {
                        return false; // outer cell is not active -> INVALID
                    }
                }
            }
        }

        true
    }
}

impl Kernel {
    pub fn new(size: usize, radius_sqr: usize) -> Kernel {
        assert!(size % 2 == 1, "kernel size must be odd");
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

    fn center(&self) -> usize {
        (self.size - 1) / 2
    }

    fn max_offset(&self) -> usize {
        (self.size - 1) / 2
    }

    pub fn get_valid_radius_bounds(size: usize) -> (usize, usize) {
        let center = Kernel::get_kernel_center(size);
        let min_radius = ((size - 1) / 2).pow(2); // min radius is from center to border
        let max_radius = center * center + center * center; // max radius is from center to corner

        (min_radius, max_radius)
    }

    pub fn is_valid_radius(size: usize, radius_sqr: usize) -> bool {
        let (min_radius, max_radius) = Kernel::get_valid_radius_bounds(size);

        min_radius <= radius_sqr && radius_sqr <= max_radius
    }

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
