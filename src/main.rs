mod editor;
mod fps_control;
mod grid_render;
mod map;
mod position;
mod random;
mod walker;
use crate::{editor::*, fps_control::*, grid_render::*, map::*, position::*, random::*, walker::*};

use macroquad::{color::*, miniquad, window::*};
use miniquad::conf::{Conf, Platform};

const DISABLE_VSYNC: bool = true;
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

#[macroquad::main(window_conf)]
async fn main() {
    let mut rnd = Random::new("iMilchshake".to_string(), vec![4, 3, 2, 1]);

    let mut editor = Editor::new(EditorPlayback::Paused);

    let mut map = Map::new(300, 300, BlockType::Hookable);
    let kernel = Kernel::new(8, 0.9);
    let waypoints: Vec<Position> = vec![
        Position::new(250, 50),
        Position::new(250, 250),
        Position::new(50, 250),
        Position::new(50, 50),
    ];
    let mut walker = CuteWalker::new(Position::new(50, 50), waypoints, kernel);
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);

    loop {
        fps_ctrl.on_frame_start();
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
                // if rnd.gen.gen_bool(0.1) {
                //     let size = rnd.gen.gen_range(3..=7);
                //     let circularity = rnd.gen.gen_range(0.0..=1.0);
                //     walker.kernel = Kernel::new(size, circularity);
                // }

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

        fps_ctrl.wait_for_next_frame().await;
    }
}
