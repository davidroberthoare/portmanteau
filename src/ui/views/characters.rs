//! Character sheets: want, need, flaw and arc, kept beside the script.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};

use msc_core::Character;

use crate::app::ScriptoriumApp;
use crate::theme::*;
use crate::ui::views::{content_frame, meta_line, page_head, primary_button};

const FACE_COLORS: [Color32; 6] = [
    Color32::from_rgb(204, 160, 92),
    Color32::from_rgb(132, 170, 145),
    Color32::from_rgb(166, 143, 185),
    Color32::from_rgb(124, 156, 196),
    Color32::from_rgb(201, 122, 108),
    Color32::from_rgb(176, 166, 132),
];

pub fn show(app: &mut ScriptoriumApp, ui: &mut egui::Ui) {
    let mut add = false;
    page_head(
        ui,
        "People",
        "Characters",
        "Who wants what, what they cannot admit, and how they change.",
        |ui| {
            if primary_button(ui, "+ New character").clicked() {
                add = true;
            }
            ui.add_space(6.0);
            meta_line(ui, &[&format!("{} people", app.ws.project.characters.len())]);
        },
    );

    if !app.ws.project.characters.is_empty()
        && !app
            .ws
            .project
            .characters
            .iter()
            .any(|c| Some(&c.id) == app.selected_character.as_ref())
    {
        app.selected_character = Some(app.ws.project.characters[0].id.clone());
    }

    content_frame().show(ui, |ui| {
        if app.ws.project.characters.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No one here yet").font(serif(20.0)).color(TEXT));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("A story needs someone who wants something.")
                        .font(sans(11.0))
                        .color(MUTED),
                );
                ui.add_space(12.0);
                if primary_button(ui, "Create a character").clicked() {
                    add = true;
                }
            });
            return;
        }

        let height = (ui.available_height() - 8.0).max(340.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(268.0, height),
                Layout::top_down(Align::Min),
                |ui| {
                    Frame::new()
                        .fill(PANEL)
                        .stroke(Stroke::new(1.0, BORDER_SOFT))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(9))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            section_label(ui, "Cast");
                            ui.add_space(6.0);
                            ScrollArea::vertical()
                                .id_salt("character_list")
                                .auto_shrink([false, false])
                                .max_height(height - 42.0)
                                .show(ui, |ui| {
                                    for ch in &app.ws.project.characters {
                                        let selected =
                                            app.selected_character.as_deref() == Some(ch.id.as_str());
                                        let (rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), 54.0),
                                            Sense::click(),
                                        );
                                        if selected {
                                            ui.painter().rect_filled(
                                                rect,
                                                CornerRadius::same(6),
                                                ACCENT_SOFT,
                                            );
                                        } else if resp.hovered() {
                                            ui.painter().rect_filled(
                                                rect,
                                                CornerRadius::same(6),
                                                PANEL_2,
                                            );
                                        }
                                        let color =
                                            FACE_COLORS[ch.color as usize % FACE_COLORS.len()];
                                        ui.painter().circle_filled(
                                            egui::pos2(rect.left() + 21.0, rect.center().y),
                                            15.0,
                                            color.gamma_multiply(0.22),
                                        );
                                        ui.painter().text(
                                            egui::pos2(rect.left() + 21.0, rect.center().y),
                                            egui::Align2::CENTER_CENTER,
                                            initials(&ch.name),
                                            sans(9.0),
                                            color,
                                        );
                                        ui.painter().text(
                                            egui::pos2(rect.left() + 44.0, rect.top() + 15.0),
                                            egui::Align2::LEFT_CENTER,
                                            &ch.name,
                                            sans(11.0),
                                            if selected { TEXT } else { MUTED },
                                        );
                                        let role: String = ch.role.chars().take(32).collect();
                                        ui.painter().text(
                                            egui::pos2(rect.left() + 44.0, rect.top() + 34.0),
                                            egui::Align2::LEFT_CENTER,
                                            role,
                                            sans(9.0),
                                            FAINT,
                                        );
                                        if resp.clicked() {
                                            app.selected_character = Some(ch.id.clone());
                                        }
                                    }
                                });
                        });
                },
            );
            ui.add_space(14.0);
            ui.vertical(|ui| {
                if let Some(id) = app.selected_character.clone() {
                    detail(app, ui, &id);
                }
            });
        });
    });

    if add {
        let mut ch = Character::new("New character", "Supporting");
        ch.color = (app.ws.project.characters.len() % FACE_COLORS.len()) as u8;
        let id = ch.id.clone();
        app.ws.project.characters.push(ch);
        app.selected_character = Some(id);
        app.ws.dirty = true;
    }
}

fn detail(app: &mut ScriptoriumApp, ui: &mut egui::Ui, id: &str) {
    let Some(pos) = app.ws.project.characters.iter().position(|c| c.id == id) else {
        return;
    };
    let mut dirty = false;
    let mut delete = false;
    let color = FACE_COLORS[app.ws.project.characters[pos].color as usize % FACE_COLORS.len()];
    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ScrollArea::vertical()
                .id_salt("character_detail")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let ch = &mut app.ws.project.characters[pos];
                    ui.horizontal(|ui| {
                        let (avatar, _) =
                            ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                        ui.painter().circle_filled(
                            avatar.center(),
                            24.0,
                            color.gamma_multiply(0.18),
                        );
                        ui.painter().text(
                            avatar.center(),
                            egui::Align2::CENTER_CENTER,
                            initials(&ch.name),
                            serif(16.0),
                            color,
                        );
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            if ui
                                .add(
                                    TextEdit::singleline(&mut ch.name)
                                        .font(serif(23.0))
                                        .frame(false)
                                        .desired_width(360.0),
                                )
                                .changed()
                            {
                                dirty = true;
                            }
                            if ui
                                .add(
                                    TextEdit::singleline(&mut ch.role)
                                        .font(sans(10.5))
                                        .text_color(MUTED)
                                        .hint_text("Role in the story")
                                        .frame(false)
                                        .desired_width(360.0),
                                )
                                .changed()
                            {
                                dirty = true;
                            }
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Delete").font(sans(9.5)).color(RED),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                delete = true;
                            }
                        });
                    });
                    ui.add_space(10.0);
                    hairline(ui);
                    ui.add_space(10.0);

                    ui.columns(2, |cols| {
                        field(&mut cols[0], "Wants", "The external goal, pursued on the page", &mut ch.want, &mut dirty);
                        field(&mut cols[1], "Needs", "What would actually make them whole", &mut ch.need, &mut dirty);
                    });
                    ui.add_space(8.0);
                    ui.columns(2, |cols| {
                        field(&mut cols[0], "Flaw", "The thing that gets in the way", &mut ch.flaw, &mut dirty);
                        field(&mut cols[1], "Arc", "Where they end up, and what it costs", &mut ch.arc, &mut dirty);
                    });
                    ui.add_space(10.0);
                    section_label(ui, "Description");
                    ui.add_space(3.0);
                    if ui
                        .add(
                            TextEdit::multiline(&mut ch.description)
                                .hint_text("Age, bearing, the detail a casting director would remember.")
                                .font(sans(11.0))
                                .desired_rows(6)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        dirty = true;
                    }

                    ui.add_space(14.0);
                    hairline(ui);
                    ui.add_space(10.0);
                    section_label(ui, "Appearances");
                    ui.add_space(4.0);
                    let name_upper = ch.name.to_uppercase();
                    let appearances: Vec<String> = app
                        .ws
                        .project
                        .documents
                        .iter()
                        .filter(|d| {
                            let body = d.body_text().to_uppercase();
                            body.contains(&name_upper)
                        })
                        .map(|d| d.title.clone())
                        .take(6)
                        .collect();
                    if appearances.is_empty() {
                        ui.label(
                            RichText::new("Not yet named in the manuscript.")
                                .font(sans(10.0))
                                .color(FAINT),
                        );
                    }
                    for title in appearances {
                        ui.horizontal(|ui| {
                            dot(ui, color);
                            ui.label(RichText::new(title).font(sans(10.0)).color(MUTED));
                        });
                    }
                });
        });

    if dirty {
        app.ws.dirty = true;
    }
    if delete {
        app.ws.project.characters.remove(pos);
        app.selected_character = app.ws.project.characters.first().map(|c| c.id.clone());
        app.ws.dirty = true;
    }
}

fn field(ui: &mut egui::Ui, title: &str, hint: &str, value: &mut String, dirty: &mut bool) {
    section_label(ui, title);
    ui.add_space(2.0);
    if ui
        .add(
            TextEdit::multiline(value)
                .hint_text(hint)
                .font(sans(10.5))
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        )
        .changed()
    {
        *dirty = true;
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}
