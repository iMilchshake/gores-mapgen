use macroquad::color::Color;
use ndarray::Array2;

use std::collections::HashMap;

#[derive(Debug)]
pub struct FloatLayer {
    pub grid: Array2<f32>,
    color_min: Color,
    color_max: Color,
}

impl FloatLayer {
    pub fn new(shape: (usize, usize), color_min: Color, color_max: Color) -> FloatLayer {
        FloatLayer {
            grid: Array2::from_elem(shape, 0.0),
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
    pub active_layers: HashMap<&'static str, bool>,
    pub bool_layers: HashMap<&'static str, BoolLayer>,
    pub float_layers: HashMap<&'static str, FloatLayer>,
}

impl DebugLayers {
    pub fn new(enable_layers: bool, shape: (usize, usize), default_alpha: f32) -> DebugLayers {
        let bool_layers: HashMap<&'static str, BoolLayer> = HashMap::from([(
            "edge_bugs",
            BoolLayer::new(shape, Color::new(1.0, 0.8, 0.2, default_alpha), true),
        )]);

        let float_layers: HashMap<&'static str, FloatLayer> = HashMap::from([(
            "flood_fill",
            FloatLayer::new(
                shape,
                Color::new(1.0, 0.8, 0.2, default_alpha),
                Color::new(0.0, 0.8, 0.2, default_alpha),
            ),
        )]);

        let active_layers: HashMap<&'static str, bool> = bool_layers
            .keys()
            .chain(float_layers.keys())
            .map(|key| (*key, enable_layers))
            .collect();

        DebugLayers {
            active_layers,
            bool_layers,
            float_layers,
        }
    }
}

// let debug_layers = HashMap::from([
//     ("edge_bugs", DebugLayer::new(true, colors::BLUE, &map)),
//     ("freeze_skips", DebugLayer::new(true, colors::ORANGE, &map)),
//     ("skips", DebugLayer::new(true, colors::GREEN, &map)),
//     ("skips_invalid", DebugLayer::new(true, colors::RED, &map)),
//     ("blobs", DebugLayer::new(false, colors::RED, &map)),
//     (
//         "lock",
//         DebugLayer::new(false, Color::new(1.0, 0.2, 0.2, 0.3), &map),
//     ),
//     (
//         "platforms",
//         DebugLayer::new(false, Color::new(1.0, 0.0, 0.0, 0.1), &map),
//     ),
//     (
//         "platforms_pos",
//         DebugLayer::new(false, Color::new(0.0, 1.0, 0.0, 0.8), &map),
//     ),
//     (
//         "platforms_floor_pos",
//         DebugLayer::new(false, Color::new(0.0, 0.7, 0.7, 0.8), &map),
//     ),
//     (
//         "platforms_walker_pos",
//         DebugLayer::new(false, Color::new(0.7, 0.7, 0.0, 0.8), &map),
//     ),
// ]);
//
