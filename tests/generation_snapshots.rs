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
const MAX_DIFF_LINES: usize = 10;

fn block_to_digit(block: &BlockType) -> char {
    match block {
        BlockType::Empty => '0',
        BlockType::EmptyFade => '1',
        BlockType::EmptyRoom => '2',
        BlockType::EmptyPlatform => '3',
        BlockType::Hookable => '4',
        BlockType::Platform => '5',
        BlockType::Freeze => '6',
        BlockType::Spawn => '7',
        BlockType::Start => '8',
        BlockType::Finish => '9',
    }
}

fn digit_to_block(digit: char) -> BlockType {
    match digit {
        '0' => BlockType::Empty,
        '1' => BlockType::EmptyFade,
        '2' => BlockType::EmptyRoom,
        '3' => BlockType::EmptyPlatform,
        '4' => BlockType::Hookable,
        '5' => BlockType::Platform,
        '6' => BlockType::Freeze,
        '7' => BlockType::Spawn,
        '8' => BlockType::Start,
        '9' => BlockType::Finish,
        _ => panic!("Invalid digit in snapshot: {}", digit),
    }
}

fn grid_to_compact_rle(grid: &Array2<BlockType>) -> String {
    let width = grid.shape()[0];
    let height = grid.shape()[1];

    let mut result = String::new();
    result.push_str(&format!("{}x{}:\n", width, height));

    for y in 0..height {
        let mut row_encoded = String::new();
        let mut current_digit = block_to_digit(&grid[(0, y)]);
        let mut count = 1;

        for x in 1..width {
            let digit = block_to_digit(&grid[(x, y)]);
            if digit == current_digit {
                count += 1;
            } else {
                if !row_encoded.is_empty() {
                    row_encoded.push('|');
                }
                row_encoded.push_str(&format!("{},{}", count, current_digit));
                current_digit = digit;
                count = 1;
            }
        }

        // Write final run
        if !row_encoded.is_empty() {
            row_encoded.push('|');
        }
        row_encoded.push_str(&format!("{},{}", count, current_digit));

        result.push_str(&row_encoded);
        result.push('\n');
    }

    result
}

fn compact_rle_to_grid(compact: &str) -> Array2<BlockType> {
    let lines: Vec<&str> = compact.lines().collect();
    assert!(!lines.is_empty(), "Empty compact RLE data");

    // Parse header: "200x700:"
    let header = lines[0].trim_end_matches(':');
    let dims: Vec<&str> = header.split('x').collect();
    assert_eq!(dims.len(), 2, "Invalid dimensions format");

    let width: usize = dims[0].parse().expect("Invalid width");
    let height: usize = dims[1].parse().expect("Invalid height");

    assert_eq!(
        lines.len() - 1,
        height,
        "Number of rows doesn't match height"
    );

    let mut grid = Array2::from_elem((width, height), BlockType::Empty);

    for (y, line) in lines.iter().skip(1).enumerate() {
        let runs: Vec<&str> = line.split('|').collect();
        let mut x = 0;

        for run in runs {
            let parts: Vec<&str> = run.split(',').collect();
            assert_eq!(parts.len(), 2, "Invalid run format: {}", run);

            let count: usize = parts[0].parse().expect("Invalid count");
            let digit = parts[1].chars().next().expect("Empty digit");
            let block = digit_to_block(digit);

            for _ in 0..count {
                assert!(x < width, "Row {} exceeds width", y);
                grid[(x, y)] = block.clone();
                x += 1;
            }
        }

        assert_eq!(x, width, "Row {} has incorrect length: {} vs {}", y, x, width);
    }

    grid
}

fn compare_grids(expected: &str, actual: &Array2<BlockType>) -> Result<(), String> {
    let expected_grid = compact_rle_to_grid(expected);

    let width = expected_grid.shape()[0];
    let height = expected_grid.shape()[1];

    if actual.shape()[0] != width || actual.shape()[1] != height {
        return Err(format!(
            "Grid dimensions mismatch: expected {}x{}, got {}x{}",
            width,
            height,
            actual.shape()[0],
            actual.shape()[1]
        ));
    }

    let mut differences = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let expected_block = &expected_grid[(x, y)];
            let actual_block = &actual[(x, y)];

            if expected_block != actual_block {
                differences.push(format!(
                    "  x={}, y={}: expected={:?}, actual={:?}",
                    x, y, expected_block, actual_block
                ));

                if differences.len() >= MAX_DIFF_LINES {
                    break;
                }
            }
        }
        if differences.len() >= MAX_DIFF_LINES {
            break;
        }
    }

    if differences.is_empty() {
        Ok(())
    } else {
        let diff_summary = if differences.len() >= MAX_DIFF_LINES {
            format!(
                "Grid mismatch (showing first {} differences):\n{}",
                MAX_DIFF_LINES,
                differences.join("\n")
            )
        } else {
            format!(
                "Grid mismatch ({} differences):\n{}",
                differences.len(),
                differences.join("\n")
            )
        };
        Err(diff_summary)
    }
}

#[test]
fn test_all_config_permutations() {
    let gen_configs = GenerationConfig::get_all_configs();
    let map_configs = MapConfig::get_all_configs();
    let thm_config = ThemeConfig::default();

    let mut all_diffs = Vec::new();

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
                    false, // no prepare export for tests
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

            let compact_grid = grid_to_compact_rle(&map.grid);
            let snapshot = format!("seed: {}\n{}", seed_used, compact_grid);

            // Try to compare with existing snapshot for detailed diff
            let snapshot_path = format!(
                "tests/snapshots/generation_snapshots__{}.snap",
                config_name
            );
            if let Ok(existing_content) = std::fs::read_to_string(&snapshot_path) {
                // Extract grid data (skip YAML header lines)
                let lines: Vec<&str> = existing_content.lines().collect();
                if let Some(seed_line_idx) = lines.iter().position(|l| l.starts_with("seed:")) {
                    // Grid data starts after "seed: XX" line
                    let existing_grid = lines[seed_line_idx + 1..].join("\n");

                    // Compare grids and collect diffs instead of panicking immediately
                    if let Err(diff_msg) = compare_grids(&existing_grid, &map.grid) {
                        all_diffs.push(format!("'{}': {}", config_name, diff_msg));
                    }
                }
            }

            insta::assert_snapshot!(config_name, snapshot);
        }
    }

    // Report all diffs at the end if any were found
    if !all_diffs.is_empty() {
        panic!(
            "\n{} snapshot(s) had mismatches:\n\n{}",
            all_diffs.len(),
            all_diffs.join("\n\n")
        );
    }
}
