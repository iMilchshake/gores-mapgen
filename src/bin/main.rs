use gores_mapgen_rust::{editor::*, fps_control::*, grid_render::*, map::*, random::Random};

use macroquad::{color::*, miniquad, window::*};
use miniquad::conf::{Conf, Platform};

const DISABLE_VSYNC: bool = true;

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
    let mut editor = Editor::new(EditorPlayback::Paused, GenerationConfig::default());
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);
    let mut gen = Generator::new(&editor.config);

    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();

        if editor.config.auto_generate && gen.walker.finished {
            editor.playback = EditorPlayback::Playing;
            let rnd = Random::from_previous_rnd(&mut gen.rnd, editor.config.step_weights.clone());
            gen = Generator::new(&editor.config);
            gen.rnd = rnd;
        }

        // perform walker step
        for _ in 0..editor.steps_per_frame {
            if editor.playback == EditorPlayback::Paused || gen.walker.finished {
                break;
            }

            gen.step(&editor).unwrap_or_else(|err| {
                println!("Pause: {:}", err);
                editor.playback.pause();
            });

            // walker did a step using SingleStep -> now pause
            if editor.playback == EditorPlayback::SingleStep {
                editor.playback.pause();
            }
        }

        editor.define_egui(&mut gen);
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
