use std::{collections::HashMap, env, isize};

use egui::{ComboBox, RichText, ScrollArea};
use mapgen_core::random::Seed;
use mapgen_core::walker::Pulse;
use tinyfiledialogs;

use crate::config::save_config;
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
                if ui.button("random seed").clicked() {
                    editor.user_seed = Seed::random();
                }
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
            if ui.button("generator").clicked() {
                let cwd = env::current_dir().unwrap();

                let initial_path = cwd
                    .join(editor.config.generator.current.clone() + ".json")
                    .to_string_lossy()
                    .to_string();

                if let Some(path_out) = tinyfiledialogs::save_file_dialog("save generator config", &initial_path) {
                    save_config(editor.config.generator.get(), &path_out).unwrap();
                }
            };

            if ui.button("walker").clicked() {
                let cwd = env::current_dir().unwrap();

                let initial_path = cwd
                    .join(editor.config.walker.current.clone() + ".json")
                    .to_string_lossy()
                    .to_string();

                if let Some(path_out) =
                    tinyfiledialogs::save_file_dialog("save walker config", &initial_path)
                {
                    save_config(editor.config.walker.get(), &path_out).unwrap();
                }
            };

            if ui.button("waypoints").clicked() {
                let cwd = env::current_dir().unwrap();

                let initial_path = cwd
                    .join(editor.config.waypoints.current.clone() + ".json")
                    .to_string_lossy()
                    .to_string();

                if let Some(path_out) =
                    tinyfiledialogs::save_file_dialog("save waypoints config", &initial_path)
                {
                    save_config(editor.config.waypoints.get(), &path_out).unwrap();
                }
            };
        });

        ComboBox::from_label("load generator config:")
            .selected_text(format!("{:}", editor.config.generator.current))
            .show_ui(ui, |ui| {
                for (name, _cfg) in editor.config.generator.all.iter() {
                    ui.selectable_value(&mut editor.config.generator.current, name.clone(), name);
                }
            });
        ComboBox::from_label("load walker config:")
            .selected_text(format!("{:}", editor.config.walker.current))
            .show_ui(ui, |ui| {
                for (name, _cfg) in editor.config.walker.all.iter() {
                    ui.selectable_value(&mut editor.config.walker.current, name.clone(), name);
                }
            });
        ComboBox::from_label("load waypoints config:")
            .selected_text(format!("{:}", editor.config.waypoints.current))
            .show_ui(ui, |ui| {
                for (name, _cfg) in editor.config.waypoints.all.iter() {
                    ui.selectable_value(&mut editor.config.waypoints.current, name.clone(), name);
                }
            });

        ui.horizontal(|ui| {
            ui.checkbox(&mut editor.edit_gen_config, "edit generation");
            ui.checkbox(&mut editor.edit_wal_config, "edit walker");
            ui.checkbox(&mut editor.edit_way_config, "edit waypoints");
        });

        ScrollArea::vertical().show(ui, |ui| {
            // =======================================[ GENERATION CONFIG EDIT ]===================================
            if editor.edit_gen_config {
                ui.separator();

                field_edit_widget(
                    ui,
                    &mut editor.config.generator.get_mut().platform_distance_bounds,
                    edit_range_usize,
                    "platform distances",
                    true,
                );
                field_edit_widget(
                    ui,
                    &mut editor.config.generator.get_mut().max_distance,
                    edit_f32_wtf,
                    "max distance",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.config.generator.get_mut().waypoint_reached_dist,
                    edit_usize,
                    "waypoint reached dist",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.config.generator.get_mut().skip_length_bounds,
                    edit_range_usize,
                    "skip length bounds",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.config.generator.get_mut().skip_min_spacing_sqr,
                    edit_usize,
                    "skip min spacing sqr",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.config.generator.get_mut().min_freeze_size,
                    edit_usize,
                    "min freeze size",
                    false,
                );
            }

            // =======================================[ WALKER CONFIG EDIT ]===================================
            if editor.edit_wal_config {
                field_edit_widget(
                    ui,
                    &mut editor.config.walker.get_mut().inner_rad_mut_prob,
                    edit_f32_prob,
                    "inner rad mut prob",
                    true,
                );
                field_edit_widget(
                    ui,
                    &mut editor.config.walker.get_mut().inner_size_mut_prob,
                    edit_f32_prob,
                    "inner size mut prob",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.config.walker.get_mut().outer_rad_mut_prob,
                    edit_f32_prob,
                    "outer rad mut prob",
                    true,
                );
                field_edit_widget(
                    ui,
                    &mut editor.config.walker.get_mut().outer_size_mut_prob,
                    edit_f32_prob,
                    "outer size mut prob",
                    true,
                );

                field_edit_widget(
                    ui,
                    &mut editor.config.walker.get_mut().momentum_prob,
                    edit_f32_prob,
                    "momentum prob",
                    true,
                );

                ui.add_enabled_ui(editor.is_setup(), |ui| {
                    random_dist_cfg_edit(
                        ui,
                        &mut editor.config.walker.get_mut().shift_weights,
                        None::<fn(&mut Ui, &mut ShiftDirection)>, // TODO: this is stupid wtwf
                        "step weights",
                        false,
                        true,
                    );
                });

                ui.add_enabled_ui(editor.is_setup(), |ui| {
                    random_dist_cfg_edit(
                        ui,
                        &mut editor.config.walker.get_mut().inner_size_probs,
                        Some(edit_usize),
                        "inner size probs",
                        true,
                        false,
                    );

                    random_dist_cfg_edit(
                        ui,
                        &mut editor.config.walker.get_mut().outer_margin_probs,
                        Some(edit_usize),
                        "outer margin probs",
                        true,
                        false,
                    );

                    random_dist_cfg_edit(
                        ui,
                        &mut editor.config.walker.get_mut().circ_probs,
                        Some(edit_f32_prob),
                        "circularity probs",
                        true,
                        false,
                    );
                });

                let pulse_enabled = editor.config.walker.get_mut().pulse.is_some();
                let pulse_button = if !pulse_enabled {
                    "enable pulse"
                } else {
                    "disable pulse"
                };

                if ui.button(pulse_button).clicked() {
                    if pulse_enabled {
                        editor.config.walker.get_mut().pulse = None;
                    } else {
                        editor.config.walker.get_mut().pulse = Some(Pulse {
                            straight_delay: 10,
                            corner_delay: 5,
                            max_kernel_size: 1,
                        });
                    }
                }

                if let Some(pulse) = &mut editor.config.walker.get_mut().pulse {
                    field_edit_widget(
                        ui,
                        &mut pulse.straight_delay,
                        edit_usize,
                        "pulse straight delay",
                        true,
                    );
    
                    field_edit_widget(
                        ui,
                        &mut pulse.corner_delay,
                        edit_usize,
                        "pulse corner delay",
                        false,
                    );
    
                    field_edit_widget(
                        ui,
                        &mut pulse.max_kernel_size,
                        edit_usize,
                        "pulse max kernel",
                        false,
                    );
                }

                field_edit_widget(
                    ui,
                    &mut editor.config.walker.get_mut().fade_steps,
                    edit_usize,
                    "fade steps",
                    false,
                );

                field_edit_widget(
                    ui,
                    &mut editor.config.walker.get_mut().fade_max_size,
                    edit_usize,
                    "fade max size",
                    false,
                );

                field_edit_widget(
                    ui,
                    &mut editor.config.walker.get_mut().fade_min_size,
                    edit_usize,
                    "fade min size",
                    false,
                );
            }

            // =======================================[ WAYPOINTS CONFIG EDIT ]===================================
            if editor.edit_way_config {
                ui.add_enabled_ui(editor.is_setup(), |ui| {
                    vec_edit_widget(
                        ui,
                        &mut editor.config.waypoints.get_mut().waypoints,
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
                &editor.config.generator.current
            )));
            ui.add(Label::new(format!(
                "config: {:?}",
                &editor.config.walker.current
            )));
            ui.add(Label::new(format!(
                "config: {:?}",
                &editor.config.waypoints.current
            )));
        });
}