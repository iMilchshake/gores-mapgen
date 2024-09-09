use crate::map::{BlockTypeTW, Map};
use crate::position::Position;
use ndarray::Array2;
use rust_embed::RustEmbed;
use std::path::PathBuf;
use twmap::{
    automapper::{self, Automapper},
    GameLayer, GameTile, Layer, Tile, TileFlags, TilemapLayer, TilesLayer, TwMap,
};

#[derive(RustEmbed)]
#[folder = "data/automapper/"]
pub struct AutoMapperConfigs;

#[derive(RustEmbed)]
#[folder = "data/basemaps/"]
pub struct BaseMaps;

impl AutoMapperConfigs {
    pub fn get_config(name: String) -> Automapper {
        let file = AutoMapperConfigs::get(&(name.clone() + ".rules"))
            .expect("automapper rule config not found");
        let data = std::str::from_utf8(&file.data).unwrap();

        Automapper::parse(name, data).expect("failed to parse .rules file")
    }
}

impl BaseMaps {
    // TODO: add support for custom name or random
    pub fn get_base_map() -> TwMap {
        let file = BaseMaps::get("grass.map").expect("base map not found");
        let mut tw_map = TwMap::parse(&file.data).expect("parsing failed");
        tw_map.load().expect("map loading failed");

        tw_map
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

    pub fn process_tile_layer(
        tw_map: &mut TwMap,
        map: &Map,
        layer_index: &usize,
        layer_name: &str,
        layer_type: &BlockTypeTW,
    ) {
        let tile_group = tw_map.groups.get_mut(2).unwrap();
        assert_eq!(tile_group.name, "Tiles");

        if let Some(Layer::Tiles(layer)) = tile_group.layers.get_mut(*layer_index) {
            assert_eq!(layer.name, layer_name);

            let image_name = tw_map.images[layer.image.unwrap() as usize].name();
            let automapper_config = TwExport::get_automapper_config(image_name.clone(), layer);

            let tiles = layer.tiles_mut().unwrap_mut();
            *tiles = Array2::<Tile>::default((map.height, map.width));

            for ((x, y), block_type) in map.grid.indexed_iter() {
                let block_type = block_type.to_tw_block_type();
                let mut set_block: bool = *layer_type == block_type;

                // custom rule for freeze
                if layer_type == &BlockTypeTW::Freeze && block_type == BlockTypeTW::Hookable {
                    let shifts = &[(-1, 0), (0, -1), (1, 0), (0, 1)];
                    for shift in shifts {
                        let neighbor_type = Position::new(x, y)
                            .shifted_by(shift.0, shift.1)
                            .ok()
                            .and_then(|pos| map.grid.get(pos.as_index()));

                        if neighbor_type.is_some_and(|t| t.is_freeze()) {
                            set_block = true;
                            break;
                        }
                    }
                }

                if set_block {
                    tiles[[y, x]] = Tile::new(1, TileFlags::empty())
                }
            }

            automapper_config.run(3777777777, tiles) // thanks Tater for the epic **random** seed
        } else {
            panic!(
                "coulnt get layer at index {:} ({:})",
                layer_index, layer_name
            );
        };
    }

    pub fn export(map: &Map, path: &PathBuf) {
        let mut tw_map = BaseMaps::get_base_map();

        TwExport::process_tile_layer(&mut tw_map, map, &0, "Freeze", &BlockTypeTW::Freeze);
        TwExport::process_tile_layer(&mut tw_map, map, &1, "Hookable", &BlockTypeTW::Hookable);

        // TODO: move into function
        // get game layer
        let game_layer = tw_map
            .find_physics_layer_mut::<GameLayer>()
            .unwrap()
            .tiles_mut()
            .unwrap_mut();

        *game_layer = Array2::<GameTile>::from_elem(
            (map.height, map.width),
            GameTile::new(0, TileFlags::empty()),
        );

        // modify game layer
        for ((x, y), value) in map.grid.indexed_iter() {
            game_layer[[y, x]] = GameTile::new(value.to_tw_game_id(), TileFlags::empty())
        }

        // save map
        println!("exporting map to {:?}", &path);

        let mut file = std::fs::File::create(path).unwrap();
        tw_map.save(&mut file).expect("failed to write map file");
    }
}
