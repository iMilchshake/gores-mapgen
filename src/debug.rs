use macroquad::color::Color;
use ndarray::Array2;

use std::collections::BTreeMap;

#[derive(Debug)]
pub struct FloatLayer {
    pub grid: Array2<Option<f32>>,
    pub color_min: Color,
    pub color_max: Color,
}

impl FloatLayer {
    pub fn new(shape: (usize, usize), color_min: Color, color_max: Color) -> FloatLayer {
        FloatLayer {
            grid: Array2::from_elem(shape, None),
            color_min,
            color_max,
        }
    }
}

#[derive(Debug)]
pub struct BoolLayer {
    pub grid: Array2<bool>,
    pub color: Color,
    pub outline: bool,
}

impl BoolLayer {
    pub fn new(shape: (usize, usize), color: Color, outline: bool) -> BoolLayer {
        BoolLayer {
            grid: Array2::from_elem(shape, false),
            color,
            outline,
        }
    }
}

pub struct DebugLayers {
    pub active_layers: BTreeMap<&'static str, bool>,
    pub bool_layers: BTreeMap<&'static str, BoolLayer>,
    pub float_layers: BTreeMap<&'static str, FloatLayer>,
}

impl DebugLayers {
    pub fn new(
        shape: (usize, usize),
        default_alpha: f32,
        previous_active_layers: Option<BTreeMap<&'static str, bool>>,
    ) -> DebugLayers {
        let bool_layers: BTreeMap<&'static str, BoolLayer> = BTreeMap::from([
            (
                "edge_bugs",
                BoolLayer::new(shape, Color::new(0.76, 0.22, 0.39, default_alpha), true),
            ),
            (
                "blobs",
                BoolLayer::new(shape, Color::new(0.9, 0.36, 0.31, default_alpha), true),
            ),
            (
                "platforms",
                BoolLayer::new(shape, Color::new(0.8, 0.81, 0.52, default_alpha), true),
            ),
            (
                "skips",
                BoolLayer::new(shape, Color::new(0.62, 0.83, 0.4, default_alpha), true),
            ),
            (
                "skips_invalid",
                BoolLayer::new(shape, Color::new(1.0, 0.61, 0.38, default_alpha), true),
            ),
            (
                "freeze_skips",
                BoolLayer::new(shape, Color::new(0.45, 0.53, 0.77, default_alpha), true),
            ),
            (
                "lock",
                BoolLayer::new(shape, Color::new(0.43, 0.28, 0.62, default_alpha), false),
            ),
            (
                "waypoint_lock",
                BoolLayer::new(shape, Color::new(0.53, 0.18, 0.52, default_alpha), false),
            ),
            (
                "noise_b",
                BoolLayer::new(shape, Color::new(0.96, 0.18, 0.39, default_alpha), false),
            ),
            (
                "noise_o",
                BoolLayer::new(shape, Color::new(0.96, 0.38, 0.19, default_alpha), false),
            ),
            (
                "a_star",
                BoolLayer::new(shape, Color::new(0.76, 0.26, 0.33, default_alpha), false),
            ),
            (
                "path",
                BoolLayer::new(shape, Color::new(0.86, 0.16, 0.33, default_alpha), false),
            ),
            (
                "floor",
                BoolLayer::new(shape, Color::new(0.96, 0.16, 0.18, default_alpha), false),
            ),
            (
                "plat",
                BoolLayer::new(shape, Color::new(0.36, 0.56, 0.18, default_alpha), false),
            ),
            (
                "selected_plat",
                BoolLayer::new(shape, Color::new(0.76, 0.16, 0.88, default_alpha), false),
            ),
        ]);

        let float_layers: BTreeMap<&'static str, FloatLayer> = BTreeMap::from([
            (
                "flood_fill",
                FloatLayer::new(
                    shape,
                    Color::new(0.0, 1.0, 0.0, default_alpha),
                    Color::new(0.0, 0.0, 1.0, default_alpha),
                ),
            ),
            (
                "main_path_dist",
                FloatLayer::new(
                    shape,
                    Color::new(0.0, 1.0, 0.0, default_alpha),
                    Color::new(0.0, 0.0, 1.0, default_alpha),
                ),
            ),
            (
                "dt",
                FloatLayer::new(
                    shape,
                    Color::new(0.0, 1.0, 0.0, default_alpha),
                    Color::new(0.0, 0.0, 1.0, default_alpha),
                ),
            ),
            (
                "plat_cand",
                FloatLayer::new(
                    shape,
                    Color::new(0.0, 1.0, 0.0, default_alpha),
                    Color::new(0.0, 0.0, 1.0, default_alpha),
                ),
            ),
            (
                "plat_group",
                FloatLayer::new(
                    shape,
                    Color::new(0.0, 1.0, 0.0, default_alpha),
                    Color::new(0.0, 0.0, 1.0, default_alpha),
                ),
            ),
        ]);

        // initialize using keys from all debug layers, or re-use if possible
        let active_layers = previous_active_layers.unwrap_or_else(|| {
            bool_layers
                .keys()
                .chain(float_layers.keys())
                .map(|key| (*key, false))
                .collect()
        });

        DebugLayers {
            active_layers,
            bool_layers,
            float_layers,
        }
    }
}
