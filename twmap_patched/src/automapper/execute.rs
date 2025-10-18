use ndarray::Array2;
use vek::Vec2;

use crate::automapper::{Automapper, Chance, Condition, Config, Rule, TileCondition};
use crate::convert::TryTo;
use crate::{AutomapperConfig, Tile, TilesLayer};

struct DoubleView<'tiles> {
    tiles: &'tiles mut Array2<Tile>,
    copy: Option<Array2<Tile>>,
}

impl DoubleView<'_> {
    fn view(&self) -> &Array2<Tile> {
        match &self.copy {
            Some(copy) => copy,
            None => self.tiles,
        }
    }
}

impl TileCondition {
    fn applies(&self, tile: Option<Tile>) -> bool {
        match self {
            TileCondition::Outside => tile.is_none(),
            TileCondition::Index(id) => tile.map(|t| t.id) == Some(*id),
            TileCondition::Tile(t) => tile == Some(*t),
        }
    }
}

impl Rule {
    fn applies(&self, tiles: &Array2<Tile>, position: Vec2<usize>) -> bool {
        let position_i64: Vec2<i64> = position.numcast().unwrap();
        let offset_i64: Vec2<i64> = self.offset.numcast().unwrap();
        let index = position_i64 + offset_i64;
        let tile = if index.x < 0 || index.y < 0 {
            None
        } else {
            let index: Vec2<usize> = index.numcast().unwrap();
            tiles.get((index.y, index.x)).copied()
        };

        let id = tile.map(|t| t.id);
        match &self.condition {
            Condition::Empty => id == Some(0),
            Condition::Full => id != Some(0),
            Condition::WhiteList(list) => list.iter().any(|cond| cond.applies(tile)),
            Condition::BlackList(list) => !list.iter().any(|cond| cond.applies(tile)),
        }
    }
}

const DEFAULT_RULE: Rule = Rule {
    offset: Vec2::new(0, 0),
    condition: Condition::Full,
};

fn hash_u32(mut n: u32) -> u32 {
    n = n.wrapping_add(1);
    n ^= n >> 17;
    n = n.wrapping_mul(0xed5ad4bb);
    n ^= n >> 11;
    n = n.wrapping_mul(0xac4c1b51);
    n ^= n >> 15;
    n = n.wrapping_mul(0x31848bab);
    n ^= n >> 14;
    n
}

const HASH_MAX: u32 = 2_u32.pow(16);
const PRIME: u32 = 31;

fn hash_location(seed: u32, run: u32, rule: u32, x: u32, y: u32) -> u32 {
    let mut hash: u32 = 1;

    for n in [seed, run, rule, x, y] {
        hash = hash.wrapping_mul(PRIME).wrapping_add(hash_u32(n));
    }
    hash = hash_u32(hash.wrapping_mul(PRIME));
    hash % HASH_MAX
}

fn random_u32() -> u32 {
    let mut u32_bytes: [u8; 4] = [0; 4];
    getrandom::getrandom(&mut u32_bytes).unwrap();
    u32::from_le_bytes(u32_bytes)
}

impl Chance {
    fn chance(&self) -> Option<f32> {
        match self {
            Chance::Always => None,
            Chance::OneOutOf(n) => Some(n.recip()),
            Chance::Percentage(p) => Some(p / 100.),
        }
    }

    fn u32_chance(&self) -> Option<u32> {
        self.chance().map(|f| (f * HASH_MAX as f32) as u32)
    }
}

impl Config {
    pub fn run(&self, mut seed: u32, tiles: &mut Array2<Tile>) {
        seed = match seed {
            0 => random_u32(),
            _ => seed,
        };
        let height = tiles.shape()[0];
        let width = tiles.shape()[1];
        for (run_i, run) in self.runs.iter().enumerate() {
            let copy = run.layer_copy.then(|| tiles.clone());
            let tilemap = DoubleView { tiles, copy };
            for (rule_i, rule) in run.rules.iter().enumerate() {
                let chance = rule.chance.u32_chance();
                for y in 0..height {
                    for x in 0..width {
                        let position = Vec2::new(x, y);
                        let default =
                            !rule.default_rule || DEFAULT_RULE.applies(tilemap.view(), position);
                        if default
                            && rule
                                .conditions
                                .iter()
                                .all(|cond| cond.applies(tilemap.view(), position))
                        {
                            if let Some(chance) = chance {
                                if hash_location(
                                    seed,
                                    run_i.try_to(),
                                    rule_i.try_to(),
                                    x.try_to(),
                                    y.try_to(),
                                ) >= chance
                                {
                                    continue;
                                }
                            }
                            tilemap.tiles[(y, x)] = rule.tile;
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ConfigOutOfBounds;

impl Automapper {
    pub fn run(
        &self,
        automapper_config: &AutomapperConfig,
        tiles: &mut Array2<Tile>,
    ) -> Result<(), ConfigOutOfBounds> {
        let index = match automapper_config.config {
            None => return Ok(()),
            Some(i) => i,
        };
        let config = match self.configs.get(index.try_to::<usize>()) {
            None => return Err(ConfigOutOfBounds),
            Some(c) => c,
        };
        config.run(automapper_config.seed, tiles);
        Ok(())
    }
}

impl TilesLayer {
    /// Requires tiles to be loaded
    pub fn run_automapper(&mut self, automapper: &Automapper) -> Result<(), ConfigOutOfBounds> {
        automapper.run(&self.automapper_config, self.tiles.unwrap_mut())
    }
}
