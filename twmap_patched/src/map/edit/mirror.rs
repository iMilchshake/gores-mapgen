use crate::edit::{calc_new_clip_pos, calc_new_offset, TileFlips};
use crate::*;

use az::{CheckedAs, UnwrappedAs};
use fixed::types::{I17F15, I27F5};
use ndarray::Array2;
use vek::Vec2;

impl TwMap {
    /// Mirrors the map. Switches left and right.
    /// Returns None if an overflow occurred.
    #[must_use]
    pub fn mirror(mut self) -> Option<TwMap> {
        self.isolate_physics_layers();
        // x_shift is the x-difference between the old and new origin
        let origin_x_shift = self
            .find_physics_layer::<GameLayer>()
            .unwrap()
            .tiles
            .shape()
            .w;
        self.groups = self
            .groups
            .into_iter()
            .map(|group| group.mirror(origin_x_shift.checked_as()?))
            .collect::<Option<_>>()?;
        self.envelopes = self
            .envelopes
            .into_iter()
            .map(|env| env.mirror())
            .collect::<Option<_>>()?;
        Some(self)
    }
}

fn mirror(vec2: Vec2<I17F15>, origin_x_shift: I17F15) -> Option<Vec2<I17F15>> {
    Some(Vec2 {
        x: origin_x_shift.checked_sub(vec2.x)?,
        y: vec2.y,
    })
}

impl QuadsLayer {
    fn mirror(mut self, align_width: I17F15) -> Option<QuadsLayer> {
        for quad in &mut self.quads {
            quad.position = mirror(quad.position, align_width)?;
            for corner in &mut quad.corners {
                *corner = mirror(*corner, align_width)?;
            }
        }
        Some(self)
    }
}

impl SoundsLayer {
    fn mirror(mut self, align_width: I17F15) -> Option<SoundsLayer> {
        for source in &mut self.sources {
            source
                .area
                .set_position(mirror(source.area.position(), align_width)?);
        }
        Some(self)
    }
}

impl Layer {
    fn mirror(self, align_width: i32) -> Option<Layer> {
        use Layer::*;
        let align_width_fix_15 = align_width.checked_as::<I17F15>()?;
        assert_eq!(align_width_fix_15.to_bits(), align_width * 32 * 1024);
        Some(match self {
            Game(l) => Game(mirror_tile_layer(l, align_width)),
            Tiles(l) => Tiles(mirror_tile_layer(l, align_width)),
            Quads(l) => Quads(l.mirror(align_width_fix_15)?),
            Front(l) => Front(mirror_tile_layer(l, align_width)),
            Tele(l) => Tele(mirror_tile_layer(l, align_width)),
            Speedup(l) => Speedup(mirror_tile_layer(l, align_width)),
            Switch(l) => Switch(mirror_tile_layer(l, align_width)),
            Tune(l) => Tune(mirror_tile_layer(l, align_width)),
            Sounds(l) => Sounds(l.mirror(align_width_fix_15)?),
            Invalid(l) => Invalid(l),
        })
    }
}

impl Group {
    fn mirror(mut self, origin_x_shift: i32) -> Option<Group> {
        let align_width = match self
            .layers
            .iter()
            .filter_map(|layer| layer.shape())
            .map(|shape| shape.w)
            .max()
        {
            None => origin_x_shift,
            Some(width) => width.checked_as::<i32>()?,
        };
        let origin_x_shift = origin_x_shift.checked_as::<I27F5>()?;
        let group_align_width = align_width.checked_as::<I27F5>()?;
        self.offset.x = calc_new_offset(
            self.offset.x,
            origin_x_shift,
            self.parallax.x,
            group_align_width,
        )?;

        if !self.is_physics_group() {
            self.clip.x = calc_new_clip_pos(origin_x_shift, self.clip.x, self.clip.w)?;
        }
        self.layers = self
            .layers
            .into_iter()
            .map(|layer| layer.mirror(align_width))
            .collect::<Option<_>>()?;
        Some(self)
    }
}

fn mirror_tiles<T: TileFlips + Copy>(tiles: &Array2<T>, align_width: i32) -> Array2<T> {
    let align_width = align_width.unwrapped_as::<usize>();
    if tiles.ncols() > align_width {
        panic!("Array width too large, can't align to smaller width")
    }
    let max_new_index = align_width - 1;
    let max_old_index = tiles.ncols() - 1;
    Array2::from_shape_fn((tiles.nrows(), align_width), |(new_y, new_x)| {
        let old_x = (max_new_index - new_x).min(max_old_index);
        let mut tile = tiles[(new_y, old_x)];
        tile.flip_x();
        tile
    })
}

fn mirror_tile_layer<T: TilemapLayer>(mut layer: T, align_width: i32) -> T
where
    T::TileType: TileFlips,
{
    let mirrored_tiles = mirror_tiles(layer.tiles().unwrap_ref(), align_width);
    *layer.tiles_mut().unwrap_mut() = mirrored_tiles;
    layer
}

impl Envelope {
    fn mirror(mut self) -> Option<Envelope> {
        match &mut self {
            Envelope::Position(env) => {
                for point in &mut env.points {
                    point.content.offset.x = point.content.offset.x.checked_mul_int(-1)?;
                    point.content.rotation = point.content.rotation.checked_mul_int(-1)?;
                }
            }
            Envelope::Color(_) => {}
            Envelope::Sound(_) => {}
        }
        Some(self)
    }
}
