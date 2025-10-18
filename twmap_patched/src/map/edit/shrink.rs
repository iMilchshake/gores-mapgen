use crate::map::PARALLAX_DIVISOR;
use crate::*;

use fixed::types::I17F15;
use ndarray::{s, Array2};
use vek::num_traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One};
use vek::{Extent2, Vec2};

use std::cmp::min;

/// up - down - left - right
pub fn shrink_ndarray<T: Default + Clone>(
    ndarray: &Array2<T>,
    up_left: Extent2<usize>,
    down_right: Extent2<usize>,
) -> Option<Array2<T>> {
    let bottom_right = Extent2::new(ndarray.ncols(), ndarray.nrows()).checked_sub(&down_right)?;
    Some(
        ndarray
            .slice(s![up_left.h..bottom_right.h, up_left.w..bottom_right.w])
            .to_owned(),
    )
}

impl TwMap {
    /// Downsizes all tilemap layers as much as possible.
    /// Note that this will also reduce the size of physics layers and thereby moves the world border!
    /// Downsizing is possible as long as any outer edge is the same as the it's neighboring row/column.
    /// Because of this, it is suggested to use [`TwMap::edit_tiles`](TwMap::edit_tiles) with [`ZeroAir`](super::ZeroAir) before using this method.
    ///
    /// Returns None if an overflow occurred.
    #[must_use]
    pub fn lossless_shrink_layers(mut self) -> Option<TwMap> {
        self.groups = self
            .groups
            .into_iter()
            .map(|group| group.lossless_layer_shrink())
            .collect::<Option<_>>()?;
        // when the physics group shrinks from the left or up, it gets offset, so we must correct that
        let physics_group = self.physics_group();
        let offset_shift = physics_group.offset;
        for group in &mut self.groups {
            group.offset = group.offset.checked_sub(
                &offset_shift
                    .checked_mul(&group.parallax.checked_as()?)?
                    .checked_div(&PARALLAX_DIVISOR.checked_as()?)?,
            )?;
            if !group.is_physics_group() {
                group
                    .clip
                    .set_position(group.clip.position().checked_add(&offset_shift)?);
            }
        }
        Some(self)
    }

    /// Downsizes all tiles layers as much as possible.
    /// Downsizing is possible as long as any outer edge is the same as the it's neighboring row/column.
    /// Because of this, it is suggested to use [`TwMap::edit_tiles`](TwMap::edit_tiles) with [`ZeroAir`](super::ZeroAir) before using this method.

    /// Returns None if an overflow occurred.
    #[must_use]
    pub fn lossless_shrink_tiles_layers(mut self) -> Option<TwMap> {
        self.groups = self
            .groups
            .into_iter()
            .map(|group| group.lossless_tiles_layer_shrink())
            .collect::<Option<_>>()?;
        Some(self)
    }
}

impl Group {
    fn lossless_tiles_layer_shrink(mut self) -> Option<Group> {
        if self
            .layers
            .iter()
            .all(|layer| layer.kind() != LayerKind::Tiles)
        {
            return Some(self); // no tiles layer -> nothing to shrink
        }
        let shrink_distances: Vec<_> = self
            .layers
            .iter()
            .map(|layer| layer.lossless_shrink_distances())
            .collect();

        let smallest_layer_height = self
            .layers
            .iter()
            .filter_map(|layer| layer.shape().map(|shape| shape.h))
            .min()
            .unwrap();
        let smallest_layer_width = self
            .layers
            .iter()
            .filter_map(|layer| layer.shape().map(|shape| shape.w))
            .min()
            .unwrap();
        let mut lowest_up_shrink = 0;
        let mut lowest_left_shrink = 0;
        if !self.is_physics_group() {
            // all layers in a group must shrink by the same amount from the left and up, since you can't offset single layers
            lowest_up_shrink = shrink_distances
                .iter()
                .filter_map(|&d| d.map(|dirs| dirs.0.h))
                .min()
                .unwrap();
            lowest_left_shrink = shrink_distances
                .iter()
                .filter_map(|&d| d.map(|dirs| dirs.0.w))
                .min()
                .unwrap();
            // ensure that the layers are at least 2 blocks high and wide
            lowest_up_shrink = min(lowest_up_shrink, smallest_layer_height.checked_sub(2)?);
            lowest_left_shrink = min(lowest_left_shrink, smallest_layer_width.checked_sub(2)?);
        }
        self.layers = self
            .layers
            .into_iter()
            .zip(shrink_distances)
            .map(|(layer, dirs)| {
                if layer.kind().is_physics_layer() {
                    Some(layer)
                } else {
                    let mut down = dirs.map(|d| d.1.h).unwrap_or(0);
                    let mut right = dirs.map(|d| d.1.w).unwrap_or(0);
                    // ensure that the layers are at least 2 blocks high and wide
                    if let Some(size) = layer.shape() {
                        down = min(down, size.h.checked_sub(lowest_up_shrink + 2)?);
                        right = min(right, size.w.checked_sub(lowest_left_shrink + 2)?);
                    }
                    let up_left = Extent2::new(lowest_left_shrink, lowest_up_shrink);
                    let down_right = Extent2::new(right, down);
                    layer.shrink(up_left, down_right)
                }
            })
            .collect::<Option<_>>()?;

        // shrinking the layers from up and left moves them, so the offset has to correct that
        let offset_shrink = Vec2::new(lowest_left_shrink, lowest_up_shrink);
        self.offset = self.offset.checked_sub(&offset_shrink.checked_as()?)?;
        Some(self)
    }

    // TODO: combine this function with the previous one
    fn lossless_layer_shrink(mut self) -> Option<Group> {
        if self
            .layers
            .iter()
            .all(|layer| !layer.kind().is_tile_map_layer())
        {
            return Some(self); // no tilemap layer -> nothing to shrink
        }
        let shrink_distances: Vec<_> = self
            .layers
            .iter()
            .map(|layer| layer.lossless_shrink_distances())
            .collect();

        let smallest_layer_height = self
            .layers
            .iter()
            .filter_map(|layer| layer.shape().map(|shape| shape.h))
            .min()
            .unwrap();
        let smallest_layer_width = self
            .layers
            .iter()
            .filter_map(|layer| layer.shape().map(|shape| shape.w))
            .min()
            .unwrap();
        // all layers in a group must shrink by the same amount from the left and up, since you can't offset single layers
        let mut lowest_up_shrink = shrink_distances
            .iter()
            .filter_map(|&d| d.map(|dirs| dirs.0.h))
            .min()
            .unwrap();
        let mut lowest_left_shrink = shrink_distances
            .iter()
            .filter_map(|&d| d.map(|dirs| dirs.0.w))
            .min()
            .unwrap();
        // ensure that the layers are at least 2 blocks high and wide
        lowest_up_shrink = min(lowest_up_shrink, smallest_layer_height.checked_sub(2)?);
        lowest_left_shrink = min(lowest_left_shrink, smallest_layer_width.checked_sub(2)?);
        // all physics layers must shrink by the same amount from the right and down, since they must be the same size
        let physics_layer_down_shrink = self
            .layers
            .iter()
            .filter_map(|l| {
                if l.kind().is_physics_layer() {
                    l.lossless_shrink_distances()
                } else {
                    None
                }
            })
            .map(|d| d.1.h)
            .min();
        let physics_layer_right_shrink = self
            .layers
            .iter()
            .filter_map(|l| {
                if l.kind().is_physics_layer() {
                    l.lossless_shrink_distances()
                } else {
                    None
                }
            })
            .map(|d| d.1.w)
            .min();
        self.layers = self
            .layers
            .into_iter()
            .zip(shrink_distances)
            .map(|(layer, dirs)| {
                let (mut down, mut right) = match layer.kind().is_physics_layer() {
                    true => (
                        physics_layer_down_shrink.unwrap(),
                        physics_layer_right_shrink.unwrap(),
                    ),
                    false => (
                        dirs.map(|d| d.1.h).unwrap_or(0),
                        dirs.map(|d| d.1.w).unwrap_or(0),
                    ),
                };
                // ensure that the layers are at least 2 blocks high and wide
                if let Some(size) = layer.shape() {
                    down = min(down, size.h.checked_sub(lowest_up_shrink + 2)?);
                    right = min(right, size.w.checked_sub(lowest_left_shrink + 2)?)
                }
                let up_left = Extent2::new(lowest_left_shrink, lowest_up_shrink);
                let down_right = Extent2::new(right, down);
                layer.shrink(up_left, down_right)
            })
            .collect::<Option<_>>()?;

        // shrinking the layers from up and left moves them, so the offset has to correct that
        let offset_shrink = Vec2::new(lowest_left_shrink, lowest_up_shrink);
        self.offset = self.offset.checked_sub(&offset_shrink.checked_as()?)?;
        Some(self)
    }
}

impl Layer {
    fn shrink(self, up_left: Extent2<usize>, down_right: Extent2<usize>) -> Option<Layer> {
        use Layer::*;
        let mut offset: Vec2<I17F15> = Vec2::from(up_left.checked_as()?);
        offset = offset.checked_mul(&Vec2::broadcast(-I17F15::one()))?;
        Some(match self {
            Game(l) => Game(checked_shrink(l, up_left, down_right)?),
            Tiles(l) => Tiles(checked_shrink(l, up_left, down_right)?),
            Quads(mut l) => {
                l.shift(offset)?;
                Quads(l)
            }
            Front(l) => Front(checked_shrink(l, up_left, down_right)?),
            Tele(l) => Tele(checked_shrink(l, up_left, down_right)?),
            Speedup(l) => Speedup(checked_shrink(l, up_left, down_right)?),
            Switch(l) => Switch(checked_shrink(l, up_left, down_right)?),
            Tune(l) => Tune(checked_shrink(l, up_left, down_right)?),
            Sounds(mut l) => {
                l.shift(offset)?;
                Sounds(l)
            }
            Invalid(l) => Invalid(l),
        })
    }

    fn lossless_shrink_distances(&self) -> Option<(Extent2<usize>, Extent2<usize>)> {
        use Layer::*;
        match self {
            Game(l) => Some(lossless_shrink_distances(l.tiles.unwrap_ref())),
            Tiles(l) => Some(lossless_shrink_distances(l.tiles.unwrap_ref())),
            Quads(_) => None,
            Front(l) => Some(lossless_shrink_distances(l.tiles.unwrap_ref())),
            Tele(l) => Some(lossless_shrink_distances(l.tiles.unwrap_ref())),
            Speedup(l) => Some(lossless_shrink_distances(l.tiles.unwrap_ref())),
            Switch(l) => Some(lossless_shrink_distances(l.tiles.unwrap_ref())),
            Tune(l) => Some(lossless_shrink_distances(l.tiles.unwrap_ref())),
            Sounds(_) => None,
            Invalid(_) => None,
        }
    }
}

fn checked_shrink<T: TilemapLayer>(
    mut layer: T,
    up_left: Extent2<usize>,
    down_right: Extent2<usize>,
) -> Option<T> {
    *layer.tiles_mut().unwrap_mut() =
        shrink_ndarray(layer.tiles().unwrap_ref(), up_left, down_right)?;
    Some(layer)
}

/// For every direction, this function will return the amount of rows/columns that are duplicates of the innermost one.
/// Use this function with caution, since for example layers with the same tile everywhere can be shrunken into nothingness in each direction.
fn lossless_shrink_distances<T: Default + Clone + PartialEq>(
    ndarray: &Array2<T>,
) -> (Extent2<usize>, Extent2<usize>) {
    let rows: Vec<_> = ndarray.rows().into_iter().collect();
    let up = match rows.windows(2).position(|rows| rows[0] != rows[1]) {
        Some(pos) => pos,
        None => ndarray.nrows(),
    };
    let down = match rows.windows(2).rev().position(|rows| rows[0] != rows[1]) {
        Some(pos) => pos,
        None => ndarray.nrows(),
    };

    let columns: Vec<_> = ndarray.columns().into_iter().collect();
    let left = match columns.windows(2).position(|rows| rows[0] != rows[1]) {
        Some(pos) => pos,
        None => ndarray.ncols(),
    };
    let right = match columns.windows(2).rev().position(|rows| rows[0] != rows[1]) {
        Some(pos) => pos,
        None => ndarray.ncols(),
    };
    (Extent2::new(left, up), Extent2::new(right, down))
}
