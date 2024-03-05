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

use macroquad::{
    color::*,
    math::{vec2, Vec2},
    miniquad,
    time::get_fps,
    window::*,
};
use miniquad::conf::{Conf, Platform};
use std::time::{self, Duration, Instant};

use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::WeightedAliasIndex;

use seahash::hash;

const TARGET_FPS: usize = 60;
const DISABLE_VSYNC: bool = true;
const AVG_FPS_FACTOR: f32 = 0.25; // how much current fps is weighted into the rolling average

const STEPS_PER_FRAME: usize = 50;

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

async fn wait_for_next_frame(frame_start: Instant, minimum_frame_time: Duration) {
    next_frame().await; // submit our render calls to our screen

    // wait for frametime to be at least minimum_frame_time which
    // results in a upper limit for the FPS
    let frame_finish = time::Instant::now();
    let frame_time = frame_finish.duration_since(frame_start);

    if frame_time < minimum_frame_time {
        let time_to_sleep = minimum_frame_time
            .checked_sub(frame_time)
            .expect("time subtraction failed");
        std::thread::sleep(time_to_sleep);
    }
}

struct Random {
    seed: String,
    seed_u64: u64,
    gen: SmallRng,
    weighted_dist: WeightedAliasIndex<i32>,
}

impl Random {
    fn new(seed: String, weights: Vec<i32>) -> Random {
        // sadly WeightedAliasIndex is initialized using a Vec. So im manually
        // checking for the correct size. I feel like there must be a better way
        assert_eq!(weights.len(), 4);

        let seed_u64 = hash(seed.as_bytes());

        Random {
            seed,
            seed_u64,
            gen: SmallRng::seed_from_u64(seed_u64),
            weighted_dist: WeightedAliasIndex::new(weights).unwrap(),
        }
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

    // fps control
    let minimum_frame_time = time::Duration::from_secs_f32(1. / TARGET_FPS as f32);

    loop {
        // framerate control
        let frame_start = time::Instant::now();
        editor.average_fps =
            (editor.average_fps * (1. - AVG_FPS_FACTOR)) + (get_fps() as f32 * AVG_FPS_FACTOR);

        // this value is only valid after calling define_egui()
        editor.canvas = None;

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

        wait_for_next_frame(frame_start, minimum_frame_time).await;
    }
}
