use std::error::Error;
use std::{collections::HashMap, fs, path::Path};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::position::{Position, ShiftDirection};
use crate::random::RandomDistConfig;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MapConfig {
    /// shape of a map using waypoints
    pub waypoints: Vec<Position>,

    /// width of the map
    pub width: usize,

    /// height of the map
    pub height: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(default)]
pub struct GenerationConfig {
    /// this can contain any description of the generation preset
    pub description: Option<String>,

    /// stores the GenerationConfig version for future migration
    pub version: String,

    /// probability for mutating inner radius
    pub inner_rad_mut_prob: f32,

    /// probability for mutating inner size
    pub inner_size_mut_prob: f32,

    /// probability for mutating outer radius
    pub outer_rad_mut_prob: f32,

    /// probability for mutating outer size
    pub outer_size_mut_prob: f32,

    /// probability weighting for random selection from best to worst towards next goal
    pub shift_weights: RandomDistConfig<ShiftDirection>,

    /// (min, max) distance between platforms
    pub platform_distance_bounds: (usize, usize),

    /// probability for doing the last shift direction again
    pub momentum_prob: f32,

    /// maximum distance from empty blocks to nearest non empty block
    pub max_distance: f32,

    /// min distance to next waypoint that is considered reached
    pub waypoint_reached_dist: usize,

    /// probabilities for (inner_kernel_size, probability)
    pub inner_size_probs: RandomDistConfig<usize>,

    /// probabilities for (outer_kernel_margin, probability)
    pub outer_margin_probs: RandomDistConfig<usize>,

    /// probabilities for (kernel circularity, probability)
    pub circ_probs: RandomDistConfig<f32>,

    /// (min, max) distance for skips
    pub skip_length_bounds: (usize, usize),

    /// min distance between skips
    pub skip_min_spacing_sqr: usize,

    /// min unconnected freeze obstacle size
    pub min_freeze_size: usize,

    /// enable pulse
    pub enable_pulse: bool,

    /// TODO:
    pub pulse_straight_delay: usize,
    pub pulse_corner_delay: usize,
    pub pulse_max_kernel_size: usize,

    /// number of initial walker steps to perform fading. Will fade from max to min kernel size.
    pub fade_steps: usize,

    /// initial max kernel size for fading
    pub fade_max_size: usize,

    /// goal min kernel size for fading
    pub fade_min_size: usize,
}

impl GenerationConfig {
    /// returns an error if the configuration would result in a crash
    pub fn validate(&self) -> Result<(), &'static str> {
        // 1. Check that there is no inner kernel size of 0
        for inner_size in self.inner_size_probs.values.as_ref().unwrap().iter() {
            if *inner_size == 0 {
                return Err("Invalid Config! (inner_size = 0)");
            }
        }

        // 2. Check fade config
        if self.fade_max_size == 0 || self.fade_min_size == 0 {
            return Err("fade kernel sizes must be larger than zero");
        }

        Ok(())
    }
}

impl Default for GenerationConfig {
    /// Default trait should mainly be used to get default values for individual arguments
    /// instead of being used as an actual generation config. (use get_initial_config())
    fn default() -> GenerationConfig {
        GenerationConfig {
            description: None,
            version: "1.0".to_string(),
            inner_rad_mut_prob: 0.25,
            inner_size_mut_prob: 0.5,
            outer_rad_mut_prob: 0.25,
            outer_size_mut_prob: 0.5,
            shift_weights: RandomDistConfig::new(None, vec![0.4, 0.22, 0.2, 0.18]),
            platform_distance_bounds: (500, 750),
            momentum_prob: 0.01,
            max_distance: 3.0,
            waypoint_reached_dist: 250,
            inner_size_probs: RandomDistConfig::new(Some(vec![3, 5]), vec![0.25, 0.75]),
            outer_margin_probs: RandomDistConfig::new(Some(vec![0, 2]), vec![0.5, 0.5]),
            circ_probs: RandomDistConfig::new(Some(vec![0.0, 0.6, 0.8]), vec![0.75, 0.15, 0.05]),
            skip_min_spacing_sqr: 45,
            skip_length_bounds: (3, 11),
            min_freeze_size: 0,
            enable_pulse: false,
            pulse_corner_delay: 5,
            pulse_straight_delay: 10,
            pulse_max_kernel_size: 4,
            fade_steps: 60,
            fade_max_size: 6,
            fade_min_size: 3,
        }
    }
}

impl Default for MapConfig {
    fn default() -> MapConfig {
        MapConfig {
            waypoints: vec![
                Position::new(50, 250),
                Position::new(250, 250),
                Position::new(250, 150),
                Position::new(50, 150),
                Position::new(50, 50),
                Position::new(250, 50),
            ],
            width: 300,
            height: 300,
        }
    }
}

pub fn load_configs_from_dir<C, P>(path: P) -> Result<HashMap<String, C>, Box<dyn Error>>
where
    C: DeserializeOwned,
    P: AsRef<Path>,
{
    let mut configs = HashMap::new();

    for file_path in fs::read_dir(path)? {
        let file_path = file_path?.path();
        let osstr_file_name = file_path.file_name().unwrap(); // it will never be None since "Returns None if the path terminates in .."
        let file_name = osstr_file_name
            .to_str().unwrap() // believe to user that it will be valid utf8, what an asshole will use utf16 for fucking generator config name?
            .replace(".json", "");

        let data = fs::read_to_string(&file_path).unwrap();

        configs.insert(file_name.to_string(), serde_json::from_str::<C>(&data)?);
    }

    Ok(configs)
}
