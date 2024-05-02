use crate::config::GenerationConfig;
use crate::position::ShiftDirection;
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::WeightedAliasIndex;
use seahash::hash;

pub struct Random {
    pub seed: Seed,
    gen: SmallRng,
    shift_dist: WeightedAliasIndex<i32>,
    inner_kernel_dist: WeightedAliasIndex<f32>,
    outer_kernel_dist: WeightedAliasIndex<f32>,
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
            shift_dist: Random::get_weighted_dist(config.shift_weights.clone()),
            inner_kernel_dist: WeightedAliasIndex::new(
                config
                    .inner_size_probs
                    .iter()
                    .map(|(_, prob)| *prob)
                    .collect(),
            )
            .unwrap(),
            outer_kernel_dist: WeightedAliasIndex::new(
                config
                    .outer_margin_probs
                    .iter()
                    .map(|(_, prob)| *prob)
                    .collect(),
            )
            .unwrap(),
        }
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

    fn get_weighted_dist(weights: Vec<i32>) -> WeightedAliasIndex<i32> {
        // sadly WeightedAliasIndex is initialized using a Vec. So im manually checking for the
        // correct size. I feel like there must be a better way also the current apprach allows
        // for invalid moves to be picked. But that should be no problem in pracise
        assert_eq!(weights.len(), 4);
        WeightedAliasIndex::new(weights).expect("expect valid weights")
    }

    /// sample a shift based on weight distribution
    pub fn sample_move(&mut self, shifts: &[ShiftDirection; 4]) -> ShiftDirection {
        let index = self.shift_dist.sample(&mut self.gen);
        let shift = shifts.get(index).expect("out of bounds");

        shift.clone()
    }

    pub fn sample_inner_kernel_size(&mut self, kernel_size_probs: &[(usize, f32)]) -> usize {
        let index = self.inner_kernel_dist.sample(&mut self.gen);
        let inner_kernel_size = kernel_size_probs.get(index).expect("out of bounds");

        return inner_kernel_size.0;
    }

    pub fn sample_outer_kernel_margin(&mut self, kernel_margin_probs: &[(usize, f32)]) -> usize {
        let index = self.outer_kernel_dist.sample(&mut self.gen);
        let inner_kernel_size = kernel_margin_probs.get(index).expect("out of bounds");

        return inner_kernel_size.0;
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
