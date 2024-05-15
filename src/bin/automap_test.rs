use std::path::PathBuf;

use gores_mapgen_rust::config::{GenerationConfig, MapConfig};
use gores_mapgen_rust::generator::Generator;
use gores_mapgen_rust::random::Seed;

fn main() {
    let map = Generator::generate_map(
        30_000,
        &Seed::from_u64(42),
        &GenerationConfig::default(),
        &MapConfig::default(),
    )
    .unwrap();

    map.export(&PathBuf::from(
        "/home/tobi/.local/share/ddnet/maps/automap_out.map",
        // "./automap_out.map",
    ));
}
