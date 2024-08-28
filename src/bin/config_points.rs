use gores_mapgen::config::{get_config_points, GenerationConfig, MapConfig, MAP_LENGTH_BASELINE};

fn main() {
    let init_gen_configs = GenerationConfig::get_all_configs();
    let init_map_configs = MapConfig::get_all_configs();

    for map_config in init_map_configs.iter() {
        let map_length_mod = map_config.get_map_length() / MAP_LENGTH_BASELINE;
        println!(
            "\n### {:<23} | LENGTH={:<9.1} | LENGTH_MODIFIER={:.2}",
            map_config.name,
            map_config.get_map_length(),
            map_length_mod,
        );

        for gen_config in init_gen_configs.iter() {
            println!(
                "\tGEN {:<15} | DIFFICULTY={:<5} | POINTS={:<5.2} | FINAL={:}",
                gen_config.name,
                gen_config.difficulty,
                get_config_points(gen_config, map_config),
                get_config_points(gen_config, map_config).floor() as usize
            );
        }
    }
}
