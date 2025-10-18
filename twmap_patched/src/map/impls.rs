use crate::map::*;

use az::CheckedCast;
use fixed::traits::ToFixed;
use fixed::types::{I17F15, I22F10, I27F5};
use image::RgbaImage;
use ndarray::Array2;
use vek::num_traits::{CheckedAdd, CheckedDiv, CheckedMul, One};
use vek::{Extent2, Rgba, Uv};

impl From<twstorage::Version> for Version {
    fn from(value: twstorage::Version) -> Self {
        match value {
            twstorage::Version::DDNet06 => Version::DDNet06,
            twstorage::Version::Teeworlds07 => Version::Teeworlds07,
        }
    }
}

impl From<Version> for twstorage::Version {
    fn from(value: Version) -> Self {
        match value {
            Version::DDNet06 => twstorage::Version::DDNet06,
            Version::Teeworlds07 => twstorage::Version::Teeworlds07,
        }
    }
}

impl Info {
    pub const MAX_AUTHOR_LENGTH: usize = 31;
    pub const MAX_VERSION_LENGTH: usize = 15;
    pub const MAX_CREDITS_LENGTH: usize = 127;
    pub const MAX_LICENSE_LENGTH: usize = 31;
    pub const MAX_SETTING_LENGTH: usize = 255;
}

impl Image {
    pub const MAX_NAME_LENGTH: usize = 127;
}

impl Envelope {
    pub const MAX_NAME_LENGTH: usize = 31;
}

impl Group {
    pub const MAX_NAME_LENGTH: usize = 11;
}

impl Layer {
    pub const MAX_NAME_LENGTH: usize = 11;
}

impl Sound {
    pub const MAX_NAME_LENGTH: usize = 127;
}

impl TwMap {
    /// Returns a empty map struct with only the version set.
    pub fn empty(version: Version) -> TwMap {
        TwMap {
            version,
            info: Info::default(),
            images: vec![],
            envelopes: vec![],
            groups: vec![],
            sounds: vec![],
        }
    }

    /// Returns a reference to the physics group.
    pub fn physics_group(&self) -> &Group {
        self.groups
            .iter()
            .find(|group| group.is_physics_group())
            .unwrap()
    }

    /// Returns a mutable reference to the physics group.
    pub fn physics_group_mut(&mut self) -> &mut Group {
        self.groups
            .iter_mut()
            .find(|group| group.is_physics_group())
            .unwrap()
    }
}

impl Group {
    /// Checks if the group contains any physics layers.
    pub fn is_physics_group(&self) -> bool {
        self.layers
            .iter()
            .any(|layer| layer.kind().is_physics_layer())
    }
}

impl From<ExternalImage> for Image {
    fn from(image: ExternalImage) -> Self {
        Image::External(image)
    }
}

impl From<EmbeddedImage> for Image {
    fn from(image: EmbeddedImage) -> Self {
        Image::Embedded(image)
    }
}

impl Image {
    pub const fn name(&self) -> &String {
        match self {
            Image::External(img) => &img.name,
            Image::Embedded(img) => &img.name,
        }
    }

    pub fn name_mut(&mut self) -> &mut String {
        match self {
            Image::External(img) => &mut img.name,
            Image::Embedded(img) => &mut img.name,
        }
    }

    pub fn size(&self) -> Extent2<u32> {
        match self {
            Image::External(image) => image.size,
            Image::Embedded(image) => image.image.size(),
        }
    }

    pub const fn image(&self) -> Option<&CompressedData<RgbaImage, ImageLoadInfo>> {
        match self {
            Image::External(_) => None,
            Image::Embedded(image) => Some(&image.image),
        }
    }

    pub fn image_mut(&mut self) -> Option<&mut CompressedData<RgbaImage, ImageLoadInfo>> {
        match self {
            Image::External(_) => None,
            Image::Embedded(image) => Some(&mut image.image),
        }
    }

    pub fn for_tilemap(&self) -> bool {
        let size = self.size();
        size.w % 16 == 0 && size.h % 16 == 0
    }
}

impl CompressedData<RgbaImage, ImageLoadInfo> {
    pub(crate) fn size(&self) -> Extent2<u32> {
        match self {
            CompressedData::Compressed(_, _, info) => info.size,
            CompressedData::Loaded(image) => Extent2::new(image.width(), image.height()),
        }
    }
}

impl LayerKind {
    pub(crate) const fn static_name(&self) -> &'static str {
        use LayerKind::*;
        match self {
            Game => "Game",
            Front => "Front",
            Tele => "Tele",
            Speedup => "Speedup",
            Switch => "Switch",
            Tune => "Tune",
            _ => panic!(),
        }
    }

    pub const fn is_physics_layer(&self) -> bool {
        use LayerKind::*;
        match self {
            Game | Front | Tele | Speedup | Switch | Tune => true,
            Tiles | Quads | Sounds | Invalid(_) => false,
        }
    }

    pub const fn is_tile_map_layer(&self) -> bool {
        use LayerKind::*;
        match self {
            Game | Tiles | Front | Tele | Speedup | Switch | Tune => true,
            Quads | Sounds | Invalid(_) => false,
        }
    }
}

impl<T> CompressedData<Array2<T>, TilesLoadInfo> {
    /// Returns the (height, width) 2-dimensional array of tiles.
    pub fn shape(&self) -> Extent2<usize> {
        match self {
            CompressedData::Compressed(_, _, info) => info.size.numcast().unwrap(),
            CompressedData::Loaded(array) => Extent2::new(array.ncols(), array.nrows()),
        }
    }
}

impl Layer {
    /// Returns an identifier for the type of the layer.
    pub const fn kind(&self) -> LayerKind {
        use Layer::*;
        match self {
            Game(_) => LayerKind::Game,
            Tiles(_) => LayerKind::Tiles,
            Quads(_) => LayerKind::Quads,
            Front(_) => LayerKind::Front,
            Tele(_) => LayerKind::Tele,
            Speedup(_) => LayerKind::Speedup,
            Switch(_) => LayerKind::Switch,
            Tune(_) => LayerKind::Tune,
            Sounds(_) => LayerKind::Sounds,
            Invalid(kind) => LayerKind::Invalid(*kind),
        }
    }

    /// Returns the (height, width) of the 2-dimensional array of tiles, if the contained layer is a tilemap layer.
    pub fn shape(&self) -> Option<Extent2<usize>> {
        use Layer::*;
        match self {
            Game(l) => Some(l.tiles().shape()),
            Tiles(l) => Some(l.tiles().shape()),
            Quads(_) => None,
            Front(l) => Some(l.tiles().shape()),
            Tele(l) => Some(l.tiles().shape()),
            Speedup(l) => Some(l.tiles().shape()),
            Switch(l) => Some(l.tiles().shape()),
            Tune(l) => Some(l.tiles().shape()),
            Sounds(_) => None,
            Invalid(_) => None,
        }
    }
}

impl Quad {
    /// Creates a rectangle quad at the specified position and size.
    /// Except position and size, it uses the default values of a newly created quad in the editor.
    pub fn new(position: Vec2<I17F15>, size: Extent2<I17F15>) -> Option<Self> {
        let two: Extent2<I17F15> = Extent2::broadcast(2.to_fixed::<I17F15>());
        let size_halfed: Vec2<I17F15> = size.checked_div(&two)?.into();
        let add = I17F15::one();
        let sub = -add;
        Some(Quad {
            corners: [
                position.checked_add(&size_halfed.checked_mul(&(sub, sub).into())?)?,
                position.checked_add(&size_halfed.checked_mul(&(add, sub).into())?)?,
                position.checked_add(&size_halfed.checked_mul(&(sub, add).into())?)?,
                position.checked_add(&size_halfed.checked_mul(&(add, add).into())?)?,
            ],
            position: Vec2::zero(),
            colors: [Rgba::white(); 4],
            texture_coords: [
                Uv::new(0, 0).checked_cast()?,
                Uv::new(1, 0).checked_cast()?,
                Uv::new(0, 1).checked_cast()?,
                Uv::new(1, 1).checked_cast()?,
            ],
            position_env: None,
            position_env_offset: 0,
            color_env: None,
            color_env_offset: 0,
        })
    }
}

impl Default for Quad {
    /// Default settings of a newly created quad in the editor
    fn default() -> Self {
        Quad::new(Vec2::zero(), Extent2::broadcast(I17F15::from_num(2))).unwrap()
    }
}

impl SoundArea {
    pub fn position(&self) -> Vec2<I17F15> {
        match self {
            SoundArea::Rectangle(rekt) => rekt.position(),
            SoundArea::Circle(disk) => disk.center,
        }
    }

    pub fn set_position(&mut self, position: Vec2<I17F15>) {
        match self {
            SoundArea::Rectangle(rekt) => rekt.set_position(position),
            SoundArea::Circle(disk) => disk.center = position,
        }
    }
}

impl Default for SoundSource {
    /// Default settings of a newly created sound source in the editor
    fn default() -> Self {
        SoundSource {
            area: SoundArea::Circle(Disk::new(Vec2::zero(), I27F5::from_bits(1500))),
            looping: true,
            panning: true,
            delay: 0,
            falloff: 80,
            position_env: None,
            position_env_offset: 0,
            sound_env: None,
            sound_env_offset: 0,
        }
    }
}

impl<T, U> From<T> for CompressedData<T, U> {
    fn from(data: T) -> Self {
        CompressedData::Loaded(data)
    }
}

impl<T, U> CompressedData<T, U> {
    /// Returns a reference to the inner loaded value. Panics if isn't loaded.
    pub const fn unwrap_ref(&self) -> &T {
        match self {
            CompressedData::Compressed(_, _, _) => {
                panic!("Data is still compressed, reference unwrap unsuccessful")
            }
            CompressedData::Loaded(data) => data,
        }
    }

    /// Returns a mutable reference to the inner loaded value. Panics if isn't loaded.
    pub fn unwrap_mut(&mut self) -> &mut T {
        match self {
            CompressedData::Compressed(_, _, _) => {
                panic!("Data is still compressed, mut reference unwrap unsuccessful")
            }
            CompressedData::Loaded(data) => data,
        }
    }

    /// Returns the inner loaded value. Panics if isn't loaded.
    pub fn unwrap(self) -> T {
        match self {
            CompressedData::Compressed(_, _, _) => {
                panic!("Data is still compressed, unwrap unsuccessful")
            }
            CompressedData::Loaded(data) => data,
        }
    }
}

impl Envelope {
    /// Returns a reference to the name of the envelope.
    pub const fn name(&self) -> &String {
        use Envelope::*;
        match self {
            Position(env) => &env.name,
            Color(env) => &env.name,
            Sound(env) => &env.name,
        }
    }

    /// Returns a mutable reference to the name of the envelope.
    pub fn name_mut(&mut self) -> &mut String {
        use Envelope::*;
        match self {
            Position(env) => &mut env.name,
            Color(env) => &mut env.name,
            Sound(env) => &mut env.name,
        }
    }
}

impl Default for Group {
    fn default() -> Self {
        Group {
            offset: Vec2::zero(),
            parallax: Vec2::broadcast(100),
            layers: Vec::new(),
            clipping: false,
            clip: Rect::default(),
            name: String::new(),
        }
    }
}

impl Group {
    /// Constructor for the physics group, make sure not to change any values apart from the layers
    pub fn physics() -> Self {
        Group {
            name: String::from("Game"),
            ..Group::default()
        }
    }
}

impl TilesLayer {
    /// Creates a tiles layer with the default values and the specified dimensions.
    pub fn new(shape: (usize, usize)) -> Self {
        TilesLayer {
            detail: false,
            color: Rgba::white(),
            color_env: None,
            color_env_offset: 0,
            image: None,
            tiles: Array2::default(shape).into(),
            name: "".to_string(),
            automapper_config: AutomapperConfig::default(),
        }
    }
}

impl Layer {
    /// Returns a reference to the name of the layer.
    pub fn name(&self) -> &str {
        use Layer::*;
        match self {
            Game(_) => LayerKind::Game.static_name(),
            Tiles(l) => &l.name,
            Quads(l) => &l.name,
            Front(_) => LayerKind::Front.static_name(),
            Tele(_) => LayerKind::Tele.static_name(),
            Speedup(_) => LayerKind::Speedup.static_name(),
            Switch(_) => LayerKind::Switch.static_name(),
            Tune(_) => LayerKind::Tune.static_name(),
            Sounds(l) => &l.name,
            Invalid(_) => panic!(),
        }
    }

    /// Returns a mutable reference to the name of the layer, if the layer type supports an editable name.
    pub fn name_mut(&mut self) -> Option<&mut String> {
        use Layer::*;
        match self {
            Game(_) => None,
            Tiles(l) => Some(&mut l.name),
            Quads(l) => Some(&mut l.name),
            Front(_) => None,
            Tele(_) => None,
            Speedup(_) => None,
            Switch(_) => None,
            Tune(_) => None,
            Sounds(l) => Some(&mut l.name),
            Invalid(_) => panic!(),
        }
    }
}

impl Tile {
    /// Constructor, required due to unused bytes which will be set to 0.
    pub const fn new(id: u8, flags: TileFlags) -> Self {
        Tile {
            id,
            flags,
            skip: 0,
            unused: 0,
        }
    }
}

impl GameTile {
    /// Constructor, required due to unused bytes which will be set to 0.
    pub const fn new(id: u8, flags: TileFlags) -> Self {
        GameTile {
            id,
            flags,
            skip: 0,
            unused: 0,
        }
    }
}

impl Speedup {
    /// Constructor, required due to unused bytes which will be set to 0.
    pub fn new(id: u8, force: u8, max_speed: u8, angle: i16) -> Self {
        Speedup {
            force,
            max_speed,
            id,
            unused_padding: 0,
            angle: angle.into(),
        }
    }
}

impl From<i16> for I16 {
    fn from(x: i16) -> Self {
        I16 {
            bytes: x.to_le_bytes(),
        }
    }
}

impl From<I16> for i16 {
    fn from(x: I16) -> Self {
        i16::from_le_bytes(x.bytes)
    }
}

impl BezierDefault for Rgba<I22F10> {
    fn bezier_default() -> Self {
        Rgba::from(I22F10::from_bits(0))
    }
}

impl BezierDefault for Position {}

impl Default for Volume {
    fn default() -> Self {
        Volume(I22F10::from_num(1))
    }
}

impl BezierDefault for Volume {
    fn bezier_default() -> Self {
        Self(I22F10::from_num(0))
    }
}

impl TilemapLayer for TilesLayer {
    type TileType = Tile;

    fn tiles(&self) -> &CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &self.tiles
    }

    fn tiles_mut(&mut self) -> &mut CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &mut self.tiles
    }
}

impl TilemapLayer for GameLayer {
    type TileType = GameTile;

    fn tiles(&self) -> &CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &self.tiles
    }

    fn tiles_mut(&mut self) -> &mut CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &mut self.tiles
    }
}

impl TilemapLayer for FrontLayer {
    type TileType = GameTile;

    fn tiles(&self) -> &CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &self.tiles
    }

    fn tiles_mut(&mut self) -> &mut CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &mut self.tiles
    }
}

impl TilemapLayer for TeleLayer {
    type TileType = Tele;

    fn tiles(&self) -> &CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &self.tiles
    }

    fn tiles_mut(&mut self) -> &mut CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &mut self.tiles
    }
}

impl TilemapLayer for SpeedupLayer {
    type TileType = Speedup;

    fn tiles(&self) -> &CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &self.tiles
    }

    fn tiles_mut(&mut self) -> &mut CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &mut self.tiles
    }
}

impl TilemapLayer for SwitchLayer {
    type TileType = Switch;

    fn tiles(&self) -> &CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &self.tiles
    }

    fn tiles_mut(&mut self) -> &mut CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &mut self.tiles
    }
}

impl TilemapLayer for TuneLayer {
    type TileType = Tune;

    fn tiles(&self) -> &CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &self.tiles
    }

    fn tiles_mut(&mut self) -> &mut CompressedData<Array2<Self::TileType>, TilesLoadInfo> {
        &mut self.tiles
    }
}

impl AnyTile for Tile {
    fn id(&self) -> u8 {
        self.id
    }

    fn id_mut(&mut self) -> &mut u8 {
        &mut self.id
    }

    fn flags(&self) -> Option<TileFlags> {
        Some(self.flags)
    }

    fn flags_mut(&mut self) -> Option<&mut TileFlags> {
        Some(&mut self.flags)
    }
}

impl AnyTile for GameTile {
    fn id(&self) -> u8 {
        self.id
    }

    fn id_mut(&mut self) -> &mut u8 {
        &mut self.id
    }

    fn flags(&self) -> Option<TileFlags> {
        Some(self.flags)
    }

    fn flags_mut(&mut self) -> Option<&mut TileFlags> {
        Some(&mut self.flags)
    }
}

impl AnyTile for Switch {
    fn id(&self) -> u8 {
        self.id
    }

    fn id_mut(&mut self) -> &mut u8 {
        &mut self.id
    }

    fn flags(&self) -> Option<TileFlags> {
        Some(self.flags)
    }

    fn flags_mut(&mut self) -> Option<&mut TileFlags> {
        Some(&mut self.flags)
    }
}

impl AnyTile for Tele {
    fn id(&self) -> u8 {
        self.id
    }

    fn id_mut(&mut self) -> &mut u8 {
        &mut self.id
    }

    fn flags(&self) -> Option<TileFlags> {
        None
    }

    fn flags_mut(&mut self) -> Option<&mut TileFlags> {
        None
    }
}

impl AnyTile for Speedup {
    fn id(&self) -> u8 {
        self.id
    }

    fn id_mut(&mut self) -> &mut u8 {
        &mut self.id
    }

    fn flags(&self) -> Option<TileFlags> {
        None
    }

    fn flags_mut(&mut self) -> Option<&mut TileFlags> {
        None
    }
}

impl AnyTile for Tune {
    fn id(&self) -> u8 {
        self.id
    }

    fn id_mut(&mut self) -> &mut u8 {
        &mut self.id
    }

    fn flags(&self) -> Option<TileFlags> {
        None
    }

    fn flags_mut(&mut self) -> Option<&mut TileFlags> {
        None
    }
}

impl AnyLayer for GameLayer {
    fn kind() -> LayerKind {
        LayerKind::Game
    }

    fn get(layer: &Layer) -> Option<&Self> {
        if let Layer::Game(l) = layer {
            Some(l)
        } else {
            None
        }
    }

    fn get_mut(layer: &mut Layer) -> Option<&mut Self> {
        if let Layer::Game(l) = layer {
            Some(l)
        } else {
            None
        }
    }
}

impl AnyLayer for TilesLayer {
    fn kind() -> LayerKind {
        LayerKind::Tiles
    }

    fn get(layer: &Layer) -> Option<&Self> {
        if let Layer::Tiles(l) = layer {
            Some(l)
        } else {
            None
        }
    }

    fn get_mut(layer: &mut Layer) -> Option<&mut Self> {
        if let Layer::Tiles(l) = layer {
            Some(l)
        } else {
            None
        }
    }
}

impl AnyLayer for QuadsLayer {
    fn kind() -> LayerKind {
        LayerKind::Quads
    }

    fn get(layer: &Layer) -> Option<&Self> {
        if let Layer::Quads(l) = layer {
            Some(l)
        } else {
            None
        }
    }

    fn get_mut(layer: &mut Layer) -> Option<&mut Self> {
        if let Layer::Quads(l) = layer {
            Some(l)
        } else {
            None
        }
    }
}

impl AnyLayer for FrontLayer {
    fn kind() -> LayerKind {
        LayerKind::Front
    }

    fn get(layer: &Layer) -> Option<&Self> {
        if let Layer::Front(l) = layer {
            Some(l)
        } else {
            None
        }
    }

    fn get_mut(layer: &mut Layer) -> Option<&mut Self> {
        if let Layer::Front(l) = layer {
            Some(l)
        } else {
            None
        }
    }
}

impl AnyLayer for TeleLayer {
    fn kind() -> LayerKind {
        LayerKind::Tele
    }

    fn get(layer: &Layer) -> Option<&Self> {
        if let Layer::Tele(l) = layer {
            Some(l)
        } else {
            None
        }
    }

    fn get_mut(layer: &mut Layer) -> Option<&mut Self> {
        if let Layer::Tele(l) = layer {
            Some(l)
        } else {
            None
        }
    }
}

impl AnyLayer for SpeedupLayer {
    fn kind() -> LayerKind {
        LayerKind::Speedup
    }

    fn get(layer: &Layer) -> Option<&Self> {
        if let Layer::Speedup(l) = layer {
            Some(l)
        } else {
            None
        }
    }

    fn get_mut(layer: &mut Layer) -> Option<&mut Self> {
        if let Layer::Speedup(l) = layer {
            Some(l)
        } else {
            None
        }
    }
}

impl AnyLayer for SwitchLayer {
    fn kind() -> LayerKind {
        LayerKind::Switch
    }

    fn get(layer: &Layer) -> Option<&Self> {
        if let Layer::Switch(l) = layer {
            Some(l)
        } else {
            None
        }
    }

    fn get_mut(layer: &mut Layer) -> Option<&mut Self> {
        if let Layer::Switch(l) = layer {
            Some(l)
        } else {
            None
        }
    }
}

impl AnyLayer for TuneLayer {
    fn kind() -> LayerKind {
        LayerKind::Tune
    }

    fn get(layer: &Layer) -> Option<&Self> {
        if let Layer::Tune(l) = layer {
            Some(l)
        } else {
            None
        }
    }

    fn get_mut(layer: &mut Layer) -> Option<&mut Self> {
        if let Layer::Tune(l) = layer {
            Some(l)
        } else {
            None
        }
    }
}

impl AnyLayer for SoundsLayer {
    fn kind() -> LayerKind {
        LayerKind::Sounds
    }

    fn get(layer: &Layer) -> Option<&Self> {
        if let Layer::Sounds(l) = layer {
            Some(l)
        } else {
            None
        }
    }

    fn get_mut(layer: &mut Layer) -> Option<&mut Self> {
        if let Layer::Sounds(l) = layer {
            Some(l)
        } else {
            None
        }
    }
}

impl PhysicsLayer for GameLayer {}
impl PhysicsLayer for FrontLayer {}
impl PhysicsLayer for TeleLayer {}
impl PhysicsLayer for SpeedupLayer {}
impl PhysicsLayer for SwitchLayer {}
impl PhysicsLayer for TuneLayer {}
