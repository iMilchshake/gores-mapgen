use clap::Parser;
use gores_mapgen::{
    args::CLIArgs,
    config::{GenerationConfig, MapConfig, ThemeConfig},
    generator::Generator,
    random::{Random, Seed},
    twmap_export::TwExport,
    utils::get_default_file_name,
};
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

    // initialize random generator using same init seed, to generate more seeds for n_maps > 1
    let mut rnd = Random::new(seed.clone(), &gen_config);
    let mut n_finished_maps = 0;

    while n_finished_maps < args.n_maps {
        // generate map
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
                    println!("Would have saved map to {:?}", export_path);
                } else {
                    TwExport::export(&map, export_path);
                    println!("Saved map to {:?}", export_path);
                }

                n_finished_maps += 1;
            }
            Ok(Err(generation_error)) => {
                println!("generation failed: {}", generation_error)
            }
            Err(panic_info) => {
                println!("generation panicked: {:?}", panic_info)
            }
        }

        seed.seed_u64 = rnd.get_u64();
    }
}
