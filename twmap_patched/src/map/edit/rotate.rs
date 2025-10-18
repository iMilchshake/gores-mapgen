use crate::edit::{calc_new_clip_pos, calc_new_offset, TileFlips};
use crate::*;

use az::{CheckedAs, UnwrappedAs};
use fixed::types::{I17F15, I27F5};
use ndarray::Array2;
use vek::Vec2;

use std::mem;

impl TwMap {
    /// Rotates the map clockwise.
    /// Returns `None` if an overflow occurred.
    #[must_use]
    pub fn rotate_right(mut self) -> Option<TwMap> {
        self.isolate_physics_layers();
        let origin_y_shift = self
            .find_physics_layer::<GameLayer>()
            .unwrap()
            .tiles
            .shape()
            .h;
        self.groups = self
            .groups
            .into_iter()
            .map(|group| group.rotate_right(origin_y_shift.checked_as()?))
            .collect::<Option<_>>()?;
        self.envelopes = self
            .envelopes
            .into_iter()
            .map(|env| env.rotate_right())
            .collect::<Option<_>>()?;
        Some(self)
    }
}

impl Group {
    fn rotate_right(mut self, origin_y_shift: i32) -> Option<Group> {
        let align_height = match self
            .layers
            .iter()
            .filter_map(|layer| layer.shape())
            .map(|shape| shape.h)
            .max()
        {
            None => origin_y_shift,
            Some(height) => height.checked_as::<i32>()?,
        };

        let origin_y_shift = origin_y_shift.checked_as::<I27F5>()?;
        let group_align_height = align_height.checked_as::<I27F5>()?;
        let original_offset_x = self.offset.x;
        self.offset.x = calc_new_offset(
            self.offset.y,
            origin_y_shift,
            self.parallax.y,
            group_align_height,
        )?;
        self.offset.y = original_offset_x;

        if !self.is_physics_group() {
            let original_clip_x = self.clip.x;
            self.clip.x = calc_new_clip_pos(origin_y_shift, self.clip.y, self.clip.h)?;
            self.clip.y = original_clip_x;
        }

        mem::swap(&mut self.clip.w, &mut self.clip.h);
        self.parallax = self.parallax.yx();

        self.layers = self
            .layers
            .into_iter()
            .map(|layer| layer.rotate(align_height))
            .collect::<Option<_>>()?;
        Some(self)
    }
}

impl Layer {
    fn rotate(self, align_height: i32) -> Option<Layer> {
        use Layer::*;
        let align_height_fix_15 = align_height.checked_as::<I17F15>()?;
        Some(match self {
            Game(l) => Game(rotate_tile_layer(l, align_height)),
            Tiles(l) => Tiles(rotate_tile_layer(l, align_height)),
            Quads(l) => Quads(l.rotate_right(align_height_fix_15)?),
            Front(l) => Front(rotate_tile_layer(l, align_height)),
            Tele(l) => Tele(rotate_tile_layer(l, align_height)),
            Speedup(l) => Speedup(rotate_tile_layer(l, align_height)),
            Switch(l) => Switch(rotate_tile_layer(l, align_height)),
            Tune(l) => Tune(rotate_tile_layer(l, align_height)),
            Sounds(l) => Sounds(l.rotate_right(align_height_fix_15)?),
            Invalid(l) => Invalid(l),
        })
    }
}

fn rotate_tile_map_right<T: TileFlips>(tiles: &Array2<T>, align_height: i32) -> Array2<T> {
    let align_height = align_height.unwrapped_as::<usize>();
    if tiles.nrows() > align_height {
        panic!("Array width too large, can't align to smaller height")
    }
    let max_new_index = align_height - 1;
    let max_old_index = tiles.nrows() - 1;
    Array2::from_shape_fn((tiles.ncols(), align_height), |(new_y, new_x)| {
        let old_y = (max_new_index - new_x).min(max_old_index);
        let mut rotated = tiles[(old_y, new_y)];
        rotated.rotate_cw();
        rotated
    })
}

fn rotate_tile_layer<T: TilemapLayer>(mut layer: T, align_height: i32) -> T
where
    T::TileType: TileFlips,
{
    let rotated_tiles = rotate_tile_map_right(layer.tiles().unwrap_ref(), align_height);
    *layer.tiles_mut().unwrap_mut() = rotated_tiles;
    layer
}

fn rotate_right(vec2: Vec2<I17F15>, origin_y_shift: I17F15) -> Option<Vec2<I17F15>> {
    Some(Vec2 {
        x: origin_y_shift.checked_sub(vec2.y)?,
        y: vec2.x,
    })
}

impl QuadsLayer {
    fn rotate_right(mut self, align_height: I17F15) -> Option<QuadsLayer> {
        for quad in &mut self.quads {
            quad.position = rotate_right(quad.position, align_height.checked_as()?)?;
            for corner in &mut quad.corners {
                *corner = rotate_right(*corner, align_height)?;
            }
        }
        Some(self)
    }
}

impl SoundsLayer {
    fn rotate_right(mut self, align_height: I17F15) -> Option<SoundsLayer> {
        for source in &mut self.sources {
            source
                .area
                .set_position(rotate_right(source.area.position(), align_height)?);
            if let SoundArea::Rectangle(rect) = &mut source.area {
                mem::swap(&mut rect.w, &mut rect.h);
            }
        }
        Some(self)
    }
}

impl Envelope {
    fn rotate_right(mut self) -> Option<Envelope> {
        match &mut self {
            Envelope::Position(env) => {
                for point in &mut env.points {
                    let original_x = point.content.offset.x;
                    point.content.offset.x = point.content.offset.y.checked_mul_int(-1)?;
                    point.content.offset.y = original_x;
                }
            }
            Envelope::Color(_) => {}
            Envelope::Sound(_) => {}
        }
        Some(self)
    }
}
