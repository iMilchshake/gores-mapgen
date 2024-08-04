use std::fs::File;
use std::io::Write;
use std::{collections::HashMap, env, isize};

use egui::RichText;
use mapgen_core::config::{GenerationConfig, MapConfig};
use tinyfiledialogs;

use crate::editor::{window_frame, Editor};
use egui::Context;
use egui::{CollapsingHeader, Label, Ui};
use macroquad::time::get_fps;
use mapgen_core::{
    position::{Position, ShiftDirection},
    random::RandomDistConfig,
};

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
                for value in vec.iter_mut() {
                    ui.horizontal(|ui| {
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

pub fn random_dist_cfg_edit<T, F>(
    ui: &mut Ui,
    cfg: &mut RandomDistConfig<T>,
    edit_element: Option<F>,
    label: &str,
    collapsed: bool,
    fixed_size: bool,
) where
    F: Fn(&mut Ui, &mut T),
    T: Default,
{
    let dist_has_values = cfg.values.is_some();

    CollapsingHeader::new(label)
        .default_open(!collapsed)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                for index in 0..cfg.probs.len() {
                    ui.horizontal(|ui| {
                        edit_f32_prob(ui, &mut cfg.probs[index]);
                        if dist_has_values && edit_element.is_some() {
                            edit_element.as_ref().unwrap()(
                                ui,
                                &mut cfg.values.as_mut().unwrap()[index],
                            );
                        }
                    });
                }

                if !fixed_size {
                    ui.horizontal(|ui| {
                        if ui.button("+").clicked() {
                            if dist_has_values {
                                cfg.values.as_mut().unwrap().push(Default::default());
                            }
                            cfg.probs.push(0.1);
                        };

                        if ui.button("-").clicked() && cfg.probs.len() > 1 {
                            if dist_has_values {
                                cfg.values.as_mut().unwrap().pop();
                            }
                            cfg.probs.pop();
                        };
                    });
                };
            });
        });

    // TODO: only normalize if a value changed?
    cfg.normalize_probs();
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

pub fn edit_probability_usize(ui: &mut Ui, value: &mut (usize, f32)) {
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

pub fn edit_probability_f32(ui: &mut Ui, value: &mut (f32, f32)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label("value:");
            edit_f32_prob(ui, &mut value.0);
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
                ui.label("u64");

                edit_u64_textfield(ui, &mut editor.user_seed.0);
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
            true,
        );

        ui.separator();
        // =======================================[ CONFIG STORAGE ]===================================
        ui.label("save config files:");
        ui.horizontal(|ui| {
            // if ui.button("load file").clicked() {
            //     let cwd = env::current_dir().unwrap();
            //     if let Some(path_in) =
            //         tinyfiledialogs::open_file_dialog("load config", &cwd.to_string_lossy(), None)
            //     {
            //         editor.gen_config = GenerationConfig::load(&path_in);
            //     }
            // }
            if ui.button("gen config").clicked() {
                let cwd = env::current_dir().unwrap();

                let initial_path = cwd
                    .join(editor.current_gen_config.clone() + ".json")
                    .to_string_lossy()
                    .to_string();

                if let Some(path_out) =
                    tinyfiledialogs::save_file_dialog("save gen config", &initial_path)
                {
                    save_gen_config(editor.cur_gen_config_mut(), &path_out);
                }
            };

            if ui.button("map config").clicked() {
                let cwd = env::current_dir().unwrap();

                let initial_path = cwd
                    .join(editor.current_map_config.clone() + ".json")
                    .to_string_lossy()
                    .to_string();

                if let Some(path_out) =
                    tinyfiledialogs::save_file_dialog("save map config", &initial_path)
                {
                    save_map_config(editor.cur_map_config_mut(), &path_out);
                }
            };
        });

        ui.label("load generation config:");
        egui::ComboBox::from_label("")
            .selected_text(format!("{:}", editor.current_gen_config))
            .show_ui(ui, |ui| {
                for (name, _cfg) in editor.gen_configs.iter() {
                    ui.selectable_value(&mut editor.current_gen_config, name.clone(), name);
                }
            });
        ui.label("load map config:");
        egui::ComboBox::from_label(" ")
            .selected_text(format!("{:}", editor.current_map_config))
            .show_ui(ui, |ui| {
                for (name, _cfg) in editor.map_configs.iter() {
                    ui.selectable_value(&mut editor.current_map_config, name.clone(), name);
                }
            });

        ui.horizontal(|ui| {
            ui.checkbox(&mut editor.edit_gen_config, "edit gen");
            ui.checkbox(&mut editor.edit_map_config, "edit map");
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            // =======================================[ GENERATION CONFIG EDIT ]===================================
            if editor.edit_gen_config {
                ui.separator();

                field_edit_widget(
                    ui,
                    &mut editor.current_gen_config,
                    edit_string,
                    "name",
                    false,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().inner_rad_mut_prob,
                    edit_f32_prob,
                    "inner rad mut prob",
                    true,
                );
                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().inner_size_mut_prob,
                    edit_f32_prob,
                    "inner size mut prob",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().outer_rad_mut_prob,
                    edit_f32_prob,
                    "outer rad mut prob",
                    true,
                );
                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().outer_size_mut_prob,
                    edit_f32_prob,
                    "outer size mut prob",
                    true,
                );

                ui.add_enabled_ui(editor.is_setup(), |ui| {
                    random_dist_cfg_edit(
                        ui,
                        &mut editor.cur_gen_config_mut().inner_size_probs,
                        Some(edit_usize),
                        "inner size probs",
                        true,
                        false,
                    );

                    random_dist_cfg_edit(
                        ui,
                        &mut editor.cur_gen_config_mut().outer_margin_probs,
                        Some(edit_usize),
                        "outer margin probs",
                        true,
                        false,
                    );

                    random_dist_cfg_edit(
                        ui,
                        &mut editor.cur_gen_config_mut().circ_probs,
                        Some(edit_f32_prob),
                        "circularity probs",
                        true,
                        false,
                    );
                });

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().platform_distance_bounds,
                    edit_range_usize,
                    "platform distances",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().momentum_prob,
                    edit_f32_prob,
                    "momentum prob",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().max_distance,
                    edit_f32_wtf,
                    "max distance",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().waypoint_reached_dist,
                    edit_usize,
                    "waypoint reached dist",
                    true,
                );

                ui.add_enabled_ui(editor.is_setup(), |ui| {
                    random_dist_cfg_edit(
                        ui,
                        &mut editor.cur_gen_config_mut().shift_weights,
                        None::<fn(&mut Ui, &mut ShiftDirection)>, // TODO: this is stupid wtwf
                        "step weights",
                        false,
                        true,
                    );
                });

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().skip_length_bounds,
                    edit_range_usize,
                    "skip length bounds",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().skip_min_spacing_sqr,
                    edit_usize,
                    "skip min spacing sqr",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().min_freeze_size,
                    edit_usize,
                    "min freeze size",
                    false,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().enable_pulse,
                    edit_bool,
                    "enable pulse",
                    false,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().pulse_straight_delay,
                    edit_usize,
                    "pulse straight delay",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().pulse_corner_delay,
                    edit_usize,
                    "pulse corner delay",
                    false,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().pulse_max_kernel_size,
                    edit_usize,
                    "pulse max kernel",
                    false,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().fade_steps,
                    edit_usize,
                    "fade steps",
                    false,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().fade_max_size,
                    edit_usize,
                    "fade max size",
                    false,
                );

                field_edit_widget(
                    ui,
                    &mut editor.cur_gen_config_mut().fade_min_size,
                    edit_usize,
                    "fade min size",
                    false,
                );
            }

            // =======================================[ MAP CONFIG EDIT ]===================================
            if editor.edit_map_config {
                field_edit_widget(
                    ui,
                    &mut editor.current_map_config,
                    edit_string,
                    "name",
                    false,
                );
                field_edit_widget(
                    ui,
                    &mut editor.cur_map_config_mut().width,
                    edit_usize,
                    "map width",
                    true,
                );
                field_edit_widget(
                    ui,
                    &mut editor.cur_map_config_mut().height,
                    edit_usize,
                    "map height",
                    true,
                );
                ui.add_enabled_ui(editor.is_setup(), |ui| {
                    vec_edit_widget(
                        ui,
                        &mut editor.cur_map_config_mut().waypoints,
                        edit_position,
                        "waypoints",
                        true,
                        false,
                    );
                });
            }
        });
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
            ui.add(Label::new(format!(
                "config: {:?}",
                &editor.current_gen_config
            )));
            if let Some(gen) = &editor.gen {
                ui.add(Label::new(format!("walker: {:?}", &gen.walker)));
            }
        });
}

fn save_map_config(config: &MapConfig, path: &str) {
    let mut file = File::create(path).expect("failed to create config file");
    let serialized = serde_json::to_string_pretty(config).expect("failed to serialize config");
    file.write_all(serialized.as_bytes())
        .expect("failed to write to config file");
}

fn save_gen_config(config: &GenerationConfig, path: &str) {
    let mut file = File::create(path).expect("failed to create config file");
    let serialized = serde_json::to_string_pretty(config).expect("failed to serialize config");
    file.write_all(serialized.as_bytes())
        .expect("failed to write to config file");
}
