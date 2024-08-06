use crate::map::Map;
use macroquad::color::Color;
use ndarray::Array2;

#[derive(Hash, Eq, PartialEq)]
pub enum DebugLayerType {
    EdgeBugs,
    FreezeSkips,
    Skips,
    SkipsInvalid,
    Blobs,
    Lock,
}

/// storage for all debug layers
pub struct DebugLayers {
    layers: [DebugLayer; 6],
}

impl DebugLayers {
    pub fn new(alpha: f32, for_map: &Map) -> DebugLayers {
        DebugLayers {
            layers: [
                // EdgeBugs
                DebugLayer::new_bool(false, false, Color::new(1.0, 0.2, 0.2, alpha), for_map),
                // FreezeSkips
                DebugLayer::new_bool(false, false, Color::new(1.0, 0.2, 0.2, alpha), for_map),
                // Skips
                DebugLayer::new_bool(false, false, Color::new(1.0, 0.2, 0.2, alpha), for_map),
                // SkipsInvalid
                DebugLayer::new_bool(false, false, Color::new(1.0, 0.2, 0.2, alpha), for_map),
                // Blobs
                DebugLayer::new_bool(false, false, Color::new(1.0, 0.2, 0.2, alpha), for_map),
                // Lock
                DebugLayer::new_bool(false, false, Color::new(1.0, 0.2, 0.2, alpha), for_map),
            ],
        }
    }

    pub fn get(&mut self, layer_type: DebugLayerType) -> &mut DebugLayer {
        &mut self.layers[layer_type as usize] // use int representation of enum for quick access
    }
}

// I tried using generics for this, but that makes it infeasible to store all in one datastructure.
// I guess using an enum for this is okay. Lot of code repetition, but allows for different
// configurations which is neat.
pub enum DebugLayer {
    Bool {
        grid: Array2<bool>,
        outline: bool,
        color: Color,
    },
    F32 {
        grid: Array2<f32>,
        outline: bool,
        color_min: Color,
        color_max: Color,
    },
    USize {
        grid: Array2<usize>,
        outline: bool,
        color_min: Color,
        color_max: Color,
    },
}

impl DebugLayer {
    pub fn new_bool(default_value: bool, outline: bool, color: Color, for_map: &Map) -> Self {
        DebugLayer::Bool {
            grid: Array2::from_elem(for_map.grid.dim(), default_value),
            outline,
            color,
        }
    }

    pub fn new_f32(
        default_value: f32,
        outline: bool,
        color_min: Color,
        color_max: Color,
        for_map: &Map,
    ) -> Self {
        DebugLayer::F32 {
            grid: Array2::from_elem(for_map.grid.dim(), default_value),
            outline,
            color_min,
            color_max,
        }
    }

    pub fn new_usize(
        default_value: usize,
        outline: bool,
        color_min: Color,
        color_max: Color,
        for_map: &Map,
    ) -> Self {
        DebugLayer::USize {
            grid: Array2::from_elem(for_map.grid.dim(), default_value),
            outline,
            color_min,
            color_max,
        }
    }
}
