use std::collections::HashMap;

use itertools::Itertools;
use ndarray::Array2;

#[derive(Debug, Clone)]
pub struct Kernel {
    pub size: usize,
    pub radius: usize,
    pub vector: Array2<bool>,
}

#[derive(Debug)]
pub struct ValidKernelTable {
    pub max_kernel_size: usize,
    pub all_valid_radii: Vec<usize>,
    valid_radii_per_size: HashMap<usize, Vec<usize>>,
    max_inner_radius_for_outer: HashMap<usize, usize>,
    min_outer_radius_for_inner: HashMap<usize, usize>,
}

impl ValidKernelTable {
    /// Precomputes all possible unique squared radii that can be used and their compatibility
    /// when used as inner and outer kernels. The algorithm that checks the validity is somewhat
    /// inefficient, so this function should be called once at startup, but not used at runtime.
    pub fn new(max_kernel_size: usize) -> ValidKernelTable {
        let all_valid_radii = ValidKernelTable::get_unique_radii(max_kernel_size, false);
        let mut max_inner_radius_for_outer: HashMap<usize, usize> = HashMap::new();
        let mut min_outer_radius_for_inner: HashMap<usize, usize> = HashMap::new();

        for outer_radius_index in 0..all_valid_radii.len() {
            let outer_radius = *all_valid_radii.get(outer_radius_index).unwrap();

            for inner_radius_index in (0..outer_radius_index).rev() {
                let inner_radius = *all_valid_radii.get(inner_radius_index).unwrap();

                // validate if inner radius is valid
                let inner_kernel = Kernel::new(max_kernel_size, inner_radius);
                let outer_kernel = Kernel::new(max_kernel_size, outer_radius);
                let kernel_valid = ValidKernelTable::check_kernels(&inner_kernel, &outer_kernel);

                // if it is optimal, store it as the upper bound and skip all possible smaller ones
                if kernel_valid {
                    // only store if not already a value present, so it remains the max value
                    max_inner_radius_for_outer
                        .entry(outer_radius)
                        .or_insert(inner_radius);

                    match min_outer_radius_for_inner.get(&inner_radius) {
                        None => {
                            // no value yet -> just save
                            min_outer_radius_for_inner.insert(inner_radius, outer_radius);
                        }
                        Some(min_outer_radius) => {
                            // current min radius is larger -> override with current outer_radius
                            if *min_outer_radius > outer_radius {
                                min_outer_radius_for_inner.insert(inner_radius, outer_radius);
                            }
                        }
                    }
                }
            }
        }

        let mut valid_radii_per_size: HashMap<usize, Vec<usize>> = HashMap::new();
        for kernel_size in 1..=max_kernel_size {
            let valid_radii = ValidKernelTable::get_unique_radii(kernel_size, true);
            valid_radii_per_size.insert(kernel_size, valid_radii);
        }

        ValidKernelTable {
            max_kernel_size,
            all_valid_radii,
            max_inner_radius_for_outer,
            min_outer_radius_for_inner,
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

    pub fn get_min_valid_outer_radius(&self, inner_radius: &usize) -> usize {
        let max_radius = self
            .min_outer_radius_for_inner
            .get(inner_radius)
            .expect("expect an entry for inner_radius");

        *max_radius
    }

    pub fn get_min_valid_outer_kernel(&self, inner_kernel: &Kernel) -> Kernel {
        let size = inner_kernel.size + 2;
        let radius = self.get_min_valid_outer_radius(&inner_kernel.radius);

        Kernel::new(size, radius)
    }

    /// Returns all valid radii for a given kernel size. Will return an empty Vec if no values have
    /// been precomputed for the kernel size
    /// TODO: this will result in an implicit copy each time - better use Option here!
    pub fn get_valid_radii(&self, size: &usize) -> Vec<usize> {
        self.valid_radii_per_size
            .get(size)
            .unwrap_or(&Vec::new())
            .to_vec()
    }

    /// iterate over all possible distances from center to valid positions within the kernel bounds
    /// to get all possible squared radii. This returns a Vec of all possible squared radii that
    /// limit at least one possible location in the kernel, so each results in a unique kernel
    fn get_unique_radii(size: usize, check_min: bool) -> Vec<usize> {
        let mut valid_distances: Vec<usize> = Vec::new();
        let center = Kernel::get_kernel_center(size);
        let max_offset = size - center - 1;
        let min_radius = Kernel::get_valid_radius_bounds(size).0;

        for x in 0..=max_offset {
            // due to symmetry only look at values >= x
            for y in x..=max_offset {
                let distance = x * x + y * y;
                let min_check: bool = !check_min || distance >= min_radius;
                if min_check && !valid_distances.iter().contains(&distance) {
                    valid_distances.push(distance);
                }
            }
        }

        valid_distances.sort(); // validation algorithm expects that this is sorted
        valid_distances
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
    pub fn new(size: usize, radius: usize) -> Kernel {
        let vector = Kernel::get_kernel_vector(size, radius);
        Kernel {
            size,
            radius,
            vector,
        }
    }

    fn get_kernel_center(size: usize) -> usize {
        (size - 1) / 2
    }

    pub fn center(&self) -> usize {
        (self.size - 1) / 2
    }

    pub fn max_offset(&self) -> usize {
        (self.size - 1) / 2
    }

    pub fn get_valid_radius_bounds(size: usize) -> (usize, usize) {
        let center = Kernel::get_kernel_center(size);
        let min_radius = ((size - 1) / 2).pow(2); // min radius is from center to border
        let max_radius = center * center + center * center; // max radius is from center to corner

        (min_radius, max_radius)
    }

    pub fn is_valid_radius(size: usize, radius: usize) -> bool {
        let (min_radius, max_radius) = Kernel::get_valid_radius_bounds(size);

        min_radius <= radius && radius <= max_radius
    }

    /// TODO: this could also be further optimized by using the kernels symmetry, but instead of
    /// optimizing this function it would make sense to replace the entire kernel
    fn get_kernel_vector(size: usize, radius: usize) -> Array2<bool> {
        let center = Kernel::get_kernel_center(size);

        let mut kernel = Array2::from_elem((size, size), false);
        for ((x, y), value) in kernel.indexed_iter_mut() {
            let distance = x.abs_diff(center).pow(2) + y.abs_diff(center).pow(2);
            if distance <= radius {
                *value = true;
            }
        }

        kernel
    }
}
