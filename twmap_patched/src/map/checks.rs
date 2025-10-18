use crate::constants;
use crate::map::parse::{BinaryQuad, BinarySoundSource};
use crate::map::*;

use az::{OverflowingAs, OverflowingCast, UnwrappedAs, WrappingCast};
use image::RgbaImage;
use ndarray::Array2;
use thiserror::Error;
use vek::num_traits::Signed;
use vek::Extent2;

use crate::compression::ZlibDecompressionError;
use std::collections::HashSet;
use std::fmt;
use std::mem;

impl TwMap {
    pub fn check(&self) -> Result<(), MapError> {
        Info::check(&self.info, self, &mut ())?;
        Image::check_all(&self.images, self)?;
        Envelope::check_all(&self.envelopes, self)?;
        Group::check_all(&self.groups, self)?;
        Sound::check_all(&self.sounds, self)?;
        self.check_amounts()?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum MapItem {
    Info,
    Image,
    Envelope,
    Group,
    Layer,
    Sound,
    Quad,
    SoundSource,
    EnvPoint,
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct MapError(#[from] pub(crate) MapErr);

#[derive(Error, Debug)]
pub(crate) enum MapErr {
    #[error("In {item:?}{}{sub}", index.map(|i| format!(" at index {} -> ", i)).unwrap_or_default())]
    Recursive {
        item: MapItem,
        index: Option<usize>,
        sub: Box<MapErr>,
    },
    #[error(transparent)]
    Head(MapErrorKind),
}

impl From<MapErrorKind> for MapErr {
    fn from(err: MapErrorKind) -> Self {
        Self::Head(err)
    }
}

impl MapErr {
    pub(crate) fn with_index(mut self, i: usize) -> Self {
        if let Self::Recursive { index, .. } = &mut self {
            *index = Some(i);
        }
        self
    }

    pub(crate) fn with_type(self, item: MapItem) -> Self {
        Self::Recursive {
            item,
            index: None,
            sub: Box::new(self),
        }
    }
}

#[derive(Error, Debug)]
#[error(transparent)]
pub(crate) enum MapErrorKind {
    Teeworlds(#[from] TeeworldsError),
    DDNet(#[from] DDNetError),
    Decompression(#[from] ZlibDecompressionError),
    #[error("{amount} items of this type is too many, the maximum is {max}")]
    Amount {
        amount: usize,
        max: usize,
    },
    #[error("Invalid image index {index} for a map with {len} images")]
    ImageIndex {
        index: u16,
        len: usize,
    },
    #[error("Invalid sound index {index} for a map with {len} sounds")]
    SoundIndex {
        index: u16,
        len: usize,
    },
    #[error("Invalid envelope index {index} for a map with {len} envelopes")]
    EnvelopeIndex {
        index: u16,
        len: usize,
    },
    #[error("Envelope at index {index} referenced as a {expected:?} envelope is instead a {actual:?} envelope")]
    EnvelopeKind {
        index: u16,
        expected: EnvelopeKind,
        actual: EnvelopeKind,
    },
    String(#[from] StringError),
    I32Fit(#[from] ValueMaxError),
    Negative(#[from] NegativeError),
    Image(#[from] ImageError),
    Info(#[from] InfoError),
    Sound(#[from] opus_headers::ParseError),
    Group(#[from] GroupError),
    Layer(#[from] LayerError),
    Tile(#[from] TileError),
    EnvPoint(#[from] EnvPointError),
}

fn check_amount(amount: usize, max: usize, item: MapItem) -> Result<(), MapErr> {
    if amount > max {
        Err(MapErr::from(MapErrorKind::Amount { amount, max }).with_type(item))
    } else {
        Ok(())
    }
}

/// Currently, DDNet maps are a strict superset to Teeworlds maps in regard to their feature set
#[derive(Error, Debug)]
pub enum DDNetError {}

#[derive(Error, Debug)]
pub enum TeeworldsError {
    #[error("Teeworlds does not support settings in the map info")]
    InfoSettings,
    #[error("Teeworlds does not support {0:?} layers")]
    DDNetLayer(LayerKind),
    #[error("Teeworlds does not support automapper configs")]
    TilesAutomapper,
    #[error("Teeworlds does not support sounds")]
    Sounds,
    #[error("Teeworlds does not support sound envelopes")]
    SoundEnv,
}

#[derive(Error, Debug)]
pub(crate) enum StringError {
    #[error("String is {len} bytes long while its maximum length is {max}")]
    Length { len: usize, max: usize },
    #[error("Name '{0}' should be sanitized, for example: {}", sanitize_filename::sanitize_with_options(.0, SANITIZE_OPTIONS))]
    Sanitization(String),
}

const SANITIZE_OPTIONS: sanitize_filename::Options = sanitize_filename::Options {
    windows: true,
    truncate: true,
    replacement: "",
};

const SANITIZE_CHECK_OPTIONS: sanitize_filename::OptionsForCheck =
    sanitize_filename::OptionsForCheck {
        windows: true,
        truncate: true,
    };

#[derive(Error, Debug)]
#[error("Value '{ident}' ({value}) is higher than its maximum value {max}")]
pub(crate) struct ValueMaxError {
    ident: &'static str,
    value: u64,
    max: i32,
}

fn check_max<T>(value: T, max: i32, ident: &'static str) -> Result<(), ValueMaxError>
where
    T: PartialOrd + WrappingCast<i64>,
    i32: OverflowingCast<T>,
{
    let (max, of) = max.overflowing_as::<T>();
    if of {
        return Ok(());
    }
    if value > max {
        Err(ValueMaxError {
            ident,
            value: 0,
            max: i32::MAX,
        })
    } else {
        Ok(())
    }
}

fn check_i32_fit<T>(value: T, ident: &'static str) -> Result<(), ValueMaxError>
where
    T: PartialOrd + WrappingCast<i64>,
    i32: OverflowingCast<T>,
{
    check_max(value, i32::MAX, ident)
}

#[derive(Error, Debug)]
#[error("Value '{ident}' ({value}) must not be negative")]
pub(crate) struct NegativeError {
    ident: &'static str,
    value: String,
}

fn check_non_negative<T>(value: T, ident: &'static str) -> Result<(), NegativeError>
where
    T: Signed + fmt::Display,
{
    if value.is_negative() {
        Err(NegativeError {
            ident,
            value: value.to_string(),
        })
    } else {
        Ok(())
    }
}

/// Passing a file extension triggers sanitization checks
fn check_string(s: &str, max: usize, file_extension: Option<&str>) -> Result<(), StringError> {
    if s.len() > max {
        return Err(StringError::Length { len: s.len(), max });
    }
    if let Some(ext) = file_extension {
        let mut filename = s.to_owned();
        filename.push('.');
        filename.push_str(ext);
        if !sanitize_filename::is_sanitized_with_options(&filename, SANITIZE_CHECK_OPTIONS) {
            return Err(StringError::Sanitization(filename));
        }
    }
    Ok(())
}

impl TwMap {
    fn image_index(&self, index: Option<u16>) -> Result<(), MapErrorKind> {
        let index = match index {
            None => return Ok(()),
            Some(i) => i,
        };
        if index.unwrapped_as::<usize>() >= self.images.len() {
            Err(MapErrorKind::ImageIndex {
                index,
                len: self.images.len(),
            })
        } else {
            Ok(())
        }
    }
    fn sound_index(&self, index: Option<u16>) -> Result<(), MapErrorKind> {
        let index = match index {
            None => return Ok(()),
            Some(i) => i,
        };
        if index.unwrapped_as::<usize>() >= self.sounds.len() {
            Err(MapErrorKind::SoundIndex {
                index,
                len: self.sounds.len(),
            })
        } else {
            Ok(())
        }
    }
    fn envelope_index(&self, index: Option<u16>, kind: EnvelopeKind) -> Result<(), MapErrorKind> {
        let index = match index {
            None => return Ok(()),
            Some(i) => i,
        };
        match self.envelopes.get(index.unwrapped_as::<usize>()) {
            None => Err(MapErrorKind::EnvelopeIndex {
                index,
                len: self.images.len(),
            }),
            Some(env) => {
                if env.kind() == kind {
                    Ok(())
                } else {
                    Err(MapErrorKind::EnvelopeKind {
                        index,
                        expected: kind,
                        actual: env.kind(),
                    })
                }
            }
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum EnvelopeKind {
    Position,
    Color,
    Sound,
}

impl Envelope {
    fn kind(&self) -> EnvelopeKind {
        match self {
            Envelope::Position(_) => EnvelopeKind::Position,
            Envelope::Color(_) => EnvelopeKind::Color,
            Envelope::Sound(_) => EnvelopeKind::Sound,
        }
    }
}

impl TwMap {
    fn check_amounts(&self) -> Result<(), MapErr> {
        let max = u16::MAX.unwrapped_as::<usize>();
        check_amount(self.envelopes.len(), max, Envelope::TYPE)?;
        check_amount(self.sounds.len(), max, Sound::TYPE)?;
        check_amount(self.groups.len(), max, Group::TYPE)?;
        let layers = self.groups.iter().flat_map(|g| &g.layers).count();
        check_amount(layers, max, Layer::TYPE)?;
        check_amount(self.images.len(), 64, Image::TYPE)?;
        Ok(())
    }
}

pub(crate) trait InternalMapChecking: Sized {
    const TYPE: MapItem;
    type State: Default;

    fn check_impl(&self, map: &TwMap) -> Result<(), MapErrorKind>;

    fn check_state_impl(&self, _: &TwMap, _state: &mut Self::State) -> Result<(), MapErrorKind> {
        Ok(())
    }

    fn check_recursive_impl(&self, _: &TwMap) -> Result<(), MapErr> {
        Ok(())
    }

    fn check(&self, map: &TwMap, state: &mut Self::State) -> Result<(), MapErr> {
        self.check_impl(map)
            .map_err(|err| MapErr::from(err).with_type(Self::TYPE))?;
        self.check_recursive_impl(map)
            .map_err(|err| err.with_type(Self::TYPE))?;
        self.check_state_impl(map, state)
            .map_err(|err| MapErr::from(err).with_type(Self::TYPE))?;
        Ok(())
    }

    fn check_state(_: Self::State) -> Result<(), MapErrorKind> {
        Ok(())
    }

    fn check_all(items: &[Self], map: &TwMap) -> Result<(), MapErr> {
        let mut state = Self::State::default();
        for (i, item) in items.iter().enumerate() {
            item.check(map, &mut state)
                .map_err(|err| err.with_index(i))?;
        }
        Self::check_state(state).map_err(|err| MapErr::from(err).with_type(Self::TYPE))?;
        Ok(())
    }
}

pub(crate) trait CheckData {
    fn check_data(&self) -> Result<(), MapErrorKind>;
}

#[derive(Error, Debug)]
#[error("Field {field}{} - {err}", .index.map(|i| format!(" with index {i}")).unwrap_or_default())]
pub(crate) struct InfoError {
    field: &'static str,
    index: Option<usize>,
    err: StringError,
}

impl InternalMapChecking for Info {
    const TYPE: MapItem = MapItem::Info;
    type State = ();

    fn check_impl(&self, map: &TwMap) -> Result<(), MapErrorKind> {
        let items = [
            (&self.author, Info::MAX_AUTHOR_LENGTH, "author", None),
            (&self.version, Info::MAX_VERSION_LENGTH, "version", None),
            (&self.credits, Info::MAX_CREDITS_LENGTH, "credits", None),
            (&self.license, Info::MAX_LICENSE_LENGTH, "license", None),
        ];
        for (s, max, field, index) in IntoIterator::into_iter(items).chain(
            self.settings
                .iter()
                .enumerate()
                .map(|(i, s)| (s, Info::MAX_SETTING_LENGTH, "setting", Some(i))),
        ) {
            check_string(s, max, None).map_err(|err| InfoError { field, index, err })?;
        }
        if map.version == Version::Teeworlds07 && !self.settings.is_empty() {
            return Err(TeeworldsError::InfoSettings.into());
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub(crate) enum ImageError {
    #[error("Image '{name}' is not a valid external image for {version:?}")]
    InvalidExternal { name: String, version: Version },
    #[error("The length ({len}) of the data of this RGBA8 image with size {size} is invalid")]
    DataSize { size: Extent2<u32>, len: usize },
    #[error("Zero sized ({0})")]
    ZeroSized(Extent2<u32>),
}

impl InternalMapChecking for Image {
    const TYPE: MapItem = MapItem::Image;
    type State = ();

    fn check_impl(&self, map: &TwMap) -> Result<(), MapErrorKind> {
        check_string(self.name(), Image::MAX_NAME_LENGTH, Some("png"))?;
        if let Image::External(ex) = self {
            if !constants::is_external_name(&ex.name, map.version) {
                return Err(MapErrorKind::Image(ImageError::InvalidExternal {
                    name: ex.name.clone(),
                    version: map.version,
                }));
            }
        }
        if let Some(emb) = self.image() {
            emb.check_data()?;
        }
        Ok(())
    }
}

impl CheckData for CompressedData<RgbaImage, ImageLoadInfo> {
    fn check_data(&self) -> Result<(), MapErrorKind> {
        let size = self.size();
        check_i32_fit(size.w, "width")?;
        check_i32_fit(size.h, "height")?;
        if size.w == 0 || size.h == 0 {
            return Err(ImageError::ZeroSized(size).into());
        }
        let pixel_count = u64::from(size.w) * u64::from(size.h);
        check_i32_fit(pixel_count, "pixel count")?;
        let data_size = pixel_count * 4; // RGBA8 image
        check_i32_fit(data_size, "image data size")?;
        match self {
            CompressedData::Compressed(_, len, ImageLoadInfo { size }) => {
                if *len != data_size.unwrapped_as::<usize>() {
                    return Err(ImageError::DataSize {
                        size: *size,
                        len: *len,
                    }
                    .into());
                }
            }
            CompressedData::Loaded(_) => {}
        }
        Ok(())
    }
}

impl InternalMapChecking for Sound {
    const TYPE: MapItem = MapItem::Sound;
    type State = ();

    fn check_impl(&self, map: &TwMap) -> Result<(), MapErrorKind> {
        check_string(&self.name, Sound::MAX_NAME_LENGTH, Some("opus"))?;
        self.data.check_data()?;
        if map.version == Version::Teeworlds07 {
            return Err(TeeworldsError::Sounds.into());
        }
        Ok(())
    }
}
impl CheckData for CompressedData<Vec<u8>, ()> {
    fn check_data(&self) -> Result<(), MapErrorKind> {
        let data_size = match self {
            CompressedData::Compressed(_, size, _) => *size,
            CompressedData::Loaded(buf) => buf.len(),
        };
        check_i32_fit(data_size, "sound data size")?;
        if let CompressedData::Loaded(buf) = self {
            opus_headers::parse_from_read(&buf[..])?;
        }
        Ok(())
    }
}

impl InternalMapChecking for Envelope {
    const TYPE: MapItem = MapItem::Envelope;
    type State = ();

    fn check_impl(&self, map: &TwMap) -> Result<(), MapErrorKind> {
        check_string(self.name(), Envelope::MAX_NAME_LENGTH, None)?;
        let envelope_amount = match self {
            Envelope::Position(env) => env.points.len(),
            Envelope::Color(env) => env.points.len(),
            Envelope::Sound(env) => env.points.len(),
        };
        check_i32_fit(envelope_amount, "env point amount")?;
        if map.version == Version::Teeworlds07 && matches!(self, Envelope::Sound(_)) {
            return Err(TeeworldsError::SoundEnv.into());
        }
        Ok(())
    }

    fn check_recursive_impl(&self, map: &TwMap) -> Result<(), MapErr> {
        match self {
            Envelope::Position(env) => EnvPoint::check_all(&env.points, map),
            Envelope::Color(env) => EnvPoint::check_all(&env.points, map),
            Envelope::Sound(env) => EnvPoint::check_all(&env.points, map),
        }
    }
}

#[derive(Error, Debug)]
pub(crate) enum EnvPointError {
    #[error("Invalid curve kind ({0})")]
    InvalidCurve(i32),
    #[error("Wrong order, the last point was at {last} ms, this on is at {this} ms")]
    Order { this: i32, last: i32 },
}

impl<T> InternalMapChecking for EnvPoint<T> {
    const TYPE: MapItem = MapItem::EnvPoint;
    type State = i32;

    fn check_impl(&self, _: &TwMap) -> Result<(), MapErrorKind> {
        check_non_negative(self.time, "time stamp")?;
        if let CurveKind::Unknown(n) = self.curve {
            return Err(EnvPointError::InvalidCurve(n).into());
        }
        Ok(())
    }

    fn check_state_impl(&self, _: &TwMap, last_time: &mut i32) -> Result<(), MapErrorKind> {
        if self.time < *last_time {
            return Err(EnvPointError::Order {
                this: self.time,
                last: *last_time,
            }
            .into());
        }
        *last_time = self.time;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub(crate) enum GroupError {
    #[error("No physics group")]
    NoPhysicsGroup,
    #[error("There must be only one physics group")]
    SecondPhysicsGroup,
    #[error("No game layer in physics group")]
    NoGameLayer,
    #[error("The physics group '{0}' should be called 'Game' instead")]
    PhysicsName(String),
    #[error("The clipping values of the physics group are changed")]
    PhysicsClip,
    #[error("The parallax values of the physics group are changed")]
    PhysicsParallax,
    #[error("The offset values of the physics group are changed")]
    PhysicsOffset,
}

impl InternalMapChecking for Group {
    const TYPE: MapItem = MapItem::Group;
    /// Represents if a physics group was already found
    type State = bool;

    fn check_impl(&self, _: &TwMap) -> Result<(), MapErrorKind> {
        check_string(&self.name, Group::MAX_NAME_LENGTH, None)?;
        check_i32_fit(self.layers.len(), "layers amount")?;
        check_non_negative(self.clip.w, "clip width")?;
        check_non_negative(self.clip.h, "clip height")?;
        if self.is_physics_group() {
            if !self.layers.iter().any(|l| matches!(l, Layer::Game(_))) {
                return Err(GroupError::NoGameLayer.into());
            }
            let default = Group::physics();
            if self.name != default.name {
                return Err(GroupError::PhysicsName(self.name.clone()).into());
            }
            if self.clipping != default.clipping || self.clip != default.clip {
                return Err(GroupError::PhysicsClip.into());
            }
            if self.offset != default.offset {
                return Err(GroupError::PhysicsOffset.into());
            }
            if self.parallax != default.parallax {
                return Err(GroupError::PhysicsParallax.into());
            }
        }
        Ok(())
    }

    fn check_state_impl(&self, _: &TwMap, has_physics: &mut bool) -> Result<(), MapErrorKind> {
        if self.is_physics_group() {
            if *has_physics {
                return Err(GroupError::SecondPhysicsGroup.into());
            }
            *has_physics = true;
        }
        Ok(())
    }

    fn check_recursive_impl(&self, map: &TwMap) -> Result<(), MapErr> {
        Layer::check_all(&self.layers, map)
    }

    fn check_state(has_physics: bool) -> Result<(), MapErrorKind> {
        if !has_physics {
            return Err(GroupError::NoPhysicsGroup.into());
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub(crate) enum LayerError {
    #[error("Invalid layer kind: {0:?}")]
    InvalidKind(InvalidLayerKind),
    #[error("Width and height must be at least 2")]
    TooSmall,
    #[error("Images used by tiles layers must have width and height be divisible by 16")]
    ImageDimensions,
    #[error("Automapper seed ({0}) must be below 1,000,000,000")]
    AutomapperSeed(u32),
    #[error("Second {0:?} layer")]
    DuplicatePhysics(LayerKind),
    #[error("The physics layers have different shapes")]
    DifferentPhysicsShapes,
    #[error("0.7 compressed tile data length must be a multiple of 4")]
    CompressedSize,
    #[error("The tile data size doesn't match with the layer dimensions")]
    TilesDataSize,
    #[error("0.7 tilemap decompression failed")]
    TeeworldsCompression,
}

impl InternalMapChecking for Layer {
    const TYPE: MapItem = MapItem::Layer;
    type State = (HashSet<LayerKind>, Option<Extent2<usize>>);

    fn check_impl(&self, map: &TwMap) -> Result<(), MapErrorKind> {
        use Layer::*;
        check_string(self.name(), Layer::MAX_NAME_LENGTH, None)?;
        if let Invalid(inv) = self {
            return Err(LayerError::InvalidKind(*inv).into());
        }
        if map.version == Version::Teeworlds07 && !matches!(self, Game(_) | Tiles(_) | Quads(_)) {
            return Err(TeeworldsError::DDNetLayer(self.kind()).into());
        }
        match self {
            Game(l) => l.tiles.check_data()?,
            Front(l) => l.tiles.check_data()?,
            Tele(l) => l.tiles.check_data()?,
            Speedup(l) => l.tiles.check_data()?,
            Switch(l) => l.tiles.check_data()?,
            Tune(l) => l.tiles.check_data()?,
            Tiles(l) => {
                l.tiles.check_data()?;
                map.envelope_index(l.color_env, EnvelopeKind::Color)?;
                map.image_index(l.image)?;
                if let Some(image) = l.image {
                    if !map.images[image.unwrapped_as::<usize>()].for_tilemap() {
                        return Err(LayerError::ImageDimensions.into());
                    }
                }
                if map.version == Version::Teeworlds07
                    && l.automapper_config != AutomapperConfig::default()
                {
                    return Err(TeeworldsError::TilesAutomapper.into());
                }
                if l.automapper_config.seed > 1_000_000_000 {
                    return Err(LayerError::AutomapperSeed(l.automapper_config.seed).into());
                }
            }
            Quads(l) => {
                map.image_index(l.image)?;
                let element_size = mem::size_of::<BinaryQuad>().unwrapped_as::<i32>();
                let max_elements = i32::MAX / element_size;
                check_max(l.quads.len(), max_elements, "quads amount")?;
            }
            Sounds(l) => {
                map.sound_index(l.sound)?;
                let element_size = mem::size_of::<BinarySoundSource>().unwrapped_as::<i32>();
                let max_elements = i32::MAX / element_size;
                check_max(l.sources.len(), max_elements, "sound sources amount")?;
            }
            Invalid(_) => {}
        }
        Ok(())
    }

    fn check_state_impl(&self, _: &TwMap, state: &mut Self::State) -> Result<(), MapErrorKind> {
        let kind = self.kind();
        if !kind.is_physics_layer() {
            return Ok(());
        }
        let (layers, expected_shape) = state;
        if layers.replace(kind).is_some() {
            return Err(LayerError::DuplicatePhysics(kind).into());
        }
        let shape = self.shape().unwrap();
        match expected_shape {
            None => *expected_shape = Some(shape),
            Some(expected) => {
                if *expected != shape {
                    return Err(LayerError::DifferentPhysicsShapes.into());
                }
            }
        }
        Ok(())
    }

    fn check_recursive_impl(&self, map: &TwMap) -> Result<(), MapErr> {
        match self {
            Layer::Quads(l) => Quad::check_all(&l.quads, map),
            Layer::Sounds(l) => SoundSource::check_all(&l.sources, map),
            _ => Ok(()),
        }
    }
}

impl InternalMapChecking for Quad {
    const TYPE: MapItem = MapItem::Quad;
    type State = ();

    fn check_impl(&self, map: &TwMap) -> Result<(), MapErrorKind> {
        map.envelope_index(self.position_env, EnvelopeKind::Position)?;
        map.envelope_index(self.color_env, EnvelopeKind::Color)?;
        Ok(())
    }
}

impl InternalMapChecking for SoundSource {
    const TYPE: MapItem = MapItem::SoundSource;
    type State = ();

    fn check_impl(&self, map: &TwMap) -> Result<(), MapErrorKind> {
        check_non_negative(self.delay, "delay")?;
        map.envelope_index(self.position_env, EnvelopeKind::Position)?;
        map.envelope_index(self.sound_env, EnvelopeKind::Sound)?;
        match self.area {
            SoundArea::Rectangle(rect) => {
                check_non_negative(rect.w, "area width")?;
                check_non_negative(rect.h, "area height")?;
            }
            SoundArea::Circle(disk) => check_non_negative(disk.radius, "area radius")?,
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
#[error("Tile at x: {x}, y: {y} - {err}")]
pub(crate) struct TileError {
    x: usize,
    y: usize,
    err: TileErrorKind,
}

#[derive(Error, Debug)]
pub enum TileErrorKind {
    #[error("Skip byte of tile is {0} instead of zero")]
    TileSkip(u8),
    #[error("Unused byte of tile is {0} instead of zero")]
    TileUnused(u8),
    #[error("Unknown tile flags used, flags: {:#010b}", .0)]
    UnknownTileFlags(u8),
    #[error("Opaque tile flag used in physics layer")]
    OpaqueTileFlag,
    #[error("Unused byte of speedup is {0} instead of zero")]
    SpeedupUnused(u8),
    #[error("Angle of speedup is {0}, but should be between 0 and (exclusive) 360")]
    SpeedupAngle(i16),
}

pub trait TileChecking {
    fn check(&self) -> Result<(), TileErrorKind> {
        Ok(())
    }
}

impl TileFlags {
    fn check(self) -> bool {
        TileFlags::from_bits(self.bits()).is_some()
    }
}

impl TileChecking for Tile {
    fn check(&self) -> Result<(), TileErrorKind> {
        use TileErrorKind::*;
        if self.skip != 0 {
            return Err(TileSkip(self.skip));
        }
        if self.unused != 0 {
            return Err(TileUnused(self.unused));
        }
        if !self.flags.check() {
            return Err(UnknownTileFlags(self.flags.bits()));
        }
        Ok(())
    }
}

impl TileChecking for GameTile {
    fn check(&self) -> Result<(), TileErrorKind> {
        use TileErrorKind::*;
        if self.skip != 0 {
            return Err(TileSkip(self.skip));
        }
        if self.unused != 0 {
            return Err(TileUnused(self.unused));
        }
        if !self.flags.check() {
            return Err(UnknownTileFlags(self.flags.bits()));
        }
        if self.flags.contains(TileFlags::OPAQUE) {
            return Err(OpaqueTileFlag);
        }
        Ok(())
    }
}

impl TileChecking for Tele {}

impl TileChecking for Switch {
    fn check(&self) -> Result<(), TileErrorKind> {
        use TileErrorKind::*;
        if !self.flags.check() {
            return Err(UnknownTileFlags(self.flags.bits()));
        }
        if self.flags.contains(TileFlags::OPAQUE) {
            return Err(OpaqueTileFlag);
        }
        Ok(())
    }
}

impl TileChecking for Speedup {
    fn check(&self) -> Result<(), TileErrorKind> {
        use TileErrorKind::*;
        if self.unused_padding != 0 {
            return Err(SpeedupUnused(self.unused_padding));
        }
        let angle = i16::from(self.angle);
        if !(0..360).contains(&angle) {
            return Err(SpeedupAngle(angle));
        }
        Ok(())
    }
}

impl TileChecking for Tune {}

impl<T: TileChecking> CheckData for CompressedData<Array2<T>, TilesLoadInfo> {
    fn check_data(&self) -> Result<(), MapErrorKind> {
        let size = self.shape();
        check_i32_fit(size.w, "width")?;
        check_i32_fit(size.h, "height")?;
        let tile_count = size.w.unwrapped_as::<u64>() * size.h.unwrapped_as::<u64>();
        check_i32_fit(tile_count, "tile count")?;
        let tile_size = mem::size_of::<T>().unwrapped_as::<u64>();
        let tile_data_size = tile_count * tile_size;
        check_i32_fit(tile_data_size, "tilemap data size")?;
        if size.w < 2 || size.h < 2 {
            return Err(LayerError::TooSmall.into());
        }
        match self {
            CompressedData::Loaded(tiles) => {
                for ((y, x), tile) in tiles.indexed_iter() {
                    tile.check().map_err(|err| TileError { x, y, err })?;
                }
            }
            CompressedData::Compressed(_, data_size, info) => {
                if info.compression {
                    if data_size % 4 != 0 {
                        return Err(LayerError::CompressedSize.into());
                    }
                } else if tile_data_size.unwrapped_as::<usize>() != *data_size {
                    return Err(LayerError::TilesDataSize.into());
                }
            }
        }
        Ok(())
    }
}
