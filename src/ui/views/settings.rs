//! Settings: the global house style, edited beside a live manuscript preview.
//! Defaults live in a config file outside any project; a new project starts
//! from them, and the current project can be updated to match at any time.

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Vec2,
};
use portmanteau_core::{CompileType, ElementStyles, ExportKind, StyleSheet};

use crate::app::{PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::views::{
    content_frame, manuscript_preview_navigator, style_editor,
    meta_line, page_head, primary_button, quiet_button, section_label, style_sheets_editor,
    BUILTIN_ELEMENTS,
};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        "Defaults",
        "Settings",
        "The house style new projects start from, with a live manuscript preview.",
        |ui| {
            meta_line(
                ui,
                &[&format!(
                    "{} built-in + {} custom element styles",
                    BUILTIN_ELEMENTS.len(),
                    app.settings.styles.custom.len()
                )],
            );
        },
    );

    content_frame().show(ui, |ui| {
        let height = (ui.available_height() - 4.0).max(420.0);
        ui.set_width(ui.available_width());
        tab_row(app, ui);
        ui.add_space(12.0);
        match app.settings_tab {
            1 => display_tab(app, ui),
            2 => super::appearance::show(app, ui),
            _ => {
                ScrollArea::vertical()
                    .id_salt("settings_content")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        preferences_tab(app, ui);
                    });
            }
        }
    });
}

/// The Settings tabs: Preferences (future options) and Display (default styles
/// and the manuscript preview).
fn tab_row(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if settings_tab_button(ui, "Preferences", app.settings_tab == 0) {
            app.settings_tab = 0;
        }
        ui.add_space(4.0);
        if settings_tab_button(ui, "Display", app.settings_tab == 1) {
            app.settings_tab = 1;
        }
        ui.add_space(4.0);
        if settings_tab_button(ui, "Appearance", app.settings_tab == 2) {
            app.settings_tab = 2;
        }
    });
}

fn settings_tab_button(ui: &mut egui::Ui, text: &str, selected: bool) -> bool {
    let button = egui::Button::new(
        RichText::new(text)
            .font(ui_body())
            .color(if selected { TEXT() } else { MUTED() }),
    )
    .fill(if selected { ACCENT_SOFT() } else { RAISED() })
    .stroke(Stroke::new(
        1.0,
        if selected {
            ACCENT().gamma_multiply(0.75)
        } else {
            BORDER()
        },
    ))
    .corner_radius(CornerRadius::same(6))
    .min_size(Vec2::new(112.0, 30.0));
    ui.add(button).clicked()
}

fn preferences_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut changed = false;
    let width = ui.available_width().min(704.0);
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(width);
            section_label(ui, "Writing");
            ui.add_space(4.0);
            changed |= ui
                .checkbox(
                    &mut app.settings.spellcheck,
                    RichText::new("Spell-checking in editors")
                        .font(ui_body())
                        .color(MUTED()),
                )
                .changed();
            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    "The Write toolbar shows a Spelling panel with suggestions. Changes here are saved automatically.",
                )
                .font(ui_small())
                .color(MUTED()),
            );
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            section_label(ui, "Software updates");
            ui.add_space(4.0);
            changed |= ui
                .checkbox(
                    &mut app.settings.updates.auto_check,
                    RichText::new("Check for updates automatically")
                        .font(ui_body())
                        .color(MUTED()),
                )
                .changed();
            ui.add_space(2.0);
            let last = app
                .settings
                .updates
                .last_checked
                .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
                .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "never".into());
            ui.label(
                RichText::new(format!(
                    "At most once a day, at startup. It only asks GitHub for the newest release number; nothing about you or your projects is sent. Last checked: {last}."
                ))
                .font(ui_small())
                .color(MUTED()),
            );
            ui.add_space(4.0);
            if ui.button("Check now").clicked() {
                app.updater.panel_open = true;
                app.updater.start(ui.ctx(), true);
            }
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            section_label(ui, "Interface text size");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (label, value) in crate::theme::UI_SCALE_PRESETS {
                    let active = (app.settings.ui_scale - value).abs() < 0.04;
                    if ui.selectable_label(active, label).clicked() && !active {
                        app.settings.ui_scale = value;
                        crate::theme::set_ui_scale(value);
                        crate::theme::install(ui.ctx());
                        changed = true;
                    }
                }
            });
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            section_label(ui, "Spelling language");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let current = app.settings.language.clone();
                let selected_label = crate::spellcheck::LANGUAGES
                    .iter()
                    .find(|(code, _)| *code == current)
                    .map(|(_, label)| *label)
                    .unwrap_or("English (US)");
                let mut change: Option<&'static str> = None;
                egui::ComboBox::from_id_salt("spelling_language")
                    .selected_text(RichText::new(selected_label).font(ui_body()).color(TEXT()))
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for (code, label) in crate::spellcheck::LANGUAGES {
                            if ui.selectable_label(current == *code, *label).clicked() {
                                change = Some(*code);
                            }
                        }
                    });
                if let Some(code) = change {
                    app.set_spell_language(code);
                }
                ui.label(
                    RichText::new("Applies to every editor and new projects.")
                        .font(ui_small())
                        .color(MUTED()),
                );
            });
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            section_label(ui, "Workspace");
            ui.add_space(4.0);
            changed |= ui
                .checkbox(
                    &mut app.settings.show_binder,
                    RichText::new("Show the binder in Write")
                        .font(ui_body())
                        .color(MUTED()),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.settings.show_inspector,
                    RichText::new("Show the inspector in Write")
                        .font(ui_body())
                        .color(MUTED()),
                )
                .changed();
            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    "The sidebar layout is remembered between launches. Focus mode (F11) hides both panels without changing this preference.",
                )
                .font(ui_small())
                .color(MUTED()),
            );
        });
    if changed {
        if crate::settings::save_settings(&app.settings).is_ok() {
            app.toast(
                "Preferences saved",
                crate::settings::config_path().display().to_string(),
                ToastKind::Info,
            );
        }
    }
}

/// Two columns, mirroring Compile: the default element styles on the left and
/// the manuscript preview on the right. The preview uses the defaults as they
/// are edited, so it is the only proof on this tab.
fn display_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut save = false;
    let mut apply = false;
    let mut reset = false;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Display").font(serif(21.0)).color(TEXT()));
            ui.label(
                RichText::new(
                    "The house style new projects start from — and the same type the editors use.",
                )
                .font(ui_small())
                .color(MUTED()),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            pill(
                ui,
                &format!(
                    "{} built-in + {} custom",
                    BUILTIN_ELEMENTS.len(),
                    app.settings.styles.custom.len()
                ),
                MUTED(),
                RAISED(),
            );
        });
    });
    ui.add_space(8.0);

    let height = ui.available_height().max(420.0);
    ui.horizontal_top(|ui| {
        // ---- Left column: the default element styles -------------------
        ui.allocate_ui_with_layout(
            Vec2::new(440.0, height),
            Layout::top_down(Align::Min),
            |ui| {
                ScrollArea::vertical()
                    .id_salt("settings_default_styles")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        Frame::new()
                            .fill(PANEL())
                            .stroke(Stroke::new(1.0, BORDER()))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::symmetric(14, 11))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                section_label(ui, "Display and default styles");
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(
                                        "Type defaults for every manuscript element. The preview uses them; apply them to a project, then each output tab refines per format.",
                                    )
                                    .font(ui_small())
                                    .color(MUTED()),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "Config · {}",
                                        crate::settings::config_path().display()
                                    ))
                                    .font(mono(8.5))
                                    .color(MUTED()),
                                );
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if primary_button(ui, "Save defaults").clicked() {
                                        save = true;
                                    }
                                    if quiet_button(ui, "Apply to current project").clicked() {
                                        apply = true;
                                    }
                                    if quiet_button(ui, "Reset defaults").clicked() {
                                        reset = true;
                                    }
                                });
                                ui.add_space(8.0);
                                hairline(ui);
                                ui.add_space(8.0);

                                let base_styles_changed = style_editor(
                                    ui,
                                    &mut app.settings.styles,
                                    "settings",
                                    CompileType::Prose,
                                );
                                if base_styles_changed {
                                    app.settings.sync_sheet_formats_from_element_styles();
                                }
                                ui.add_space(8.0);
                                section_label(ui, "Writing behavior · per-format styles (film, TV, stage)");
                                if style_sheets_editor(ui, &mut app.settings.style_sheets) {
                                    app.settings.sync_element_styles_from_sheets();
                                }
                            });
                    });
            },
        );
        ui.add_space(16.0);

        // ---- Right column: preview the manuscript with these defaults ---
        let preview_width = ui.available_width();
        ui.allocate_ui_with_layout(
            Vec2::new(preview_width, height),
            Layout::top_down(Align::Min),
            |ui| {
                ScrollArea::vertical()
                    .id_salt("settings_preview")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let mut settings = app.ws.project.settings_for(ExportKind::Pdf);
                        settings.styles = app.settings.styles.clone();
                        manuscript_preview_navigator(
                            &app.ws.project,
                            &settings,
                            &mut app.settings_preview_page,
                            height,
                            "settings",
                            ui,
                        );
                    });
            },
        );
    });

    if reset {
        app.settings.styles = ElementStyles::default();
        app.settings.style_sheets = StyleSheet::industry_defaults(&app.settings.styles);
        app.toast(
            "Defaults reset",
            "Built-in element styles restored — save to keep them",
            ToastKind::Info,
        );
    }
    if save {
        match crate::settings::save_settings(&app.settings) {
            Ok(()) => app.toast(
                "Defaults saved",
                crate::settings::config_path().display().to_string(),
                ToastKind::Ok,
            ),
            Err(e) => app.toast("Could not save defaults", e, ToastKind::Error),
        }
    }
    if apply {
        app.ws.project.styles = app.settings.element_styles_for_new_project();
        app.ws.project.style_sheets = app.settings.style_sheets_for_new_project();
        app.ws.dirty = true;
        app.toast(
            "Defaults applied",
            "The current project now uses the global element styles",
            ToastKind::Ok,
        );
    }
}
