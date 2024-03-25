use egui::RichText;
use std::time::Instant;

const STEPS_PER_FRAME: usize = 50;

use crate::{
    kernel::Kernel,
    map::{BlockType, Map},
    position::Position,
    random::Random,
    walker::CuteWalker,
};
use egui::{epaint::Shadow, CollapsingHeader, Color32, Frame, Label, Margin, Ui};
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
const AVG_FPS_FACTOR: f32 = 0.25; // how much current fps is weighted into the rolling average

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

pub fn vec_edit_widget<T, F>(
    ui: &mut Ui,
    vec: &mut Vec<T>,
    edit_element: F,
    label: &str,
    collapsed: bool,
    fixed_size: bool,
) where
    F: Fn(&mut Ui, &mut T),
    T: Default,
{
    CollapsingHeader::new(label)
        .default_open(!collapsed)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                for (i, value) in vec.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(i.to_string());
                        edit_element(ui, value);
                    });
                }

                if !fixed_size {
                    ui.horizontal(|ui| {
                        if ui.button("+").clicked() {
                            vec.push(Default::default());
                        }

                        if ui.button("-").clicked() && !vec.is_empty() {
                            vec.pop();
                        }
                    });
                }
            });
        });
}

pub fn field_edit_widget<T, F>(ui: &mut Ui, value: &mut T, edit_element: F, label: &str)
where
    F: Fn(&mut Ui, &mut T),
    T: Default,
{
    ui.vertical(|ui| {
        ui.label(label);
        edit_element(ui, value);
    });
}

pub fn edit_usize(ui: &mut Ui, value: &mut usize) {
    ui.add(egui::DragValue::new(value));
}

pub fn edit_i32(ui: &mut Ui, value: &mut i32) {
    ui.add(egui::DragValue::new(value));
}

pub fn edit_f32(ui: &mut Ui, value: &mut f32) {
    ui.add(egui::Slider::new(value, 0.0..=1.0));
}

pub fn edit_string(ui: &mut Ui, value: &mut String) {
    ui.add(egui::widgets::TextEdit::singleline(value));
}

pub fn edit_position(ui: &mut Ui, position: &mut Position) {
    ui.horizontal(|ui| {
        ui.label("x:");
        ui.add(egui::widgets::DragValue::new(&mut position.x));
        ui.label("y:");
        ui.add(egui::widgets::DragValue::new(&mut position.y));
    });
}

pub struct GenerationConfig {
    pub seed: String,
    pub max_inner_size: usize,
    pub max_outer_size: usize,
    pub inner_rad_mut_prob: f32,
    pub inner_size_mut_prob: f32,
    pub waypoints: Vec<Position>,
    pub step_weights: Vec<i32>,
    pub auto_generate: bool,
}

impl Default for GenerationConfig {
    // TODO: might make some sense to move waypoints somewhere else
    fn default() -> GenerationConfig {
        GenerationConfig {
            seed: "iMilchshake".to_string(),
            max_inner_size: 2,
            max_outer_size: 4,
            inner_rad_mut_prob: 0.1,
            inner_size_mut_prob: 0.3,
            waypoints: vec![
                Position::new(250, 50),
                Position::new(250, 250),
                Position::new(50, 250),
                Position::new(50, 50),
            ],
            step_weights: vec![6, 5, 4, 3],
            auto_generate: false,
        }
    }
}

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,
    pub rnd: Random,
}

impl Generator {
    /// derive a initial generator state based on a GenerationConfig
    pub fn new(config: &GenerationConfig) -> Generator {
        let spawn = Position::new(50, 50);
        let map = Map::new(300, 300, BlockType::Hookable, spawn.clone());
        let init_inner_kernel = Kernel::new(config.max_inner_size, 0.0);
        let init_outer_kernel = Kernel::new(config.max_outer_size, 0.1);
        let walker = CuteWalker::new(spawn, init_inner_kernel, init_outer_kernel, config);
        let rnd = Random::from_str_seed(config.seed.clone(), config.step_weights.clone());

        Generator { walker, map, rnd }
    }

    pub fn step(&mut self, editor: &Editor) -> Result<(), &'static str> {
        // check if walker has reached goal position
        if self.walker.is_goal_reached() == Some(true) {
            self.walker.next_waypoint();
        }

        // randomly mutate kernel
        self.walker.mutate_kernel(&editor.config, &mut self.rnd);

        // perform one step
        self.walker
            .probabilistic_step(&mut self.map, &mut self.rnd)?;

        Ok(())
    }
}

pub struct Editor {
    state: EditorState,
    pub canvas: Option<egui::Rect>,
    pub egui_wants_mouse: Option<bool>,
    pub average_fps: f32,
    pub config: GenerationConfig,
    pub steps_per_frame: usize,
    zoom: f32,
    offset: Vec2,
    cam: Option<Camera2D>,
    last_mouse: Option<Vec2>,
}

impl Editor {
    pub fn new(config: GenerationConfig) -> Editor {
        Editor {
            state: EditorState::Paused(PausedState::Setup),
            canvas: None,
            egui_wants_mouse: None,
            average_fps: 0.0,
            zoom: 1.0,
            offset: Vec2::ZERO,
            cam: None,
            last_mouse: None,
            config,
            steps_per_frame: STEPS_PER_FRAME,
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

    pub fn define_egui(&mut self, gen: &mut Generator) {
        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label(RichText::new("Control").heading());

                ui.horizontal(|ui| {
                    // toggle pause
                    if !self.is_paused() {
                        if ui.button("stop").clicked() {
                            self.set_stopped();
                        }
                    } else if ui.button("play").clicked() {
                        self.set_playing();
                    }

                    // pause, allow single step
                    if ui.button("single step").clicked() {
                        self.set_single_step();
                    }

                    if ui.button("reset").clicked() {
                        self.set_setup();
                        *gen = Generator::new(&self.config);
                    }
                });

                field_edit_widget(ui, &mut self.steps_per_frame, edit_usize, "steps_per_frame");

                ui.checkbox(&mut self.config.auto_generate, "auto generate");

                ui.separator();

                field_edit_widget(ui, &mut self.config.seed, edit_string, "seed");
                field_edit_widget(
                    ui,
                    &mut self.config.max_inner_size,
                    edit_usize,
                    "max inner size",
                );
                field_edit_widget(
                    ui,
                    &mut self.config.max_outer_size,
                    edit_usize,
                    "max outer size",
                );
                field_edit_widget(
                    ui,
                    &mut self.config.inner_rad_mut_prob,
                    edit_f32,
                    "inner rad mut prob",
                );
                field_edit_widget(
                    ui,
                    &mut self.config.inner_size_mut_prob,
                    edit_f32,
                    "inner size mut prob",
                );

                vec_edit_widget(
                    ui,
                    &mut self.config.waypoints,
                    edit_position,
                    "waypoints",
                    true,
                    false,
                );

                ui.add_visible_ui(self.is_setup(), |ui| {
                    vec_edit_widget(
                        ui,
                        &mut self.config.step_weights,
                        edit_i32,
                        "step weights",
                        false,
                        true,
                    );
                });
                // self.config
                //     .show_top(ui, RichText::new("Config").heading(), None);
            });

            egui::Window::new("DEBUG")
                .frame(window_frame())
                .default_open(false)
                .show(egui_ctx, |ui| {
                    ui.add(Label::new(format!("fps: {:}", get_fps())));
                    ui.add(Label::new(format!(
                        "avg: {:}",
                        self.average_fps.round() as usize
                    )));
                    ui.add(Label::new(format!("playback: {:?}", self.state)));
                    ui.add(Label::new(format!(
                        "seed: {:?}",
                        (&gen.rnd.seed_hex, &gen.rnd.seed_u64, &gen.rnd.seed_str)
                    )));
                });

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
        self.state = EditorState::Playing(PlayingState::Continuous);
    }

    pub fn set_single_step(&mut self) {
        self.state = EditorState::Playing(PlayingState::SingleStep);
    }

    pub fn set_setup(&mut self) {
        self.state = EditorState::Paused(PausedState::Setup);
    }

    pub fn set_stopped(&mut self) {
        self.state = EditorState::Paused(PausedState::Stopped);
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
        // no idea why i dont have to use negative values here???
        set_camera(&Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
        )));
    }

    pub fn set_cam(&mut self, map: &Map) {
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

    pub fn handle_user_inputs(&mut self, map: &Map) {
        if is_key_pressed(KeyCode::R) {
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
        }

        if is_key_pressed(KeyCode::E) {
            let t0 = Instant::now();
            map.export();
            let time = Instant::now().duration_since(t0);
            dbg!(time);
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
                let display_factor = self.get_display_factor(map);
                let local_delta = Vec2::new(mouse.0, mouse.1) - last_mouse;
                self.offset += local_delta / (self.zoom * display_factor);
            }

            self.last_mouse = Some(mouse.into());

        // mouse pressed for first frame, reset last position
        } else if is_mouse_button_released(MouseButton::Left) {
            self.last_mouse = None;
        }
    }
}
