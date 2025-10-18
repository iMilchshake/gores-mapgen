use crate::compression::decompress;
use crate::convert::{To, TryTo};
use crate::map::*;

use image::RgbaImage;
use ndarray::Array2;

use crate::map::checks::{
    CheckData, InternalMapChecking, LayerError, MapErr, MapErrorKind, MapItem,
};
use std::iter;

pub(crate) trait InternalLoad: CheckData {
    fn internal_load_unchecked(&mut self) -> Result<(), MapError>;

    fn internal_load(&mut self) -> Result<(), MapError> {
        self.internal_load_unchecked()?;
        self.check_data().map_err(MapErr::from)?;
        Ok(())
    }
}

impl InternalLoad for CompressedData<Vec<u8>, ()> {
    fn internal_load_unchecked(&mut self) -> Result<(), MapError> {
        self.check_data().map_err(MapErr::from)?;
        if let CompressedData::Compressed(compressed_data, size, _) = self {
            let decompressed_data = decompress(compressed_data, *size)
                .map_err(|err| MapErr::from(MapErrorKind::from(err)))?;
            *self = CompressedData::Loaded(decompressed_data);
        }
        Ok(())
    }
}

impl InternalLoad for CompressedData<RgbaImage, ImageLoadInfo> {
    fn internal_load_unchecked(&mut self) -> Result<(), MapError> {
        self.check_data().map_err(MapErr::from)?;
        if let CompressedData::Compressed(compressed_data, size, info) = self {
            let decompressed_data = decompress(compressed_data, *size)
                .map_err(|err| MapErr::from(MapErrorKind::from(err)))?;
            let loaded_data = <RgbaImage>::from_raw(
                info.size.w.try_to(),
                info.size.h.try_to(),
                decompressed_data,
            )
            .unwrap();
            *self = CompressedData::Loaded(loaded_data);
        }
        Ok(())
    }
}

impl<T: AnyTile> InternalLoad for CompressedData<Array2<T>, TilesLoadInfo>
where
    Self: CheckData,
{
    fn internal_load_unchecked(&mut self) -> Result<(), MapError> {
        if let CompressedData::Compressed(_, _, _) = self {
            self.check_data().map_err(MapErr::from)?;
        }
        if let CompressedData::Compressed(compressed_data, size, info) = self {
            let dims = info.size.unwrapped_as::<usize>();

            let mut decompressed_data = decompress(compressed_data, *size)
                .map_err(|err| MapErr::from(MapErrorKind::from(err)))?;
            if info.compression {
                assert_eq!(decompressed_data.len() % 4, 0);
                let data: Vec<u8> = decompressed_data
                    .chunks(4)
                    .flat_map(|tile| {
                        iter::repeat(tile)
                            .take(tile[2].to::<usize>() + 1)
                            .flat_map(|tile| [tile[0], tile[1], 0, tile[3]].to_vec())
                    })
                    .collect();
                if data.len() != dims.w * dims.h * 4 {
                    return Err(
                        MapErr::from(MapErrorKind::from(LayerError::TeeworldsCompression))
                            .with_type(Layer::TYPE)
                            .into(),
                    );
                }
                decompressed_data = data;
            }
            let tiles = T::view_boxed_slice(decompressed_data.into_boxed_slice())
                .unwrap()
                .to_vec();
            let tiles = Array2::from_shape_vec((dims.h, dims.w), tiles).unwrap();
            *self = CompressedData::Loaded(tiles);
        }
        Ok(())
    }
}

pub trait Load {
    fn load_unchecked(&mut self) -> Result<(), MapError>;

    fn load(&mut self) -> Result<(), MapError>;
}

impl<T: InternalLoad> Load for T {
    fn load_unchecked(&mut self) -> Result<(), MapError> {
        self.internal_load_unchecked()
    }

    fn load(&mut self) -> Result<(), MapError> {
        self.internal_load()
    }
}

impl Load for Image {
    fn load_unchecked(&mut self) -> Result<(), MapError> {
        if let Some(img) = self.image_mut() {
            img.load_unchecked()
                .map_err(|err| MapError(err.0.with_type(MapItem::Image)))
        } else {
            Ok(())
        }
    }

    fn load(&mut self) -> Result<(), MapError> {
        if let Some(img) = self.image_mut() {
            img.load()
                .map_err(|err| MapError(err.0.with_type(MapItem::Image)))
        } else {
            Ok(())
        }
    }
}

impl Load for Sound {
    fn load_unchecked(&mut self) -> Result<(), MapError> {
        self.data
            .load_unchecked()
            .map_err(|err| MapError(err.0.with_type(MapItem::Sound)))
    }

    fn load(&mut self) -> Result<(), MapError> {
        self.data
            .load()
            .map_err(|err| MapError(err.0.with_type(MapItem::Sound)))
    }
}

impl Load for Layer {
    fn load_unchecked(&mut self) -> Result<(), MapError> {
        match self {
            Layer::Game(l) => l.tiles_mut().load_unchecked(),
            Layer::Tiles(l) => l.tiles_mut().load_unchecked(),
            Layer::Front(l) => l.tiles_mut().load_unchecked(),
            Layer::Tele(l) => l.tiles_mut().load_unchecked(),
            Layer::Speedup(l) => l.tiles_mut().load_unchecked(),
            Layer::Switch(l) => l.tiles_mut().load_unchecked(),
            Layer::Tune(l) => l.tiles_mut().load_unchecked(),
            Layer::Sounds(_) | Layer::Quads(_) | Layer::Invalid(_) => Ok(()),
        }
    }

    fn load(&mut self) -> Result<(), MapError> {
        match self {
            Layer::Game(l) => l.tiles_mut().load(),
            Layer::Tiles(l) => l.tiles_mut().load(),
            Layer::Front(l) => l.tiles_mut().load(),
            Layer::Tele(l) => l.tiles_mut().load(),
            Layer::Speedup(l) => l.tiles_mut().load(),
            Layer::Switch(l) => l.tiles_mut().load(),
            Layer::Tune(l) => l.tiles_mut().load(),
            Layer::Sounds(_) | Layer::Quads(_) | Layer::Invalid(_) => Ok(()),
        }
        .map_err(|err| MapError(err.0.with_type(MapItem::Layer)))
    }
}

pub trait LoadMultiple {
    type Item;

    fn load_unchecked(&mut self) -> Result<(), MapError>;

    fn load_conditionally(
        &mut self,
        condition: impl Fn(&Self::Item) -> bool + Copy,
    ) -> Result<(), MapError>;

    fn load(&mut self) -> Result<(), MapError> {
        self.load_conditionally(|_| true)
    }
}

impl<T: Load> LoadMultiple for [T] {
    type Item = T;

    fn load_unchecked(&mut self) -> Result<(), MapError> {
        for (i, item) in self.iter_mut().enumerate() {
            item.load_unchecked()
                .map_err(|err| MapError(err.0.with_index(i)))?;
        }
        Ok(())
    }

    fn load_conditionally(
        &mut self,
        condition: impl Fn(&Self::Item) -> bool + Copy,
    ) -> Result<(), MapError> {
        for (i, item) in self.iter_mut().enumerate() {
            if !condition(item) {
                continue;
            }
            item.load().map_err(|err| MapError(err.0.with_index(i)))?;
        }
        Ok(())
    }
}

impl LoadMultiple for Group {
    type Item = Layer;

    fn load_unchecked(&mut self) -> Result<(), MapError> {
        self.layers
            .load_unchecked()
            .map_err(|err| MapError(err.0.with_type(MapItem::Group)))
    }

    fn load_conditionally(
        &mut self,
        condition: impl Fn(&Self::Item) -> bool + Copy,
    ) -> Result<(), MapError> {
        self.layers
            .load_conditionally(condition)
            .map_err(|err| MapError(err.0.with_type(MapItem::Group)))
    }
}

impl LoadMultiple for [Group] {
    type Item = Layer;

    fn load_unchecked(&mut self) -> Result<(), MapError> {
        for (i, item) in self.iter_mut().enumerate() {
            item.load_unchecked()
                .map_err(|err| MapError(err.0.with_index(i)))?;
        }
        Ok(())
    }

    fn load_conditionally(
        &mut self,
        condition: impl Fn(&Self::Item) -> bool + Copy,
    ) -> Result<(), MapError> {
        for (i, item) in self.iter_mut().enumerate() {
            item.load_conditionally(condition)
                .map_err(|err| MapError(err.0.with_index(i)))?;
        }
        Ok(())
    }
}

impl TwMap {
    pub fn load(&mut self) -> Result<(), MapError> {
        self.images.load()?;
        self.groups.load()?;
        self.sounds.load()?;
        Ok(())
    }

    pub fn load_unchecked(&mut self) -> Result<(), MapError> {
        self.images.load_unchecked()?;
        self.groups.load_unchecked()?;
        self.sounds.load_unchecked()?;
        Ok(())
    }
}
