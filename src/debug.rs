use std::collections::HashMap;

use macroquad::color::Color;
use ndarray::Array2;

#[derive(Debug)]
struct FloatLayer {
    grid: Array2<f32>,
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
struct BoolLayer {
    grid: Array2<bool>,
    color: Color,
    outline: bool,
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
    active_layers: HashMap<String, bool>,
    bool_layers: HashMap<String, BoolLayer>,
    float_layers: HashMap<String, FloatLayer>,
}

impl DebugLayers {
    pub fn new(enable_layers: bool, shape: (usize, usize), default_alpha: f32) -> DebugLayers {
        let bool_layers: HashMap<String, BoolLayer> = HashMap::from([(
            "edge_bugs".to_string(),
            BoolLayer::new(shape, Color::new(1.0, 0.8, 0.2, default_alpha), true),
        )]);

        let float_layers: HashMap<String, FloatLayer> = HashMap::from([(
            "flood_fill".to_string(),
            FloatLayer::new(
                shape,
                Color::new(1.0, 0.8, 0.2, default_alpha),
                Color::new(0.0, 0.8, 0.2, default_alpha),
            ),
        )]);

        let active_layers: HashMap<String, bool> = bool_layers
            .keys()
            .chain(float_layers.keys())
            .map(|key| (key.clone(), enable_layers))
            .collect();

        DebugLayers {
            active_layers,
            bool_layers,
            float_layers,
        }
    }
}
