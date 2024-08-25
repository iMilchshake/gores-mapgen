use std::collections::HashMap;
use timing::Timer;

use crate::{
    config::{GenerationConfig, MapConfig},
    debug::DebugLayers,
    kernel::Kernel,
    map::{BlockType, Map, Overwrite},
    position::Position,
    post_processing::{self as post, get_flood_fill},
    random::{Random, Seed},
    walker::CuteWalker,
};

use macroquad::color::{colors, Color};

const PRINT_TIMES: bool = false;

pub fn print_time(timer: &Timer, message: &str) {
    // TODO: add cli flag for this
    if PRINT_TIMES {
        println!("{}: {:?}", message, timer.elapsed());
    }
}

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,

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
        let spawn = map_config.waypoints.first().unwrap().clone();
        let mut rnd = Random::new(seed, gen_config);

        let subwaypoints =
            Generator::generate_sub_waypoints(&map_config.waypoints, gen_config, &mut rnd)
                .unwrap_or(map_config.waypoints.clone()); // on failure just use initial waypoints

        // initialize walker
        let inner_kernel_size = rnd.sample_inner_kernel_size();
        let outer_kernel_size = inner_kernel_size + rnd.sample_outer_kernel_margin();
        let inner_kernel = Kernel::new(inner_kernel_size, 0.0);
        let outer_kernel = Kernel::new(outer_kernel_size, 0.0);
        let walker = CuteWalker::new(
            spawn.clone(),
            inner_kernel,
            outer_kernel,
            subwaypoints,
            &map,
        );

        Generator {
            walker,
            map,
            rnd,
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

            // TODO: very imperformant clone here, REVERT REVERT
            // fuck i want to call this in post procesing aswell -> move to map/generator
            self.debug_layers.get_mut("lock").unwrap().grid = self.walker.locked_positions.clone();

            // handle platforms TODO: remove once post processing is implemented
            // self.walker.check_platform(
            //     &mut self.map,
            //     config.platform_distance_bounds.0,
            //     config.platform_distance_bounds.1,
            // )?;
        }

        Ok(())
    }

    /// Generate subwaypoints for more consistent distance between walker waypoints. This
    /// ensures more controllable and consistent behaviour of the walker with respect to the
    /// distance to the target waypoint.
    /// TODO: currently uses non squared distances, could be optimized
    pub fn generate_sub_waypoints(
        waypoints: &[Position],
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
            let num_subwaypoints =
                ((distance / gen_config.max_subwaypoint_dist).floor() as usize).max(1);

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

    // TODO: move this "do all" function into post processing script?
    pub fn perform_all_post_processing(
        &mut self,
        gen_config: &GenerationConfig,
        debug_layers: &Option<DebugLayers>,
    ) -> Result<(), &'static str> {
        let timer = Timer::start();

        // lock all remaining blocks
        self.walker
            .lock_previous_location(&self.map, gen_config, true)?;

        if let Some(ref debug_layers) = debug_layers {
            debug_layers.bool_layers.get_mut("lock").unwrap().grid =
                self.walker.locked_positions.clone();
        }

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

        if gen_config.min_freeze_size > 0 {
            // TODO: Maybe add some alternative function for the case of min_freeze_size=1
            post::remove_freeze_blobs(self, gen_config.min_freeze_size);
            print_time(&timer, "detect blobs");
        }

        let flood_fill = get_flood_fill(self, &self.spawn);
        print_time(&timer, "flood fill");

        post::gen_all_platform_candidates(
            &self.walker.position_history,
            &flood_fill,
            &mut self.map,
            gen_config,
            &mut self.debug_layers,
        );
        print_time(&timer, "platforms");

        post::generate_all_skips(
            self,
            gen_config.skip_length_bounds,
            gen_config.skip_min_spacing_sqr,
            gen_config.max_level_skip,
            &flood_fill,
        );
        print_time(&timer, "generate skips");

        post::fill_open_areas(self, &gen_config.max_distance);
        print_time(&timer, "place obstacles");

        // post::remove_unused_blocks(&mut self.map, &self.walker.locked_positions);

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

        // perform all walker steps
        for _ in 0..max_steps {
            if gen.walker.finished {
                break;
            }
            gen.step(gen_config)?;
        }

        // perform all post processing step without creating any debug layers
        gen.perform_all_post_processing(gen_config, &None)?;

        Ok(gen.map)
    }
}
