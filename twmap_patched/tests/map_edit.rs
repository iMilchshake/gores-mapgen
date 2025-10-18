use twmap::*;

const MAPS: [&str; 4] = ["dm1", "editor", "tileflags_ddnet", "tileflags_teeworlds"];

fn map_from_path(path: &str) -> TwMap {
    let data = std::fs::read(path).unwrap();
    TwMap::parse(&data).unwrap()
}

#[test]
fn mirror() {
    for map_name in MAPS {
        let path = format!("tests/{}.map", map_name);
        let mut map = map_from_path(&path);
        map.load().unwrap();
        map.isolate_physics_layers();
        map = map.lossless_shrink_layers().unwrap();
        let original_map = map.clone();
        map = map.mirror().unwrap().mirror().unwrap();
        map = map.lossless_shrink_layers().unwrap();
        assert_eq!(map, original_map, "Map: {}", map_name);
    }
}

#[test]
fn rotate() {
    for map_name in MAPS {
        let path = format!("tests/{}.map", map_name);
        let mut map = map_from_path(&path);
        map.load().unwrap();
        map.isolate_physics_layers();
        map = map.lossless_shrink_layers().unwrap();
        let original_map = map.clone();
        for _ in 0..4 {
            map = map.rotate_right().unwrap();
        }
        map = map.lossless_shrink_layers().unwrap();
        assert_eq!(map, original_map, "Map: {}", map_name);
    }
}

#[test]
fn mirror_rotate() {
    for map_name in MAPS {
        let path = format!("tests/{}.map", map_name);
        let mut map = map_from_path(&path);
        map.load().unwrap();
        map.isolate_physics_layers();
        map = map.lossless_shrink_layers().unwrap();
        let original_map = map.clone();
        for _ in 0..4 {
            map = map.mirror().unwrap();
            map = map.rotate_right().unwrap();
        }
        map = map.lossless_shrink_layers().unwrap();
        assert_eq!(map, original_map, "Map: {}", map_name);
    }
}
