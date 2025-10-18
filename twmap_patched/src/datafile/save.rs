use crate::compression::compress;
use crate::convert::{To, TryTo};
use crate::datafile::parse::Item;

use thiserror::Error;

use crate::Error;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::io::{Error as IoError, Write};

#[derive(Error, Debug)]
#[error(transparent)]
pub struct DatafileSaveError(SizeError);

#[derive(Error, Debug)]
struct SizeError {
    size: u64,
}

impl fmt::Display for SizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Maps may at most be ~2 GB ({} bytes) in size, this one is {} bytes in size",
            i32::MAX,
            self.size,
        )
    }
}

impl Item {
    // count of bytes used
    fn len(&self) -> u64 {
        (self.item_data.len().try_to::<u64>() + 2) * 4
    }

    fn write(&self, file: &mut dyn Write, type_id: u16) -> Result<(), Error> {
        file.write_all(&u16::to_le_bytes(self.id))?;
        file.write_all(&u16::to_le_bytes(type_id))?;
        let size = self.item_data.len().try_to::<i32>() * 4;
        file.write_all(&i32::to_le_bytes(size))?;
        for &n in &self.item_data {
            file.write_all(&i32::to_le_bytes(n))?;
        }
        Ok(())
    }
}

pub fn save(
    output: &mut dyn Write,
    items: &HashMap<u16, Vec<Item>>,
    uncompressed_data_items: &[Cow<[u8]>],
) -> Result<(), Error> {
    check_items(items);
    version_header(output)?;
    let data_items: Vec<Vec<u8>> = uncompressed_data_items
        .iter()
        .map(|data| compress(data))
        .collect();
    header(output, items, &data_items)?;
    item_types(output, items)?;
    item_offsets(output, items)?;
    data_offsets(output, &data_items)?;
    data_sizes(output, uncompressed_data_items)?;
    write_items(output, items)?;
    write_data_items(output, &data_items)?;
    output.flush()?;
    Ok(())
}

fn check_items(items: &HashMap<u16, Vec<Item>>) {
    // checks if every item type has at least one item
    for (type_id, items_vec) in items {
        if items_vec.is_empty() {
            panic!("Empty ItemTypes are not valid. Empty type id: {}", type_id);
        }
    }
}

fn version_header(file: &mut dyn Write) -> Result<(), IoError> {
    file.write_all(b"DATA")?;
    file.write_all(&i32::to_le_bytes(4))?;
    //file.write_all(b"ATAD")?;
    Ok(())
}

// calculates the num_items field for the header
fn num_items(items: &HashMap<u16, Vec<Item>>) -> u64 {
    let mut sum = 0;
    for items_vec in items.values() {
        sum += items_vec.len().try_to::<u64>();
    }
    sum
}

// calculates the amount of i32 of all items
fn item_size(items: &HashMap<u16, Vec<Item>>) -> u64 {
    let mut sum = 0;
    for items_vec in items.values() {
        for item in items_vec {
            sum += item.len();
        }
    }
    sum
}

fn data_size(data_items: &[Vec<u8>]) -> u64 {
    let mut sum = 0;
    for data_item in data_items {
        sum += data_item.len().try_to::<u64>();
    }
    sum
}

fn header(
    file: &mut dyn Write,
    items: &HashMap<u16, Vec<Item>>,
    data_items: &[Vec<u8>],
) -> Result<(), Error> {
    let num_item_types = items.len().try_to::<u64>();
    let num_items = num_items(items);
    let num_data = data_items.len().try_to::<u64>();
    let item_size = item_size(items);
    let data_size = data_size(data_items);

    // swap_len: count of all bytes representing ints in the entire datafile (so up until the data items), after the swaplen field
    let swap_len = (5 + num_item_types * 3 + num_items + num_data * 2) * 4 + item_size;

    // size: count of bytes in the entire datafile, after the swaplen field
    let size = swap_len + data_size;

    if size > i32::MAX.try_to::<u64>() {
        return Err(DatafileSaveError(SizeError { size }).into());
    }

    for &n in [
        size,
        swap_len,
        num_item_types,
        num_items,
        num_data,
        item_size,
        data_size,
    ]
    .iter()
    {
        file.write_all(&i32::to_le_bytes(n.try_to()))?;
    }
    Ok(())
}

fn item_types(file: &mut dyn Write, items: &HashMap<u16, Vec<Item>>) -> Result<(), Error> {
    let mut start = 0;
    let mut keys: Vec<&u16> = items.keys().collect();
    keys.sort();
    for key in keys {
        let type_id = (*key).to::<i32>();
        let num = items[key].len().try_to::<i32>(); // would've already errored in header (items_int_count)
        for &n in [type_id, start, num].iter() {
            file.write_all(&i32::to_le_bytes(n))?;
        }
        start += num;
    }
    Ok(())
}

fn item_offsets(file: &mut dyn Write, items: &HashMap<u16, Vec<Item>>) -> Result<(), Error> {
    let mut keys: Vec<&u16> = items.keys().collect();
    keys.sort();
    let mut start: i32 = 0;
    for key in keys {
        for item in items.get(key).unwrap() {
            let size = item.len().try_to::<i32>();
            file.write_all(&i32::to_le_bytes(start))?;
            start += size;
        }
    }
    Ok(())
}

fn data_offsets(file: &mut dyn Write, compressed_data_items: &[Vec<u8>]) -> Result<(), Error> {
    let mut start: i32 = 0;
    for compressed_data in compressed_data_items {
        file.write_all(&i32::to_le_bytes(start))?;
        start += compressed_data.len().try_to::<i32>();
    }
    Ok(())
}

fn data_sizes(file: &mut dyn Write, data_items: &[Cow<[u8]>]) -> Result<(), Error> {
    for data_item in data_items {
        let size = data_item.len().try_to::<i32>();
        file.write_all(&i32::to_le_bytes(size))?;
    }
    Ok(())
}

fn write_items(file: &mut dyn Write, items: &HashMap<u16, Vec<Item>>) -> Result<(), Error> {
    let mut keys: Vec<&u16> = items.keys().collect();
    keys.sort();
    for key in keys {
        for item in items.get(key).unwrap() {
            item.write(file, *key)?;
        }
    }
    Ok(())
}

fn write_data_items(file: &mut dyn Write, data_items: &[Vec<u8>]) -> Result<(), Error> {
    for data_item in data_items {
        file.write_all(data_item)?;
    }
    Ok(())
}
