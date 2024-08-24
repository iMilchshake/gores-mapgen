use twmap::*;

fn main() {
    let mut map = TwMap::parse(&std::fs::read("src/test.map").expect("map file couldn't be read"))
        .expect("parsing failed");
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
    let mut file = std::fs::File::create("src/test_out.map").expect("Could not create file");
    map.save(&mut file).expect("saving failed");
}
