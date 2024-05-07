use std::{collections::HashMap, env, isize};

use egui::RichText;
use tinyfiledialogs;

use crate::{
    config::GenerationConfig,
    editor::{window_frame, Editor},
    position::Position,
    random::Seed,
};
use egui::Context;
use egui::{CollapsingHeader, Label, Ui};
use macroquad::time::get_fps;

/// Helper function for input sanitization
fn normalize_probs(vec: &mut [(usize, f32)]) {
    let sum: f32 = vec.iter().map(|(_, val)| val).sum();
    // if all values are zero, set all to 1/n
    if sum == 0.0 {
        let len = vec.len();
        for (_, val) in vec.iter_mut() {
            *val = 1.0 / len as f32;
        }
    // otherwise normalize, if required
    } else if sum != 1.0 {
        for (_, val) in vec.iter_mut() {
            *val /= sum; // Normalize the vector
        }
    }
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
                for (_i, value) in vec.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        // ui.label(i.to_string());
                        edit_element(ui, value);
                    });
                }

                if !fixed_size {
                    ui.horizontal(|ui| {
                        if ui.button("+").clicked() {
                            vec.push(Default::default());
                        };

                        if ui.button("-").clicked() && vec.len() > 1 {
                            vec.pop();
                        };
                    });
                };
            });
        });
}

pub fn hashmap_edit_widget<T, F>(
    ui: &mut Ui,
    hashmap: &mut HashMap<&'static str, T>,
    edit_element: F,
    label: &str,
    collapsed: bool,
) where
    F: Fn(&mut Ui, &mut T),
{
    CollapsingHeader::new(label)
        .default_open(!collapsed)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                for (val1, val2) in hashmap.iter_mut() {
                    ui.horizontal(|ui| {
                        ui.label(val1.to_string());
                        edit_element(ui, val2);
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
) where
    F: Fn(&mut Ui, &mut T),
    T: Default,
{
    if vertical {
        ui.vertical(|ui| {
            ui.label(label);
            edit_element(ui, value)
        });
    } else {
        ui.horizontal(|ui| {
            ui.label(label);
            edit_element(ui, value)
        });
    }
}

/// edit u64 using a crappy textfield, as DragValue results in numeric instabilities
fn edit_u64_textfield(ui: &mut egui::Ui, value: &mut u64) -> egui::Response {
    let mut int_as_str = format!("{}", value);
    let res = ui.add(egui::TextEdit::singleline(&mut int_as_str).desired_width(150.0));
    if int_as_str.is_empty() {
        *value = 0;
    } else if let Ok(result) = int_as_str.parse() {
        *value = result;
    }
    res
}

pub fn edit_usize(ui: &mut Ui, value: &mut usize) {
    ui.add(egui::DragValue::new(value));
}

pub fn edit_pos_i32(ui: &mut Ui, value: &mut i32) {
    ui.add(egui::DragValue::new(value).clamp_range(0..=isize::max_value()));
}

// TODO: IMAGINE having a dynamic range argument.. imagine, that would be nice
pub fn edit_f32_wtf(ui: &mut Ui, value: &mut f32) {
    ui.add(egui::Slider::new(value, 0.0..=15.0));
}

pub fn edit_f32_prob(ui: &mut Ui, value: &mut f32) {
    ui.spacing_mut().slider_width = 75.0;
    ui.add(
        egui::Slider::new(value, 0.0..=1.0)
            .fixed_decimals(3)
            .step_by(0.001),
    );
}

pub fn edit_string(ui: &mut Ui, value: &mut String) {
    let text_edit = egui::TextEdit::singleline(value).desired_width(100.0);
    ui.add(text_edit);
}

pub fn edit_probability_tuple(ui: &mut Ui, value: &mut (usize, f32)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label("value:");
            edit_usize(ui, &mut value.0);
        });
        ui.vertical(|ui| {
            ui.label("prob:");
            edit_f32_prob(ui, &mut value.1)
        });
    });
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

pub fn edit_bool(ui: &mut Ui, value: &mut bool) {
    ui.add(egui::Checkbox::new(value, ""));
}

pub fn sidebar(ctx: &Context, editor: &mut Editor) {
    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        // =======================================[ STATE CONTROL ]===================================
        ui.label(RichText::new("Control").heading());
        ui.horizontal(|ui| {
            // instant+auto generate will result in setup state before any new frame is
            // rendered. therefore, disable these elements so user doesnt expect them to
            // work.
            let enable_playback_control = !editor.instant || !editor.auto_generate;
            ui.add_enabled_ui(enable_playback_control, |ui| {
                if editor.is_setup() {
                    if ui.button("start").clicked() {
                        editor.set_playing();
                    }
                } else if editor.is_paused() {
                    if ui.button("resume").clicked() {
                        editor.set_playing();
                    }
                } else if ui.button("pause").clicked() {
                    editor.set_stopped();
                }

                if ui.button("single step").clicked() {
                    editor.set_single_step();
                }
            });

            if !editor.is_setup() && ui.button("setup").clicked() {
                editor.set_setup();
            }
        });

        // =======================================[ SPEED CONTROL ]===================================
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!editor.instant, |ui| {
                field_edit_widget(ui, &mut editor.steps_per_frame, edit_usize, "speed", true);
            });
            ui.vertical(|ui| {
                ui.checkbox(&mut editor.instant, "instant");
                ui.checkbox(&mut editor.auto_generate, "auto generate");
            });
        });

        // =======================================[ SEED CONTROL ]===================================
        if editor.is_setup() {
            ui.horizontal(|ui| {
                ui.label("str");
                let text_edit =
                    egui::TextEdit::singleline(&mut editor.user_seed.seed_str).desired_width(150.0);
                if ui.add(text_edit).changed() {
                    editor.user_seed.seed_u64 = Seed::str_to_u64(&editor.user_seed.seed_str);
                }
            });

            ui.horizontal(|ui| {
                ui.label("u64");

                if edit_u64_textfield(ui, &mut editor.user_seed.seed_u64).changed() {
                    editor.user_seed.seed_str = String::new();
                }
            });

            ui.horizontal(|ui| {
                ui.checkbox(&mut editor.fixed_seed, "fixed seed");
                if ui.button("save map").clicked() {
                    editor.save_map_dialog();
                }
            });
        }
        ui.separator();
        // =======================================[ DEBUG LAYERS ]===================================

        hashmap_edit_widget(
            ui,
            &mut editor.visualize_debug_layers,
            edit_bool,
            "debug layers",
            false,
        );

        ui.separator();
        // =======================================[ CONFIG STORAGE ]===================================
        ui.label("load/save config files:");
        ui.horizontal(|ui| {
            if ui.button("load file").clicked() {
                let cwd = env::current_dir().unwrap();
                if let Some(path_in) =
                    tinyfiledialogs::open_file_dialog("load config", &cwd.to_string_lossy(), None)
                {
                    editor.config = GenerationConfig::load(&path_in);
                }
            }
            if ui.button("save file").clicked() {
                let cwd = env::current_dir().unwrap();

                let initial_path = cwd
                    .join(editor.config.name.clone() + ".json")
                    .to_string_lossy()
                    .to_string();

                if let Some(path_out) =
                    tinyfiledialogs::save_file_dialog("save config", &initial_path)
                {
                    editor.config.save(&path_out);
                }
            };
        });

        ui.label("load predefined configs:");
        egui::ComboBox::from_label("")
            //.selected_text(format!("{:}", editor.config.name.clone()))
            .show_ui(ui, |ui| {
                for (name, cfg) in editor.configs.iter() {
                    ui.selectable_value(&mut editor.config, cfg.clone(), name);
                }
            });

        ui.checkbox(&mut editor.edit_preset, "show config");

        // =======================================[ CONFIG EDIT ]===================================
        if editor.edit_preset {
            ui.separator();

            field_edit_widget(ui, &mut editor.config.name, edit_string, "name", false);

            field_edit_widget(
                ui,
                &mut editor.config.inner_rad_mut_prob,
                edit_f32_prob,
                "inner rad mut prob",
                true,
            );
            field_edit_widget(
                ui,
                &mut editor.config.inner_size_mut_prob,
                edit_f32_prob,
                "inner size mut prob",
                true,
            );

            field_edit_widget(
                ui,
                &mut editor.config.outer_rad_mut_prob,
                edit_f32_prob,
                "outer rad mut prob",
                true,
            );
            field_edit_widget(
                ui,
                &mut editor.config.outer_size_mut_prob,
                edit_f32_prob,
                "outer size mut prob",
                true,
            );

            ui.add_enabled_ui(editor.is_setup(), |ui| {
                vec_edit_widget(
                    ui,
                    &mut editor.config.inner_size_probs,
                    edit_probability_tuple,
                    "inner size probs",
                    true,
                    false,
                );
                normalize_probs(&mut editor.config.inner_size_probs);

                vec_edit_widget(
                    ui,
                    &mut editor.config.outer_margin_probs,
                    edit_probability_tuple,
                    "outer margin probs",
                    true,
                    false,
                );
                normalize_probs(&mut editor.config.outer_margin_probs);
            });

            field_edit_widget(
                ui,
                &mut editor.config.platform_distance_bounds,
                edit_range_usize,
                "platform distances",
                true,
            );

            field_edit_widget(
                ui,
                &mut editor.config.momentum_prob,
                edit_f32_prob,
                "momentum prob",
                true,
            );

            field_edit_widget(
                ui,
                &mut editor.config.max_distance,
                edit_f32_wtf,
                "max distance",
                true,
            );

            field_edit_widget(
                ui,
                &mut editor.config.waypoint_reached_dist,
                edit_usize,
                "waypoint reached dist",
                true,
            );

            // only show these in setup mode
            ui.add_enabled_ui(editor.is_setup(), |ui| {
                vec_edit_widget(
                    ui,
                    &mut editor.config.waypoints,
                    edit_position,
                    "waypoints",
                    true,
                    false,
                );

                vec_edit_widget(
                    ui,
                    &mut editor.config.shift_weights,
                    edit_pos_i32,
                    "step weights",
                    false,
                    true,
                );
            });

            field_edit_widget(
                ui,
                &mut editor.config.skip_length_bounds,
                edit_range_usize,
                "skip length bounds",
                true,
            );

            field_edit_widget(
                ui,
                &mut editor.config.skip_min_spacing_sqr,
                edit_usize,
                "skip length bounds",
                true,
            );
        }
    });
}

pub fn debug_window(ctx: &Context, editor: &mut Editor) {
    egui::Window::new("DEBUG")
        .frame(window_frame())
        .default_open(false)
        .show(ctx, |ui| {
            ui.add(Label::new(format!("fps: {:}", get_fps())));
            ui.add(Label::new(format!(
                "avg: {:}",
                editor.average_fps.round() as usize
            )));

            ui.add(Label::new(format!("seed: {:?}", editor.user_seed)));

            ui.add(Label::new(format!("config: {:?}", &editor.config)));

            ui.add(Label::new(format!("walker: {:?}", &editor.gen.walker)));
        });
}
