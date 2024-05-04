use derivative::Derivative;
use ndarray::Array2;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Kernel {
    pub size: usize,
    pub circularity: f32,
    pub radius: f32,

    #[derivative(Debug = "ignore")]
    pub vector: Array2<bool>,
}

impl Kernel {
    pub fn new(size: usize, circularity: f32) -> Kernel {
        assert!(
            (0.0..=1.0).contains(&circularity),
            "circularity mut be in [0, 1]"
        );
        let radius = Kernel::circularity_to_radius(size, circularity);
        let vector = Kernel::get_kernel_vector(size, radius);

        Kernel {
            size,
            circularity,
            radius,
            vector,
        }
    }

    pub fn kernel_center(kernel_size: usize) -> f32 {
        (kernel_size - 1) as f32 / 2.0
    }

    pub fn circularity_to_radius(kernel_size: usize, circularity: f32) -> f32 {
        let (min_radius, max_radius) = Kernel::get_valid_radius_bounds(kernel_size);

        circularity * min_radius + (1.0 - circularity) * max_radius
    }

    pub fn get_valid_radius_bounds(size: usize) -> (f32, f32) {
        let center = Kernel::kernel_center(size);
        let min_radius = (size - 1) as f32 / 2.0; // min radius is from center to nearest border
        let max_radius = f32::sqrt(center * center + center * center); // max radius is from center to corner

        (min_radius, max_radius)
    }

    fn get_kernel_vector(size: usize, radius: f32) -> Array2<bool> {
        let center = Kernel::kernel_center(size);
        let mut kernel = Array2::from_elem((size, size), false);

        for ((x, y), value) in kernel.indexed_iter_mut() {
            let distance = f32::sqrt((x as f32 - center).powi(2) + (y as f32 - center).powi(2));
            if distance <= radius {
                *value = true;
            }
        }

        kernel
    }
}
