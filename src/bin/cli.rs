use clap::Parser;
use gores_mapgen::{
    args::CLIArgs,
    config::{GenerationConfig, MapConfig, ThemeConfig},
    generator::Generator,
    random::{Random, Seed},
};
use log::{info, warn};
use simple_logger::SimpleLogger;
use std::panic::{self};

fn main() {
    let args = CLIArgs::parse();
    SimpleLogger::new().init().unwrap();

    let all_map_configs = MapConfig::get_all_configs();
    let all_gen_configs = GenerationConfig::get_all_configs();

    let map_config = all_map_configs
        .iter()
        .find(|c| c.name == args.map_config_name)
        .unwrap_or_else(|| panic!("map config '{}' not found", args.map_config_name));

    let gen_config = all_gen_configs
        .iter()
        .find(|c| c.name == args.gen_config_name)
        .unwrap_or_else(|| panic!("gen config '{}' not found", args.map_config_name));

    let seed = args.fixed_seed.unwrap_or(Random::get_u64_from_entropy());

    // disable panic hook so they no longer get printed
    // panic::set_hook(Box::new(|_info| {}));

    let generation_result = panic::catch_unwind(|| {
        Generator::generate_map(
            args.max_gen_steps,
            &Seed::from_u64(seed),
            gen_config,
            map_config,
            &ThemeConfig::default(),
            false, // TODO: add CLIArg
        )
    });

    match generation_result {
        // map was generated successfully
        Ok(Ok(_map)) => {
            info!("generation success!");
        }
        // no panic, but map generation failed
        Ok(Err(generation_error)) => {
            warn!("generation failed: {}", generation_error)
        }
        // map generation panic
        Err(panic_info) => {
            warn!("generation panicked: {:?}", panic_info)
        }
    }
}
