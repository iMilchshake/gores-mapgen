use crate::map::Map;
use macroquad::color::Color;
use ndarray::Array2;

pub struct DebugLayers {
    /// edge bugs that were fixed
    pub edge_bugs: DebugLayer,

    /// skips that were generated
    pub skips: DebugLayer,

    /// freeze skips that were generated
    pub freeze_skips: DebugLayer,

    /// skips that have been invalidated
    pub invalid_skips: DebugLayer,

    /// blobs that have been found
    pub blobs: DebugLayer,
}

/// Wrapper that stores all debug layers
impl DebugLayers {
    pub fn new(for_map: &Map, activate_all: bool, opacity: f32) -> DebugLayers {
        DebugLayers {
            edge_bugs: DebugLayer::new(
                for_map,
                true,
                Color::new(1.0, 0.8, 0.4, opacity),
                activate_all,
            ),
            freeze_skips: DebugLayer::new(
                for_map,
                true,
                Color::new(0.5, 0.85, 0.96, opacity),
                activate_all,
            ),
            skips: DebugLayer::new(
                for_map,
                true,
                Color::new(0.52, 0.85, 0.5, opacity),
                activate_all,
            ),
            invalid_skips: DebugLayer::new(
                for_map,
                true,
                Color::new(1.0, 0.4, 0.4, opacity),
                activate_all,
            ),
            blobs: DebugLayer::new(
                for_map,
                true,
                Color::new(0.65, 0.28, 0.85, opacity),
                activate_all,
            ),
        }
    }

    // TODO: this approach is kinda stupid
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&'static str, &mut DebugLayer)> {
        [
            ("edge_bugs", &mut self.edge_bugs),
            ("skips", &mut self.skips),
            ("freeze_skips", &mut self.freeze_skips),
            ("invalid_skips", &mut self.invalid_skips),
            ("blobs", &mut self.blobs),
        ]
        .into_iter()
    }
}

/// Allows storing various debug information
/// TODO: add support for continuous values
#[derive(Debug)]
pub struct DebugLayer {
    /// debugging values
    pub grid: Array2<bool>,

    /// should active blocks be visualized via an outline or filled?
    pub outline: bool,

    /// Color for visualization of active blocks
    pub color: Color,

    /// is this layer currently active?
    pub active: bool,
}

impl DebugLayer {
    pub fn new(for_map: &Map, outline: bool, color: Color, active: bool) -> Self {
        DebugLayer {
            grid: Array2::from_elem(for_map.grid.dim(), false),
            outline,
            color,
            active,
        }
    }
}
