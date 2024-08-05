use std::{fs, path::PathBuf};

use clap::{command, crate_version, Parser};
use exporter::{Exporter, ExporterConfig};
use mapgen_core::{
    config::{GenerationConfig, MapConfig},
    generator::Generator,
    map::{Map, BlockType},
    random::Seed,
};
use twmap::TwMap;

pub mod exporter;

#[derive(Parser, Debug)]
struct ExporterArgs {
    /// debug to console
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// path to base map
    #[arg(long, default_value = "./base.map")]
    base_map: PathBuf,

    /// path to map config
    #[arg(long, default_value = "./map.json")]
    map_config: PathBuf,

    /// path to generator config
    #[arg(long, default_value = "./gen.json")]
    gen_config: PathBuf,

    /// path to exporter config
    #[arg(long, default_value = "./exp.json")]
    exp_config: PathBuf,

    /// path to exporter config
    #[arg(long, default_value = "./out.map")]
    out: PathBuf,

    /// seed for generation
    #[arg(long, default_value_t = 0xdeadbeef)]
    seed: u64,

    /// max steps of generation
    #[arg(long, default_value_t = 0xb00b)]
    max_steps: usize,
}

#[derive(Parser, Debug)]
#[command(name = "mapgen-exporter")]
#[command(version = crate_version!())]
#[command(about = "Generate and export gores maps", long_about = None)]
enum Command {
    #[clap(name = "genex", about = "Generate and export gores map with provided configurations")]
    Genex(ExporterArgs),
}

fn main() {
    match Command::parse() {
        Command::Genex(args) => {
            let map_config_data = fs::read_to_string(args.map_config).expect("failed to load map configuration");
            let gen_config_data = fs::read_to_string(args.gen_config).expect("failed to load generator configuration");
            let exp_config_data = fs::read_to_string(args.exp_config).expect("failed to load exporter configuration");

            let map_config: MapConfig = serde_json::from_str(&map_config_data).unwrap();
            let gen_config: GenerationConfig = serde_json::from_str(&gen_config_data).unwrap();
            let exp_config: ExporterConfig = serde_json::from_str(&exp_config_data).unwrap();

            let mut tw_map = TwMap::parse_file(args.base_map).expect("failed to parse base map");
            tw_map.load().expect("failed to load base map");

            let mut generator = Generator::new(
                Map::new(map_config, BlockType::Hookable),
                Seed::from_u64(args.seed),
                gen_config,
            );

            generator.finalize(args.max_steps).unwrap();

            let mut exporter = Exporter::new(&mut tw_map, &generator.map, exp_config);

            exporter.finalize().save_map(&args.out);
        }
    }
}
