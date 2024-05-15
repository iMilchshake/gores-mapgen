use std::collections::BTreeMap;
use timing::Timer;

use crate::{
    config::{GenerationConfig, MapConfig},
    debug::DebugLayer,
    kernel::Kernel,
    map::{BlockType, Map},
    position::Position,
    post_processing as post,
    random::{Random, Seed},
    walker::CuteWalker,
};

use macroquad::color::colors;

pub fn print_time(timer: &Timer, message: &str) {
    println!("{}: {:?}", message, timer.elapsed());
}

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,
    pub rnd: Random,
    pub debug_layers: BTreeMap<&'static str, DebugLayer>,
}

impl Generator {
    /// derive a initial generator state based on a GenerationConfig
    pub fn new(gen_config: &GenerationConfig, map_config: &MapConfig, seed: Seed) -> Generator {
        let spawn = Position::new(50, 250);
        let map = Map::new(300, 300, BlockType::Hookable, spawn.clone());
        let init_inner_kernel = Kernel::new(5, 0.0);
        let init_outer_kernel = Kernel::new(7, 0.0);
        let walker = CuteWalker::new(
            spawn,
            init_inner_kernel,
            init_outer_kernel,
            gen_config,
            map_config,
        );
        let rnd = Random::new(seed, gen_config);

        let debug_layers = BTreeMap::from([
            ("edge_bugs", DebugLayer::new(true, colors::BLUE, &map)),
            ("skips", DebugLayer::new(true, colors::GREEN, &map)),
            ("skips_invalid", DebugLayer::new(true, colors::RED, &map)),
            ("blobs", DebugLayer::new(false, colors::RED, &map)),
        ]);

        Generator {
            walker,
            map,
            rnd,
            debug_layers,
        }
    }

    pub fn step(&mut self, config: &GenerationConfig) -> Result<(), &'static str> {
        // check if walker has reached goal position
        if self.walker.is_goal_reached(&config.waypoint_reached_dist) == Some(true) {
            self.walker.next_waypoint();
        }

        if !self.walker.finished {
            config.validate()?;

            // randomly mutate kernel
            self.walker.mutate_kernel(config, &mut self.rnd);

            // perform one step
            self.walker
                .probabilistic_step(&mut self.map, config, &mut self.rnd)?;

            // handle platforms
            self.walker.check_platform(
                &mut self.map,
                config.platform_distance_bounds.0,
                config.platform_distance_bounds.1,
            )?;
        }

        Ok(())
    }

    pub fn post_processing(&mut self, config: &GenerationConfig) {
        let timer = Timer::start();

        let edge_bugs = post::fix_edge_bugs(self).expect("fix edge bugs failed");
        self.debug_layers.get_mut("edge_bugs").unwrap().grid = edge_bugs;
        print_time(&timer, "fix edge bugs");

        self.map
            .generate_room(&self.map.spawn.clone(), 4, 3, Some(&BlockType::Start))
            .expect("start room generation failed");
        self.map
            .generate_room(&self.walker.pos.clone(), 4, 3, Some(&BlockType::Finish))
            .expect("start finish room generation");
        print_time(&timer, "place rooms");

        if config.min_freeze_size > 0 {
            // TODO: Maybe add some alternative function for the case of min_freeze_size=1
            post::remove_freeze_blobs(self, config.min_freeze_size);
            print_time(&timer, "detect blobs");
        }

        post::fill_open_areas(self, &config.max_distance);
        print_time(&timer, "place obstacles");

        post::generate_all_skips(self, config.skip_length_bounds, config.skip_min_spacing_sqr);
        print_time(&timer, "generate skips");
    }

    /// Generates an entire map with a single function call. This function is used by the CLI.
    /// It is important to keep this function up to date with the editor generation, so that
    /// fixed seed map generations result in the same map.
    pub fn generate_map(
        max_steps: usize,
        seed: &Seed,
        gen_config: &GenerationConfig,
        map_config: &MapConfig,
    ) -> Result<Map, &'static str> {
        let mut gen = Generator::new(gen_config, map_config, seed.clone());

        for _ in 0..max_steps {
            if gen.walker.finished {
                break;
            }
            gen.step(gen_config)?;
        }

        gen.post_processing(gen_config);

        Ok(gen.map)
    }
}
