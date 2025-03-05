use crate::position::ShiftDirection;
use crate::{config::GenerationConfig, editor::SeedType};
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

    pub fn max_value(&self) -> Option<&T>
    where
        T: Ord,
    {
        self.values.as_ref()?.iter().max()
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

pub struct Random {
    pub seed: Seed,
    gen: SmallRng,
    shift_dist: RandomDist<ShiftDirection>,
    inner_kernel_size_dist: RandomDist<usize>,
    outer_kernel_margin_dist: RandomDist<usize>,
    circ_dist: RandomDist<f32>,
}

/// u64 seed wrapper with various conversion methods
#[derive(Debug, Clone)]
pub struct Seed {
    pub seed_u64: u64,
}

impl Seed {
    pub fn from_u64(seed_u64: u64) -> Seed {
        Seed { seed_u64 }
    }

    pub fn from_random(rnd: &mut Random) -> Seed {
        Seed::from_u64(rnd.get_u64())
    }

    pub fn from_base64(base64_str: &String) -> Option<Seed> {
        URL_SAFE
            .decode(base64_str)
            .ok()
            .filter(|bytes| bytes.len() == 8)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u64::from_be_bytes)
            .map(Seed::from_u64)
    }

    pub fn to_base64(&self) -> String {
        URL_SAFE.encode(self.seed_u64.to_be_bytes())
    }

    pub fn random() -> Seed {
        Seed::from_u64(Random::get_u64_from_entropy())
    }

    pub fn from_string(seed_str: &String, seed_type: &SeedType) -> Option<Seed> {
        match seed_type {
            // hash string to u64
            SeedType::STRING => Some(Self::from_u64(hash(seed_str.as_bytes()))),

            // parse string to u64
            SeedType::U64 => seed_str
                .parse::<u64>()
                .ok()
                .map(Self::from_u64),

            SeedType::BASE64 => Self::from_base64(seed_str),
        }
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
    pub fn get_u64_from_entropy() -> u64 {
        let mut tmp_rng = SmallRng::from_entropy();
        tmp_rng.next_u64()
    }

    pub fn get_usize_in_range(&mut self, low: usize, high: usize) -> usize {
        assert!(high >= low, "no valid range");
        let n = (high - low) + 1;
        let rnd_value = self.gen.next_u64() as usize;

        low + (rnd_value % n)
    }

    pub fn get_f32_in_range(&mut self, low: f32, high: f32) -> f32 {
        assert!(high >= low, "no valid range");
        let ratio = self.get_unit_ratio();
        low + (high - low) * ratio
    }

    pub fn get_u64(&mut self) -> u64 {
        self.gen.next_u64()
    }

    pub fn get_u32(&mut self) -> u32 {
        self.gen.next_u32()
    }

    pub fn get_bool_with_prob(&mut self, probability: f32) -> bool {
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

    /// uniformly pick one element from a given slice
    pub fn pick_from_slice<'a, T>(&'a mut self, values: &'a [T]) -> &'a T {
        &values[self.get_usize_in_range(0, values.len() - 1)]
    }

    /// generate a f32 in range [0, 1]
    pub fn get_unit_ratio(&mut self) -> f32 {
        self.gen.next_u64() as f32 / u64::MAX as f32
    }

    /// generate valid "sub" bounds inside provided bounds
    pub fn get_bounds(&mut self, min: usize, max: usize) -> (usize, usize) {
        let bound1 = self.get_usize_in_range(min, max);
        let bound2 = self.get_usize_in_range(min, max);
        if bound1 <= bound2 {
            (bound1, bound2)
        } else {
            (bound2, bound1)
        }
    }

    pub fn get_random_usize_dist_config(
        &mut self,
        max_elements: usize,
        value_bounds: Option<(usize, usize)>,
    ) -> RandomDistConfig<usize> {
        let element_count = self.get_usize_in_range(1, max_elements);

        let values = value_bounds.map(|value_bounds| {
            (0..element_count)
                .map(|_| self.get_usize_in_range(value_bounds.0, value_bounds.1))
                .collect()
        });

        let probs = (0..element_count).map(|_| self.get_unit_ratio()).collect();

        let mut random_dist_config = RandomDistConfig::new(values, probs);
        random_dist_config.normalize_probs();
        random_dist_config
    }
}
