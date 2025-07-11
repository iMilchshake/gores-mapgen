use crate::noise::Noise;
use crate::position::{Position, ShiftDirection};
use crate::random::{Random, RandomDistConfig};
use log::warn;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;

pub const MAP_LENGTH_BASELINE: f32 = 650.0;

pub fn get_config_points(gen_config: &GenerationConfig, map_config: &MapConfig) -> f32 {
    gen_config.difficulty * (map_config.get_map_length() / MAP_LENGTH_BASELINE)
}

#[derive(RustEmbed)]
#[folder = "data/gen_configs/"]
pub struct GenerationConfigStorage;

#[derive(RustEmbed)]
#[folder = "data/map_configs/"]
pub struct MapConfigStorage;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MapConfig {
    /// name of the map config
    pub name: String,

    /// shape of a map using waypoints
    pub waypoints: Vec<Position>,

    /// width of the map
    pub width: usize,

    /// height of the map
    pub height: usize,
}

impl MapConfig {
    pub fn get_all_configs() -> Vec<MapConfig> {
        let mut configs: Vec<MapConfig> = Vec::new();

        for file_name in MapConfigStorage::iter() {
            let file = MapConfigStorage::get(&file_name).unwrap();
            let data = std::str::from_utf8(&file.data).unwrap();
            match serde_json::from_str::<MapConfig>(data) {
                Ok(config) => {
                    configs.push(config);
                }
                Err(e) => {
                    warn!("couldn't parse map config {}: {}", file_name, e);
                }
            }
        }

        configs.sort_by(|a, b| a.name.cmp(&b.name));

        configs
    }

    pub fn save(&self, path: &str) {
        let mut file = File::create(path).expect("failed to create config file");
        let serialized = serde_json::to_string_pretty(self).expect("failed to serialize config");
        file.write_all(serialized.as_bytes())
            .expect("failed to write to config file");
    }

    /// This function defines the initial default config for actual map generator
    pub fn get_initial_config() -> MapConfig {
        let file = MapConfigStorage::get("small_s_tight.json").unwrap();
        let data = std::str::from_utf8(&file.data).unwrap();
        let config: MapConfig = serde_json::from_str(data).unwrap();
        config
    }

    /// calculates approximative map length based on waypoints
    pub fn get_map_length(&self) -> f32 {
        self.waypoints
            .windows(2)
            .map(|w| w[0].distance(&w[1]))
            .sum()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(default)]
pub struct GenerationConfig {
    /// name of the preset
    pub name: String,

    /// this can contain any description of the generation preset
    pub description: Option<String>,

    /// difficulty
    pub difficulty: f32,

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

    // ===================================[ platforms ]==========================================
    /// TODO: min euclidean distance between platforms -> the initial idea here was that i could
    /// have maps with multiple paths, and the generation still work, so that two platforms that
    /// are equally far on the map, but spaced apart on different paths can co-exist. However,
    /// the following implementation assumes that there is a single path for fair platform
    /// placement based on inter-platform gaps.
    /// TODO: Add a second platform selection algorithm specifically for chaotic presets (e.g. maze)
    // pub plat_max_euclidean_distance: usize,
    pub plat_target_distance: usize,
    pub plat_max_freeze: usize,
    pub plat_height: usize,
    pub plat_min_width: usize,
    pub plat_max_width: usize,

    // ===================================[ ]==========================================
    /// probability for doing the last shift direction again
    pub momentum_prob: f32,

    /// maximum distance from empty blocks to nearest non empty block for obstacle generation
    /// TODO: rename in new version bump, as this is not self explanatory at all xd
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

    /// min distance between skips. If a skip is validated, all neighbouring skips closer than this
    /// range are invalidated.
    pub skip_min_spacing_sqr: usize,

    /// maximum amount of the level is allowed to skip. This ensures that different parts of a map
    /// are not connected.
    pub max_level_skip: usize,

    /// min unconnected freeze obstacle size
    pub min_freeze_size: usize,

    // ===================================[ Pulse ]==========================================
    /// enable pulse
    pub enable_pulse: bool,

    /// TODO:
    pub pulse_straight_delay: usize,
    pub pulse_corner_delay: usize,
    pub pulse_max_kernel_size: usize,

    // ===================================[ Fade ]==========================================
    /// number of initial walker steps to perform fading. Will fade from max to min kernel size.
    pub fade_steps: usize,

    /// initial max kernel size for fading
    pub fade_max_size: usize,

    /// goal min kernel size for fading
    pub fade_min_size: usize,

    // ===================================[ Waypoints ]=====================================

    // maximum valid distance between subwaypoints
    pub max_subwaypoint_dist: f32,

    /// maximum distance that subwaypoints are shifted from their base position
    pub subwaypoint_max_shift_dist: f32,

    /// whether to ignore waypoints that are locked or out of bounds
    pub skip_invalid_waypoints: bool,

    // ===================================[ Locking ]=====================================
    /// how close previous positions may be locked
    pub pos_lock_max_dist: f32,

    /// how many steps the locking may lack behind until the generation is considered "stuck"
    pub pos_lock_max_delay: usize,

    /// whether kernel locking should be used
    pub enable_kernel_lock: bool,

    /// how far future waypoints lock blocks around them to ensure
    /// they arent blocked before having to actually be reached
    pub waypoint_lock_distance: usize,

    // ===================================[ Dead End Removal ]=============================
    /// enable dead end removal feature
    pub use_dead_end_removal: bool,

    /// how far empty blocks need be be apart from "main path" for them to count as a dead end
    pub dead_end_threshold: usize,
}

impl GenerationConfig {
    /// returns an error if the configuration would result in a crash
    pub fn validate(&self) -> Result<(), &'static str> {
        // check that there is no inner kernel size of 0
        for inner_size in self.inner_size_probs.values.as_ref().unwrap().iter() {
            if *inner_size == 0 {
                return Err("Invalid Config! (inner_size = 0)");
            }
        }

        // check that even in worse case, after fixing edge bugs, still at least 1x1 empty is left
        let min_inner_size = self
            .inner_size_probs
            .values
            .as_ref()
            .unwrap()
            .iter()
            .min()
            .unwrap();
        let min_outer_margin = self
            .outer_margin_probs
            .values
            .as_ref()
            .unwrap()
            .iter()
            .min()
            .unwrap();
        if (min_inner_size + min_outer_margin) < 3 {
            return Err("kernel inner+outer must be at least 3");
        }

        // check that shift_prob[0] > shift_prob[3], otherwise walker will diverge
        if self.shift_weights.probs[0] < self.shift_weights.probs[3] {
            return Err("shift_prob[0] must be larger than shift_prob[4], walker will diverge");
        }

        // check fade config
        if self.fade_max_size == 0 || self.fade_min_size == 0 {
            return Err("fade kernel sizes must be larger than zero");
        }

        // check subwaypoint config
        if self.max_subwaypoint_dist <= 0.0 {
            return Err("max subwaypoint distance must be >0");
        }

        Ok(())
    }

    pub fn random(rnd: &mut Random) -> GenerationConfig {
        let use_locking = rnd.get_bool_with_prob(0.5);

        let (max_subwaypoint_dist, subwaypoint_max_shift_dist) = if use_locking {
            (
                rnd.get_f32_in_range(20.0, 50.0), // greatly reduce number of sub-waypoints
                rnd.get_f32_in_range(0.0, 5.0),   // greatly reduce waypoint shift
            )
        } else {
            (
                rnd.get_f32_in_range(1.0, 100.0),
                rnd.get_f32_in_range(0.0, 50.0),
            )
        };

        // as shift_weights always requires exactly 4 values, i just generate it like this..
        let mut shift_weights = RandomDistConfig::new(
            None,
            vec![
                rnd.get_unit_ratio(),
                rnd.get_unit_ratio(),
                rnd.get_unit_ratio(),
                rnd.get_unit_ratio(),
            ],
        );
        shift_weights.normalize_probs();

        let mut circ_probs = RandomDistConfig::new(
            Some(vec![0.0, 0.6, 0.8]),
            vec![
                rnd.get_unit_ratio(),
                rnd.get_unit_ratio(),
                rnd.get_unit_ratio(),
            ],
        );
        circ_probs.normalize_probs();

        let outer_margin_ratio = rnd.get_unit_ratio();
        let outer_margin_probs = RandomDistConfig::new(
            Some(vec![0, 2, 4]),
            vec![
                1. - outer_margin_ratio,
                outer_margin_ratio,
                (outer_margin_ratio * outer_margin_ratio),
            ],
        );

        GenerationConfig {
            name: "Random".to_string(),
            inner_rad_mut_prob: rnd.get_unit_ratio(),
            inner_size_mut_prob: rnd.get_unit_ratio(),
            outer_rad_mut_prob: rnd.get_unit_ratio(),
            outer_size_mut_prob: rnd.get_unit_ratio(),
            // plat_max_euclidean_distance: rnd.get_usize_in_range(0, 100),
            plat_target_distance: rnd.get_usize_in_range(0, 100),
            plat_max_freeze: rnd.get_usize_in_range(1, 5),
            plat_height: rnd.get_usize_in_range(1, 10),
            plat_min_width: rnd.get_usize_in_range(1, 7),
            momentum_prob: rnd.get_unit_ratio(),
            max_distance: rnd.get_f32_in_range(1.42, 5.0),
            waypoint_reached_dist: rnd.get_usize_in_range(5, 500),
            inner_size_probs: rnd.get_random_usize_dist_config(6, Some((1, 8))),
            skip_min_spacing_sqr: rnd.get_usize_in_range(1, 10) * rnd.pick_from_slice(&[1, 10]),
            skip_length_bounds: rnd.get_bounds(1, 50),
            max_level_skip: rnd.get_usize_in_range(5, 1000),
            enable_pulse: rnd.get_bool_with_prob(0.5),
            pulse_corner_delay: rnd.get_usize_in_range(0, 15),
            pulse_straight_delay: rnd.get_usize_in_range(0, 15),
            pulse_max_kernel_size: rnd.get_usize_in_range(0, 5),
            fade_steps: rnd.get_usize_in_range(0, 200),
            pos_lock_max_delay: rnd.get_usize_in_range(1, 10_000),
            pos_lock_max_dist: rnd.get_f32_in_range(1.0, 100.0),
            shift_weights,
            outer_margin_probs,
            circ_probs,
            max_subwaypoint_dist,
            subwaypoint_max_shift_dist,
            // lock_kernel_size,
            min_freeze_size: 0, // disable blob removal for now?
            // waypoint locking can make generation more stable, but for random
            // configs it mostly screws stuff up, so im just disabling it :)
            waypoint_lock_distance: 0,
            // inner_size_probs: RandomDistConfig::new(Some(vec![3, 5]), vec![0.25, 0.75]),
            // fade_max_size: 6,
            // fade_min_size: 3,
            ..Default::default()
        }
    }

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

    pub fn get_all_configs() -> Vec<GenerationConfig> {
        let mut configs: Vec<GenerationConfig> = Vec::new();

        for file_name in GenerationConfigStorage::iter() {
            let file = GenerationConfigStorage::get(&file_name).unwrap();
            let data = std::str::from_utf8(&file.data).unwrap();
            match serde_json::from_str::<GenerationConfig>(data) {
                Ok(config) => {
                    configs.push(config);
                }
                Err(e) => {
                    warn!("couldn't parse gen config {}: {}", file_name, e);
                }
            }
        }

        configs.sort_by(|a, b| a.name.cmp(&b.name));

        configs
    }

    /// This function defines the initial default config for actual map generator
    pub fn get_initial_config() -> GenerationConfig {
        if let Some(file) = GenerationConfigStorage::get("hard.json") {
            if let Ok(data) = std::str::from_utf8(&file.data) {
                if let Ok(config) = serde_json::from_str(data) {
                    return config;
                }
            }
        }

        GenerationConfig::default()
    }
}

impl Default for GenerationConfig {
    /// Default trait should mainly be used to get default values for individual arguments
    /// instead of being used as an actual generation config. (use get_initial_config())
    fn default() -> GenerationConfig {
        GenerationConfig {
            name: "default".to_string(),
            description: None,
            difficulty: 1.0,
            version: "1.0".to_string(),
            inner_rad_mut_prob: 0.25,
            inner_size_mut_prob: 0.5,
            outer_rad_mut_prob: 0.25,
            outer_size_mut_prob: 0.5,
            shift_weights: RandomDistConfig::new(None, vec![0.4, 0.22, 0.2, 0.18]),
            // plat_max_euclidean_distance: 150,
            plat_target_distance: 150,
            plat_max_freeze: 2,
            plat_height: 4,
            plat_min_width: 3,
            plat_max_width: 10,
            momentum_prob: 0.01,
            max_distance: 3.0,
            waypoint_reached_dist: 250,
            inner_size_probs: RandomDistConfig::new(Some(vec![3, 5]), vec![0.25, 0.75]),
            outer_margin_probs: RandomDistConfig::new(Some(vec![0, 2]), vec![0.5, 0.5]),
            circ_probs: RandomDistConfig::new(Some(vec![0.0, 0.6, 0.8]), vec![0.75, 0.15, 0.05]),
            skip_min_spacing_sqr: 45,
            skip_length_bounds: (3, 11),
            max_level_skip: 90,
            min_freeze_size: 0,
            enable_pulse: false,
            pulse_corner_delay: 5,
            pulse_straight_delay: 10,
            pulse_max_kernel_size: 4,
            fade_steps: 60,
            fade_max_size: 6,
            fade_min_size: 3,
            max_subwaypoint_dist: 50.0,
            subwaypoint_max_shift_dist: 5.0,
            skip_invalid_waypoints: false,
            pos_lock_max_delay: 1000,
            pos_lock_max_dist: 20.0,
            enable_kernel_lock: true,
            waypoint_lock_distance: 10,
            use_dead_end_removal: true,
            dead_end_threshold: 10,
        }
    }
}

impl Default for MapConfig {
    fn default() -> MapConfig {
        MapConfig {
            name: "default".to_string(),
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

/// Configuration for map visuals and appearance, isolated from map and generation config.
pub struct ThemeConfig {
    /// spawn width
    pub spawn_width: usize,

    /// spawn height (must be even TODO: u sure?)
    pub spawn_height: usize,

    /// how many blocks are reserved outside of spawn (not in starting orientation)
    /// TODO: is technically a padding, not a margin as it is reduced from width/height
    pub spawn_margin: usize,

    pub spawn_platform_width: usize,

    /// empty blocks around info text
    pub text_margin: usize,

    /// text box top offset (relative to bot left spawn corner)
    pub textbox_top_offset: usize,

    /// text box left offset (relative to bot left spawn corner)
    pub textbox_left_offset: usize,

    pub overlay_noise_scale: f32,
    pub overlay_noise_invert: bool,
    pub overlay_noise_threshold: f32,
    pub overlay_noise_type: Noise,

    pub background_noise_scale: f32,
    pub background_noise_invert: bool,
    pub background_noise_threshold: f32,
    pub background_noise_type: Noise,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            spawn_width: 40,
            spawn_height: 32,
            spawn_margin: 6,
            spawn_platform_width: 12,
            text_margin: 1,
            textbox_top_offset: 3,
            textbox_left_offset: 5,
            overlay_noise_scale: 10.0,
            overlay_noise_invert: false,
            overlay_noise_threshold: 0.25,
            overlay_noise_type: Noise::Worley,
            background_noise_scale: 30.0,
            background_noise_invert: false,
            background_noise_threshold: 0.00,
            background_noise_type: Noise::Perlin,
        }
    }
}

pub trait Config {
    fn get_name(&self) -> &String;
}

impl Config for MapConfig {
    fn get_name(&self) -> &String {
        &self.name
    }
}

impl Config for GenerationConfig {
    fn get_name(&self) -> &String {
        &self.name
    }
}

pub fn get_filtered_configs<T>(configs: &[T], preset_names: &[String]) -> Vec<T>
where
    T: Config + Clone + std::fmt::Debug,
{
    let filtered_configs: Vec<_> = configs
        .iter()
        .filter(|config| preset_names.contains(config.get_name()))
        .cloned()
        .collect();

    if filtered_configs.is_empty() {
        panic!(
            "no configs left after filtering, preset_names={:?}",
            preset_names
        );
    }

    filtered_configs
}
