mod editor;
mod grid_render;
mod map;
mod position;
mod walker;

use editor::*;
use grid_render::*;
use map::*;
use position::*;
use walker::*;

use macroquad::{color::*, math::Vec2, miniquad, time::get_fps, window::*};
use miniquad::conf::{Conf, Platform};
use std::time::{self, Duration, Instant};

use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::WeightedAliasIndex;

use seahash::hash;

const DISABLE_VSYNC: bool = true;
const STEPS_PER_FRAME: usize = 50;

pub struct FPSControl {
    max_fps: Option<usize>,
    frame_start: Option<Instant>,
    min_frame_time: Option<Duration>,
}

impl FPSControl {
    pub fn new() -> FPSControl {
        FPSControl {
            frame_start: None,
            max_fps: None,
            min_frame_time: None,
        }
    }

    pub fn with_max_fps(mut self, max_fps: usize) -> Self {
        self.max_fps = Some(max_fps);
        self.min_frame_time = Some(Duration::from_secs_f32(1. / max_fps as f32));

        self
    }

    pub fn on_frame_start(&mut self) {
        if let Some(_) = self.max_fps {
            self.frame_start = Some(time::Instant::now());
        }
    }

    pub async fn wait_for_next_frame(&self) {
        next_frame().await; // submit our render calls to our screen

        if let Some(_) = self.max_fps {
            let frame_start = self.frame_start.expect("this should be set on_frame_start");
            let min_frame_time = self.min_frame_time.expect("should be set in MaxFps mode");

            // wait for frametime to be at least minimum_frame_time which
            // results in a upper limit for the FPS
            let frame_finish = Instant::now();
            let frame_time = frame_finish.duration_since(frame_start);

            if frame_time < min_frame_time {
                let time_to_sleep = min_frame_time
                    .checked_sub(frame_time)
                    .expect("time subtraction failed");
                std::thread::sleep(time_to_sleep);
            }
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "egui with macroquad".to_owned(),
        platform: Platform {
            swap_interval: match DISABLE_VSYNC {
                true => Some(0), // set swap_interval to 0 to disable vsync
                false => None,
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

// TODO: not quite sure where to put this, this doesnt
// have any functionality, so a seperate file feels overkill
#[derive(Debug, Copy, Clone)]
pub enum ShiftDirection {
    Up,
    Right,
    Down,
    Left,
}

struct Random {
    seed: String,
    seed_u64: u64,
    gen: SmallRng,
    weighted_dist: WeightedAliasIndex<i32>,
}

impl Random {
    fn new(seed: String, weights: Vec<i32>) -> Random {
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
    fn sample_move(&mut self, shifts: [ShiftDirection; 4]) -> ShiftDirection {
        let index = self.weighted_dist.sample(&mut self.gen);
        *shifts.get(index).expect("out of bounds")
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut rnd = Random::new("iMilchshake".to_string(), vec![4, 3, 2, 1]);

    let mut editor = Editor::new(EditorPlayback::Paused);

    let mut map = Map::new(300, 300, BlockType::Empty);
    let kernel = Kernel::new(8, 0.9);
    let waypoints: Vec<Position> = vec![
        Position::new(250, 50),
        Position::new(250, 250),
        Position::new(50, 250),
        Position::new(50, 50),
    ];
    let mut walker = CuteWalker::new(Position::new(50, 50), waypoints, kernel);
    let mut fps_control = FPSControl::new().with_max_fps(60);

    loop {
        fps_control.on_frame_start();
        editor.on_frame_start();

        // walker logic
        if editor.playback.is_not_paused() {
            for _ in 0..STEPS_PER_FRAME {
                // check if walker has reached goal position
                if walker.is_goal_reached() == Some(true) {
                    walker.next_waypoint().unwrap_or_else(|_| {
                        println!("pause due to reaching last checkpoint");
                        editor.playback.pause();
                    });
                }

                // randomly mutate kernel
                if rnd.gen.gen_bool(0.1) {
                    let size = rnd.gen.gen_range(3..=7);
                    let circularity = rnd.gen.gen_range(0.0..=1.0);
                    walker.kernel = Kernel::new(size, circularity);
                }

                // perform one greedy step
                if let Err(err) = walker.probabilistic_step(&mut map, &mut rnd) {
                    println!("greedy step failed: '{:}' - pausing...", err);
                    editor.playback.pause();
                }

                // walker did a step using SingleStep -> now pause
                if editor.playback == EditorPlayback::SingleStep {
                    editor.playback.pause();
                    break; // skip following steps for this frame!
                }
            }
        }

        editor.define_egui(&walker);
        editor.set_cam(&map);
        editor.handle_user_inputs(&map);

        clear_background(WHITE);
        draw_grid_blocks(&map.grid);
        draw_waypoints(&walker.waypoints);
        draw_walker(&walker);
        draw_walker_kernel(&walker);

        egui_macroquad::draw();

        fps_control.wait_for_next_frame().await;
    }
}
