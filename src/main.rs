#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use clap::Parser;
use gores_mapgen::{
    args::Args,
    config::{GenerationConfig, MapConfig},
    editor::*,
    fps_control::*,
    map::*,
    rendering::*,
};
use macroquad::{color::*, miniquad, window::*};
use miniquad::conf::{Conf, Platform};
use simple_logger::SimpleLogger;
use std::panic::{self, AssertUnwindSafe};

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
    // initialization
    let args = Args::parse();
    SimpleLogger::new().init().unwrap();
    let mut editor = Editor::new(
        GenerationConfig::get_initial_gen_config(),
        MapConfig::get_initial_config(),
        &args,
    );
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);

    // main loop for gui (and step-wise map generation)
    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();

        // "auto generate": start generating next map right away
        if editor.is_paused() && editor.auto_generate {
            editor.set_playing();
        }

        // "instant": perform maximum possible amount of generation steps
        let generation_steps = match editor.instant {
            true => usize::MAX,
            false => editor.steps_per_frame,
        };

        for _ in 0..generation_steps {
            if editor.is_paused() || editor.gen.walker.finished {
                break;
            }

            editor
                .gen
                .step(&editor.gen_config, true, &mut editor.debug_layers)
                .unwrap_or_else(|err| {
                    println!("Walker Step Failed: {:}", err);
                    editor.set_setup();

                    if editor.retry_on_failure {
                        editor.set_playing();
                    }
                });

            // walker did a step using SingleStep -> now pause
            if editor.is_single_setp() {
                editor.set_stopped();
            }
        }

        // this is called ONCE after map was generated
        // TODO: handling successfull generation via 'setup' state is kinda stupid, i should
        // just add a new state variable for this, in the generator?
        if editor.gen.walker.finished && !editor.is_setup() {
            // kinda crappy, but ensure that even a panic doesnt crash the program
            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                editor
                    .gen
                    .perform_all_post_processing(
                        &editor.gen_config,
                        &editor.thm_config,
                        &mut editor.debug_layers,
                    )
                    .unwrap_or_else(|err| {
                        println!("Post Processing Failed: {:}", err);
                    });
            }));

            // switch into setup mode for next map
            editor.set_setup();
        }

        editor.define_egui();
        editor.update_cam();
        editor.handle_user_inputs();

        clear_background(WHITE);
        draw_chunked_grid(
            &editor.gen.map.grid,
            &editor.gen.map.chunk_edited,
            editor.gen.map.chunk_size,
        );
        draw_font_layer(&editor.gen.map.font_layer);

        // draw debug layers
        if let Some(ref mut debug_layers) = editor.debug_layers {
            for (layer_name, debug_layer) in debug_layers.bool_layers.iter() {
                if !debug_layers.active_layers.get(layer_name).unwrap() {
                    continue;
                }

                draw_bool_grid(&debug_layer.grid, &debug_layer.color, &debug_layer.outline)
            }

            for (layer_name, debug_layer) in debug_layers.float_layers.iter() {
                if !debug_layers.active_layers.get(layer_name).unwrap() {
                    continue;
                }

                draw_opt_float_grid(
                    &debug_layer.grid,
                    &debug_layer.color_min,
                    &debug_layer.color_max,
                );
            }
        }

        // TODO: group in some "debug" visualization call
        draw_walker_kernel(&editor.gen.walker, KernelType::Outer);
        draw_walker_kernel(&editor.gen.walker, KernelType::Inner);
        draw_walker(&editor.gen.walker);
        draw_waypoints(&editor.gen.walker, colors::BLUE, colors::RED);

        // TODO: move to key input function!
        if macroquad::input::is_key_down(miniquad::KeyCode::D) {
            draw_mouse_map_cell_pos(&editor.map_cam);
        }

        // editor.map_cam.draw_cam_debug();

        egui_macroquad::draw();
        fps_ctrl.wait_for_next_frame().await;
    }
}
