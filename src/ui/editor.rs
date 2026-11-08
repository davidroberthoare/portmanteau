//! The writing surface: a paper-like prose editor and a typed screenplay editor.

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, Frame, Id, Key, Layout, Margin, Modifiers,
    RichText, ScrollArea, Sense, Stroke, TextEdit, Vec2,
};
use msc_core::{DocKind, ElementKind, Medium};

use crate::app::{ScriptoriumApp, View};
use crate::theme::*;

pub fn show(app: &mut ScriptoriumApp, ui: &mut egui::Ui) {
    toolbar(app, ui);
    let Some(doc_id) = app.selected.clone() else {
        empty_state(app, ui);
        return;
    };
    if app.ws.project.document(&doc_id).is_none() {
        empty_state(app, ui);
        return;
    }

    let wide = if app.focus_mode { 860.0 } else { 780.0 };
    let page_w = (ui.available_width() - 72.0).clamp(420.0, wide);
    let mut take_snapshot = false;
    ScrollArea::vertical()
        .id_salt("editor_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(22.0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(page_w);
                Frame::new()
                    .fill(PAPER)
                    .stroke(Stroke::new(1.0, PAPER_EDGE))
                    .corner_radius(CornerRadius::same(3))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 10],
                        blur: 34,
                        spread: 0,
                        color: Color32::from_black_alpha(90),
                    })
                    .inner_margin(Margin {
                        left: if app.focus_mode { 64 } else { 54 },
                        right: if app.focus_mode { 64 } else { 54 },
                        top: 44,
                        bottom: 34,
                    })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let is_script = app
                            .ws
                            .project
                            .document(&doc_id)
                            .map(|d| d.medium.is_script())
                            .unwrap_or(false);
                        if is_script {
                            script_page(app, ui, &doc_id);
                        } else {
                            prose_page(app, ui, &doc_id, &mut take_snapshot);
                        }
                    });
            });
            ui.add_space(36.0);
        });

    if take_snapshot {
        app.take_snapshot_public();
    }
}

fn toolbar(app: &mut ScriptoriumApp, ui: &mut egui::Ui) {
    Frame::new()
        .fill(Color32::from_rgb(21, 24, 29))
        .inner_margin(Margin::symmetric(16, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (id, medium) = match &app.selected {
                    Some(id) => {
                        let medium = app
                            .ws
                            .project
                            .document(id)
                            .map(|d| d.medium)
                            .unwrap_or(Medium::Prose);
                        (id.clone(), medium)
                    }
                    None => (String::new(), Medium::Prose),
                };
                // Breadcrumb.
                let mut chain: Vec<String> = Vec::new();
                if let Some(doc) = app.ws.project.document(&id) {
                    chain.push(doc.title.clone());
                    let mut parent = doc.parent.clone();
                    while let Some(pid) = parent {
                        if let Some(p) = app.ws.project.document(&pid) {
                            chain.push(p.title.clone());
                            parent = p.parent.clone();
                        } else {
                            break;
                        }
                    }
                }
                chain.reverse();
                for (i, part) in chain.iter().enumerate() {
                    if i > 0 {
                        ui.label(RichText::new("/").font(sans(10.0)).color(FAINT));
                    }
                    ui.label(
                        RichText::new(part)
                            .font(sans(10.5))
                            .color(if i + 1 == chain.len() { MUTED } else { FAINT }),
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let snap = egui::Button::new(RichText::new("Snapshot").font(sans(10.5)).color(MUTED))
                        .fill(PANEL_2)
                        .corner_radius(CornerRadius::same(5));
                    if ui.add(snap).on_hover_text("Save a version of this document").clicked() {
                        app.take_snapshot_public();
                    }
                    let focus = egui::Button::new(
                        RichText::new(if app.focus_mode { "Exit focus" } else { "Focus" })
                            .font(sans(10.5))
                            .color(if app.focus_mode { ACCENT } else { MUTED }),
                    )
                    .fill(PANEL_2)
                    .corner_radius(CornerRadius::same(5));
                    if ui.add(focus).clicked() {
                        app.focus_mode = !app.focus_mode;
                    }

                    let mut new_medium = medium;
                    ComboBox::from_id_salt("editor_medium")
                        .selected_text(RichText::new(medium.label()).font(sans(10.5)).color(MUTED))
                        .width(98.0)
                        .show_ui(ui, |ui| {
                            for m in [
                                Medium::Prose,
                                Medium::Screenplay,
                                Medium::StagePlay,
                                Medium::TvScript,
                            ] {
                                ui.selectable_value(&mut new_medium, m, m.label());
                            }
                        });
                    if new_medium != medium {
                        if let Some(doc) = app.ws.project.document_mut(&id) {
                            doc.convert_medium(new_medium);
                            app.ws.dirty = true;
                            app.block_focus = Some(0);
                            app.selected_block = 0;
                        }
                    }
                    let stats = app.ws.project.document(&id).map(|d| {
                        (d.words(), d.pages(), d.status.clone(), d.kind)
                    });
                    if let Some((words, pages, status, kind)) = stats {
                        pill(
                            ui,
                            &format!("{words} words · {pages} pp"),
                            FAINT,
                            PANEL_2,
                        );
                        ui.label(
                            RichText::new(&status)
                                .font(sans(10.0))
                                .color(status_color(&status)),
                        );
                        if kind == DocKind::Scene {
                            ui.label(RichText::new("scene").font(sans(9.5)).color(PURPLE));
                        }
                    }
                });
            });
        });
}

fn prose_page(app: &mut ScriptoriumApp, ui: &mut egui::Ui, doc_id: &str, take_snapshot: &mut bool) {
    let mut dirty = false;
    let project_title = app.ws.project.title.clone();
    {
        let Some(doc) = app.ws.project.document_mut(doc_id) else { return };
        paper_header(ui, &project_title, doc.medium, &doc.status, doc.words(), doc.pages());

        ui.add_space(16.0);
        if ui
            .add(
                TextEdit::singleline(&mut doc.title)
                    .font(serif(29.0))
                    .text_color(PAPER_TEXT)
                    .frame(false)
                    .desired_width(f32::INFINITY),
            )
            .changed()
        {
            doc.touch();
            dirty = true;
        }
        ui.add_space(6.0);
        paper_rule(ui);
        ui.add_space(14.0);

        let rows = ((doc.text.chars().count() as f32) / 72.0).ceil().max(18.0) as usize;
        let edit = TextEdit::multiline(&mut doc.text)
            .font(serif(app.editor_font_size))
            .text_color(PAPER_TEXT)
            .frame(false)
            .desired_width(f32::INFINITY)
            .desired_rows(rows.min(120))
            .margin(Margin::same(0))
            .hint_text(
                RichText::new("Begin at the beginning…")
                    .font(serif(17.0))
                    .color(PAPER_MUTED),
            );
        let response = ui.add(edit);
        if response.changed() {
            doc.touch();
            dirty = true;
        }
        ui.add_space(26.0);
        let words = doc.words();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} words  ·  {} min read", words, msc_core::reading_minutes(words)))
                    .font(sans(9.0))
                    .color(PAPER_MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("1").font(serif(9.0)).color(PAPER_MUTED));
            });
        });
        let _ = take_snapshot;
    }
    if dirty {
        app.ws.dirty = true;
    }
}

fn script_page(app: &mut ScriptoriumApp, ui: &mut egui::Ui, doc_id: &str) {
    let mut dirty = false;
    let mut add_at: Option<usize> = None;
    let mut remove_at: Option<usize> = None;
    let mut cycle_at: Option<usize> = None;
    let project_title = app.ws.project.title.clone();
    {
        let Some(doc) = app.ws.project.document_mut(doc_id) else { return };
        paper_header(ui, &project_title, doc.medium, &doc.status, doc.words(), doc.pages());
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(doc.title.to_uppercase())
                    .font(mono(10.0))
                    .color(PAPER_MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let snippet: String = doc.synopsis.chars().take(58).collect();
                let meta = if doc.synopsis.chars().count() > 58 {
                    format!("{snippet}…")
                } else {
                    snippet
                };
                ui.label(
                    RichText::new(meta)
                        .font(mono(8.5))
                        .color(PAPER_MUTED),
                );
            });
        });
        ui.add_space(4.0);
        paper_rule(ui);
        ui.add_space(10.0);

        // Element palette for the selected block.
        let selected_kind = doc
            .blocks
            .get(app.selected_block)
            .map(|b| b.kind)
            .unwrap_or(ElementKind::Action);
        ui.horizontal_wrapped(|ui| {
            for kind in ElementKind::all(doc.medium) {
                let active = kind == selected_kind;
                let btn = egui::Button::new(
                    RichText::new(kind.short())
                        .font(sans(8.5))
                        .color(if active { Color32::from_rgb(30, 26, 19) } else { PAPER_MUTED }),
                )
                .fill(if active { ACCENT } else { Color32::from_rgb(235, 229, 216) })
                .corner_radius(CornerRadius::same(10))
                .min_size(Vec2::new(0.0, 19.0));
                if ui.add(btn).clicked() {
                    if let Some(block) = doc.blocks.get_mut(app.selected_block) {
                        block.kind = kind;
                        dirty = true;
                    }
                }
            }
            ui.add_space(6.0);
            let add = egui::Button::new(
                RichText::new("+ block").font(sans(8.5)).color(PAPER_MUTED),
            )
            .frame(false);
            if ui.add(add).clicked() {
                doc.blocks.push(msc_core::Block::new(
                    doc.medium.default_element(),
                    "",
                ));
                app.selected_block = doc.blocks.len().saturating_sub(1);
                app.block_focus = Some(app.selected_block);
                dirty = true;
            }
        });
        ui.add_space(12.0);

        if doc.blocks.is_empty() {
            doc.blocks.push(msc_core::Block::new(
                doc.medium.default_element(),
                "",
            ));
            app.block_focus = Some(0);
        }

        let first_scene_number = 1usize;
        let mut scene_no = first_scene_number;
        let content_width = ui.available_width();
        for i in 0..doc.blocks.len() {
            let block_id = Id::new(("script_block", doc_id, i));
            if app.block_focus == Some(i) {
                ui.memory_mut(|m| m.request_focus(block_id));
            }
            // Keyboard: Enter starts a new element, Tab cycles its type.
            let focused = ui.memory(|m| m.has_focus(block_id));
            if focused {
                let enter = ui.input_mut(|inp| {
                    inp.consume_key(Modifiers::NONE, Key::Enter)
                });
                if enter {
                    add_at = Some(i);
                }
                let tab = ui.input_mut(|inp| inp.consume_key(Modifiers::NONE, Key::Tab));
                let shift_tab = ui.input_mut(|inp| {
                    inp.consume_key(Modifiers::SHIFT, Key::Tab)
                });
                if tab || shift_tab {
                    cycle_at = Some(i);
                }
                if ui.input(|inp| inp.key_pressed(Key::Backspace))
                    && doc.blocks[i].text.is_empty()
                    && doc.blocks.len() > 1
                {
                    remove_at = Some(i);
                }
            }

            let kind = doc.blocks[i].kind;
            let is_scene = kind == ElementKind::SceneHeading;
            let scene_label = if is_scene {
                let label = format!("{scene_no}");
                scene_no += 1;
                label
            } else {
                String::new()
            };
            let color = kind_color(kind);
            let gutter_id = Id::new(("gutter", doc_id, i));

            ui.horizontal(|ui| {
                ui.set_width(content_width);
                // Element gutter.
                let (grect, gresp) = ui.allocate_exact_size(Vec2::new(40.0, 18.0), Sense::click());
                if gresp.hovered() {
                    ui.painter().rect_filled(grect, CornerRadius::same(3), Color32::from_rgb(236, 230, 217));
                }
                ui.painter().text(
                    grect.left_center() + Vec2::new(1.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    kind.short(),
                    sans(7.8),
                    color,
                );
                if !scene_label.is_empty() {
                    ui.painter().text(
                        grect.right_center() - Vec2::new(1.0, 8.0),
                        egui::Align2::RIGHT_BOTTOM,
                        scene_label,
                        mono(7.5),
                        PAPER_MUTED,
                    );
                }
                if gresp.on_hover_text("Change element type").clicked() {
                    cycle_at = Some(i);
                }
                let _ = gutter_id;

                let indent = match kind {
                    ElementKind::Character => content_width * 0.30,
                    ElementKind::Parenthetical => content_width * 0.20,
                    ElementKind::Dialogue => content_width * 0.12,
                    ElementKind::Transition => 0.0,
                    _ => 0.0,
                };
                if indent > 0.0 {
                    ui.add_space(indent);
                }
                let width = match kind {
                    ElementKind::Character => content_width * 0.50,
                    ElementKind::Parenthetical => content_width * 0.52,
                    ElementKind::Dialogue => content_width * 0.66,
                    ElementKind::Transition => content_width * 0.55,
                    _ => content_width - 46.0,
                };
                let font_size = match kind {
                    ElementKind::SceneHeading => 11.0,
                    ElementKind::Character => 10.5,
                    ElementKind::Parenthetical => 10.0,
                    ElementKind::Dialogue => 10.5,
                    ElementKind::Transition => 10.0,
                    _ => 10.5,
                };
                let rows = ((doc.blocks[i].text.chars().count() as f32 * font_size * 0.62)
                    / width.max(80.0))
                .ceil()
                .clamp(1.0, 8.0) as usize;
                let text_color = if kind == ElementKind::SceneHeading {
                    Color32::from_rgb(31, 29, 24)
                } else {
                    PAPER_TEXT
                };
                let mut edit = TextEdit::multiline(&mut doc.blocks[i].text)
                    .id(block_id)
                    .font(mono(font_size))
                    .text_color(text_color)
                    .frame(false)
                    .desired_width(width)
                    .desired_rows(rows)
                    .margin(Margin::symmetric(2, 1))
                    .hint_text(
                        RichText::new(kind.label())
                            .font(mono(font_size))
                            .color(PAPER_MUTED),
                    );
                if kind == ElementKind::Transition {
                    edit = edit.horizontal_align(Align::RIGHT);
                }
                if kind.is_uppercase() && kind != ElementKind::Character {
                    edit = edit.text_color(Color32::from_rgb(44, 40, 33));
                }
                let resp = ui.add(edit);
                if resp.clicked() {
                    app.selected_block = i;
                }
                if resp.changed() {
                    doc.touch();
                    dirty = true;
                }
                if kind == ElementKind::Transition {
                    ui.add_space(4.0);
                }
            });
            ui.add_space(if matches!(
                kind,
                ElementKind::Character | ElementKind::Parenthetical | ElementKind::Dialogue
            ) {
                1.0
            } else {
                8.0
            });
        }
        app.block_focus = None;

        ui.add_space(18.0);
        ui.horizontal(|ui| {
            let add = egui::Button::new(
                RichText::new("+ Add element").font(sans(9.0)).color(PAPER_MUTED),
            )
            .frame(false);
            if ui.add(add).clicked() {
                let last_kind = doc.blocks.last().map(|b| b.kind).unwrap_or(ElementKind::Action);
                doc.blocks.push(msc_core::Block::new(next_kind(last_kind, doc.medium), ""));
                app.selected_block = doc.blocks.len() - 1;
                app.block_focus = Some(app.selected_block);
                dirty = true;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} pages", doc.pages()))
                        .font(sans(8.5))
                        .color(PAPER_MUTED),
                );
            });
        });
    }

    if let Some(i) = cycle_at {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            if let Some(block) = doc.blocks.get_mut(i) {
                block.kind = block.kind.next(doc.medium);
                app.selected_block = i;
                app.ws.dirty = true;
            }
        }
    }
    if let Some(i) = add_at {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            if let Some(block) = doc.blocks.get_mut(i) {
                block.text = block.text.trim_end_matches('\n').to_string();
                let next = next_kind(block.kind, doc.medium);
                doc.blocks.insert(i + 1, msc_core::Block::new(next, ""));
            }
            app.selected_block = i + 1;
            app.block_focus = Some(i + 1);
            app.ws.dirty = true;
        }
    }
    if let Some(i) = remove_at {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            if doc.blocks.len() > 1 {
                doc.blocks.remove(i);
                let next = i.saturating_sub(1);
                app.selected_block = next;
                app.block_focus = Some(next);
                app.ws.dirty = true;
            }
        }
    }
    if dirty {
        app.ws.dirty = true;
    }
}

fn next_kind(after: ElementKind, medium: Medium) -> ElementKind {
    match after {
        ElementKind::Character => ElementKind::Dialogue,
        ElementKind::Parenthetical => ElementKind::Dialogue,
        ElementKind::Dialogue => ElementKind::Action,
        ElementKind::SceneHeading => ElementKind::Action,
        ElementKind::Transition => ElementKind::SceneHeading,
        ElementKind::Act => ElementKind::Scene,
        ElementKind::Scene => ElementKind::StageDirection,
        _ => {
            if medium == Medium::StagePlay {
                ElementKind::StageDirection
            } else {
                ElementKind::Action
            }
        }
    }
}

fn kind_color(kind: ElementKind) -> Color32 {
    match kind {
        ElementKind::SceneHeading => Color32::from_rgb(160, 112, 44),
        ElementKind::Character => Color32::from_rgb(118, 92, 140),
        ElementKind::Parenthetical => Color32::from_rgb(132, 117, 94),
        ElementKind::Dialogue => Color32::from_rgb(78, 104, 128),
        ElementKind::Transition => Color32::from_rgb(146, 96, 78),
        ElementKind::Act | ElementKind::Scene => Color32::from_rgb(112, 134, 110),
        _ => Color32::from_rgb(120, 112, 98),
    }
}

fn paper_header(
    ui: &mut egui::Ui,
    project_title: &str,
    medium: Medium,
    status: &str,
    words: u32,
    pages: u32,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(project_title.to_uppercase())
                .font(sans(8.0))
                .color(PAPER_MUTED)
                .extra_letter_spacing(1.4),
        );
        ui.label(RichText::new("·").font(sans(8.0)).color(PAPER_MUTED));
        ui.label(
            RichText::new(medium.label().to_uppercase())
                .font(sans(8.0))
                .color(PAPER_MUTED)
                .extra_letter_spacing(1.0),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{status} · {words} words · {pages} pp"))
                    .font(sans(8.0))
                    .color(PAPER_MUTED),
            );
        });
    });
}

fn paper_rule(ui: &mut egui::Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, PAPER_EDGE),
    );
}

fn empty_state(app: &mut ScriptoriumApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(120.0);
        ui.label(RichText::new("Nothing open").font(serif(30.0)).color(MUTED));
        ui.add_space(8.0);
        ui.label(
            RichText::new("Choose a document in the binder, or start a new chapter.")
                .font(sans(12.0))
                .color(FAINT),
        );
        ui.add_space(16.0);
        let btn = egui::Button::new(
            RichText::new("New chapter").font(sans(12.0)).color(Color32::from_rgb(28, 24, 17)),
        )
        .fill(ACCENT)
        .corner_radius(CornerRadius::same(6))
        .min_size(Vec2::new(120.0, 32.0));
        if ui.add(btn).clicked() {
            app.new_chapter_public();
        }
        ui.add_space(6.0);
        if ui
            .add(egui::Button::new(RichText::new("Open the corkboard").font(sans(11.0)).color(MUTED)).frame(false))
            .clicked()
        {
            app.view = View::Corkboard;
        }
    });
}
