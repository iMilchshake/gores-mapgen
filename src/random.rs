use crate::position::ShiftDirection;
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::WeightedAliasIndex;
use seahash::hash;

pub struct Random {
    pub seed_str: Option<String>,
    pub seed_hex: String,
    pub seed_u64: u64,
    gen: SmallRng,
    weighted_dist: WeightedAliasIndex<i32>,
}

impl Random {
    pub fn new(seed_u64: u64, weights: Vec<i32>) -> Random {
        Random {
            seed_str: None,
            seed_u64,
            seed_hex: format!("{:X}", seed_u64),
            gen: SmallRng::seed_from_u64(seed_u64),
            weighted_dist: Random::get_weighted_dist(weights),
        }
    }

    /// derive a u64 seed from entropy
    pub fn get_random_seed() -> u64 {
        let mut tmp_rng = SmallRng::from_entropy();
        tmp_rng.next_u64()
    }

    pub fn from_str_seed(seed_str: String, weights: Vec<i32>) -> Random {
        let seed_u64 = hash(seed_str.as_bytes());
        let mut rnd = Random::new(seed_u64, weights);
        rnd.seed_str = Some(seed_str);

        rnd
    }

    /// uses another rnd struct to derive initial seed for a new rnd struct
    pub fn from_previous_rnd(rnd: &mut Random, weights: Vec<i32>) -> Random {
        let seed_u64 = rnd.gen.next_u64();

        Random::new(seed_u64, weights)
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

    pub fn str_seed_to_u64(seed_str: &String) -> u64 {
        hash(seed_str.as_bytes())
    }

    fn get_weighted_dist(weights: Vec<i32>) -> WeightedAliasIndex<i32> {
        // sadly WeightedAliasIndex is initialized using a Vec. So im manually checking for the
        // correct size. I feel like there must be a better way also the current apprach allows
        // for invalid moves to be picked. But that should be no problem in pracise
        assert_eq!(weights.len(), 4);
        WeightedAliasIndex::new(weights).expect("expect valid weights")
    }

    /// sample a shift based on weight distribution
    pub fn sample_move(&mut self, shifts: [ShiftDirection; 4]) -> ShiftDirection {
        let index = self.weighted_dist.sample(&mut self.gen);
        *shifts.get(index).expect("out of bounds")
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

    /// this can be used to skip a gen step to ensure that a value is consumed in any case
    pub fn skip(&mut self) {
        self.gen.next_u64();
    }

    pub fn pick_element<'a, T>(&'a mut self, values: &'a Vec<T>) -> &T {
        &values[self.in_range_exclusive(0, values.len())]
    }

    pub fn random_circularity(&mut self) -> f32 {
        self.gen.next_u64() as f32 / u64::max_value() as f32
    }
}
