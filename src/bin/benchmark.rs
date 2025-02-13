use std::panic;
use std::time::{Duration, Instant};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use seed_gen::cli::{SeedIter, Seeds};

use gores_mapgen::config::{get_filtered_configs, GenerationConfig, MapConfig, ThemeConfig};
use gores_mapgen::generator::Generator;
use gores_mapgen::random::Seed;

#[derive(Parser, Debug)]
/// Benchmarks map generation with the specified options. Default is seeds 0 to 99.
pub struct Args {
    /// The maximum amount of generation steps before generation stops
    #[arg(short = 's', long, default_value = "200000")]
    pub max_generation_steps: usize,

    /// Generation presets to use (comma-separated values)
    #[arg(short, long, value_delimiter = ',')]
    pub gen_preset_names: Option<Vec<String>>,

    /// Map presets to use (comma-separated values)
    #[arg(short, long, value_delimiter = ',')]
    pub map_preset_names: Option<Vec<String>>,

    /// Specify which seed/seeds to use. Default 0 to 99
    #[command(subcommand)]
    pub seeds: Option<Seeds>,
}

/// derive seed iter from cli args, use default 0 to 99 if non is specified
fn get_seed_iter(args: &Args) -> SeedIter {
    args.seeds
        .clone()
        .unwrap_or(Seeds::Range {
            min: 0,
            max: 99,
            step: None,
        })
        .iter()
}

fn main() {
    let args = Args::parse();

    // determine seed count
    // TODO: iterates over one entire seed_gen once.. -> implement size_hint()?
    let seed_count = get_seed_iter(&args).count();

    let init_map_configs = match &args.map_preset_names {
        Some(preset_names) => get_filtered_configs(&MapConfig::get_all_configs(), preset_names),
        None => MapConfig::get_all_configs(),
    };

    let init_gen_configs = match &args.gen_preset_names {
        Some(preset_names) => {
            get_filtered_configs(&GenerationConfig::get_all_configs(), preset_names)
        }
        None => GenerationConfig::get_all_configs(),
    };

    // disable panic hook so they no longer get printed
    panic::set_hook(Box::new(|_info| {}));

    for map_config in init_map_configs.iter() {
        println!(
            "\n### LAYOUT={} | LENGTH={:.1}",
            map_config.name,
            map_config.get_map_length()
        );

        for gen_config in init_gen_configs.iter() {
            let mut elapsed = Duration::ZERO;
            let mut panic_count = 0;
            let mut error_count = 0;
            let mut valid_count = 0;

            let pb = ProgressBar::new(seed_count as u64);
            pb.set_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise}] {bar:56.cyan/blue} {pos:>7}/{len:7} {msg}",
                )
                .unwrap()
                .progress_chars("##-"),
            );
            for seed in get_seed_iter(&args) {
                let seed = Seed::from_u64(seed);
                let start_time = Instant::now();
                let generation_result = panic::catch_unwind(|| {
                    Generator::generate_map(
                        args.max_generation_steps,
                        &seed,
                        gen_config,
                        map_config,
                        &ThemeConfig::default(),
                        false,
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
                pb.inc(1);
            }
            pb.finish_and_clear();

            let avg_elapsed_text = elapsed
                .checked_div(valid_count)
                .map(|v| format!("{} ms", v.as_millis()))
                .unwrap_or("?".to_string());
            let error_rate = (error_count as f32) / (seed_count as f32);
            let panic_rate = (panic_count as f32) / (seed_count as f32);

            println!(
                "GEN {:<15} | AVG_TIME={:<12} | ERROR_RATE={:<4.2} | PANIC_RATE={:<4.2}",
                gen_config.name, avg_elapsed_text, error_rate, panic_rate
            );
        }
    }
}
