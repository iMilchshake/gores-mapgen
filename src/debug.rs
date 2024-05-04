use crate::map::Map;
use macroquad::color::Color;
use ndarray::{s, Array2, ShapeBuilder};

/// Allows storing various debug information
#[derive(Debug)]
pub struct DebugLayer {
    pub grid: Array2<bool>,

    /// should active blocks be visualized via an outline or filled?
    pub outline: bool,

    /// Color for visualization of active blocks
    pub color: Color,
}

impl DebugLayer {
    pub fn new(outline: bool, color: Color, for_map: &Map) -> Self {
        DebugLayer {
            grid: Array2::from_elem(for_map.grid.dim(), false),
            outline,
            color,
        }
    }
}
