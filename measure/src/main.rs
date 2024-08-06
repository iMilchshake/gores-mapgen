use mapgen_core::{
    config::{load_configs_from_dir, GenerationConfig, MapConfig},
    generator::Generator,
    random::Seed,
};

const SILENT: bool = true;
const LAST_SEED: u64 = 100_000; // u64::MAX

fn main() {
    use std::time::Instant;

    let gen_config = load_configs_from_dir::<GenerationConfig, _>("../data/configs/gen")
        .unwrap()
        .get("insaneV2")
        .unwrap()
        .clone();
    let map_config = load_configs_from_dir::<MapConfig, _>("../data/configs/map")
        .unwrap()
        .get("hor_line")
        .unwrap()
        .clone();

    let now = Instant::now();

    {
        for seed in 0..LAST_SEED {
            if !SILENT {
                print!("processing {}", seed);
            }

            let result = Generator::generate_map(
                200_000,
                Seed::from_u64(seed),
                gen_config.clone(),
                map_config.clone(),
            );

            if !SILENT {
                match result {
                    Err(error) => println!(": {}", error),
                    _ => println!(": success"),
                }
            }
        }
    }

    let elapsed = now.elapsed();

    println!("elapsed {:.2?}", elapsed);
}
