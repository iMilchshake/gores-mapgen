use crate::map::PARALLAX_DIVISOR;
use crate::*;

use fixed::types::I27F5;
use ndarray::{s, Array2};
use vek::num_traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub};
use vek::Extent2;

impl TwMap {
    /// Increases the size of the tilemap layers by the specified tile amount in each direction.
    /// To keep the look of the map the same, this also moves the quads in quads layers and adjust the offset of groups as well as their clipping.
    /// Returns None if an overflow occurred.
    #[must_use]
    pub fn extend_layers(
        mut self,
        up_left: Extent2<u16>,
        down_right: Extent2<u16>,
    ) -> Option<TwMap> {
        let tiles_up_left = up_left.checked_as()?;
        let tiles_down_right = down_right.numcast()?;
        let object_offset = up_left.checked_as()?;
        for group in &mut self.groups {
            for layer in &mut group.layers {
                use Layer::*;
                match layer {
                    Game(l) => extend_layer(l, tiles_up_left, tiles_down_right)?,
                    Tiles(l) => extend_layer(l, tiles_up_left, tiles_down_right)?,
                    Quads(l) => l.shift(object_offset.into())?,
                    Front(l) => extend_layer(l, tiles_up_left, tiles_down_right)?,
                    Tele(l) => extend_layer(l, tiles_up_left, tiles_down_right)?,
                    Speedup(l) => extend_layer(l, tiles_up_left, tiles_down_right)?,
                    Switch(l) => extend_layer(l, tiles_up_left, tiles_down_right)?,
                    Tune(l) => extend_layer(l, tiles_up_left, tiles_down_right)?,
                    Sounds(l) => l.shift(object_offset.into())?,
                    Invalid(_) => {}
                }
            }
            let group_up_left: Extent2<I27F5> = up_left.checked_as()?;
            let group_up_left_parallaxed = group_up_left
                .checked_mul(&group.parallax.checked_as()?.into())?
                .checked_div(&PARALLAX_DIVISOR.checked_as()?.into())?;
            // TODO: why add with but also without parallax
            group.offset = group
                .offset
                .checked_add(&group_up_left.into())?
                .checked_sub(&group_up_left_parallaxed.into())?;
            if !group.is_physics_group() {
                let new_group_pos = group.clip.position().checked_add(&group_up_left.into())?;
                group.clip.x = new_group_pos.x;
                group.clip.y = new_group_pos.y;
            }
        }
        Some(self)
    }
}

/// Extend 2D ndarray in each direction by the specified amount.
/// Copies the outer edges into the new region.
/// This simulates the behavior of the end of tilemaps in gameplay, since the outer edges will continue to infinity.
///
/// Returns None if the specified extend amounts cause a integer overflow
// TODO: maybe simplify with simple indexing
pub fn edge_extend_ndarray<T: Default + Copy>(
    ndarray: &Array2<T>,
    up_left: Extent2<usize>,
    down_right: Extent2<usize>,
) -> Option<Array2<T>> {
    let up = up_left.h;
    let down = down_right.h;
    let left = up_left.w;
    let right = down_right.w;
    let old_height = ndarray.nrows();
    let new_height = old_height.checked_add(up)?.checked_add(down)?;
    let old_width = ndarray.ncols();
    let new_width = old_width.checked_add(left)?.checked_add(right)?;
    let mut new_ndarray = Array2::default((new_height, new_width));
    // copy old layer into the new one
    new_ndarray
        .slice_mut(s![up..up + ndarray.nrows(), left..left + ndarray.ncols()])
        .assign(ndarray);
    // copy the corner of the old ndarray into the corner region of the new one
    let corners = vec![
        ((0..up, 0..left), ndarray[(0, 0)]), // top left
        (
            (0..up, new_width - right..new_width),
            ndarray[(0, old_width - 1)],
        ), // top right
        (
            (new_height - down..new_height, 0..left),
            ndarray[(old_height - 1, 0)],
        ), // bottom
        (
            (new_height - down..new_height, new_width - right..new_width),
            ndarray[(old_height - 1, old_width - 1)],
        ),
    ];
    for ((y_slice, x_slice), corner) in corners {
        new_ndarray
            .slice_mut(s![y_slice, x_slice])
            .map_inplace(|e| *e = corner);
    }
    // copy the old outermost rows/columns into the new area
    let vertical_edge_ranges = vec![
        (ndarray.row(0), (0..up, left..left + old_width)), // upper edge
        (
            ndarray.row(old_height - 1),
            (up + old_height..new_height, left..left + old_width),
        ), // lower edge
    ];
    let horizontal_edge_ranges = vec![
        (ndarray.column(0), (up..up + old_height, 0..left)), // left edge
        (
            ndarray.column(old_width - 1),
            (up..up + old_height, left + old_width..new_width),
        ), // right edge
    ];
    for (edge, (y_slice, x_slice)) in vertical_edge_ranges {
        new_ndarray
            .slice_mut(s![y_slice, x_slice])
            .rows_mut()
            .into_iter()
            .for_each(|mut row| row.assign(&edge))
    }
    for (edge, (y_slice, x_slice)) in horizontal_edge_ranges {
        new_ndarray
            .slice_mut(s![y_slice, x_slice])
            .columns_mut()
            .into_iter()
            .for_each(|mut row| row.assign(&edge))
    }
    Some(new_ndarray)
}

fn extend_layer<T: TilemapLayer>(
    layer: &mut T,
    up_left: Extent2<usize>,
    down_right: Extent2<usize>,
) -> Option<()> {
    *layer.tiles_mut().unwrap_mut() =
        edge_extend_ndarray(layer.tiles().unwrap_ref(), up_left, down_right)?;
    Some(())
}
