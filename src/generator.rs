use clap::crate_version;
use ndarray::s;
use timing::Timer;

use crate::{
    config::{GenerationConfig, MapConfig, ThemeConfig},
    debug::DebugLayers,
    kernel::Kernel,
    map::{BlockType, Map, Overwrite},
    noise::{self},
    position::Position,
    post_processing::{self as post, get_flood_fill},
    random::{Random, Seed},
    walker::CuteWalker,
};

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
            &pos.shifted_by(-(room_size - platform_margin), room_size)?,
            &pos.shifted_by(room_size - platform_margin, room_size)?,
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
    pub fn new(
        gen_config: &GenerationConfig,
        map_config: &MapConfig,
        thm_config: &ThemeConfig,
        seed: Seed,
    ) -> Generator {
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

        let mut gen = Generator {
            walker,
            map,
            rnd,
            spawn,
        };

        gen.preprocessing(thm_config);
        gen
    }

    pub fn preprocessing(&mut self, thm_config: &ThemeConfig) {
        // test locking for spawn TODO: add helper
        let mut view = self.walker.locked_positions.slice_mut(s![
            self.spawn.x - thm_config.spawn_width..=self.spawn.x,
            self.spawn.y - thm_config.spawn_height / 2..=self.spawn.y + thm_config.spawn_height / 2
        ]);
        view.fill(true);
    }

    pub fn generate_spawn(&mut self, thm_config: &ThemeConfig) {
        assert!(thm_config.spawn_height % 2 == 0, "spawn height not even");

        // TODO: these inconsistent types are annoying xd
        let spawn_width: i32 = thm_config.spawn_width as i32;
        let spawn_height: i32 = thm_config.spawn_height as i32;
        let margin: i32 = thm_config.spawn_margin as i32;
        let platform_width: usize = thm_config.spawn_platform_width;

        let top_left = self
            .spawn
            .shifted_by(-spawn_width + margin, -(spawn_height / 2) + margin)
            .unwrap();

        let bot_right = self
            .spawn
            .shifted_by(0, (spawn_height / 2) - margin)
            .unwrap();

        // carve empty area
        self.map.set_area(
            &top_left,
            &bot_right,
            &BlockType::EmptyReserved,
            &Overwrite::Force,
        );

        // set start line
        self.map.set_area_border(
            &top_left.shifted_by(-1, -1).unwrap(),
            &bot_right.shifted_by(1, 1).unwrap(),
            &BlockType::Start,
            &Overwrite::ReplaceNonSolidForce,
        );

        // set elevated platform
        self.map.set_area(
            &Position::new(top_left.x, self.spawn.y - 1),
            &Position::new(top_left.x + platform_width, self.spawn.y + 1),
            &BlockType::Hookable,
            &Overwrite::ReplaceNonSolidForce,
        );

        // set spawns
        self.map.set_area(
            &Position::new(top_left.x, self.spawn.y - 2),
            &Position::new(top_left.x + platform_width, self.spawn.y - 2),
            &BlockType::Spawn,
            &Overwrite::ReplaceNonSolidForce,
        );
        self.map.set_area(
            &Position::new(top_left.x, bot_right.y),
            &Position::new(top_left.x + platform_width, bot_right.y),
            &BlockType::Spawn,
            &Overwrite::ReplaceNonSolidForce,
        );

        let info_text = format!(
            "RANDOM   GORES\n\
            BY IMILCHSHAKE\n\
            VERSION: {:}\n\
            !BETA!",
            crate_version!()
        );

        let text_width = info_text.lines().map(str::len).max().unwrap_or(0) as i32;
        let text_height = info_text.lines().count() as i32;

        // carve area for text
        let text_margin = thm_config.text_margin as i32;

        let textbox_top_left = Position::new(
            top_left.x + thm_config.textbox_left_offset,
            bot_right.y + thm_config.textbox_top_offset + 1,
        );
        let textbox_bot_right = textbox_top_left
            .shifted_by(
                text_width - 1 + (2 * text_margin),
                text_height - 1 + (2 * text_margin),
            )
            .unwrap();

        self.map.set_area(
            &textbox_top_left,
            &textbox_bot_right,
            &BlockType::EmptyReserved,
            &Overwrite::Force,
        );

        self.write_text(
            &textbox_top_left
                .shifted_by(text_margin, text_margin)
                .unwrap(),
            &info_text,
        );
    }

    pub fn write_text(&mut self, pos: &Position, text: &String) {
        let mut cursor = pos.clone();

        for ch in text.chars() {
            if ch == '\n' {
                cursor.y += 1;
                cursor.x = pos.x;
            } else {
                self.map.font_layer[cursor.as_index()] = ch;
                cursor.x += 1;
            }
        }
    }

    /// perform one step of the map generation
    pub fn step(
        &mut self,
        gen_config: &GenerationConfig,
        validate: bool,
        debug_layers: &mut Option<DebugLayers>,
    ) -> Result<(), &'static str> {
        // check if walker has reached goal position
        if self
            .walker
            .is_goal_reached(&gen_config.waypoint_reached_dist)
            == Some(true)
        {
            self.walker.next_waypoint();
            if gen_config.waypoint_lock_distance > 0 {
                self.walker
                    .update_waypoint_locks(gen_config.waypoint_lock_distance, debug_layers);
            }
        }

        if !self.walker.finished {
            if validate {
                gen_config.validate()?;
            }

            // randomly mutate kernel
            if self.walker.steps > gen_config.fade_steps {
                self.walker.mutate_kernel(gen_config, &mut self.rnd);
            } else {
                self.walker.set_fade_kernel(
                    self.walker.steps,
                    gen_config.fade_min_size,
                    gen_config.fade_max_size,
                    gen_config.fade_steps,
                );
            }

            // perform one step
            self.walker.probabilistic_step(
                &mut self.map,
                gen_config,
                &mut self.rnd,
                debug_layers,
            )?;
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
        thm_config: &ThemeConfig,
        debug_layers: &mut Option<DebugLayers>,
    ) -> Result<(), &'static str> {
        let timer = Timer::start();

        // lock all remaining blocks
        self.walker
            .lock_previous_location(&self.map, gen_config, true)?;

        let edge_bugs = post::fix_edge_bugs(self).expect("fix edge bugs failed");
        print_time(&timer, "fix edge bugs");

        self.generate_spawn(thm_config);

        // generate_room(&mut self.map, &self.spawn, 6, 3, Some(&BlockType::Start))
        //     .expect("start room generation failed");
        generate_room(
            &mut self.map,
            &self.walker.pos.clone(),
            4,
            3,
            Some(&BlockType::Finish),
        )
        .expect("start finish room generation");
        self.write_text(&self.walker.pos.shifted_by(-2, 0)?, &"GG :>".to_string());
        print_time(&timer, "place rooms");

        if gen_config.min_freeze_size > 0 {
            // TODO: Maybe add some alternative function for the case of min_freeze_size=1
            post::remove_freeze_blobs(self, gen_config.min_freeze_size, debug_layers);
            print_time(&timer, "detect blobs");
        }

        let flood_fill = get_flood_fill(self, &self.spawn, debug_layers);
        print_time(&timer, "flood fill");

        post::gen_all_platform_candidates(
            &self.walker.position_history,
            &flood_fill,
            &mut self.map,
            gen_config,
            debug_layers,
        );
        print_time(&timer, "platforms");

        post::generate_all_skips(
            self,
            gen_config.skip_length_bounds,
            gen_config.skip_min_spacing_sqr,
            gen_config.max_level_skip,
            &flood_fill,
            debug_layers,
        );
        print_time(&timer, "generate skips");

        post::fill_open_areas(self, &gen_config.max_distance, debug_layers);
        print_time(&timer, "place obstacles");

        // post::remove_unused_blocks(&mut self.map, &self.walker.locked_positions);

        if let Some(debug_layers) = debug_layers {
            debug_layers.bool_layers.get_mut("lock").unwrap().grid =
                self.walker.locked_positions.clone();
            debug_layers.bool_layers.get_mut("edge_bugs").unwrap().grid = edge_bugs;
        }
        print_time(&timer, "set debug layers");

        self.map.noise_overlay = noise::generate_noise_array(
            &self.map,
            thm_config.overlay_noise_scale,
            thm_config.overlay_noise_invert,
            thm_config.overlay_noise_threshold,
            thm_config.overlay_noise_type,
            true,
            self.rnd.random_u32(),
        );

        if let Some(debug_layers) = debug_layers {
            debug_layers.bool_layers.get_mut("noise").unwrap().grid =
                self.map.noise_overlay.clone();
        }

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
        thm_config: &ThemeConfig,
    ) -> Result<Map, &'static str> {
        let mut gen = Generator::new(gen_config, map_config, thm_config, seed.clone());

        // validate config
        gen_config.validate()?;

        // perform all walker steps, skip further validation/debugging
        for _ in 0..max_steps {
            if gen.walker.finished {
                break;
            }
            gen.step(gen_config, false, &mut None)?;
        }

        // perform all post processing step without creating any debug layers
        gen.perform_all_post_processing(gen_config, thm_config, &mut None)?;

        Ok(gen.map)
    }
}
