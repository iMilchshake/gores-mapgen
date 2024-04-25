use std::path::PathBuf;

use gores_mapgen_rust::config::GenerationConfig;
use gores_mapgen_rust::generator::Generator;
use gores_mapgen_rust::random::Seed;

fn main() {
    let map =
        Generator::generate_map(30_000, &Seed::from_u64(42), &GenerationConfig::default()).unwrap();

    map.export(&PathBuf::from(
        // "/home/tobi/.local/share/ddnet/maps/automap_out.map",
        "./automap_out.map",
    ));
}
