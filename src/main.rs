#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use clap::{crate_version, Parser};
use gores_mapgen_rust::{
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

#[derive(Parser, Debug)]
#[command(name = "Random Gores Map Generator")]
#[command(version = crate_version!())]
#[command(about = "Visual editor for generating maps and customizing the generators presets", long_about = None)]
struct Args {
    /// select initial generation config
    config: Option<String>,

    /// enable instant, auto generate and fixed seed
    #[arg(short, long)]
    testing: bool,
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

#[macroquad::main(window_conf)]
async fn main() {
    let args = Args::parse();
    SimpleLogger::new().init().unwrap();

    let mut editor = Editor::new(
        GenerationConfig::get_initial_config(true),
        MapConfig::get_initial_config(),
    );
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);

    if args.testing {
        editor.instant = true;
        editor.fixed_seed = true;
        editor.auto_generate = true;
        editor.edit_gen_config = true;
    }

    if let Some(config_name) = args.config {
        if editor.init_gen_configs.contains_key(&config_name) {
            editor.gen_config = editor.init_gen_configs.get(&config_name).unwrap().clone();
        }
    }

    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();

        // optionally, start generating next map right away
        if editor.is_paused() && editor.auto_generate {
            editor.set_playing();
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

            editor.gen.step(&editor.gen_config).unwrap_or_else(|err| {
                println!("Walker Step Failed: {:}", err);
                editor.set_setup();
            });

            // walker did a step using SingleStep -> now pause
            if editor.is_single_setp() {
                editor.set_stopped();
            }
        }

        // this is called ONCE after map was generated
        if editor.gen.walker.finished && !editor.is_setup() {
            // kinda crappy, but ensure that even a panic doesnt crash the program
            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                editor
                    .gen
                    .perform_all_post_processing(&editor.gen_config)
                    .unwrap_or_else(|err| {
                        println!("Post Processing Failed: {:}", err);
                    });
            }));

            // switch into setup mode for next map
            editor.set_setup();
        }

        editor.define_egui();
        editor.set_cam();
        editor.handle_user_inputs();

        clear_background(WHITE);
        // draw_grid_blocks(&editor.gen.map.grid);
        draw_chunked_grid(
            &editor.gen.map.grid,
            &editor.gen.map.chunk_edited,
            editor.gen.map.chunk_size,
        );

        // TODO: group in some "debug" visualization call
        draw_walker_kernel(&editor.gen.walker, KernelType::Outer);
        draw_walker_kernel(&editor.gen.walker, KernelType::Inner);
        draw_walker(&editor.gen.walker);
        draw_waypoints(&editor.gen.walker.waypoints, colors::BLUE);
        draw_waypoints(&editor.map_config.waypoints, colors::RED);

        // draw debug layers
        for (layer_name, debug_layer) in editor.gen.debug_layers.iter() {
            if *editor.visualize_debug_layers.get(layer_name).unwrap() {
                draw_bool_grid(&debug_layer.grid, &debug_layer.color, &debug_layer.outline)
            }
        }

        egui_macroquad::draw();

        fps_ctrl.wait_for_next_frame().await;
    }
}
