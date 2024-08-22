use gores_mapgen::config::{GenerationConfig, MapConfig};
use gores_mapgen::generator::Generator;
use gores_mapgen::random::Seed;

fn main() {
    for seed in 0..u64::MAX {
        println!("generating {:?}", seed);
        let _ = Generator::generate_map(
            200_000,
            &Seed::from_u64(seed),
            GenerationConfig::get_all_configs().get("insaneV2").unwrap(),
            MapConfig::get_all_configs().get("hor_line").unwrap(),
        );
    }
}
