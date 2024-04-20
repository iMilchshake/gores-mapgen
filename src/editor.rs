use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, isize};

use egui::{InnerResponse, RichText};
use tinyfiledialogs;

const STEPS_PER_FRAME: usize = 50;

use crate::{
    config::GenerationConfig, generator::Generator, map::Map, position::Position, random::Random,
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

pub fn field_edit_widget<T, F>(
    ui: &mut Ui,
    value: &mut T,
    edit_element: F,
    label: &str,
    vertical: bool,
) -> InnerResponse<()>
where
    F: Fn(&mut Ui, &mut T),
    T: Default,
{
    if vertical {
        ui.vertical(|ui| {
            ui.label(label);
            edit_element(ui, value);
        })
    } else {
        ui.horizontal(|ui| {
            ui.label(label);
            edit_element(ui, value);
        })
    }
}

pub fn edit_usize(ui: &mut Ui, value: &mut usize) {
    ui.add(egui::DragValue::new(value));
}

pub fn edit_pos_i32(ui: &mut Ui, value: &mut i32) {
    ui.add(egui::DragValue::new(value).clamp_range(0..=isize::max_value()));
}

pub fn edit_f32(ui: &mut Ui, value: &mut f32) {
    ui.add(egui::Slider::new(value, 0.0..=1.0));
}

pub fn edit_string(ui: &mut Ui, value: &mut String) {
    let text_edit = egui::TextEdit::singleline(value).desired_width(100.0);
    ui.add(text_edit);
}

pub fn edit_position(ui: &mut Ui, position: &mut Position) {
    ui.horizontal(|ui| {
        ui.label("x:");
        ui.add(egui::widgets::DragValue::new(&mut position.x));
        ui.label("y:");
        ui.add(egui::widgets::DragValue::new(&mut position.y));
    });
}

pub fn edit_range_usize(ui: &mut Ui, values: &mut (usize, usize)) {
    ui.horizontal(|ui| {
        ui.label("min:");
        ui.add(egui::widgets::DragValue::new(&mut values.0).clamp_range(0..=values.1));
        ui.label("max:");
        ui.add(
            egui::widgets::DragValue::new(&mut values.1).clamp_range(values.0..=usize::max_value()),
        );
    });
}

pub struct Editor {
    state: EditorState,
    configs: HashMap<String, GenerationConfig>,
    pub canvas: Option<egui::Rect>,
    pub egui_wants_mouse: Option<bool>,
    pub average_fps: f32,
    pub config: GenerationConfig,
    pub steps_per_frame: usize,
    zoom: f32,
    offset: Vec2,
    cam: Option<Camera2D>,
    last_mouse: Option<Vec2>,
    pub gen: Generator,
    user_str_seed: String,
    pub instant: bool,

    /// whether to keep generating after a map is generated
    pub auto_generate: bool,

    /// whether to keep using the same seed for next generations
    pub fixed_seed: bool,

    /// whether to show the GenerationConfig settings
    edit_preset: bool,
}

impl Editor {
    pub fn new(config: GenerationConfig) -> Editor {
        let configs: HashMap<String, GenerationConfig> = GenerationConfig::get_configs();
        let gen = Generator::new(&config, 0); // TODO: overwritten anyways? Option?
        Editor {
            state: EditorState::Paused(PausedState::Setup),
            configs,
            canvas: None,
            egui_wants_mouse: None,
            average_fps: 0.0,
            zoom: 1.0,
            offset: Vec2::ZERO,
            cam: None,
            last_mouse: None,
            config,
            steps_per_frame: STEPS_PER_FRAME,
            gen,
            user_str_seed: "iMilchshake".to_string(),
            instant: false,
            auto_generate: false,
            fixed_seed: false,
            edit_preset: false,
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
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label(RichText::new("Control").heading());

                ui.horizontal(|ui| {
                    // instant+auto generate will result in setup state before any new frame is
                    // rendered. therefore, disable these elements so user doesnt expect them to
                    // work.
                    let enable_playback_control = !self.instant || !self.auto_generate;
                    ui.add_enabled_ui(enable_playback_control, |ui| {
                        if self.is_setup() {
                            if ui.button("start").clicked() {
                                self.set_playing();
                            }
                        } else if self.is_paused() {
                            if ui.button("resume").clicked() {
                                self.set_playing();
                            }
                        } else if ui.button("pause").clicked() {
                            self.set_stopped();
                        }

                        if ui.button("single step").clicked() {
                            self.set_single_step();
                        }
                    });

                    if !self.is_setup() && ui.button("setup").clicked() {
                        self.set_setup();
                    }
                });

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!self.instant, |ui| {
                        field_edit_widget(ui, &mut self.steps_per_frame, edit_usize, "speed", true);
                    });
                    ui.vertical(|ui| {
                        ui.checkbox(&mut self.instant, "instant");
                        ui.checkbox(&mut self.auto_generate, "auto generate");
                    });
                });

                if self.is_setup() {
                    field_edit_widget(ui, &mut self.user_str_seed, edit_string, "str seed", true);
                    ui.checkbox(&mut self.fixed_seed, "fixed seed");
                }
                ui.separator();

                ui.label("load/save config files:");
                ui.horizontal(|ui| {
                    if ui.button("load file").clicked() {
                        let cwd = env::current_dir().unwrap();
                        if let Some(path_in) = tinyfiledialogs::open_file_dialog(
                            "load config",
                            &cwd.to_string_lossy(),
                            None,
                        ) {
                            self.config = GenerationConfig::load(&path_in);
                        }
                    }
                    if ui.button("save file").clicked() {
                        let cwd = env::current_dir().unwrap();

                        let initial_path = cwd
                            .join(self.config.name.clone() + ".json")
                            .to_string_lossy()
                            .to_string();

                        dbg!(&initial_path);

                        if let Some(path_out) =
                            tinyfiledialogs::save_file_dialog("save config", &initial_path)
                        {
                            self.config.save(&path_out);
                        }
                    };
                });

                ui.label("load predefined configs:");
                egui::ComboBox::from_label("")
                    //.selected_text(format!("{:}", self.config.name.clone()))
                    .show_ui(ui, |ui| {
                        for (name, cfg) in self.configs.iter() {
                            ui.selectable_value(&mut self.config, cfg.clone(), name);
                        }
                    });

                ui.checkbox(&mut self.edit_preset, "show config");

                if self.edit_preset {
                    ui.separator();

                    field_edit_widget(ui, &mut self.config.name, edit_string, "name", false);

                    field_edit_widget(
                        ui,
                        &mut self.config.inner_size_bounds,
                        edit_range_usize,
                        "inner size range",
                        true,
                    );
                    field_edit_widget(
                        ui,
                        &mut self.config.outer_size_bounds,
                        edit_range_usize,
                        "outer size range",
                        true,
                    );
                    field_edit_widget(
                        ui,
                        &mut self.config.inner_rad_mut_prob,
                        edit_f32,
                        "inner rad mut prob",
                        true,
                    );
                    field_edit_widget(
                        ui,
                        &mut self.config.inner_size_mut_prob,
                        edit_f32,
                        "inner size mut prob",
                        true,
                    );

                    field_edit_widget(
                        ui,
                        &mut self.config.outer_rad_mut_prob,
                        edit_f32,
                        "outer rad mut prob",
                        true,
                    );
                    field_edit_widget(
                        ui,
                        &mut self.config.outer_size_mut_prob,
                        edit_f32,
                        "outer size mut prob",
                        true,
                    );

                    field_edit_widget(
                        ui,
                        &mut self.config.platform_distance_bounds,
                        edit_range_usize,
                        "platform distances",
                        true,
                    );

                    field_edit_widget(
                        ui,
                        &mut self.config.momentum_prob,
                        edit_f32,
                        "momentum prob",
                        true,
                    );

                    // only show these in setup mode
                    ui.add_visible_ui(self.is_setup(), |ui| {
                        vec_edit_widget(
                            ui,
                            &mut self.config.waypoints,
                            edit_position,
                            "waypoints",
                            true,
                            false,
                        );

                        vec_edit_widget(
                            ui,
                            &mut self.config.step_weights,
                            edit_pos_i32,
                            "step weights",
                            false,
                            true,
                        );
                    });
                }
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
                        (
                            &self.gen.rnd.seed_hex,
                            &self.gen.rnd.seed_u64,
                            &self.gen.rnd.seed_str
                        )
                    )));
                    ui.add(Label::new(format!("config: {:?}", &self.config)));
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
        let seed_u64 = if !self.user_str_seed.is_empty() {
            // generate new seed based on user string
            let seed_u64 = Random::str_seed_to_u64(&self.user_str_seed);
            if !self.fixed_seed {
                self.user_str_seed = String::new();
            }
            seed_u64
        } else if self.fixed_seed {
            self.gen.rnd.seed_u64 // re-use last seed
        } else {
            self.gen.rnd.random_u64() // generate new seed from previous generator
        };

        self.gen = Generator::new(&self.config, seed_u64);
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

    pub fn handle_user_inputs(&mut self) {
        if is_key_pressed(KeyCode::R) {
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
        }

        // if is_key_pressed(KeyCode::E) {
        //     let t0 = Instant::now();
        //     let name: String = self.gen.rnd.seed_hex.clone();
        //     self.gen.map.export(name);
        //     let time = Instant::now().duration_since(t0);
        //     dbg!(time);
        // }

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
}
