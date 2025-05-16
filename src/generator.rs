use clap::crate_version;
use timing::Timer;

use crate::{
    config::{GenerationConfig, MapConfig, ThemeConfig},
    debug::DebugLayers,
    kernel::Kernel,
    map::{BlockType, Map, Overwrite},
    position::Position,
    post_processing::{self as post, flood_fill},
    random::{Random, Seed},
    utils::safe_slice_mut,
    walker::CuteWalker,
};

pub fn print_time(timer: &mut Timer, message: &str, print: bool) {
    if print {
        println!("{}: {:?}", message, timer.elapsed());
        *timer = Timer::start() // start new timer
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
            gen_config,
        );

        let mut gen = Generator {
            walker,
            map,
            rnd,
            spawn,
        };

        gen.preprocessing(thm_config).unwrap(); // TODO: move somewhere else + pass
        gen
    }

    pub fn preprocessing(&mut self, thm_config: &ThemeConfig) -> Result<(), &'static str> {
        // test locking for spawn TODO: add helper
        let spawn_width: i32 = thm_config.spawn_width as i32;
        let spawn_height: i32 = thm_config.spawn_height as i32;
        let margin: i32 = thm_config.spawn_margin as i32;

        let top_left = self
            .spawn
            .shifted_by(-spawn_width + margin, -(spawn_height / 2))?;
        let bot_right = self.spawn.shifted_by(margin, spawn_height / 2)?;

        // lock area around spawn area so in future walker cant cross it
        let mut spawn_lock = safe_slice_mut(
            &mut self.walker.locked_positions,
            &top_left,
            &bot_right,
            &self.map,
        )?;
        spawn_lock.fill(true);

        // unlock some [1 x margin] wide path so walker can escape locking
        let mut spawn_escape_unlock = safe_slice_mut(
            &mut self.walker.locked_positions,
            &self.spawn,
            &self.spawn.shifted_by(margin, 0)?,
            &self.map,
        )?;
        spawn_escape_unlock.fill(false);

        // lock padding at map border. amount of padding should ensure that no kernel or locking
        // operation can be out of bounds. As locking is always at least as large as the largest
        // kernel, i just use lock size +1
        let padding = (self.walker.lock_size) + 1;
        let mut top_pad = safe_slice_mut(
            &mut self.walker.locked_positions,
            &Position::new(0, 0),
            &Position::new(self.map.width - 1, padding),
            &self.map,
        )?;
        top_pad.fill(true);

        let mut bot_pad = safe_slice_mut(
            &mut self.walker.locked_positions,
            &Position::new(0, self.map.height - (1 + padding)),
            &Position::new(self.map.width - 1, self.map.height - 1),
            &self.map,
        )?;
        bot_pad.fill(true);

        let mut left_pad = safe_slice_mut(
            &mut self.walker.locked_positions,
            &Position::new(0, 0),
            &Position::new(padding, self.map.height - 1),
            &self.map,
        )?;
        left_pad.fill(true);

        let mut right_pad = safe_slice_mut(
            &mut self.walker.locked_positions,
            &Position::new(self.map.width - (1 + padding), 0),
            &Position::new(self.map.width - 1, self.map.height - 1),
            &self.map,
        )?;
        right_pad.fill(true);

        Ok(())
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
            .shifted_by(-spawn_width + (2 * margin), -(spawn_height / 2) + margin)
            .unwrap();

        let bot_right = self
            .spawn
            .shifted_by(0, (spawn_height / 2) - margin)
            .unwrap();

        // carve empty area
        self.map.set_area(
            &top_left,
            &bot_right,
            &BlockType::EmptyRoom,
            &Overwrite::Force,
        );

        // set start line
        self.map.set_area_border(
            &top_left.shifted_by(-1, -1).unwrap(),
            &bot_right.shifted_by(1, 1).unwrap(),
            &BlockType::Start,
            &Overwrite::ReplaceNonSolidFade,
        );

        // set elevated platform
        self.map.set_area(
            &Position::new(top_left.x, self.spawn.y - 1),
            &Position::new(top_left.x + platform_width, self.spawn.y + 1),
            &BlockType::Hookable,
            &Overwrite::ReplaceNonSolidRoom,
        );

        // set spawns
        self.map.set_area(
            &Position::new(top_left.x, self.spawn.y - 2),
            &Position::new(top_left.x + platform_width, self.spawn.y - 2),
            &BlockType::Spawn,
            &Overwrite::ReplaceNonSolidRoom,
        );
        self.map.set_area(
            &Position::new(top_left.x, bot_right.y),
            &Position::new(top_left.x + platform_width, bot_right.y),
            &BlockType::Spawn,
            &Overwrite::ReplaceNonSolidRoom,
        );

        let char_per_line = 14;
        let crate_version = crate_version!();
        let prefix_full = "VERSION: ";
        let prefix_short = "V: ";
        let prefix = if prefix_full.len() + crate_version.len() <= char_per_line {
            prefix_full
        } else {
            prefix_short
        };
        let available_space = char_per_line - prefix.len();
        let version_str = format!("{:>width$}", crate_version, width = available_space);
        let version_line = format!("{}{}", prefix, version_str);

        let info_text = format!(
            "RANDOM   GORES\n\
            BY IMILCHSHAKE\n\
            {}\n",
            version_line
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
            &BlockType::EmptyRoom,
            &Overwrite::Force,
        );

        self.map.write_text(
            &textbox_top_left
                .shifted_by(text_margin, text_margin)
                .unwrap(),
            &info_text,
        );
    }

    /// perform one step of the map generation
    pub fn step(
        &mut self,
        gen_config: &GenerationConfig,
        validate: bool,
        debug_layers: &mut Option<DebugLayers>,
    ) -> Result<(), &'static str> {
        // check if walker has reached currernt goal position
        if self
            .walker
            .is_goal_reached(&gen_config.waypoint_reached_dist)
            == Some(true)
        {
            // get next waypoint
            self.walker.next_waypoint();

            // if enabled, keep skipping invalid waypoints
            while let Some(goal) = &self.walker.goal {
                let is_invalid = gen_config.skip_invalid_waypoints
                    && (!self.map.pos_in_bounds(goal)
                        || self.walker.locked_positions[goal.as_index()]);

                if is_invalid {
                    self.walker.next_waypoint();
                } else {
                    break; // valid waypoint -> stop searching
                }
            }

            // lock all other waypoints
            if gen_config.waypoint_lock_distance > 0 {
                self.walker.update_waypoint_locks(
                    gen_config.waypoint_lock_distance,
                    &self.map,
                    debug_layers,
                )?;
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

    pub fn perform_all_post_processing(
        &mut self,
        gen_config: &GenerationConfig,
        thm_config: &ThemeConfig,
        debug_layers: &mut Option<DebugLayers>,
        verbose: bool,
    ) -> Result<(), &'static str> {
        let mut timer = Timer::start();

        let edge_bugs = post::fix_edge_bugs_expanding(self).expect("fix edge bugs failed");
        print_time(&mut timer, "fix edge bugs", verbose);

        self.generate_spawn(thm_config);
        post::generate_finish_room(self, &self.walker.pos.clone(), 4)?;
        print_time(&mut timer, "place rooms", verbose);

        // lock all remaining blocks
        self.walker
            .lock_previous_location(&self.map, gen_config, true)?;
        print_time(&mut timer, "finish walker lock", verbose);

        if gen_config.min_freeze_size > 0 {
            // TODO: Maybe add some alternative function for the case of min_freeze_size=1
            post::remove_freeze_blobs(self, gen_config.min_freeze_size, debug_layers);
            print_time(&mut timer, "detect blobs", verbose);
        }

        let ff = flood_fill(self, &[self.spawn.clone()], Some(&self.walker.pos), false)?;
        print_time(&mut timer, "flood fill", verbose);
        let ff_main_path = flood_fill(self, ff.path.as_ref().unwrap(), None, true)?;
        print_time(&mut timer, "flood fill (main path dist)", verbose);

        // fill up dead ends
        if gen_config.use_dead_end_removal {
            let dead_end_blocks =
                post::fill_dead_ends(&mut self.map, gen_config, &ff_main_path.distance)?;
            print_time(&mut timer, "fill dead ends", verbose);

            // fix stair artifacts resulting from dead end filling
            post::fix_stairs(&mut self.map, dead_end_blocks, &mut self.rnd);
            print_time(&mut timer, "fix stairs", verbose);
        }

        // TODO: only perform this for updated blocks?
        post::fix_edge_bugs_expanding(self).expect("fix edge bugs failed");
        print_time(&mut timer, "fix edge_bugs #2", verbose);

        // post::gen_legacy_all_platforms(
        //     &self.walker.position_history,
        //     &ff.distance,
        //     &mut self.map,
        //     gen_config,
        //     debug_layers,
        // );
        // print_time(&mut timer, "platforms", verbose);

        post::generate_all_skips(
            self,
            gen_config.skip_length_bounds,
            gen_config.skip_min_spacing_sqr,
            gen_config.max_level_skip,
            &ff.distance,
            debug_layers,
        );
        print_time(&mut timer, "generate skips", verbose);

        let ff_map_length =
            ff.distance[self.walker.pos.as_index()].expect("cant determine map length");

        // platforms
        let floor_pos = post::generate_platforms(
            &mut self.map,
            gen_config,
            &ff.distance,
            ff_map_length,
            debug_layers,
        )?;
        print_time(&mut timer, "generate platforms", verbose);

        post::fill_open_areas(self, &gen_config.max_distance, debug_layers);
        print_time(&mut timer, "place obstacles", verbose);

        // post::remove_unused_blocks(&mut self.map, &self.walker.locked_positions);

        if let Some(debug_layers) = debug_layers {
            debug_layers
                .float_layers
                .get_mut("flood_fill")
                .unwrap()
                .grid = ff.distance.map(|v| v.map(|v| v as f32));
            if let Some(path) = ff.path.as_ref() {
                let path_grid = &mut debug_layers.bool_layers.get_mut("path").unwrap().grid;
                for pos in path {
                    path_grid[pos.as_index()] = true;
                }
            }
            debug_layers
                .float_layers
                .get_mut("main_path_dist")
                .unwrap()
                .grid = ff_main_path.distance.map(|v| v.map(|v| v as f32));
            debug_layers.bool_layers.get_mut("lock").unwrap().grid =
                self.walker.locked_positions.clone();
            debug_layers.bool_layers.get_mut("edge_bugs").unwrap().grid = edge_bugs;

            let grid = &mut debug_layers.bool_layers.get_mut("floor").unwrap().grid;

            // floor
            for floor_pos in floor_pos {
                grid[floor_pos.pos.as_index()] = true;
            }
        }
        print_time(&mut timer, "set debug layers", verbose);

        Ok(())
    }

    /// Perform preprocessing steps that are intended for map export, this call can be skipped
    /// if the generated maps are not intended to be exported
    pub fn export_preprocess(
        &mut self,
        thm_config: &ThemeConfig,
        debug_layers: &mut Option<DebugLayers>,
        verbose: bool,
    ) {
        let mut timer = Timer::start();

        // flip before generating noise, as overlay noise depends on it
        if self.rnd.get_bool_with_prob(0.5) {
            self.map.flip_x_axis();
            print_time(&mut timer, "flip map", verbose);
        }

        post::generate_noise_layers(&mut self.map, &mut self.rnd, thm_config, debug_layers);
        print_time(&mut timer, "generate noise layers", verbose);
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
        export_preprocess: bool,
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
        gen.perform_all_post_processing(gen_config, thm_config, &mut None, false)?;

        // if enabled, perform all export preprocessing steps without debug layers
        if export_preprocess {
            gen.export_preprocess(thm_config, &mut None, false);
        }

        Ok(gen.map)
    }
}
