use crate::ShiftDirection;
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::WeightedAliasIndex;
use seahash::hash;

pub struct Random {
    seed: String,
    seed_u64: u64,
    gen: SmallRng,
    weighted_dist: WeightedAliasIndex<i32>,
}

impl Random {
    pub fn new(seed: String, weights: Vec<i32>) -> Random {
        let seed_u64 = hash(seed.as_bytes());
        Random {
            seed,
            seed_u64,
            gen: SmallRng::seed_from_u64(seed_u64),
            weighted_dist: Random::get_weighted_dist(weights),
        }
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
        self.gen.gen_bool(probability.into())
    }

    pub fn pick_element(&mut self, values: &Vec<usize>) -> usize {
        values[self.gen.gen_range(0..values.len())]
    }

    pub fn random_kernel_size(&mut self, max_size: usize) -> usize {
        assert!(max_size >= 1); // at least 1
        let sizes_count = max_size.div_ceil(2);
        let size_index = self.gen.gen_range(0..sizes_count);

        2 * size_index + 1
    }
}
