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
    let mut editor = Editor::new(GenerationConfig::default());
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);
    let mut seed = String::new();

    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();

        if editor.config.auto_generate && editor.gen.walker.finished {
            // let rnd =
            //     Random::from_previous_rnd(&mut editor.gen.rnd, editor.config.step_weights.clone());
            // editor.gen = Generator::new(&editor.config);
            // editor.gen.rnd = rnd;
            editor.set_setup();
            editor.set_playing();
        }

        // perform walker step
        for _ in 0..editor.steps_per_frame {
            if editor.is_paused() || editor.gen.walker.finished {
                break;
            }

            editor.gen.step(&editor.config).unwrap_or_else(|err| {
                println!("Pause: {:}", err);
                editor.set_stopped();
            });

            // walker did a step using SingleStep -> now pause
            if editor.is_single_setp() {
                editor.set_stopped();
            }
        }

        editor.define_egui();
        editor.set_cam();
        editor.handle_user_inputs();

        clear_background(WHITE);
        draw_grid_blocks(&editor.gen.map.grid);
        draw_waypoints(&editor.config.waypoints);
        draw_walker(&editor.gen.walker);
        draw_walker_kernel(&editor.gen.walker, KernelType::Outer);
        draw_walker_kernel(&editor.gen.walker, KernelType::Inner);

        egui_macroquad::draw();

        fps_ctrl.wait_for_next_frame().await;
    }
}
