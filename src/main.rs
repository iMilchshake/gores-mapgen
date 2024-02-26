mod grid_render;
mod map;
mod position;
mod walker;
use grid_render::*;
use map::*;
use position::*;
use walker::*;

use egui::{epaint::Shadow, Color32, Frame, Label, Margin, Rect};
use macroquad::prelude::*;
use miniquad::conf::Platform;
use std::{
    default,
    time::{self, Duration, Instant},
};

const TARGET_FPS: usize = 60;
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

struct State {
    map_gen_state: MapGenState,
    editor_state: EditorState,
}

struct MapGenState {
    walker: CuteWalker,
}

struct EditorState {
    pause: bool,
    allowed_step: usize,
    canvas: Option<Rect>,
    average_fps: f32,
}

impl EditorState {
    fn new(initial_pause: bool) -> EditorState {
        EditorState {
            pause: initial_pause,
            allowed_step: 0,
            canvas: None,
            average_fps: TARGET_FPS as f32,
        }
    }
}

impl MapGenState {
    fn new(initial_walker_pos: Position) -> MapGenState {
        MapGenState {
            walker: CuteWalker::new(initial_walker_pos),
        }
    }
}

fn define_egui(editor_state: &mut EditorState, map_gen_state: &MapGenState) {
    // define egui
    egui_macroquad::ui(|egui_ctx| {
        egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
            ui.label("hello world");

            // toggle pause
            if ui.button("toggle").clicked() {
                editor_state.pause = !editor_state.pause;
            }

            // pause, allow single step
            if ui.button("single").clicked() {
                editor_state.pause = true;
                editor_state.allowed_step += 1;
            }
            ui.separator();
        });

        egui::Window::new("DEBUG")
            .frame(window_frame())
            .show(egui_ctx, |ui| {
                ui.add(Label::new(format!("fps: {:}", get_fps().to_string())));
                ui.add(Label::new(format!(
                    "avg: {:}",
                    editor_state.average_fps.round() as usize
                )));
                ui.add(Label::new(format!(
                    "allowed_step: {:}",
                    editor_state.allowed_step.to_string()
                )));
                ui.add(Label::new(format!("{:?}", map_gen_state.walker)));
                // ui.add(Label::new(format!("{:?}", editor_state.curr_goal)));
            });

        // store remaining space for macroquad drawing TODO: i could set this to None before this
        editor_state.canvas = Some(egui_ctx.available_rect());
    });
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut map = Map::new(100, 100, BlockType::Empty);
    let kernel = Kernel::new(8, 0.9);

    let mut editor_state = EditorState::new(false);
    let mut map_gen_state = MapGenState::new(Position::new(50, 25));

    // setup waypoints
    let goals: Vec<Position> = vec![
        Position::new(99, 33),
        Position::new(0, 33),
        Position::new(50, 33),
        Position::new(50, 100),
    ];
    let mut goals_iter = goals.iter();
    let mut curr_goal = goals_iter.next().unwrap();

    // fps control
    let minimum_frame_time = time::Duration::from_secs_f32(1. / TARGET_FPS as f32);

    loop {
        let frame_start = time::Instant::now();
        editor_state.average_fps = (editor_state.average_fps * (1. - AVG_FPS_FACTOR))
            + (get_fps() as f32 * AVG_FPS_FACTOR);

        // if goal is reached
        if map_gen_state.walker.pos.eq(&curr_goal) {
            if let Some(next_goal) = goals_iter.next() {
                curr_goal = next_goal;
            } else {
                println!("pause due to reaching last checkpoint");
                editor_state.pause = true;
            }
        }

        if !editor_state.pause {
            editor_state.allowed_step += 1;
        }

        if map_gen_state.walker.steps < editor_state.allowed_step {
            // get greedy shift towards goal
            let shift = map_gen_state.walker.pos.get_greedy_dir(&curr_goal);

            // apply that shift
            map_gen_state
                .walker
                .shift_pos(shift, &map)
                .unwrap_or_else(|_| {
                    println!("walker exceeded bounds, pausing...");
                    editor_state.pause = true;
                    editor_state.allowed_step -= 1;
                });

            // remove blocks using a kernel at current position
            map.update(&map_gen_state.walker.pos, &kernel, BlockType::Filled)
                .ok();
        }

        define_egui(&mut editor_state, &map_gen_state);

        let display_factor = f32::min(
            editor_state.canvas.unwrap().width() / map.width as f32,
            editor_state.canvas.unwrap().height() / map.height as f32,
        );

        clear_background(WHITE);
        draw_grid_blocks(&mut map.grid, display_factor, vec2(0.0, 0.0));
        draw_walker(&map_gen_state.walker, display_factor, vec2(0.0, 0.0));

        // draw egui on top of macroquad
        egui_macroquad::draw();

        wait_for_next_frame(frame_start, minimum_frame_time).await;
    }
}
