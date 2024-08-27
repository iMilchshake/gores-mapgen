use std::{path::PathBuf, str::FromStr};

const STEPS_PER_FRAME: usize = 50;

use crate::{
    config::{GenerationConfig, MapConfig},
    debug::DebugLayers,
    generator::Generator,
    gui::{debug_window, sidebar},
    map_camera::MapCamera,
    random::Seed,
};
use egui::{epaint::Shadow, Color32, Frame, Margin};
use std::env;

use macroquad::input::{
    is_key_pressed, is_mouse_button_down, mouse_delta_position, mouse_position, mouse_wheel,
    KeyCode, MouseButton,
};
use macroquad::time::get_fps;
use macroquad::{
    camera::{set_camera, Camera2D},
    input::is_mouse_button_pressed,
};

const AVG_FPS_FACTOR: f32 = 0.025; // how much current fps is weighted into the rolling average

pub fn window_frame() -> Frame {
    Frame {
        fill: Color32::from_gray(0),
        inner_margin: Margin::same(5.0),
        shadow: Shadow::NONE,
        ..Default::default()
    }
}

#[derive(PartialEq, Debug)]
enum EditorState {
    Playing(PlayingState),
    Paused(PausedState),
}

#[derive(PartialEq, Debug)]
enum PlayingState {
    /// keep generating (default)
    Continuous,

    /// only perform one generation step
    SingleStep,
}

#[derive(PartialEq, Debug)]
enum PausedState {
    /// temporarily stopped/paused generation
    Stopped,

    /// dont start generation yet to allow setup configuration
    Setup,
}
pub struct Editor {
    state: EditorState,
    pub init_gen_configs: Vec<GenerationConfig>,
    pub init_map_configs: Vec<MapConfig>,
    pub canvas: Option<egui::Rect>,
    pub egui_wants_mouse: Option<bool>,
    pub average_fps: f32,
    pub gen_config: GenerationConfig,
    pub map_config: MapConfig,
    pub steps_per_frame: usize,
    pub cam: Option<Camera2D>,
    pub gen: Generator,
    pub debug_layers: Option<DebugLayers>,
    pub disable_debug_layers: bool,

    pub user_seed: Seed,

    pub instant: bool,

    /// whether to keep generating after a map is generated
    pub auto_generate: bool,

    /// whether to keep using the same seed for next generations
    pub fixed_seed: bool,

    /// whether to show the GenerationConfig settings
    pub edit_gen_config: bool,

    /// whether to show the GenerationConfig settings
    pub edit_map_config: bool,

    /// keeps track of camera for map visualization
    pub map_cam: MapCamera,
}

impl Editor {
    pub fn new(gen_config: GenerationConfig, map_config: MapConfig, disable_debug: bool) -> Editor {
        let init_gen_configs: Vec<GenerationConfig> = GenerationConfig::get_all_configs();
        let init_map_configs: Vec<MapConfig> = MapConfig::get_all_configs();
        let gen = Generator::new(&gen_config, &map_config, Seed::from_u64(0));

        Editor {
            state: EditorState::Paused(PausedState::Setup),
            debug_layers: None,
            disable_debug_layers: disable_debug,
            init_gen_configs,
            init_map_configs,
            canvas: None,
            egui_wants_mouse: None,
            average_fps: 0.0,
            cam: None,
            map_cam: MapCamera::default(),
            gen_config,
            map_config,
            steps_per_frame: STEPS_PER_FRAME,
            gen,
            user_seed: Seed::from_string(&"iMilchshake".to_string()),
            instant: false,
            auto_generate: false,
            fixed_seed: false,
            edit_gen_config: false,
            edit_map_config: false,
        }
    }

    pub fn initialize_debug_layers(&mut self) {
        if self.disable_debug_layers {
            return;
        }

        self.debug_layers = Some(DebugLayers::new(
            false,
            (self.gen.map.width, self.gen.map.height),
            0.5,
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
            sidebar(egui_ctx, self);
            debug_window(egui_ctx, self);

            // store remaining space for macroquad drawing
            self.canvas = Some(egui_ctx.available_rect());
            self.egui_wants_mouse = Some(egui_ctx.wants_pointer_input());
        });
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state, EditorState::Playing(_))
    }

    pub fn is_paused(&self) -> bool {
        matches!(self.state, EditorState::Paused(_))
    }

    pub fn is_setup(&self) -> bool {
        matches!(self.state, EditorState::Paused(PausedState::Setup))
    }

    pub fn is_single_setp(&self) -> bool {
        matches!(self.state, EditorState::Playing(PlayingState::SingleStep))
    }

    pub fn toggle(&mut self) {
        match self.state {
            EditorState::Paused(_) => self.set_playing(),
            EditorState::Playing(_) => self.set_stopped(),
        };
    }

    pub fn set_playing(&mut self) {
        if self.is_setup() {
            self.initialize_generator();
        }
        self.state = EditorState::Playing(PlayingState::Continuous);
    }

    pub fn set_single_step(&mut self) {
        if self.is_setup() {
            self.initialize_generator();
        }
        self.state = EditorState::Playing(PlayingState::SingleStep);
    }

    pub fn set_setup(&mut self) {
        self.state = EditorState::Paused(PausedState::Setup);
    }

    pub fn set_stopped(&mut self) {
        self.state = EditorState::Paused(PausedState::Stopped);
    }

    fn initialize_generator(&mut self) {
        if !self.fixed_seed {
            self.user_seed = Seed::from_random(&mut self.gen.rnd);
        }

        self.gen = Generator::new(&self.gen_config, &self.map_config, self.user_seed.clone());
        self.initialize_debug_layers();
    }

    fn mouse_in_viewport(cam: &Camera2D) -> bool {
        let (mouse_x, mouse_y) = mouse_position();
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

        let cam = self.map_cam.get_macroquad_cam();
        set_camera(&cam);
        self.cam = Some(cam);
    }

    pub fn save_map_dialog(&self) {
        let cwd = env::current_dir().unwrap();
        let initial_path = cwd.join("name.map").to_string_lossy().to_string();
        if let Some(path_out) = tinyfiledialogs::save_file_dialog("save map", &initial_path) {
            self.gen.map.export(&PathBuf::from_str(&path_out).unwrap());
        }
    }

    pub fn handle_user_inputs(&mut self) {
        // if is_key_pressed(KeyCode::E) {
        //     self.save_map_dialog();
        // }

        if is_key_pressed(KeyCode::Space) {
            self.set_playing();
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
            && Editor::mouse_in_viewport(self.cam.as_ref().unwrap())
            && !is_mouse_button_pressed(MouseButton::Left)
        {
            self.map_cam.shift(delta);
        }
    }

    pub fn load_gen_config(&mut self, config_name: &str) -> Result<(), &'static str> {
        if let Some(config) = self
            .init_gen_configs
            .iter()
            .find(|&c| c.name == config_name)
        {
            self.gen_config = config.clone();
            Ok(())
        } else {
            Err("Generation config not found!")
        }
    }
}
