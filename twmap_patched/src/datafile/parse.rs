/*
 * Documentation used: https://github.com/heinrich5991/libtw2/blob/master/doc/datafile.md
 * Author of the documentation: heinrich5991
*/

/*
 * most functions are given a byte slice and also return one.
 *  -> the parameter byte slice is the remaining, unread part of the file
 *  -> the function will then consume a chunk of bytes from the front of the slice
 *  -> then return the remaining data, alongside the parsed data
*/

use crate::compression::compress;
use crate::convert::{To, TryTo};
use crate::Error;

use log::info;
use structview::{i32_le, u16_le, View};
use thiserror::Error;

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::fmt;
use std::mem;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct DatafileParseError(DatafileParseErr);

#[derive(Error, Debug)]
#[error(transparent)]
enum DatafileParseErr {
    Magic(#[from] MagicError),
    Header(#[from] HeaderError),
    ItemTypes(#[from] ItemTypeError),
    ItemOffsets(OffsetsError),
    DataOffsets(OffsetsError),
    DataSizes(#[from] DataSizesError),
    Items(#[from] ItemError),
    DataItems(#[from] DataItemsError),
    #[error("There is unused data at the end of the file")]
    LeftOverData,
}

impl From<DatafileParseErr> for Error {
    fn from(err: DatafileParseErr) -> Self {
        Error::DatafileParse(DatafileParseError(err))
    }
}

impl<T: Into<DatafileParseErr>> From<T> for DatafileParseError {
    fn from(err: T) -> Self {
        Self(err.into())
    }
}

#[derive(Error, Debug)]
struct LengthError;

impl fmt::Display for LengthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not enough bytes left")
    }
}

fn view<T: View>(data: &[u8]) -> Result<(&T, &[u8]), LengthError> {
    if data.len() < mem::size_of::<T>() {
        Err(LengthError)
    } else {
        let (struct_data, remaining_data) = data.split_at(mem::size_of::<T>());
        Ok((T::view(struct_data).unwrap(), remaining_data))
    }
}

fn view_multiple<T: View>(data: &[u8], amount: usize) -> Result<(&[T], &[u8]), LengthError> {
    let size = mem::size_of::<T>() * amount;
    if data.len() < size {
        Err(LengthError)
    } else {
        let (struct_data, remaining_data) = data.split_at(size);
        Ok((T::view_slice(struct_data).unwrap(), remaining_data))
    }
}

#[derive(Error, Debug)]
enum MagicError {
    #[error("{0}")]
    Length(#[from] LengthError),
    #[error("This is not a teeworlds map")]
    Incorrect,
}

fn parse_magic(data: &[u8]) -> Result<&[u8], MagicError> {
    let (magic, data) = view::<[u8; 4]>(data)?;
    if magic != b"DATA" && magic != b"ATAD" {
        Err(MagicError::Incorrect)
    } else {
        Ok(data)
    }
}

#[derive(Debug, View, Copy, Clone)]
#[repr(C)]
pub struct Header {
    pub version: i32_le,
    pub size: i32_le,
    pub swap_len: i32_le,
    pub num_item_types: i32_le,
    pub num_items: i32_le,
    pub num_data: i32_le,
    pub item_block_size: i32_le,
    pub data_block_size: i32_le,
}

#[derive(Error, Debug)]
enum HeaderError {
    #[error("{0}")]
    Length(#[from] LengthError),
    #[error("Unsupported version, supported are versions 3 and 4")]
    Version,
    #[error("The size is not accurate")]
    Size,
    #[error("The swap len is not accurate")]
    SwapLen,
    #[error("Amount of item types is negative")]
    NumItemTypes,
    #[error("Amount of items is negative")]
    NumItems,
    #[error("Amount of data items is negative")]
    NumData,
    #[error("Item block size is negative")]
    ItemBlockSize,
    #[error("Data block size is negative")]
    DataBlockSize,
}

impl Header {
    fn parse(data: &[u8]) -> Result<(&Header, &[u8]), HeaderError> {
        use HeaderError::*;
        let (header, data) = view::<Header>(data)?;

        if ![3, 4].contains(&header.version.to_int()) {
            return Err(Version);
        }
        if header.num_item_types.to_int() < 0 {
            return Err(NumItemTypes);
        }
        if header.num_items.to_int() < 0 {
            return Err(NumItems);
        }
        if header.num_data.to_int() < 0 {
            return Err(NumData);
        }
        if header.item_block_size.to_int() < 0 {
            return Err(ItemBlockSize);
        }
        if header.data_block_size.to_int() < 0 {
            return Err(DataBlockSize);
        }
        let expected_size = data.len().try_to::<i64>() + 20;
        let offset = header.num_data.to_int().try_to::<i64>() * 4;
        if header.size.to_int().to::<i64>() != expected_size {
            if header.size.to_int().to::<i64>() == expected_size - offset {
                info!("Faulty size calculation in the datafile header");
            } else {
                return Err(Size);
            }
        }
        let expected_swap_len = expected_size - header.data_block_size.to_int().to::<i64>();
        if header.swap_len.to_int().to::<i64>() != expected_swap_len {
            if header.swap_len.to_int().to::<i64>() == expected_swap_len - offset {
                info!("Faulty swap_len calculation in the datafile header");
            } else {
                return Err(SwapLen);
            }
        }
        Ok((header, data))
    }
}

#[derive(Debug, View, Copy, Clone)]
#[repr(C)]
pub struct ItemType {
    pub type_id: i32_le,
    pub start: i32_le,
    pub num: i32_le,
}

#[derive(Error, Debug)]
enum ItemTypeError {
    #[error("{0}")]
    Length(#[from] LengthError),
    #[error("A type id is too large (must fit into an u16")]
    InvalidTypeId,
    #[error("The same type id is used twice")]
    DuplicateTypeId,
    #[error("Negative item amount")]
    NegativeItemAmount,
    #[error("Item amount is zero")]
    ZeroItems,
    #[error("The item ranges overlap")]
    Overlap,
    #[error("The item ranges leave a gap")]
    Gap,
    #[error("The item ranges together use more items than there are")]
    TooFewItems,
    #[error("The item ranges together use less items than there are")]
    TooManyItems,
}

impl ItemType {
    fn parse(
        data: &[u8],
        num_item_types: i32_le,
        num_items: i32_le,
    ) -> Result<(&[ItemType], &[u8]), ItemTypeError> {
        let (item_types, data) = view_multiple::<ItemType>(data, num_item_types.to_int().try_to())?;

        let mut used_type_ids = HashSet::new();
        let mut expected_start = 0_i64;
        for item_type in item_types {
            if u16::try_from(item_type.type_id.to_int()).is_err() {
                return Err(ItemTypeError::InvalidTypeId);
            }
            let new = used_type_ids.insert(item_type.type_id.to_int());
            if !new {
                return Err(ItemTypeError::DuplicateTypeId);
            }
            match item_type.num.to_int() {
                i32::MIN..=-1 => return Err(ItemTypeError::NegativeItemAmount),
                0 => return Err(ItemTypeError::ZeroItems),
                1..=i32::MAX => {}
            }
            match item_type.start.to_int().to::<i64>().cmp(&expected_start) {
                Ordering::Less => return Err(ItemTypeError::Overlap),
                Ordering::Equal => {}
                Ordering::Greater => return Err(ItemTypeError::Gap),
            }
            expected_start += item_type.num.to_int().to::<i64>();
        }
        match expected_start.cmp(&num_items.to_int().to::<i64>()) {
            Ordering::Less => return Err(ItemTypeError::TooFewItems),
            Ordering::Equal => {}
            Ordering::Greater => return Err(ItemTypeError::TooManyItems),
        }
        Ok((item_types, data))
    }
}

#[derive(Error, Debug)]
enum OffsetsError {
    #[error("{0}")]
    Length(#[from] LengthError),
    #[error("The first offset value isn't 0")]
    FirstNonZero,
    #[error("Negative value")]
    Negative,
    #[error("A value is lower than the last one")]
    TooLow,
}

fn parse_offsets(data: &[u8], amount: i32_le) -> Result<(&[i32_le], &[u8]), OffsetsError> {
    let (offsets, data) = view_multiple::<i32_le>(data, amount.to_int().try_to())?;

    if let Some(offset) = offsets.first() {
        if offset.to_int() != 0 {
            return Err(OffsetsError::FirstNonZero);
        }
    }
    let mut min_value = 0_i64;
    for offset in offsets {
        let offset = offset.to_int().to::<i64>();
        if offset < 0 {
            return Err(OffsetsError::Negative);
        }
        if offset < min_value {
            return Err(OffsetsError::TooLow);
        }
        min_value = offset;
    }
    Ok((offsets, data))
}

#[derive(Error, Debug)]
enum DataSizesError {
    #[error("{0}")]
    Length(#[from] LengthError),
    #[error("Negative value")]
    Negative,
}

fn parse_data_sizes(data: &[u8], amount: i32_le) -> Result<(&[i32_le], &[u8]), DataSizesError> {
    let (data_sizes, data) = view_multiple::<i32_le>(data, amount.to_int().try_to())?;

    if data_sizes.iter().any(|size| size.to_int() < 0) {
        return Err(DataSizesError::Negative);
    }
    Ok((data_sizes, data))
}

#[derive(Debug, View, Copy, Clone)]
#[repr(C)]
pub struct ItemHeader {
    pub id: u16_le,
    pub type_id: u16_le,
    pub size: i32_le,
}

#[derive(Debug)]
pub struct ViewedItem<'a> {
    pub item_header: &'a ItemHeader,
    pub item_data: &'a [i32_le],
}

#[derive(Error, Debug)]
enum ItemError {
    #[error("{0}")]
    Length(#[from] LengthError),
    #[error("Negative item data size")]
    NegativeSize,
    #[error("Item data size is not divisible by 4")]
    InvalidSize,
    #[error("Wrong type id")]
    WrongTypeId,
}

impl ViewedItem<'_> {
    fn parse<'a>(
        mut data: &'a [u8],
        item_types: &[ItemType],
    ) -> Result<(Vec<ViewedItem<'a>>, &'a [u8]), ItemError> {
        let mut items = Vec::new();
        for item_type in item_types {
            for _ in 0..item_type.num.to_int() {
                let (item_header, tmp_data) = view::<ItemHeader>(data)?;
                if item_header.size.to_int() < 0 {
                    return Err(ItemError::NegativeSize);
                }
                if item_header.size.to_int() % 4 != 0 {
                    return Err(ItemError::InvalidSize);
                }
                if item_header.type_id.to_int() != item_type.type_id.to_int().try_to::<u16>() {
                    return Err(ItemError::WrongTypeId);
                }
                let (item_data, tmp_data) = view_multiple::<i32_le>(
                    tmp_data,
                    item_header.size.to_int().try_to::<usize>() / 4,
                )?;
                data = tmp_data;
                items.push(ViewedItem {
                    item_header,
                    item_data,
                })
            }
        }
        Ok((items, data))
    }
}

#[derive(Error, Debug)]
enum DataItemsError {
    #[error("{0}")]
    Length(#[from] LengthError),
    #[error("The last data item supposedly has a negative size")]
    LastDataItemNegativeSize,
}

fn parse_data_items<'a>(
    mut data: &'a [u8],
    data_offsets: &[i32_le],
    total_data_size: i32_le,
) -> Result<(Vec<&'a [u8]>, &'a [u8]), DataItemsError> {
    let mut data_items = Vec::new();
    for size in data_offsets.windows(2).map(|offsets| {
        offsets[1].to_int().try_to::<usize>() - offsets[0].to_int().try_to::<usize>()
    }) {
        let (data_item, new_data) = view_multiple::<u8>(data, size)?;
        data = new_data;
        data_items.push(data_item);
    }
    if let Some(offset) = data_offsets.last() {
        if offset.to_int() > total_data_size.to_int() {
            return Err(DataItemsError::LastDataItemNegativeSize);
        }
        let size = total_data_size.to_int().try_to::<usize>() - offset.to_int().try_to::<usize>();
        let (last_data_item, new_data) = view_multiple::<u8>(data, size)?;
        data = new_data;
        data_items.push(last_data_item);
    }
    Ok((data_items, data))
}

pub struct RawDatafile<'a> {
    pub header: &'a Header,
    pub item_types: &'a [ItemType],
    pub item_offsets: &'a [i32_le],
    pub data_offsets: &'a [i32_le],
    pub data_sizes: Option<&'a [i32_le]>,
    pub items: Vec<ViewedItem<'a>>,
    pub data_items: Vec<&'a [u8]>,
}

impl RawDatafile<'_> {
    pub fn parse(data: &[u8]) -> Result<RawDatafile, DatafileParseError> {
        let data = parse_magic(data)?;
        let (header, data) = Header::parse(data)?;
        let (item_types, data) = ItemType::parse(data, header.num_item_types, header.num_items)?;
        let (item_offsets, data) =
            parse_offsets(data, header.num_items).map_err(DatafileParseErr::ItemOffsets)?;
        let (data_offsets, mut data) =
            parse_offsets(data, header.num_data).map_err(DatafileParseErr::DataOffsets)?;
        let mut data_sizes = None;
        if header.version.to_int() >= 4 {
            let (new_data_sizes, new_data) = parse_data_sizes(data, header.num_data)?;
            data = new_data;
            data_sizes = Some(new_data_sizes);
        }
        let (items, data) = ViewedItem::parse(data, item_types)?;
        let (data_items, data) = parse_data_items(data, data_offsets, header.data_block_size)?;
        if !data.is_empty() {
            return Err(DatafileParseErr::LeftOverData.into());
        }
        Ok(RawDatafile {
            header,
            item_types,
            item_offsets,
            data_offsets,
            data_sizes,
            items,
            data_items,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Item {
    pub id: u16,
    pub item_data: Vec<i32>,
}

impl From<&ViewedItem<'_>> for Item {
    fn from(viewed_item: &ViewedItem) -> Self {
        Item {
            id: viewed_item.item_header.id.to_int(),
            item_data: viewed_item
                .item_data
                .iter()
                .map(|i32_le| i32_le.to_int())
                .collect(),
        }
    }
}

pub struct Datafile<'a> {
    pub items: HashMap<u16, Vec<Item>>,
    pub data_items: Vec<(Cow<'a, [u8]>, usize)>,
}

impl<'a> RawDatafile<'a> {
    pub fn to_datafile(&self) -> Datafile<'a> {
        let mut items = HashMap::new();
        for item_type in self.item_types {
            let start: usize = item_type.start.to_int().try_to();
            let end = start + item_type.num.to_int().try_to::<usize>();
            let item_type_items = self.items[start..end].iter().map(Item::from).collect();
            items.insert(item_type.type_id.to_int().try_to::<u16>(), item_type_items);
        }
        let data_items = match self.data_sizes {
            None => self
                .data_items
                .iter()
                .map(|&data_item| (Cow::from(compress(data_item)), data_item.len()))
                .collect(),
            Some(data_sizes) => self
                .data_items
                .iter()
                .zip(data_sizes.iter())
                .map(|(&data_item, data_size)| {
                    (Cow::from(data_item), data_size.to_int().try_to::<usize>())
                })
                .collect(),
        };
        Datafile { items, data_items }
    }
}
