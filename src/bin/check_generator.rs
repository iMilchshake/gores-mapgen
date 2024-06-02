use gores_mapgen_rust::config::{GenerationConfig, MapConfig};
use gores_mapgen_rust::generator::Generator;
use gores_mapgen_rust::random::Seed;

fn main() {
    for seed in 0..u64::max_value() {
        println!("generating {:?}", seed);
        Generator::generate_map(
            200_000,
            &Seed::from_u64(seed),
            &GenerationConfig::get_all_configs().get("insaneV2").unwrap(),
            &MapConfig::get_all_configs().get("hor_line").unwrap(),
        );
    }
}
