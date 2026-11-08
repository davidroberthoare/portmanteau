//! Research: clippings, interviews, data and field notes beside the manuscript.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};

use crate::app::ScriptoriumApp;
use crate::theme::*;
use crate::ui::views::{content_frame, meta_line, page_head, primary_button};

pub fn show(app: &mut ScriptoriumApp, ui: &mut egui::Ui) {
    let mut add = false;
    page_head(
        ui,
        "Sources",
        "Research",
        "Keep the world of the story close: interviews, archives, images, arguments with yourself.",
        |ui| {
            if primary_button(ui, "+ New note").clicked() {
                add = true;
            }
        },
    );

    let items = app.ws.project.research.clone();
    if !items.is_empty()
        && !items
            .iter()
            .any(|i| Some(&i.id) == app.selected_research.as_ref())
    {
        app.selected_research = Some(items[0].id.clone());
    }

    content_frame().show(ui, |ui| {
        if items.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No research yet").font(serif(20.0)).color(TEXT));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("A folder of small, useful things. Start with one note.")
                        .font(sans(11.0))
                        .color(MUTED),
                );
                ui.add_space(12.0);
                if primary_button(ui, "Add the first note").clicked() {
                    add = true;
                }
            });
            return;
        }

        let height = (ui.available_height() - 8.0).max(320.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(286.0, height),
                Layout::top_down(Align::Min),
                |ui| {
                    Frame::new()
                        .fill(PANEL)
                        .stroke(Stroke::new(1.0, BORDER_SOFT))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                section_label(ui, "Sourcebook");
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    meta_line(ui, &[&format!("{} notes", items.len())]);
                                });
                            });
                            ui.add_space(6.0);
                            ScrollArea::vertical()
                                .id_salt("research_list")
                                .auto_shrink([false, false])
                                .max_height(height - 58.0)
                                .show(ui, |ui| {
                                    for item in &items {
                                        let selected =
                                            app.selected_research.as_deref() == Some(item.id.as_str());
                                        let (rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), 46.0),
                                            Sense::click(),
                                        );
                                        if selected {
                                            ui.painter().rect_filled(
                                                rect,
                                                CornerRadius::same(5),
                                                ACCENT_SOFT,
                                            );
                                        } else if resp.hovered() {
                                            ui.painter().rect_filled(
                                                rect,
                                                CornerRadius::same(5),
                                                PANEL_2,
                                            );
                                        }
                                        let kind_color = match item.kind.as_str() {
                                            "Interview" => PURPLE,
                                            "Data" => BLUE,
                                            "Archive" => ACCENT,
                                            _ => SAGE,
                                        };
                                        ui.painter().circle_filled(
                                            egui::pos2(rect.left() + 9.0, rect.top() + 15.0),
                                            3.0,
                                            kind_color,
                                        );
                                        let title_rect = egui::Rect::from_min_max(
                                            egui::pos2(rect.left() + 18.0, rect.top()),
                                            egui::pos2(rect.right() - 4.0, rect.bottom()),
                                        );
                                        ui.painter().with_clip_rect(title_rect).text(
                                            title_rect.left_top() + Vec2::new(0.0, 7.0),
                                            egui::Align2::LEFT_TOP,
                                            &item.title,
                                            sans(10.5),
                                            if selected { TEXT } else { MUTED },
                                        );
                                        ui.painter().with_clip_rect(title_rect).text(
                                            title_rect.left_top() + Vec2::new(0.0, 24.0),
                                            egui::Align2::LEFT_TOP,
                                            format!("{} · {}", item.kind, item.source),
                                            sans(8.5),
                                            FAINT,
                                        );
                                        if resp.clicked() {
                                            app.selected_research = Some(item.id.clone());
                                        }
                                    }
                                });
                        });
                },
            );
            ui.add_space(12.0);
            ui.vertical(|ui| {
                if let Some(id) = app.selected_research.clone() {
                    editor(app, ui, &id);
                }
            });
        });
    });

    if add {
        let res = app.ws.execute(
            "research.add",
            serde_json::json!({
                "title": "Untitled note",
                "body": "",
                "kind": "Note",
                "source": ""
            }),
        );
        if res.ok {
            if let Some(id) = res.data.get("id").and_then(|v| v.as_str()) {
                app.selected_research = Some(id.to_string());
            }
            app.ws.dirty = true;
        }
    }
}

fn editor(app: &mut ScriptoriumApp, ui: &mut egui::Ui, id: &str) {
    let Some(pos) = app.ws.project.research.iter().position(|r| r.id == id) else {
        return;
    };
    let old_title = app.ws.project.research[pos].title.clone();
    let old_body = app.ws.project.research[pos].body.clone();
    let mut title = old_title.clone();
    let mut body = old_body.clone();
    let mut kind = app.ws.project.research[pos].kind.clone();
    let mut source = app.ws.project.research[pos].source.clone();
    let mut changed = false;
    let mut open_doc: Option<String> = None;

    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, "Research note");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    meta_line(ui, &[&format!("{} words", msc_core::count_words(&body))]);
                });
            });
            ui.add_space(5.0);
            if ui
                .add(
                    TextEdit::singleline(&mut title)
                        .font(serif(22.0))
                        .frame(false)
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                changed = true;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Kind").font(sans(9.5)).color(FAINT));
                egui::ComboBox::from_id_salt("research_kind")
                    .selected_text(RichText::new(&kind).font(sans(10.0)).color(MUTED))
                    .width(118.0)
                    .show_ui(ui, |ui| {
                        for option in ["Note", "Interview", "Data", "Archive", "Image", "Place"] {
                            if ui
                                .selectable_value(&mut kind, option.to_string(), option)
                                .clicked()
                            {
                                changed = true;
                            }
                        }
                    });
                ui.add_space(8.0);
                ui.label(RichText::new("Source").font(sans(9.5)).color(FAINT));
                if ui
                    .add(
                        TextEdit::singleline(&mut source)
                            .hint_text("Where it came from")
                            .font(sans(10.0))
                            .desired_width(220.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
            ui.add_space(8.0);
            hairline(ui);
            ui.add_space(10.0);

            let rows = ((body.chars().count() as f32) / 75.0).ceil().max(16.0) as usize;
            if ui
                .add(
                    TextEdit::multiline(&mut body)
                        .font(serif(13.5))
                        .desired_rows(rows.min(40))
                        .desired_width(f32::INFINITY)
                        .frame(false)
                        .hint_text("What did you find, and why does it matter to the story?"),
                )
                .changed()
            {
                changed = true;
            }

            ui.add_space(14.0);
            hairline(ui);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if quiet_link(ui, "Open as a document").clicked() {
                    if let Some(doc) = app
                        .ws
                        .project
                        .documents
                        .iter()
                        .find(|d| d.kind == msc_core::DocKind::Research && d.title == old_title)
                    {
                        open_doc = Some(doc.id.clone());
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(doc) = app
                        .ws
                        .project
                        .documents
                        .iter()
                        .find(|d| d.kind == msc_core::DocKind::Research && d.title == old_title)
                    {
                        meta_line(ui, &[&format!("Linked to {}", doc.title)]);
                    }
                });
            });
        });

    if changed {
        if let Some(item) = app.ws.project.research.get_mut(pos) {
            item.title = title.clone();
            item.body = body.clone();
            item.kind = kind;
            item.source = source;
        }
        if let Some(doc) = app
            .ws
            .project
            .documents
            .iter_mut()
            .find(|d| d.kind == msc_core::DocKind::Research && d.title == old_title)
        {
            doc.title = title;
            doc.text = body;
            doc.touch();
        }
        app.ws.dirty = true;
    }
    if let Some(doc) = open_doc {
        app.open_doc(doc);
    }
}

fn quiet_link(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).font(sans(10.0)).color(BLUE))
            .frame(false)
            .min_size(Vec2::new(0.0, 22.0)),
    )
}

pub fn _unused(ui: &mut egui::Ui) {
    let _ = Frame::new().inner_margin(Margin::same(0)).show(ui, |_| {});
    let _ = Color32::WHITE;
}
