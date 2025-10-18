use crate::datafile;

use bitflags::bitflags;
use fixed::types::{I17F15, I22F10, I27F5};
use image::RgbaImage;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use structview::View;
use thiserror::Error;
use vek::{Disk, Extent2, Rect, Rgba, Uv, Vec2};

use std::io;

mod checks;
/// Complex methods for map structs
pub mod edit;
/// Simple impl blocks for map structs and traits
mod impls;
mod load;
/// MapDir format implementation
mod map_dir;
mod parse;
mod save;

pub use checks::MapError;
pub use load::{Load, LoadMultiple};
pub use map_dir::MapDirParseError;
pub use parse::MapParseError;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Version {
    DDNet06,
    Teeworlds07,
}

/// The TwMap struct represents a Teeworlds 0.7 map or a DDNet 0.6 map.
/// Which one of those it is will always be determined during the parsing process and is stored in the `version` field.
///
/// The library cares a lot about the integrity of the struct.
/// The [`check`](struct.TwMap.html#method.check) method verifies that all limitations are met.
///
/// # Parsing
///
/// TwMap has several different parsing methods: [`parse`](struct.TwMap.html#method.parse), [`parse_path`](struct.TwMap.html#method.parse_path), [`parse_file`](struct.TwMap.html#method.parse_file), [`parse_dir`](struct.TwMap.html#method.parse_dir), [`parse_datafile`](struct.TwMap.html#method.parse_datafile)
///
/// Each of them execute the [`check`](struct.TwMap.html#method.check) method to finalize the process.
///
/// If you want to leave out the checks, you can use the `_unchecked` variation of that parsing method if it is provided.
/// Note that the `_unchecked` variation might also exclude some common fixes.
///
/// # Saving
///
/// TwMap can save maps in the binary format ([`save`](struct.TwMap.html#method.save), [`save_file`](struct.TwMap.html#method.save_file)) and in the MapDir format ([`save_dir`](struct.TwMap.html#method.save_dir)).
///
/// Each saving method will first execute [`check`](struct.TwMap.html#method.check), if the map fails the check, it will not be saved.
///
/// # Loading
///
/// When loading a map from the binary format, a lot of data will be decompressed in the process.
/// Since this is the main slowdown factor, some larger data chunks will be left compressed.
/// The compressed parts are the `data` field in [`Image`](struct.Image.html)s and [`Sound`](struct.Sound.html)s and the `tiles` field in tilemap layers.
/// If you want to save the map at the end anyways, then you can simply use the [`load`](struct.TwMap.html#method.load) method on the entire map.
/// If not, use the `load` method only on the images, sounds and tilemap layers that you want to use.
///
/// **Note:**
/// - you can also use `load` on slices and vectors of Images, Sounds, Layers, Groups with layers,
/// - some methods rely on having parts of the map loaded, especially more abstract methods like [`mirror`](struct.TwMap.html#method.mirror) and [`rotate_right`](struct.TwMap.html#method.rotate_right)
/// - if you want to leave out the checks on the decompressed data, you can use the `load_unchecked` methods
///
/// # Fixed Point Integers
///
/// In many parts of the struct fixed point integers are used.
/// They are represented using the crate `fixed`.
/// Position and size fixed point integers are always chosen so that 1 always translates to the width of 1 tile.
/// The other usages are usually chosen so that 0 = 0%, 1 = 100%.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TwMap {
    /// Determines what version this map should be saved as
    pub version: Version,
    /// Metadata
    pub info: Info,
    /// Textures
    pub images: Vec<Image>,
    /// Animation strips
    pub envelopes: Vec<Envelope>,
    /// Collections of layers
    pub groups: Vec<Group>,
    /// Sfx (not available in Teeworlds07 maps)
    pub sounds: Vec<Sound>,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("Map - {0}")]
    Map(#[from] MapError),
    #[error("Map from Datafile - {0}")]
    MapParse(#[from] parse::MapParseError),
    #[error("Datafile saving - {0}")]
    DatafileSave(#[from] datafile::DatafileSaveError),
    #[error("Datafile parsing - {0}")]
    DatafileParse(#[from] datafile::DatafileParseError),
    #[error("IO - {0}")]
    Io(#[from] io::Error),
    #[error("MapDir - {0}")]
    MapDirParse(#[from] map_dir::MapDirParseError),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
// identifier for all relevant types of layers
pub enum LayerKind {
    Game,
    Tiles,
    Quads,
    Front,
    Tele,
    Speedup,
    Switch,
    Tune,
    Sounds,
    Invalid(InvalidLayerKind),
}

#[derive(Debug, Eq, PartialOrd, PartialEq, Copy, Clone, Hash)]
pub enum InvalidLayerKind {
    Unknown(i32),        // unknown value of 'LAYERTYPE' identifier
    UnknownTilemap(i32), // 'LAYERTYPE' identified a tile layer, unknown value of 'TILESLAYERFLAG' identifier
    NoType,              // layer item too short to get 'LAYERTYPE' identifier
    NoTypeTilemap, // 'LAYERTYPE' identified a tile layer, layer item too short to get 'TILESLAYERFLAG' identifier
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
/// Holds compressed data. Use [`Load`] or [`LoadMultiple`] to decompress this.
/// Too construct with the already loaded data, simply use `CompressedData::from`.
pub enum CompressedData<T, U> {
    Compressed(Vec<u8>, usize, U),
    Loaded(T),
}

/// Note that all strings have size limits, check the constants associated with this struct.
#[derive(Debug, Default, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Info {
    pub author: String,
    pub version: String,
    pub credits: String,
    pub license: String,
    pub settings: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Image {
    /// Image data should be shipped with the client.
    External(ExternalImage),
    /// Image data stored in map file.
    Embedded(EmbeddedImage),
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct ExternalImage {
    /// The name with the file extension `.png` is considered a file name and must be sanitized.
    #[serde(skip)]
    pub name: String,

    pub size: Extent2<u32>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct ImageLoadInfo {
    pub size: Extent2<u32>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct EmbeddedImage {
    /// The name with the file extension `.png` is considered a file name and must be sanitized.
    pub name: String,

    pub image: CompressedData<RgbaImage, ImageLoadInfo>, // None if the image is external (not embedded)
}

const PARALLAX_DIVISOR: Vec2<i32> = Vec2::new(100, 100);

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    /// Offsets the layers from the upper left corner of the map (in tiles)
    pub offset: Vec2<I27F5>,
    /// How fast this group moves relative to the physics group (100, 100) is the same speed, (0, 0) is static.
    pub parallax: Vec2<i32>,
    #[serde(skip)]
    pub layers: Vec<Layer>,
    /// Enables the clip.
    pub clipping: bool,
    /// Defines a rectangle outside of which the group is invisible.
    /// The rectangle is not affected by parallax or offset.
    pub clip: Rect<I27F5, I27F5>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Sound {
    /// Translates directly to a filename ({name}.opus).
    /// The file name must be sanitized.
    pub name: String,
    pub data: CompressedData<Vec<u8>, ()>,
}

#[derive(Default, Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct AutomapperConfig {
    /// Automappers can have multiple configs to choose from.
    /// This selects the config with a simple index.
    pub config: Option<u16>,
    /// 0 => random seed.
    pub seed: u32,
    /// Whether the editor should run the automapper after every change automatically.
    pub automatic: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct BezierCurve<T> {
    pub handle_l: Vec2<T>,
    pub handle_r: Vec2<T>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "type")]
/// Belongs to a envelope point.
/// Describes the change in value from this point to the next.
///
/// For math: `frac` is where the interpolate between the two points (0 is at first point, 1 at second, 0.5 halfway),
/// The resulting value then mixes the two values: `left + (right - left) * result`.
pub enum CurveKind<T> {
    /// Value of first point until second point, abrupt change there.
    Step,
    /// Linear interpolation between the two points.
    /// Math: `frac`
    #[default]
    Linear,
    /// First slow, later much faster value change.
    /// Math: `frac^3`
    Slow,
    /// First fast, later much slower value change.
    /// Math: `1 - (1 - frac)^3`
    Fast,
    /// Slow, faster then once more slow value change.
    /// Math: `3 * frac^2 - 2 * frac^3`
    Smooth,
    /// Very flexible curve, each channel individually.
    /// Uses bezier curves for interpolation.
    /// Only kind of curve where every channel gets their own interpolation values.
    Bezier(BezierCurve<T>),
    /// For compatibility, will error on check.
    Unknown(i32),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct EnvPoint<T> {
    /// Time stamp of this point in milliseconds, must not be negative.
    pub time: i32,
    pub content: T,
    /// Interpolation mode between this point and the next one.
    #[serde(flatten)]
    pub curve: CurveKind<T>,
}

/// Default bezier tangent values are all zeroes.
/// This trait implements the constructors for those values.
/// only I32Color needs to implement this manually, since volume (i32) and Position are already all zeroes in their default values.
pub trait BezierDefault: Default {
    fn bezier_default() -> Self {
        Default::default()
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    #[serde(flatten)]
    pub offset: Vec2<I17F15>,
    /// Rotation in degrees. 90 would be a 90° clockwise rotation
    pub rotation: I22F10,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct Volume(pub I22F10);

#[derive(Default, Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Env<T: Copy> {
    pub name: String,
    /// Whether the envelope should be synchronized across clients by using server time.
    /// Will use client time otherwise
    pub synchronized: bool,
    /// Must be in chronological order based on their time
    pub points: Vec<EnvPoint<T>>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Envelope {
    Position(Env<Position>),
    Color(Env<Rgba<I22F10>>),
    /// Not available in Teeworlds07 maps.
    Sound(Env<Volume>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TilesLoadInfo {
    pub size: Extent2<u32>,
    pub compression: bool,
}

unsafe impl View for TileFlags {} // deriving directly didn't work
bitflags! {
    /// These bitflags are found in many of the tile types found in tilemap layers.
    /// Note that the naming of the flags is very confusing and it is almost **never** a good idea to edit them directly.
    /// Use the helper methods instead, as they will have much more expected behavior!
    ///
    /// The big issue is that while [`Tile`]s from [`TilesLayer`]s have 8 valid orientations,
    /// directional tiles from physics layers (that contain `TileFlag`s) only have 4 valid: Up, Down Left and Right.
    /// Up is the default orientation of a physics tile.
    /// While this could've been no issue, directional physics tiles actually have broken physics when in another state.
    #[repr(C)]
    #[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, Hash, Eq, PartialEq)]
    #[serde(into = "map_dir::DirTileFlags", try_from = "map_dir::DirTileFlags")]
    pub struct TileFlags: u8 {
        /// Mirrors the tile horizontally, switching Left <-> Right.
        ///
        /// Formerly, this flag was called `FLIP_V`.
        const FLIP_X = 0b0001;
        /// Mirrors the tile vertically, switching Top <-> Bottom.
        ///
        /// Formerly, this flag was called `FLIP_V`.
        const FLIP_Y = 0b0010;
        /// Hints that this tile has no (significant) transparency.
        const OPAQUE = 0b0100;
        /// Rotates the tile clockwise.
        /// The rotation happens **after** the flip operations!
        const ROTATE = 0b1000;
    }
}

#[derive(Debug, Copy, Clone, View, Eq, PartialEq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Tile {
    // for TilesLayer
    pub id: u8,
    #[serde(flatten)]
    pub flags: TileFlags,
    #[serde(skip)]
    pub(crate) skip: u8, // used for 0.7 tile compression
    #[serde(skip)]
    pub(crate) unused: u8,
}

#[derive(Debug, Copy, Clone, View, Eq, PartialEq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct GameTile {
    // for GameLayer and FrontLayer
    pub id: u8,
    #[serde(flatten)]
    pub flags: TileFlags,
    #[serde(skip)]
    pub(crate) skip: u8, // used for 0.7 tile compression
    #[serde(skip)]
    pub(crate) unused: u8,
}

#[derive(Debug, Copy, Clone, View, Eq, PartialEq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Tele {
    pub number: u8,
    pub id: u8,
}

#[derive(Debug, Copy, Clone, View, Eq, PartialEq, Default, Serialize, Deserialize)]
#[repr(C)]
#[serde(into = "i16", from = "i16")]
/// Required to make the Speedup tile struct 1-byte-aligned
pub struct I16 {
    pub(crate) bytes: [u8; 2],
}

#[derive(Debug, Copy, Clone, View, Eq, PartialEq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Speedup {
    pub force: u8,
    pub max_speed: u8,
    pub id: u8,
    #[serde(skip)]
    pub(crate) unused_padding: u8,
    /// Angle, value between 0 and (exclusive) 360
    /// 0 points right, angle increases clock-wise
    pub angle: I16,
}

#[derive(Debug, Copy, Clone, View, Eq, PartialEq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Switch {
    pub number: u8,
    pub id: u8,
    #[serde(flatten)]
    pub flags: TileFlags,
    pub delay: u8,
}

#[derive(Debug, Copy, Clone, View, Eq, PartialEq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Tune {
    pub number: u8,
    pub id: u8,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct GameLayer {
    #[serde(flatten, with = "map_dir::tiles_serialization")]
    pub tiles: CompressedData<Array2<GameTile>, TilesLoadInfo>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct FrontLayer {
    #[serde(flatten, with = "map_dir::tiles_serialization")]
    pub tiles: CompressedData<Array2<GameTile>, TilesLoadInfo>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct TeleLayer {
    #[serde(flatten, with = "map_dir::tiles_serialization")]
    pub tiles: CompressedData<Array2<Tele>, TilesLoadInfo>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct SpeedupLayer {
    #[serde(flatten, with = "map_dir::tiles_serialization")]
    pub tiles: CompressedData<Array2<Speedup>, TilesLoadInfo>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct SwitchLayer {
    #[serde(flatten, with = "map_dir::tiles_serialization")]
    pub tiles: CompressedData<Array2<Switch>, TilesLoadInfo>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct TuneLayer {
    #[serde(flatten, with = "map_dir::tiles_serialization")]
    pub tiles: CompressedData<Array2<Tune>, TilesLoadInfo>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct TilesLayer {
    pub name: String,
    pub detail: bool,
    pub color: Rgba<u8>,
    #[serde(with = "map_dir::envelope_index_serialization")]
    pub color_env: Option<u16>,
    pub color_env_offset: i32,
    #[serde(with = "map_dir::image_index_serialization")]
    pub image: Option<u16>,
    #[serde(flatten, with = "map_dir::tiles_serialization")]
    pub tiles: CompressedData<Array2<Tile>, TilesLoadInfo>,
    pub automapper_config: AutomapperConfig,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Quad {
    /// Position of the 4 corners (top-left -> top-right -> bottom-left -> bottom-right).
    /// The positions are **NOT** relative to the position of the quad itself.
    pub corners: [Vec2<I17F15>; 4],
    /// Position of the quad, around which it rotates.
    pub position: Vec2<I17F15>,
    pub colors: [Rgba<u8>; 4],
    pub texture_coords: [Uv<I22F10>; 4], // represents the stretching done by shift+dragging of corners in the editor
    #[serde(with = "map_dir::envelope_index_serialization")]
    pub position_env: Option<u16>,
    pub position_env_offset: i32,
    #[serde(with = "map_dir::envelope_index_serialization")]
    pub color_env: Option<u16>,
    pub color_env_offset: i32,
}

#[derive(Default, Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct QuadsLayer {
    pub name: String,
    pub detail: bool,
    pub quads: Vec<Quad>,
    #[serde(with = "map_dir::image_index_serialization")]
    pub image: Option<u16>, // index to its image
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SoundArea {
    Rectangle(Rect<I17F15, I17F15>),
    Circle(Disk<I17F15, I27F5>),
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct SoundSource {
    pub area: SoundArea,
    pub looping: bool,
    pub panning: bool,
    pub delay: i32,
    pub falloff: u8,
    #[serde(with = "map_dir::envelope_index_serialization")]
    pub position_env: Option<u16>,
    pub position_env_offset: i32,
    #[serde(with = "map_dir::envelope_index_serialization")]
    pub sound_env: Option<u16>,
    pub sound_env_offset: i32,
}

#[derive(Default, Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct SoundsLayer {
    pub name: String,
    pub detail: bool,
    pub sources: Vec<SoundSource>,
    #[serde(with = "map_dir::sound_index_serialization")]
    pub sound: Option<u16>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Layer {
    Game(GameLayer),
    Tiles(TilesLayer),
    Quads(QuadsLayer),
    Front(FrontLayer),
    Tele(TeleLayer),
    Speedup(SpeedupLayer),
    Switch(SwitchLayer),
    Tune(TuneLayer),
    Sounds(SoundsLayer),
    #[serde(skip)]
    Invalid(InvalidLayerKind),
}

pub trait TilemapLayer: AnyLayer {
    type TileType: AnyTile;

    fn tiles(&self) -> &CompressedData<Array2<Self::TileType>, TilesLoadInfo>;

    fn tiles_mut(&mut self) -> &mut CompressedData<Array2<Self::TileType>, TilesLoadInfo>;
}

pub trait AnyTile: Default + PartialEq + Copy + Clone + checks::TileChecking + View {
    fn id(&self) -> u8;

    fn id_mut(&mut self) -> &mut u8;

    fn flags(&self) -> Option<TileFlags>;

    fn flags_mut(&mut self) -> Option<&mut TileFlags>;
}

pub trait AnyLayer: Sized {
    fn kind() -> LayerKind;

    fn get(layer: &Layer) -> Option<&Self>;

    fn get_mut(layer: &mut Layer) -> Option<&mut Self>;
}

/// Marker trait, implemented for all physics layers
pub trait PhysicsLayer: TilemapLayer {}
