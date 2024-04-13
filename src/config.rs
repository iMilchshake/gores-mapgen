use crate::position::Position;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;

#[derive(RustEmbed)]
#[folder = "configs/"]
pub struct Configs;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default = "GenerationConfig::migrate_default")]
#[serde(deny_unknown_fields)]
pub struct GenerationConfig {
    /// (min, max) values for inner kernel
    pub inner_size_bounds: (usize, usize),

    /// (min, max) values for outer kernel
    pub outer_size_bounds: (usize, usize),

    /// probability for mutating inner radius
    pub inner_rad_mut_prob: f32,

    /// probability for mutating inner size
    pub inner_size_mut_prob: f32,

    /// probability for mutating outer radius
    pub outer_rad_mut_prob: f32,

    /// probability for mutating outer size
    pub outer_size_mut_prob: f32,

    /// probability weighting for random selection from best to worst towards next goal
    pub step_weights: Vec<i32>,

    // ------- TODO: these should go somewhere else -----
    pub waypoints: Vec<Position>,
}

impl GenerationConfig {
    pub fn save(&self) {
        let mut file = File::create("config.json").expect("failed to create config file");
        let serialized = serde_json::to_string_pretty(self).expect("failed to serialize config");
        file.write_all(serialized.as_bytes())
            .expect("failed to write to config file");
    }

    pub fn load() -> GenerationConfig {
        let serialized_from_file =
            fs::read_to_string("config.json").expect("failed to read config file");
        let deserialized: GenerationConfig =
            serde_json::from_str(&serialized_from_file).expect("failed to deserialize config file");

        deserialized
    }

    /// returns the same value as default(). Used together with serde default to achieve implicit
    /// config migration using default values. Prints a warning indicating that a implicit
    /// migration happened.
    pub fn migrate_default() -> GenerationConfig {
        println!("[WARNING]: Default values of GenerationConfig have been used for implicit config migration. Consider manually migrating all configurations.");

        GenerationConfig::default()
    }
}

impl Default for GenerationConfig {
    fn default() -> GenerationConfig {
        GenerationConfig {
            inner_size_bounds: (3, 3),
            outer_size_bounds: (1, 5),
            inner_rad_mut_prob: 0.25,
            inner_size_mut_prob: 0.5,
            outer_rad_mut_prob: 0.25,
            outer_size_mut_prob: 0.5,
            waypoints: vec![
                Position::new(250, 250),
                Position::new(250, 150),
                Position::new(50, 150),
                Position::new(50, 50),
                Position::new(250, 50),
            ],
            step_weights: vec![20, 11, 10, 9],
        }
    }
}
