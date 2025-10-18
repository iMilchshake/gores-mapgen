use crate::constants;
use crate::convert::{To, TryTo};
use crate::map::*;

use fixed::types::{I17F15, I27F5};
use image::{ImageError, Pixel, RgbaImage};
use vek::num_traits::CheckedAdd;
use vek::Extent2;

use std::fs;
use std::io;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

mod dilate;
mod extend;
mod mirror;
mod optimize_mapres;
mod rotate;
mod scale;
mod shrink;
mod tiles;
mod unused;

pub use dilate::dilate;
pub use extend::edge_extend_ndarray;
pub use shrink::shrink_ndarray;
pub use tiles::{
    DDNetFixPhysicsRotation, DDNetMigrateSpeedup, DDNetNormalizePhysicsRotation, EditTile,
    TileFlips, ZeroAir, ZeroUnusedParts,
};

/// Desired amount of sub-tiles (tiles / 32) in default camera view
const AMOUNT: u32 = 1150 * 1000;
/// Maximum amount of sub-tiles horizontally
const MAX_WIDTH: i32 = 1500;
/// Maximum amount of sub-tiles vertically
const MAX_HEIGHT: i32 = 1150;

/// The maximum amount of tiles horizontally and vertically that the camera can cover.
/// Note that it can never be the maximum in both directions.
/// Default zoom level is assumed.
pub const MAX_CAMERA_DIMENSIONS: Extent2<I27F5> = Extent2 {
    w: I27F5::from_bits(MAX_WIDTH),
    h: I27F5::from_bits(MAX_HEIGHT),
};

/// The aspect ratio is width / height.
/// Returns the dimension in tiles that Teeworlds/DDNet would render in that aspect ratio.
/// Default zoom level is assumed.
pub fn camera_dimensions(aspect_ratio: f32) -> Extent2<f32> {
    /*
    width (x), height (y) calculation from the aspect ratio
        x * y = x * yß
    <=>     x = (x * y) / y
    <=>   x^2 = (x * y) * (x / y)
    <=>   x^2 = AMOUNT * aspect_ratio
    <=>     x = sqrt(AMOUNT * aspect_ratio)
    */
    let mut width = (AMOUNT as f32 * aspect_ratio).sqrt();
    // x * y = x * y <=> y = x * (y / x) <=> y = x / aspect_ratio
    let mut height = width / aspect_ratio;
    // If a calculated length exceeds the maximum, cap it at the respective maximum
    if width > MAX_WIDTH as f32 {
        width = MAX_WIDTH as f32;
        height = width / aspect_ratio;
    }
    if height > MAX_HEIGHT as f32 {
        height = MAX_HEIGHT as f32;
        width = height * aspect_ratio;
    }
    Extent2::new(width / 32., height / 32.)
}

impl TwMap {
    /// Returns a reference to the specified physics layer, if the map contains it.
    /// Note that every map must have a Game layer to pass the checks.
    pub fn find_physics_layer<T: PhysicsLayer>(&self) -> Option<&T> {
        match self
            .physics_group()
            .layers
            .iter()
            .rev()
            .find(|l| l.kind() == T::kind())
        {
            None => None,
            Some(l) => T::get(l),
        }
    }

    /// Returns a mutable reference to the specified physics layer, if the map contains it.
    /// Note that every map must have a Game layer to pass the checks.
    pub fn find_physics_layer_mut<T: PhysicsLayer>(&mut self) -> Option<&mut T> {
        match self
            .physics_group_mut()
            .layers
            .iter_mut()
            .rev()
            .find(|l| l.kind() == T::kind())
        {
            None => None,
            Some(l) => T::get_mut(l),
        }
    }
}

impl LayerKind {
    // returns index suitable for checking for duplicates
    const fn duplicate_index(&self) -> usize {
        match self {
            LayerKind::Game => 0,
            LayerKind::Front => 1,
            LayerKind::Switch => 2,
            LayerKind::Tele => 3,
            LayerKind::Speedup => 4,
            LayerKind::Tune => 5,
            _ => panic!(), // may exist multiple times
        }
    }
}

impl TwMap {
    pub fn parse_path_unchecked<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref();

        let metadata = fs::metadata(path)?;
        if metadata.is_file() {
            let map_data = fs::read(path)?;
            TwMap::parse(&map_data)
        } else if metadata.is_dir() {
            TwMap::parse_dir_unchecked(path)
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "Neither a file nor directory").into())
        }
    }

    /// Parses binary as well as MapDir maps.
    pub fn parse_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let map = TwMap::parse_path_unchecked(&path)?;
        map.check()?;
        Ok(map)
    }
}

impl Sound {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Sound, opus_headers::ParseError> {
        let path = path.as_ref();
        let name = path.file_stem().unwrap().to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "The file name includes invalid utf-8",
            )
        })?;
        let reader = std::fs::File::open(path)?;
        Self::from_reader(name, reader)
    }

    /// Creates a sound from a reader.
    pub fn from_reader<R: Read>(
        name: &str,
        mut reader: R,
    ) -> Result<Sound, opus_headers::ParseError> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        opus_headers::parse_from_read(&buf[..])?;

        Ok(Sound {
            name: name.to_string(),
            data: buf.into(),
        })
    }
}

impl EmbeddedImage {
    /// Creates a embedded image using a provided image file.
    /// This function supports png and any other file formats you activate via features of the crate `image`.
    /// In your `Cargo.toml` you should turn off the default features of `image` by setting `default-features = false`.
    /// Then you can activate different file formats by activating specific features.
    /// Errors with an io::ErrorKind::InvalidInput if the filename includes invalid utf-8
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<EmbeddedImage, ImageError> {
        let path = path.as_ref();
        let name = match path.file_stem().unwrap().to_str() {
            Some(str) => str,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "The file name includes invalid utf-8",
                )
                .into())
            }
        };
        let reader = std::fs::File::open(path)?;
        Self::from_reader(name, reader)
    }

    /// Creates an embedded image from the provided reader.
    /// This function wraps the provided reader around a [BufReader] so you don't have to do it.
    pub fn from_reader<R: Read + Seek>(name: &str, inner: R) -> Result<EmbeddedImage, ImageError> {
        let reader =
            image::ImageReader::with_format(BufReader::new(inner), image::ImageFormat::Png);
        let image = reader.decode()?.into_rgba8().into();
        Ok(EmbeddedImage {
            name: name.to_string(),
            image,
        })
    }
}

impl ExternalImage {
    /// Tries to embed the external images by loading them from the file `<mapres_directory>/<image_name>`
    pub fn embed<P: AsRef<Path>>(&self, mapres_directory: P) -> Result<EmbeddedImage, ImageError> {
        let mut file_name = self.name.clone();
        file_name.push_str(".png");

        let path = mapres_directory.as_ref().join(&file_name);

        EmbeddedImage::from_file(path)
    }
}

impl TwMap {
    /// Tries to embed the external images by loading them from the file `<mapres_directory>/<image_name>`
    pub fn embed_images<P: AsRef<Path>>(&mut self, mapres_directory: P) -> Result<(), ImageError> {
        let mut result = Ok(());
        for image in &mut self.images {
            if let Image::External(ex) = image {
                match ex.embed(mapres_directory.as_ref()) {
                    Ok(emb) => *image = Image::Embedded(emb),
                    Err(err) => {
                        if result.is_ok() {
                            result = Err(err);
                        }
                    }
                }
            }
        }
        result
    }

    /// Embed images with the twstorage file paths.
    /// This will take the config directory into account.
    pub fn embed_images_auto(&mut self) -> Result<(), ImageError> {
        let mut result = Ok(());

        fn read_mapres(name: &str, version: Version) -> Result<EmbeddedImage, ImageError> {
            let path = format!("mapres/{name}.png");
            let file = twstorage::read_file(&path, version.into())?;
            EmbeddedImage::from_reader(name, file)
        }

        for image in &mut self.images {
            if let Image::External(ext) = image {
                match read_mapres(&ext.name, self.version) {
                    Ok(emb) => *image = Image::Embedded(emb),
                    Err(err) => {
                        if result.is_ok() {
                            result = Err(err);
                        }
                    }
                }
            }
        }

        result
    }
}

impl TwMap {
    /// For easy remapping of all image indices in tiles layers and quads.
    pub fn edit_image_indices(&mut self, edit_fn: impl Fn(Option<u16>) -> Option<u16>) {
        for group in &mut self.groups {
            for layer in &mut group.layers {
                match layer {
                    Layer::Tiles(l) => l.image = edit_fn(l.image),
                    Layer::Quads(l) => l.image = edit_fn(l.image),
                    _ => {}
                }
            }
        }
    }

    /// For easy remapping of all envelope indices in tiles, quads and sounds layers.
    pub fn edit_env_indices(&mut self, edit_fn: impl Fn(Option<u16>) -> Option<u16>) {
        for group in &mut self.groups {
            for layer in &mut group.layers {
                use Layer::*;
                match layer {
                    Tiles(l) => l.color_env = edit_fn(l.color_env),
                    Quads(l) => {
                        for quad in &mut l.quads {
                            quad.color_env = edit_fn(quad.color_env);
                            quad.position_env = edit_fn(quad.position_env);
                        }
                    }
                    Sounds(l) => {
                        for source in &mut l.sources {
                            source.position_env = edit_fn(source.position_env);
                            source.sound_env = edit_fn(source.sound_env);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// For easy remapping of all sound indices in sounds layers.
    pub fn edit_sound_indices(&mut self, edit_fn: impl Fn(Option<u16>) -> Option<u16>) {
        for group in &mut self.groups {
            for layer in &mut group.layers {
                if let Layer::Sounds(l) = layer {
                    l.sound = edit_fn(l.sound)
                }
            }
        }
    }
}

/// Returns the table for the OPAQUE tile flag, derived from the image data
pub fn calc_opaque_table(image: &RgbaImage) -> [[bool; 16]; 16] {
    let tile_width = image.width() / 16;
    let tile_height = image.height() / 16;
    let mut table = [[false; 16]; 16];
    if tile_width != tile_height {
        return table;
    }

    for tile_y in 0..16 {
        let tile_y_pos = tile_y * tile_width;
        for tile_x in 0..16 {
            let tile_x_pos = tile_x * tile_width;
            let mut opaque = true;

            'outer: for pixel_y in 0..tile_width {
                for pixel_x in 0..tile_width {
                    let y = tile_y_pos + pixel_y;
                    let x = tile_x_pos + pixel_x;
                    if image.get_pixel(x, y).channels()[3] < 250 {
                        opaque = false;
                        break 'outer;
                    }
                }
            }

            table[tile_y.try_to::<usize>()][tile_x.try_to::<usize>()] = opaque;
        }
    }
    table[0][0] = false; // top left tile is manually emptied
    table
}

impl Image {
    /// Returns the table for the OPAQUE tile flag of the image.
    pub fn calc_opaque_table(&self, version: Version) -> [[bool; 16]; 16] {
        match self {
            Image::External(image) => {
                constants::external_opaque_table(&image.name, version).unwrap_or([[false; 16]; 16])
            }
            Image::Embedded(image) => calc_opaque_table(image.image.unwrap_ref()),
        }
    }
}

impl TwMap {
    /// Fill in all OPAQUE tile flags.
    pub fn process_tile_flag_opaque(&mut self) {
        let tables: Vec<[[bool; 16]; 16]> = self
            .images
            .iter()
            .map(|image| image.calc_opaque_table(self.version))
            .collect();
        for group in &mut self.groups {
            for layer in &mut group.layers {
                if let Layer::Tiles(layer) = layer {
                    let opaque_table = {
                        if layer.color.a != 255 {
                            &[[false; 16]; 16]
                        } else {
                            match layer.image {
                                None => &[[false; 16]; 16],
                                Some(index) => &tables[index.to::<usize>()],
                            }
                        }
                    };
                    let tiles = layer.tiles.unwrap_mut();
                    for tile in tiles {
                        let opaque_value =
                            opaque_table[tile.id.to::<usize>() / 16][tile.id.to::<usize>() % 16];
                        tile.flags.set(TileFlags::OPAQUE, opaque_value);
                    }
                }
            }
        }
    }

    /// Set the width and height of external images to their default values.
    pub fn set_external_image_dimensions(&mut self) {
        for image in &mut self.images {
            if let Image::External(ex) = image {
                if let Some(size) = constants::external_dimensions(&ex.name, self.version) {
                    ex.size = size;
                }
            }
        }
    }
}

impl QuadsLayer {
    // TODO: somehow make atomic + the return value reasonable
    fn shift(&mut self, offset: Vec2<I17F15>) -> Option<()> {
        for quad in &mut self.quads {
            quad.position = quad.position.checked_add(&offset)?;
            for corner in &mut quad.corners {
                *corner = corner.checked_add(&offset)?;
            }
        }
        Some(())
    }
}

impl SoundsLayer {
    // TODO: somehow make the return value reasonable
    fn shift(&mut self, offset: Vec2<I17F15>) -> Option<()> {
        for source in &mut self.sources {
            source
                .area
                .set_position(source.area.position().checked_add(&offset)?);
        }
        Some(())
    }
}

impl TwMap {
    /// Move cosmetic layers from the physics group into separate groups, keeping the render order
    pub fn isolate_physics_layers(&mut self) {
        let index = self
            .groups
            .iter()
            .position(|g| g.is_physics_group())
            .unwrap();
        let game_group = &mut self.groups[index];
        let mut front_group = Group {
            name: String::from("v Front"),
            ..Group::physics()
        };
        let mut back_group = Group {
            name: String::from("^ Back"),
            ..Group::physics()
        };
        let mut i = 0;
        let mut after = false;
        while i < game_group.layers.len() {
            if !game_group.layers[i].kind().is_physics_layer() {
                if after {
                    back_group.layers.push(game_group.layers.remove(i));
                } else {
                    front_group.layers.push(game_group.layers.remove(i));
                }
            } else {
                if game_group.layers[i].kind() == LayerKind::Game {
                    after = true;
                }
                i += 1;
            }
        }
        if !back_group.layers.is_empty() {
            self.groups.insert(index + 1, back_group);
        }
        if !front_group.layers.is_empty() {
            self.groups.insert(index, front_group);
        }
    }
}

fn calc_new_offset(
    former_offset: I27F5,
    origin_shift: I27F5,
    parallax: i32,
    align_size: I27F5,
) -> Option<I27F5> {
    let origin_shift_parallaxed = origin_shift
        .checked_mul_int(parallax)?
        .checked_div_int(100)?;
    let offset_offset = origin_shift_parallaxed.checked_sub(align_size)?;
    former_offset
        .checked_add(offset_offset)?
        .checked_mul_int(-1)
}

fn calc_new_clip_pos(align_size: I27F5, clip_pos: I27F5, clip_size: I27F5) -> Option<I27F5> {
    let new_clip_corner_pos = clip_pos.checked_add(clip_size)?;
    align_size.checked_sub(new_clip_corner_pos)
}
