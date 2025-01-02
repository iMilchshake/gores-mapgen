use clap::{crate_version, Parser};

#[derive(Parser, Debug)]
#[command(name = "Random Gores Map Generator - Editor")]
#[command(version = crate_version!())]
#[command(about = "Visual editor for generating maps and customizing the generators presets", long_about = None)]
pub struct EditorArgs {
    /// select initial generation config
    pub gen_config: Option<String>,

    /// select initial map config
    pub map_config: Option<String>,

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
}

#[derive(Parser, Debug)]
#[command(name = "Random Gores Map Generator - CLI")]
#[command(version = crate_version!())]
#[command(about = "CLI for generating maps using generators presets", long_about = None)]
pub struct CLIArgs {
    /// select initial generation config
    pub gen_config_name: String,

    /// select initial map config
    pub map_config_name: String,

    /// enable fixed seed
    #[arg(short = 's', long = "seed")]
    pub fixed_seed: Option<u64>,

    /// The maximum amount of generation steps before generation stops
    #[arg(long, default_value = "200000")]
    pub max_gen_steps: usize,
}
