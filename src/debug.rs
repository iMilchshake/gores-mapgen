use crate::map::Map;
use macroquad::color::Color;
use ndarray::Array2;

pub struct DebugLayers {
    /// edge bugs that were fixed
    pub edge_bugs: DebugLayerBase,

    /// skips that were generated
    pub skips: DebugLayerBase,

    /// freeze skips that were generated
    pub freeze_skips: DebugLayerBase,

    /// skips that have been invalidated
    pub invalid_skips: DebugLayerBase,

    /// blobs that have been found
    pub blobs: DebugLayerBase,
}

/// Wrapper that stores all debug layers
impl DebugLayers {
    pub fn new(for_map: &Map, activate_all: bool, opacity: f32) -> DebugLayers {
        let shape = (for_map.width, for_map.height);

        DebugLayers {
            edge_bugs: DebugLayerBase::new_bool_layer(
                shape,
                activate_all,
                true,
                Color::new(1.0, 0.8, 0.4, opacity),
            ),
            freeze_skips: DebugLayerBase::new_bool_layer(
                shape,
                activate_all,
                true,
                Color::new(0.5, 0.85, 0.96, opacity),
            ),
            skips: DebugLayerBase::new_bool_layer(
                shape,
                activate_all,
                true,
                Color::new(0.52, 0.85, 0.5, opacity),
            ),
            invalid_skips: DebugLayerBase::new_bool_layer(
                shape,
                activate_all,
                true,
                Color::new(1.0, 0.4, 0.4, opacity),
            ),
            blobs: DebugLayerBase::new_bool_layer(
                shape,
                activate_all,
                true,
                Color::new(0.65, 0.28, 0.85, opacity),
            ),
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&'static str, &mut DebugLayerBase)> {
        std::iter::once(("edge_bugs", &mut self.edge_bugs))
            .chain(std::iter::once(("skips", &mut self.skips)))
            .chain(std::iter::once(("freeze_skips", &mut self.freeze_skips)))
            .chain(std::iter::once(("invalid_skips", &mut self.invalid_skips)))
            .chain(std::iter::once(("blobs", &mut self.blobs)))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &DebugLayerBase)> {
        std::iter::once(("edge_bugs", &self.edge_bugs))
            .chain(std::iter::once(("skips", &self.skips)))
            .chain(std::iter::once(("freeze_skips", &self.freeze_skips)))
            .chain(std::iter::once(("invalid_skips", &self.invalid_skips)))
            .chain(std::iter::once(("blobs", &self.blobs)))
    }
}

#[derive(Debug)]
pub enum DebugLayer {
    BoolLayer {
        /// debugging values
        grid: Array2<bool>,

        /// Color for visualization of active blocks
        color: Color,
    },
    FloatLayer {
        /// debugging values
        grid: Array2<f32>,

        /// Minimum color for visualization
        min_color: Color,

        /// Maximum color for visualization
        max_color: Color,
    },
}

#[derive(Debug)]
pub struct DebugLayerBase {
    /// is this layer currently active?
    pub active: bool,

    /// should active blocks be visualized via an outline or filled?
    pub outline: bool,

    /// debug layer
    pub debug_layer: DebugLayer,
}

impl DebugLayerBase {
    pub fn new_bool_layer(
        shape: (usize, usize),
        active: bool,
        outline: bool,
        color: Color,
    ) -> Self {
        Self {
            active,
            outline,
            debug_layer: DebugLayer::BoolLayer {
                grid: Array2::from_elem(shape, false),
                color,
            },
        }
    }

    pub fn new_float_layer(
        shape: (usize, usize),
        active: bool,
        outline: bool,
        min_color: Color,
        max_color: Color,
    ) -> Self {
        Self {
            active,
            outline,
            debug_layer: DebugLayer::FloatLayer {
                grid: Array2::from_elem(shape, 0.0),
                min_color,
                max_color,
            },
        }
    }
}
