#![allow(clippy::upper_case_acronyms)]

use crate::automapper::{
    Automapper, Chance, Condition, Config, IndexRule, Rule, Run, TileCondition,
};
use crate::convert::TryTo;
use crate::{Tile, TileFlags};
use std::fmt;
use std::str;
use std::str::FromStr;
use thiserror::Error;
use vek::Vec2;

// Considerations:
// - Automapper with zero configs
// - Config with zero rules

struct Lines<'file> {
    lines: str::Lines<'file>,
    peeked_line: Option<(u32, &'file str)>,
    line_number: u32,
}

struct Tokens<'line> {
    line: &'line str,
    line_number: u32,
    split: str::Split<'line, &'static [char]>,
    next_index: u32,
    current: Option<(u32, &'line str)>,
}

impl<'file> Lines<'file> {
    fn new(file: &'file str) -> Self {
        Self {
            lines: file.lines(),
            peeked_line: None,
            line_number: 1,
        }
    }

    fn peek(&mut self) -> Option<Tokens<'file>> {
        if let Some(line) = self.peeked_line {
            return Some(Tokens::new(line));
        }
        loop {
            match self.lines.next() {
                None => break None,
                Some(line) => {
                    self.line_number += 1;
                    // Skip empty and comment lines
                    if line.is_empty()
                        || line.strip_prefix('#').is_some()
                        || line.chars().all(|c| SPLIT_CHARS.contains(&c))
                    {
                        continue;
                    } else {
                        self.peeked_line = Some((self.line_number, line));
                        break Some(Tokens::new(self.peeked_line.unwrap()));
                    }
                }
            }
        }
    }

    fn next(&mut self) -> Option<Tokens<'file>> {
        let next = self.peek();
        self.peeked_line = None;
        next
    }

    fn seek_peek(
        &mut self,
        cancel: &[Pattern],
        seek: &[Pattern],
    ) -> Result<Option<(Tokens<'file>, Pattern)>, SyntaxError> {
        match self.peek() {
            None => Ok(None),
            Some(mut tokens) => {
                if cancel.iter().any(|pattern| pattern.matches(tokens.line)) {
                    return Ok(None);
                }
                for pattern in seek {
                    if pattern.matches(tokens.line) {
                        return Ok(Some((tokens, *pattern)));
                    }
                }
                let expected: Vec<_> = seek
                    .iter()
                    .chain(cancel.iter())
                    .map(|p| p.inner())
                    .collect();
                tokens.next().unwrap();
                Err(tokens.error(&expected))
            }
        }
    }

    /// Like seek_peek, but consumes the next element if a 'seek' is encountered
    fn consume_next(
        &mut self,
        cancel: &[Pattern],
        seek: &[Pattern],
    ) -> Result<Option<(Tokens<'file>, Pattern)>, SyntaxError> {
        Ok(match self.seek_peek(cancel, seek)? {
            None => None,
            Some(next) => {
                self.next();
                Some(next)
            }
        })
    }
}

const EOL: &str = "\n";
const SPLIT_CHARS: [char; 2] = [' ', '\t'];

impl<'line> Tokens<'line> {
    fn new(line: (u32, &'line str)) -> Self {
        Self {
            line: line.1,
            line_number: line.0,
            split: line.1.split(&SPLIT_CHARS),
            next_index: 0,
            current: None,
        }
    }

    fn next(&mut self) -> Option<&str> {
        loop {
            match self.split.next() {
                None => break None,
                Some(token) => {
                    self.current = Some((self.next_index, token));

                    self.next_index += token.len().try_to::<u32>();
                    self.next_index += 1; // Space separator
                    if token.strip_prefix('#').is_some() {
                        // Consume rest of iterator
                        for _ in self.split.by_ref() {}
                        break None;
                    }
                    match token.is_empty() {
                        true => continue, // Skip multiple spaces
                        false => break Some(token),
                    }
                }
            }
        }
    }

    fn error(&self, expected: &[&'static str]) -> SyntaxError {
        let (character, unexpected) = match self.current {
            Some(token) => token,
            None => (self.line.len().try_to(), EOL),
        };
        SyntaxError {
            line: self.line_number,
            character,
            expected: expected.to_vec(),
            unexpected: unexpected.to_owned(),
        }
    }

    fn line_error(&self, character: u32, expected: &[&'static str]) -> SyntaxError {
        SyntaxError {
            line: self.line_number,
            character,
            expected: expected.to_vec(),
            unexpected: self.line.to_owned(),
        }
    }

    fn expect_eol(&mut self) -> Result<(), SyntaxError> {
        match self.next() {
            None => Ok(()),
            Some(_) => Err(self.error(&[EOL])),
        }
    }

    /// Patterns may only be Token patterns.
    /// The first token must already be checked via Pattern::matches
    fn single_token(mut self, pattern: Pattern) -> Result<(), SyntaxError> {
        let expected_token = match pattern {
            Pattern::Prefix(_) => unreachable!(),
            Pattern::Token(token) => token,
        };
        assert_eq!(self.next().unwrap(), expected_token);
        self.expect_eol()
    }

    fn expect_some<T: FromStr>(&mut self, expected: &[&'static str]) -> Result<T, SyntaxError> {
        let token = match self.next() {
            None => return Err(self.error(expected)),
            Some(t) => t,
        };
        T::from_str(token).map_err(|_| self.error(expected))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Pattern {
    Prefix(&'static str),
    Token(&'static str),
}

const NEW_CONFIG: Pattern = Pattern::Prefix("[");
const NEW_RUN: Pattern = Pattern::Token("NewRun");
const NEW_NO_COPY: Pattern = Pattern::Token("NoLayerCopy");
const NEW_RULE: Pattern = Pattern::Token("Index");
const NEW_NO_DEFAULT: Pattern = Pattern::Token("NoDefaultRule");
const NEW_CONDITION: Pattern = Pattern::Token("Pos");
const NEW_RANDOM: Pattern = Pattern::Token("Random");

impl Pattern {
    fn inner(&self) -> &'static str {
        match self {
            Pattern::Prefix(p) => p,
            Pattern::Token(t) => t,
        }
    }

    fn matches(&self, line: &str) -> bool {
        match self {
            Pattern::Prefix(p) => line.strip_prefix(p).is_some(),
            Pattern::Token(t) => line.split(&SPLIT_CHARS).next().unwrap() == *t,
        }
    }
}

impl Automapper {
    pub fn parse(name: String, file: &str) -> Result<Self, SyntaxError> {
        let mut configs = Vec::new();
        let mut lines = Lines::new(file);
        while lines.seek_peek(&[], &[NEW_CONFIG])?.is_some() {
            configs.push(Config::parse(&mut lines)?);
        }
        Ok(Self { name, configs })
    }
}

#[derive(Error, Debug, Clone)]
pub struct SyntaxError {
    line: u32,
    character: u32,
    expected: Vec<&'static str>,
    unexpected: String,
}

fn token_str(token: &str) -> &str {
    match token {
        EOL => "End of Line",
        other => other,
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Syntax error in line {}, starting at character {}: ",
            self.line, self.character
        )?;
        match self.expected.len() {
            0 => {}
            1 => write!(f, "Expected \"{}. \"", self.expected[0])?,
            n => {
                write!(f, "Expected either ")?;
                for i in 0..n - 1 {
                    write!(f, "\"{}\", ", token_str(self.expected[i]))?;
                }
                write!(f, "or \"{}\". ", self.expected.last().unwrap())?;
            }
        };
        write!(f, "Unexpected input: \"{}\"", token_str(&self.unexpected))
    }
}

impl Config {
    /// Expects the line with the name immediately
    fn parse(lines: &mut Lines) -> Result<Self, SyntaxError> {
        let tokens = lines.next().unwrap();
        let line = tokens.line;
        let mut name = line.strip_prefix('[').unwrap(); // Guaranteed by prefix pattern
        name = name
            .strip_suffix(']')
            .ok_or_else(|| tokens.line_error(line.len().try_to(), &["]"]))?;
        let name = name.to_owned();
        let mut runs = Vec::new();
        while lines
            .seek_peek(&[NEW_CONFIG], &[NEW_RUN, NEW_NO_COPY, NEW_RULE])?
            .is_some()
        {
            runs.push(Run::parse(lines)?);
        }
        Ok(Self { name, runs })
    }
}

impl Run {
    /// Will consume the next 'NextRun' line
    fn parse(lines: &mut Lines) -> Result<Self, SyntaxError> {
        let mut rules = Vec::new();
        let mut layer_copy = true;
        let mut seek: &[_] = &[NEW_NO_COPY, NEW_RULE, NEW_RUN];
        while let Some((tokens, p)) = lines.seek_peek(&[NEW_CONFIG], seek)? {
            match p {
                NEW_NO_COPY => {
                    layer_copy = false;
                    tokens.single_token(NEW_NO_COPY)?;
                    seek = &[NEW_RULE, NEW_RUN];
                    lines.next().unwrap();
                }
                NEW_RULE => rules.push(IndexRule::parse(lines)?),
                NEW_RUN => {
                    tokens.single_token(NEW_RUN)?;
                    // Remove NewRun line
                    lines.next().unwrap();
                    break;
                }
                _ => unreachable!(),
            }
        }
        Ok(Self { layer_copy, rules })
    }
}

macro_rules! token {
    ( $token_name:ident, $( $variant:ident ),* ) => {
        enum $token_name {
            $(
                $variant,
            )*
        }

        impl $token_name {
            #[allow(dead_code)]
            const fn as_str(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant => stringify!($variant),
                    )*
                }
            }

            fn parse(token: &str) -> Result<Self, ()> {
                match token {
                    $(
                        stringify!($variant) => Ok(Self::$variant),
                    )*
                    _ => Err(()),
                }
            }

            fn next(tokens: &mut Tokens) -> Result<Self, SyntaxError> {
                match tokens.next() {
                    None => Err(tokens.error(&[$(stringify!($variant), )*])),
                    Some(token) => match Self::parse(token) {
                        Ok(parsed) => Ok(parsed),
                        Err(()) => Err(tokens.error(&[$(stringify!($variant), )*])),
                    }
                }
            }
        }
    };
}

macro_rules! eol_token {
    ( $token_name:ident, $( $variant:ident ),* ) => {
        enum $token_name {
            $(
                $variant,
            )*
        }

        impl $token_name {
            #[allow(dead_code)]
            const fn as_str(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant => stringify!($variant),
                    )*
                }
            }

            fn parse(token: Option<&str>) -> Result<Option<Self>, ()> {
                match token {
                    None => Ok(None),
                    $(
                        Some(stringify!($variant)) => Ok(Some(Self::$variant)),
                    )*
                    Some(_) => Err(()),
                }
            }

            fn next_maybe(tokens: &mut Tokens) -> Result<Option<Self>, SyntaxError> {
                Self::parse(tokens.next()).map_err(|_| tokens.error(&[$(stringify!($variant), )*]))
            }
        }
    };
}

eol_token!(TileFlagsToken, XFLIP, YFLIP, ROTATE);
token!(RulePrefixToken, Index);
token!(RandomPrefixToken, Random);

const TILE_ID_EXPECTED: &str = "Tile id (Number between 0 and 256)";
const NO_DUPLICATE_ORIENTATION_EXPECTED: &str = "No duplicate orientation";
const RANDOM_EXPECTED: [&str; 2] = ["Number greater than 1", "Percentage lower than 100%"];

impl FromStr for Chance {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.strip_suffix('%') {
            None => {
                let f = s.parse::<f32>().map_err(|_| ())?;
                if f.is_nan() || f <= 1. || f.is_infinite() {
                    Err(())
                } else {
                    Ok(Self::OneOutOf(f))
                }
            }
            Some(s) => {
                let f = s.parse::<f32>().map_err(|_| ())?;
                if f.is_nan() || f <= 0. || f >= 100. {
                    Err(())
                } else {
                    Ok(Self::Percentage(f))
                }
            }
        }
    }
}

impl IndexRule {
    fn parse(lines: &mut Lines) -> Result<Self, SyntaxError> {
        let mut tokens = lines.next().unwrap();
        RulePrefixToken::next(&mut tokens).unwrap();
        // Guaranteed until here by matching NEWRULE pattern
        let id = tokens.expect_some::<u8>(&[TILE_ID_EXPECTED])?;
        let mut flags = TileFlags::empty();
        while let Some(flag_token) = TileFlagsToken::next_maybe(&mut tokens)? {
            let flag = match flag_token {
                TileFlagsToken::XFLIP => TileFlags::FLIP_X,
                TileFlagsToken::YFLIP => TileFlags::FLIP_Y,
                TileFlagsToken::ROTATE => TileFlags::ROTATE,
            };
            if flags.contains(flag) {
                return Err(tokens.error(&[NO_DUPLICATE_ORIENTATION_EXPECTED]));
            } else {
                flags.insert(flag);
            }
        }
        let tile = Tile::new(id, flags);

        let mut conditions = Vec::new();
        let mut chance = Chance::Always;
        let mut default_rule = true;
        let mut seek: &[_] = &[NEW_NO_DEFAULT, NEW_CONDITION, NEW_RANDOM];
        while let Some((mut tokens, p)) =
            lines.consume_next(&[NEW_CONFIG, NEW_RUN, NEW_NO_COPY, NEW_RULE], seek)?
        {
            match p {
                NEW_NO_DEFAULT => {
                    default_rule = false;
                    tokens.single_token(NEW_NO_DEFAULT)?;
                    seek = &[NEW_CONDITION, NEW_RANDOM];
                }
                NEW_CONDITION => {
                    let rule = Rule::parse(tokens)?;
                    if rule.offset == Vec2::zero() {
                        default_rule = false;
                    }
                    conditions.push(rule);
                }
                NEW_RANDOM => {
                    RandomPrefixToken::next(&mut tokens).unwrap();
                    chance = tokens.expect_some::<Chance>(&RANDOM_EXPECTED)?;
                    tokens.expect_eol()?;
                }
                _ => unreachable!(),
            }
        }
        Ok(Self {
            tile,
            default_rule,
            chance,
            conditions,
        })
    }
}

token!(ConditionPrefixToken, Pos);
token!(ConditionVariantToken, FULL, EMPTY, INDEX, NOTINDEX);

const X_OFFSET_EXPECTED: &str = "x-offset of condition";
const Y_OFFSET_EXPECTED: &str = "y-offset of condition";

impl Rule {
    fn parse(mut tokens: Tokens) -> Result<Self, SyntaxError> {
        ConditionPrefixToken::next(&mut tokens).unwrap();
        let x = tokens.expect_some::<i32>(&[X_OFFSET_EXPECTED])?;
        let y = tokens.expect_some::<i32>(&[Y_OFFSET_EXPECTED])?;
        let condition = Condition::parse(&mut tokens)?;

        Ok(Self {
            offset: Vec2 { x, y },
            condition,
        })
    }
}

impl Condition {
    fn parse(tokens: &mut Tokens) -> Result<Self, SyntaxError> {
        let condition = match ConditionVariantToken::next(tokens)? {
            ConditionVariantToken::FULL => Condition::Full,
            ConditionVariantToken::EMPTY => Condition::Empty,
            ConditionVariantToken::INDEX => {
                Condition::WhiteList(TileCondition::parse_list(tokens)?)
            }
            ConditionVariantToken::NOTINDEX => {
                Condition::BlackList(TileCondition::parse_list(tokens)?)
            }
        };
        tokens.expect_eol()?;
        Ok(condition)
    }
}

eol_token!(TileConditionToken, NONE, XFLIP, YFLIP, ROTATE, OR);
eol_token!(AnotherTileConditionToken, OR);
eol_token!(OrTileFlagToken, XFLIP, YFLIP, ROTATE, OR);

const OUTSIDE_INDEX_EXPECTED: [&str; 2] = [
    "Tile id (Number between 0 and 256)",
    "-1 (outside of tilemap)",
];

struct OutsideIndex(Option<u8>);
impl FromStr for OutsideIndex {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "-1" => Ok(Self(None)),
            _ => match s.parse::<u8>() {
                Ok(n) => Ok(Self(Some(n))),
                Err(_) => Err(()),
            },
        }
    }
}

impl Tokens<'_> {
    fn another_tile_token(&mut self) -> Result<bool, SyntaxError> {
        Ok(match AnotherTileConditionToken::next_maybe(self)? {
            None => false,
            Some(AnotherTileConditionToken::OR) => true,
        })
    }
}

impl TileCondition {
    fn parse_list(tokens: &mut Tokens) -> Result<Vec<Self>, SyntaxError> {
        let mut another_one = true;
        let mut conditions = Vec::new();
        while another_one {
            let (condition, still_another_one) = Self::parse(tokens)?;
            conditions.push(condition);
            another_one = still_another_one;
        }
        Ok(conditions)
    }

    fn parse(tokens: &mut Tokens) -> Result<(Self, bool), SyntaxError> {
        let OutsideIndex(index) = tokens.expect_some::<OutsideIndex>(&OUTSIDE_INDEX_EXPECTED)?;
        Ok(match index {
            None => (Self::Outside, tokens.another_tile_token()?),
            Some(id) => match TileConditionToken::next_maybe(tokens)? {
                None => (Self::Index(id), false),
                Some(TileConditionToken::OR) => (Self::Index(id), true),
                Some(TileConditionToken::NONE) => (
                    Self::Tile(Tile::new(id, TileFlags::empty())),
                    tokens.another_tile_token()?,
                ),
                Some(orientation) => {
                    let mut flags = match orientation {
                        TileConditionToken::XFLIP => TileFlags::FLIP_X,
                        TileConditionToken::YFLIP => TileFlags::FLIP_Y,
                        TileConditionToken::ROTATE => TileFlags::ROTATE,
                        _ => unreachable!(),
                    };
                    loop {
                        let flag_token = match OrTileFlagToken::next_maybe(tokens)? {
                            None => break (Self::Tile(Tile::new(id, flags)), false),
                            Some(flag) => flag,
                        };
                        let flag = match flag_token {
                            OrTileFlagToken::OR => break (Self::Tile(Tile::new(id, flags)), true),
                            OrTileFlagToken::XFLIP => TileFlags::FLIP_X,
                            OrTileFlagToken::YFLIP => TileFlags::FLIP_Y,
                            OrTileFlagToken::ROTATE => TileFlags::ROTATE,
                        };
                        if flags.contains(flag) {
                            return Err(tokens.error(&[NO_DUPLICATE_ORIENTATION_EXPECTED]));
                        } else {
                            flags.insert(flag);
                        }
                    }
                }
            },
        })
    }
}
