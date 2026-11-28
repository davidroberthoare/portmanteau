//! A small sample of the app drawn with the current theme, so each colour can
//! be seen where it is used while it is being edited.

use eframe::egui::{
    self, text::LayoutJob, Color32, CornerRadius, Frame, Margin, RichText, Stroke, TextFormat,
};

use crate::theme::*;
use crate::i18n::tr;

fn run(job: &mut LayoutJob, text: &str, font: egui::FontId, color: Color32) {
    job.append(text, 0.0, TextFormat { font_id: font, color, ..Default::default() });
}

fn page_line(ui: &mut egui::Ui, parts: &[(&str, Color32, Option<Color32>, bool)]) {
    let mut job = LayoutJob::default();
    for (text, color, background, underline) in parts {
        job.append(
            text,
            0.0,
            TextFormat {
                font_id: serif(14.0),
                color: *color,
                background: background.unwrap_or(Color32::TRANSPARENT),
                underline: if *underline { Stroke::new(1.2, *color) } else { Stroke::NONE },
                ..Default::default()
            },
        );
    }
    ui.add(egui::Label::new(job).wrap());
}

pub fn show(ui: &mut egui::Ui) {
    Frame::new()
        .fill(BG())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Top bar
            Frame::new()
                .fill(PANEL())
                .stroke(Stroke::new(1.0, BORDER()))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("Portmanteau")).font(serif(15.0)).color(TEXT()));
                        for name in ["File", "Edit"] {
                            ui.add(egui::Button::new(RichText::new(name).font(ui_small()).color(TEXT())).fill(RAISED()).stroke(Stroke::new(1.0, BORDER())));
                        }
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(80.0, 5.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, CornerRadius::same(3), RAISED());
                        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(32.0, 5.0)), CornerRadius::same(3), ACCENT());
                        ui.label(RichText::new(tr("608 / 1200 today")).font(ui_small()).color(MUTED()));
                    });
                });
            ui.add_space(8.0);
            Frame::new()
                .fill(PANEL())
                .stroke(Stroke::new(1.0, BORDER()))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    section_label(ui, tr("Binder"));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("▸ Draft")).font(ui_body()).color(FOLDER()));
                        Frame::new().fill(ACCENT_SOFT()).corner_radius(CornerRadius::same(4)).inner_margin(Margin::symmetric(6, 3)).show(ui, |ui| {
                            ui.label(RichText::new(tr("Chapter One")).font(ui_body()).color(TEXT()));
                            ui.label(RichText::new("328").font(ui_small()).color(MUTED()));
                        });
                        ui.label(RichText::new(tr("Chapter Two")).font(ui_body()).color(TEXT()));
                        ui.label(RichText::new("213").font(ui_small()).color(MUTED()));
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        pill(ui, "Revised", SUCCESS(), ACCENT_SOFT());
                        pill(ui, "Draft", INFO(), ACCENT_SOFT());
                        pill(ui, "Script", SPECIAL(), ACCENT_SOFT());
                        pill(ui, "Warning", WARNING(), ACCENT_SOFT());
                        pill(ui, "Error", DANGER(), ACCENT_SOFT());
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add(egui::Button::new(RichText::new(tr("Hovered")).font(ui_small()).color(TEXT())).fill(HOVER()).stroke(Stroke::new(1.0, BORDER())));
                        ui.add(egui::Button::new(RichText::new(tr("Compile")).font(ui_small()).color(PANEL())).fill(ACCENT()));
                    });
                });
            ui.add_space(8.0);
            {
                // Page
                Frame::new()
                    .fill(PAPER())
                    .stroke(Stroke::new(1.0, BORDER()))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let mut head = LayoutJob::default();
                        run(&mut head, "Chapter One", serif(20.0), HEADING());
                        ui.add(egui::Label::new(head));
                        ui.add_space(4.0);
                        let text = PAPER_TEXT();
                        let spelled = Some(Color32::TRANSPARENT);
                        page_line(ui, &[
                            ("The tide went out that morning, and the ", text, None, false),
                            ("harbour", DANGER(), spelled, true),
                            (" wall was ", text, None, false),
                            ("bare", text, Some(FIND_MATCH()), false),
                            (". She read ", text, None, false),
                            ("the old survey", INFO(), None, true),
                            (" twice.", text, None, false),
                        ]);
                        ui.add_space(8.0);
                        page_line(ui, &[("INT. LIGHTHOUSE - NIGHT", SCENE_HEADING(), None, false)]);
                        page_line(ui, &[("Mara climbs the last stair, ", text, None, false), ("bare", text, Some(FIND_CURRENT()), false), (" feet cold.", text, None, false)]);
                        ui.add_space(4.0);
                        page_line(ui, &[("MARA", CHARACTER(), None, false)]);
                        page_line(ui, &[("It's on the map. It was always on the map.", text, None, false)]);
                    });
            }
        });
}
