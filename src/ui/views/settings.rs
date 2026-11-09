//! Settings: the global house style. Defaults live in a config file outside
//! any project; a new project starts from them, and the current project can
//! be updated to match at any time.

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke,
};
use portmanteau_core::ElementStyles;

use crate::app::{PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::views::{
    content_frame, custom_styles_editor, element_preview, element_styles_editor, meta_line,
    page_head, primary_button, quiet_button, BUILTIN_ELEMENTS,
};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut save = false;
    let mut apply = false;
    let mut reset = false;

    page_head(
        ui,
        "Defaults",
        "Settings",
        "Set the house style once. New projects start from these defaults; the current project can adopt them at any time.",
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
        preview_panel(ui, &app.settings.styles);
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
                                "Type defaults for every manuscript element. Apply them to a project, then Compile refines per format.",
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

                let height = (ui.available_height() - 176.0).max(240.0);
                ScrollArea::vertical()
                    .id_salt("settings_styles")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        element_styles_editor(ui, &mut app.settings.styles);
                    });
                ui.add_space(4.0);
                ScrollArea::vertical()
                    .id_salt("settings_custom_styles")
                    .auto_shrink([false, true])
                    .max_height((ui.available_height() - 4.0).max(90.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        custom_styles_editor(ui, &mut app.settings.styles);
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
fn preview_panel(ui: &mut egui::Ui, styles: &ElementStyles) {
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
