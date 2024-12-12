use noise::{
    utils::{NoiseFnWrapper, NoiseMapBuilder, PlaneMapBuilder},
    Fbm, Perlin, Worley,
};

use ndarray::Array2;

use crate::map::Map;

#[derive(Clone, Copy)]
pub enum Noise {
    Perlin,
    Worley,
}

pub fn generate_noise_array(
    map: &Map,
    noise_scale: f32,
    noise_invert: bool,
    noise_threshold: f32,
    noise_type: Noise,
    only_solid_overlay: bool,
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

    let noise_bool_array = Array2::from_shape_fn((map.width, map.height), |(x, y)| {
        let noise_value = noise_map.get_value(x, y);
        let noise_active = (noise_value > noise_threshold as f64) ^ noise_invert;
        let valid_overlay = !only_solid_overlay || map.grid[(x, y)].is_solid();
        let at_border = x == 0 || y == 0 || x == map.width - 1 || y == map.height - 1;

        noise_active && !at_border && valid_overlay
    });

    noise_bool_array
}
