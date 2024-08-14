use gores_mapgen::config::{GenerationConfig, MapConfig};
use gores_mapgen::generator::Generator;
use gores_mapgen::random::Seed;
use rand::prelude::*;
use std::collections::HashMap;
use std::panic;
use std::time::Instant;

fn main() {
    let init_gen_configs: HashMap<String, GenerationConfig> = GenerationConfig::get_all_configs();
    let init_map_configs: HashMap<String, MapConfig> = MapConfig::get_all_configs();

    let seed: u64 = random::<u64>();

    for (gen_config_name, gen_config) in init_gen_configs.iter() {
        for (map_config_name, map_config) in init_map_configs.iter() {
            let seed = Seed::from_u64(seed);
            let start_time = Instant::now();
            let _ = panic::catch_unwind(|| {
                let gen_result = Generator::generate_map(200_000, &seed, gen_config, map_config);
                let elapsed = start_time.elapsed(); 
                match gen_result {
                    Ok(_) => {
                        println!("GEN {gen_config_name} WITH {map_config_name} MAP GEN | ELAPSED TIME: {elapsed:?\n}")
                    }
                    Err(e) => println!(
                        "ERROR IN GENERATING MAP: {e} | GENERATION THAT FAILED: {gen_config_name} with {map_config_name}"
                    ),
                }
            });
        }
    }
}
