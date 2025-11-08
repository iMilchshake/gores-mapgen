#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use clap::Parser;
use gores_mapgen::{
    args::EditorArgs, config::ThemeConfig, editor::*, generator::GenerationStatus, map::*,
    rendering::*,
};
use macroquad::prelude::{error, info, warn};
use macroquad::{color::*, miniquad, window::*};
use miniquad::conf::{Conf, Platform};

const DISABLE_VSYNC: bool = true;

fn window_conf() -> Conf {
    Conf {
        window_title: "gores-mapgen-editor".to_owned(),
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
    std::panic::set_hook(Box::new(|info| {
        if let Some(loc) = info.location() {
            error!("PANIC at {}:{}: {}", loc.file(), loc.line(), info);
        } else {
            error!("PANIC: {}", info);
        }
    }));

    // initialization
    let args = EditorArgs::parse();

    // simple_logger::SimpleLogger::new().init().unwrap();
    let mut editor = Editor::new("hard", "small_s_tight", ThemeConfig::default(), &args);

    // main loop for gui (and step-wise map generation)
    loop {
        editor.on_frame_start();

        // "auto generate": start generating next map right away
        if editor.playback_mode == PlaybackMode::Paused && editor.auto_generate {
            if editor.gen.status.is_finished() {
                editor.reset_generation(true, true);
            }
        }

        // "instant": perform maximum possible amount of generation steps
        let generation_steps = match editor.instant {
            true => usize::MAX,
            false => editor.steps_per_frame,
        };

        for _ in 0..generation_steps {
            if editor.playback_mode == PlaybackMode::Paused || editor.gen.walker.finished {
                break;
            }

            editor
                .gen
                .step(&editor.gen_config, true, &mut editor.debug_layers)
                .unwrap_or_else(|err| {
                    error!("Walker Step Failed: {:}", err);
                    editor.playback_mode = PlaybackMode::Paused;
                    editor.gen.status = GenerationStatus::Failed(format!("Walker failed: {}", err));
                });

            // walker did a step using SingleStep -> now pause
            if editor.playback_mode == PlaybackMode::SingleStep {
                editor.playback_mode = PlaybackMode::Paused;
            }
        }

        // this is called ONCE when walker just finished (status still Walking)
        if editor.gen.walker.finished && editor.gen.status == GenerationStatus::Walking {
            editor
                .gen
                .perform_all_post_processing(
                    &editor.gen_config,
                    &editor.thm_config,
                    &mut editor.debug_layers,
                    editor.verbose_post_process,
                    editor.prepare_export,
                )
                .unwrap_or_else(|err| {
                    if err == "Post-processing panicked" {
                        editor.playback_mode = PlaybackMode::Paused;
                        editor.auto_generate = false;
                    }
                    error!("Post Processing Failed: {:}", err);
                });

            // check status to handle success/failure
            if editor.retry_on_failure && editor.gen.status == GenerationStatus::Success {
                editor.retry_count = 0;
            }

            editor.playback_mode = PlaybackMode::Paused;
        }

        // handle retry on failure: in [0, N-1] -> retry | == N warn | > N dont do anything.
        if editor.retry_on_failure && editor.retry_count <= editor.max_retries {
            if let GenerationStatus::Failed(_) = editor.gen.status {
                if editor.retry_count < editor.max_retries {
                    editor.retry_count += 1;
                    info!(
                        "Retrying generation ({}/{})",
                        editor.retry_count,
                        editor.max_retries
                    );
                    editor.reset_generation(true, false);
                } else {
                    warn!(
                        "Max retries ({}) reached, stopping automatic retries",
                        editor.max_retries
                    );
                    editor.retry_count += 1; // we further increment by one signaling to stop
                }
            }
        }

        editor.define_egui();
        editor.update_cam();
        editor.handle_user_inputs();

        clear_background(WHITE);

        if editor.use_chunked_rendering {
            draw_chunked_grid(
                &editor.gen.map.grid,
                editor
                    .gen
                    .map
                    .chunk_edited
                    .as_ref()
                    .expect("chunk tracking not enabled"),
                editor.gen.map.chunk_size,
            );
        } else {
            draw_grid(&editor.gen.map.grid, blocktype_to_color);
        }

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
        next_frame().await; // submit our render calls to our screen
    }
}
