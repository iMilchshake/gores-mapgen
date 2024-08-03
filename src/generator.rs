use std::collections::BTreeMap;
use timing::Timer;

use crate::{
    config::{GenerationConfig, MapConfig},
    debug::DebugLayer,
    kernel::Kernel,
    map::{BlockType, Map, Overwrite},
    position::Position,
    post_processing as post,
    random::{Random, Seed},
    walker::CuteWalker,
};

use macroquad::color::colors;

pub fn print_time(_timer: &Timer, _message: &str) {
    // println!("{}: {:?}", message, timer.elapsed());
}

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,
    pub debug_layers: BTreeMap<&'static str, DebugLayer>,

    /// PRNG wrapper
    pub rnd: Random,

    /// remember where generation began, so a start room can be placed in post processing
    spawn: Position,
}

pub fn generate_room(
    map: &mut Map,
    pos: &Position,
    room_size: usize,
    platform_margin: usize,
    zone_type: Option<&BlockType>,
) -> Result<(), &'static str> {
    let room_size: i32 = room_size as i32;
    let platform_margin: i32 = platform_margin as i32;

    if !map.pos_in_bounds(&pos.shifted_by(room_size + 2, room_size + 1).unwrap())
        || !map.pos_in_bounds(&pos.shifted_by(room_size + 1, room_size + 1).unwrap())
    {
        return Err("generate room out of bounds");
    }

    // carve room
    map.set_area_border(
        &pos.shifted_by(-room_size, -room_size)?,
        &pos.shifted_by(room_size, room_size)?,
        &BlockType::Empty,
        &Overwrite::Force,
    );

    // only reserve - 1 so that when this is used for platforms
    map.set_area(
        &pos.shifted_by(-room_size + 1, -room_size + 1)?,
        &pos.shifted_by(room_size - 1, room_size - 1)?,
        &BlockType::EmptyReserved,
        &Overwrite::Force,
    );

    // set start/finish line
    if let Some(zone_type) = zone_type {
        map.set_area_border(
            &pos.shifted_by(-room_size - 1, -room_size - 1)?,
            &pos.shifted_by(room_size + 1, room_size + 1)?,
            zone_type,
            &Overwrite::ReplaceNonSolidForce,
        );
    }

    // set spawns
    if zone_type == Some(&BlockType::Start) {
        map.set_area(
            &pos.shifted_by(-(room_size - platform_margin), room_size - 1)?,
            &pos.shifted_by(room_size - platform_margin, room_size - 1)?,
            &BlockType::Spawn,
            &Overwrite::Force,
        );
    }

    // set platform below spawns
    if zone_type == Some(&BlockType::Start) {
        map.set_area(
            &pos.shifted_by(-(room_size - platform_margin), room_size + 1)?,
            &pos.shifted_by(room_size - platform_margin, room_size + 1)?,
            &BlockType::Platform,
            &Overwrite::Force,
        );
    }

    // for non start/finish rooms -> place center platform
    if zone_type.is_none() {
        map.set_area(
            &pos.shifted_by(-(room_size - platform_margin), room_size - 3)?,
            &pos.shifted_by(room_size - platform_margin, room_size - 3)?,
            &BlockType::Platform,
            &Overwrite::Force,
        );
    }

    Ok(())
}

impl Generator {
    /// derive an initial generator state based on a GenerationConfig
    pub fn new(gen_config: &GenerationConfig, map_config: &MapConfig, seed: Seed) -> Generator {
        let map = Map::new(map_config.width, map_config.height, BlockType::Hookable);
        let spawn = map_config.waypoints.get(0).unwrap().clone();
        let mut rnd = Random::new(seed, gen_config);

        let subwaypoints =
            Generator::generate_sub_waypoints(&map_config.waypoints, &gen_config, &mut rnd)
                .unwrap_or(map_config.waypoints.clone()); // on failure just use initial waypoints

        // initialize walker
        let inner_kernel_size = rnd.sample_inner_kernel_size();
        let outer_kernel_size = inner_kernel_size + rnd.sample_outer_kernel_margin();
        let inner_kernel = Kernel::new(inner_kernel_size, 0.0);
        let outer_kernel = Kernel::new(outer_kernel_size, 0.0);
        let walker = CuteWalker::new(spawn.clone(), inner_kernel, outer_kernel, subwaypoints);

        // TODO: rework shitty debug storage
        let debug_layers = BTreeMap::from([
            ("edge_bugs", DebugLayer::new(true, colors::BLUE, &map)),
            ("freeze_skips", DebugLayer::new(true, colors::ORANGE, &map)),
            ("skips", DebugLayer::new(true, colors::GREEN, &map)),
            ("skips_invalid", DebugLayer::new(true, colors::RED, &map)),
            ("blobs", DebugLayer::new(false, colors::RED, &map)),
        ]);

        Generator {
            walker,
            map,
            rnd,
            debug_layers,
            spawn,
        }
    }

    /// perform one step of the map generation
    pub fn step(&mut self, config: &GenerationConfig) -> Result<(), &'static str> {
        // check if walker has reached goal position
        if self.walker.is_goal_reached(&config.waypoint_reached_dist) == Some(true) {
            self.walker.next_waypoint();
        }

        if !self.walker.finished {
            config.validate()?; // TODO: how much does this slow down generation?

            // randomly mutate kernel
            if self.walker.steps > config.fade_steps {
                self.walker.mutate_kernel(config, &mut self.rnd);
            } else {
                self.walker.set_fade_kernel(
                    self.walker.steps,
                    config.fade_min_size,
                    config.fade_max_size,
                    config.fade_steps,
                );
            }

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

    /// Generate subwaypoints for more consistent distance between walker waypoints. This
    /// ensures more controllable and consistent behaviour of the walker with respect to the
    /// distance to the target waypoint.
    /// TODO: currently uses non squared distances, could be optimized
    pub fn generate_sub_waypoints(
        waypoints: &Vec<Position>,
        gen_config: &GenerationConfig,
        rnd: &mut Random,
    ) -> Option<Vec<Position>> {
        if gen_config.max_subwaypoint_dist <= 0.0 {
            return None;
        }

        let mut subwaypoints: Vec<Position> = Vec::new();

        // iterate over all neighboring pairs of global waypoints
        for (p1, p2) in waypoints.windows(2).map(|w| (&w[0], &w[1])) {
            let distance = p1.distance(p2);
            let num_subwaypoints = (distance / gen_config.max_subwaypoint_dist).floor() as usize;

            for subwaypoint_index in 0..num_subwaypoints {
                let lerp_weight = (subwaypoint_index as f32) / (num_subwaypoints as f32);
                let base_subwaypoint = p1.lerp(p2, lerp_weight);

                // try to shift waypoint in random direction
                let mutated_subwaypoint = base_subwaypoint
                    .random_shift(rnd, gen_config.subwaypoint_max_shift_dist)
                    .unwrap_or(base_subwaypoint);

                subwaypoints.push(mutated_subwaypoint);
            }
        }

        // add last point
        subwaypoints.push(waypoints.last().unwrap().clone());

        Some(subwaypoints)
    }

    pub fn post_processing(&mut self, config: &GenerationConfig) -> Result<(), &'static str> {
        let timer = Timer::start();

        let edge_bugs = post::fix_edge_bugs(self).expect("fix edge bugs failed");
        self.debug_layers.get_mut("edge_bugs").unwrap().grid = edge_bugs;
        print_time(&timer, "fix edge bugs");

        generate_room(&mut self.map, &self.spawn, 6, 3, Some(&BlockType::Start))
            .expect("start room generation failed");
        generate_room(
            &mut self.map,
            &self.walker.pos.clone(),
            4,
            3,
            Some(&BlockType::Finish),
        )
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

        Ok(())
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

        gen.post_processing(gen_config)?;

        Ok(gen.map)
    }
}
