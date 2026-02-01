use std::path::PathBuf;

use clap::Parser;

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")");

#[derive(Parser, Debug)]
#[command(name = "gores-mapgen: Editor")]
#[command(version = VERSION)]
#[command(about = "Visual editor for generating maps and customizing the generators presets", long_about = None)]
pub struct EditorArgs {
    /// select initial generation config
    pub gen_config: Option<String>,

    /// select initial map config
    pub map_config: Option<String>,

    /// select initial seed (base64)
    pub init_seed: Option<String>,

    /// trigger map generation on startup
    #[arg(short = 'g', long)]
    pub generate: bool,

    /// enable instant generation
    #[arg(short = 'i', long)]
    pub instant: bool,

    /// enable fixed seed
    #[arg(short = 'f', long)]
    pub fixed_seed: bool,

    /// enable auto generation
    #[arg(short = 'a', long)]
    pub auto_generation: bool,

    /// disable all debug visualization calculations for improved performance
    #[arg(short = 'd', long)]
    pub disable_debug: bool,

    /// comma seperated list of debug layers to enable on startup
    #[arg(short = 'e', long, value_delimiter = ',', num_args = 1..)]
    pub enable_layers: Option<Vec<String>>,

    /// maximum number of retries when generation fails
    #[arg(short = 'r', long, default_value = "25")]
    pub max_retries: usize,
}

#[derive(Parser, Debug)]
#[command(name = "gores-mapgen: CLI")]
#[command(version = VERSION)]
#[command(about = "CLI for procedual random map generator for the gores gamemode in DDNet.", long_about = None)]
pub struct CLIArgs {
    /// select initial generation config
    pub gen_config_name: String,

    /// select initial map config
    pub map_config_name: String,

    /// output path (can be file path for single map generation, otherwise use folder path)
    #[arg(short = 'o', long = "out", default_value = ".")]
    pub out_path: PathBuf,

    /// dry run, dont save generated maps
    #[arg(short = 'd')]
    pub dry_run: bool,

    /// set base64 fixed seed
    #[arg(short = 's', long = "seed", conflicts_with = "seed_u64")]
    pub seed: Option<String>,

    /// set u64 fixed seed
    #[arg(long = "seed_u64", conflicts_with = "seed")]
    pub seed_u64: Option<u64>,

    /// number of of maps to generate
    #[arg(short = 'n', default_value_t = 1)]
    pub n_maps: usize,

    /// The maximum amount of generation steps before generation stops
    #[arg(long, default_value = "200000")]
    pub max_steps: usize,
}
