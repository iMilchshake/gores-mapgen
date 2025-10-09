use gores_mapgen::{
    config::{GenerationConfig, MapConfig, ThemeConfig},
    generator::Generator,
    map::BlockType,
    random::Seed,
};
use ndarray::Array2;

const TEST_SEED: u64 = 42;
const MAX_STEPS: usize = 100_000;
const MAX_RETRIES: u64 = 100;

fn grid_to_string(grid: &Array2<BlockType>) -> String {
    let mut result = String::new();

    for y in 0..grid.shape()[1] {
        for x in 0..grid.shape()[0] {
            let ch = match &grid[(x, y)] {
                BlockType::Empty => ' ',
                BlockType::EmptyFade => '·',
                BlockType::EmptyRoom => '◦',
                BlockType::EmptyPlatform => '∘',
                BlockType::Hookable => '█',
                BlockType::Platform => '▬',
                BlockType::Freeze => '▒',
                BlockType::Spawn => '◉',
                BlockType::Start => '▸',
                BlockType::Finish => '◂',
            };
            result.push(ch);
        }
        result.push('\n');
    }

    result
}

#[test]
fn test_all_config_permutations() {
    let gen_configs = GenerationConfig::get_all_configs();
    let map_configs = MapConfig::get_all_configs();
    let thm_config = ThemeConfig::default();

    for gen_config in gen_configs.iter() {
        for map_config in map_configs.iter() {
            let config_name = format!("{}_{}", gen_config.name, map_config.name);

            let mut map = None;
            let mut seed_used = TEST_SEED;
            for retry in 0..MAX_RETRIES {
                let seed = Seed::from_u64(TEST_SEED + retry);

                if let Ok(generated_map) = Generator::generate_map(
                    MAX_STEPS,
                    &seed,
                    &gen_config,
                    &map_config,
                    &thm_config,
                    false, // no export preprocessing for tests
                ) {
                    map = Some(generated_map);
                    seed_used = TEST_SEED + retry;
                    break;
                }
            }

            let map = map.expect(&format!(
                "Failed to generate map for {} after {} retries",
                config_name, MAX_RETRIES
            ));

            let snapshot = format!("seed: {}\n\n{}", seed_used, grid_to_string(&map.grid));

            insta::assert_snapshot!(config_name, snapshot);
        }
    }
}
