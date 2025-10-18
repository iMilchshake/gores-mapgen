use crate::compression;
use crate::datafile::{Datafile, Item, RawDatafile};
use crate::map::*;

use az::{CheckedAs, CheckedCast, UnwrappedAs};
use bitflags::bitflags;
use fixed::traits::Fixed;
use fixed::types::{I17F15, I22F10, I27F5};
use structview::{i32_le, View};
use thiserror::Error;
use vek::{Extent2, Uv};

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::fmt;
use std::fs;
use std::mem;
use std::ops::RangeInclusive;
use std::path::Path;
use vek::num_traits::Zero;

impl TwMap {
    /// For parsing a binary map file.
    #[deprecated(note = "Please use `parse` instead")]
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<TwMap, Error> {
        let data = fs::read(path)?;
        let map = TwMap::parse(&data)?;
        map.check()?;
        Ok(map)
    }

    #[deprecated(note = "Please use `parse_unchecked` instead")]
    pub fn parse_file_unchecked<P: AsRef<Path>>(path: P) -> Result<TwMap, Error> {
        let data = fs::read(path)?;
        let map = TwMap::parse_unchecked(&data)?;
        Ok(map)
    }

    /// For parsing binary map data.
    pub fn parse(data: &[u8]) -> Result<TwMap, Error> {
        let map = TwMap::parse_unchecked(data)?;
        map.check()?;
        Ok(map)
    }

    pub fn parse_unchecked(data: &[u8]) -> Result<TwMap, Error> {
        let raw_datafile = RawDatafile::parse(data)?;
        let datafile = raw_datafile.to_datafile();

        TwMap::parse_datafile_unchecked(&datafile).map_err(Error::MapParse)
    }

    /// Parses the [`TwMap`] struct from a [`Datafile`].
    /// Afterwards, it runs [`TwMap::check`].
    pub fn parse_datafile(df: &Datafile) -> Result<TwMap, Error> {
        let map = TwMap::parse_datafile_unchecked(df)?;
        map.check()?;
        Ok(map)
    }

    /// Parses the [`TwMap`] struct from a [`Datafile`].
    /// This function explicitly doesn't run [`TwMap::check`] after parsing.
    pub fn parse_datafile_unchecked(df: &Datafile) -> Result<TwMap, MapParseError> {
        let ex_index = ExType::parse_all(df, &HashMap::new())?;
        let ex_index: HashMap<[u8; 16], u16> = ex_index
            .into_iter()
            .map(|ExType { uuid, type_id }| (uuid, type_id))
            .collect();

        let _version = MapVersion::parse_single_item_only(df, &ex_index)?;
        let info = Info::parse_single_item_only_or_default(df, &ex_index)?;
        let images = Image::parse_all(df, &ex_index)?;

        // The bezier curve data for DDNet is also parsed in here
        let env_points =
            <Vec<EnvPoint<[i32; 4]>>>::parse_single_item_only_or_default(df, &ex_index)?;
        let mut envelopes = Envelope::parse_all(df, &ex_index)?;
        EnvPoint::distribute(env_points, &mut envelopes)?;

        let mut groups = Group::parse_all(df, &ex_index)?;
        let tilemap_version = Layer::check_versions(df, &ex_index)?;
        let layers = Layer::parse_all(df, &ex_index)?;
        Layer::distribute(layers, &mut groups)?;
        let automappers = <(Vec2<usize>, AutomapperConfig)>::parse_all(df, &ex_index)?;
        AutomapperConfig::distribute(automappers, &mut groups);

        let sounds = Sound::parse_all(df, &ex_index)?;
        let version = match tilemap_version {
            None => {
                return Err(MapParseError {
                    item_type: ItemType::Version,
                    index: None,
                    kind: LayerError::NoTilemap.into(),
                })
            }
            Some(i32::MIN..=3) => Version::DDNet06,
            Some(4..=i32::MAX) => Version::Teeworlds07,
        };

        let mut map = TwMap {
            version,
            info,
            images,
            envelopes,
            groups,
            sounds,
        };
        let removed = map.remove_duplicate_physics_layers();
        if removed > 0 {
            log::warn!("Removed {} duplicate physics layers.", removed);
        }
        map.correct_physics_group_name();
        Ok(map)
    }
}

#[derive(Error, Debug)]
pub struct MapParseError {
    item_type: ItemType,
    index: Option<usize>,
    kind: MapParseErrorKind,
}

impl fmt::Display for MapParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} error{}: {}",
            self.item_type,
            self.index
                .map(|i| format!(" at index {}", i))
                .unwrap_or_default(),
            self.kind
        )
    }
}

trait ItemParseErrorTrait: Sized {
    const KIND: ItemType;
    const VERSION_CHECK: Option<VersionChecker> = None;
    type State: Default;
    fn parse_impl(
        item: &Item,
        state: &mut Self::State,
        df: &Datafile,
        ex_index: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind>;

    fn parse_all(df: &Datafile, ex_index: &ExTypeIndex) -> Result<Vec<Self>, MapParseError> {
        let items = match df.get_items(ex_index, Self::KIND) {
            None => return Ok(Vec::new()),
            Some(items) => items,
        };

        if let Some(version_check) = Self::VERSION_CHECK {
            version_check.check(items, Self::KIND)?;
        }

        // TODO: see if we should check the id order of the items
        let mut parsed = Vec::new();
        let mut state = Self::State::default();
        for (i, item) in items.iter().enumerate() {
            let new =
                Self::parse_impl(item, &mut state, df, ex_index).map_err(|kind| match kind {
                    MapParseErrorKind::Recursive(err) => *err,
                    _ => MapParseError {
                        item_type: Self::KIND,
                        index: Some(i),
                        kind,
                    },
                })?;
            parsed.push(new);
        }
        Ok(parsed)
    }

    fn parse_single_item_only(
        df: &Datafile,
        ex_index: &ExTypeIndex,
    ) -> Result<Self, MapParseError> {
        let items = df.get_items(ex_index, Self::KIND);
        let item_count = items.map(|items| items.len()).unwrap_or(0);
        if item_count != 1 {
            return Err(MapParseError {
                item_type: Self::KIND,
                index: None,
                kind: MapParseErrorKind::ItemCount(item_count),
            });
        }
        let mut all = Self::parse_all(df, ex_index)?;
        assert_eq!(all.len(), 1);
        Ok(all.pop().unwrap())
    }
}

trait ItemParseErrorTraitSingle: ItemParseErrorTrait + Default {
    fn parse_single_item_only_or_default(
        df: &Datafile,
        ex_index: &ExTypeIndex,
    ) -> Result<Self, MapParseError> {
        let items = df.get_items(ex_index, Self::KIND);
        let item_count = match items {
            None => return Ok(Self::default()),
            Some(items) => items.len(),
        };
        if item_count != 1 {
            return Err(MapParseError {
                item_type: Self::KIND,
                index: None,
                kind: MapParseErrorKind::ItemCount(item_count),
            });
        }
        let mut all = Self::parse_all(df, ex_index)?;
        assert_eq!(all.len(), 1);
        Ok(all.pop().unwrap())
    }
}

impl<T: ItemParseErrorTrait + Default> ItemParseErrorTraitSingle for T {}

#[derive(Debug)]
pub(crate) enum Identifier {
    TypeId(u16),
    Uuid([u8; 16]),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub(crate) enum ItemType {
    Version,
    Info,
    Image,
    Envelope,
    Group,
    Layer,
    EnvPoints,
    EnvPointsBezierData, // DDNet only
    Sound,               // DDNet only
    ExType,              // DDNet only
    AutoMapperConfig,    // DDNet only
}

impl ItemType {
    pub(crate) fn identifier(&self) -> Identifier {
        match self {
            ItemType::Version => Identifier::TypeId(0),
            ItemType::Info => Identifier::TypeId(1),
            ItemType::Image => Identifier::TypeId(2),
            ItemType::Envelope => Identifier::TypeId(3),
            ItemType::Group => Identifier::TypeId(4),
            ItemType::Layer => Identifier::TypeId(5),
            ItemType::EnvPoints => Identifier::TypeId(6),
            ItemType::EnvPointsBezierData => Identifier::Uuid([
                0x3a, 0xb4, 0xf8, 0x4d, 0xd9, 0xcc, 0x3c, 0x78, 0xb5, 0x85, 0xd, 0x6c, 0xcd, 0xb2,
                0xe2, 0x5c,
            ]),
            ItemType::Sound => Identifier::TypeId(7),
            ItemType::ExType => Identifier::TypeId(0xffff),
            ItemType::AutoMapperConfig => Identifier::Uuid([
                0x3e, 0x1b, 0x27, 0x16, 0x17, 0x8c, 0x39, 0x78, 0x9b, 0xd9, 0xb1, 0x1a, 0xe0, 0x41,
                0xd, 0xd8,
            ]),
        }
    }
}

#[derive(Error, Debug)]
#[error(transparent)]
enum MapParseErrorKind {
    #[error("Item data is too short. Required length is {expected}, actual length: {actual}")]
    TooShort {
        expected: usize,
        actual: usize,
    },
    #[error("Item data is too long. Maximum known length is {expected}, actual length: {actual}")]
    TooLong {
        expected: usize,
        actual: usize,
    },
    #[error("Expected a single item, got {0}")]
    ItemCount(usize),
    Version(#[from] VersionError),
    String(#[from] StringParseError),
    NumConversion(#[from] NumConversionError),
    BoolConversion(#[from] BoolConversionError),
    OptIndex(#[from] OptIndexError),
    DataItem(#[from] DataItemError),
    ExType(#[from] ExTypeError),
    Image(#[from] ImageError),
    Envelope(#[from] EnvelopeError),
    EnvPoint(#[from] EnvPointError),
    Group(#[from] GroupError),
    Layer(#[from] LayerError),
    Automapper(#[from] AutomapperError),
    Sound(#[from] SoundError),
    Recursive(#[from] Box<MapParseError>),
}

// START OF PARSER HELPERS

impl Item {
    fn require_length(&self, expected: usize) -> Result<(), MapParseErrorKind> {
        if self.item_data.len() < expected {
            Err(MapParseErrorKind::TooShort {
                expected,
                actual: self.item_data.len(),
            })
        } else {
            Ok(())
        }
    }

    fn require_max_length(&self, expected: usize) -> Result<(), MapParseErrorKind> {
        if self.item_data.len() > expected {
            Err(MapParseErrorKind::TooLong {
                expected,
                actual: self.item_data.len(),
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Error, Debug)]
struct NumConversionError {
    ident: &'static str,
    value: i32,
}

impl fmt::Display for NumConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.value.is_negative() {
            write!(
                f,
                "'Value '{}' ({}) can't be negative",
                self.ident, self.value
            )
        } else {
            write!(f, "Value '{}' ({}) is too large", self.ident, self.value)
        }
    }
}

fn checked_convert<T>(value: i32, ident: &'static str) -> Result<T, NumConversionError>
where
    i32: CheckedCast<T>,
{
    match value.checked_as::<T>() {
        None => Err(NumConversionError { ident, value }),
        Some(n) => Ok(n),
    }
}

#[derive(Error, Debug)]
struct BoolConversionError {
    ident: &'static str,
    value: i32,
}

impl fmt::Display for BoolConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Boolean value '{}' ({}) is neither 0 nor 1",
            self.ident, self.value
        )
    }
}

fn convert_bool(value: i32, ident: &'static str) -> Result<bool, BoolConversionError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(BoolConversionError { ident, value }),
    }
}

#[derive(Error, Debug)]
struct OptIndexError {
    value: i32,
    ident: &'static str,
}

impl fmt::Display for OptIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Optional index '{}' ({}) is neither -1 or in its u16 range",
            self.ident, self.value,
        )
    }
}

fn convert_opt_index(value: i32, ident: &'static str) -> Result<Option<u16>, OptIndexError> {
    if value == -1 {
        Ok(None)
    } else {
        match value.checked_as() {
            Some(n) => Ok(Some(n)),
            None => Err(OptIndexError { value, ident }),
        }
    }
}

#[derive(Error, Debug)]
enum VersionError {
    #[error("The item is too short (length {length}) to even include the version which would be at index {index}")]
    TooShortItem { index: usize, length: usize },
    #[error("Two items present mismatching versions: All former ones had version {expected}, this one has version {actual}")]
    DifferentVersions { expected: i32, actual: i32 },
    #[error("Unsupported version {actual}, supported are versions from {} up to {}", .expected.start(), .expected.end())]
    Invalid {
        expected: RangeInclusive<i32>,
        actual: i32,
    },
}

struct VersionChecker {
    index: usize,
    versions: RangeInclusive<i32>,
}

impl VersionChecker {
    fn check(&self, items: &[Item], item_type: ItemType) -> Result<Option<i32>, MapParseError> {
        let mut expected_version = None;
        for (index, item) in items.iter().enumerate() {
            let version = match item.item_data.get(self.index) {
                None => {
                    return Err(MapParseError {
                        item_type,
                        index: None,
                        kind: VersionError::TooShortItem {
                            index: self.index,
                            length: item.item_data.len(),
                        }
                        .into(),
                    })
                }
                Some(n) => *n,
            };
            match expected_version {
                None => expected_version = Some(version),
                Some(expected) => {
                    if version != expected {
                        return Err(MapParseError {
                            item_type,
                            index: Some(index),
                            kind: VersionError::DifferentVersions {
                                expected,
                                actual: version,
                            }
                            .into(),
                        });
                    }
                }
            }
        }
        if let Some(version) = expected_version {
            if !self.versions.contains(&version) {
                return Err(MapParseError {
                    item_type,
                    index: None,
                    kind: VersionError::Invalid {
                        expected: self.versions.clone(),
                        actual: version,
                    }
                    .into(),
                });
            }
        }
        Ok(expected_version)
    }
}

#[derive(Error, Debug)]
enum DataItemError {
    #[error("Decompression failed: {0}")]
    Decompression(compression::ZlibDecompressionError),
    #[error("The data index is negative ({0})")]
    NegativeIndex(i32),
    #[error("The data index ({value}) is out of bounds, there are {length} data items")]
    OutOfBounds { length: usize, value: usize },
}

impl Datafile<'_> {
    fn get_items(&self, ex_index: &ExTypeIndex, item_type: ItemType) -> Option<&Vec<Item>> {
        let type_id = match item_type.identifier() {
            Identifier::TypeId(id) => id,
            Identifier::Uuid(uuid) => match ex_index.get(&uuid) {
                None => return None,
                Some(id) => *id,
            },
        };
        self.items.get(&type_id)
    }

    fn data_item(&self, index: i32) -> Result<(&[u8], usize), DataItemError> {
        let index = match index {
            i32::MIN..=-1 => return Err(DataItemError::NegativeIndex(index)),
            0..=i32::MAX => index.unwrapped_as::<usize>(),
        };
        match self.data_items.get(index) {
            None => Err(DataItemError::OutOfBounds {
                length: self.data_items.len(),
                value: index,
            }),
            Some((data, size)) => Ok((data.as_ref(), *size)),
        }
    }

    fn optional_data_item(&self, index: i32) -> Result<Option<(&[u8], usize)>, DataItemError> {
        match index {
            -1 => Ok(None),
            _ => Ok(Some(self.data_item(index)?)),
        }
    }

    fn decompressed_data_item(&self, index: i32) -> Result<Vec<u8>, DataItemError> {
        let (compressed_data, size) = self.data_item(index)?;
        let decompressed_data = match compression::decompress(compressed_data, size) {
            Err(err) => return Err(DataItemError::Decompression(err)),
            Ok(data) => data,
        };
        Ok(decompressed_data)
    }

    fn optional_decompressed_data_item(
        &self,
        index: i32,
    ) -> Result<Option<Vec<u8>>, DataItemError> {
        match index {
            -1 => Ok(None),
            _ => Ok(Some(self.decompressed_data_item(index)?)),
        }
    }
}

#[derive(Error, Debug)]
enum StringParseError {
    #[error("String was missing a nul byte")]
    NulByte,
}

fn parse_lossy_utf_string(data: &[u8], max_len: usize) -> String {
    let mut string = String::from_utf8_lossy(data).to_string();
    if string.len() > max_len {
        log::warn!("A string contains invalid utf8 ('{}'). The invalid bytes are replaced with replacement characters, which resulted in a too long string. It will be cut off at the max size", string);
    }
    while string.len() > max_len {
        string.pop();
    }
    string
}

fn parse_c_string(data: &[u8], max_len: usize) -> Result<String, StringParseError> {
    // Remove nul byte first
    if data.last() != Some(&0) {
        return Err(StringParseError::NulByte);
    }
    Ok(parse_lossy_utf_string(&data[..data.len() - 1], max_len))
}

fn parse_i32_string(numbers: &[i32]) -> Result<String, StringParseError> {
    let mut string = Vec::new();
    for &n in numbers {
        string.extend_from_slice(&n.to_be_bytes());
    }

    if string.iter().all(|&c| c == 0) {
        log::info!("Zeroed i32-string found (rare bug on old maps)");
        return Ok(String::new());
    }

    // Pop guaranteed nul byte
    if string.pop() != Some(0) {
        return Err(StringParseError::NulByte);
    }
    string = string.into_iter().map(|x| x.wrapping_add(128)).collect();
    // pop remaining nul bytes
    while string.last() == Some(&0) {
        string.pop();
    }
    Ok(parse_lossy_utf_string(&string, numbers.len() * 4 - 1))
}

// START OF PARSING

struct ExType {
    uuid: [u8; 16],
    type_id: u16,
}

type ExTypeIndex = HashMap<[u8; 16], u16>;

#[derive(Error, Debug)]
enum ExTypeError {
    #[error("Two different type ids were assigned to the same item type")]
    DuplicateUuid,
    #[error("The datafile has no items for the newly assigned type id")]
    MissingItemType,
}

impl ItemParseErrorTrait for ExType {
    const KIND: ItemType = ItemType::ExType;
    type State = ExTypeIndex;

    fn parse_impl(
        item: &Item,
        ex_index: &mut Self::State,
        df: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        item.require_length(4)?;
        item.require_max_length(4)?;
        let mut uuid = <[u8; 16]>::default();
        for (i, n) in item.item_data.iter().enumerate() {
            uuid[i * 4..(i + 1) * 4].copy_from_slice(&n.to_be_bytes());
        }
        if ex_index.contains_key(&uuid) {
            if ex_index[&uuid] == item.id {
                log::info!("Duplicate announcement of extension type");
            } else {
                return Err(ExTypeError::DuplicateUuid.into());
            }
        }
        if !df.items.contains_key(&item.id) {
            return Err(ExTypeError::MissingItemType.into());
        }
        ex_index.insert(uuid, item.id);
        Ok(Self {
            uuid,
            type_id: item.id,
        })
    }
}

pub(crate) struct MapVersion {
    _version: i32,
}

impl ItemParseErrorTrait for MapVersion {
    const KIND: ItemType = ItemType::Version;
    const VERSION_CHECK: Option<VersionChecker> = Some(VersionChecker {
        index: 0,
        versions: 1..=1,
    });
    type State = ();

    fn parse_impl(
        item: &Item,
        _: &mut (),
        _: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        item.require_length(1)?;
        item.require_max_length(1)?;
        let _version = item.item_data[0];
        Ok(MapVersion { _version })
    }
}

impl ItemParseErrorTrait for Info {
    const KIND: ItemType = ItemType::Info;
    const VERSION_CHECK: Option<VersionChecker> = Some(VersionChecker {
        index: 0,
        versions: 1..=1,
    });
    type State = ();

    fn parse_impl(
        item: &Item,
        _: &mut (),
        df: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        item.require_length(5)?;
        let max_lengths = [
            Info::MAX_AUTHOR_LENGTH,
            Info::MAX_VERSION_LENGTH,
            Info::MAX_CREDITS_LENGTH,
            Info::MAX_LICENSE_LENGTH,
        ];
        let mut strings = Vec::new();
        for (&data_index, &max_len) in item.item_data[1..5].iter().zip(max_lengths.iter()) {
            if let Some(data) = df.optional_decompressed_data_item(data_index)? {
                strings.push(parse_c_string(&data, max_len)?);
            } else {
                strings.push(String::new());
            }
        }

        let mut settings_strings = Vec::new();
        if item.item_data.len() > 5 {
            item.require_length(6)?;
            item.require_max_length(6)?;
            if let Some(mut data) = df.optional_decompressed_data_item(item.item_data[5])? {
                match data.pop() {
                    None => {}
                    Some(0) => {
                        for string_data in data.split(|&x| x == 0) {
                            let string = String::from_utf8_lossy(string_data).to_string();
                            settings_strings.push(string);
                        }
                    }
                    Some(_) => return Err(StringParseError::NulByte.into()),
                }
            }
        }

        Ok(Self {
            author: strings.remove(0),
            version: strings.remove(0),
            credits: strings.remove(0),
            license: strings.remove(0),
            settings: settings_strings,
        })
    }
}

#[derive(Error, Debug)]
enum ImageError {
    #[error("The 'external' bool is redundant and didn't match the optional data item index")]
    ExternalBool,
    #[error("Variant is set to another one than RGBA")]
    InvalidVariant,
}

impl ItemParseErrorTrait for Image {
    const KIND: ItemType = ItemType::Image;
    const VERSION_CHECK: Option<VersionChecker> = Some(VersionChecker {
        index: 0,
        versions: 1..=2,
    });
    type State = ();

    fn parse_impl(
        item: &Item,
        _: &mut (),
        df: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        item.require_length(6)?;
        let version = item.item_data[0];
        let width = checked_convert(item.item_data[1], "width")?;
        let height = checked_convert(item.item_data[2], "height")?;
        let size = Extent2::new(width, height);

        let name_data = df.decompressed_data_item(item.item_data[4])?;
        let name = parse_c_string(&name_data, Image::MAX_NAME_LENGTH)?;

        let compressed_image =
            df.optional_data_item(item.item_data[5])?
                .map(|(data, data_size)| {
                    CompressedData::Compressed(data.to_vec(), data_size, ImageLoadInfo { size })
                });

        let is_external = convert_bool(item.item_data[3], "is_external")?;
        if is_external != compressed_image.is_none() {
            return Err(ImageError::ExternalBool.into());
        }

        if version > 1 {
            item.require_length(7)?;
            item.require_max_length(7)?;
            if item.item_data[6] != 1 {
                return Err(ImageError::InvalidVariant.into());
            }
        } else {
            item.require_max_length(6)?;
        }

        Ok(match compressed_image {
            Some(compressed_image) => EmbeddedImage {
                name,
                image: compressed_image,
            }
            .into(),
            None => ExternalImage { size, name }.into(),
        })
    }
}

#[derive(Error, Debug)]
enum EnvelopeError {
    #[error("The env points range overlaps with the previous one")]
    Overlap,
    #[error("The env point range leaves some points orphaned between itself and the range of the previous envelope")]
    Gap,
    #[error(
        "{amount} env points declared, while there are only {remaining} left to be distributed"
    )]
    TooHighAmount { amount: i32, remaining: i32 },
    #[error("In the first version of the envelope item, the name i32 always has to be -1")]
    FirstVersionName,
    #[error("Unknown type of envelope: {0}")]
    InvalidType(i32),
}

const ENV_VERSION_CHECKER: VersionChecker = VersionChecker {
    index: 0,
    versions: 1..=3,
};

fn env_point_length(env_version: i32) -> usize {
    match env_version {
        1 | 2 => 6,
        3 => 22,
        _ => unreachable!(),
    }
}

impl ItemParseErrorTrait for Envelope {
    const KIND: ItemType = ItemType::Envelope;
    const VERSION_CHECK: Option<VersionChecker> = Some(ENV_VERSION_CHECKER);
    type State = i32;

    /// Assumes that the envelope points are parsed before this function is called
    fn parse_impl(
        item: &Item,
        expected_start: &mut i32,
        df: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        item.require_length(5)?;
        let version = item.item_data[0];
        let start = item.item_data[2];
        match start.cmp(expected_start) {
            Ordering::Less => return Err(EnvelopeError::Overlap.into()),
            Ordering::Equal => {}
            Ordering::Greater => return Err(EnvelopeError::Gap.into()),
        }

        let bytes_per_env_point = env_point_length(version);
        let total_points = df
            .get_items(&HashMap::new(), ItemType::EnvPoints)
            .unwrap()
            .first()
            .unwrap()
            .item_data
            .len()
            / bytes_per_env_point;
        let total_points_i32 = total_points.unwrapped_as::<i32>();
        let remaining_points = total_points_i32 - *expected_start;
        let amount = item.item_data[3];
        let amount_usize = checked_convert::<usize>(amount, "envelope point amount")?;
        if amount > remaining_points {
            return Err(EnvelopeError::TooHighAmount {
                amount,
                remaining: remaining_points,
            }
            .into());
        }
        *expected_start += amount;

        let mut name = String::new();
        let mut synchronized = false;

        if item.item_data.len() > 5 {
            item.require_length(12)?;
            name = parse_i32_string(&item.item_data[4..12])?;
            if version >= 2 {
                item.require_length(13)?;
                item.require_max_length(13)?;
                synchronized = convert_bool(item.item_data[12], "synchronized")?;
            } else {
                item.require_max_length(12)?;
            }
        } else if item.item_data[4] != -1 {
            return Err(EnvelopeError::FirstVersionName.into());
        }
        Ok(match item.item_data[1] {
            1 => Envelope::Sound(Env {
                name,
                synchronized,
                points: vec![
                    EnvPoint {
                        time: 0,
                        content: Default::default(),
                        curve: CurveKind::Step,
                    };
                    amount_usize
                ],
            }),
            3 => Envelope::Position(Env {
                name,
                synchronized,
                points: vec![
                    EnvPoint {
                        time: 0,
                        content: Default::default(),
                        curve: CurveKind::Step,
                    };
                    amount_usize
                ],
            }),
            4 => Envelope::Color(Env {
                name,
                synchronized,
                points: vec![
                    EnvPoint {
                        time: 0,
                        content: Default::default(),
                        curve: CurveKind::Step,
                    };
                    amount_usize
                ],
            }),
            _ => return Err(EnvelopeError::InvalidType(item.item_data[1]).into()),
        })
    }
}

#[derive(Error, Debug)]
enum EnvPointError {
    #[error("No envelopes, but envelope points")]
    PointWithoutEnvs,
    #[error("Data length {length} is not divisible by the size of a single point ({point_len})")]
    DataLen { length: usize, point_len: usize },
    #[error("Data length {0} is zero or not divisible by the bezier data size 16")]
    BezierDataLen(usize),
    #[error("{0} points left after the env points got distributed to the envelopes")]
    RemainingPoints(usize),
    #[error("Both Teeworlds bezier data as well as DDNet bezier data available")]
    DuplicateBezierData,
    #[error("{expected} envelope points, but {value} bezier curves from DDNet data")]
    BezierCurveAmount { expected: usize, value: usize },
    #[error("Curve type bezier, but no bezier data is available")]
    BezierNoData,
}

impl EnvPoint<[i32; 4]> {
    fn parse(data: (&[i32], Option<&BezierCurve<[i32; 4]>>)) -> Result<Self, EnvPointError> {
        let time = data.0[0];
        let content = data.0[2..6].try_into().unwrap();
        let curve = CurveKind::parse(data.0[1], data.1)?;
        Ok(Self {
            time,
            content,
            curve,
        })
    }
}

impl BezierCurve<[i32; 4]> {
    fn parse_all(data: &[i32]) -> Result<Vec<Self>, EnvPointError> {
        if data.is_empty() {
            log::info!("Zero-sized DDNet bezier data");
            return Ok(Vec::new());
        }
        // I would've liked to insert checks for non-zero, unused bezier data
        // However, most maps failed that check for some reason
        if data.len() % 16 != 0 {
            return Err(EnvPointError::BezierDataLen(data.len()));
        }
        if data[0..8].iter().any(|&x| x != 0) {
            //return Err(EnvPointError::NonZeroUnusedBezier);
        }
        let mut last_data = [0; 16];
        last_data[..8].copy_from_slice(&data[data.len() - 8..]);
        let last = BezierCurve::parse(&last_data);
        Ok(data[8..data.len() - 8]
            .chunks(16)
            .map(|data| Self::parse(data.try_into().unwrap()))
            .chain(Some(last))
            .collect())
    }
}

impl BezierCurve<[i32; 4]> {
    fn parse(data: &[i32; 16]) -> Self {
        Self {
            handle_l: Vec2::new(
                data[0..4].try_into().unwrap(),
                data[4..8].try_into().unwrap(),
            ),
            handle_r: Vec2::new(
                data[8..12].try_into().unwrap(),
                data[12..16].try_into().unwrap(),
            ),
        }
    }
}
impl CurveKind<[i32; 4]> {
    fn parse(id: i32, bezier: Option<&BezierCurve<[i32; 4]>>) -> Result<Self, EnvPointError> {
        Ok(match id {
            0 => CurveKind::Step,
            1 => CurveKind::Linear,
            2 => CurveKind::Slow,
            3 => CurveKind::Fast,
            4 => CurveKind::Smooth,
            5 => match bezier {
                Some(bezier) => CurveKind::Bezier(*bezier),
                None => return Err(EnvPointError::BezierNoData),
            },
            _ => CurveKind::Unknown(id),
        })
    }
}

impl ItemParseErrorTrait for Vec<EnvPoint<[i32; 4]>> {
    const KIND: ItemType = ItemType::EnvPoints;
    type State = ();

    fn parse_impl(
        item: &Item,
        _: &mut (),
        df: &Datafile,
        ex_index: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        let mut bezier_curves =
            match <Vec<BezierCurve<[i32; 4]>>>::parse_single_item_only_or_default(df, ex_index) {
                Err(err) => return Err(MapParseErrorKind::Recursive(Box::new(err))),
                Ok(bezier_curves) => bezier_curves,
            };
        let envelope_items = df.get_items(&HashMap::new(), ItemType::Envelope);
        let envelope_version = match envelope_items {
            None => {
                return if item.item_data.is_empty() && bezier_curves.is_empty() {
                    Ok(Vec::new())
                } else {
                    Err(EnvPointError::PointWithoutEnvs.into())
                }
            }
            Some(items) => ENV_VERSION_CHECKER
                .check(items, ItemType::Envelope)
                .map_err(Box::new)?
                .unwrap(), // TODO: is this unwrap okay?
        };
        if envelope_version >= 3 && !bezier_curves.is_empty() {
            return Err(EnvPointError::DuplicateBezierData.into());
        }
        let size = env_point_length(envelope_version);
        if item.item_data.len() % size != 0 {
            return Err(EnvPointError::DataLen {
                length: item.item_data.len(),
                point_len: size,
            }
            .into());
        }
        let env_point_amount: usize = (item.item_data.len() / size).unwrapped_as();
        if !bezier_curves.is_empty() && env_point_amount != bezier_curves.len() {
            return Err(EnvPointError::BezierCurveAmount {
                expected: env_point_amount,
                value: bezier_curves.len(),
            }
            .into());
        }
        // Extract Teeworlds 0.7 bezier data
        if envelope_version >= 3 {
            let bezier_curves_data: Vec<i32> = item
                .item_data
                .chunks(size)
                .flat_map(|data| data[6..].iter().copied())
                .collect();
            bezier_curves = BezierCurve::parse_all(&bezier_curves_data)?;
        }
        // Add Option and `None`s to represent no available bezier data
        let bezier_curves = bezier_curves
            .iter()
            .map(Some)
            .chain(std::iter::repeat(None));
        let env_points = item
            .item_data
            .chunks(size)
            .zip(bezier_curves)
            .map(EnvPoint::parse)
            .collect::<Result<_, _>>()?;
        Ok(env_points)
    }
}

impl ItemParseErrorTrait for Vec<BezierCurve<[i32; 4]>> {
    const KIND: ItemType = ItemType::EnvPointsBezierData;
    type State = ();

    fn parse_impl(
        item: &Item,
        _: &mut (),
        _: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        Ok(BezierCurve::parse_all(&item.item_data)?)
    }
}

trait FromChannels {
    fn from_channels(vals: [i32; 4]) -> Self;
}

impl FromChannels for Position {
    fn from_channels(vals: [i32; 4]) -> Self {
        Position {
            offset: Vec2::new(I17F15::from_bits(vals[0]), I17F15::from_bits(vals[1])),
            rotation: I22F10::from_bits(vals[2]),
        }
    }
}

impl FromChannels for Rgba<I22F10> {
    fn from_channels(vals: [i32; 4]) -> Self {
        Self {
            r: I22F10::from_bits(vals[0]),
            g: I22F10::from_bits(vals[1]),
            b: I22F10::from_bits(vals[2]),
            a: I22F10::from_bits(vals[3]),
        }
    }
}

impl FromChannels for Volume {
    fn from_channels(vals: [i32; 4]) -> Self {
        Volume(I22F10::from_bits(vals[0]))
    }
}

fn convert_curve_kind<T: FromChannels>(curve: CurveKind<[i32; 4]>) -> CurveKind<T> {
    use CurveKind::*;
    match curve {
        Step => Step,
        Linear => Linear,
        Slow => Slow,
        Fast => Fast,
        Smooth => Smooth,
        Bezier(b) => Bezier(BezierCurve {
            handle_l: b.handle_l.map(T::from_channels),
            handle_r: b.handle_r.map(T::from_channels),
        }),
        Unknown(id) => Unknown(id),
    }
}

fn convert_env_points<T: FromChannels>(points: &[EnvPoint<[i32; 4]>]) -> Vec<EnvPoint<T>> {
    points
        .iter()
        .map(|point| EnvPoint {
            time: point.time,
            content: T::from_channels(point.content),
            curve: convert_curve_kind(point.curve),
        })
        .collect()
}

impl EnvPoint<[i32; 4]> {
    fn distribute(points: Vec<Self>, envelopes: &mut Vec<Envelope>) -> Result<(), MapParseError> {
        let mut remaining = points.as_slice();
        for env in envelopes {
            match env {
                Envelope::Position(env) => {
                    let (curr, rem) = remaining.split_at(env.points.len());
                    remaining = rem;
                    env.points = convert_env_points(curr);
                }
                Envelope::Color(env) => {
                    let (curr, rem) = remaining.split_at(env.points.len());
                    remaining = rem;
                    env.points = convert_env_points(curr);
                }
                Envelope::Sound(env) => {
                    let (curr, rem) = remaining.split_at(env.points.len());
                    remaining = rem;
                    env.points = convert_env_points(curr);
                }
            }
        }
        if remaining.is_empty() {
            Ok(())
        } else {
            Err(MapParseError {
                item_type: ItemType::EnvPoints,
                index: None,
                kind: EnvPointError::RemainingPoints(remaining.len()).into(),
            })
        }
    }
}

#[derive(Error, Debug)]
enum GroupError {
    #[error("The layer range overlaps with the previous group")]
    Overlap,
    #[error("The layers range leaves some layers orphaned between itself and the range of the previous group")]
    Gap,
    #[error("{amount} layers declared, while there are only {remaining} left to be distributed")]
    TooHighAmount { amount: i32, remaining: i32 },
}

impl ItemParseErrorTrait for Group {
    const KIND: ItemType = ItemType::Group;
    const VERSION_CHECK: Option<VersionChecker> = Some(VersionChecker {
        index: 0,
        versions: 1..=3,
    });
    type State = i32;

    fn parse_impl(
        item: &Item,
        expected_start: &mut i32,
        df: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        item.require_length(7)?;
        let version = item.item_data[0];

        let start = item.item_data[5];
        match start.cmp(expected_start) {
            Ordering::Less => return Err(GroupError::Overlap.into()),
            Ordering::Equal => {}
            Ordering::Greater => return Err(GroupError::Gap.into()),
        };

        let total_layers = df
            .get_items(&HashMap::new(), ItemType::Layer)
            .map(|items| items.len())
            .unwrap_or(0);
        let total_layers_i32 = total_layers.unwrapped_as::<i32>();
        let remaining_layers = total_layers_i32 - *expected_start;
        let amount = item.item_data[6];
        let amount_usize = checked_convert::<usize>(amount, "layer amount")?;
        if amount > remaining_layers {
            return Err(GroupError::TooHighAmount {
                amount,
                remaining: remaining_layers,
            }
            .into());
        }
        *expected_start += amount;

        let mut clipping = false;
        let mut clip_x: I27F5 = I27F5::zero();
        let mut clip_y = I27F5::zero();
        let mut clip_width = I27F5::zero();
        let mut clip_height = I27F5::zero();
        let mut name = String::new();
        if version >= 2 {
            item.require_length(12)?;
            clipping = convert_bool(item.item_data[7], "clipping")?;
            clip_x = I27F5::from_bits(item.item_data[8]);
            clip_y = I27F5::from_bits(item.item_data[9]);
            clip_width = I27F5::from_bits(item.item_data[10]);
            clip_height = I27F5::from_bits(item.item_data[11]);
            if !clipping && clip_width.is_negative() {
                log::warn!(
                    "Negative group clip width ({}), but with clipping disabled, setting to zero",
                    clip_width
                );
                clip_width = I27F5::zero();
            }
            if !clipping && clip_height.is_negative() {
                log::warn!(
                    "Negative group clip height ({}), but with clipping disabled, setting to zero",
                    clip_height
                );
                clip_height = I27F5::zero();
            }

            if version >= 3 {
                item.require_length(15)?;
                item.require_max_length(15)?;
                name = parse_i32_string(&item.item_data[12..15])?;
            } else {
                item.require_max_length(12)?;
            }
        } else {
            item.require_max_length(7)?;
        }
        let parallax = Vec2::new(item.item_data[3], item.item_data[4]);
        Ok(Group {
            name,
            offset: Vec2::new(
                I27F5::from_bits(item.item_data[1]),
                I27F5::from_bits(item.item_data[2]),
            ),
            parallax,
            layers: vec![Layer::Invalid(InvalidLayerKind::NoType); amount_usize],
            clipping,
            clip: Rect::new(clip_x, clip_y, clip_width, clip_height),
        })
    }
}

impl TwMap {
    // TODO: use this
    fn correct_physics_group_name(&mut self) {
        if self
            .groups
            .iter()
            .filter(|group| group.is_physics_group())
            .count()
            == 1
        {
            let physics_group = self.physics_group_mut();
            if physics_group.name != "Game" {
                log::info!(
                    "Correcting the physics group name from '{}' to 'Game'",
                    physics_group.name
                );
                physics_group.name = "Game".into();
            }
        }
    }
}

#[derive(Error, Debug)]
enum LayerError {
    #[error("Unknown tilemap layer variant: {0}")]
    UnknownTilemapLayer(i32),
    #[error("Unknown layer variant: {0} (this is not a tilemap layer)")]
    UnknownLayer(i32),
    #[error("The layer flags contain an unknown flag")]
    UnknownFlag,
    #[error("The vanilla compatibility data has an invalid size")]
    CompatibilityDataLen,
    #[error("The vanilla compatibility data isn't zeroed out")]
    CompatibilityDataValues,
    #[error("Physics layer default value for {0:?} isn't the default value")]
    PhysicsValue(PhysicsLayerValue),
    #[error("After all groups received their layers, there are {0} layers left")]
    Remaining(usize),
    #[error("Quads data length is not divisible by the size of a single quad")]
    QuadsDataLen,
    #[error("Quads data provides a different amount of quads than announced")]
    QuadsAmount,
    #[error("Unknown Shape ({0})")]
    UnknownShape(i32),
    #[error("Source data length is not divisible by the size of a single source")]
    SourceDataLen,
    #[error("Source data provides a different amount of sources than announced")]
    SourceAmount,
    #[error("Not a single tilemap layer")]
    NoTilemap,
}

#[derive(Debug)]
enum PhysicsLayerValue {
    Color,
    ColorEnvelope,
    ColorEnvelopeOffset,
    Image,
}

impl Item {
    fn layer_kind(&self) -> Result<LayerKind, MapParseErrorKind> {
        use LayerKind::*;
        self.require_length(2)?;
        Ok(match self.item_data[1] {
            // so called 'LAYERTYPE'
            2 => {
                self.require_length(7)?;
                match self.item_data[6] {
                    // so called 'TILESLAYERFLAG'
                    0 => Tiles,
                    1 => Game,
                    2 => Tele,
                    4 => Speedup,
                    8 => Front,
                    16 => Switch,
                    32 => Tune,
                    _ => return Err(LayerError::UnknownTilemapLayer(self.item_data[6]).into()),
                }
            }
            3 => Quads,
            9 | 10 => Sounds,
            _ => return Err(LayerError::UnknownLayer(self.item_data[1]).into()),
        })
    }
}

const TILEMAP_LAYER_VERSION_CHECKER: VersionChecker = VersionChecker {
    index: 3,
    versions: 1..=4,
};

const QUADS_LAYER_VERSION_CHECKER: VersionChecker = VersionChecker {
    index: 3,
    versions: 1..=2,
};

const SOUNDS_LAYER_VERSION_CHECKER: VersionChecker = VersionChecker {
    index: 3,
    versions: 2..=2,
};

impl Layer {
    fn check_kind<T: Fn(LayerKind) -> bool>(
        items: &[Item],
        checker: VersionChecker,
        selector: T,
    ) -> Result<Option<i32>, MapParseError> {
        let relevant_items: Vec<Item> = items
            .iter()
            .filter(|item| selector(item.layer_kind().unwrap()))
            .cloned()
            .collect();
        let relevant_items_indices: Vec<usize> = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| selector(item.layer_kind().unwrap()).then_some(index))
            .collect();
        let version = checker
            .check(&relevant_items, Self::KIND)
            .map_err(|mut err| {
                if let Some(i) = err.index.as_mut() {
                    *i = relevant_items_indices[*i];
                }
                err
            })?;
        Ok(version)
    }

    fn check_versions(df: &Datafile, ex_index: &ExTypeIndex) -> Result<Option<i32>, MapParseError> {
        let items = match df.get_items(ex_index, ItemType::Layer) {
            None => return Ok(None),
            Some(items) => items,
        };
        for (index, item) in items.iter().enumerate() {
            item.layer_kind().map_err(|kind| MapParseError {
                item_type: ItemType::Layer,
                index: Some(index),
                kind,
            })?;
        }

        let tilemap_version = Self::check_kind(items, TILEMAP_LAYER_VERSION_CHECKER, |kind| {
            kind.is_tile_map_layer()
        })?;
        Self::check_kind(items, QUADS_LAYER_VERSION_CHECKER, |kind| {
            kind == LayerKind::Quads
        })?;
        Self::check_kind(items, SOUNDS_LAYER_VERSION_CHECKER, |kind| {
            kind == LayerKind::Sounds
        })?;

        Ok(tilemap_version)
    }
}

bitflags! {
    pub(crate) struct LayerFlags: i32 {
        const DETAIL = 0b1;
    }
}

impl LayerKind {
    pub(crate) fn data_index(&self) -> usize {
        use LayerKind::*;
        match self {
            Game | Tiles => 14,
            Front => 20,
            Tele => 18,
            Speedup => 19,
            Switch => 21,
            Tune => 22,
            _ => panic!(),
        }
    }
}

impl TilesLayer {
    fn parse_generic(
        item: &Item,
        df: &Datafile,
    ) -> Result<(TilesLayer, LayerKind), MapParseErrorKind> {
        use LayerKind::*;
        let kind = item.layer_kind()?;
        debug_assert!(kind.is_physics_layer() || kind == Tiles);
        item.require_length(15)?;
        let version = item.item_data[3];

        let flags = match LayerFlags::from_bits(item.item_data[2]) {
            Some(flags) => flags,
            None => return Err(LayerError::UnknownFlag.into()),
        };

        let width = checked_convert(item.item_data[4], "width")?;
        let height = checked_convert(item.item_data[5], "height")?;
        let tile_amount = u64::from(width) * u64::from(height);
        let size: Extent2<u32> = Extent2::new(width, height);

        let color = Rgba {
            r: checked_convert(item.item_data[7], "Color component r")?,
            g: checked_convert(item.item_data[8], "Color component g")?,
            b: checked_convert(item.item_data[9], "Color component b")?,
            a: checked_convert(item.item_data[10], "Color component a")?,
        };
        let color_env = convert_opt_index(item.item_data[11], "color envelope")?;
        let color_env_offset = item.item_data[12];
        let image = convert_opt_index(item.item_data[13], "image")?;

        let compression = match kind {
            Game | Tiles => version >= 4,
            _ => false,
        };
        let mut data_index = kind.data_index();
        let mut name = String::new();
        if version < 3 {
            // name was missing then
            if data_index > 14 {
                data_index -= 3;
            }
        } else {
            item.require_length(18)?;
            name = parse_i32_string(&item.item_data[15..18])?;
        }

        item.require_length(data_index + 1)?;
        let (data, mut data_size) = df.data_item(item.item_data[data_index])?;
        let mut data = data.to_vec();

        if kind != Game && kind != Tiles {
            let compatibility_data = df.decompressed_data_item(item.item_data[14])?;
            let expected_size = tile_amount * 4;
            if compatibility_data.len().unwrapped_as::<u64>() != expected_size {
                return Err(LayerError::CompatibilityDataLen.into());
            }
            if compatibility_data.into_iter().any(|c| c != 0) {
                return Err(LayerError::CompatibilityDataValues.into());
            }
        }

        let old_tile_versions = match kind {
            Switch => SWITCH_VERSIONS.as_ref(),
            Speedup => SPEEDUP_VERSIONS.as_ref(),
            _ => &[],
        };
        for old_version in old_tile_versions {
            if let Some(updated) = old_version.try_update(&data, data_size, tile_amount) {
                let (updated_data, updated_data_size) = updated?;
                data = updated_data;
                data_size = updated_data_size;
            }
        }

        let load_info = TilesLoadInfo { size, compression };

        Ok((
            Self {
                name,
                detail: flags.contains(LayerFlags::DETAIL),
                color,
                color_env,
                color_env_offset,
                image,
                tiles: CompressedData::Compressed(data, data_size, load_info),
                automapper_config: AutomapperConfig::default(),
            },
            kind,
        ))
    }
}

struct OutdatedTileVersion {
    pub bytes_per_tile: usize,
    pub convert_fnc: fn(&[u8]) -> Vec<u8>,
}

fn convert_old_speedup(data: &[u8]) -> Vec<u8> {
    let mut speedup = [0; 6];
    speedup[0] = data[0];
    speedup[2] = 28;
    speedup[4] = data[2];
    speedup[5] = data[3];
    speedup.to_vec()
}

static SPEEDUP_VERSIONS: [OutdatedTileVersion; 1] = [OutdatedTileVersion {
    bytes_per_tile: 4,
    convert_fnc: convert_old_speedup,
}];

fn convert_tele_to_switch(data: &[u8]) -> Vec<u8> {
    let mut switch = [0; 4];
    switch[0] = data[0];
    switch[1] = data[1];
    switch.to_vec()
}

fn convert_old_switch(data: &[u8]) -> Vec<u8> {
    let mut switch = [0; 4];
    switch[0] = data[0];
    switch[1] = data[1];
    switch[2] = data[2];
    switch.to_vec()
}

static SWITCH_VERSIONS: [OutdatedTileVersion; 2] = [
    OutdatedTileVersion {
        bytes_per_tile: 2,
        convert_fnc: convert_tele_to_switch,
    },
    OutdatedTileVersion {
        bytes_per_tile: 3,
        convert_fnc: convert_old_switch,
    },
];

impl OutdatedTileVersion {
    fn try_update(
        &self,
        compressed_data: &[u8],
        data_size: usize,
        tile_amount: u64,
    ) -> Option<Result<(Vec<u8>, usize), MapParseErrorKind>> {
        let tile_count: usize = tile_amount.checked_as()?;
        let outdated_size = tile_count.checked_mul(self.bytes_per_tile)?;
        if data_size == outdated_size {
            let decompressed_data = match compression::decompress(compressed_data, data_size) {
                Err(err) => return Some(Err(DataItemError::Decompression(err).into())),
                Ok(data) => data,
            };
            let fnc = self.convert_fnc;
            let updated_data: Vec<u8> = decompressed_data
                .chunks(self.bytes_per_tile)
                .flat_map(fnc)
                .collect();
            Some(Ok((
                compression::compress(&updated_data),
                updated_data.len(),
            )))
        } else {
            None
        }
    }
}

trait PhysicsLayerFromTiles: PhysicsLayer {
    const STATIC_NAME: &'static str;

    fn from_tiles_direct(tiles: CompressedData<Array2<Self::TileType>, TilesLoadInfo>) -> Self;

    fn from_tiles_layer(layer: TilesLayer) -> Result<Self, LayerError> {
        if layer.name.as_str() != Self::STATIC_NAME {
            log::info!(
                "Physics layer is called '{}', should have the name {} instead",
                layer.name,
                Self::STATIC_NAME
            );
        }
        if layer.detail {
            log::warn!(
                "{:?} layer has detail enabled (disabled automatically)",
                Self::kind()
            );
        }
        if layer.color_env.is_some() {
            return Err(LayerError::PhysicsValue(PhysicsLayerValue::ColorEnvelope));
        }
        if layer.color_env_offset != 0 {
            return Err(LayerError::PhysicsValue(
                PhysicsLayerValue::ColorEnvelopeOffset,
            ));
        }
        if layer.color != Rgba::white() {
            return Err(LayerError::PhysicsValue(PhysicsLayerValue::Color));
        }
        if layer.image.is_some() {
            return Err(LayerError::PhysicsValue(PhysicsLayerValue::Image));
        }
        let tiles = if let CompressedData::Compressed(data, size, info) = layer.tiles {
            CompressedData::Compressed(data, size, info)
        } else {
            unreachable!()
        };
        Ok(Self::from_tiles_direct(tiles))
    }
}

macro_rules! physics_layer_from_tiles {
    ($layer_name:ident, $static_name:expr) => {
        impl PhysicsLayerFromTiles for $layer_name {
            const STATIC_NAME: &'static str = $static_name;

            fn from_tiles_direct(
                tiles: CompressedData<Array2<Self::TileType>, TilesLoadInfo>,
            ) -> Self {
                $layer_name { tiles }
            }
        }
    };
}

physics_layer_from_tiles!(GameLayer, "Game");
physics_layer_from_tiles!(FrontLayer, "Front");
physics_layer_from_tiles!(TeleLayer, "Tele");
physics_layer_from_tiles!(SpeedupLayer, "Speedup");
physics_layer_from_tiles!(SwitchLayer, "Switch");
physics_layer_from_tiles!(TuneLayer, "Tune");

impl TilesLayer {
    fn convert_to(self, kind: LayerKind) -> Result<Layer, MapParseErrorKind> {
        Ok(match kind {
            LayerKind::Game => Layer::Game(GameLayer::from_tiles_layer(self)?),
            LayerKind::Tiles => Layer::Tiles(self),
            LayerKind::Front => Layer::Front(FrontLayer::from_tiles_layer(self)?),
            LayerKind::Tele => Layer::Tele(TeleLayer::from_tiles_layer(self)?),
            LayerKind::Speedup => Layer::Speedup(SpeedupLayer::from_tiles_layer(self)?),
            LayerKind::Switch => Layer::Switch(SwitchLayer::from_tiles_layer(self)?),
            LayerKind::Tune => Layer::Tune(TuneLayer::from_tiles_layer(self)?),
            _ => unreachable!(),
        })
    }
}

#[repr(C)]
#[derive(View, Copy, Clone)]
struct BinaryColor {
    r: i32_le,
    g: i32_le,
    b: i32_le,
    a: i32_le,
}

impl BinaryColor {
    fn to_color(self) -> Result<Rgba<u8>, MapParseErrorKind> {
        Ok(Rgba {
            r: checked_convert(self.r.to_int(), "Color component r")?,
            g: checked_convert(self.g.to_int(), "Color component g")?,
            b: checked_convert(self.b.to_int(), "Color component b")?,
            a: checked_convert(self.a.to_int(), "Color component a")?,
        })
    }
}

#[repr(C)]
#[derive(View, Copy, Clone)]
struct BinaryPoint {
    x: i32_le,
    y: i32_le,
}

impl BinaryPoint {
    fn to_2<T: Fixed<Bits = i32>, U: From<Vec2<T>>>(self) -> U {
        Vec2 {
            x: T::from_bits(self.x.to_int()),
            y: T::from_bits(self.y.to_int()),
        }
        .into()
    }
}

#[repr(C)]
#[derive(View, Copy, Clone)]
pub(crate) struct BinaryQuad {
    corners: [BinaryPoint; 4],
    position: BinaryPoint,
    colors: [BinaryColor; 4],
    texture_coords: [BinaryPoint; 4],

    position_env: i32_le,
    position_env_offset: i32_le,
    color_env: i32_le,
    color_env_offset: i32_le,
}

impl BinaryQuad {
    fn to_quad(self) -> Result<Quad, MapParseErrorKind> {
        let mut corners = <[Vec2<_>; 4]>::default();
        let mut colors = <[Rgba<u8>; 4]>::default();
        let mut texture_coords = <[Uv<_>; 4]>::default();
        for i in 0..4 {
            corners[i] = self.corners[i].to_2();
            colors[i] = self.colors[i].to_color()?;
            texture_coords[i] = self.texture_coords[i].to_2();
        }
        Ok(Quad {
            corners,
            position: self.position.to_2(),
            colors,
            texture_coords,
            position_env: convert_opt_index(self.position_env.to_int(), "position envelope")?,
            position_env_offset: self.position_env_offset.to_int(),
            color_env: convert_opt_index(self.color_env.to_int(), "color envelope")?,
            color_env_offset: self.color_env_offset.to_int(),
        })
    }
}

impl QuadsLayer {
    fn parse(item: &Item, df: &Datafile) -> Result<Layer, MapParseErrorKind> {
        item.require_length(7)?;
        let version = item.item_data[3];
        let flags = match LayerFlags::from_bits(item.item_data[2]) {
            Some(flags) => flags,
            None => return Err(LayerError::UnknownFlag.into()),
        };

        let quad_amount: usize = checked_convert(item.item_data[4], "quad amount")?;
        let quad_data = df.decompressed_data_item(item.item_data[5])?;
        if quad_data.len() % mem::size_of::<BinaryQuad>() != 0 {
            return Err(LayerError::QuadsDataLen.into());
        }
        let implied_amount = quad_data.len() / mem::size_of::<BinaryQuad>();
        let quads: Vec<Quad> =
            if quad_amount == 0 && implied_amount == 1 && quad_data.iter().all(|b| *b == 0) {
                Vec::new()
            } else {
                if implied_amount != quad_amount {
                    return Err(LayerError::QuadsAmount.into());
                }
                quad_data
                    .chunks(mem::size_of::<BinaryQuad>())
                    .map(|data| BinaryQuad::view(data).unwrap().to_quad())
                    .collect::<Result<_, _>>()?
            };

        let image = convert_opt_index(item.item_data[6], "image")?;
        let mut name = String::new();
        if version >= 2 {
            item.require_length(10)?;
            item.require_max_length(10)?;
            name = parse_i32_string(&item.item_data[7..10])?;
        } else {
            item.require_max_length(7)?;
        }

        Ok(Layer::Quads(QuadsLayer {
            name,
            detail: flags.contains(LayerFlags::DETAIL),
            quads,
            image,
        }))
    }
}

#[repr(C)]
#[derive(View, Copy, Clone)]
struct BinarySoundShape {
    kind: i32_le,
    value1: i32_le,
    value2: i32_le,
}

impl BinarySoundShape {
    fn to_shape(self, position: Vec2<I17F15>) -> Result<SoundArea, LayerError> {
        Ok(match self.kind.to_int() {
            0 => SoundArea::Rectangle(Rect {
                x: position.x,
                y: position.y,
                w: I17F15::from_bits(self.value1.to_int()),
                h: I17F15::from_bits(self.value2.to_int()),
            }),
            1 => SoundArea::Circle(Disk::new(position, I27F5::from_bits(self.value1.to_int()))),
            _ => return Err(LayerError::UnknownShape(self.kind.to_int())),
        })
    }
}

#[repr(C)]
#[derive(View, Copy, Clone)]
pub(crate) struct BinarySoundSource {
    position: BinaryPoint,
    looping: i32_le,
    panning: i32_le,
    delay: i32_le,
    falloff: i32_le,
    position_env: i32_le,
    position_env_offset: i32_le,
    sound_env: i32_le,
    sound_env_offset: i32_le,
    shape: BinarySoundShape,
}

impl BinarySoundSource {
    fn to_source(self) -> Result<SoundSource, MapParseErrorKind> {
        Ok(SoundSource {
            area: self.shape.to_shape(self.position.to_2())?,
            looping: convert_bool(self.looping.to_int(), "looping")?,
            panning: convert_bool(self.panning.to_int(), "panning")?,
            delay: self.delay.to_int(),
            falloff: checked_convert(self.falloff.to_int(), "falloff")?,
            position_env: convert_opt_index(self.position_env.to_int(), "position envelope")?,
            position_env_offset: self.position_env_offset.to_int(),
            sound_env: convert_opt_index(self.sound_env.to_int(), "sound envelope")?,
            sound_env_offset: self.sound_env_offset.to_int(),
        })
    }
}

#[repr(C)]
#[derive(View, Copy, Clone)]
struct BinaryDeprecatedSoundSource {
    position: BinaryPoint,
    looping: i32_le,
    delay: i32_le,
    radius: i32_le,
    position_env: i32_le,
    position_env_offset: i32_le,
    sound_env: i32_le,
    sound_env_offset: i32_le,
}

impl BinaryDeprecatedSoundSource {
    fn to_source(self) -> Result<SoundSource, MapParseErrorKind> {
        Ok(SoundSource {
            area: SoundArea::Circle(Disk::new(
                self.position.to_2(),
                I27F5::from_bits(self.radius.to_int()),
            )),
            looping: convert_bool(self.looping.to_int(), "looping")?,
            panning: true,
            delay: self.delay.to_int(),
            falloff: 0,
            position_env: convert_opt_index(self.position_env.to_int(), "position envelope")?,
            position_env_offset: self.position_env_offset.to_int(),
            sound_env: convert_opt_index(self.sound_env.to_int(), "sound envelope")?,
            sound_env_offset: self.sound_env_offset.to_int(),
        })
    }
}

enum SoundsLayerVersion {
    Normal,
    Deprecated,
}

impl SoundsLayer {
    fn parse(item: &Item, df: &Datafile) -> Result<Layer, MapParseErrorKind> {
        use SoundsLayerVersion::*;

        item.require_length(10)?;
        item.require_max_length(10)?;

        let sounds_layer_version = match item.item_data[1] {
            9 => Deprecated,
            10 => Normal,
            _ => unreachable!(), // unreachable since LayerKind was determined with this number
        };
        let flags = match LayerFlags::from_bits(item.item_data[2]) {
            Some(flags) => flags,
            None => return Err(LayerError::UnknownFlag.into()),
        };

        let source_amount: usize = checked_convert(item.item_data[4], "source amount")?;
        let sound_source_data = df.decompressed_data_item(item.item_data[5])?;
        let source_len = match sounds_layer_version {
            Deprecated => mem::size_of::<BinaryDeprecatedSoundSource>(),
            Normal => mem::size_of::<BinarySoundSource>(),
        };
        if sound_source_data.len() % source_len != 0 {
            return Err(LayerError::SourceDataLen.into());
        }
        let implied_source_amount = sound_source_data.len() / source_len;
        let sources = if implied_source_amount == 1
            && source_amount == 0
            && sound_source_data.iter().all(|b| *b == 0)
        {
            Vec::new()
        } else {
            if implied_source_amount != source_amount {
                return Err(LayerError::SourceAmount.into());
            }
            sound_source_data
                .chunks(source_len)
                .map(|data| match sounds_layer_version {
                    Deprecated => BinaryDeprecatedSoundSource::view(data).unwrap().to_source(),
                    Normal => BinarySoundSource::view(data).unwrap().to_source(),
                })
                .collect::<Result<_, _>>()?
        };

        let sound = convert_opt_index(item.item_data[6], "sound")?;
        let name = parse_i32_string(&item.item_data[7..10])?;
        Ok(Layer::Sounds(SoundsLayer {
            name,
            detail: flags.contains(LayerFlags::DETAIL),
            sources,
            sound,
        }))
    }
}

impl ItemParseErrorTrait for Layer {
    const KIND: ItemType = ItemType::Layer;
    type State = ();

    fn parse_impl(
        item: &Item,
        _: &mut (),
        df: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        use LayerKind::*;
        match item.layer_kind()? {
            Game | Tiles | Front | Tele | Speedup | Switch | Tune => {
                let (tiles_layer, tilemap_kind) = TilesLayer::parse_generic(item, df)?;
                tiles_layer.convert_to(tilemap_kind)
            }
            Quads => QuadsLayer::parse(item, df),
            Sounds => SoundsLayer::parse(item, df),
            Invalid(kind) => Ok(Layer::Invalid(kind)),
        }
    }
}

impl Layer {
    fn distribute(mut layers: Vec<Layer>, groups: &mut Vec<Group>) -> Result<(), MapParseError> {
        for group in groups {
            group.layers = layers.split_off(group.layers.len());
            mem::swap(&mut group.layers, &mut layers);
        }
        if layers.is_empty() {
            Ok(())
        } else {
            Err(MapParseError {
                item_type: ItemType::Layer,
                index: None,
                kind: LayerError::Remaining(layers.len()).into(),
            })
        }
    }
}

#[derive(Error, Debug)]
enum AutomapperError {
    #[error("Unknown bit flags")]
    UnknownFlags,
    #[error("Another auto mapper was already assigned to this layer")]
    DuplicateAssign,
    #[error("Group index {index} with only {len} groups")]
    InvalidGroup { index: usize, len: usize },
    #[error("Layer index {index} with only {len} layers")]
    InvalidLayer { index: usize, len: usize },
}

bitflags! {
    pub(crate) struct AutoMapperFlags: i32 {
        const AUTOMATIC = 0b1;
    }
}

impl ItemParseErrorTrait for (Vec2<usize>, AutomapperConfig) {
    const KIND: ItemType = ItemType::AutoMapperConfig;
    type State = HashSet<Vec2<usize>>;

    fn parse_impl(
        item: &Item,
        state: &mut Self::State,
        df: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        item.require_length(6)?;
        item.require_max_length(6)?;
        let group = checked_convert(item.item_data[1], "group")?;
        let group_amount = df
            .get_items(&HashMap::new(), ItemType::Group)
            .map(Vec::len)
            .unwrap_or(0);
        if group >= group_amount {
            return Err(AutomapperError::InvalidGroup {
                index: group,
                len: group_amount,
            }
            .into());
        }
        let layer = checked_convert(item.item_data[2], "layer")?;
        let layer_amount = df
            .get_items(&HashMap::new(), ItemType::Layer)
            .map(Vec::len)
            .unwrap_or(0);
        if layer >= layer_amount {
            return Err(AutomapperError::InvalidLayer {
                index: layer,
                len: layer_amount,
            }
            .into());
        }
        let position = Vec2::new(group, layer);
        let config = convert_opt_index(item.item_data[3], "config")?;
        let seed = item.item_data[4] as u32;
        let flags = match AutoMapperFlags::from_bits(item.item_data[5]) {
            Some(flags) => flags,
            None => return Err(AutomapperError::UnknownFlags.into()),
        };
        if state.contains(&position) {
            return Err(AutomapperError::DuplicateAssign.into());
        }
        state.insert(position);

        Ok((
            position,
            AutomapperConfig {
                config,
                seed,
                automatic: flags.contains(AutoMapperFlags::AUTOMATIC),
            },
        ))
    }
}

impl AutomapperConfig {
    fn distribute(automappers: Vec<(Vec2<usize>, Self)>, groups: &mut [Group]) {
        for (Vec2 { x, y }, auto_mapper) in automappers {
            if let Layer::Tiles(layer) = &mut groups[x].layers[y] {
                layer.automapper_config = auto_mapper;
            }
        }
    }
}

#[derive(Error, Debug)]
enum SoundError {
    #[error("External sounds are not supported")]
    External,
    #[error("Sound size is ")]
    SoundSize,
}

impl ItemParseErrorTrait for Sound {
    const KIND: ItemType = ItemType::Sound;
    type State = ();

    fn parse_impl(
        item: &Item,
        _: &mut (),
        df: &Datafile,
        _: &ExTypeIndex,
    ) -> Result<Self, MapParseErrorKind> {
        item.require_length(5)?;
        item.require_max_length(5)?;

        let external_bool = convert_bool(item.item_data[1], "external")?;
        if external_bool {
            return Err(SoundError::External.into());
        }
        let name_data = df.decompressed_data_item(item.item_data[2])?;
        let name = parse_c_string(&name_data, Sound::MAX_NAME_LENGTH)?;

        let (data, size) = df.data_item(item.item_data[3])?;
        let data = CompressedData::Compressed(data.to_vec(), size, ());

        let sound_size: usize = checked_convert(item.item_data[4], "sound size")?;
        if sound_size != size {
            return Err(SoundError::SoundSize.into());
        }

        Ok(Sound { name, data })
    }
}
