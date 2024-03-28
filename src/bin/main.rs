use gores_mapgen_rust::{editor::*, fps_control::*, grid_render::*, map::*};

use macroquad::{color::*, miniquad, window::*};
use miniquad::conf::{Conf, Platform};
use ndarray::Array2;

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
    let mut fps_ctrl = FPSControl::new(); //.with_max_fps(60);

    let mut edge_bugs: Option<Array2<bool>> = None;

    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();

        // this is called ONCE after map was generated
        if editor.gen.walker.finished && !editor.is_setup() {
            // perform post processing
            edge_bugs = Some(editor.gen.fix_edge_bugs());

            // switch into setup mode for next map
            editor.set_setup();

            // optionally, start generating next map right away
            if editor.config.auto_generate {
                editor.set_playing();
            }
        }

        // perform walker step
        let steps = match editor.instant {
            true => usize::max_value(),
            false => editor.steps_per_frame,
        };

        for _ in 0..steps {
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
        // draw_grid_blocks(&editor.gen.map.grid);
        draw_waypoints(&editor.config.waypoints);
        draw_chunked_grid(
            &editor.gen.map.grid,
            &editor.gen.map.chunk_edited,
            editor.gen.map.chunk_size,
        );
        draw_walker(&editor.gen.walker);
        draw_walker_kernel(&editor.gen.walker, KernelType::Outer);
        draw_walker_kernel(&editor.gen.walker, KernelType::Inner);

        if let Some(edge_bugs) = &edge_bugs {
            draw_bool_grid(edge_bugs, Color::new(1.0, 0.0, 0.0, 0.1));
        }

        egui_macroquad::draw();

        fps_ctrl.wait_for_next_frame().await;
    }
}
