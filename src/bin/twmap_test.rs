use twmap::*;

fn main() {
    let mut map = TwMap::parse_file("src/test.map").expect("parsing failed");
    map.load().expect("loading failed");

    // get game layer
    let game_layer = map
        .find_physics_layer_mut::<GameLayer>()
        .unwrap()
        .tiles_mut()
        .unwrap_mut();

    // modify game layer
    game_layer[[0, 0]] = GameTile::new(1, TileFlags::empty());

    // save map
    map.save_file("src/test_out.map").expect("saving failed");
}
