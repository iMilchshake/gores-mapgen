use crate::map::{BlockType, Map};
use ndarray::Array2;
use rust_embed::RustEmbed;
use std::path::PathBuf;
use twmap::{
    automapper::{self, Automapper},
    GameLayer, GameTile, Layer, Tile, TileFlags, TilemapLayer, TilesLayer, TwMap,
};

#[derive(RustEmbed)]
#[folder = "automapper/"]
pub struct AutoMapperConfigs;

impl AutoMapperConfigs {
    pub fn get_config(name: String) -> Automapper {
        let file = AutoMapperConfigs::get(&(name.clone() + ".rules"))
            .expect("automapper rule config not found");
        let data = std::str::from_utf8(&file.data).unwrap();

        Automapper::parse(name, data).expect("failed to parse .rules file")
    }
}

pub struct TwExport;

impl TwExport {
    pub fn get_automapper_config(rule_name: String, layer: &TilesLayer) -> automapper::Config {
        let config_index = layer.automapper_config.config.unwrap();
        let automapper = AutoMapperConfigs::get_config(rule_name);
        let automapper_config = automapper
            .configs
            .get(config_index as usize)
            .expect("coulnt fetch automapper config via index");

        automapper_config.clone()
    }

    pub fn process_layer<F>(
        tw_map: &mut TwMap,
        map: &Map,
        layer_index: &usize,
        layer_name: &str,
        block_type_in_layer: F,
    ) where
        F: Fn(&BlockType) -> bool,
    {
        let tile_group = tw_map.groups.get_mut(2).unwrap();
        assert_eq!(tile_group.name, "Tiles");

        if let Some(Layer::Tiles(layer)) = tile_group.layers.get_mut(*layer_index) {
            assert_eq!(layer.name, layer_name);

            let image_name = tw_map.images[layer.image.unwrap() as usize].name();
            let automapper_config = TwExport::get_automapper_config(image_name.clone(), layer);

            let tiles = layer.tiles_mut().unwrap_mut();
            *tiles = Array2::<Tile>::default((map.width, map.height));

            for ((x, y), value) in map.grid.indexed_iter() {
                if block_type_in_layer(value) {
                    tiles[[y, x]] = Tile::new(1, TileFlags::empty())
                }
            }

            automapper_config.run(3777777777, tiles) // thanks Tater for the epic **random** seed
        } else {
            panic!("coulnt get layer at index");
        };
    }

    pub fn export(map: &Map, path: &PathBuf) {
        let mut tw_map = TwMap::parse_file("automap_test.map").expect("parsing failed");
        tw_map.load().expect("loading failed");

        TwExport::process_layer(&mut tw_map, map, &0, "Freeze", |t| {
            (*t == BlockType::Freeze) || BlockType::is_hookable(t)
        });
        TwExport::process_layer(&mut tw_map, map, &1, "Hookable", BlockType::is_hookable);

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
        tw_map.save_file(path).expect("failed to write map file");
    }
}
