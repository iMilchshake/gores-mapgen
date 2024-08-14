use gores_mapgen::config::{GenerationConfig, MapConfig};
use gores_mapgen::generator::Generator;
use gores_mapgen::random::Seed;
use std::collections::HashMap;
use std::panic;
use std::time::{Duration, Instant};

// TODO: add clap cli for this?
const MAX_SEED: u64 = 100;
const MAX_GENERATION_STEPS: usize = 200_000;

fn main() {
    // disable panic hook so they no longer get printed
    panic::set_hook(Box::new(|_info| {}));

    let init_gen_configs: HashMap<String, GenerationConfig> = GenerationConfig::get_all_configs();
    let init_map_configs: HashMap<String, MapConfig> = MapConfig::get_all_configs();

    for (map_config_name, map_config) in init_map_configs.iter() {
        println!("\n### Map Layout: {map_config_name}");

        for (gen_config_name, gen_config) in init_gen_configs.iter() {
            let mut elapsed = Duration::ZERO;
            let mut panic_count = 0;
            let mut error_count = 0;
            let mut valid_count = 0;

            for seed in 0..MAX_SEED {
                let seed = Seed::from_u64(seed);

                let start_time = Instant::now();
                let generation_result = panic::catch_unwind(|| {
                    Generator::generate_map(MAX_GENERATION_STEPS, &seed, gen_config, map_config)
                });

                match generation_result {
                    // map was generated successfully
                    Ok(Ok(_map)) => {
                        elapsed += start_time.elapsed();
                        valid_count += 1;
                    }
                    // no panic, but map generation failed
                    Ok(Err(_generation_error)) => {
                        error_count += 1;
                    }
                    // map generation panic
                    Err(_panic_info) => {
                        panic_count += 1;
                    }
                }
            }

            let avg_elapsed_text = elapsed
                .checked_div(valid_count)
                .map(|v| format!("{v:?}"))
                .unwrap_or("XXX".to_string());
            let error_rate = (error_count as f32) / (MAX_SEED as f32);
            let panic_rate = (panic_count as f32) / (MAX_SEED as f32);

            println!("GEN {gen_config_name} | AVG_TIME={avg_elapsed_text} | ERROR_RATE={error_rate} | PANIC_RATE={panic_rate}");
        }
    }
}
