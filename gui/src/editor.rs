use std::{collections::HashMap, env};

use mapgen_core::{
    config::{load_configs_from_dir, GenerationConfig, MapConfig},
    generator::Generator,
    map::{BlockType, Map},
    random::{Random, Seed},
};
use mapgen_exporter::Exporter;
use twmap::TwMap;

use egui::{epaint::Shadow, Color32, Frame, Margin};
use macroquad::{
    camera::{set_camera, Camera2D},
    input::{
        is_key_pressed, is_mouse_button_down, is_mouse_button_released, mouse_position,
        mouse_wheel, KeyCode, MouseButton,
    },
    math::{Rect, Vec2},
    time::get_fps,
    window::{screen_height, screen_width},
};

use rand_distr::num_traits::Zero;

use crate::gui::{debug_window, sidebar};

const STEPS_PER_FRAME: usize = 50;
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
    pub gen_configs: HashMap<String, GenerationConfig>,
    pub map_configs: HashMap<String, MapConfig>,
    pub canvas: Option<egui::Rect>,
    pub egui_wants_mouse: Option<bool>,
    pub average_fps: f32,
    pub current_gen_config: String,
    pub current_map_config: String,
    pub steps_per_frame: usize,
    zoom: f32,
    offset: Vec2,
    cam: Option<Camera2D>,
    last_mouse: Option<Vec2>,
    pub gen: Option<Generator>,

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
    pub fn new(init_gen_config: String, init_map_config: String) -> Editor {
        let gen_configs =
            load_configs_from_dir::<GenerationConfig, _>("../data/configs/gen").unwrap();
        let map_configs = load_configs_from_dir::<MapConfig, _>("../data/configs/map").unwrap();

        // TODO: add fallback
        assert!(gen_configs.contains_key(&init_gen_config));
        assert!(map_configs.contains_key(&init_map_config));

        let visualize_debug_layers: HashMap<&'static str, bool> = HashMap::new();

        Editor {
            state: EditorState::Paused(PausedState::Setup),
            gen_configs,
            map_configs,
            canvas: None,
            egui_wants_mouse: None,
            average_fps: 0.0,
            zoom: 1.0,
            offset: Vec2::ZERO,
            cam: None,
            last_mouse: None,
            current_gen_config: init_gen_config,
            current_map_config: init_map_config,
            steps_per_frame: STEPS_PER_FRAME,
            gen: None,
            user_seed: Seed::from_str("iMilchshake"),
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
            self.on_start();
        }
        self.state = EditorState::Playing(PlayingState::Continuous);
    }

    pub fn set_single_step(&mut self) {
        if self.is_setup() {
            self.on_start();
        }
        self.state = EditorState::Playing(PlayingState::SingleStep);
    }

    pub fn set_setup(&mut self) {
        self.state = EditorState::Paused(PausedState::Setup);
    }

    pub fn set_stopped(&mut self) {
        self.state = EditorState::Paused(PausedState::Stopped);
    }

    fn on_start(&mut self) {
        if !self.fixed_seed {
            self.user_seed = Seed::from_u64(Random::get_random_u64());
        }

        self.gen = Some(Generator::new(
            Map::new(self.cur_map_config(), BlockType::Hookable),
            self.user_seed,
            self.cur_gen_config(),
        ));
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
        if let Some(gen) = &self.gen {
            let map = &gen.map;
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
    }

    pub fn save_map_dialog(&self) {
        if let Some(gen) = &self.gen {
            let cwd = env::current_dir().unwrap();
            let initial_path = cwd.join("name.map").to_string_lossy().to_string();
            if let Some(path_out) = tinyfiledialogs::save_file_dialog("save map", &initial_path) {
                // TODO: Allow to select (or sample randomly) from various base maps
                let mut tw_map =
                    TwMap::parse_file("./automap_test.map").expect("failed to parse base map");
                tw_map.load().expect("failed to load base map");
                let mut exporter = Exporter::new(&mut tw_map, &gen.map, Default::default());
                exporter.finalize().save_map(path_out);
            }
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
                let display_factor = if let Some(gen) = &self.gen {
                    self.get_display_factor(&gen.map)
                } else {
                    1.0
                };
                let local_delta = Vec2::new(mouse.0, mouse.1) - last_mouse;
                self.offset += local_delta / (self.zoom * display_factor);
            }

            self.last_mouse = Some(mouse.into());

        // mouse pressed for first frame, reset last position
        } else if is_mouse_button_released(MouseButton::Left) {
            self.last_mouse = None;
        }
    }

    pub fn cur_gen_config(&self) -> GenerationConfig {
        self.gen_configs[&self.current_gen_config].clone()
    }

    pub fn cur_map_config(&self) -> MapConfig {
        self.map_configs[&self.current_map_config].clone()
    }

    pub fn cur_gen_config_mut(&mut self) -> &mut GenerationConfig {
        self.gen_configs.get_mut(&self.current_gen_config).unwrap()
    }

    pub fn cur_map_config_mut(&mut self) -> &mut MapConfig {
        self.map_configs.get_mut(&self.current_map_config).unwrap()
    }
}
