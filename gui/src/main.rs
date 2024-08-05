pub mod editor;
pub mod fps_control;
pub mod gui;
pub mod rendering;

use crate::{editor::*, fps_control::*, rendering::*};
use clap::{crate_version, Parser};
use macroquad::{color::*, miniquad, window::*};
use mapgen_core::map::*;
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

    /// name of initial generation config
    #[arg(short, default_value = "hardV2")]
    gen_config: String,

    /// name of initial map config
    #[arg(short, default_value = "small_s")]
    map_config: String,
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

    let mut editor = Editor::new(args.gen_config, args.map_config);
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);

    if args.testing {
        editor.instant = true;
        editor.fixed_seed = true;
        editor.auto_generate = true;
        editor.edit_gen_config = true;
    }

    if let Some(config_name) = args.config {
        if editor.gen_configs.contains_key(&config_name) {
            editor.current_gen_config = config_name;
        }
    }

    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();
        editor.define_egui();

        // optionally, start generating next map right away
        if editor.is_paused() && editor.auto_generate {
            editor.set_playing();
        }

        // perform walker steps
        let steps = match editor.instant {
            true => usize::max_value(),
            false => editor.steps_per_frame,
        };

        if editor.gen.as_ref().is_some() {
            for _ in 0..steps {
                if editor.is_paused() || editor.gen.as_ref().unwrap().walker.finished {
                    break;
                }

                editor.gen.as_mut().unwrap().step().unwrap_or_else(|err| {
                    println!("Walker Step Failed: {:}", err);
                    editor.set_setup();
                });

                // walker did a step using SingleStep -> now pause
                if editor.is_single_setp() {
                    editor.set_stopped();
                }
            }

            // this is called ONCE after map was generated
            if editor.gen.as_ref().unwrap().walker.finished && !editor.is_setup() {
                // kinda crappy, but ensure that even a panic doesnt crash the program
                let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                    editor
                        .gen
                        .as_mut()
                        .unwrap()
                        .post_processing()
                        .unwrap_or_else(|err| {
                            println!("Post Processing Failed: {:}", err);
                        });
                }));

                // switch into setup mode for next map
                editor.set_setup();
            }

            editor.set_cam();
            editor.handle_user_inputs();

            clear_background(WHITE);

            draw_chunked_grid(
                &editor.gen.as_ref().unwrap().map.grid,
                &editor.gen.as_ref().unwrap().map.chunks_edited,
                editor.gen.as_ref().unwrap().map.chunk_size,
            );
            draw_walker_kernel(&editor.gen.as_ref().unwrap().walker, KernelType::Outer);
            draw_walker_kernel(&editor.gen.as_ref().unwrap().walker, KernelType::Inner);
            draw_walker(&editor.gen.as_ref().unwrap().walker);
            draw_waypoints(&editor.cur_map_config_mut().waypoints);
        }

        egui_macroquad::draw();
        fps_ctrl.wait_for_next_frame().await;
    }
}
