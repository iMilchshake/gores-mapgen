use crate::convert::TryTo;
use crate::map::*;

use image::ImageError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

thread_local! {
    pub(crate) static IMAGE_NAMES: RefCell<Option<Vec<String>>> = RefCell::new(None);
    pub(crate) static SOUND_NAMES: RefCell<Option<Vec<String>>> = RefCell::new(None);
    pub(crate) static ENVELOPE_NAMES: RefCell<Option<Vec<String>>> = RefCell::new(None);

    pub(crate) static IMAGE_NAMES_MAPPING: RefCell<Option<HashMap<String, u16>>> = RefCell::new(None);
    pub(crate) static SOUND_NAMES_MAPPING: RefCell<Option<HashMap<String, u16>>> = RefCell::new(None);
    pub(crate) static ENVELOPE_NAMES_MAPPING: RefCell<Option<HashMap<String, u16>>> = RefCell::new(None);
}

fn save_json_file<T: Serialize>(json: &T, path: &Path) -> io::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(json).unwrap();
    bytes.push(b'\n');
    save_bin_file(&bytes, path, "json")
}

fn save_bin_file(buf: &[u8], path: &Path, extension: &str) -> io::Result<()> {
    let mut file = fs::File::create(with_extension(path, extension))?;
    file.write_all(buf)?;
    Ok(())
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct MapDirParseError(#[from] MapDirErr);

#[derive(Error, Debug)]
struct MapDirErr {
    pub path: PathBuf,
    pub kind: MapDirErrorKind,
}

#[derive(Debug)]
enum MapDirErrorKind {
    InvalidUtf8,
    DuplicateImageName,
    Io(io::Error),
    Json(serde_json::Error),
    Image(ImageError),
}

impl fmt::Display for MapDirErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use MapDirErrorKind::*;
        write!(f, "{:?}: ", self.path)?;
        match &self.kind {
            InvalidUtf8 => write!(f, "Invalid utf-8 in file name"),
            DuplicateImageName => write!(
                f,
                "Two images have the same file name with different extensions (.png, .json)"
            ),
            Io(err) => write!(f, "{}", err),
            Json(err) => write!(f, "{}", err),
            Image(err) => write!(f, "{}", err),
        }
    }
}

trait WithPath {
    type OkType;

    fn err_with_path<U: Into<PathBuf>>(self, path: U) -> Result<Self::OkType, MapDirErr>;
}

impl<T> WithPath for Result<T, MapDirErrorKind> {
    type OkType = T;
    fn err_with_path<U: Into<PathBuf>>(self, path: U) -> Result<T, MapDirErr> {
        match self {
            Ok(ok) => Ok(ok),
            Err(kind) => Err(MapDirErr {
                kind,
                path: path.into(),
            }),
        }
    }
}

impl<T> WithPath for io::Result<T> {
    type OkType = T;
    fn err_with_path<U: Into<PathBuf>>(self, path: U) -> Result<T, MapDirErr> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(MapDirErr {
                kind: MapDirErrorKind::Io(err),
                path: path.into(),
            }),
        }
    }
}

impl<T> WithPath for Result<T, serde_json::Error> {
    type OkType = T;
    fn err_with_path<U: Into<PathBuf>>(self, path: U) -> Result<T, MapDirErr> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(MapDirErr {
                kind: MapDirErrorKind::Json(err),
                path: path.into(),
            }),
        }
    }
}

impl<T> WithPath for Result<T, ImageError> {
    type OkType = T;
    fn err_with_path<U: Into<PathBuf>>(self, path: U) -> Result<T, MapDirErr> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(MapDirErr {
                kind: MapDirErrorKind::Image(err),
                path: path.into(),
            }),
        }
    }
}

impl TwMap {
    /// Saves the map in the MapDir format to the passed path.
    /// Will not overwrite existing files/directories, this would result in an IO-error.
    pub fn save_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        self.load()?;
        self.check()?;
        self.save_dir_unwrap(path)?;
        Ok(())
    }

    fn save_dir_unwrap<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        IMAGE_NAMES.with(|f| {
            *f.borrow_mut() = Some(
                self.images
                    .iter()
                    .enumerate()
                    .map(|(index, image)| indexed_name(image.name(), index, self.images.len()))
                    .collect(),
            )
        });

        ENVELOPE_NAMES.with(|f| {
            *f.borrow_mut() = Some(
                self.envelopes
                    .iter()
                    .enumerate()
                    .map(|(index, env)| indexed_name(env.name(), index, self.envelopes.len()))
                    .collect(),
            )
        });

        SOUND_NAMES.with(|f| {
            *f.borrow_mut() = Some(
                self.sounds
                    .iter()
                    .enumerate()
                    .map(|(index, env)| indexed_name(&env.name, index, self.sounds.len()))
                    .collect(),
            )
        });

        fs::create_dir(path)?;

        save_json_file(&self.info, &path.join("info"))?;

        let dir_version = DirVersion::from(self.version);
        save_json_file(&dir_version, &path.join("version"))?;

        if !self.images.is_empty() {
            let images_path = path.join("images");
            fs::create_dir(&images_path)?;

            for (i, image) in self.images.iter().enumerate() {
                let image_file_name = indexed_name(image.name(), i, self.images.len());
                let image_path = images_path.join(&image_file_name);
                image.save_as_file(&image_path)?;
            }
        }

        if !self.envelopes.is_empty() {
            let envelopes_path = path.join("envelopes");
            fs::create_dir(&envelopes_path)?;
            for (i, envelope) in self.envelopes.iter().enumerate() {
                let envelope_file_name = indexed_name(envelope.name(), i, self.envelopes.len());
                let envelope_path = envelopes_path.join(&envelope_file_name);
                save_json_file(&envelope, &envelope_path)?;
            }
        }

        let groups_path = path.join("groups");
        fs::create_dir(&groups_path)?;
        for (i, group) in self.groups.iter().enumerate() {
            let group_directory_name = indexed_name(&group.name, i, self.groups.len());
            let group_path = groups_path.join(&group_directory_name);
            fs::create_dir(&group_path)?;
            save_json_file(&group, &group_path.join("group"))?;

            let group_layers_path = group_path.join("layers");
            if !group.layers.is_empty() {
                fs::create_dir(&group_layers_path)?;
            }
            for (i, layer) in group.layers.iter().enumerate() {
                let layer_file_name = indexed_name(layer.name(), i, group.layers.len());
                let layer_path = group_layers_path.join(layer_file_name);
                save_json_file(&layer, &layer_path)?;
            }
        }

        if !self.sounds.is_empty() {
            let sounds_path = path.join("sounds");
            fs::create_dir(&sounds_path)?;
            for (i, sound) in self.sounds.iter().enumerate() {
                let sound_file_name = indexed_name(&sound.name, i, self.sounds.len());
                let sound_path = sounds_path.join(&sound_file_name);
                save_bin_file(sound.data.unwrap_ref(), &sound_path, "opus")?;
            }
        }

        Ok(())
    }

    /// For parsing a TwMap map directory.
    pub fn parse_dir<P: AsRef<Path>>(path: P) -> Result<TwMap, Error> {
        let mut map = TwMap::parse_dir_unchecked(path)?;
        map.check()?;
        map.process_tile_flag_opaque();
        map.set_external_image_dimensions();
        Ok(map)
    }

    pub fn parse_dir_unchecked<P: AsRef<Path>>(path: P) -> Result<TwMap, Error> {
        let path = path.as_ref();

        fs::read_dir(path)?;

        let images_path = path.join("images");
        let envelopes_path = path.join("envelopes");
        let groups_path = path.join("groups");
        let sounds_path = path.join("sounds");

        let image_names_mapping = create_dir_mapping(&images_path, &["png", "json"])?;
        IMAGE_NAMES_MAPPING.with(|f| *f.borrow_mut() = Some(image_names_mapping));

        let envelope_names_mapping = create_dir_mapping(&envelopes_path, &["json"])?;
        ENVELOPE_NAMES_MAPPING.with(|f| *f.borrow_mut() = Some(envelope_names_mapping));

        let sound_names_mapping = create_dir_mapping(&sounds_path, &["opus"])?;
        SOUND_NAMES_MAPPING.with(|f| *f.borrow_mut() = Some(sound_names_mapping));

        let info_path = path.join("info.json");
        let info = Info::deserialize_json(&info_path)?;

        let version_path = path.join("version.json");
        let dir_version = DirVersion::deserialize_json(&version_path)?;
        let version = dir_version.version;

        let mut images = Vec::new();
        for image_name in list_dir(&images_path, &["png", "json"])? {
            let image_path = images_path.join(&image_name);
            let mut image = Image::from_file(&image_path)?;
            *image.name_mut() = strip_name(image_name);
            images.push(image);
        }

        let mut envelopes = Vec::new();
        for envelope_name in list_dir(&envelopes_path, &["json"])? {
            let envelope_path = envelopes_path.join(&envelope_name);
            let envelope = Envelope::deserialize_json(&envelope_path)?;
            envelopes.push(envelope);
        }

        let mut groups = Vec::new();
        for group_name in list_dir(&groups_path, &[])? {
            let group_path = groups_path.join(&group_name);
            let group_info_path = group_path.join("group.json");
            let mut group = Group::deserialize_json(&group_info_path)?;

            let mut layers = Vec::new();
            let group_layers_path = group_path.join("layers");
            for layer_name in list_dir(&group_layers_path, &["json"])? {
                let layer_path = group_layers_path.join(&layer_name);
                let layer = Layer::deserialize_json(&layer_path)?;
                layers.push(layer);
            }
            group.layers = layers;

            groups.push(group);
        }

        let mut sounds = Vec::new();
        for sound_name in list_dir(&sounds_path, &["opus"])? {
            let sound_path = sounds_path.join(&sound_name);
            let data = fs::read(&sound_path).err_with_path(&sound_path)?;
            sounds.push(Sound {
                name: strip_name(sound_name),
                data: CompressedData::Loaded(data),
            })
        }

        Ok(TwMap {
            version,
            info,
            images,
            envelopes,
            groups,
            sounds,
        })
    }
}

impl From<MapDirErr> for Error {
    fn from(err: MapDirErr) -> Self {
        Error::MapDirParse(MapDirParseError(err))
    }
}

trait DeserializeJson: for<'de> Deserialize<'de> {
    fn deserialize_json(path: &Path) -> Result<Self, MapDirErr> {
        let file = fs::read(path).err_with_path(path)?;
        serde_json::from_slice(&file).err_with_path(path)
    }
}

impl<T: for<'de> Deserialize<'de>> DeserializeJson for T {}

fn create_dir_mapping(
    path: &Path,
    extensions: &[&str],
) -> Result<HashMap<String, u16>, MapDirParseError> {
    let file_names = list_dir(path, extensions)?;
    let mut file_mapping = HashMap::new();
    for (i, file_name) in file_names.iter().enumerate() {
        let name = PathBuf::from(&file_name)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        if file_mapping.contains_key(&name) {
            return Err(MapDirErrorKind::DuplicateImageName).err_with_path(path.join(&name))?;
        }
        file_mapping.insert(name, i.try_to::<u16>());
    }
    Ok(file_mapping)
}

fn list_dir(path: &Path, extensions: &[&str]) -> Result<Vec<String>, MapDirErr> {
    let extensions: Vec<OsString> = extensions.iter().map(OsString::from).collect();

    // filter out file names with incorrect extension
    let dir_entries = match fs::read_dir(path) {
        Ok(dir_entries) => dir_entries,
        Err(err) => {
            return if err.kind() == io::ErrorKind::NotFound {
                Ok(Vec::new())
            } else {
                Err(err).err_with_path(path)
            }
        }
    };

    // filter faulty entries / entries with incorrect file extensions
    let file_names = dir_entries
        .filter_map(|dir_entry| match dir_entry {
            Err(err) => Some(Err(err).err_with_path(path)),
            Ok(entry) => {
                let file_name = entry.file_name();
                if let Some(ext) = PathBuf::from(&file_name).extension() {
                    if extensions.contains(&ext.to_os_string()) {
                        Some(Ok(file_name))
                    } else {
                        None
                    }
                } else {
                    // no extensions is used for listing directories
                    if extensions.is_empty() {
                        Some(Ok(file_name))
                    } else {
                        None
                    }
                }
            }
        })
        .collect::<Result<Vec<_>, MapDirErr>>()?;

    // ensure proper utf-8
    let mut file_names = file_names
        .into_iter()
        .map(|os_str| match os_str.to_str() {
            None => Err(MapDirErrorKind::InvalidUtf8).err_with_path(path.join(os_str)),
            Some(str) => Ok(str.to_string()),
        })
        .collect::<Result<Vec<String>, MapDirErr>>()?;

    // sort alphabetically
    file_names.sort();
    Ok(file_names)
}

fn with_extension(path: &Path, extension: &str) -> PathBuf {
    let mut path = path.as_os_str().to_os_string();
    path.push(".");
    path.push(extension);
    PathBuf::from(path)
}

fn indexed_name(name: &str, i: usize, length: usize) -> String {
    let pad_length = (length - 1).to_string().chars().count();
    if name.is_empty() {
        format!("{:0width$}", i, width = pad_length)
    } else {
        format!("{:0width$}_{}", i, sanitize_name(name), width = pad_length)
    }
}

// removes index + file extension
fn strip_name(mut name: String) -> String {
    // remove file extension
    name = PathBuf::from(name)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // remove prefixed index, if it exists
    if let Some(mid) = name.find('_') {
        let (prefix, remaining) = name.split_at(mid + 1);
        match prefix[..prefix.len() - 1].parse::<usize>() {
            Ok(_) => remaining.to_string(),
            Err(_) => name,
        }
    } else {
        match name.parse::<usize>() {
            Ok(_) => String::new(),
            Err(_) => name,
        }
    }
}

fn sanitize_name(name: &str) -> String {
    sanitize_filename::sanitize_with_options(
        name,
        sanitize_filename::Options {
            windows: true,
            truncate: true,
            replacement: "_",
        },
    )
}

#[derive(Serialize, Deserialize)]
struct DirVersion {
    #[serde(flatten)]
    version: Version,
    created_by: String,
}

impl From<Version> for DirVersion {
    fn from(version: Version) -> Self {
        let mut created_by = env!("CARGO_PKG_NAME").to_string();
        created_by.push(' ');
        created_by.extend(env!("CARGO_PKG_VERSION").chars());
        DirVersion {
            version,
            created_by,
        }
    }
}

impl Image {
    fn save_as_file(&self, path: &Path) -> io::Result<()> {
        match self {
            Image::External(img) => save_json_file(&img, path)?,
            Image::Embedded(img) => {
                let image = img.image.unwrap_ref();
                image
                    .save(with_extension(path, "png"))
                    .map_err(|err| match err {
                        ImageError::IoError(err) => err,
                        _ => panic!("Image saving error that shouldn't happen."),
                    })?;
            }
        }
        Ok(())
    }
}

impl Image {
    fn from_file(path: &Path) -> Result<Self, MapDirErr> {
        match path.extension().unwrap().to_str().unwrap() {
            "json" => Ok(ExternalImage::deserialize_json(path)?.into()),
            "png" => Ok(EmbeddedImage::from_file(path).err_with_path(path)?.into()),
            _ => unreachable!(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct DirTileFlags {
    mirrored: bool,
    rotation: i32,
}

impl From<TileFlags> for DirTileFlags {
    fn from(flags: TileFlags) -> Self {
        let flip_v = flags.contains(TileFlags::FLIP_X);
        let flip_h = flags.contains(TileFlags::FLIP_Y);
        let rotate = flags.contains(TileFlags::ROTATE);

        DirTileFlags {
            mirrored: flip_v ^ flip_h,
            rotation: match (flip_h, rotate) {
                (false, false) => 0,
                (false, true) => 90,
                (true, false) => 180,
                (true, true) => 270,
            },
        }
    }
}

pub(crate) struct DirTileFlagsError(i32);

impl fmt::Display for DirTileFlagsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rotation ({}) can only be one of the values 0, 90, 180 and 270",
            self.0
        )
    }
}

impl TryFrom<DirTileFlags> for TileFlags {
    type Error = DirTileFlagsError;

    fn try_from(v: DirTileFlags) -> Result<Self, Self::Error> {
        let mut flags = TileFlags::empty();
        match v.rotation {
            0 => {}
            90 => flags.insert(TileFlags::ROTATE),
            180 => flags.insert(TileFlags::FLIP_Y),
            270 => flags.insert(TileFlags::FLIP_Y | TileFlags::ROTATE),
            n => return Err(DirTileFlagsError(n)),
        }
        if v.mirrored ^ flags.contains(TileFlags::FLIP_Y) {
            flags.insert(TileFlags::FLIP_X);
        }
        Ok(flags)
    }
}

pub(crate) mod tiles_serialization {
    use crate::convert::TryTo;
    use crate::map::*;
    use ndarray::Array2;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt;

    #[derive(Serialize, Deserialize)]
    struct DirTile<T> {
        #[serde(flatten)]
        position: Vec2<u32>,
        #[serde(flatten)]
        tile: T,
    }

    #[derive(Serialize, Deserialize)]
    struct DirTiles<T> {
        width: usize,
        height: usize,
        tiles: Vec<DirTile<T>>,
    }

    fn dir_tiles<T>(tiles: &Array2<T>) -> DirTiles<T>
    where
        T: Serialize + Default + PartialEq + Copy,
    {
        let width = tiles.ncols();
        let height = tiles.nrows();

        let tiles = tiles
            .indexed_iter()
            .filter_map(|((y, x), tile)| {
                if *tile == T::default() {
                    None
                } else {
                    let tile = *tile;
                    let dir_tile = DirTile {
                        position: Vec2::new(x.try_to(), y.try_to()),
                        tile,
                    };
                    Some(dir_tile)
                }
            })
            .collect::<Vec<_>>();

        DirTiles {
            width,
            height,
            tiles,
        }
    }

    pub(crate) fn serialize<T, S>(
        tiles: &CompressedData<Array2<T>, TilesLoadInfo>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        T: Serialize + Default + PartialEq + Copy,
        S: Serializer,
    {
        let tiles = tiles.unwrap_ref();
        dir_tiles(tiles).serialize(serializer)
    }

    enum DirTileError {
        DuplicateTile(Vec2<u32>),
        OutOfBounds(Vec2<u32>, (usize, usize)),
    }

    impl fmt::Display for DirTileError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use DirTileError::*;
            match self {
                DuplicateTile(pos) => write!(f, "duplicate tile at ({}, {})", pos.x, pos.y),
                OutOfBounds(pos, bound) => write!(
                    f,
                    "tile out of bounds at {}, layer height: {}, width: {}",
                    pos, bound.0, bound.1
                ),
            }
        }
    }

    fn undir_tiles<'a, T>(dir_tiles: DirTiles<T>) -> Result<Array2<T>, DirTileError>
    where
        T: Deserialize<'a> + Default + PartialEq + Copy,
    {
        use DirTileError::*;

        let shape = (dir_tiles.height, dir_tiles.width);
        let mut tiles: Array2<Option<T>> = Array2::default(shape);

        for dir_tile in dir_tiles.tiles {
            let index = (dir_tile.position.y.try_to(), dir_tile.position.x.try_to());
            match tiles.get(index) {
                None => return Err(OutOfBounds(dir_tile.position, shape)),
                Some(Some(_)) => return Err(DuplicateTile(dir_tile.position)),
                Some(None) => tiles[index] = Some(dir_tile.tile),
            }
        }
        let tiles = tiles.map(|&t| match t {
            None => T::default(),
            Some(t) => t,
        });
        Ok(tiles)
    }

    pub(crate) fn deserialize<'de, T, D>(
        deserializer: D,
    ) -> Result<CompressedData<Array2<T>, TilesLoadInfo>, D::Error>
    where
        T: Deserialize<'de> + Default + PartialEq + Copy,
        D: Deserializer<'de>,
    {
        let tiles = DirTiles::deserialize(deserializer)?;
        let tiles = undir_tiles(tiles).map_err(serde::de::Error::custom)?;
        Ok(CompressedData::Loaded(tiles))
    }
}

pub(crate) mod image_index_serialization {
    use super::{IMAGE_NAMES, IMAGE_NAMES_MAPPING};
    use crate::convert::To;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt;

    pub(crate) fn serialize<S: Serializer>(
        opt_index: &Option<u16>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        IMAGE_NAMES.with(|f| {
            opt_index
                .map(|index| f.borrow().as_ref().unwrap()[index.to::<usize>()].clone())
                .serialize(serializer)
        })
    }

    struct ImageNameMissing(String);

    impl fmt::Display for ImageNameMissing {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "image with name {} isn't available", self.0)
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<u16>, D::Error> {
        let opt_name = <Option<String>>::deserialize(deserializer)?;
        let opt_index = match opt_name {
            None => None,
            Some(name) => {
                IMAGE_NAMES_MAPPING.with(|f| match f.borrow().as_ref().unwrap().get(&name) {
                    None => Err(serde::de::Error::custom(ImageNameMissing(name))),
                    Some(&index) => Ok(Some(index)),
                })?
            }
        };
        Ok(opt_index)
    }
}

pub(crate) mod sound_index_serialization {
    use super::{SOUND_NAMES, SOUND_NAMES_MAPPING};
    use crate::convert::To;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt;

    pub(crate) fn serialize<S: Serializer>(
        opt_index: &Option<u16>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        SOUND_NAMES.with(|f| {
            opt_index
                .map(|index| f.borrow().as_ref().unwrap()[index.to::<usize>()].clone())
                .serialize(serializer)
        })
    }

    struct SoundNameMissing(String);

    impl fmt::Display for SoundNameMissing {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "sound with name {} isn't available", self.0)
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<u16>, D::Error> {
        let opt_name = <Option<String>>::deserialize(deserializer)?;
        let opt_index = match opt_name {
            None => None,
            Some(name) => {
                SOUND_NAMES_MAPPING.with(|f| match f.borrow().as_ref().unwrap().get(&name) {
                    None => Err(serde::de::Error::custom(SoundNameMissing(name))),
                    Some(&index) => Ok(Some(index)),
                })?
            }
        };
        Ok(opt_index)
    }
}

pub(crate) mod envelope_index_serialization {
    use super::{ENVELOPE_NAMES, ENVELOPE_NAMES_MAPPING};
    use crate::convert::To;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt;

    pub(crate) fn serialize<S: Serializer>(
        opt_index: &Option<u16>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        ENVELOPE_NAMES.with(|f| {
            opt_index
                .map(|index| f.borrow().as_ref().unwrap()[index.to::<usize>()].clone())
                .serialize(serializer)
        })
    }

    struct EnvelopeNameMissing(String);

    impl fmt::Display for EnvelopeNameMissing {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "envelope with name {} isn't available", self.0)
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<u16>, D::Error> {
        let opt_name = <Option<String>>::deserialize(deserializer)?;
        let opt_index = match opt_name {
            None => None,
            Some(name) => {
                ENVELOPE_NAMES_MAPPING.with(|f| match f.borrow().as_ref().unwrap().get(&name) {
                    None => Err(serde::de::Error::custom(EnvelopeNameMissing(name))),
                    Some(&index) => Ok(Some(index)),
                })?
            }
        };
        Ok(opt_index)
    }
}
