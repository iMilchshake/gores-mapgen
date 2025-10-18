use std::fmt;

use crate::automapper::{Automapper, Chance};
use crate::convert::TryTo;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum CheckError {
    ConfigNameLength {
        index: u32,
        name: String,
    },
    InvalidRandom {
        config_index: u32,
        run_index: u32,
        rule_index: u32,
        value: f32,
    },
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::ConfigNameLength {
                index,
                name,
            } => write!(f, "Config index {}: Too long name ({}), it must be shorter than 128 bytes", index, name),
            CheckError::InvalidRandom {
                config_index,
                run_index,
                rule_index,
                value,
            } => write!(f, "Config index {}, Run index {}, Rule index {}: {:?} is invalid, out-of-n must be larger than 1, percentages must be inbetween 0 and 100", config_index, run_index, rule_index, value)
        }
    }
}

pub const MAX_CONFIG_NAME_LENGTH: usize = 127;

impl Automapper {
    pub fn check(&self) -> Result<(), CheckError> {
        for (config_i, config) in self.configs.iter().enumerate() {
            if config.name.len() > MAX_CONFIG_NAME_LENGTH {
                return Err(CheckError::ConfigNameLength {
                    index: config_i.try_to(),
                    name: config.name.clone(),
                });
            }
            for (run_i, run) in config.runs.iter().enumerate() {
                for (rule_i, rule) in run.rules.iter().enumerate() {
                    match rule.chance {
                        Chance::Always => {}
                        Chance::OneOutOf(value) => {
                            if value.is_nan() || value.is_infinite() || value <= 1. {
                                return Err(CheckError::InvalidRandom {
                                    config_index: config_i.try_to(),
                                    run_index: run_i.try_to(),
                                    rule_index: rule_i.try_to(),
                                    value,
                                });
                            }
                        }
                        Chance::Percentage(value) => {
                            if value.is_nan() || value <= 0. || value >= 100. {
                                return Err(CheckError::InvalidRandom {
                                    config_index: config_i.try_to(),
                                    run_index: run_i.try_to(),
                                    rule_index: rule_i.try_to(),
                                    value,
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
