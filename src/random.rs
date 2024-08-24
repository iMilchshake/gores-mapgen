use crate::config::GenerationConfig;
use crate::position::ShiftDirection;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::WeightedAliasIndex;
use seahash::hash;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RandomDistConfig<T> {
    pub values: Option<Vec<T>>,
    pub probs: Vec<f32>,
}

impl<T> RandomDistConfig<T> {
    pub fn new(values: Option<Vec<T>>, probs: Vec<f32>) -> RandomDistConfig<T> {
        RandomDistConfig { values, probs }
    }

    pub fn normalize_probs(&mut self) {
        let probs_sum: f32 = self.probs.iter().sum();

        if probs_sum == 1.0 {
            return; // skip if already normalized
        }

        // if all values are zero, set all to 1/n
        if probs_sum == 0.0 {
            let len = self.probs.len();
            for val in self.probs.iter_mut() {
                *val = 1.0 / len as f32;
            }
        // otherwise normalize, if required
        } else if probs_sum != 1.0 {
            for val in self.probs.iter_mut() {
                *val /= probs_sum; // Normalize the vector
            }
        }
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
}

/// PRNG wrapper.
pub struct Random {
    pub seed: Seed,
    gen: SmallRng,
    shift_dist: RandomDist<ShiftDirection>,
    inner_kernel_size_dist: RandomDist<usize>,
    outer_kernel_margin_dist: RandomDist<usize>,
    circ_dist: RandomDist<f32>,
}

#[derive(Debug, Clone)]
pub struct Seed {
    pub seed_u64: u64,
    pub seed_str: String, // TODO: yeah okay no idea why i did this lol
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

    pub fn from_base64(base64_str: String) -> Seed {
        let decoded_bytes = URL_SAFE
            .decode(base64_str)
            .expect("couldn't decode given base64 string");
        assert_eq!(decoded_bytes.len(), 8, "exactly 8 bytes expected");
        let seed_u64: u64 = u64::from_be_bytes(decoded_bytes.try_into().unwrap());
        Seed::from_u64(seed_u64)
    }

    pub fn to_base64(&self) -> String {
        URL_SAFE.encode(self.seed_u64.to_be_bytes())
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

    pub fn sample_inner_kernel_size(&mut self) -> usize {
        let dist = &self.inner_kernel_size_dist;
        let index = dist.rnd_dist.sample(&mut self.gen);
        *dist.rnd_cfg.values.as_ref().unwrap().get(index).unwrap()
    }

    pub fn sample_outer_kernel_margin(&mut self) -> usize {
        let dist = &self.outer_kernel_margin_dist;
        let index = dist.rnd_dist.sample(&mut self.gen);
        *dist.rnd_cfg.values.as_ref().unwrap().get(index).unwrap()
    }

    pub fn sample_circularity(&mut self) -> f32 {
        let dist = &self.circ_dist;
        let index = dist.rnd_dist.sample(&mut self.gen);
        *dist.rnd_cfg.values.as_ref().unwrap().get(index).unwrap()
    }

    pub fn sample_shift(&mut self, ordered_shifts: &[ShiftDirection; 4]) -> ShiftDirection {
        let dist = &self.shift_dist;
        let index = dist.rnd_dist.sample(&mut self.gen);
        *ordered_shifts.get(index).unwrap()
    }

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
            (self.gen.next_u64() as f32) < (u64::MAX as f32 * probability)
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

    pub fn random_fraction(&mut self) -> f32 {
        self.gen.next_u64() as f32 / u64::MAX as f32
    }
}
