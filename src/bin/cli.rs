use clap::Parser;
use gores_mapgen::{
    args::CLIArgs,
    config::{GenerationConfig, MapConfig, ThemeConfig},
    generator::Generator,
    random::{Random, Seed},
    twmap_export::TwExport,
    utils::get_default_file_name,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::panic;

fn main() {
    let args = CLIArgs::parse();

    // get generation config
    let all_gen_configs = GenerationConfig::get_all_configs();
    let gen_config = all_gen_configs
        .iter()
        .find(|c| c.name == args.gen_config_name)
        .unwrap_or_else(|| panic!("gen config '{}' not found", args.map_config_name));

    // get map config
    let all_map_configs = MapConfig::get_all_configs();
    let map_config = all_map_configs
        .iter()
        .find(|c| c.name == args.map_config_name)
        .unwrap_or_else(|| panic!("map config '{}' not found", args.map_config_name));

    // get initial seed
    let mut seed = if let Some(seed_base64) = args.seed {
        Seed::from_base64(&seed_base64).expect(&format!(
            "{:} is not a valid base64 seed (valid example: 'smNMEGR4lAg=')",
            seed_base64
        ))
    } else if let Some(seed_u64) = args.seed_u64 {
        Seed::from_u64(seed_u64)
    } else {
        Seed::from_u64(3777777777) // thanks Tater for the epic **random** seed
    };

    let mut rnd = Random::new(seed.clone(), &gen_config); // used to generate new seeds (n_maps>1)
    let mut n_finished_maps = 0;
    let mut n_attempts = 0;

    let progress_bar = ProgressBar::new(args.n_maps as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} maps (eta: {eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    while n_finished_maps < args.n_maps {
        n_attempts += 1;
        progress_bar.set_message(format!("attempt {}", n_attempts));

        let generation_result = panic::catch_unwind(|| {
            Generator::generate_map(
                args.max_steps,
                &seed,
                gen_config,
                map_config,
                &ThemeConfig::default(),
                true,
            )
        });

        match generation_result {
            Ok(Ok(map)) => {
                // get final path
                let is_file_path = args.out_path.extension().is_some();
                let export_path = if is_file_path {
                    assert_eq!(
                        args.n_maps, 1,
                        "Path including file name is only supported for n_maps==1"
                    );
                    &args.out_path
                } else {
                    let filename = get_default_file_name(gen_config, map_config, &seed);
                    &args.out_path.join(filename)
                };

                // export
                if args.dry_run {
                    progress_bar.println(format!("Would have saved map to {:?}", export_path));
                } else {
                    TwExport::export(&map, export_path);
                    progress_bar.println(format!("Saved map to {:?}", export_path));
                }

                n_finished_maps += 1;
                progress_bar.inc(1);
            }
            Ok(Err(generation_error)) => {
                progress_bar.println(format!("generation failed: {}", generation_error));
            }
            Err(panic_info) => {
                progress_bar.println(format!("generation panicked: {:?}", panic_info));
            }
        }

        seed.seed_u64 = rnd.get_u64(); // update seed
    }

    progress_bar.finish_with_message(format!(
        "completed {} maps in {} attempts",
        n_finished_maps, n_attempts
    ));
}
