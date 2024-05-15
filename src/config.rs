use crate::position::Position;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;

#[derive(RustEmbed)]
#[folder = "data/gen_configs/"]
pub struct GenerationConfigStorage;

pub struct MapConfig {
    /// defines the shape of a map using waypoints
    pub waypoints: Vec<Position>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(default)]
pub struct GenerationConfig {
    /// name of the preset
    pub name: String,

    /// this can contain any description of the generation preset
    pub description: Option<String>,

    /// probability for mutating inner radius
    pub inner_rad_mut_prob: f32,

    /// probability for mutating inner size
    pub inner_size_mut_prob: f32,

    /// probability for mutating outer radius
    pub outer_rad_mut_prob: f32,

    /// probability for mutating outer size
    pub outer_size_mut_prob: f32,

    /// probability weighting for random selection from best to worst towards next goal
    pub shift_weights: Vec<i32>,

    /// (min, max) distance between platforms
    pub platform_distance_bounds: (usize, usize),

    /// probability for doing the last shift direction again
    pub momentum_prob: f32,

    /// maximum distance from empty blocks to nearest non empty block
    pub max_distance: f32,

    /// min distance to next waypoint that is considered reached
    pub waypoint_reached_dist: usize,

    /// probabilities for (inner_kernel_size, probability)
    pub inner_size_probs: Vec<(usize, f32)>,

    /// probabilities for (outer_kernel_margin, probability)
    pub outer_margin_probs: Vec<(usize, f32)>,

    // (min, max) distance for skips
    pub skip_length_bounds: (usize, usize),

    // min distance between skips
    pub skip_min_spacing_sqr: usize,

    // min unconnected freeze obstacle size
    pub min_freeze_size: usize,
}

impl GenerationConfig {
    /// returns an error if the configuration would result in a crash
    pub fn validate(&self) -> Result<(), &'static str> {
        // 1. Check that there is no inner kernel size of 0
        for (inner_size, _) in self.inner_size_probs.iter() {
            if *inner_size == 0 {
                return Err("Invalid Config! (inner_size = 0)");
            }
        }
        Ok(())
    }

    /// stores GenerationConfig in cwd as <name>.json
    pub fn save(&self, path: &str) {
        let mut file = File::create(path).expect("failed to create config file");
        let serialized = serde_json::to_string_pretty(self).expect("failed to serialize config");
        file.write_all(serialized.as_bytes())
            .expect("failed to write to config file");
    }

    pub fn load(path: &str) -> GenerationConfig {
        let serialized_from_file = fs::read_to_string(path).expect("failed to read config file");
        let deserialized: GenerationConfig =
            serde_json::from_str(&serialized_from_file).expect("failed to deserialize config file");

        deserialized
    }

    pub fn get_configs() -> HashMap<String, GenerationConfig> {
        let mut configs = HashMap::new();

        for file_name in GenerationConfigStorage::iter() {
            let file = GenerationConfigStorage::get(&file_name).unwrap();
            let data = std::str::from_utf8(&file.data).unwrap();
            let config: GenerationConfig = serde_json::from_str(data).unwrap();
            configs.insert(config.name.clone(), config);
        }

        configs
    }

    /// This function defines the initial default config for actual map generator
    pub fn get_initial_config() -> GenerationConfig {
        let file = GenerationConfigStorage::get("hardV2.json").unwrap();
        let data = std::str::from_utf8(&file.data).unwrap();
        let config: GenerationConfig = serde_json::from_str(data).unwrap();
        config
    }
}

impl Default for GenerationConfig {
    /// Default trait should mainly be used to get default values for individual arguments
    /// instead of being used as an actual generation config. (use get_initial_config())
    fn default() -> GenerationConfig {
        GenerationConfig {
            name: "default".to_string(),
            description: None,
            inner_rad_mut_prob: 0.25,
            inner_size_mut_prob: 0.5,
            outer_rad_mut_prob: 0.25,
            outer_size_mut_prob: 0.5,
            shift_weights: vec![20, 11, 10, 9],
            platform_distance_bounds: (500, 750),
            momentum_prob: 0.01,
            max_distance: 3.0,
            waypoint_reached_dist: 250,
            inner_size_probs: vec![(3, 0.25), (5, 0.75)],
            outer_margin_probs: vec![(0, 0.5), (2, 0.5)],
            skip_min_spacing_sqr: 45,
            skip_length_bounds: (3, 11),
            min_freeze_size: 0, // TODO: disable by default for now
        }
    }
}

impl Default for MapConfig {
    fn default() -> MapConfig {
        MapConfig {
            waypoints: vec![
                Position::new(250, 250),
                Position::new(250, 150),
                Position::new(50, 150),
                Position::new(50, 50),
                Position::new(250, 50),
            ],
        }
    }
}
