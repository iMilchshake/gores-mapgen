use crate::Tile;
use vek::Vec2;

mod checks;
mod execute;
mod parse;

pub use checks::{CheckError, MAX_CONFIG_NAME_LENGTH};
pub use execute::ConfigOutOfBounds;
pub use parse::SyntaxError;

#[derive(Debug, Clone, PartialEq)]
pub struct Automapper {
    /// Should be the same as the dedicated mapres
    pub name: String,
    /// Different configs to choose from (e.g. Grass, Cave)
    pub configs: Vec<Config>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub name: String,
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub layer_copy: bool,
    pub rules: Vec<IndexRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexRule {
    pub tile: Tile,
    pub default_rule: bool,
    pub chance: Chance,
    pub conditions: Vec<Rule>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Chance {
    Always,
    OneOutOf(f32),
    Percentage(f32),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Rule {
    pub offset: Vec2<i32>,
    pub condition: Condition,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Condition {
    Empty,
    Full,
    WhiteList(Vec<TileCondition>),
    BlackList(Vec<TileCondition>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TileCondition {
    /// Outside of the tilemap
    Outside,
    /// Tile with specified index, but any tile-flags
    Index(u8),
    Tile(Tile),
}
