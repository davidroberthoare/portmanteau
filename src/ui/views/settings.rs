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
    content_frame, element_styles_editor, meta_line, page_head, primary_button, quiet_button,
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
                    "{} global element styles",
                    ELEMENT_LABELS.len()
                )],
            );
        },
    );

    content_frame().show(ui, |ui| {
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

                let height = (ui.available_height() - 4.0).max(300.0);
                ScrollArea::vertical()
                    .id_salt("settings_styles")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        element_styles_editor(ui, &mut app.settings.styles);
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

/// The element count shown in the header. Kept local to this screen so the
/// label list stays the single source of truth in `views/mod.rs`.
const ELEMENT_LABELS: [&str; 15] = [
    "Title page title",
    "Title page subtitle",
    "Title page author",
    "Chapter heading",
    "Body",
    "Scene heading",
    "Action",
    "Character",
    "Parenthetical",
    "Dialogue",
    "Transition",
    "Act",
    "Scene",
    "Stage direction",
    "General",
];
