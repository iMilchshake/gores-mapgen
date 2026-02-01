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

// TODO: Add support for custom gen/map configs (via paths)

fn main() {
    let args = CLIArgs::parse();

    let all_gen_configs = GenerationConfig::get_all_configs();
    let gen_config = all_gen_configs
        .iter()
        .find(|c| c.name == args.gen_config_name)
        .unwrap_or_else(|| panic!("gen config '{}' not found", args.gen_config_name));

    let all_map_configs = MapConfig::get_all_configs();
    let map_config = all_map_configs
        .iter()
        .find(|c| c.name == args.map_config_name)
        .unwrap_or_else(|| panic!("map config '{}' not found", args.map_config_name));

    let mut seed = if let Some(seed_base64) = args.seed {
        Seed::from_base64(&seed_base64).unwrap_or_else(|| {
            panic!("{seed_base64} is not a valid base64 seed (valid example: 'smNMEGR4lAg=')")
        })
    } else if let Some(seed_u64) = args.seed_u64 {
        Seed::from_u64(seed_u64)
    } else {
        Seed::from_u64(3777777777) // default seed when none provided via CLI
    };

    let mut seed_generator = Random::new(seed.clone(), gen_config); // used to generate new seeds (n_maps>1)
    let mut completed_count = 0;

    let progress_bar = ProgressBar::new(args.n_maps as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} maps (eta: {eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    while completed_count < args.n_maps {
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

        let map = match generation_result {
            Ok(Ok(map)) => map,
            Ok(Err(err)) => {
                progress_bar.println(format!("generation failed: {err}"));
                continue;
            }
            Err(panic_info) => {
                progress_bar.println(format!("generation panicked: {panic_info:?}"));
                continue;
            }
        };

        let export_path = if args.out_path.extension().is_some() {
            assert_eq!(
                args.n_maps, 1,
                "Path including file name is only supported for n_maps==1"
            );
            args.out_path.clone()
        } else {
            args.out_path
                .join(get_default_file_name(gen_config, map_config, &seed))
        };

        if args.dry_run {
            progress_bar.println(format!("Would have saved map to {export_path:?}"));
        } else {
            TwExport::export(&map, &export_path);
            progress_bar.println(format!("Saved map to {export_path:?}"));
        }

        completed_count += 1;
        progress_bar.inc(1);

        seed = Seed::from_random(&mut seed_generator);
    }

    progress_bar.finish();
}
