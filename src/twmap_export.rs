use crate::map::Map;
use ndarray::Array2;
use std::{fs, path::PathBuf};
use twmap::{
    automapper::Automapper, GameLayer, GameTile, Layer, Tile, TileFlags, TilemapLayer, TwMap,
};

pub struct TwExport;

impl TwExport {
    pub fn export(map: &Map, path: &PathBuf) {
        let mut tw_map = TwMap::parse_file("automap_test.map").expect("parsing failed");
        tw_map.load().expect("loading failed");

        // get Tiles group
        let tile_group = tw_map.groups.get_mut(2).unwrap();
        assert_eq!(tile_group.name, "Tiles");

        // get Hookable and Freeze layer in Tiles group
        if let Some(Layer::Tiles(layer)) = tile_group.layers.get_mut(1) {
            assert_eq!(layer.name, "Hookable");

            let image_name = tw_map.images[layer.image.unwrap() as usize].name();
            let config_index = layer.automapper_config.config.unwrap();

            let tiles = layer.tiles_mut().unwrap_mut();
            *tiles = Array2::<Tile>::default((map.width, map.height));

            for ((x, y), value) in map.grid.indexed_iter() {
                if value.in_tw_hookable_layer() {
                    tiles[[y, x]] = Tile::new(1, TileFlags::empty())
                }
            }

            let file = fs::read_to_string("automapper/ddnet_walls.rules")
                .expect("failed to read .rules file");
            let automapper = Automapper::parse(image_name.to_string(), &file)
                .expect("failed to parse .rules file");
            let config = automapper
                .configs
                .get(config_index as usize)
                .expect("failed to fetch config");

            config.run(1337, tiles);
        } else {
            panic!("coulnt get Hookable layer at index 0");
        };

        // get game layer
        let game_layer = tw_map
            .find_physics_layer_mut::<GameLayer>()
            .unwrap()
            .tiles_mut()
            .unwrap_mut();

        *game_layer = Array2::<GameTile>::from_elem(
            (map.width, map.height),
            GameTile::new(0, TileFlags::empty()),
        );

        // modify game layer
        for ((x, y), value) in map.grid.indexed_iter() {
            game_layer[[y, x]] = GameTile::new(value.to_tw_game_id(), TileFlags::empty())
        }

        // save map
        println!("exporting map to {:?}", &path);
    }
}
