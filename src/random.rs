use crate::position::ShiftDirection;
use crate::{config::GenerationConfig, generator::Generator};
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::{weighted_alias::AliasableWeight, WeightedAliasIndex};
use seahash::hash;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RandomDistConfig<T> {
    pub values: Vec<T>, // TODO: option here?
    pub probs: Vec<f32>,
}

impl<T> RandomDistConfig<T> {
    pub fn new(values: Vec<T>, probs: Vec<f32>) -> RandomDistConfig<T> {
        RandomDistConfig { values, probs }
    }
}

pub struct RandomDist<T> {
    rnd_cfg: RandomDistConfig<T>,
    rnd_dist: WeightedAliasIndex<f32>,
}

pub enum RandomDistType {
    InnerSize,
    OuterMargin,
    Circularity,
    ShiftDirection,
}

impl<T: Clone> RandomDist<T> {
    pub fn new(config: RandomDistConfig<T>) -> RandomDist<T> {
        RandomDist {
            rnd_dist: WeightedAliasIndex::new(config.probs.clone()).unwrap(),
            rnd_cfg: config,
        }
    }

    pub fn sample(&mut self, gen: &mut SmallRng) -> T {
        let index = self.rnd_dist.sample(gen);

        self.rnd_cfg
            .values
            .get(index)
            .expect("out of bounds")
            .clone()
    }
}

pub struct Random {
    pub seed: Seed,
    pub gen: SmallRng,
    shift_dist: RandomDist<ShiftDirection>,
    pub inner_kernel_size_dist: RandomDist<usize>,
    outer_kernel_margin_dist: RandomDist<usize>,
    circ_dist: RandomDist<f32>,
}

#[derive(Debug, Clone)]
pub struct Seed {
    pub seed_u64: u64,
    pub seed_str: String,
}

impl Seed {
    pub fn from_u64(seed_u64: u64) -> Seed {
        Seed {
            seed_u64,
            seed_str: String::new(),
        }
    }

    pub fn from_string(seed_str: &String) -> Seed {
        Seed {
            seed_u64: Seed::str_to_u64(seed_str),
            seed_str: seed_str.to_owned(),
        }
    }

    pub fn from_random(rnd: &mut Random) -> Seed {
        Seed::from_u64(rnd.random_u64())
    }

    pub fn random() -> Seed {
        Seed::from_u64(Random::get_random_u64())
    }

    pub fn str_to_u64(seed_str: &String) -> u64 {
        hash(seed_str.as_bytes())
    }
}

impl Random {
    pub fn new(seed: Seed, config: &GenerationConfig) -> Random {
        Random {
            gen: SmallRng::seed_from_u64(seed.seed_u64),
            seed,
            shift_dist: RandomDist::new(config.shift_weights.clone()),
            outer_kernel_margin_dist: RandomDist::new(config.outer_margin_probs.clone()),
            inner_kernel_size_dist: RandomDist::new(config.inner_size_probs.clone()),
            circ_dist: RandomDist::new(config.circ_probs.clone()),
            // TODO: clones here fine?
        }
    }

    // pub fn sample_dist_values<T: Clone>(&mut self, values: &[T], dist: &RandomDist<T>) -> T {
    //     let index = dist.rnd_dist.sample(&mut self.gen);
    //     dist.rnd_cfg
    //         .values
    //         .get(index)
    //         .expect("out of bounds")
    //         .clone()
    // }

    /// derive a u64 seed from entropy
    pub fn get_random_u64() -> u64 {
        let mut tmp_rng = SmallRng::from_entropy();
        tmp_rng.next_u64()
    }

    pub fn in_range_inclusive(&mut self, low: usize, high: usize) -> usize {
        assert!(high >= low, "no valid range");
        let n = (high - low) + 1;
        let rnd_value = self.gen.next_u64() as usize;

        low + (rnd_value % n)
    }

    pub fn in_range_exclusive(&mut self, low: usize, high: usize) -> usize {
        assert!(high > low, "no valid range");
        let n = high - low;
        let rnd_value = self.gen.next_u64() as usize;

        low + (rnd_value % n)
    }

    pub fn random_u64(&mut self) -> u64 {
        self.gen.next_u64()
    }

    pub fn with_probability(&mut self, probability: f32) -> bool {
        if probability == 1.0 {
            self.skip();
            true
        } else if probability == 0.0 {
            self.skip();
            false
        } else {
            (self.gen.next_u64() as f32) < (u64::max_value() as f32 * probability)
        }
    }

    /// skip one gen step to ensure that a value is consumed in any case
    pub fn skip(&mut self) {
        self.gen.next_u64();
    }

    /// skip n gen steps to ensure that n values are consumed in any case
    pub fn skip_n(&mut self, n: usize) {
        for _ in 0..n {
            self.gen.next_u64();
        }
    }

    pub fn pick_element<'a, T>(&'a mut self, values: &'a [T]) -> &T {
        &values[self.in_range_exclusive(0, values.len())]
    }

    pub fn random_circularity(&mut self) -> f32 {
        self.gen.next_u64() as f32 / u64::max_value() as f32
    }
}
