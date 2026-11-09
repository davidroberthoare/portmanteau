//! The writing surface: a paper-like prose editor and a typed screenplay editor.

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, Frame, Id, Key, Layout, Margin, Modifiers,
    RichText, ScrollArea, Sense, Stroke, TextEdit, Vec2,
};
use portmanteau_core::{
    DocKind, Document, ElementKind, ElementStyle, ElementStyles, ExportSettings, Medium,
};

use crate::app::{PortmanteauApp, WriteMode};
use crate::theme::*;

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let doc_id = app
        .selected
        .clone()
        .filter(|id| app.ws.project.document(id).is_some());
    let miss = if app.settings.spellcheck {
        doc_id
            .as_ref()
            .and_then(|id| app.ws.project.document(id))
            .map(|doc| app.spell.misspellings(&doc.body_text()))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let page_plan = match (&doc_id, app.write_mode) {
        (Some(id), WriteMode::Paged) => build_page_plan(app, id),
        _ => None,
    };
    toolbar(
        app,
        ui,
        page_plan.as_ref().map(|plan| plan.pages.len()),
        miss.len(),
    );

    let Some(doc_id) = doc_id else {
        empty_state(app, ui);
        return;
    };

    if app.settings.spellcheck && app.show_spelling {
        spelling_panel(app, ui, &doc_id, &miss);
    }

    if app.write_mode == WriteMode::Paged {
        let plan = page_plan.unwrap_or_else(|| {
            build_page_plan(app, &doc_id).unwrap_or_else(empty_page_plan)
        });
        paged_view(app, ui, &plan);
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

fn toolbar(
    app: &mut PortmanteauApp,
    ui: &mut egui::Ui,
    page_total: Option<usize>,
    miss_count: usize,
) {
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

                let mut new_mode = app.write_mode;
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let snap = egui::Button::new(RichText::new("Snapshot").font(sans(10.5)).color(MUTED))
                        .fill(PANEL_2)
                        .corner_radius(CornerRadius::same(5));
                    if ui.add(snap).on_hover_text("Save a version of this document").clicked() {
                        app.take_snapshot_public();
                    }

                    if app.settings.spellcheck {
                        let label = if miss_count > 0 {
                            format!("Spelling · {miss_count}")
                        } else {
                            "Spelling".to_string()
                        };
                        let color = if app.show_spelling {
                            ACCENT
                        } else if miss_count > 0 {
                            MUTED
                        } else {
                            FAINT
                        };
                        let spell = egui::Button::new(
                            RichText::new(label).font(sans(10.5)).color(color),
                        )
                        .fill(if app.show_spelling { ACCENT_SOFT } else { PANEL_2 })
                        .corner_radius(CornerRadius::same(5));
                        if ui
                            .add(spell)
                            .on_hover_text("Review misspellings and apply suggestions")
                            .clicked()
                        {
                            app.show_spelling = !app.show_spelling;
                        }
                    }

                    // Continuous vs Paged. Paged is a read-only print preview.
                    if app.write_mode == WriteMode::Paged {
                        if let Some(total) = page_total {
                            let current = app.paged_page.clamp(1, total.max(1));
                            pill(ui, &format!("Page {current} of {total}"), FAINT, PANEL_2);
                        }
                    }
                    Frame::new()
                        .fill(PANEL_2)
                        .corner_radius(CornerRadius::same(5))
                        .inner_margin(Margin::symmetric(2, 1))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 1.0;
                                ui.spacing_mut().button_padding = Vec2::new(4.0, 2.0);
                                for mode in [WriteMode::Continuous, WriteMode::Paged] {
                                    let active = new_mode == mode;
                                    let btn = egui::Button::new(
                                        RichText::new(mode.label())
                                            .font(sans(9.5))
                                            .color(if active { ACCENT } else { MUTED }),
                                    )
                                    .fill(if active { ACCENT_SOFT } else { PANEL_2 })
                                    .corner_radius(CornerRadius::same(4))
                                    .min_size(Vec2::new(0.0, 19.0));
                                    if ui
                                        .add(btn)
                                        .on_hover_text(match mode {
                                            WriteMode::Continuous => "Edit the single paper page",
                                            WriteMode::Paged => "Read-only page preview",
                                        })
                                        .clicked()
                                    {
                                        new_mode = mode;
                                    }
                                }
                            });
                        });

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

                    // Paged mode is a read-only preview, so the medium
                    // switcher is hidden there and the space goes to the page
                    // readout instead.
                    if app.write_mode == WriteMode::Continuous {
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
                if new_mode != app.write_mode {
                    if new_mode == WriteMode::Paged {
                        app.paged_page = 1;
                    }
                    app.write_mode = new_mode;
                }
            });
        });
}

/// The spelling review strip under the toolbar. Suggestions are collected
/// during the draw and the replacement is applied after it, so the document is
/// never borrowed while the widgets run (AGENTS.md §8).
fn spelling_panel(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str, miss: &[String]) {
    let mut action: Option<(String, String)> = None;
    Frame::new()
        .fill(Color32::from_rgb(21, 24, 29))
        .inner_margin(Margin::symmetric(16, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Spelling · {}", miss.len()))
                        .font(sans(10.5))
                        .color(if miss.is_empty() { FAINT } else { MUTED }),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let close = egui::Button::new(
                        RichText::new("Close").font(sans(9.5)).color(FAINT),
                    )
                    .frame(false);
                    if ui.add(close).clicked() {
                        app.show_spelling = false;
                    }
                });
            });
            if miss.is_empty() {
                ui.label(
                    RichText::new("No misspellings found in this document.")
                        .font(sans(9.5))
                        .color(FAINT),
                );
            } else {
                ScrollArea::vertical()
                    .id_salt("spelling_list")
                    .max_height(150.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for word in miss {
                            let suggestions = app.spell.suggest(word, 3);
                            egui::CollapsingHeader::new(
                                RichText::new(word).font(mono(10.0)).color(MUTED),
                            )
                            .id_salt(("spelling_word", word.as_str()))
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    if suggestions.is_empty() {
                                        ui.label(
                                            RichText::new("(no suggestions)")
                                                .font(sans(9.5))
                                                .color(FAINT),
                                        );
                                    } else {
                                        for suggestion in &suggestions {
                                            let button = egui::Button::new(
                                                RichText::new(suggestion)
                                                    .font(sans(9.5))
                                                    .color(ACCENT),
                                            )
                                            .fill(PANEL_2)
                                            .corner_radius(CornerRadius::same(4))
                                            .min_size(Vec2::new(0.0, 18.0));
                                            if ui.add(button).clicked() {
                                                action = Some((
                                                    word.clone(),
                                                    suggestion.clone(),
                                                ));
                                            }
                                        }
                                    }
                                });
                            });
                        }
                    });
            }
        });

    if let Some((word, replacement)) = action {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            let mut changed = false;
            if doc.medium.is_script() {
                for block in &mut doc.blocks {
                    let next = replace_word_in_text(&block.text, &word, &replacement);
                    if next != block.text {
                        block.text = next;
                        changed = true;
                    }
                }
            } else {
                let next = replace_word_in_text(&doc.text, &word, &replacement);
                if next != doc.text {
                    doc.text = next;
                    changed = true;
                }
            }
            if changed {
                doc.touch();
                app.ws.dirty = true;
            }
        }
    }
}

/// Replace a whole token in `text`, matching case-insensitively but keeping a
/// leading capital if the original had one.
fn replace_word_in_text(text: &str, target: &str, replacement: &str) -> String {
    let target = target.to_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut token = String::new();
    for ch in text.chars() {
        if ch.is_alphabetic() || ((ch == '\'' || ch == '-') && !token.is_empty()) {
            token.push(ch);
        } else {
            flush_replacement(&mut token, &mut out, target.as_str(), replacement);
            out.push(ch);
        }
    }
    flush_replacement(&mut token, &mut out, target.as_str(), replacement);
    out
}

fn flush_replacement(token: &mut String, out: &mut String, target: &str, replacement: &str) {
    if token.is_empty() {
        return;
    }
    let lower = token.to_lowercase();
    let normalized = lower
        .trim_matches(|c: char| !c.is_alphabetic() && c != '\'' && c != '-');
    let normalized = normalized.trim_matches(|c: char| c == '\'' || c == '-');
    if normalized == target {
        out.push_str(&match_initial_case(token, replacement));
    } else {
        out.push_str(token);
    }
    token.clear();
}

fn match_initial_case(original: &str, replacement: &str) -> String {
    let uppercase = original
        .chars()
        .find(|c| c.is_alphabetic())
        .map(|c| c.is_uppercase())
        .unwrap_or(false);
    if !uppercase {
        return replacement.to_string();
    }
    let mut chars = replacement.chars();
    match chars.next() {
        Some(first) => {
            let mut out = String::new();
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

fn prose_page(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str, take_snapshot: &mut bool) {
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
        let pages = doc.pages();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} words  ·  {} min read", words, portmanteau_core::reading_minutes(words)))
                    .font(sans(9.0))
                    .color(PAPER_MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{pages}")).font(serif(9.0)).color(PAPER_MUTED));
            });
        });
        let _ = take_snapshot;
    }
    if dirty {
        app.ws.dirty = true;
    }
}

fn script_page(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
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
                doc.blocks.push(portmanteau_core::Block::new(
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
            doc.blocks.push(portmanteau_core::Block::new(
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
            ui.horizontal(|ui| {
                ui.set_width(content_width);
                // Element gutter: scene numbers only.
                let (grect, gresp) = ui.allocate_exact_size(Vec2::new(40.0, 18.0), Sense::hover());
                if gresp.hovered() {
                    ui.painter().rect_filled(grect, CornerRadius::same(3), Color32::from_rgb(236, 230, 217));
                }
                if !scene_label.is_empty() {
                    ui.painter().text(
                        grect.right_center() - Vec2::new(1.0, 8.0),
                        egui::Align2::RIGHT_BOTTOM,
                        scene_label,
                        mono(7.5),
                        PAPER_MUTED,
                    );
                }

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
                doc.blocks.push(portmanteau_core::Block::new(next_kind(last_kind, doc.medium), ""));
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
                doc.blocks.insert(i + 1, portmanteau_core::Block::new(next, ""));
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

// ---------------------------------------------------------------------------
// Paged (print preview) view
// ---------------------------------------------------------------------------

/// One rendered line inside a page. The paging estimator fills pages with these
/// so a page break can fall between any two lines.
struct PageLine {
    text: String,
    size: f32,
    lead: f32,
    gap_before: f32,
    mono: bool,
    indent: f32,
    right: bool,
}

/// An estimated pagination of one document. Geometry and the pages themselves
/// are owned so the view can draw without borrowing the project.
struct PagePlan {
    page_w: f32,
    content_w: f32,
    content_h: f32,
    pages: Vec<Vec<PageLine>>,
}

fn empty_page_plan() -> PagePlan {
    PagePlan {
        page_w: 444.0,
        content_w: 336.0,
        content_h: 552.0,
        pages: vec![Vec::new()],
    }
}

/// Page size from the active export target's paper, mapped into egui points at
/// a readable on-screen scale (1 mm ~ 3 points). Margins match the continuous
/// paper: 54 pt left/right, 44 top, 34 bottom.
fn page_geometry(settings: &ExportSettings) -> (f32, f32) {
    let (mm_w, mm_h) = settings.paper.mm();
    ((mm_w * 3.0).clamp(420.0, 760.0), (mm_h * 3.0).max(520.0))
}

fn build_page_plan(app: &PortmanteauApp, doc_id: &str) -> Option<PagePlan> {
    let doc = app.ws.project.document(doc_id)?;
    let settings = app.ws.project.settings_for(app.ws.project.export.kind);
    Some(if doc.medium.is_script() {
        script_plan(doc, &settings)
    } else {
        prose_plan(doc, &settings)
    })
}

/// Best-effort prose paging. Paragraphs are wrapped to the text width with a
/// character-width estimate (`size * 0.55`), then lines are packed until the
/// page's text height is used up.
fn prose_plan(doc: &Document, settings: &ExportSettings) -> PagePlan {
    let (page_w, page_h) = page_geometry(settings);
    let content_w = page_w - 108.0;
    let content_h = page_h - 78.0;
    let body = &settings.styles.body;
    let heading = &settings.styles.chapter_heading;
    let body_size = body.size.max(6.0);
    let line_h = (body_size * body.leading.max(0.8)).max(8.0);
    let chars_per_line = (content_w / (body_size * 0.5)).floor().max(10.0) as usize;

    let mut pages: Vec<Vec<PageLine>> = vec![Vec::new()];
    let mut used = 0.0f32;

    // The chapter title sits at the top of the first page, set with the
    // chapter-heading style.
    let title = doc.title.trim();
    if !title.is_empty() {
        let title_size = heading.size.max(body_size);
        let title_lead = (title_size * heading.leading.max(0.9)).max(10.0);
        pages[0].push(PageLine {
            text: title.to_string(),
            size: title_size,
            lead: title_lead,
            gap_before: 0.0,
            mono: false,
            indent: 0.0,
            right: false,
        });
        used += title_lead;
        let gap = line_h * 0.9;
        pages[0].push(PageLine {
            text: " ".into(),
            size: body_size,
            lead: line_h,
            gap_before: gap,
            mono: false,
            indent: 0.0,
            right: false,
        });
        used += gap + line_h;
    }

    for paragraph in doc.body_text().split("\n\n") {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        for (i, text) in wrap_words(paragraph, chars_per_line).into_iter().enumerate() {
            let mut gap = if i == 0 && !pages.last().unwrap().is_empty() {
                line_h * 0.45
            } else {
                0.0
            };
            if used + gap + line_h > content_h && !pages.last().unwrap().is_empty() {
                pages.push(Vec::new());
                used = 0.0;
                gap = 0.0;
            }
            if pages.last().unwrap().is_empty() {
                gap = 0.0;
            }
            pages.last_mut().unwrap().push(PageLine {
                text,
                size: body_size,
                lead: line_h,
                gap_before: gap,
                mono: false,
                indent: 0.0,
                right: false,
            });
            used += gap + line_h;
        }
    }

    PagePlan {
        page_w,
        content_w,
        content_h,
        pages,
    }
}

/// Script paging: roughly 55 typed lines per page, with each block's wrapped
/// height counted in those lines. The page keeps the paper width and frame but
/// grows tall enough for all 55 lines.
fn script_plan(doc: &Document, settings: &ExportSettings) -> PagePlan {
    let (page_w, _) = page_geometry(settings);
    let content_w = page_w - 108.0;
    let base = &settings.styles.action;
    let base_size = base.size.max(7.0);
    let base_line = (base_size * base.leading.max(0.85)).max(9.0);
    let lines_per_page = 55.0f32;
    let content_h = base_line * lines_per_page;

    let mut pages: Vec<Vec<PageLine>> = vec![Vec::new()];
    let mut used = 0.0f32;

    for (index, block) in doc.blocks.iter().enumerate() {
        let style = script_element_style(&settings.styles, block.kind);
        let size = style.size.max(6.0);
        let lead = (size * style.leading.max(0.85)).max(8.0);
        let indent = style.indent.max(0.0).min(content_w * 0.42);
        let chars_per_line = ((content_w - indent) / (size * 0.6))
            .floor()
            .max(8.0) as usize;
        let wrapped = wrap_words(&block.text, chars_per_line);
        let block_lines = (wrapped.len() as f32 * lead / base_line).max(1.0);
        let mut gap_lines = if index == 0 || block.kind.is_dialogue() {
            0.0
        } else {
            0.4
        };
        if used + gap_lines + block_lines > lines_per_page
            && !pages.last().unwrap().is_empty()
        {
            pages.push(Vec::new());
            used = 0.0;
            gap_lines = 0.0;
        }
        if pages.last().unwrap().is_empty() {
            gap_lines = 0.0;
        }
        for (i, text) in wrapped.into_iter().enumerate() {
            pages.last_mut().unwrap().push(PageLine {
                text,
                size,
                lead,
                gap_before: if i == 0 { gap_lines * base_line } else { 0.0 },
                mono: true,
                indent,
                right: block.kind == ElementKind::Transition,
            });
        }
        used += gap_lines + block_lines;
    }

    PagePlan {
        page_w,
        content_w,
        content_h,
        pages,
    }
}

fn script_element_style<'a>(styles: &'a ElementStyles, kind: ElementKind) -> &'a ElementStyle {
    match kind {
        ElementKind::SceneHeading => &styles.scene_heading,
        ElementKind::Action | ElementKind::Shot => &styles.action,
        ElementKind::Character => &styles.character,
        ElementKind::Parenthetical => &styles.parenthetical,
        ElementKind::Dialogue | ElementKind::Lyric => &styles.dialogue,
        ElementKind::Transition => &styles.transition,
        ElementKind::Act => &styles.act,
        ElementKind::Scene => &styles.scene,
        ElementKind::StageDirection => &styles.stage_direction,
        ElementKind::General => &styles.general,
    }
}

/// Break a paragraph into display lines using an estimated character width.
/// Wrapping happens on spaces; a single word longer than the line stays whole.
fn wrap_words(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(4);
    let mut out = Vec::new();
    for raw in text.split('\n') {
        let mut line = String::new();
        for word in raw.split_whitespace() {
            let word_len = word.chars().count();
            if line.is_empty() {
                line = word.to_string();
            } else if line.chars().count() + 1 + word_len <= max_chars {
                line.push(' ');
                line.push_str(word);
            } else {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            }
        }
        if !line.is_empty() {
            out.push(line);
        } else if raw.trim().is_empty() && !out.is_empty() {
            out.push(" ".into());
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn paged_view(app: &mut PortmanteauApp, ui: &mut egui::Ui, plan: &PagePlan) {
    let mut page_rects = Vec::with_capacity(plan.pages.len());
    let mut viewport_top = f32::NEG_INFINITY;
    ScrollArea::vertical()
        .id_salt("editor_paged_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            viewport_top = ui.clip_rect().top();
            ui.add_space(22.0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(plan.page_w);
                ui.label(
                    RichText::new("Read-only page preview — switch to Continuous to edit")
                        .font(sans(9.5))
                        .color(FAINT),
                );
                ui.add_space(10.0);
                for (index, page) in plan.pages.iter().enumerate() {
                    page_rects.push(page_frame(ui, plan, page, index + 1));
                    if index + 1 < plan.pages.len() {
                        ui.add_space(26.0);
                    }
                }
            });
            ui.add_space(36.0);
        });

    // Which page has the top of the viewport scrolled into it? The toolbar
    // reads this on the next frame.
    app.paged_page = page_rects
        .iter()
        .enumerate()
        .rev()
        .find(|(_, rect)| rect.top() <= viewport_top + 1.0)
        .map(|(index, _)| index + 1)
        .unwrap_or(1)
        .min(plan.pages.len().max(1));
}

fn page_frame(ui: &mut egui::Ui, plan: &PagePlan, lines: &[PageLine], number: usize) -> egui::Rect {
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
            left: 54,
            right: 54,
            top: 44,
            bottom: 34,
        })
        .show(ui, |ui| {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.set_width(plan.content_w);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let mut used = 0.0f32;
                for line in lines {
                    if line.gap_before > 0.0 {
                        ui.add_space(line.gap_before);
                        used += line.gap_before;
                    }
                    let rich = RichText::new(if line.text.is_empty() {
                        " "
                    } else {
                        line.text.as_str()
                    })
                    .font(if line.mono {
                        mono(line.size)
                    } else {
                        serif(line.size)
                    })
                    .color(PAPER_TEXT);
                    if line.right {
                        // Constrain the height: an unconstrained right-to-left
                        // layout would swallow the rest of the page.
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), line.lead.max(8.0)),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                ui.add(egui::Label::new(rich).wrap());
                            },
                        );
                    } else if line.indent > 0.0 {
                        ui.horizontal(|ui| {
                            ui.add_space(line.indent);
                            ui.add(egui::Label::new(rich).wrap());
                        });
                    } else {
                        ui.add(egui::Label::new(rich).wrap());
                    }
                    used += line.lead;
                }
                let remaining = (plan.content_h - used).max(6.0);
                ui.add_space(remaining);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("{number}"))
                            .font(serif(9.0))
                            .color(PAPER_MUTED),
                    );
                });
            });
        })
        .response
        .rect
}

fn empty_state(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
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
    });
}
