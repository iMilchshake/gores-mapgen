use ndarray::prelude::*;
use ndarray::Array2;
use noise::{
    utils::{NoiseMapBuilder, PlaneMapBuilder},
    Fbm, Perlin, Worley,
};

use crate::map::Map;

#[derive(Clone, Copy)]
pub enum Noise {
    Perlin,
    Worley,
}

#[allow(clippy::too_many_arguments)]
pub fn generate_noise_array(
    map: &Map,
    noise_scale: f32,
    noise_invert: bool,
    noise_threshold: f32,
    noise_type: Noise,
    only_solid_overlay: bool,
    add_solid_background: bool,
    seed: u32,
) -> Array2<bool> {
    let aspect_ratio = map.width as f64 / map.height as f64;
    let noise_scale_x = noise_scale as f64;
    let noise_scale_y = noise_scale as f64 / aspect_ratio;

    // might seem verbose, but will be more flexible in the future,
    // and doesnt need any type fuckery :)
    let noise_map = match noise_type {
        Noise::Worley => PlaneMapBuilder::new(Fbm::<Worley>::new(seed))
            .set_size(map.width, map.height)
            .set_x_bounds(0., noise_scale_x)
            .set_y_bounds(0., noise_scale_y)
            .build(),
        Noise::Perlin => PlaneMapBuilder::new(Fbm::<Perlin>::new(seed))
            .set_size(map.width, map.height)
            .set_x_bounds(0., noise_scale_x)
            .set_y_bounds(0., noise_scale_y)
            .build(),
    };

    Array2::from_shape_fn((map.width, map.height), |(x, y)| {
        let noise_value = noise_map.get_value(x, y);
        let noise_active = (noise_value > noise_threshold as f64) ^ noise_invert;
        let is_solid = map.grid[(x, y)].is_solid();
        // if enabled, block MUST be solid for active output
        let valid_overlay = !only_solid_overlay || is_solid;
        // if enabled, output will be active for every solid block
        let valid_background = add_solid_background && is_solid;
        let at_border = x == 0 || y == 0 || x == map.width - 1 || y == map.height - 1;

        (noise_active || valid_background) && !at_border && valid_overlay
    })
}

pub fn dilate(input: &Array2<bool>) -> Array2<bool> {
    let mut result = Array2::from_elem(input.dim(), false);

    for y in 1..input.nrows() - 1 {
        for x in 1..input.ncols() - 1 {
            let window = input.slice(s![y - 1..=y + 1, x - 1..=x + 1]);
            result[[y, x]] = window.iter().any(|&val| val);
        }
    }

    result
}

pub fn erode(input: &Array2<bool>) -> Array2<bool> {
    let mut result = Array2::from_elem(input.dim(), true);

    for y in 1..input.nrows() - 1 {
        for x in 1..input.ncols() - 1 {
            let window = input.slice(s![y - 1..=y + 1, x - 1..=x + 1]);
            result[[y, x]] = window.iter().all(|&val| val);
        }
    }

    result
}

pub fn opening(input: &Array2<bool>) -> Array2<bool> {
    let eroded = erode(input);
    dilate(&eroded)
}

pub fn closing(input: &Array2<bool>) -> Array2<bool> {
    let dilated = dilate(input);
    erode(&dilated)
}
