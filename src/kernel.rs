use std::collections::HashMap;

use egui::epaint::ahash::HashMap;
use itertools::Itertools;
use ndarray::Array2;

#[derive(Debug, Clone)]
pub struct Kernel {
    pub size: usize,
    pub radius: f32,
    pub vector: Array2<bool>,
}

#[derive(Debug)]
pub struct ValidKernelTable {
    /// maximum valid kernel size
    pub max_kernel_size: usize,

    /// maps kernel size to Vec of all possible kernels - index is refered to as kernel id
    /// therefore, kernel id is unique for each kernel size
    all_kernels: HashMap<usize, Vec<Kernel>>,

    /// maps inner kernel and outer size (inner size, inner kernel id, outer size) to min valid outer kernel id
    valid_outer: HashMap<(usize, usize, usize), usize>,

    /// maps outer kernel and inner size (outer size, outer kernel id, inner size) to min valid outer kernel id
    valid_inner: HashMap<(usize, usize, usize), usize>,
}

impl ValidKernelTable {
    pub fn new(max_kernel_size: usize) -> ValidKernelTable {
        // step 1: generate all kernels
        let mut all_kernels = HashMap::new();
        for kernel_size in 1..=max_kernel_size {
            let mut kernels: Vec<Kernel> = Vec::new();
            for radius in ValidKernelTable::get_unique_radii(kernel_size, true) {
                kernels.push(Kernel::new(kernel_size, radius));
            }
            all_kernels.insert(kernel_size, kernels);
        }

        // step 2: check valid configurations
        let mut valid_outer = HashMap::new();
        let mut valid_inner = HashMap::new();
        for inner_size in 1..=max_kernel_size - 2 {
            for (inner_id, inner_kernel) in all_kernels.get(&inner_size).unwrap().iter().enumerate()
            {
                for outer_size in inner_size + 2..=max_kernel_size {
                    for (outer_id, outer_kernel) in
                        all_kernels.get(&outer_size).unwrap().iter().enumerate()
                    {
                        let kernel_valid =
                            ValidKernelTable::check_kernels(&inner_kernel, &outer_kernel);

                        if kernel_valid {
                            // only store if not already a value present
                            if valid_outer.get(&(inner_size, inner_id, outer_size)) == None {
                                valid_outer.insert((inner_size, inner_id, outer_size), outer_id);
                            }

                            match valid_inner.get(&(outer_size, outer_id, inner_size)) {
                                None => {
                                    // no value yet -> just save
                                    valid_inner
                                        .insert((outer_size, outer_id, inner_size), inner_id);
                                }
                                Some(min_inner_id) => {
                                    // current min radius is larger -> override with current outer_radius
                                    if *min_inner_id > outer_id {
                                        valid_inner
                                            .insert((outer_size, outer_id, inner_size), inner_id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        ValidKernelTable {
            all_kernels,
            max_kernel_size,
            valid_outer,
            valid_inner,
        }
    }

    /// iterate over all possible distances from center to valid positions within the kernel bounds
    /// to get all possible squared radii. This returns a Vec of all possible squared radii that
    /// limit at least one possible location in the kernel, so each results in a unique kernel
    fn get_unique_radii(size: usize, check_min: bool) -> Vec<f32> {
        let mut valid_radii: Vec<f32> = Vec::new();
        let center = Kernel::get_kernel_center(size);
        let max_offset = size - (center as usize) - 1;
        let min_radius = Kernel::get_valid_radius_bounds(size).0;

        let is_even = size % 2 == 0;

        for x in 0..=max_offset {
            // due to symmetry only look at values >= x
            for y in x..=max_offset {
                let distance = match is_even {
                    false => (x * x + y * y) as f32,
                    true => {
                        let _x = (x as f32) - 0.5;
                        let _y = (y as f32) - 0.5;
                        _x * _x + _y * _y
                    }
                };
                let min_check: bool = !check_min || distance >= min_radius;
                if min_check && !valid_radii.iter().contains(&distance) {
                    valid_radii.push(distance);
                }
            }
        }

        valid_radii.sort_by(|r1, r2| r1.total_cmp(&r2)); // validation algorithm expects that this is sorted
        valid_radii
    }

    fn check_kernels(inner_kernel: &Kernel, outer_kernel: &Kernel) -> bool {
        assert!(
            outer_kernel.size - 2 >= inner_kernel.size,
            "max_outer_size needs to be +2 of max_inner_size"
        );

        let outer_margin = (inner_kernel.size - outer_kernel.size) / 2; // top-left margin

        for x in (0..=inner_kernel.size).rev() {
            for y in (0..=inner_kernel.size).rev() {
                if inner_kernel.vector.get((x, y)) != Some(&true) {
                    continue; // inner cell is not active, skip
                }

                // check adjacent neighboring cells in outer kernel
                // TODO: replace by ndaray/slice/stencil any check
                for &offset in &[(1, 1), (0, 1), (0, 1)] {
                    let outer_pos = (x + outer_margin, y + outer_margin);

                    let outer_pos_off1 = (x + offset.0, y + offset.1); // bot-right
                    let outer_pos_off2 = (x - offset.0, y - offset.1); //top-left
                    if outer_kernel.vector.get(outer_pos_off1) != Some(&true)
                        || outer_kernel.vector.get(outer_pos_off2) != Some(&true)
                    {
                        return false; // outer cell is not active -> INVALID
                    }
                }
            }
        }

        true
    }
}

impl Kernel {
    pub fn new(size: usize, radius: f32) -> Kernel {
        let vector = Kernel::get_kernel_vector(size, radius);
        Kernel {
            size,
            radius,
            vector,
        }
    }

    fn get_kernel_center(size: usize) -> f32 {
        (size - 1) as f32 / 2.0
    }

    pub fn center(&self) -> f32 {
        (self.size - 1) as f32 / 2.0
    }

    pub fn get_valid_radius_bounds(size: usize) -> (f32, f32) {
        let center = Kernel::get_kernel_center(size);
        let min_radius = ((size - 1) as f32 / 2.0).powi(2); // min radius is from center to border
        let max_radius = center * center + center * center; // max radius is from center to corner

        (min_radius, max_radius)
    }

    pub fn is_valid_radius(size: usize, radius: f32) -> bool {
        let (min_radius, max_radius) = Kernel::get_valid_radius_bounds(size);

        min_radius <= radius && radius <= max_radius
    }

    /// for odd kernel sizes the center cannot be represented using an integer. This function
    /// should be able to calculate the correct kernel vectors for even and odd sized kernels
    pub fn get_kernel_vector(size: usize, radius: f32) -> Array2<bool> {
        let center = ((size as f32) - 1.0) / 2.0;

        let mut kernel = Array2::from_elem((size, size), false);
        for ((x, y), value) in kernel.indexed_iter_mut() {
            let distance = ((x as f32) - center).powi(2) + ((y as f32) - center).powi(2);
            if distance <= radius {
                *value = true;
            }
        }

        kernel
    }
}
