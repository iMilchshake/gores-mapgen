use mapgen_core::{
    map::{GameTile, Map},
    position::Position,
};
use ndarray::Array2;
use serde::{self, Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use twmap::{
    automapper::Automapper,
    GameLayer, Layer, Tile, TileFlags, TilemapLayer, TwMap,
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(default)]
pub struct ExporterConfig {
    base_map: String,
    automapper_rules_path: PathBuf,

    design_group_name: String,
    freeze_layer_name: String,
    hookable_layer_name: String,
}

impl Default for ExporterConfig {
    fn default() -> Self {
        Self {
            base_map: Default::default(),
            automapper_rules_path: PathBuf::from_str("../data/rules").unwrap(),
            design_group_name: "Tiles".to_string(),
            freeze_layer_name: "Freeze".to_string(),
            hookable_layer_name: "Hookable".to_string(),
        }
    }
}

fn load_automapper(name: String, rules_path: &Path) -> Automapper {
    let content = std::fs::read_to_string(rules_path.join(format!("{}.rules", name)))
        .expect("failed to read .rules file");

    Automapper::parse(name, &content).expect("failed to parse .rules file")
}

pub struct Exporter<'a, 'b> {
    tw_map: &'a mut TwMap,
    map: &'b Map,
    config: ExporterConfig,
}

impl<'a, 'b> Exporter<'a, 'b> {
    pub fn new(tw_map: &'a mut TwMap, map: &'b Map, config: ExporterConfig) -> Exporter<'a, 'b> {
        Exporter { tw_map, map, config }
    }

    fn process_layer(&mut self, layer_type: GameTile) {
        // otherwise hookable, be aware
        let needed_design_layer_name = if layer_type == GameTile::Freeze {
            &self.config.freeze_layer_name
        } else {
            &self.config.hookable_layer_name
        };

        let mut design_group = None;
        let mut design_layer = None;

        for group in &mut self.tw_map.groups {
            if group.name == self.config.design_group_name {
                design_group = Some(group);
            }
        }

        let design_group = match design_group {
            Some(group) => group,
            None => return,
        };

        for layer in &mut design_group.layers {
            if layer.name() == needed_design_layer_name {
                design_layer = Some(layer);
            }
        }

        let design_layer = match design_layer {
            Some(layer) => layer,
            None => return,
        };

        if let Layer::Tiles(layer) = design_layer {
            let image_name = self.tw_map.images[layer.image.unwrap() as usize]
                .name()
                .clone();

            let automapper = load_automapper(image_name, &self.config.automapper_rules_path);

            let config_index = layer.automapper_config.config.unwrap();
            let automapper_config = &automapper.configs[config_index as usize];

            let tiles = layer.tiles_mut().unwrap_mut();
            *tiles = Array2::<Tile>::default((self.map.height, self.map.width));

            for ((x, y), block_type) in self.map.grid.indexed_iter() {
                let block_type = block_type.to_game_tile();
                let mut set_block: bool = layer_type == block_type;

                // custom rule for freeze
                if layer_type == GameTile::Freeze && block_type == GameTile::Hookable {
                    let shifts = &[(-1, 0), (0, -1), (1, 0), (0, 1)];
                    for shift in shifts {
                        let neighbor_type = Position::new(x, y)
                            .shifted_by(shift.0, shift.1)
                            .ok()
                            .and_then(|pos| self.map.grid.get(pos.as_index()));

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

            // thanks Tater for the epic **random** seed
            automapper_config.run(3777777777, tiles)
        } else {
            panic!(
                "coulnt get '{}' layer at group {}",
                &self.config.design_group_name, needed_design_layer_name
            );
        }
    }

    pub fn finalize<P: AsRef<Path>>(&mut self, out_path: P) {
        self.process_layer(GameTile::Freeze);
        self.process_layer(GameTile::Hookable);

        // get game layer
        let game_layer = self
            .tw_map
            .find_physics_layer_mut::<GameLayer>()
            .unwrap()
            .tiles_mut()
            .unwrap_mut();

        *game_layer = Array2::<twmap::GameTile>::from_elem(
            (self.map.height, self.map.width),
            twmap::GameTile::new(0, TileFlags::empty()),
        );

        // modify game layer
        for ((x, y), value) in self.map.grid.indexed_iter() {
            game_layer[[y, x]] = twmap::GameTile::new(value.to_ingame_id(), TileFlags::empty())
        }

        // save map
        self.tw_map
            .save_file(out_path)
            .expect("failed to write map file");
    }
}
