//! Settings: global house style, on-screen display preferences and a live
//! manuscript preview. Defaults live in a config file outside any project; a
//! new project starts from them, and the current project can be updated to
//! match at any time.

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Vec2,
};
use portmanteau_core::{ElementStyles, ExportKind};

use crate::app::{PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::views::{
    content_frame, custom_styles_editor, element_preview, element_styles_editor,
    manuscript_preview_navigator, meta_line, page_head, primary_button, quiet_button,
    section_label, BUILTIN_ELEMENTS,
};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        "Defaults",
        "Settings",
        "The house style and on-screen reading preferences. New projects start from these defaults.",
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
        ScrollArea::vertical()
            .id_salt("settings_content")
            .auto_shrink([false, false])
            .max_height(height)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                tab_row(app, ui);
                ui.add_space(12.0);
                match app.settings_tab {
                    1 => display_tab(app, ui),
                    _ => preferences_tab(app, ui),
                }
            });
    });
}

/// The Settings tabs: Preferences (future options) and Display (editor display,
/// default styles and the manuscript preview).
fn tab_row(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if settings_tab_button(ui, "Preferences", app.settings_tab == 0) {
            app.settings_tab = 0;
        }
        ui.add_space(4.0);
        if settings_tab_button(ui, "Display", app.settings_tab == 1) {
            app.settings_tab = 1;
        }
    });
}

fn settings_tab_button(ui: &mut egui::Ui, text: &str, selected: bool) -> bool {
    let button = egui::Button::new(
        RichText::new(text)
            .font(sans(11.0))
            .color(if selected { TEXT } else { MUTED }),
    )
    .fill(if selected { ACCENT_SOFT } else { PANEL_2 })
    .stroke(Stroke::new(
        1.0,
        if selected {
            ACCENT.gamma_multiply(0.75)
        } else {
            BORDER_SOFT
        },
    ))
    .corner_radius(CornerRadius::same(6))
    .min_size(Vec2::new(112.0, 30.0));
    ui.add(button).clicked()
}

fn preferences_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Preferences");
            ui.add_space(4.0);
            ui.checkbox(
                &mut app.settings.spellcheck,
                RichText::new("Spell-checking in editors")
                    .font(sans(10.5))
                    .color(MUTED),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    "The Write toolbar shows a Spelling panel with suggestions. Save defaults to keep this setting.",
                )
                .font(sans(9.5))
                .color(FAINT),
            );
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            ui.label(
                RichText::new("More preferences coming soon.")
                    .font(sans(9.5))
                    .color(FAINT),
            );
        });
}

fn display_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut save = false;
    let mut apply = false;
    let mut reset = false;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Display").font(serif(21.0)).color(TEXT));
            ui.label(
                RichText::new(
                    "How the words look while you write, and the global house style new projects start from.",
                )
                .font(sans(10.0))
                .color(FAINT),
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
                MUTED,
                PANEL_2,
            );
        });
    });
    ui.add_space(8.0);

    let preview_settings = app.ws.project.settings_for(ExportKind::Pdf);
    manuscript_preview_navigator(
        &app.ws.project,
        &preview_settings,
        &mut app.settings_preview_page,
        ui,
    );
    ui.add_space(10.0);

    live_preview_panel(ui, &app.settings.styles);
    ui.add_space(10.0);

    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Editor display");
            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    "On-screen type sizes. The compiled page uses each output format's own element styles.",
                )
                .font(sans(9.5))
                .color(FAINT),
            );
            ui.add_space(6.0);
            egui::Grid::new("display_grid")
                .num_columns(4)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Prose size").font(sans(9.5)).color(MUTED));
                    ui.add(
                        egui::DragValue::new(&mut app.settings.display.prose_size)
                            .speed(0.1)
                            .range(9.0..=32.0)
                            .suffix(" pt"),
                    );
                    ui.label(RichText::new("Leading").font(sans(9.5)).color(MUTED));
                    ui.add(
                        egui::DragValue::new(&mut app.settings.display.prose_leading)
                            .speed(0.02)
                            .range(1.0..=2.6)
                            .suffix("×"),
                    );
                    ui.end_row();

                    ui.label(RichText::new("Script size").font(sans(9.5)).color(MUTED));
                    ui.add(
                        egui::DragValue::new(&mut app.settings.display.script_size)
                            .speed(0.1)
                            .range(7.0..=24.0)
                            .suffix(" pt"),
                    );
                    ui.label(RichText::new("Leading").font(sans(9.5)).color(MUTED));
                    ui.add(
                        egui::DragValue::new(&mut app.settings.display.script_leading)
                            .speed(0.02)
                            .range(1.0..=2.2)
                            .suffix("×"),
                    );
                    ui.end_row();
                });
        });

    ui.add_space(10.0);
    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Editing");
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Spell-checking in editors now lives on the Preferences tab.",
                )
                .font(sans(9.5))
                .color(FAINT),
            );
        });

    ui.add_space(10.0);
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    section_label(ui, "Default styles");
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(
                            "Type defaults for every manuscript element. Apply them to a project, then each output tab refines per format.",
                        )
                        .font(sans(9.5))
                        .color(FAINT),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Config · {}",
                            crate::settings::config_path().display()
                        ))
                        .font(mono(8.5))
                        .color(FAINT),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
            });
            ui.add_space(8.0);
            hairline(ui);
            ui.add_space(8.0);

            element_styles_editor(ui, &mut app.settings.styles, false, false);
            ui.add_space(2.0);
            custom_styles_editor(ui, &mut app.settings.styles);
        });

    ui.add_space(10.0);
    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("New projects copy these defaults automatically.")
                        .font(sans(9.5))
                        .color(MUTED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("Global settings · local file")
                            .font(sans(9.0))
                            .color(FAINT),
                    );
                });
            });
        });

    if reset {
        app.settings.styles = ElementStyles::default();
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
        app.ws.project.styles = app.settings.styles.clone();
        app.ws.dirty = true;
        app.toast(
            "Defaults applied",
            "The current project now uses the global element styles",
            ToastKind::Ok,
        );
    }
}

/// The live style preview: pick any built-in or custom element and see the
/// sample set in its current type and paragraph style. The picker's choice
/// lives in egui's temporary data so it survives the per-frame redraw.
fn live_preview_panel(ui: &mut egui::Ui, styles: &ElementStyles) {
    let id = egui::Id::new("settings_preview_element");
    let mut selected: String = ui
        .data(|data| data.get_temp(id))
        .unwrap_or_else(|| "body".to_string());

    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, "Live preview");
                ui.add_space(6.0);
                let display = BUILTIN_ELEMENTS
                    .iter()
                    .find(|(key, _)| *key == selected)
                    .map(|(_, label)| (*label).to_string())
                    .unwrap_or_else(|| selected.clone());
                egui::ComboBox::from_id_salt("settings_preview_element_pick")
                    .selected_text(RichText::new(display).font(sans(9.5)).color(TEXT))
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        for (key, label) in BUILTIN_ELEMENTS {
                            ui.selectable_value(&mut selected, key.to_string(), label);
                        }
                        if !styles.custom.is_empty() {
                            ui.separator();
                            for custom in &styles.custom {
                                ui.selectable_value(
                                    &mut selected,
                                    custom.name.clone(),
                                    format!("{} (custom)", custom.name),
                                );
                            }
                        }
                    });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    meta_line(ui, &["live as you edit below"]);
                });
            });
            ui.add_space(6.0);
            let style = styles
                .element(&selected)
                .or_else(|| styles.custom_named(&selected).map(|custom| &custom.style));
            match style {
                Some(style) => element_preview(ui, style),
                None => {
                    selected = "body".to_string();
                    element_preview(ui, &styles.body);
                }
            }
            ui.add_space(5.0);
            ui.label(
                RichText::new(
                    "Approximate on-screen proof. The compiled page uses the same font, size, leading, alignment and indents, but line breaking and page margins may differ.",
                )
                .font(sans(8.5))
                .color(FAINT),
            );
        });
    ui.data_mut(|data| data.insert_temp(id, selected));
}
