use crate::convert::TryTo;
use crate::datafile::{save as save_datafile, Item};
use crate::map::*;

use structview::View;

use crate::edit::ZeroAir;
use crate::map::parse::{BinaryQuad, BinarySoundSource, Identifier, ItemType, MapVersion};
use fixed::traits::Fixed;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::io::Write;
use std::iter;
use std::mem;
use std::path::Path;
use std::slice;

// contains the data of a item_type, that is meant to be added to the datafile
#[derive(Debug)]
struct ItemTypeInsert<'a> {
    id: Identifier,
    items: Vec<Item>,
    data_items: Vec<Cow<'a, [u8]>>,
}

fn uuid_equal(u1: &[u8; 16], u2: &[i32; 4]) -> bool {
    u1.chunks(4)
        .map(|bytes| i32::from_be_bytes(bytes[0..4].try_into().unwrap()))
        .zip(u2.iter())
        .all(|(x, &y)| x == y)
}

// initializes an ex item type by adding its uuid to the ex item index, returning the initialized type id
fn add_uuid_type(items: &mut HashMap<u16, Vec<Item>>, uuid: [u8; 16]) -> u16 {
    let ex_item_id = 0xffff;
    match items.get(&ex_item_id) {
        // None => insert the ex items index
        None => {
            items.insert(ex_item_id, Vec::new());
        }
        // Some => check if uuid was already inserted before
        Some(ex_items) => {
            for item in ex_items {
                if uuid_equal(&uuid, item.item_data[..4].try_into().unwrap()) {
                    panic!(
                        "Tried to insert ex_items of already inserted uuid: {:?}.",
                        uuid
                    );
                }
            }
        }
    }
    let ex_items = items.get_mut(&ex_item_id).unwrap();
    let type_id = ex_item_id - ex_items.len().try_to::<u16>() - 1;
    let item = Item {
        id: type_id,
        item_data: uuid
            .chunks(4)
            .map(|bytes| i32::from_be_bytes(bytes[0..4].try_into().unwrap()))
            .collect(),
    };
    ex_items.push(item);
    type_id
}

// wrapper around insert, does nothing on None
fn option_insert<'a>(
    items: &mut HashMap<u16, Vec<Item>>,
    data_items: &mut Vec<Cow<'a, [u8]>>,
    option_to_insert: Option<ItemTypeInsert<'a>>,
) {
    match option_to_insert {
        None => (),
        Some(to_insert) => insert(items, data_items, to_insert),
    }
}

// add the data of a item type to the datafile, using a ItemTypeInsert
fn insert<'a>(
    items: &mut HashMap<u16, Vec<Item>>,
    data_items: &mut Vec<Cow<'a, [u8]>>,
    mut to_insert: ItemTypeInsert<'a>,
) {
    assert_ne!(to_insert.items.len(), 0);
    let type_id = match to_insert.id {
        Identifier::TypeId(id) => id,
        Identifier::Uuid(uuid) => add_uuid_type(items, uuid),
    };
    let replaced = items.insert(type_id, to_insert.items);
    if replaced.is_some() {
        panic!(
            "Tried to insert items of already existing type id: {}",
            type_id
        );
    }
    data_items.append(&mut to_insert.data_items);
}

impl TwMap {
    /// Saves the map in the binary format.
    /// Will overwrite if that file already exists.
    #[deprecated(note = "Please use `save` instead")]
    pub fn save_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        let mut file = fs::File::create(path)?;
        self.save(&mut file)
    }

    /// Saves the map in the binary format.
    /// Since it uses a `dyn Write` as a parameter, you are free to save it into a vec, file, etc.
    pub fn save(&mut self, output: &mut dyn Write) -> Result<(), Error> {
        self.load()?;
        self.check()?;
        self.edit_tiles::<ZeroAir>();
        self.process_tile_flag_opaque();
        self.set_external_image_dimensions();

        let (items, data_items) = self.save_to_datafile_unwrap();
        save_datafile(output, &items, &data_items)?;

        Ok(())
    }

    // stitches the datafile together by continuously filling up items and data_items with all components of the map
    #[allow(clippy::type_complexity)] // TODO: refactor return types into a type
    pub(crate) fn save_to_datafile_unwrap(&self) -> (HashMap<u16, Vec<Item>>, Vec<Cow<[u8]>>) {
        let mut items = HashMap::new();
        let mut data_items = Vec::new();

        let raw_map_version = MapVersion::save();
        insert(&mut items, &mut data_items, raw_map_version);

        let raw_map_info = Info::save(&self.info, self.version, data_items.len().try_to());
        insert(&mut items, &mut data_items, raw_map_info);

        let raw_map_images = Image::save(&self.images, self.version, data_items.len().try_to());
        option_insert(&mut items, &mut data_items, raw_map_images);

        let (raw_map_envelopes, raw_env_points, raw_bezier_data) =
            Envelope::save(&self.envelopes, self.version);
        option_insert(&mut items, &mut data_items, raw_map_envelopes);

        let raw_map_groups = Group::save(&self.groups);
        insert(&mut items, &mut data_items, raw_map_groups);

        let raw_map_layers = Layer::save(&self.groups, self.version, data_items.len().try_to());
        insert(&mut items, &mut data_items, raw_map_layers);

        // moved here for ordering by type_id
        insert(&mut items, &mut data_items, raw_env_points);

        let raw_map_sounds = Sound::save(&self.sounds, data_items.len().try_to());
        option_insert(&mut items, &mut data_items, raw_map_sounds);

        if self.version == Version::DDNet06 {
            let raw_auto_mappers = AutomapperConfig::save(&self.groups);
            option_insert(&mut items, &mut data_items, raw_auto_mappers);
        }

        option_insert(&mut items, &mut data_items, raw_bezier_data);

        (items, data_items)
    }
}

// returns the c representation of a string by adding a null byte
fn c_string(string: &str) -> Vec<u8> {
    let mut bytes = string.as_bytes().to_vec();
    bytes.push(0); // null byte to terminate string
    bytes
}

// MapVersion is inside of every map
impl MapVersion {
    fn save<'a>() -> ItemTypeInsert<'a> {
        let item = Item {
            id: 0,
            item_data: vec![1],
        };
        ItemTypeInsert {
            id: ItemType::Version.identifier(),
            items: vec![item],
            data_items: vec![],
        }
    }
}

impl Info {
    fn save<'a>(&self, version: Version, data_len: i32) -> ItemTypeInsert<'a> {
        let (item, data_items) = self.itemize(data_len, version);
        ItemTypeInsert {
            id: ItemType::Info.identifier(),
            items: vec![item],
            data_items,
        }
    }

    fn itemize<'a>(&self, mut data_len: i32, version: Version) -> (Item, Vec<Cow<'a, [u8]>>) {
        let mut item_data = vec![-1; 5];
        item_data[0] = 1; // version
        let mut data = Vec::new();
        let fields = [&self.author, &self.version, &self.credits, &self.license];
        for (i, string) in fields.iter().enumerate() {
            if !string.is_empty() {
                item_data[i + 1] = data_len;
                data_len += 1;
                data.push(Cow::from(c_string(string)))
            }
        }
        if version == Version::DDNet06 {
            if !self.settings.is_empty() {
                item_data.push(data_len);
                let mut string_data = Vec::new();
                for string in &self.settings {
                    string_data.extend(c_string(string))
                }
                data.push(Cow::from(string_data))
            } else {
                item_data.push(-1);
            }
        }
        let item = Item { id: 0, item_data };
        (item, data)
    }
}

impl Image {
    fn save(map_images: &[Image], version: Version, data_len: i32) -> Option<ItemTypeInsert> {
        if map_images.is_empty() {
            return None;
        }
        let mut items = Vec::new();
        let mut data_items = Vec::new();
        for (i, image) in map_images.iter().enumerate() {
            let id = i.try_to();
            let (new_item, new_data_items) =
                image.itemize(id, version, data_len + data_items.len().try_to::<i32>());
            items.push(new_item);
            data_items.extend(new_data_items);
        }
        Some(ItemTypeInsert {
            id: ItemType::Image.identifier(),
            items,
            data_items,
        })
    }

    fn itemize(&self, id: u16, version: Version, data_len: i32) -> (Item, Vec<Cow<[u8]>>) {
        let mut item_data = vec![0; 6];
        let mut data_items = Vec::new();
        item_data[0] = match version {
            Version::DDNet06 => 1,
            Version::Teeworlds07 => 2,
        }; // version
        let size = self.size();
        item_data[1] = size.w.try_to();
        item_data[2] = size.h.try_to();
        item_data[4] = data_len; // data index of string
        data_items.push(Cow::from(c_string(self.name())));
        match self.image() {
            None => {
                item_data[3] = true.into();
                item_data[5] = -1;
            }
            Some(CompressedData::Compressed(_, _, _)) => unreachable!(),
            Some(CompressedData::Loaded(image)) => {
                item_data[3] = false.into();
                item_data[5] = data_len + 1;
                data_items.push(Cow::from(image.as_ref()))
            }
        }
        if version == Version::Teeworlds07 {
            item_data.push(1); // identifies image as rgba, support for rgb (0) is deprecated
        }
        let item = Item { id, item_data };
        (item, data_items)
    }
}

fn string_to_i32s(str: &str, i32_count: usize) -> Vec<i32> {
    assert!(str.len() < i32_count * 4);
    let mut bytes: Vec<u8> = str
        .bytes()
        .chain(iter::repeat(0).take(i32_count * 4 - str.len()))
        .map(|x| x.wrapping_sub(128)) // still why
        .collect();
    bytes[i32_count * 4 - 1] = 0; // set last byte to null
    (0..i32_count)
        .map(|i| i32::from_be_bytes(bytes[i * 4..(i + 1) * 4].try_into().unwrap()))
        .collect()
}

fn itemize<T: EnvPointContentWriting + Copy>(
    env: &Env<T>,
    id: u16,
    total_points_len: i32,
    env_version: i32,
    point_data: &mut Vec<i32>,
    bezier_data: &mut Vec<i32>,
) -> (Item, i32) {
    let mut item_data = [0; 13];

    item_data[0] = env_version;
    item_data[1] = T::channels(); // envelope type (color -> 4, position -> 3, ...)
    item_data[2] = total_points_len; // index of first point
    item_data[3] = env.points.len().try_to::<i32>(); // number of points in envelope
    item_data[12] = env.synchronized.into();
    let i32_string = string_to_i32s(&env.name, 8);
    item_data[4..12].copy_from_slice(&i32_string);
    for point in &env.points {
        point_data.extend(to_raw_point(point));
        bezier_data.extend(to_raw_bezier(point));
    }
    let item = Item {
        id,
        item_data: item_data.to_vec(),
    };
    (item, env.points.len().try_to::<i32>())
}

impl<T: Copy> CurveKind<T> {
    fn to_id(self) -> i32 {
        use CurveKind::*;
        match self {
            Step => 0,
            Linear => 1,
            Slow => 2,
            Fast => 3,
            Smooth => 4,
            Bezier(_) => 5,
            Unknown(n) => panic!(
                "Tried to access i32 representation of unknown curve type {}",
                n
            ),
        }
    }
}

trait EnvPointContentWriting {
    fn to_raw(self) -> [i32; 4];

    fn channels() -> i32;
}

impl EnvPointContentWriting for Volume {
    fn to_raw(self) -> [i32; 4] {
        [self.0.to_bits(), 0, 0, 0]
    }

    fn channels() -> i32 {
        1
    }
}

impl EnvPointContentWriting for Position {
    fn to_raw(self) -> [i32; 4] {
        [
            self.offset.x.to_bits(),
            self.offset.y.to_bits(),
            self.rotation.to_bits(),
            0,
        ]
    }

    fn channels() -> i32 {
        3
    }
}

impl EnvPointContentWriting for Rgba<I22F10> {
    fn to_raw(self) -> [i32; 4] {
        [
            self.r.to_bits(),
            self.g.to_bits(),
            self.b.to_bits(),
            self.a.to_bits(),
        ]
    }

    fn channels() -> i32 {
        4
    }
}

fn array_curve_kind<T: EnvPointContentWriting>(curve_type: CurveKind<T>) -> CurveKind<[i32; 4]> {
    use CurveKind::*;
    match curve_type {
        Step => Step,
        Linear => Linear,
        Slow => Slow,
        Fast => Fast,
        Smooth => Smooth,
        Unknown(n) => Unknown(n),
        Bezier(b) => Bezier(BezierCurve {
            handle_l: b.handle_l.map(EnvPointContentWriting::to_raw),
            handle_r: b.handle_r.map(EnvPointContentWriting::to_raw),
        }),
    }
}

fn to_raw_point<T: EnvPointContentWriting + Copy>(point: &EnvPoint<T>) -> [i32; 6] {
    let mut data = [0; 6];
    data[0] = point.time;
    data[1] = point.curve.to_id();
    data[2..6].copy_from_slice(&point.content.to_raw());

    data
}

fn to_raw_bezier<T: EnvPointContentWriting + Copy>(point: &EnvPoint<T>) -> [i32; 16] {
    let mut data = [0; 16];
    if let CurveKind::Bezier(b) = array_curve_kind(point.curve) {
        data[0..4].copy_from_slice(&b.handle_l.x);
        data[4..8].copy_from_slice(&b.handle_l.y);
        data[8..12].copy_from_slice(&b.handle_r.x);
        data[12..16].copy_from_slice(&b.handle_r.y);
    }
    data
}

fn contains_bezier<T>(points: &[EnvPoint<T>]) -> bool {
    points
        .iter()
        .any(|p| matches!(p.curve, CurveKind::Bezier(_)))
}

impl Envelope {
    fn save(
        map_envelopes: &[Envelope],
        version: Version,
    ) -> (
        Option<ItemTypeInsert>,
        ItemTypeInsert,
        Option<ItemTypeInsert>,
    ) {
        // if no envelopes exist, the env_point item type will still be inserted
        let mut env_points_save = ItemTypeInsert {
            id: ItemType::EnvPoints.identifier(),
            items: vec![Item {
                id: 0,
                item_data: vec![],
            }],
            data_items: vec![],
        };
        if map_envelopes.is_empty() {
            return (None, env_points_save, None);
        }

        let mut envelope_items = Vec::new();
        // Eight 'out' bezier values of -1th envelope point
        let mut point_data = Vec::new();
        let mut bezier_data = vec![0; 8];
        let mut total_points_len = 0;
        let bezier_in_use = map_envelopes.iter().any(|env| match env {
            Envelope::Position(env) => contains_bezier(&env.points),
            Envelope::Color(env) => contains_bezier(&env.points),
            Envelope::Sound(env) => contains_bezier(&env.points),
        });
        let env_version = match (bezier_in_use, version) {
            (true, Version::Teeworlds07) => 3,
            _ => 2,
        };
        for (i, envelope) in map_envelopes.iter().enumerate() {
            let (new_item, new_env_point_count) = envelope.itemize(
                i.try_to(),
                total_points_len,
                env_version,
                &mut point_data,
                &mut bezier_data,
            );
            envelope_items.push(new_item);
            total_points_len += new_env_point_count;
        }
        let envelope_save = ItemTypeInsert {
            id: ItemType::Envelope.identifier(),
            items: envelope_items,
            data_items: vec![],
        };
        // Eight 'in' bezier values of last bezier curve aren't saved
        bezier_data.truncate(bezier_data.len() - 8);
        let mut env_bezier_data_save = None;
        env_points_save.items[0].item_data = match version {
            Version::DDNet06 => {
                if bezier_in_use {
                    env_bezier_data_save = Some(ItemTypeInsert {
                        id: ItemType::EnvPointsBezierData.identifier(),
                        items: vec![Item {
                            id: 0,
                            item_data: bezier_data,
                        }],
                        data_items: vec![],
                    })
                }
                point_data
            }
            Version::Teeworlds07 => {
                if !bezier_in_use {
                    point_data
                } else {
                    point_data
                        .chunks(6)
                        .zip(bezier_data.chunks(16))
                        .flat_map(|(x, y)| x.iter().chain(y.iter()).copied())
                        .collect()
                }
            }
        };

        (Some(envelope_save), env_points_save, env_bezier_data_save)
    }

    fn itemize(
        &self,
        id: u16,
        point_start: i32,
        env_version: i32,
        point_data: &mut Vec<i32>,
        bezier_data: &mut Vec<i32>,
    ) -> (Item, i32) {
        use Envelope::*;
        match self {
            Position(p) => itemize(p, id, point_start, env_version, point_data, bezier_data),
            Color(c) => itemize(c, id, point_start, env_version, point_data, bezier_data),
            Sound(s) => itemize(s, id, point_start, env_version, point_data, bezier_data),
        }
    }
}

impl Group {
    fn save(map_groups: &[Group]) -> ItemTypeInsert {
        let mut items = Vec::new();

        let mut layer_count = 0;
        for (i, group) in map_groups.iter().enumerate() {
            let (new_group, added_layer_count) = group.itemize(layer_count, i.try_to());
            items.push(new_group);
            layer_count += added_layer_count;
        }

        ItemTypeInsert {
            id: ItemType::Group.identifier(),
            items,
            data_items: vec![],
        }
    }

    fn itemize(&self, first_layer_index: i32, id: u16) -> (Item, i32) {
        let mut item_data = [0; 15];

        item_data[0] = 3; // version
        item_data[1] = self.offset.x.to_bits();
        item_data[2] = self.offset.y.to_bits();
        item_data[3] = self.parallax.x;
        item_data[4] = self.parallax.y;
        item_data[5] = first_layer_index;
        item_data[6] = self.layers.len().try_to();
        item_data[7] = self.clipping.into();
        item_data[8] = self.clip.x.to_bits();
        item_data[9] = self.clip.y.to_bits();
        item_data[10] = self.clip.w.to_bits();
        item_data[11] = self.clip.h.to_bits();
        item_data[12..15].copy_from_slice(&string_to_i32s(&self.name, 3));

        let item_data = item_data.to_vec();
        (Item { id, item_data }, self.layers.len().try_to())
    }
}

trait ViewAsBytes: View {
    fn into_boxed_bytes(boxed_slice: Box<[Self]>) -> Box<[u8]> {
        let len = boxed_slice.len() * mem::size_of::<Self>();
        let ptr = Box::into_raw(boxed_slice);
        unsafe {
            let byte_slice = slice::from_raw_parts_mut(ptr as *mut u8, len);
            Box::from_raw(byte_slice)
        }
    }
}

impl<T: View> ViewAsBytes for T {}

trait TileCompression: AnyTile {
    // only tiles of type 'Tile' and 'GameTile' can actually be compressed
    fn compress_tiles(_: Vec<Self>) -> Vec<Self> {
        panic!();
    }
}

impl TileCompression for Tile {
    fn compress_tiles(tiles: Vec<Tile>) -> Vec<Tile> {
        let mut compressed_tiles = vec![tiles[0]];

        for &tile in &tiles[1..] {
            let current_tile = compressed_tiles.last_mut().unwrap();

            if current_tile.skip == u8::MAX {
                // maximum for tile skipping (255)
                compressed_tiles.push(tile);
            } else if current_tile.id == tile.id && current_tile.flags == tile.flags {
                current_tile.skip += 1;
            } else {
                compressed_tiles.push(tile);
            }
        }
        compressed_tiles
    }
}

impl TileCompression for GameTile {
    fn compress_tiles(tiles: Vec<GameTile>) -> Vec<GameTile> {
        let mut compressed_tiles = vec![tiles[0]];

        for &tile in &tiles[1..] {
            let current_tile = compressed_tiles.last_mut().unwrap();

            if current_tile.skip == u8::MAX {
                // maximum for tile skipping (255)
                compressed_tiles.push(tile);
            } else if current_tile.id == tile.id && current_tile.flags == tile.flags {
                current_tile.skip += 1;
            } else {
                compressed_tiles.push(tile);
            }
        }
        compressed_tiles
    }
}

impl TileCompression for Tele {}

impl TileCompression for Switch {}

impl TileCompression for Tune {}

impl TileCompression for Speedup {}

trait PhysicsLayerSaving: TilemapLayer
where
    Self::TileType: TileCompression,
{
    fn tiles_layer_flag() -> i32;

    fn itemize(&self, version: Version, data_len: i32, id: u16) -> (Item, Vec<Cow<[u8]>>) {
        let mut item_data = vec![-1; 18];
        let mut data_items = Vec::new();
        item_data[0] = 0; // uninitialized value
        item_data[1] = 2; // layer type -> tile layer
        item_data[2] = 0; // flags, first bit: detail (not interesting for physics layers)
        item_data[3] = match version {
            // write version
            Version::DDNet06 => 3,
            Version::Teeworlds07 => 4,
        };
        let width = self.tiles().unwrap_ref().ncols();
        item_data[4] = width.try_to();
        let height = self.tiles().unwrap_ref().nrows();
        item_data[5] = height.try_to();
        item_data[6] = Self::tiles_layer_flag(); // specifies type of tile layer
        item_data[7..11].copy_from_slice(&[255; 4]); // color
        item_data[11] = -1; // color env index
        item_data[12] = 0; // color env offset
        item_data[13] = -1; // image index
        item_data[14] = data_len;
        item_data[15..18].copy_from_slice(&string_to_i32s(
            &String::from(Self::kind().static_name()),
            3,
        ));
        if version == Version::DDNet06 {
            item_data.extend([-1; 5]);
            if Self::kind() != LayerKind::Game {
                item_data[Self::kind().data_index()] = data_len + 1;
                data_items.push(Cow::from(
                    iter::repeat(0)
                        .take(width * height * mem::size_of::<Tile>())
                        .collect::<Vec<_>>(),
                ));
            }
        }
        let (mut tiles, offset) = self
            .tiles()
            .unwrap_ref()
            .to_owned()
            .into_raw_vec_and_offset();
        assert_eq!(offset, Some(0));

        if version == Version::Teeworlds07 && Self::kind() == LayerKind::Game {
            tiles = Self::TileType::compress_tiles(tiles);
        }

        data_items.push(Cow::from(
            Self::TileType::into_boxed_bytes(tiles.into_boxed_slice()).into_vec(),
        ));
        (
            Item {
                id,
                item_data: item_data.to_vec(),
            },
            data_items,
        )
    }
}

impl PhysicsLayerSaving for GameLayer {
    fn tiles_layer_flag() -> i32 {
        1
    }
}

impl PhysicsLayerSaving for FrontLayer {
    fn tiles_layer_flag() -> i32 {
        8
    }
}

impl PhysicsLayerSaving for TeleLayer {
    fn tiles_layer_flag() -> i32 {
        2
    }
}

impl PhysicsLayerSaving for SpeedupLayer {
    fn tiles_layer_flag() -> i32 {
        4
    }
}

impl PhysicsLayerSaving for SwitchLayer {
    fn tiles_layer_flag() -> i32 {
        16
    }
}

impl PhysicsLayerSaving for TuneLayer {
    fn tiles_layer_flag() -> i32 {
        32
    }
}

fn from_u16_option_index(index: Option<u16>) -> i32 {
    match index {
        None => -1,
        Some(n) => n.into(),
    }
}

impl TilesLayer {
    fn itemize(&self, version: Version, data_len: i32, id: u16) -> (Item, Vec<Cow<[u8]>>) {
        let mut item_data = vec![-1; 18];
        let mut data_items = Vec::new();
        let tiles = self.tiles.unwrap_ref();
        item_data[0] = 0; // uninitialized value
        item_data[1] = 2; // layer type -> tile layer
        item_data[2] = self.detail.into(); // flags, first bit: detail
        item_data[3] = match version {
            Version::DDNet06 => 3,
            Version::Teeworlds07 => 4,
        };
        let width = tiles.ncols();
        item_data[4] = width.try_to();
        let height = tiles.nrows();
        item_data[5] = height.try_to();
        item_data[6] = 0; // specifies type of tile layer
        item_data[7] = self.color.r.into();
        item_data[8] = self.color.g.into();
        item_data[9] = self.color.b.into();
        item_data[10] = self.color.a.into();
        item_data[11] = from_u16_option_index(self.color_env); // color env index
        item_data[12] = self.color_env_offset; // color env offset
        item_data[13] = from_u16_option_index(self.image); // image index
        item_data[14] = data_len;
        item_data[15..18].copy_from_slice(&string_to_i32s(&self.name, 3));
        let (mut tiles, offset) = tiles.to_owned().into_raw_vec_and_offset();
        assert_eq!(offset, Some(0));
        if version == Version::Teeworlds07 {
            tiles = Tile::compress_tiles(tiles);
        }
        data_items.push(Cow::from(
            Tile::into_boxed_bytes(tiles.into_boxed_slice()).into_vec(),
        ));
        if version == Version::DDNet06 {
            item_data.extend([-1; 5]);
        }
        (
            Item {
                id,
                item_data: item_data.to_vec(),
            },
            data_items,
        )
    }
}

fn to_i32<T: Fixed<Bits = i32>>(vec2: Vec2<T>) -> [i32; 2] {
    [vec2.x.to_bits(), vec2.y.to_bits()]
}

fn uv_to_i32<T: Fixed<Bits = i32>>(uv: Uv<T>) -> [i32; 2] {
    [uv.u.to_bits(), uv.v.to_bits()]
}

fn u8_c_to_i32(c: Rgba<u8>) -> [i32; 4] {
    [c.r.into(), c.g.into(), c.b.into(), c.a.into()]
}

impl Quad {
    fn to_bytes(&self) -> Vec<u8> {
        let mut i32_data = Vec::new();
        for i in 0..4 {
            i32_data.extend(to_i32(self.corners[i]));
        }
        i32_data.extend(to_i32(self.position));
        for i in 0..4 {
            i32_data.extend(u8_c_to_i32(self.colors[i]));
        }
        for i in 0..4 {
            i32_data.extend(uv_to_i32(self.texture_coords[i]));
        }
        i32_data.push(from_u16_option_index(self.position_env));
        i32_data.push(self.position_env_offset);
        i32_data.push(from_u16_option_index(self.color_env));
        i32_data.push(self.color_env_offset);
        let mut u8_data = Vec::new();
        for n in i32_data {
            u8_data.extend(i32::to_le_bytes(n));
        }
        u8_data
    }
}

impl QuadsLayer {
    fn create_data_item(&self) -> Cow<[u8]> {
        let mut data_item = Vec::new();
        // Dummy data for compatibility
        if self.quads.is_empty() {
            data_item.extend(iter::repeat(0).take(mem::size_of::<BinaryQuad>()));
        }
        for quad in &self.quads {
            data_item.extend(quad.to_bytes());
        }
        Cow::from(data_item)
    }

    fn itemize(&self, data_len: i32, id: u16) -> (Item, Vec<Cow<[u8]>>) {
        let mut item_data = [-1; 10];
        item_data[0] = 0; // uninitialized value
        item_data[1] = 3; // layer type -> quad layer
        item_data[2] = self.detail.into(); // flags, 1 -> detail
        item_data[3] = 2; // version
        item_data[4] = self.quads.len().try_to();
        item_data[5] = data_len; // data index
        item_data[6] = from_u16_option_index(self.image); // image index
        item_data[7..10].copy_from_slice(&string_to_i32s(&self.name, 3));

        let data_items = vec![self.create_data_item()];
        (
            Item {
                id,
                item_data: item_data.to_vec(),
            },
            data_items,
        )
    }
}

impl SoundSource {
    fn to_bytes(&self) -> Vec<u8> {
        use SoundArea::*;
        let mut i32_data = Vec::new();
        i32_data.extend(to_i32(self.area.position()));
        i32_data.push(self.looping.into());
        i32_data.push(self.panning.into());
        i32_data.push(self.delay);
        i32_data.push(self.falloff.into());
        i32_data.push(from_u16_option_index(self.position_env));
        i32_data.push(self.position_env_offset);
        i32_data.push(from_u16_option_index(self.sound_env));
        i32_data.push(self.sound_env_offset);
        match self.area {
            Rectangle(rekt) => {
                i32_data.push(0); // identifier for Rectangle
                i32_data.push(rekt.w.to_bits());
                i32_data.push(rekt.h.to_bits());
            }
            Circle(disk) => {
                i32_data.push(1); // identifier for Circle
                i32_data.push(disk.radius.to_bits());
                i32_data.push(-1); // unused i32
            }
        }
        let mut u8_data = Vec::new();
        for n in i32_data {
            u8_data.extend(i32::to_le_bytes(n));
        }
        u8_data
    }
}

impl SoundsLayer {
    fn create_data_item(&self) -> Cow<[u8]> {
        let mut data_item = Vec::new();
        // Dummy data for compatibility
        if self.sources.is_empty() {
            data_item.extend(iter::repeat(0).take(mem::size_of::<BinarySoundSource>()));
        }
        for source in &self.sources {
            data_item.extend(source.to_bytes());
        }
        Cow::from(data_item)
    }

    fn itemize(&self, data_len: i32, id: u16) -> (Item, Vec<Cow<[u8]>>) {
        let mut item_data = [-1; 10];
        item_data[0] = 1; // uninitialized value
        item_data[1] = 10; // layer type -> sound layer
        item_data[2] = self.detail.into(); // flags, 1 -> detail
        item_data[3] = 2; //version
        item_data[4] = self.sources.len().try_to();
        item_data[5] = data_len; // data index
        item_data[6] = from_u16_option_index(self.sound);
        item_data[7..10].copy_from_slice(&string_to_i32s(&self.name, 3));
        let data_items = vec![self.create_data_item()];
        (
            Item {
                id,
                item_data: item_data.to_vec(),
            },
            data_items,
        )
    }
}

impl Layer {
    fn save(groups: &[Group], version: Version, data_len: i32) -> ItemTypeInsert {
        let mut items = Vec::new();
        let mut data_items = Vec::new();

        for group in groups {
            for layer in &group.layers {
                let (new_item, new_data_items) = layer.itemize(
                    items.len().try_to(),
                    version,
                    data_len + data_items.len().try_to::<i32>(),
                );
                items.push(new_item);
                data_items.extend(new_data_items);
            }
        }

        ItemTypeInsert {
            id: ItemType::Layer.identifier(),
            items,
            data_items,
        }
    }

    // itemizes item, returns additional auto mapper item if tile layer
    fn itemize(&self, id: u16, version: Version, data_len: i32) -> (Item, Vec<Cow<[u8]>>) {
        use Layer::*;
        let (layer_item, data_items) = match self {
            Game(l) => l.itemize(version, data_len, id),
            Front(l) => l.itemize(version, data_len, id),
            Tele(l) => l.itemize(version, data_len, id),
            Speedup(l) => l.itemize(version, data_len, id),
            Switch(l) => l.itemize(version, data_len, id),
            Tune(l) => l.itemize(version, data_len, id),
            Tiles(l) => l.itemize(version, data_len, id),
            Quads(l) => l.itemize(data_len, id),
            Sounds(l) => l.itemize(data_len, id),
            Invalid(_) => panic!(),
        };
        (layer_item, data_items)
    }
}

impl Sound {
    fn save(map_sounds: &[Sound], data_len: i32) -> Option<ItemTypeInsert> {
        if map_sounds.is_empty() {
            return None;
        }
        let mut items = Vec::new();
        let mut data_items = Vec::new();
        for (i, sound) in map_sounds.iter().enumerate() {
            let (new_item, new_data_items) =
                sound.itemize(i.try_to(), data_len + data_items.len().try_to::<i32>());
            items.push(new_item);
            data_items.extend(new_data_items);
        }
        Some(ItemTypeInsert {
            id: ItemType::Sound.identifier(),
            items,
            data_items,
        })
    }

    fn itemize(&self, id: u16, data_len: i32) -> (Item, Vec<Cow<[u8]>>) {
        let mut item_data = vec![0; 5];
        let mut data_items = Vec::new();
        item_data[0] = 1; // version
        item_data[2] = data_len; // data index of string
        data_items.push(Cow::from(c_string(&self.name)));
        match &self.data {
            CompressedData::Compressed(_, _, _) => unreachable!(),
            CompressedData::Loaded(data) => {
                item_data[1] = false.into();
                item_data[3] = data_len + 1;
                item_data[4] = data.len().try_to();
                data_items.push(Cow::from(data))
            }
        }
        let item = Item { id, item_data };
        (item, data_items)
    }
}

impl AutomapperConfig {
    fn save(groups: &[Group]) -> Option<ItemTypeInsert> {
        let mut items = Vec::new();

        for (group_id, group) in groups.iter().enumerate() {
            for (layer_id, layer) in group.layers.iter().enumerate() {
                if let Layer::Tiles(layer) = layer {
                    items.push(layer.automapper_config.itemize(
                        items.len().try_to(),
                        group_id.try_to(),
                        layer_id.try_to(),
                    ));
                }
            }
        }
        match items.len() {
            0 => None,
            _ => Some(ItemTypeInsert {
                id: ItemType::AutoMapperConfig.identifier(),
                items,
                data_items: vec![],
            }),
        }
    }
    fn itemize(&self, id: u16, group_id: u16, layer_id: u16) -> Item {
        let mut item_data = [-1; 6];
        item_data[0] = 1; // was previously uninitialized, do not use
        item_data[1] = group_id.into();
        item_data[2] = layer_id.into();
        item_data[3] = from_u16_option_index(self.config);
        item_data[4] = self.seed as i32;
        item_data[5] = self.automatic.into();
        Item {
            id,
            item_data: item_data.to_vec(),
        }
    }
}
