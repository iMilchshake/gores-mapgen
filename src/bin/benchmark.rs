use std::collections::HashMap;
use std::panic;
use std::time::{Duration, Instant};

use gores_mapgen::config::{GenerationConfig, MapConfig};
use gores_mapgen::generator::Generator;

use clap::Parser;
use gores_mapgen::random::Seed;
use seed_gen::cli::Seeds;

#[derive(Parser, Debug)]
/// Benchmarks map generation with the specified options. Default is seeds 0 to 100.
pub struct Args {
    #[arg(short, long, default_value = "200000")]
    /// The maximum amount of generation steps before generation stops
    pub max_generation_steps: usize,

    #[command(subcommand)]
    /// Specify which seed/seeds to use. Default 0 to 100
    pub seeds: Option<Seeds>,
}

fn main() {
    let args = Args::parse();

    // disable panic hook so they no longer get printed
    panic::set_hook(Box::new(|_info| {}));

    // TODO: it would be great to sort these by name, so the order of map/gen configs is
    // consistent. But i guess this should be done in the config storage, not here.
    let init_gen_configs: HashMap<String, GenerationConfig> = GenerationConfig::get_all_configs();
    let init_map_configs: HashMap<String, MapConfig> = MapConfig::get_all_configs();

    for (map_config_name, map_config) in init_map_configs.iter() {
        println!(
            "\n### LAYOUT={} | LENGTH={:.1}",
            map_config_name,
            map_config.get_map_length()
        );

        for (gen_config_name, gen_config) in init_gen_configs.iter() {
            let mut elapsed = Duration::ZERO;
            let mut panic_count = 0;
            let mut error_count = 0;
            let mut valid_count = 0;
            let mut iterations = 0;

            let seeds = args.seeds.clone().unwrap_or(Seeds::Range {
                min: 0,
                max: 100,
                step: None,
            });

            for seed in &seeds {
                let seed = Seed::from_u64(seed);
                iterations += 1;
                let start_time = Instant::now();
                let generation_result = panic::catch_unwind(|| {
                    Generator::generate_map(
                        args.max_generation_steps,
                        &seed,
                        gen_config,
                        map_config,
                    )
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
                .map(|v| format!("{} ms", v.as_millis()))
                .unwrap_or("?".to_string());
            let error_rate = (error_count as f32) / (iterations as f32);
            let panic_rate = (panic_count as f32) / (iterations as f32);

            println!(
                "GEN {:<15} | AVG_TIME={:<12} | ERROR_RATE={:<4.2} | PANIC_RATE={:<4.2}",
                gen_config_name, avg_elapsed_text, error_rate, panic_rate
            );
        }
    }
}
