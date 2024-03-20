mod editor;
mod fps_control;
mod grid_render;
mod kernel;
mod map;
mod position;
mod random;
mod walker;
use crate::{
    editor::*, fps_control::*, grid_render::*, kernel::Kernel, map::*, position::*, random::*,
    walker::*,
};

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
    let init_config = GenerationConfig::new(
        3,
        5,
        0.5,
        0.2,
        vec![
            Position::new(250, 50),
            Position::new(250, 250),
            Position::new(50, 250),
            Position::new(50, 50),
        ],
        "iMilchshake".to_string(),
    );
    let mut editor = Editor::new(EditorPlayback::Paused, init_config);
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);
    let mut gen = Generator::new(&editor.config);

    let mut test = TestStruct::default();

    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();

        // walker logic TODO: move this into Generator struct
        if editor.playback.is_not_paused() {
            for _ in 0..STEPS_PER_FRAME {
                // check if walker has reached goal position
                if gen.walker.is_goal_reached() == Some(true) {
                    gen.walker
                        .next_waypoint(&editor.config)
                        .unwrap_or_else(|_| {
                            println!("pause due to error fetching next checkpoint");
                            editor.playback.pause();
                        });
                }

                // randomly mutate kernel
                gen.walker.mutate_kernel(&editor.config, &mut gen.rnd);

                // perform one greedy step
                if let Err(err) = gen.walker.probabilistic_step(&mut gen.map, &mut gen.rnd) {
                    println!("walker step failed: '{:}' - pausing...", err);
                    editor.playback.pause();
                }

                // walker did a step using SingleStep -> now pause
                if editor.playback == EditorPlayback::SingleStep {
                    editor.playback.pause();
                    break; // skip following steps for this frame!
                }
            }
        }

        editor.define_egui(&mut gen, &mut test);
        editor.set_cam(&gen.map);
        editor.handle_user_inputs(&gen.map);

        clear_background(WHITE);
        draw_grid_blocks(&gen.map.grid);
        draw_waypoints(&editor.config.waypoints);
        draw_walker(&gen.walker);
        draw_walker_kernel(&gen.walker, KernelType::Outer);
        draw_walker_kernel(&gen.walker, KernelType::Inner);

        egui_macroquad::draw();

        fps_ctrl.wait_for_next_frame().await;
    }
}
