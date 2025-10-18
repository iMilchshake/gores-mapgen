use twmap::*;

fn map_from_path(path: &str) -> TwMap {
    let data = std::fs::read(path).unwrap();
    TwMap::parse(&data).unwrap()
}

fn save_load_eq(mut map: TwMap) {
    let mut bin_data = Vec::new();
    map.save(&mut bin_data).unwrap();
    let mut bin_loaded_map = TwMap::parse(&bin_data).unwrap();
    bin_loaded_map.load().unwrap();

    let tmp_dir = tempfile::tempdir().unwrap();
    let path = tmp_dir.path().join("map");
    map.save_dir(&path).unwrap();
    let dir_loaded_map = TwMap::parse_dir(&path).unwrap();

    assert_eq!(dir_loaded_map, map);
}

#[test]
fn dm1_dir() {
    let dm1 = map_from_path("tests/dm1.map");
    save_load_eq(dm1);
}

#[test]
fn editor_dir() {
    let editor_map = map_from_path("tests/editor.map");
    save_load_eq(editor_map);
}

#[test]
fn tileflags_ddnet_dir() {
    let editor_map = map_from_path("tests/tileflags_ddnet.map");
    save_load_eq(editor_map);
}

#[test]
fn tileflags_teeworlds_dir() {
    let editor_map = map_from_path("tests/tileflags_teeworlds.map");
    save_load_eq(editor_map);
}
