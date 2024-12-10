use std::{collections::BTreeMap, env};

use egui::{Align2, RichText};
use tinyfiledialogs;

use crate::{
    editor::{window_frame, Editor},
    position::{Position, ShiftDirection},
    random::{RandomDistConfig, Seed},
};
use egui::Context;
use egui::{CollapsingHeader, Label, Ui};
use macroquad::time::get_fps;

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
                        edit_f32_slider_prob(ui, &mut cfg.probs[index]);
                        if dist_has_values {
                            if let Some(edit_element) = &edit_element {
                                edit_element(ui, &mut cfg.values.as_mut().unwrap()[index]);
                            }
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

pub fn btree_edit_widget<T, F>(
    ui: &mut Ui,
    hashmap: &mut BTreeMap<&'static str, T>,
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
    ui.add(egui::DragValue::new(value).clamp_range(0..=isize::MAX));
}

pub fn edit_f32_slider_bounded(min: f32, max: f32) -> impl Fn(&mut Ui, &mut f32) {
    move |ui: &mut Ui, value: &mut f32| {
        ui.spacing_mut().slider_width = 75.0;
        ui.add(egui::Slider::new(value, min..=max));
    }
}

pub fn edit_f32_slider_prob(ui: &mut Ui, value: &mut f32) {
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
            edit_f32_slider_prob(ui, &mut value.1)
        });
    });
}

pub fn edit_probability_f32(ui: &mut Ui, value: &mut (f32, f32)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label("value:");
            edit_f32_slider_prob(ui, &mut value.0);
        });
        ui.vertical(|ui| {
            ui.label("prob:");
            edit_f32_slider_prob(ui, &mut value.1)
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
        ui.add(egui::widgets::DragValue::new(&mut values.1).clamp_range(values.0..=usize::MAX));
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
                ui.checkbox(&mut editor.retry_on_failure, "retry fail");
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

        if let Some(ref mut debug_layers) = editor.debug_layers {
            btree_edit_widget(
                ui,
                &mut debug_layers.active_layers,
                edit_bool,
                "debug layers",
                true,
            );

            ui.separator();
        }
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
                    .join(editor.gen_config.name.clone() + ".json")
                    .to_string_lossy()
                    .to_string();

                if let Some(path_out) =
                    tinyfiledialogs::save_file_dialog("save gen config", &initial_path)
                {
                    editor.gen_config.save(&path_out);
                }
            };

            if ui.button("map config").clicked() {
                let cwd = env::current_dir().unwrap();

                let initial_path = cwd
                    .join(editor.gen_config.name.clone() + ".json")
                    .to_string_lossy()
                    .to_string();

                if let Some(path_out) =
                    tinyfiledialogs::save_file_dialog("save map config", &initial_path)
                {
                    editor.map_config.save(&path_out);
                }
            };
        });

        ui.label("load generation config:");
        egui::ComboBox::from_label("")
            .selected_text(editor.gen_config.name.to_string())
            .show_ui(ui, |ui| {
                for cfg in editor.init_gen_configs.iter() {
                    ui.selectable_value(&mut editor.gen_config, cfg.clone(), &cfg.name);
                }
            });
        ui.label("load map config:");
        egui::ComboBox::from_label(" ")
            .selected_text(editor.map_config.name.to_string())
            .show_ui(ui, |ui| {
                for cfg in editor.init_map_configs.iter() {
                    // TODO: reinitialize generator with new mapconfig! careful with overriding gen config!
                    ui.selectable_value(&mut editor.map_config, cfg.clone(), &cfg.name);
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

                field_edit_widget(ui, &mut editor.gen_config.name, edit_string, "name", false);

                field_edit_widget(
                    ui,
                    &mut editor.gen_config.difficulty,
                    edit_f32_slider_bounded(0.1, 5.0),
                    "difficulty",
                    false,
                );

                CollapsingHeader::new("Kernel Config ")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.inner_rad_mut_prob,
                            edit_f32_slider_prob,
                            "inner rad mut prob",
                            true,
                        );
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.inner_size_mut_prob,
                            edit_f32_slider_prob,
                            "inner size mut prob",
                            true,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.outer_rad_mut_prob,
                            edit_f32_slider_prob,
                            "outer rad mut prob",
                            true,
                        );
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.outer_size_mut_prob,
                            edit_f32_slider_prob,
                            "outer size mut prob",
                            true,
                        );

                        ui.add_enabled_ui(editor.is_setup(), |ui| {
                            random_dist_cfg_edit(
                                ui,
                                &mut editor.gen_config.inner_size_probs,
                                Some(edit_usize),
                                "inner size probs",
                                true,
                                false,
                            );

                            random_dist_cfg_edit(
                                ui,
                                &mut editor.gen_config.outer_margin_probs,
                                Some(edit_usize),
                                "outer margin probs",
                                true,
                                false,
                            );

                            random_dist_cfg_edit(
                                ui,
                                &mut editor.gen_config.circ_probs,
                                Some(edit_f32_slider_prob),
                                "circularity probs",
                                true,
                                false,
                            );
                        });
                    });

                CollapsingHeader::new("Platforms")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.plat_min_distance,
                            edit_usize,
                            "min distance",
                            true,
                        );
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.plat_width_bounds,
                            edit_range_usize,
                            "width bounds",
                            true,
                        );
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.plat_height_bounds,
                            edit_range_usize,
                            "height bounds",
                            true,
                        );
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.plat_min_empty_height,
                            edit_usize,
                            "min empty height",
                            true,
                        );
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.plat_soft_overhang,
                            edit_bool,
                            "soft overhang",
                            true,
                        );
                    });

                CollapsingHeader::new("Momentum")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.momentum_prob,
                            edit_f32_slider_prob,
                            "momentum prob",
                            true,
                        );
                    });

                CollapsingHeader::new("Obstacles")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.max_distance,
                            edit_f32_slider_bounded(0.1, 15.0),
                            "max distance",
                            true,
                        );
                    });

                CollapsingHeader::new("Waypoints")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.waypoint_reached_dist,
                            edit_usize,
                            "waypoint reached dist",
                            true,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.max_subwaypoint_dist,
                            edit_f32_slider_bounded(0.1, 100.0),
                            "subpoint max dist",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.subwaypoint_max_shift_dist,
                            edit_f32_slider_bounded(0.0, 300.0),
                            "subpoint max shift",
                            false,
                        );
                    });

                ui.add_enabled_ui(editor.is_setup(), |ui| {
                    random_dist_cfg_edit(
                        ui,
                        &mut editor.gen_config.shift_weights,
                        // TODO: this is stupid wtf, but thats fine as this functionality
                        // will be reworked with the upcoming dynamic weighting for cells anyways
                        None::<fn(&mut Ui, &mut ShiftDirection)>,
                        "Step Weights",
                        true,
                        true,
                    );
                });
                CollapsingHeader::new("Skips")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.skip_length_bounds,
                            edit_range_usize,
                            "skip length bounds",
                            true,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.skip_min_spacing_sqr,
                            edit_usize,
                            "skip min spacing sqr",
                            true,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.max_level_skip,
                            edit_usize,
                            "max level skip",
                            true,
                        );
                    });

                CollapsingHeader::new("Blob removal")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.min_freeze_size,
                            edit_usize,
                            "min freeze size",
                            false,
                        );
                    });

                CollapsingHeader::new("Pulse")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.enable_pulse,
                            edit_bool,
                            "enable pulse",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.pulse_straight_delay,
                            edit_usize,
                            "pulse straight delay",
                            true,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.pulse_corner_delay,
                            edit_usize,
                            "pulse corner delay",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.pulse_max_kernel_size,
                            edit_usize,
                            "pulse max kernel",
                            false,
                        );
                    });

                CollapsingHeader::new("Fade")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.fade_steps,
                            edit_usize,
                            "fade steps",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.fade_max_size,
                            edit_usize,
                            "fade max size",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.fade_min_size,
                            edit_usize,
                            "fade min size",
                            false,
                        );
                    });

                CollapsingHeader::new("Position Locking")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.pos_lock_max_dist,
                            edit_f32_slider_bounded(0.0, 150.0),
                            "pos lock max dist",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.pos_lock_max_delay,
                            edit_usize,
                            "pos lock max delay",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.lock_kernel_size,
                            edit_usize,
                            "lock kernel size",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.waypoint_lock_distance,
                            edit_usize,
                            "waypoint lock dist",
                            false,
                        );
                    });

                CollapsingHeader::new("noise")
                    .default_open(false)
                    .show(ui, |ui| {
                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.noise_scale,
                            edit_f32_slider_bounded(0.0, 10.0),
                            "noise scale",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.noise_threshold,
                            edit_f32_slider_bounded(-1.0, 1.0),
                            "noise threshold",
                            false,
                        );

                        field_edit_widget(
                            ui,
                            &mut editor.gen_config.noise_invert,
                            edit_bool,
                            "noise invert",
                            false,
                        );
                    });
            }

            // =======================================[ MAP CONFIG EDIT ]===================================
            if editor.edit_map_config {
                field_edit_widget(ui, &mut editor.map_config.name, edit_string, "name", false);
                field_edit_widget(
                    ui,
                    &mut editor.map_config.width,
                    edit_usize,
                    "map width",
                    true,
                );
                field_edit_widget(
                    ui,
                    &mut editor.map_config.height,
                    edit_usize,
                    "map height",
                    true,
                );
                ui.add_enabled_ui(editor.is_setup(), |ui| {
                    vec_edit_widget(
                        ui,
                        &mut editor.map_config.waypoints,
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
            ui.add(Label::new(format!("config: {:?}", &editor.gen_config)));
            ui.add(Label::new(format!("walker: {:?}", &editor.gen.walker)));
        });
}

pub fn debug_layers_window(ctx: &Context, editor: &mut Editor) {
    if editor.debug_layers.is_none() {
        return;
    }
    let debug_layers = editor.debug_layers.as_ref().unwrap();
    let map_mouse_pos = editor.map_cam.get_map_mouse_pos();
    let map_mouse_pos_cell = (
        map_mouse_pos.x.floor() as usize,
        map_mouse_pos.y.floor() as usize,
    );

    egui::Window::new("debug_layers_window")
        .frame(window_frame())
        .title_bar(false)
        .default_open(true)
        .anchor(Align2::RIGHT_TOP, egui::vec2(-5., 5.))
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    for (name, _layer) in debug_layers.bool_layers.iter() {
                        ui.label(*name);
                    }
                    for (name, _layer) in debug_layers.float_layers.iter() {
                        ui.label(*name);
                    }
                });
                ui.vertical(|ui| {
                    for (_name, _layer) in debug_layers.bool_layers.iter() {
                        ui.label(format!(
                            "{:?}",
                            _layer.grid.get(map_mouse_pos_cell).unwrap()
                        ));
                    }
                    for (_name, _layer) in debug_layers.float_layers.iter() {
                        ui.label(format!(
                            "{:?}",
                            _layer.grid.get(map_mouse_pos_cell).unwrap()
                        ));
                    }
                });
            })
        });
}
