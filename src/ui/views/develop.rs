//! Development: the idea board and story structure, where a draft is argued
//! into shape before it is written.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};

use portmanteau_core::{new_id, Beat};

use crate::app::PortmanteauApp;
use crate::theme::*;
use crate::ui::views::{content_frame, meta_line, page_head, primary_button, quiet_button};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut add_idea = false;
    page_head(
        ui,
        "Invention",
        "Develop",
        "Premise, images, voice and the beats that hold the story up.",
        |ui| {
            if primary_button(ui, "+ New idea").clicked() {
                add_idea = true;
            }
        },
    );

    content_frame().show(ui, |ui| {
        // The compass: logline and theme in one quiet strip.
        Frame::new()
            .fill(PANEL)
            .stroke(Stroke::new(1.0, BORDER_SOFT))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(16, 11))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        section_label(ui, "The spine");
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(if app.ws.project.logline.is_empty() {
                                "No logline yet. One sentence: who wants what, and what stands in the way?"
                            } else {
                                &app.ws.project.logline
                            })
                            .font(serif(13.0))
                            .color(if app.ws.project.logline.is_empty() { FAINT } else { TEXT }),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let done = app.ws.project.beats.iter().filter(|b| b.done).count();
                        meta_line(
                            ui,
                            &[
                                &format!("{} ideas", app.ws.project.ideas.len()),
                                &format!("{done}/{} beats", app.ws.project.beats.len()),
                            ],
                        );
                    });
                });
            });
        ui.add_space(14.0);

        let height = (ui.available_height() - 8.0).max(360.0);
        ui.horizontal_top(|ui| {
            // ---- Idea board -------------------------------------------------
            ui.allocate_ui_with_layout(
                Vec2::new((ui.available_width() * 0.58).max(420.0), height),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.horizontal(|ui| {
                        section_label(ui, "Idea board");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if quiet_button(ui, "+ Add").clicked() {
                                add_idea = true;
                            }
                        });
                    });
                    ui.add_space(6.0);
                    ScrollArea::vertical()
                        .id_salt("idea_scroll")
                        .auto_shrink([false, false])
                        .max_height(height - 34.0)
                        .show(ui, |ui| {
                            let ideas = app.ws.project.ideas.clone();
                            let gap = 10.0;
                            let cols = ((ui.available_width() / 210.0).floor() as usize).clamp(1, 3);
                            let width =
                                (ui.available_width() - gap * (cols - 1) as f32) / cols as f32;
                            for chunk in ideas.chunks(cols) {
                                ui.horizontal_top(|ui| {
                                    for idea in chunk {
                                        idea_card(app, ui, idea, width);
                                        ui.add_space(gap);
                                    }
                                });
                                ui.add_space(gap);
                            }
                            if ideas.is_empty() {
                                ui.label(
                                    RichText::new("No ideas pinned yet.")
                                        .font(sans(10.5))
                                        .color(FAINT),
                                );
                            }
                        });
                },
            );
            ui.add_space(14.0);
            // ---- Story beats ------------------------------------------------
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                beats_panel(app, ui, height);
            });
        });
    });

    if add_idea {
        let res = app.ws.execute(
            "idea.add",
            serde_json::json!({"title": "Untitled idea", "body": "", "kind": "Idea"}),
        );
        if res.ok {
            if let Some(id) = res.data.get("id").and_then(|v| v.as_str()) {
                app.selected_idea = Some(id.to_string());
            }
            app.ws.dirty = true;
        }
    }
}

fn idea_card(
    app: &mut PortmanteauApp,
    ui: &mut egui::Ui,
    idea: &portmanteau_core::IdeaCard,
    width: f32,
) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 132.0), Sense::click());
    let colors = [ACCENT, SAGE, PURPLE, BLUE, RED, Color32::from_rgb(170, 158, 120)];
    let color = colors[idea.color as usize % colors.len()];
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(7),
        if resp.hovered() { PANEL_3 } else { PANEL_2 },
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(7),
        Stroke::new(1.0, if resp.hovered() { BORDER } else { BORDER_SOFT }),
        egui::StrokeKind::Inside,
    );
    ui.painter().rect_filled(
        egui::Rect::from_min_size(rect.min + Vec2::new(1.0, 1.0), Vec2::new(width - 2.0, 3.0)),
        CornerRadius {
            nw: 6,
            ne: 6,
            sw: 0,
            se: 0,
        },
        color,
    );
    ui.painter().text(
        rect.left_top() + Vec2::new(11.0, 11.0),
        egui::Align2::LEFT_TOP,
        idea.kind.to_uppercase(),
        sans(8.0),
        color,
    );
    if idea.pinned {
        ui.painter().text(
            rect.right_top() + Vec2::new(-11.0, 10.0),
            egui::Align2::RIGHT_TOP,
            "PINNED",
            sans(7.5),
            FAINT,
        );
    }
    let title = ui.painter().layout(
        idea.title.clone(),
        serif(14.0),
        TEXT,
        width - 22.0,
    );
    ui.painter()
        .galley(rect.left_top() + Vec2::new(11.0, 27.0), title, TEXT);
    let body_text: String = if idea.body.chars().count() > 150 {
        format!("{}…", idea.body.chars().take(147).collect::<String>())
    } else {
        idea.body.clone()
    };
    let body = ui.painter().layout(body_text, sans(9.5), MUTED, width - 22.0);
    ui.painter()
        .galley(rect.left_top() + Vec2::new(11.0, 52.0), body, MUTED);

    if resp.clicked() {
        app.selected_idea = Some(idea.id.clone());
    }
    resp.on_hover_text("Open this card");
}

fn beats_panel(app: &mut PortmanteauApp, ui: &mut egui::Ui, height: f32) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, "Story beats");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let done = app.ws.project.beats.iter().filter(|b| b.done).count();
                    meta_line(ui, &[&format!("{done} done")]);
                });
            });
            ui.add_space(7.0);
            let mut add_act: Option<u8> = None;
            let mut dirty = false;
            ScrollArea::vertical()
                .id_salt("beats_scroll")
                .auto_shrink([false, false])
                .max_height(height - 48.0)
                .show(ui, |ui| {
                    for act in 1u8..=3 {
                        let act_name = match act {
                            1 => "Act one · setup",
                            2 => "Act two · pressure",
                            _ => "Act three · reckoning",
                        };
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(act_name.to_uppercase())
                                    .font(sans(8.5))
                                    .color(ACCENT)
                                    .extra_letter_spacing(1.0),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("+").font(sans(12.0)).color(FAINT),
                                        )
                                        .frame(false),
                                    )
                                    .on_hover_text("Add a beat to this act")
                                    .clicked()
                                {
                                    add_act = Some(act);
                                }
                            });
                        });
                        ui.add_space(4.0);
                        let indices: Vec<usize> = app
                            .ws
                            .project
                            .beats
                            .iter()
                            .enumerate()
                            .filter(|(_, b)| b.act == act)
                            .map(|(i, _)| i)
                            .collect();
                        for i in indices {
                            let beat = &mut app.ws.project.beats[i];
                            Frame::new()
                                .fill(if beat.done { SAGE_SOFT } else { PANEL_2 })
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(9, 7))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut beat.done, "").changed() {
                                            dirty = true;
                                        }
                                        if ui
                                            .add(
                                                TextEdit::singleline(&mut beat.title)
                                                    .font(sans(10.0))
                                                    .text_color(if beat.done { SAGE } else { TEXT })
                                                    .frame(false)
                                                    .desired_width(f32::INFINITY),
                                            )
                                            .changed()
                                        {
                                            dirty = true;
                                        }
                                    });
                                    ui.add_space(1.0);
                                    if ui
                                        .add(
                                            TextEdit::multiline(&mut beat.summary)
                                                .font(sans(9.3))
                                                .text_color(MUTED)
                                                .frame(false)
                                                .desired_rows(2)
                                                .desired_width(f32::INFINITY),
                                        )
                                        .changed()
                                    {
                                        dirty = true;
                                    }
                                });
                            ui.add_space(4.0);
                        }
                        ui.add_space(7.0);
                    }
                });
            if dirty {
                app.ws.dirty = true;
            }
            if let Some(act) = add_act {
                app.ws.project.beats.push(Beat {
                    id: new_id("beat"),
                    title: "New beat".into(),
                    summary: "What changes here?".into(),
                    act,
                    done: false,
                    template: "Three act".into(),
                });
                app.ws.dirty = true;
            }
        });
}
