use twmap::compression::decompress;
use twmap::datafile::{save, Datafile, Item, RawDatafile};

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;

// extracts all data items from a datafile
fn get_data_items(df: &Datafile) -> Vec<Vec<u8>> {
    let mut data_items = Vec::new();
    for (data, size) in df.data_items.iter() {
        data_items.push(decompress(data, *size).unwrap());
    }
    data_items
}

// checks if saving and loading right after changes any data
fn save_load_eq(items: &HashMap<u16, Vec<Item>>, data_items: &[Vec<u8>]) {
    let mut bytes = Vec::new();
    save(
        &mut bytes,
        items,
        &data_items.iter().map(Cow::from).collect::<Vec<_>>(),
    )
    .unwrap();
    let df = RawDatafile::parse(&bytes).unwrap().to_datafile();
    let loaded_items = df.items.clone();
    let loaded_data_items = get_data_items(&df);
    assert_eq!(loaded_items, *items);
    assert_eq!(loaded_data_items, *data_items);
}

#[test]
fn minimal() {
    let items_1 = vec![
        Item {
            id: 0,
            item_data: vec![1, 2, 3],
        },
        Item {
            id: 1,
            item_data: vec![4, 5, 6],
        },
    ];

    let items_2 = vec![
        Item {
            id: 0xffff,
            item_data: vec![0xa, 0xb],
        },
        Item {
            id: 0xeeee,
            item_data: vec![0xc, 0xd],
        },
        Item {
            id: 0xdddd,
            item_data: vec![0xe, 0xf],
        },
    ];
    let mut items = HashMap::new();
    items.insert(5, items_1);
    items.insert(0xbeef, items_2);
    let data_items = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
    save_load_eq(&items, &data_items);
}

#[test]
fn dm1_test() {
    let data = fs::read("tests/dm1.map").unwrap();
    let df = RawDatafile::parse(&data).unwrap().to_datafile();
    let items = df.items.clone();
    let data_items = get_data_items(&df);
    save_load_eq(&items, &data_items);
}

#[test]
fn editor_test() {
    let data = fs::read("tests/editor.map").unwrap();
    let df = RawDatafile::parse(&data).unwrap().to_datafile();
    let items = df.items.clone();
    let data_items = get_data_items(&df);
    save_load_eq(&items, &data_items);
}
