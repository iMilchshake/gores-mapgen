use crate::{
    config::{GenerationConfig, MapConfig},
    kernel::Kernel,
    map::{BlockType, Map, Overwrite},
    position::Position,
    post_processing as post,
    random::{Random, Seed},
    walker::CuteWalker,
};

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,

    /// PRNG wrapper
    pub rnd: Random,

    /// remember where generation began, so a start room can be placed in post processing
    spawn: Position,

    config: GenerationConfig,
}

pub fn generate_room(
    map: &mut Map,
    pos: Position,
    room_size: i32,
    platform_margin: i32,
    zone_type: Option<BlockType>,
) -> Result<(), &'static str> {
    if !map.pos_in_bounds(&pos.shifted_by(room_size + 2, room_size + 1)?)
        || !map.pos_in_bounds(&pos.shifted_by(room_size + 1, room_size + 1)?)
    {
        return Err("generate room out of bounds");
    }

    // carve room
    map.set_area_border(
        pos.shifted_by(-room_size, -room_size)?,
        pos.shifted_by(room_size, room_size)?,
        BlockType::Empty,
        Overwrite::Force,
    );

    // only reserve - 1 so that when this is used for platforms
    map.set_area(
        pos.shifted_by(-room_size + 1, -room_size + 1)?,
        pos.shifted_by(room_size - 1, room_size - 1)?,
        BlockType::EmptyReserved,
        Overwrite::Force,
    );

    match zone_type {
        Some(zone_type) => {
            // set start/finish line
            map.set_area_border(
                pos.shifted_by(-room_size - 1, -room_size - 1)?,
                pos.shifted_by(room_size + 1, room_size + 1)?,
                zone_type,
                Overwrite::ReplaceNonSolidForce,
            );

            // set spawns
            if zone_type == BlockType::Start {
                map.set_area(
                    pos.shifted_by(-(room_size - platform_margin), room_size - 1)?,
                    pos.shifted_by(room_size - platform_margin, room_size - 1)?,
                    BlockType::Spawn,
                    Overwrite::Force,
                );

                map.set_area(
                    pos.shifted_by(-(room_size - platform_margin), room_size + 1)?,
                    pos.shifted_by(room_size - platform_margin, room_size + 1)?,
                    BlockType::Platform,
                    Overwrite::Force,
                );
            }
        }
        None => {
            // for non start/finish rooms -> place center platform
            map.set_area(
                pos.shifted_by(-(room_size - platform_margin), room_size - 3)?,
                pos.shifted_by(room_size - platform_margin, room_size - 3)?,
                BlockType::Platform,
                Overwrite::Force,
            );
        }
    }

    Ok(())
}

impl Generator {
    /// derive a initial generator state based on a GenerationConfig
    pub fn new(map: Map, seed: Seed, config: GenerationConfig) -> Generator {
        let spawn = map.config.waypoints[0];
        let walker = CuteWalker::new(spawn, Kernel::new(5, 0.0), Kernel::new(7, 0.0), &map.config);

        let rnd = Random::new(seed, &config);

        Generator {
            walker,
            map,
            rnd,
            spawn,
            config,
        }
    }

    pub fn step(&mut self) -> Result<(), &'static str> {
        // check if walker has reached goal position
        if self
            .walker
            .is_goal_reached(self.config.waypoint_reached_dist)
            == Some(true)
        {
            self.walker.next_waypoint();
        }

        if !self.walker.finished {
            // randomly mutate kernel
            if self.walker.steps > self.config.fade_steps {
                self.walker.mutate_kernel(&self.config, &mut self.rnd);
            } else {
                self.walker.set_fade_kernel(
                    self.walker.steps,
                    self.config.fade_min_size,
                    self.config.fade_max_size,
                    self.config.fade_steps,
                );
            }

            // perform one step
            self.walker
                .probabilistic_step(&mut self.map, &self.config, &mut self.rnd)?;

            // handle platforms
            self.walker.check_platform(
                &mut self.map,
                self.config.platform_distance_bounds.0,
                self.config.platform_distance_bounds.1,
            )?;
        }

        Ok(())
    }

    pub fn post_processing(&mut self) -> Result<(), &'static str> {
        post::fix_edge_bugs(&mut self.map)?;

        generate_room(
            &mut self.map,
            self.spawn,
            6,
            3,
            Some(BlockType::Start)
        )?;
        
        generate_room(
            &mut self.map,
            self.walker.pos,
            4,
            3,
            Some(BlockType::Finish),
        )?;

        if self.config.min_freeze_size > 0 {
            // TODO: Maybe add some alternative function for the case of min_freeze_size=1
            post::remove_freeze_blobs(&mut self.map, self.config.min_freeze_size);
        }

        post::fill_open_areas(&mut self.map, self.config.max_distance);

        post::generate_all_skips(
            &mut self.map,
            self.config.skip_length_bounds,
            self.config.skip_min_spacing_sqr,
        )?;

        Ok(())
    }

    /// Generates an entire map with a single function call. This function is used by the CLI.
    /// It is important to keep this function up to date with the editor generation, so that
    /// fixed seed map generations result in the same map.
    pub fn generate_map(
        max_steps: usize,
        seed: Seed,
        gen_config: GenerationConfig,
        map_config: MapConfig,
    ) -> Result<Map, &'static str> {
        let map = Map::new(map_config, BlockType::Hookable);

        let mut gen = Generator::new(map, seed, gen_config);

        for _ in 0..max_steps {
            if gen.walker.finished {
                break;
            }
            gen.step()?;
        }

        gen.post_processing()?;

        Ok(gen.map)
    }

    pub fn finalize(&mut self, max_steps: usize) -> Result<(), &'static str> {
        self.config.validate()?;

        for _ in 0..max_steps {
            if self.walker.finished {
                break;
            }
            self.step()?;
        }

        self.post_processing()?;

        Ok(())
    }
}
