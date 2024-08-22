use std::{collections::HashMap, path::PathBuf, str::FromStr};

const STEPS_PER_FRAME: usize = 50;

use crate::{
    config::{GenerationConfig, MapConfig},
    generator::Generator,
    gui::{debug_window, sidebar},
    map::Map,
    random::Seed,
};
use egui::{epaint::Shadow, Color32, Frame, Margin};
use std::env;

use macroquad::camera::{set_camera, Camera2D};
use macroquad::input::{
    is_key_pressed, is_mouse_button_down, is_mouse_button_released, mouse_position, mouse_wheel,
    KeyCode, MouseButton,
};
use macroquad::math::{Rect, Vec2};
use macroquad::time::get_fps;
use macroquad::window::{screen_height, screen_width};
use rand_distr::num_traits::Zero;

const ZOOM_FACTOR: f32 = 0.9;
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
    zoom: f32,
    offset: Vec2,
    cam: Option<Camera2D>,
    last_mouse: Option<Vec2>,
    pub gen: Generator,

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

    /// asd
    pub visualize_debug_layers: HashMap<&'static str, bool>,
}

impl Editor {
    pub fn new(gen_config: GenerationConfig, map_config: MapConfig) -> Editor {
        let init_gen_configs: Vec<GenerationConfig> = GenerationConfig::get_all_configs();
        let init_map_configs: Vec<MapConfig> = MapConfig::get_all_configs();

        // TODO: its kinda stupid to initialize this as its literally re-initialized anyways
        // when starting the first map generation. But i dont wanna bother adding an Option here as
        // the generator also holds the initial empty map which is used for visualization.
        let gen = Generator::new(&gen_config, &map_config, Seed::from_u64(0));

        let mut visualize_debug_layers: HashMap<&'static str, bool> = HashMap::new();
        for layer_name in gen.debug_layers.keys() {
            visualize_debug_layers.insert(layer_name, true);
        }

        Editor {
            state: EditorState::Paused(PausedState::Setup),
            init_gen_configs,
            init_map_configs,
            canvas: None,
            egui_wants_mouse: None,
            average_fps: 0.0,
            zoom: 1.0,
            offset: Vec2::ZERO,
            cam: None,
            last_mouse: None,
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
            visualize_debug_layers,
        }
    }

    pub fn on_frame_start(&mut self) {
        // framerate control
        self.average_fps =
            (self.average_fps * (1. - AVG_FPS_FACTOR)) + (get_fps() as f32 * AVG_FPS_FACTOR);

        // this value is only valid for each frame after calling define_egui()
        self.canvas = None;
    }

    pub fn get_display_factor(&self, map: &Map) -> f32 {
        let canvas = self
            .canvas
            .expect("expect define_egui() to be called before");

        f32::min(
            canvas.width() / map.width as f32,
            canvas.height() / map.height as f32,
        )
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
    }

    fn mouse_in_viewport(cam: &Camera2D) -> bool {
        let (mouse_x, mouse_y) = mouse_position();
        0.0 <= mouse_x
            && mouse_x <= cam.viewport.unwrap().2 as f32
            && 0.0 <= mouse_y
            && mouse_y <= cam.viewport.unwrap().3 as f32
    }

    /// this should result in the exact same behaviour as if not using a camera at all
    pub fn reset_camera() {
        set_camera(&Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
        )));
    }

    pub fn set_cam(&mut self) {
        let map = &self.gen.map;
        let display_factor = self.get_display_factor(map);
        let x_view = display_factor * map.width as f32;
        let y_view = display_factor * map.height as f32;
        let y_shift = screen_height() - y_view;
        let map_rect = Rect::new(0.0, 0.0, map.width as f32, map.height as f32);
        let mut cam = Camera2D::from_display_rect(map_rect);

        // so i guess this is (x, y, width, height) not two positions?
        cam.viewport = Some((0, y_shift as i32, x_view as i32, y_view as i32));

        cam.target -= self.offset;
        cam.zoom *= self.zoom;

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
        if is_key_pressed(KeyCode::E) {
            self.save_map_dialog();
        }

        if is_key_pressed(KeyCode::Space) {
            self.set_playing();
        }

        if is_key_pressed(KeyCode::R) {
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
        }

        // handle mouse inputs
        let mouse_wheel_y = mouse_wheel().1;
        if !mouse_wheel_y.is_zero() {
            if mouse_wheel_y.is_sign_positive() {
                self.zoom /= ZOOM_FACTOR;
            } else {
                self.zoom *= ZOOM_FACTOR;
            }
        }

        let egui_wants_mouse = self
            .egui_wants_mouse
            .expect("expect to be set after define_gui()");

        if !egui_wants_mouse
            && is_mouse_button_down(MouseButton::Left)
            && Editor::mouse_in_viewport(self.cam.as_ref().unwrap())
        {
            let mouse = mouse_position();

            if let Some(last_mouse) = self.last_mouse {
                let display_factor = self.get_display_factor(&self.gen.map);
                let local_delta = Vec2::new(mouse.0, mouse.1) - last_mouse;
                self.offset += local_delta / (self.zoom * display_factor);
            }

            self.last_mouse = Some(mouse.into());

        // mouse pressed for first frame, reset last position
        } else if is_mouse_button_released(MouseButton::Left) {
            self.last_mouse = None;
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
