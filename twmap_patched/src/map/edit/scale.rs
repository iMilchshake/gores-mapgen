use crate::*;

use az::{CheckedAs, UnwrappedAs};
use fixed::traits::Fixed;
use fixed::types::I17F15;
use image::{GenericImage, GenericImageView, RgbaImage};
use ndarray::Array2;
use vek::num_traits::CheckedMul;
use vek::{Extent2, Vec2};

use std::mem;
use std::ops::Deref;

fn scale<T: Fixed>(vec: Vec2<T>, factor: T) -> Option<Vec2<T>> {
    vec.checked_mul(&Vec2::broadcast(factor))
}

fn scale_ext<T: Fixed>(ext: Extent2<T>, factor: T) -> Option<Extent2<T>> {
    ext.checked_mul(&Extent2::broadcast(factor))
}

impl Quad {
    fn scale(self, factor: I17F15) -> Option<Self> {
        let corners = self.corners.map(|c| scale(c, factor));
        if corners.iter().any(|c| c.is_none()) {
            return None;
        }
        let corners = corners.map(|c| c.unwrap());
        Some(Self {
            corners,
            position: scale(self.position, factor)?,
            ..self
        })
    }
}

impl QuadsLayer {
    fn scale(self, factor: I17F15) -> Option<Self> {
        Some(Self {
            quads: self
                .quads
                .into_iter()
                .map(|q| q.scale(factor))
                .collect::<Option<_>>()?,
            ..self
        })
    }
}

impl SoundArea {
    fn scale(self, factor: I17F15) -> Option<Self> {
        match self {
            Self::Rectangle(mut rekt) => {
                rekt.set_position(scale(rekt.position(), factor)?);
                rekt.set_extent(scale_ext(rekt.extent(), factor)?);
            }
            Self::Circle(mut disk) => {
                disk.center = scale(disk.center, factor)?;
                disk.radius = disk.radius.checked_mul(factor.checked_as()?)?;
            }
        }
        Some(self)
    }
}

impl SoundSource {
    fn scale(self, factor: I17F15) -> Option<Self> {
        Some(Self {
            area: self.area.scale(factor)?,
            ..self
        })
    }
}

impl SoundsLayer {
    fn scale(self, factor: I17F15) -> Option<Self> {
        Some(Self {
            sources: self
                .sources
                .into_iter()
                .map(|s| s.scale(factor))
                .collect::<Option<_>>()?,
            ..self
        })
    }
}

impl EnvPoint<Position> {
    fn scale(mut self, factor: I17F15) -> Option<Self> {
        self.content.offset = scale(self.content.offset, factor)?;
        Some(self)
    }
}

impl Envelope {
    fn scale(self, factor: u8) -> Option<Self> {
        let f = factor.checked_as()?;
        Some(match self {
            Envelope::Position(mut env) => {
                env.points = env
                    .points
                    .into_iter()
                    .map(|p| p.scale(f))
                    .collect::<Option<_>>()?;
                Envelope::Position(env)
            }
            env => env,
        })
    }
}

type IdMapping = [Option<(usize, u8)>; 256];
type ScaledImage = (Vec<EmbeddedImage>, IdMapping);

impl EmbeddedImage {
    /// Factor may only be one of 1, 2, 4, 8, 16
    fn scale(&self, factor: u8) -> Option<ScaledImage> {
        let factor = u32::from(factor);

        let mapres = self.image.unwrap_ref();
        let tile_w = mapres.width() / 16;
        let tile_h = mapres.height() / 16;

        let mut tiles: [u32; 256] = [0; 256];
        tiles
            .iter_mut()
            .enumerate()
            .for_each(|(id, x)| *x = id.unwrapped_as());
        let tiles = tiles.map(|id| {
            let y = id / 16;
            let x = id % 16;
            mapres.view(x * tile_w, y * tile_h, tile_w, tile_h)
        });

        // Figure out which tile indices are actually used in the mapres
        let mut keep_tiles = tiles.map(|view| view.pixels().any(|p| p.2 .0[3] != 0));
        keep_tiles[0] = false;

        let tiles_per_row = 16 / factor;
        let tiles_per_image = tiles_per_row.pow(2) - 1;
        let tiles_to_keep_count = keep_tiles
            .iter()
            .filter(|b| **b)
            .count()
            .checked_as::<u32>()?;
        if tiles_to_keep_count == 0 {
            return Some((Vec::new(), [None; 256]));
        }
        let mut image_count = (tiles_to_keep_count / tiles_per_image).checked_as::<usize>()?;
        if tiles_to_keep_count % tiles_per_image != 0 {
            image_count += 1;
        }
        let new_image_width = mapres.width() / factor;
        let new_image_height = mapres.height() / factor;
        let mut images = vec![RgbaImage::new(new_image_width, new_image_height); image_count];
        let mut mapping = [None; 256];
        for (index_usize, id_usize) in keep_tiles
            .iter()
            .enumerate()
            .filter_map(|(id, b)| b.then(|| id))
            .enumerate()
        {
            let image_index = index_usize / tiles_per_image.checked_as::<usize>()?;
            let index = index_usize.checked_as::<u32>()?;
            let new_id = (index % tiles_per_image) + 1;
            let big_x = new_id % tiles_per_row;
            let big_y = new_id / tiles_per_row;
            mapping[id_usize] = Some((
                image_index,
                (big_y * factor * 16 + big_x * factor).checked_as()?,
            ));
            images[image_index]
                .sub_image(big_x * tile_w, big_y * tile_h, tile_w, tile_h)
                .copy_from(tiles[id_usize].deref(), 0, 0)
                .unwrap();
        }
        let images = images
            .into_iter()
            .map(|img| EmbeddedImage {
                name: self.name.clone(),
                image: img.into(),
            })
            .collect();
        Some((images, mapping))
    }
}

impl Image {
    fn scale(&self, factor: u8) -> Option<ScaledImage> {
        match self {
            Image::External(_) => unreachable!(),
            Image::Embedded(emb) => emb.scale(factor),
        }
    }
}

fn scale_physics_tiles<T: AnyTile>(tilemap: &Array2<T>, factor: u8) -> Option<Array2<T>> {
    let width = tilemap.ncols().checked_mul(factor.checked_as()?)?;
    let height = tilemap.nrows().checked_mul(factor.checked_as()?)?;
    let f = usize::from(factor);
    Some(Array2::from_shape_fn((height, width), |(y, x)| {
        tilemap[(y / f, x / f)]
    }))
}

fn scale_physics_layer<T: PhysicsLayer>(mut layer: T, factor: u8) -> Option<T> {
    let tiles = layer.tiles().unwrap_ref();
    let scaled_tiles = scale_physics_tiles(tiles, factor)?;
    *layer.tiles_mut() = scaled_tiles.into();
    Some(layer)
}

impl Tile {
    fn scale(
        self,
        x: usize,
        y: usize,
        factor: usize,
        current_image: usize,
        mapping: &IdMapping,
    ) -> Self {
        let mapped = mapping[usize::from(self.id)];
        let id = match mapped {
            None => 0,
            Some((image_index, id)) => {
                if image_index == current_image {
                    let mut x_diff = x % factor;
                    let mut y_diff = y % factor;
                    if self.flags.contains(TileFlags::ROTATE) {
                        mem::swap(&mut x_diff, &mut y_diff);
                        y_diff = factor - 1 - y_diff;
                    }
                    if self.flags.contains(TileFlags::FLIP_Y) {
                        y_diff = factor - 1 - y_diff;
                    }
                    if self.flags.contains(TileFlags::FLIP_X) {
                        x_diff = factor - 1 - x_diff;
                    }
                    id + (x_diff + y_diff * 16).unwrapped_as::<u8>()
                } else {
                    0
                }
            }
        };
        Tile { id, ..self }
    }
}

fn scale_tiles(
    tilemap: &Array2<Tile>,
    factor: u8,
    image_count: u32,
    id_mapping: &IdMapping,
) -> Option<Vec<Array2<Tile>>> {
    let factor_usize = usize::from(factor);
    let width = tilemap.ncols().checked_mul(factor.checked_as()?)?;
    let height = tilemap.nrows().checked_mul(factor.checked_as()?)?;

    let image_count = image_count.checked_as::<usize>()?;
    Some(
        (0..image_count)
            .map(|current_image| {
                Array2::from_shape_fn((height, width), |(y, x)| {
                    tilemap[(y / factor_usize, x / factor_usize)].scale(
                        x,
                        y,
                        factor_usize,
                        current_image,
                        id_mapping,
                    )
                })
            })
            .collect(),
    )
}

impl TilesLayer {
    fn scale(self, factor: u8, image_mapping: &ImageMappings) -> Option<Vec<Self>> {
        if self.image.is_none() {
            let tiles = scale_physics_tiles(self.tiles.unwrap_ref(), factor)?.into();
            return Some(vec![Self { tiles, ..self }]);
        }
        let img = &image_mapping[self.image.unwrap().checked_as::<usize>()?]
            .as_ref()
            .unwrap();
        let tiles = scale_tiles(
            self.tiles.unwrap_ref(),
            factor,
            img.image_count,
            &img.id_mapping,
        )?;
        Some(
            tiles
                .into_iter()
                .enumerate()
                .map(|(i, tiles)| TilesLayer {
                    image: Some(img.first_index + i.unwrapped_as::<u16>()),
                    tiles: tiles.into(),
                    ..self.clone()
                })
                .collect(),
        )
    }
}
/// None for images not used in tiles layers
type ImageMappings = Vec<Option<ImageMapping>>;
struct ImageMapping {
    first_index: u16,
    image_count: u32,
    id_mapping: IdMapping,
}
impl Layer {
    fn scale(self, factor: u8, image_mapping: &ImageMappings) -> Option<Vec<Self>> {
        Some(vec![match self {
            Layer::Game(l) => Layer::Game(scale_physics_layer(l, factor)?),
            Layer::Tiles(t) => {
                let layers = t.scale(factor, image_mapping)?;
                return Some(layers.into_iter().map(Layer::Tiles).collect());
            }
            Layer::Quads(q) => Layer::Quads(q.scale(factor.checked_as()?)?),
            Layer::Front(l) => Layer::Front(scale_physics_layer(l, factor)?),
            Layer::Tele(l) => Layer::Tele(scale_physics_layer(l, factor)?),
            Layer::Speedup(l) => Layer::Speedup(scale_physics_layer(l, factor)?),
            Layer::Switch(l) => Layer::Switch(scale_physics_layer(l, factor)?),
            Layer::Tune(l) => Layer::Tune(scale_physics_layer(l, factor)?),
            Layer::Sounds(s) => Layer::Sounds(s.scale(factor.checked_as()?)?),
            Layer::Invalid(i) => Layer::Invalid(i),
        }])
    }
}

impl Group {
    fn scale(mut self, factor: u8, image_mapping: &ImageMappings) -> Option<Self> {
        let f = factor.checked_as()?;
        self.offset = scale(self.offset, f)?;
        let tiles: Vec<Vec<_>> = self
            .layers
            .into_iter()
            .map(|l| l.scale(factor, image_mapping))
            .collect::<Option<_>>()?;
        self.layers = tiles.into_iter().flatten().collect();
        self.clip.set_position(scale(self.clip.position(), f)?);
        self.clip.set_extent(scale_ext(self.clip.extent(), f)?);
        Some(self)
    }
}

impl TwMap {
    /// Panics:
    /// - The factor must be one of 2, 4, 8 or 16!
    /// - All images must be embedded
    pub fn scale(mut self, factor: u8) -> Option<Self> {
        if ![2, 4, 8, 16].contains(&factor) {
            panic!("Invalid scale factor for TwMap::scale, possible values: 2, 4, 8 and 16");
        }
        if self.images.iter().any(|img| img.image().is_none()) {
            panic!("All images must be embedded for TwMap::scale")
        }

        let mut images_used_by_tiles_layers = vec![false; self.images.len()];
        for layer in self.groups.iter().flat_map(|g| &g.layers) {
            if let Layer::Tiles(l) = layer {
                if let Some(img) = l.image {
                    images_used_by_tiles_layers[img.checked_as::<usize>()?] = true;
                }
            }
        }
        // Create a ScaledImage for every mapres used by tiles layers
        let mut scaled_images: Vec<Option<ScaledImage>> = Vec::new();
        for (img, used) in self.images.iter().zip(images_used_by_tiles_layers) {
            scaled_images.push(match used {
                false => None,
                true => Some(img.scale(factor)?),
            });
        }

        let mut image_mapping = Vec::new();
        for scaled in scaled_images {
            match scaled {
                None => image_mapping.push(None),
                Some(scaled) => {
                    let first = self.images.len();
                    image_mapping.push(Some(ImageMapping {
                        first_index: first.checked_as()?,
                        image_count: scaled.0.len().checked_as()?,
                        id_mapping: scaled.1,
                    }));
                    self.images
                        .extend(scaled.0.into_iter().map(Image::Embedded));
                }
            }
        }
        self.groups = self
            .groups
            .into_iter()
            .map(|g| g.scale(factor, &image_mapping))
            .collect::<Option<_>>()?;
        self.envelopes = self
            .envelopes
            .into_iter()
            .map(|e| e.scale(factor))
            .collect::<Option<_>>()?;
        Some(self)
    }
}
