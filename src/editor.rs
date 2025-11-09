const STEPS_PER_FRAME: usize = 50;

use crate::{
    args::EditorArgs,
    config::{GenerationConfig, MapConfig, ThemeConfig},
    debug::DebugLayers,
    file_io::{FileDialog, FileDialogResult, FileOperationType},
    generator::Generator,
    gui,
    map_camera::MapCamera,
    random::Seed,
};
use egui::{epaint::Shadow, Color32, Frame, Margin};

use macroquad::prelude::{error, info, warn};
use macroquad::time::get_fps;
use macroquad::{camera::Camera2D, input::is_mouse_button_pressed};
use macroquad::{
    input::{
        is_key_down, is_key_pressed, is_mouse_button_down, mouse_delta_position, mouse_position,
        mouse_wheel, KeyCode, MouseButton,
    },
    window::screen_height,
};

const AVG_FPS_FACTOR: f32 = 0.025; // how much current fps is weighted into the rolling average

pub fn window_frame() -> Frame {
    Frame {
        fill: Color32::from_gray(0),
        inner_margin: Margin::same(5),
        shadow: Shadow::NONE,
        ..Default::default()
    }
}

#[derive(PartialEq, Debug)]
pub enum PlaybackMode {
    /// run generation steps continuously
    Playing,
    /// stopped, not running
    Paused,
    /// run one step then auto-pause
    SingleStep,
}

#[derive(Debug, PartialEq)]
pub enum SeedType {
    U64,
    STRING,
    BASE64,
}

pub struct Editor {
    pub playback_mode: PlaybackMode,
    // TODO: shouldnt these be part of generator??
    pub gen_config: GenerationConfig,
    pub map_config: MapConfig,
    pub thm_config: ThemeConfig,

    pub gen_configs: Vec<GenerationConfig>,
    pub map_configs: Vec<MapConfig>,
    pub debug_layers: Option<DebugLayers>,
    pub average_fps: f32,
    pub gen: Generator,

    pub user_seed: Seed,
    pub user_seed_str: String,
    pub seed_input_type: SeedType,

    /// keeps track of camera for map visualization
    pub map_cam: MapCamera,
    pub canvas: Option<egui::Rect>,
    pub egui_wants_mouse: Option<bool>,
    pub show_debug_layers: bool,
    pub show_debug_widget: bool,
    pub show_theme_widget: bool,

    /// whether to show the GenerationConfig settings
    pub edit_gen_config: bool,

    /// whether to show the GenerationConfig settings
    pub edit_map_config: bool,

    /// whether to skip initialization of debug layers
    pub disable_debug_layers: bool,

    /// how many generation steps are performed in one frame render
    pub steps_per_frame: usize,

    /// whether to perform entire map generation in one frame render
    /// if yes, steps_per_frame is ignored
    pub instant: bool,

    /// whether to keep generating after a map is generated
    pub auto_generate: bool,

    /// whether to keep using the same seed for next generations
    pub fixed_seed: bool,

    /// whether to keep using the same seed for next generations
    pub retry_on_failure: bool,

    /// maximum number of retries when generation fails
    pub max_retries: usize,

    /// current number of retries for the current generation attempt
    pub retry_count: usize,

    /// Whether to perform map export preparation such as generation of noise layers.
    /// This is computational expensive and should only be done for debugging purposes,
    /// or if the map is intended to be exported.
    pub prepare_export: bool,

    pub verbose_post_process: bool,

    /// very good performance optimization, only disable for debugging
    pub use_chunked_rendering: bool,

    /// Whether to flip maps after generation
    pub use_map_flip: bool,

    /// File dialogs (platform-agnostic)
    pub save_map_dialog: FileDialog,
    pub load_gen_config_dialog: FileDialog,
    pub load_map_config_dialog: FileDialog,
    pub save_gen_config_dialog: FileDialog,
    pub save_map_config_dialog: FileDialog,
}

impl Editor {
    pub fn new(
        init_gen_config_name: &str,
        init_map_config_name: &str,
        thm_config: ThemeConfig,
        args: &EditorArgs,
    ) -> Editor {
        let gen_configs: Vec<GenerationConfig> = GenerationConfig::get_all_configs();
        let map_configs: Vec<MapConfig> = MapConfig::get_all_configs();

        let init_gen_config = gen_configs
            .iter()
            .find(|c| c.name == init_gen_config_name)
            .unwrap();
        let init_map_config = map_configs
            .iter()
            .find(|c| c.name == init_map_config_name)
            .unwrap();

        let gen = Generator::new(
            init_gen_config,
            init_map_config,
            &thm_config,
            Seed::from_u64(0),
            true,
        );

        let user_seed = if let Some(ref seed_base64) = args.init_seed {
            Seed::from_base64(seed_base64).expect("no valid base64 seed")
        } else {
            Seed::from_string(&"iMilchshake".to_string(), &SeedType::STRING).unwrap()
        };

        let mut editor = Editor {
            playback_mode: PlaybackMode::Paused,
            debug_layers: None,
            disable_debug_layers: args.disable_debug,
            gen_config: init_gen_config.clone(),
            map_config: init_map_config.clone(),
            gen_configs,
            map_configs,
            canvas: None,
            egui_wants_mouse: None,
            average_fps: 0.0,
            map_cam: MapCamera::default(),
            thm_config: ThemeConfig::default(),
            steps_per_frame: STEPS_PER_FRAME,
            gen,
            user_seed,
            user_seed_str: String::new(),
            seed_input_type: SeedType::BASE64,
            instant: args.instant,
            auto_generate: args.auto_generation,
            fixed_seed: args.fixed_seed,
            edit_gen_config: false,
            edit_map_config: false,
            retry_on_failure: false,
            max_retries: args.max_retries,
            retry_count: 0,
            show_theme_widget: false,
            show_debug_widget: false,
            show_debug_layers: false,
            prepare_export: false,
            verbose_post_process: false,
            use_chunked_rendering: true,
            use_map_flip: false,
            save_map_dialog: FileDialog::new(FileOperationType::SaveMap),
            load_gen_config_dialog: FileDialog::new(FileOperationType::LoadGenerationConfig)
                .with_filter(crate::file_io::FileFilter::json()),
            load_map_config_dialog: FileDialog::new(FileOperationType::LoadMapConfig)
                .with_filter(crate::file_io::FileFilter::json()),
            save_gen_config_dialog: FileDialog::new(FileOperationType::SaveGenerationConfig),
            save_map_config_dialog: FileDialog::new(FileOperationType::SaveMapConfig),
        };

        // initialize debug layers
        if !editor.disable_debug_layers {
            editor.initialize_debug_layers();
            if let Some(ref enable_layers) = args.enable_layers {
                for layer_name in enable_layers {
                    let layer = editor
                        .debug_layers
                        .as_mut()
                        .unwrap()
                        .active_layers
                        .get_mut(layer_name.as_str());

                    *layer.unwrap_or_else(|| panic!("layer name '{}' doesnt exist", layer_name)) =
                        true;
                }
            }
        }

        // load initial gen/map configs
        if let Some(config_name) = &args.gen_config {
            if editor.load_gen_config(config_name).is_err() {
                warn!("Coulnt load gen config {}", config_name);
            }
        }
        if let Some(config_name) = &args.map_config {
            if editor.load_map_config(config_name).is_err() {
                warn!("Coulnt load map config {}", config_name);
            }
        }

        if args.generate {
            editor.reset_generation(true, true);
        }

        editor
    }

    pub fn initialize_debug_layers(&mut self) {
        assert!(
            !self.disable_debug_layers,
            "debug layers already initialized"
        );

        // if possible, get currently active layers for re-using
        let previously_active_layers = self.debug_layers.take().map(|d| d.active_layers);

        self.debug_layers = Some(DebugLayers::new(
            (self.gen.map.width, self.gen.map.height),
            0.5,
            previously_active_layers,
        ));
    }

    pub fn on_frame_start(&mut self) {
        // framerate control
        self.average_fps =
            (self.average_fps * (1. - AVG_FPS_FACTOR)) + (get_fps() as f32 * AVG_FPS_FACTOR);

        // this value is only valid for each frame after calling define_egui()
        self.canvas = None;
    }

    pub fn define_egui(&mut self) {
        egui_macroquad::ui(|egui_ctx| {
            gui::sidebar(egui_ctx, self);
            gui::menu(egui_ctx, self);

            if self.show_debug_widget {
                gui::debug_window(egui_ctx, self);
            }

            // TODO: move to key input function!
            if self.show_debug_layers || macroquad::input::is_key_down(KeyCode::D) {
                gui::debug_layers_widget(egui_ctx, self);
            }

            if self.show_theme_widget {
                gui::theme_widget(egui_ctx, self);
            }

            // Update file dialogs
            self.save_map_dialog.update(egui_ctx);
            self.load_gen_config_dialog.update(egui_ctx);
            self.load_map_config_dialog.update(egui_ctx);
            self.save_gen_config_dialog.update(egui_ctx);
            self.save_map_config_dialog.update(egui_ctx);

            // Handle file dialog results
            self.handle_save_map();
            gui::handle_config_dialogs(self);

            // store remaining space for macroquad drawing
            self.canvas = Some(egui_ctx.available_rect());
            self.egui_wants_mouse = Some(egui_ctx.wants_pointer_input());
        });
    }

    fn initialize_generator(&mut self) {
        if !self.fixed_seed {
            self.user_seed = Seed::from_random(&mut self.gen.rnd);
        }

        self.gen = Generator::new(
            &self.gen_config,
            &self.map_config,
            &self.thm_config,
            self.user_seed.clone(),
            true,
        );

        // reset debug layers, if used
        if !self.disable_debug_layers {
            self.initialize_debug_layers();
        }
    }

    fn mouse_in_viewport(cam: &Camera2D) -> bool {
        let (mouse_x, mut mouse_y) = mouse_position();
        mouse_y = screen_height() - mouse_y; // invert mouse_y, as cameras are flipped D:

        // this assumes that the viewport is bottom_left aligned and starts at (0, 0)!

        0.0 <= mouse_x
            && mouse_x <= cam.viewport.unwrap().2 as f32
            && 0.0 <= mouse_y
            && mouse_y <= cam.viewport.unwrap().3 as f32
    }

    pub fn update_cam(&mut self) {
        self.map_cam
            .update_map_size(self.gen.map.width, self.gen.map.height);
        self.map_cam
            .update_viewport_from_egui_rect(&self.canvas.unwrap());
        self.map_cam.update_macroquad_cam();
    }

    pub fn handle_save_map(&mut self) {
        if let Some(result) = self.save_map_dialog.take_result() {
            match result {
                FileDialogResult::SavePath(path) => {
                    // perform export preparation, if not enabled in editor
                    if !self.prepare_export {
                        self.gen
                            .prepare_export(&self.thm_config, &mut self.debug_layers, false);
                    }
                    let path_buf = std::path::PathBuf::from(&path);
                    self.gen.map.export(&path_buf);
                }
                FileDialogResult::Cancelled => {
                    info!("Map save cancelled");
                }
                FileDialogResult::Error(err) => {
                    error!("Failed to save map: {}", err);
                }
                _ => {}
            }
        }
    }

    pub fn handle_user_inputs(&mut self) {
        is_key_pressed(KeyCode::LeftShift);

        if is_key_pressed(KeyCode::Space) {
            if self.gen.status.is_finished() {
                self.retry_on_failure = is_key_down(KeyCode::LeftShift);
                self.reset_generation(true, true);
            } else {
                self.playback_mode = PlaybackMode::Playing; // just resume
            }
        }

        if is_key_pressed(KeyCode::R) {
            self.map_cam.reset();
        }

        if mouse_wheel().1.abs() > 0.0 {
            self.map_cam.zoom(mouse_wheel().1.is_sign_positive());
        }

        let egui_wants_mouse = self.egui_wants_mouse.unwrap();

        // handle panning
        let delta = mouse_delta_position();
        if !egui_wants_mouse
            && is_mouse_button_down(MouseButton::Left)
            && Editor::mouse_in_viewport(self.map_cam.get_macroquad_cam())
            && !is_mouse_button_pressed(MouseButton::Left)
        {
            self.map_cam.shift(delta);
        }
    }

    pub fn load_gen_config(&mut self, config_name: &str) -> Result<(), &'static str> {
        if let Some(config) = self.gen_configs.iter().find(|&c| c.name == config_name) {
            self.gen_config = config.clone();
            Ok(())
        } else {
            Err("Generation config not found!")
        }
    }

    pub fn load_map_config(&mut self, config_name: &str) -> Result<(), &'static str> {
        if let Some(config) = self.map_configs.iter().find(|&c| c.name == config_name) {
            self.map_config = config.clone();
            Ok(())
        } else {
            Err("Generation config not found!")
        }
    }

    /// Utility function to cleanly start a new generation.
    pub fn reset_generation(&mut self, start: bool, reset_retry: bool) {
        self.initialize_generator();
        if reset_retry {
            self.retry_count = 0;
        }
        self.playback_mode = match start {
            true => PlaybackMode::Playing,
            false => PlaybackMode::Paused,
        };
    }
}
