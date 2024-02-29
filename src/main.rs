mod grid_render;
mod map;
mod position;
mod walker;
use grid_render::*;
use map::*;
use position::*;
use walker::*;

use egui::{epaint::Shadow, Color32, Frame, Label, Margin, Rect};
use macroquad::{
    color::*,
    math::{vec2, Vec2},
    miniquad,
    time::get_fps,
    window::*,
};
use miniquad::conf::{Conf, Platform};
use std::time::{self, Duration, Instant};

use rand::{rngs::SmallRng, RngCore};
use rand::{Rng, SeedableRng};
use seahash::hash;

const TARGET_FPS: usize = 9999;
const DISABLE_VSYNC: bool = true;
const AVG_FPS_FACTOR: f32 = 0.25; // how much current fps is weighted into the rolling average

fn window_frame() -> Frame {
    Frame {
        fill: Color32::from_gray(0),
        inner_margin: Margin::same(5.0),
        shadow: Shadow::NONE,
        ..Default::default()
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
#[derive(Debug)]
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

#[derive(PartialEq, Debug)]
enum EditorPlayback {
    Paused,
    SingleStep,
    Playing,
}

impl EditorPlayback {
    fn not_paused(&self) -> bool {
        match self {
            EditorPlayback::Paused => false,
            EditorPlayback::Playing | EditorPlayback::SingleStep => true,
        }
    }

    fn toggle(&mut self) {
        *self = match self {
            EditorPlayback::Paused => EditorPlayback::Playing,
            EditorPlayback::Playing | EditorPlayback::SingleStep => EditorPlayback::Paused,
        };
    }
}

struct State {
    mapgen: MapGeneration,
    editor: Editor,
}

struct MapGeneration {
    walker: CuteWalker,
}

struct Editor {
    playback: EditorPlayback,
    canvas: Option<Rect>,
    average_fps: f32,
}

impl Editor {
    fn new(initial_playback: EditorPlayback) -> Editor {
        Editor {
            playback: initial_playback,
            canvas: None,
            average_fps: TARGET_FPS as f32,
        }
    }

    fn get_display_factor(&self, map: &Map) -> Option<f32> {
        self.canvas.map(|canvas| {
            f32::min(
                canvas.width() / map.width as f32,
                canvas.height() / map.height as f32,
            )
        })
    }

    fn define_egui(&mut self, mapgen: &MapGeneration) {
        // define egui
        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label("hello world");

                // toggle pause
                if ui.button("toggle").clicked() {
                    self.playback.toggle();
                }

                // pause, allow single step
                if ui.button("single").clicked() {
                    self.playback = EditorPlayback::SingleStep;
                }
                ui.separator();
            });

            egui::Window::new("DEBUG")
                .frame(window_frame())
                .show(egui_ctx, |ui| {
                    ui.add(Label::new(format!("fps: {:}", get_fps())));
                    ui.add(Label::new(format!(
                        "avg: {:}",
                        self.average_fps.round() as usize
                    )));
                    ui.add(Label::new(format!("{:?}", mapgen.walker)));
                    ui.add(Label::new(format!("{:?}", self.playback)));
                    // ui.add(Label::new(format!("{:?}", editor.curr_goal)));
                });

            // store remaining space for macroquad drawing
            self.canvas = Some(egui_ctx.available_rect());
        });
    }
}

// TODO: if i keep adding everting to the walker, it might
// make more sense to just use one struct lol, but i believe
// all the generation config parameters will end up in here
impl MapGeneration {
    fn new(walker: CuteWalker) -> MapGeneration {
        MapGeneration { walker }
    }
}

struct Random {
    seed: String,
    seed_u64: u64,
    gen: SmallRng,
}

impl Random {
    fn new(seed: String) -> Random {
        let seed_u64 = hash(seed.as_bytes());
        Random {
            seed,
            seed_u64,
            gen: SmallRng::seed_from_u64(seed_u64),
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut rnd = Random::new("iMilchshake".to_string());
    let mut editor = Editor::new(EditorPlayback::Playing);

    let mut map = Map::new(100, 100, BlockType::Empty);
    let kernel = Kernel::new(8, 0.9);
    let waypoints: Vec<Position> = vec![
        Position::new(100, 10),
        Position::new(10, 95),
        Position::new(95, 95),
        Position::new(95, 10),
    ];
    let mut mapgen = MapGeneration::new(CuteWalker::new(Position::new(10, 10), waypoints, kernel));

    // fps control
    let minimum_frame_time = time::Duration::from_secs_f32(1. / TARGET_FPS as f32);

    loop {
        // framerate control
        let frame_start = time::Instant::now();
        editor.average_fps =
            (editor.average_fps * (1. - AVG_FPS_FACTOR)) + (get_fps() as f32 * AVG_FPS_FACTOR);

        // this value is only valid after calling define_egui()
        editor.canvas = None;

        // if goal is reached
        if mapgen.walker.pos.eq(&mapgen.walker.curr_goal) {
            mapgen.walker.next_waypoint().unwrap_or_else(|_| {
                println!("pause due to reaching last checkpoint");
                editor.playback = EditorPlayback::Paused;
            });
        }

        if editor.playback.not_paused() {
            // randomly mutate kernel
            let size = rnd.gen.gen_range(1..10);
            let circularity = rnd.gen.gen_range(0.0..=1.0);
            mapgen.walker.kernel = Kernel::new(size, circularity);

            // perform one greedy step
            if let Err(err) = mapgen.walker.greedy_step(&mut map) {
                println!("greedy step failed: '{:}' - pausing...", err);
                editor.playback = EditorPlayback::Paused;
            }

            // walker did a step using SingleStep -> now pause
            if editor.playback == EditorPlayback::SingleStep {
                editor.playback = EditorPlayback::Paused;
            }
        }

        editor.define_egui(&mapgen);

        clear_background(WHITE);
        let display_factor = editor
            .get_display_factor(&map)
            .expect("should be set after define_egui call");
        draw_grid_blocks(&map.grid, display_factor, vec2(0.0, 0.0));

        draw_walker(&mapgen.walker, display_factor, vec2(0.0, 0.0));
        egui_macroquad::draw();

        wait_for_next_frame(frame_start, minimum_frame_time).await;
    }
}
