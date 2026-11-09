//! The non-writing workspaces: corkboard, outline, research, development,
//! characters, version history, compilation and the command desk.

pub mod agent;
pub mod characters;
pub mod compile;
pub mod corkboard;
pub mod develop;
pub mod history;
pub mod outline;
pub mod research;
pub mod settings;

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, TextEdit, Vec2,
};
use portmanteau_core::ElementStyles;

use crate::app::{PortmanteauApp, View};
use crate::theme::*;

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    match app.view {
        View::Corkboard => corkboard::show(app, ui),
        View::Outline => outline::show(app, ui),
        View::Research => research::show(app, ui),
        View::Develop => develop::show(app, ui),
        View::Characters => characters::show(app, ui),
        View::History => history::show(app, ui),
        View::Compile => compile::show(app, ui),
        View::Agent => agent::show(app, ui),
        View::Settings => settings::show(app, ui),
        View::Write => {}
    }
}

/// Shared page header with a kicker, a large title, supporting copy and an
/// optional right-hand action area.
pub fn page_head<R>(
    ui: &mut egui::Ui,
    kicker: &str,
    title: &str,
    subtitle: &str,
    right: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let mut result = None;
    Frame::new()
        .fill(BG)
        .inner_margin(Margin {
            left: 26,
            right: 26,
            top: 20,
            bottom: 15,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(kicker.to_uppercase())
                            .font(sans(9.5))
                            .color(ACCENT)
                            .extra_letter_spacing(1.5),
                    );
                    ui.add_space(2.0);
                    ui.label(RichText::new(title).font(serif(26.0)).color(TEXT));
                    ui.add_space(2.0);
                    ui.label(RichText::new(subtitle).font(sans(11.0)).color(MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    result = Some(right(ui));
                });
            });
        });
    hairline(ui);
    result.unwrap()
}

pub fn content_frame() -> Frame {
    Frame::new().inner_margin(Margin {
        left: 26,
        right: 26,
        top: 18,
        bottom: 12,
    })
}

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(text)
                .font(sans(11.5))
                .color(Color32::from_rgb(29, 24, 17)),
        )
        .fill(ACCENT)
        .corner_radius(CornerRadius::same(6))
        .min_size(Vec2::new(0.0, 29.0)),
    )
}

pub fn quiet_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).font(sans(10.5)).color(MUTED))
            .fill(PANEL_2)
            .stroke(Stroke::new(1.0, BORDER_SOFT))
            .corner_radius(CornerRadius::same(5))
            .min_size(Vec2::new(0.0, 26.0)),
    )
}

/// A small metadata line used at the foot of cards and rows.
pub fn meta_line(ui: &mut egui::Ui, parts: &[&str]) {
    let text = parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("  ·  ");
    ui.label(RichText::new(text).font(sans(9.5)).color(FAINT));
}

const ELEMENT_ROWS: [(&str, &str); 15] = [
    ("Title page title", "title_page_title"),
    ("Title page subtitle", "title_page_subtitle"),
    ("Title page author", "title_page_author"),
    ("Chapter heading", "chapter_heading"),
    ("Body", "body"),
    ("Scene heading", "scene_heading"),
    ("Action", "action"),
    ("Character", "character"),
    ("Parenthetical", "parenthetical"),
    ("Dialogue", "dialogue"),
    ("Transition", "transition"),
    ("Act", "act"),
    ("Scene", "scene"),
    ("Stage direction", "stage_direction"),
    ("General", "general"),
];

/// The shared per-element style editor: one rounded row per manuscript
/// element with font, size, leading and indent. Used by Compile (the current
/// project) and Settings (the global defaults).
pub fn element_styles_editor(ui: &mut egui::Ui, styles: &mut ElementStyles) {
    for (label, key) in ELEMENT_ROWS {
        if let Some(element) = styles.element_mut(key) {
            element_row(ui, label, element);
            ui.add_space(4.0);
        }
    }
}

fn element_row(ui: &mut egui::Ui, label: &str, element: &mut portmanteau_core::ElementStyle) {
    Frame::new()
        .fill(PANEL_2)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(118.0, 18.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.label(RichText::new(label).font(sans(9.5)).color(TEXT));
                    },
                );
                ui.add(
                    TextEdit::singleline(&mut element.font)
                        .hint_text("Font family")
                        .font(mono(9.0))
                        .desired_width(145.0),
                );
                ui.add_space(4.0);
                ui.label(RichText::new("Size").font(sans(8.5)).color(FAINT));
                ui.add(
                    egui::DragValue::new(&mut element.size)
                        .speed(0.1)
                        .range(4.0..=72.0)
                        .suffix(" pt"),
                );
                ui.add_space(4.0);
                ui.label(RichText::new("Leading").font(sans(8.5)).color(FAINT));
                ui.add(
                    egui::DragValue::new(&mut element.leading)
                        .speed(0.02)
                        .range(0.5..=3.0)
                        .suffix("×"),
                );
                ui.add_space(4.0);
                ui.label(RichText::new("Indent").font(sans(8.5)).color(FAINT));
                ui.add(
                    egui::DragValue::new(&mut element.indent)
                        .speed(0.5)
                        .range(0.0..=800.0)
                        .suffix(" pt"),
                );
            });
        });
}


