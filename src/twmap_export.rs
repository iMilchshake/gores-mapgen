use crate::map::{BlockTypeTW, Map};
use crate::position::Position;
use clap::crate_version;
use ndarray::Array2;
use rust_embed::RustEmbed;
use std::char;
use std::path::PathBuf;
use twmap::{
    automapper::{self, Automapper},
    GameLayer, GameTile, Layer, Tile, TileFlags, TilemapLayer, TilesLayer, TwMap,
};

const AUTOMAPPER_SEED: u32 = 3777777777; // thanks Tater for the epic **random** seed

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

/// place a tile with id=1 for each true value
pub fn set_bool_active(_: usize, _: usize, active: &bool) -> u8 {
    *active as u8
}

/// place a tile with matching id for each char
pub fn set_char_id(_: usize, _: usize, char: &char) -> u8 {
    match *char {
        ' ' => 0,
        '.' => 52,
        ':' => 64,
        '>' => 114,
        '!' => 48,

        // a-Z or A-Z
        ch if ch.is_ascii_alphabetic() => ch.to_ascii_lowercase() as u8 - b'a' + 1,

        // digits
        ch if ch.is_ascii_digit() => {
            if ch == '0' {
                63
            } else {
                ch.to_digit(10).unwrap() as u8 + 53
            }
        }

        _ => panic!("unsupported character: {:}", char),
    }
}

pub struct TwExport {}

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
        layer_index: usize,
        layer_name: &str,
        layer_type: &BlockTypeTW,
    ) {
        let tile_group = tw_map.groups.get_mut(3).unwrap();
        assert_eq!(tile_group.name, "FG_Tiles");
        if let Some(Layer::Tiles(layer)) = tile_group.layers.get_mut(layer_index) {
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

    pub fn process_tile_layer_new<F, T>(
        tw_map: &mut TwMap,
        group: (usize, &str),
        layer: (usize, &str),
        grid: &Array2<T>,
        set_tile_id: F,
        use_automap: bool,
    ) where
        F: Fn(usize, usize, &T) -> u8,
    {
        let tile_group = tw_map.groups.get_mut(group.0).unwrap();
        assert_eq!(tile_group.name, group.1);
        if let Some(Layer::Tiles(tiles_layer)) = tile_group.layers.get_mut(layer.0) {
            assert_eq!(tiles_layer.name, layer.1);

            let tiles = tiles_layer.tiles_mut().unwrap_mut();
            *tiles = Array2::<Tile>::default((grid.shape()[1], grid.shape()[0]));

            for ((x, y), block_type) in grid.indexed_iter() {
                let tile_id = set_tile_id(x, y, block_type);
                if tile_id != 0 {
                    tiles[[y, x]] = Tile::new(tile_id, TileFlags::empty())
                }
            }

            if use_automap {
                let mapres_name = tw_map.images[tiles_layer.image.unwrap() as usize].name();
                let automapper_config =
                    TwExport::get_automapper_config(mapres_name.clone(), tiles_layer);
                automapper_config.run(AUTOMAPPER_SEED, tiles_layer.tiles_mut().unwrap_mut())
            }
        } else {
            panic!("coulnt get layer at index {:} ({:})", layer.0, layer.1);
        };
    }

    pub fn process_game_layer(tw_map: &mut TwMap, map: &Map) {
        let game_layer = tw_map
            .find_physics_layer_mut::<GameLayer>()
            .unwrap()
            .tiles_mut()
            .unwrap_mut();

        *game_layer = Array2::<GameTile>::from_elem(
            (map.height, map.width),
            GameTile::new(0, TileFlags::empty()),
        );

        for ((x, y), value) in map.grid.indexed_iter() {
            game_layer[[y, x]] = GameTile::new(value.to_tw_game_id(), TileFlags::empty())
        }
    }

    pub fn export(map: &Map, path: &PathBuf) {
        let mut tw_map = BaseMaps::get_base_map();

        // add map generator information
        tw_map.info.author = "iMilchshake".to_string();
        tw_map.info.version = format!("crate v{}", crate_version!());
        tw_map.info.credits = "https://github.com/iMilchshake/gores-mapgen".to_string();

        TwExport::process_tile_layer_new(
            &mut tw_map,
            (1, "BG_Tiles"),
            (0, "Background"),
            &map.noise_background,
            set_bool_active,
            true,
        );
        // TODO: replace with new
        TwExport::process_tile_layer(&mut tw_map, map, 0, "Freeze", &BlockTypeTW::Freeze);
        TwExport::process_tile_layer_new(
            &mut tw_map,
            (3, "FG_Tiles"),
            (1, "Hookable"),
            &map.grid,
            |_, _, block_type| block_type.is_solid() as u8,
            true,
        );
        TwExport::process_tile_layer_new(
            &mut tw_map,
            (3, "FG_Tiles"),
            (2, "Font"),
            &map.font_layer,
            set_char_id,
            false,
        );
        TwExport::process_tile_layer_new(
            &mut tw_map,
            (3, "FG_Tiles"),
            (3, "Overlay"),
            &map.noise_overlay,
            set_bool_active,
            true,
        );

        TwExport::process_game_layer(&mut tw_map, map);

        println!("exporting map to {:?}", &path);
        let mut file = std::fs::File::create(path).unwrap();
        tw_map.save(&mut file).expect("failed to write map file");
    }
}
