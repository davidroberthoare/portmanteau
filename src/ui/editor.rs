//! The writing surface: a paper-like prose editor and a typed screenplay editor.

use std::collections::HashMap;
use std::ops::Range;

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, FontId, Frame, Id, Key, Layout, Margin,
    Modifiers, RichText, ScrollArea, Sense, Stroke, TextEdit, Vec2,
};
use portmanteau_core::spell::{byte_offset, word_at, SpellChecker};
use portmanteau_core::{DocKind, ElementKind, ItemKind, Medium};

use crate::app::{
    ImageInsertState, NewCharacterState, PortmanteauApp, RichTextTarget, WriteMode,
};
use crate::theme::*;
use crate::ui::page::{
    empty_page_plan, paged_view, prose_plan, script_element_style, script_plan, PagePlan,
};
use crate::ui::views::{content_frame, page_head};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let doc_id = app
        .selected
        .clone()
        .filter(|id| app.ws.project.document(id).is_some());

    let Some(doc_id) = doc_id else {
        if !app.focus_mode {
            toolbar(app, ui, None, 0);
        }
        empty_state(app, ui);
        return;
    };

    // Folders are containers, not text documents. They get their own panel
    // instead of the paper/script surface and the writing toolbar.
    let is_folder = app
        .ws
        .project
        .document(&doc_id)
        .map(|doc| doc.kind == DocKind::Folder)
        .unwrap_or(false);
    if is_folder {
        folder_panel(app, ui, &doc_id);
        return;
    }

    let miss = if app.settings.spellcheck {
        app.ws
            .project
            .document(&doc_id)
            .map(|doc| app.spell.misspellings(&doc.body_text()))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let page_plan = match app.write_mode {
        WriteMode::Paged => build_page_plan(app, &doc_id),
        WriteMode::Continuous => None,
    };
    if !app.focus_mode {
        toolbar(
            app,
            ui,
            page_plan.as_ref().map(|plan| plan.pages.len()),
            miss.len(),
        );

        if app.settings.spellcheck && app.show_spelling {
            spelling_panel(app, ui, &doc_id, &miss);
        }
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
    let is_script = app
        .ws
        .project
        .document(&doc_id)
        .is_some_and(|document| document.medium.is_script());
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
                        color: Color32::from_black_alpha(24),
                    })
                    .inner_margin(Margin {
                        left: if is_script {
                            108
                        } else if app.focus_mode {
                            64
                        } else {
                            54
                        },
                        right: if is_script {
                            72
                        } else if app.focus_mode {
                            64
                        } else {
                            54
                        },
                        top: if is_script { 72 } else { 44 },
                        bottom: if is_script { 72 } else { 34 },
                    })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
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

/// Folders are pure containers, so the Write view shows this panel rather than
/// a paper page: an editable name and synopsis, then the documents inside.
fn folder_panel(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    let synopsis_sheet =
        portmanteau_core::StyleSheet::novel_prose(&app.ws.project.styles);
    let Some(doc) = app.ws.project.document(doc_id) else {
        return;
    };
    let original_title = doc.title.clone();
    let original_synopsis = doc.synopsis.editor_text();
    let children: Vec<(String, String, DocKind, Medium, u32)> = app
        .ws
        .project
        .children(Some(doc_id))
        .into_iter()
        .map(|child| {
            (
                child.id.clone(),
                child.title.clone(),
                child.kind,
                child.medium,
                child.words(),
            )
        })
        .collect();

    let mut title = original_title.clone();
    let mut synopsis = original_synopsis.clone();
    let mut open_child: Option<String> = None;

    let subtitle = match children.len() {
        1 => "1 document in this folder.".to_string(),
        n => format!("{n} documents in this folder."),
    };
    page_head(ui, "Folder", &title, &subtitle, |_ui| {});

    ScrollArea::vertical()
        .id_salt("folder_panel_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            content_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                section_label(ui, "Folder name");
                ui.add_space(4.0);
                ui.add(
                    TextEdit::singleline(&mut title)
                        .font(serif(22.0))
                        .text_color(TEXT)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(16.0);
                section_label(ui, "Synopsis");
                ui.add_space(4.0);
                ui.add(
                    TextEdit::multiline(&mut synopsis)
                        .font(serif(13.0))
                        .text_color(TEXT)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .hint_text(
                            RichText::new("What belongs in this folder?")
                                .font(serif(13.0))
                                .color(FAINT),
                        ),
                );
                ui.add_space(20.0);
                section_label(ui, "Contents");
                ui.add_space(6.0);
                if children.is_empty() {
                    ui.label(
                        RichText::new("This folder is empty.")
                            .font(ui_body())
                            .color(FAINT),
                    );
                } else {
                    for (child_id, child_title, kind, medium, words) in &children {
                        if folder_child_row(ui, child_title, *kind, *medium, *words) {
                            open_child = Some(child_id.clone());
                        }
                    }
                }
                ui.add_space(18.0);
                ui.label(
                    RichText::new(
                        "Folders group documents. Add content with the + button in the binder.",
                    )
                    .font(ui_small())
                    .color(FAINT),
                );
            });
        });

    if title != original_title || synopsis != original_synopsis {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            if title != original_title {
                doc.title = title;
                doc.touch();
            }
            if synopsis != original_synopsis {
                doc.synopsis.sync_editor_text(&synopsis, &synopsis_sheet);
                doc.touch();
            }
        }
        app.ws.dirty = true;
    }

    if let Some(id) = open_child {
        app.open_doc(id);
    }
}

fn folder_child_row(
    ui: &mut egui::Ui,
    title: &str,
    kind: DocKind,
    medium: Medium,
    words: u32,
) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(5), PANEL_2);
    }
    let (glyph, color) = match kind {
        DocKind::Folder => ("▣", MUTED),
        DocKind::Chapter => ("¶", ACCENT_INK),
        DocKind::Scene if medium.is_script() => ("▶", PURPLE),
        DocKind::Scene => ("◆", SAGE),
        DocKind::Research => ("✎", BLUE),
        DocKind::Character => ("☺", PURPLE),
        DocKind::FrontMatter | DocKind::BackMatter => ("▤", FAINT),
        DocKind::Text => ("◆", FAINT),
    };
    ui.painter().text(
        egui::pos2(rect.left() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        glyph,
        ui_body(),
        color,
    );
    let right = if kind.is_folder() { 10.0 } else { 66.0 };
    let title_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 28.0, rect.top()),
        egui::pos2(rect.right() - right, rect.bottom()),
    );
    ui.painter().with_clip_rect(title_rect).text(
        title_rect.left_center(),
        egui::Align2::LEFT_CENTER,
        title,
        ui_body(),
        TEXT,
    );
    if !kind.is_folder() {
        ui.painter().text(
            egui::pos2(rect.right() - 10.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format!("{words} w"),
            mono(9.0),
            FAINT,
        );
    }
    resp.clicked()
}

fn toolbar(
    app: &mut PortmanteauApp,
    ui: &mut egui::Ui,
    page_total: Option<usize>,
    miss_count: usize,
) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .inner_margin(Margin::symmetric(16, 7))
        .show(ui, |ui| {
            let row_width = ui.available_width();
            ui.allocate_ui_with_layout(
                Vec2::new(row_width, TOOLBAR_CONTROL_H),
                Layout::left_to_right(Align::Center),
                |ui| {
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
                // One clipped breadcrumb leaves the right-hand controls a
                // fixed lane, so a long chapter title can never overlap the
                // word count or mode controls.
                let crumb = chain.join("  /  ");
                let style_slot = if medium.is_script() && app.write_mode == WriteMode::Continuous {
                    190.0
                } else {
                    0.0
                };
                let crumb_width = (ui.available_width() - 520.0 - style_slot).clamp(70.0, 420.0);
                ui.scope(|ui| {
                    ui.set_max_width(crumb_width);
                    ui.add(
                        egui::Label::new(
                            RichText::new(crumb)
                                .font(ui_body())
                                .color(MUTED),
                        )
                        .truncate(),
                    );
                });

                if style_slot > 0.0 {
                    script_style_dropdown(app, ui, &id);
                }

                let mut new_mode = app.write_mode;
                ui.with_layout(Layout::right_to_left(Align::Center).with_cross_justify(true), |ui| {
                    if app.settings.spellcheck {
                        let label = if miss_count > 0 {
                            format!("Spelling · {miss_count}")
                        } else {
                            "Spelling".to_string()
                        };
                        let color = if app.show_spelling {
                            ACCENT_INK
                        } else if miss_count > 0 {
                            MUTED
                        } else {
                            FAINT
                        };
                        let spell = egui::Button::new(
                            RichText::new(label).font(ui_body()).color(color),
                        )
                        .fill(if app.show_spelling { ACCENT_SOFT } else { PANEL_2 })
                        .corner_radius(CornerRadius::same(5))
                        .min_size(Vec2::new(0.0, TOOLBAR_CONTROL_H));
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
                        .inner_margin(Margin::symmetric(2, 2))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 1.0;
                                ui.spacing_mut().button_padding = Vec2::new(4.0, 2.0);
                                for mode in [WriteMode::Continuous, WriteMode::Paged] {
                                    let active = new_mode == mode;
                                    let btn = egui::Button::new(
                                        RichText::new(mode.label())
                                            .font(ui_small())
                                            .color(if active { ACCENT_INK } else { MUTED }),
                                    )
                                    .fill(if active { ACCENT_SOFT } else { PANEL_2 })
                                    .corner_radius(CornerRadius::same(4))
                                    .min_size(Vec2::new(0.0, TOOLBAR_GROUP_H));
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
                            .font(ui_body())
                            .color(if app.focus_mode { ACCENT_INK } else { MUTED }),
                    )
                    .fill(PANEL_2)
                    .corner_radius(CornerRadius::same(5))
                    .min_size(Vec2::new(0.0, TOOLBAR_CONTROL_H));
                    if ui
                        .add(focus)
                        .on_hover_text("Hide the binder and inspector (F11)")
                        .clicked()
                    {
                        app.focus_mode = !app.focus_mode;
                    }

                    editor_zoom_control(app, ui);

                    // Paged mode is a read-only preview, so the medium
                    // switcher is hidden there and the space goes to the page
                    // readout instead.
                    if app.write_mode == WriteMode::Continuous {
                        let mut new_medium = medium;
                        ComboBox::from_id_salt("editor_medium")
                            .selected_text(RichText::new(medium.label()).font(ui_body()).color(MUTED))
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
                });
                if new_mode != app.write_mode {
                    if new_mode == WriteMode::Paged {
                        app.paged_page = 1;
                    }
                    app.write_mode = new_mode;
                }
                },
            );
        });
}

/// Paragraph-style dropdown for script documents. It sets the style of the
/// paragraph that holds the caret and returns focus to the writing surface.
fn script_style_dropdown(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    let selected = app.selected_block;
    let Some(doc) = app.ws.project.document(doc_id) else {
        return;
    };
    let sheet = app.ws.project.style_sheet_for(doc);
    let current = doc
        .content
        .paragraphs
        .get(selected)
        .map(|paragraph| paragraph.style_id.clone())
        .unwrap_or_else(|| sheet.default_style.clone());
    let current_name = sheet
        .style(&current)
        .map(|style| style.name.clone())
        .unwrap_or_else(|| "Style".to_owned());
    let mut chosen: Option<String> = None;
    ComboBox::from_id_salt("script_paragraph_style")
        .selected_text(RichText::new(current_name).font(ui_body()).color(TEXT))
        .width(150.0)
        .show_ui(ui, |ui| {
            for style in &sheet.styles {
                if ui
                    .selectable_label(
                        style.id == current,
                        RichText::new(&style.name).font(ui_body()),
                    )
                    .clicked()
                {
                    chosen = Some(style.id.clone());
                }
            }
        });
    if let Some(style_id) = chosen {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            if let Some(paragraph) = doc.content.paragraphs.get_mut(selected) {
                paragraph.style_id = style_id;
                doc.touch();
                app.ws.dirty = true;
                app.block_focus = Some(selected);
            }
        }
    }
}

/// Compact `−` / percentage / `+` control for the writing surface. It changes
/// only view state; the project is untouched and nothing is saved.
fn editor_zoom_control(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let zoom = app.editor_zoom.clamp(0.7, 1.6);
    let mut next = zoom;
    // A fixed-width allocation keeps the group compact inside the
    // right-aligned toolbar row instead of stretching across it.
    ui.allocate_ui_with_layout(
        Vec2::new(96.0, TOOLBAR_CONTROL_H),
        Layout::left_to_right(Align::Center),
        |ui| {
            Frame::new()
                .fill(PANEL_2)
                .corner_radius(CornerRadius::same(5))
                .inner_margin(Margin::symmetric(2, 2))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    ui.spacing_mut().button_padding = Vec2::new(4.0, 2.0);
                    if zoom_button(ui, "−").on_hover_text("Zoom out").clicked() {
                        next = step_zoom(zoom, -1);
                    }
                    ui.add_sized(
                        Vec2::new(44.0, TOOLBAR_GROUP_H),
                        egui::Label::new(
                            RichText::new(format!("{}%", (zoom * 100.0).round() as i32))
                                .font(ui_small())
                                .color(MUTED),
                        )
                        .selectable(false),
                    )
                    .on_hover_text("Editor zoom");
                    if zoom_button(ui, "+").on_hover_text("Zoom in").clicked() {
                        next = step_zoom(zoom, 1);
                    }
                });
        },
    );
    app.editor_zoom = next;
}

fn zoom_button(ui: &mut egui::Ui, glyph: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(glyph).font(ui_body()).color(MUTED))
            .fill(PANEL_2)
            .stroke(Stroke::new(1.0, BORDER_SOFT))
            .corner_radius(CornerRadius::same(4))
            .min_size(Vec2::new(22.0, TOOLBAR_GROUP_H)),
    )
}

fn step_zoom(zoom: f32, direction: i32) -> f32 {
    let stepped = ((zoom * 10.0).round() + direction as f32) / 10.0;
    stepped.clamp(0.7, 1.6)
}

/// The spelling review strip under the toolbar. Suggestions are collected
/// during the draw and the replacement is applied after it, so the document is
/// never borrowed while the widgets run (AGENTS.md §8).
fn spelling_panel(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str, miss: &[String]) {
    let mut action: Option<(String, String)> = None;
    Frame::new()
        .fill(PANEL_2)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .inner_margin(Margin::symmetric(16, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Spelling · {}", miss.len()))
                        .font(ui_body())
                        .color(if miss.is_empty() { FAINT } else { MUTED }),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let close = egui::Button::new(
                        RichText::new("Close").font(ui_small()).color(FAINT),
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
                        .font(ui_small())
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
                                                .font(ui_small())
                                                .color(FAINT),
                                        );
                                    } else {
                                        for suggestion in &suggestions {
                                            let button = egui::Button::new(
                                                RichText::new(suggestion)
                                                    .font(ui_small())
                                                    .color(ACCENT_INK),
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
            for paragraph in &mut doc.content.paragraphs {
                let current = paragraph.plain_text();
                let next = replace_word_in_text(&current, &word, &replacement);
                if next != current {
                    paragraph.set_plain_text(next);
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

/// Colour of the squiggle under a misspelled word. Deliberately softer than
/// pure red so it reads on the warm paper background.
const SPELL_UNDERLINE: Color32 = Color32::from_rgb(196, 64, 64);

/// Build the text layout for an editor surface, underlining the misspelled
/// byte ranges.
///
/// `base` carries the element's font, colour and line height; only the
/// underline differs on a misspelled range. Ranges are clamped and must fall
/// on char boundaries (they come from the tokeniser, but a stale span must
/// never panic). An empty text still gets one section so the layout matches
/// the old `single_section` path exactly.
fn spell_layout_job(
    text: &str,
    spans: &[(usize, usize)],
    base: &egui::TextFormat,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    if text.is_empty() {
        job.append("", 0.0, base.clone());
        return job;
    }
    let underline = egui::TextFormat {
        underline: Stroke::new(1.4, SPELL_UNDERLINE),
        ..base.clone()
    };
    let mut cursor = 0usize;
    for &(start, end) in spans {
        let start = start.min(text.len());
        let end = end.min(text.len());
        if start < cursor
            || start >= end
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
        {
            continue;
        }
        if cursor < start {
            job.append(&text[cursor..start], 0.0, base.clone());
        }
        job.append(&text[start..end], 0.0, underline.clone());
        cursor = end;
    }
    if cursor < text.len() {
        job.append(&text[cursor..], 0.0, base.clone());
    }
    job
}

/// What a right-click choice in the spelling menu asks the editor to do.
enum SpellAction {
    Replace {
        doc_id: String,
        /// `Some(index)` for a screenplay block, `None` for the prose body.
        block: Option<usize>,
        range: std::ops::Range<usize>,
        replacement: String,
    },
    Ignore(String),
    Add(String),
}

/// A choice made in the spelling context menu, before it is attached to a
/// document or block.
enum SpellChoice {
    Replace(std::ops::Range<usize>, String),
    Ignore(String),
    Add(String),
}

/// Draw the spelling context menu for the word at `byte` and report the
/// choice. Labels are the visible button text, which also keeps the menu
/// drivable by `egui_kittest`'s AccessKit queries.
fn spell_menu_ui(
    ui: &mut egui::Ui,
    checker: &SpellChecker,
    text: &str,
    byte: usize,
) -> Option<SpellChoice> {
    let Some((range, word)) = word_at(text, byte) else {
        ui.label(RichText::new("No word here").font(ui_small()).color(FAINT));
        return None;
    };
    if checker.is_correct(&word) {
        ui.label(
            RichText::new(format!("“{word}” is spelled correctly"))
                .font(ui_small())
                .color(FAINT),
        );
        return None;
    }
    ui.label(
        RichText::new(format!("Correct “{word}”"))
            .font(ui_small())
            .color(MUTED)
            .strong(),
    );
    let suggestions = checker.suggest(&word, 6);
    if suggestions.is_empty() {
        ui.label(RichText::new("(no suggestions)").font(ui_small()).color(FAINT));
    } else {
        for suggestion in suggestions {
            let label = match_initial_case(&word, &suggestion);
            let button = egui::Button::new(RichText::new(&label).font(ui_small()));
            if ui.add(button).clicked() {
                ui.close();
                return Some(SpellChoice::Replace(range, label));
            }
        }
    }
    ui.separator();
    if ui
        .add(egui::Button::new(
            RichText::new("Ignore for this session").font(ui_small()),
        ))
        .clicked()
    {
        ui.close();
        return Some(SpellChoice::Ignore(word));
    }
    if ui
        .add(egui::Button::new(
            RichText::new("Add to dictionary").font(ui_small()),
        ))
        .clicked()
    {
        ui.close();
        return Some(SpellChoice::Add(word));
    }
    None
}

/// Byte offset the spelling menu should inspect for `text`.
///
/// The offset is captured on the frame of the secondary click and stashed in
/// egui's temporary data, because the popup's contents also run on later
/// frames when `interact_pointer_pos` is no longer set. Without a stored
/// offset the caret (or the widget centre) is used as a fallback.
fn spell_menu_byte(
    ctx: &egui::Context,
    key: Id,
    response: &egui::Response,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    text: &str,
) -> usize {
    if response.secondary_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let cursor = galley.cursor_from_pos(pos - galley_pos);
            let byte = byte_offset(text, cursor.index.0);
            ctx.data_mut(|data| data.insert_temp(key, byte));
        }
    }
    if let Some(byte) = ctx.data(|data| data.get_temp::<usize>(key)) {
        return byte.min(text.len());
    }
    let pos = response
        .interact_pointer_pos()
        .unwrap_or_else(|| response.rect.center());
    let cursor = galley.cursor_from_pos(pos - galley_pos);
    byte_offset(text, cursor.index.0)
}

/// Apply the menu's choices after the document borrow has ended, so the
/// document is never borrowed while a widget runs (AGENTS.md §8).
fn apply_spell_actions(app: &mut PortmanteauApp, actions: Vec<SpellAction>) {
    for action in actions {
        match action {
            SpellAction::Replace {
                doc_id,
                block,
                range,
                replacement,
            } => {
                let Some(sheet) = app
                    .ws
                    .project
                    .style_sheets
                    .iter()
                    .find(|sheet| {
                        app.ws
                            .project
                            .document(&doc_id)
                            .is_some_and(|doc| doc.style_sheet_id == sheet.id)
                    })
                    .cloned()
                else {
                    continue;
                };
                let Some(doc) = app.ws.project.document_mut(&doc_id) else {
                    continue;
                };
                let text = doc.content.editor_text();
                let (start_byte, end_byte) = if let Some(index) = block {
                    let Some(paragraph) = doc.content.paragraphs.get(index) else {
                        continue;
                    };
                    let local = paragraph.editor_text();
                    if range.start > range.end
                        || range.end > local.len()
                        || !local.is_char_boundary(range.start)
                        || !local.is_char_boundary(range.end)
                    {
                        continue;
                    }
                    let prefix_chars: usize = doc.content.paragraphs[..index]
                        .iter()
                        .map(|p| p.editor_text().chars().count() + 1)
                        .sum();
                    let start_chars = prefix_chars + local[..range.start].chars().count();
                    let end_chars = prefix_chars + local[..range.end].chars().count();
                    let start_byte = text
                        .char_indices()
                        .nth(start_chars)
                        .map(|(byte, _)| byte)
                        .unwrap_or(text.len());
                    let end_byte = text
                        .char_indices()
                        .nth(end_chars)
                        .map(|(byte, _)| byte)
                        .unwrap_or(text.len());
                    (start_byte, end_byte)
                } else {
                    if range.start > range.end
                        || range.end > text.len()
                        || !text.is_char_boundary(range.start)
                        || !text.is_char_boundary(range.end)
                    {
                        continue;
                    }
                    (range.start, range.end)
                };
                if text.is_char_boundary(start_byte) && text.is_char_boundary(end_byte) {
                    let start = text[..start_byte].chars().count();
                    let end = text[..end_byte].chars().count();
                    doc.content
                        .replace_range(start..end, &replacement, &sheet);
                    doc.touch();
                    app.ws.dirty = true;
                }
            }
            SpellAction::Ignore(word) => app.spell.ignore(&word),
            SpellAction::Add(word) => app.add_to_dictionary(&word),
        }
    }
}

fn prose_page(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str, take_snapshot: &mut bool) {
    let mut dirty = false;
    let zoom = app.editor_zoom.clamp(0.7, 1.6) * PROSE_ZOOM_BASE;
    // The prose surface takes its type from the project's body element style,
    // the same style the compiled page and preview use.
    let styles = app.ws.project.styles.clone();
    let body = styles.body.clone();
    let family = family_for_font(&body.font);
    let text_size = body.size.max(6.0) * zoom;
    let project_title = app.ws.project.title.clone();
    let sheet = app
        .ws
        .project
        .document(doc_id)
        .and_then(|doc| {
            app.ws
                .project
                .style_sheets
                .iter()
                .find(|sheet| sheet.id == doc.style_sheet_id)
                .cloned()
        })
        .unwrap_or_else(|| portmanteau_core::StyleSheet::novel_prose(&styles));
    let content_snapshot = app
        .ws
        .project
        .document(doc_id)
        .map(|doc| doc.content.clone())
        .unwrap_or_default();
    let image_textures = load_rich_image_textures(app, ui.ctx(), &content_snapshot);
    let spell_on = app.settings.spellcheck;
    // Disjoint borrow of the checker, taken before the project's document is
    // borrowed mutably, so the layouter and menu can read live suggestions.
    let checker = &app.spell;
    let mut spell_actions: Vec<SpellAction> = Vec::new();
    {
        let Some(doc) = app.ws.project.document_mut(doc_id) else { return };
        paper_header(ui, &project_title, doc.medium, &doc.status, doc.words(), doc.pages());

        ui.add_space(16.0);
        if ui
            .add(
                TextEdit::singleline(&mut doc.title)
                    .font(serif(29.0))
                    .text_color(PAPER_TEXT)
                    .frame(Frame::NONE)
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
        let prose_id = Id::unique(("prose_body", doc_id));
        let ctx = ui.ctx().clone();
        let toolbar = inline_format_toolbar(ui, &ctx, prose_id, &mut doc.content);
        if toolbar.changed {
            doc.touch();
            dirty = true;
        }
        if let Some(range) = toolbar.insert_image {
            app.image_insert = Some(ImageInsertState {
                target: RichTextTarget::DocumentBody(doc_id.to_owned()),
                range,
                path: String::new(),
                alt_text: String::new(),
                caption: String::new(),
                standalone: false,
                error: String::new(),
            });
        }

        let mut edit_text = doc.content.editor_text();
        let rows = ((edit_text.chars().count() as f32) / 72.0).ceil().max(18.0) as usize;
        // One dictionary pass per frame, reused by every layouter call.
        let spans = if spell_on {
            checker.misspelled_spans(&edit_text)
        } else {
            Vec::new()
        };
        let content_for_layout = doc.content.clone();
        let sheet_for_layout = sheet.clone();
        let styles_for_layout = styles.clone();
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
            let job = script_flow_layout_job(
                text.as_str(),
                &content_for_layout,
                &sheet_for_layout,
                &styles_for_layout,
                zoom,
                wrap_width,
                &spans,
            );
            ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
        };
        let edit = TextEdit::multiline(&mut edit_text)
            .id(prose_id)
            .font(FontId::new(text_size, family.clone()))
            .layouter(&mut layouter)
            .text_color(PAPER_TEXT)
            .frame(Frame::NONE)
            .desired_width(f32::INFINITY)
            .desired_rows(rows.min(120))
            .margin(Margin::same(0))
            .hint_text(
                RichText::new("Begin at the beginning…")
                    .font(FontId::new(text_size, family.clone()))
                    .color(PAPER_MUTED),
            );
        let output = edit.show(ui);
        if output.response.changed() {
            doc.content.sync_editor_text(&edit_text, &sheet);
            doc.touch();
            dirty = true;
        }
        crate::ui::editor::remember_selection_anchor(ui.ctx(), output.response.id, &output.galley, output.galley_pos);
        paint_rich_images(
            ui,
            &output.galley,
            output.galley_pos,
            output.response.rect,
            &doc.content,
            &image_textures,
            zoom,
        );
        if spell_on {
            let key = Id::unique(("spell_menu", doc_id));
            let byte = spell_menu_byte(
                ui.ctx(),
                key,
                &output.response,
                &output.galley,
                output.galley_pos,
                &edit_text,
            );
            let mut choice = None;
            output.response.context_menu(|ui| {
                choice = spell_menu_ui(ui, checker, &edit_text, byte);
            });
            if let Some(choice) = choice {
                spell_actions.push(match choice {
                    SpellChoice::Replace(range, replacement) => SpellAction::Replace {
                        doc_id: doc_id.to_string(),
                        block: None,
                        range,
                        replacement,
                    },
                    SpellChoice::Ignore(word) => SpellAction::Ignore(word),
                    SpellChoice::Add(word) => SpellAction::Add(word),
                });
            }
        }
        ui.add_space(26.0);
        let words = doc.words();
        let pages = doc.pages();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} words  ·  {} min read", words, portmanteau_core::reading_minutes(words)))
                    .font(ui_small())
                    .color(PAPER_MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{pages}")).font(serif(9.0)).color(PAPER_MUTED));
            });
        });
        let _ = take_snapshot;
    }
    apply_spell_actions(app, spell_actions);
    if dirty {
        app.ws.dirty = true;
    }
}

fn script_page(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    script_flow_page(app, ui, doc_id);
}

/// Screenplay view over the same single flowing rich-text buffer used by
/// prose. Paragraph style ids carry screenplay semantics; the one TextEdit
/// owns the caret and selection for the complete document.
fn script_flow_page(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    let zoom = app.editor_zoom.clamp(0.7, 1.6) * SCRIPT_ZOOM_BASE;
    let project_title = app.ws.project.title.clone();
    let styles = app.ws.project.styles.clone();
    let sheet = app
        .ws
        .project
        .document(doc_id)
        .and_then(|doc| {
            app.ws
                .project
                .style_sheets
                .iter()
                .find(|sheet| sheet.id == doc.style_sheet_id)
                .cloned()
        })
        .unwrap_or_else(|| portmanteau_core::StyleSheet::film_screenplay(&styles));
    let mut character_names = app
        .ws
        .project
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::Character)
        .map(|item| item.title.trim().to_owned())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    character_names.sort_by(|left, right| left.to_lowercase().cmp(&right.to_lowercase()));
    character_names.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    let content_snapshot = app
        .ws
        .project
        .document(doc_id)
        .map(|doc| doc.content.clone())
        .unwrap_or_default();
    let image_textures = load_rich_image_textures(app, ui.ctx(), &content_snapshot);
    let checker = &app.spell;
    let spell_on = app.settings.spellcheck;
    let flow_id = Id::unique(("script_flow", doc_id));
    let mut spell_action: Option<SpellChoice> = None;
    let mut new_character: Option<NewCharacterState> = None;
    let mut dirty = false;

    {
        let Some(doc) = app.ws.project.document_mut(doc_id) else {
            return;
        };
        paper_header(ui, &project_title, doc.medium, &doc.status, doc.words(), doc.pages());
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(doc.title.to_uppercase())
                    .font(mono(10.0))
                    .color(PAPER_MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} paragraphs", doc.content.paragraphs.len()))
                        .font(mono(8.5))
                        .color(PAPER_MUTED),
                );
            });
        });
        ui.add_space(4.0);
        paper_rule(ui);
        ui.add_space(10.0);

        if doc.content.paragraphs.is_empty() {
            doc.content = portmanteau_core::RichDocument::empty(sheet.default_style.clone());
        }
        let caret = egui::text_edit::TextEditState::load(ui.ctx(), flow_id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| range.primary.index.0)
            .unwrap_or(0);
        let selected = doc.content.position_at(caret).paragraph;
        app.selected_block = selected;
        let selected_style = doc
            .content
            .paragraphs
            .get(selected)
            .map(|paragraph| paragraph.style_id.clone())
            .unwrap_or_else(|| sheet.default_style.clone());

        let mut chosen_style: Option<String> = None;

        if let Some(style_id) = chosen_style.take() {
            if let Some(paragraph) = doc.content.paragraphs.get_mut(selected) {
                paragraph.style_id = style_id;
                doc.touch();
                dirty = true;
            }
        }

        if let Some(focus_paragraph) = app.block_focus {
            ui.memory_mut(|memory| memory.request_focus(flow_id));
            let focus_char = rich_paragraph_start(&doc.content, focus_paragraph)
                + doc
                    .content
                    .paragraphs
                    .get(focus_paragraph)
                    .map(|paragraph| paragraph.editor_text().chars().count())
                    .unwrap_or(0);
            let mut state =
                egui::text_edit::TextEditState::load(ui.ctx(), flow_id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(focus_char),
            )));
            state.store(ui.ctx(), flow_id);
        }
        // A style change can be selected with the palette or by the familiar
        // screenplay shortcut. Tab remains a paragraph-style cycle; arrows
        // remain owned by this one TextEdit and move through the whole flow.
        if ui.memory(|memory| memory.has_focus(flow_id)) {
            ui.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
            let backwards =
                ui.input_mut(|input| input.consume_key(Modifiers::SHIFT, Key::Tab));
            let forwards = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Tab));
            let kind = ctrl_digit_element(ui, doc.medium);
            if backwards || forwards {
                if let Some(position) = sheet
                    .styles
                    .iter()
                    .position(|style| style.id == selected_style)
                {
                    let count = sheet.styles.len();
                    let next = if forwards {
                        (position + 1) % count
                    } else {
                        (position + count - 1) % count
                    };
                    chosen_style = Some(sheet.styles[next].id.clone());
                }
            } else if let Some(kind) = kind {
                chosen_style =
                    Some(portmanteau_core::rich_text::script_style_id(kind).to_owned());
            }
            if let Some(style_id) = chosen_style.take() {
                if let Some(paragraph) = doc.content.paragraphs.get_mut(selected) {
                    paragraph.style_id = style_id;
                    doc.touch();
                    dirty = true;
                }
            }
        }

        ui.add_space(8.0);
        let ctx = ui.ctx().clone();
        let toolbar = inline_format_toolbar(ui, &ctx, flow_id, &mut doc.content);
        if toolbar.changed {
            doc.touch();
            dirty = true;
        }
        if let Some(range) = toolbar.insert_image {
            app.image_insert = Some(ImageInsertState {
                target: RichTextTarget::DocumentBody(doc_id.to_owned()),
                range,
                path: String::new(),
                alt_text: String::new(),
                caption: String::new(),
                standalone: false,
                error: String::new(),
            });
        }
        ui.add_space(4.0);
        let before = doc.content.editor_text();
        let mut edit_text = before.clone();
        let spans = if spell_on {
            checker.misspelled_spans(&edit_text)
        } else {
            Vec::new()
        };
        let content_for_layout = doc.content.clone();
        let sheet_for_layout = sheet.clone();
        let styles_for_layout = styles.clone();
        let desired_rows = edit_text.lines().count().clamp(1, 2000);
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
            let job = script_flow_layout_job(
                text.as_str(),
                &content_for_layout,
                &sheet_for_layout,
                &styles_for_layout,
                zoom,
                wrap_width,
                &spans,
            );
            ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
        };
        let edit = TextEdit::multiline(&mut edit_text)
            .id(flow_id)
            .font(mono(12.0 * zoom))
            .layouter(&mut layouter)
            .text_color(PAPER_TEXT)
            .frame(Frame::NONE)
            .desired_width(f32::INFINITY)
            .desired_rows(desired_rows)
            .margin(Margin::symmetric(2, 1))
            .event_filter(egui::EventFilter {
                horizontal_arrows: true,
                vertical_arrows: true,
                tab: true,
                ..Default::default()
            })
            .hint_text(
                RichText::new("Action")
                    .font(mono(12.0 * zoom))
                    .color(PAPER_MUTED),
            );

        let mut typed_word = String::new();
        let mut typed_caret = 0usize;
        let mut text_before_completion = String::new();
        let mut character_paragraph = 0usize;
        let completion = egui::CompletionPopup::new(flow_id)
            .word_boundary(|ch| ch == '\n')
            .accept_keys(vec![Key::Enter])
            .align(egui::RectAlign::BOTTOM_START)
            .max_height(260.0)
            .show(ui, edit, |query| {
                typed_word = query.word.to_owned();
                typed_caret = query.word_range.end.0;
                text_before_completion = query.text.to_owned();
                let position = query
                    .text
                    .chars()
                    .take(typed_caret)
                    .filter(|ch| *ch == '\n')
                    .count();
                character_paragraph = position;
                let is_character = doc
                    .content
                    .paragraphs
                    .get(position)
                    .and_then(|paragraph| sheet.style(&paragraph.style_id))
                    .and_then(|style| style.semantic_kind)
                    == Some(ElementKind::Character);
                if !is_character {
                    return Vec::new();
                }

                let word = query.word.trim();
                if word.is_empty() {
                    let blocks = doc.content.to_script_blocks(&sheet);
                    return next_speaker(&blocks, position)
                        .map(|speaker| {
                            vec![egui::Suggestion::new(speaker.clone())
                                .content(format!("↺ {speaker}"))]
                        })
                        .unwrap_or_default();
                }
                let needle = word.to_lowercase();
                if character_names
                    .iter()
                    .any(|name| name.to_lowercase() == needle)
                {
                    return Vec::new();
                }
                let matches = character_names
                    .iter()
                    .filter(|name| name.to_lowercase().contains(&needle))
                    .map(|name| egui::Suggestion::new(name.clone()))
                    .collect::<Vec<_>>();
                if matches.is_empty() {
                    vec![egui::Suggestion::new(NEW_CHARACTER_SENTINEL)
                        .content("+ New character…")]
                } else {
                    matches
                }
            });

        if let Some(accepted) = &completion.accepted {
            if accepted.insert == NEW_CHARACTER_SENTINEL {
                edit_text = text_before_completion.clone();
                if let Some(mut state) =
                    egui::text_edit::TextEditState::load(ui.ctx(), flow_id)
                {
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                        egui::text::CCursor::new(typed_caret),
                    )));
                    state.store(ui.ctx(), flow_id);
                }
                new_character = Some(NewCharacterState {
                    doc_id: doc_id.to_owned(),
                    block: character_paragraph,
                    name: typed_word,
                    role: String::new(),
                });
            } else {
                dirty = true;
            }
        }

        if completion.text_edit.response.changed() || completion.accepted.is_some() {
            doc.content.sync_editor_text(&edit_text, &sheet);
            doc.touch();
            dirty = true;
        }
        crate::ui::editor::remember_selection_anchor(ui.ctx(), completion.text_edit.response.id, &completion.text_edit.galley, completion.text_edit.galley_pos);
        paint_rich_images(
            ui,
            &completion.text_edit.galley,
            completion.text_edit.galley_pos,
            completion.text_edit.response.rect,
            &doc.content,
            &image_textures,
            zoom,
        );
        if completion.text_edit.response.has_focus() {
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    flow_id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        tab: true,
                        escape: false,
                    },
                )
            });
        }
        if let Some(range) = completion
            .text_edit
            .cursor_range
            .map(|range| range.primary.index.0)
        {
            app.selected_block = doc.content.position_at(range).paragraph;
        }
        if spell_on {
            let key = Id::unique(("script_spell_menu", doc_id));
            let byte = spell_menu_byte(
                ui.ctx(),
                key,
                &completion.text_edit.response,
                &completion.text_edit.galley,
                completion.text_edit.galley_pos,
                &edit_text,
            );
            completion.text_edit.response.context_menu(|ui| {
                if let Some(choice) = spell_menu_ui(ui, checker, &edit_text, byte) {
                    spell_action = Some(choice);
                }
            });
        }
        app.block_focus = None;
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!(
                "{} paragraphs · {} words",
                doc.content.paragraphs.len(),
                doc.words()
            ))
            .font(ui_small())
            .color(PAPER_MUTED),
        );
    }

    if let Some(state) = new_character {
        app.new_character = Some(state);
    }
    if let Some(choice) = spell_action {
        match choice {
            SpellChoice::Replace(range, replacement) => apply_spell_actions(
                app,
                vec![SpellAction::Replace {
                    doc_id: doc_id.to_owned(),
                    block: None,
                    range,
                    replacement,
                }],
            ),
            SpellChoice::Ignore(word) => app.spell.ignore(&word),
            SpellChoice::Add(word) => app.add_to_dictionary(&word),
        }
        dirty = true;
    }
    if dirty {
        app.ws.dirty = true;
    }
}

pub(crate) fn script_flow_layout_job(
    text: &str,
    content: &portmanteau_core::RichDocument,
    sheet: &portmanteau_core::StyleSheet,
    styles: &portmanteau_core::ElementStyles,
    zoom: f32,
    wrap_width: f32,
    misspelled: &[(usize, usize)],
) -> egui::text::LayoutJob {
    use egui::text::LayoutJob;

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    let mut byte_start = 0usize;
    let lines = if text.is_empty() {
        vec![""]
    } else {
        text.split_inclusive('\n').collect::<Vec<_>>()
    };
    for (index, line) in lines.into_iter().enumerate() {
        let style_id = content
            .paragraphs
            .get(index)
            .map(|paragraph| paragraph.style_id.as_str())
            .unwrap_or(sheet.default_style.as_str());
        let paragraph_style = sheet.style(style_id);
        let kind = paragraph_style
            .and_then(|style| style.semantic_kind)
            .unwrap_or(ElementKind::Action);
        let element_style = paragraph_style
            .map(|style| style.format.clone())
            .unwrap_or_else(|| styles.style_for_kind(kind).clone());
        let family = family_for_font(&element_style.font);
        let font_size = element_style.size.max(6.0) * zoom;
        let format = egui::TextFormat {
            font_id: FontId::new(font_size, family),
            color: if kind.is_uppercase() {
                Color32::from_rgb(44, 40, 33)
            } else {
                PAPER_TEXT
            },
            line_height: Some(font_size * element_style.leading.max(0.85)),
            ..Default::default()
        };
        let uppercase = paragraph_style
            .is_some_and(|style| style.text_transform == portmanteau_core::TextTransform::Uppercase)
            || element_style.uppercase;
        let display_line = transform_for_display(line, uppercase);
        let measured_width = line
            .trim_end_matches('\n')
            .chars()
            .count() as f32
            * font_size
            * 0.6;
        let aligned_indent = match element_style.align {
            portmanteau_core::Alignment::Left | portmanteau_core::Alignment::Justify => 0.0,
            portmanteau_core::Alignment::Center => (wrap_width - measured_width) * 0.5,
            portmanteau_core::Alignment::Right => wrap_width - measured_width,
        };
        let leading = (element_style.indent + element_style.first_line_indent)
            .max(aligned_indent)
            .max(0.0)
            * zoom;
        let paragraph = content.paragraphs.get(index);
        let raw_char_bytes: Vec<usize> = line.char_indices().map(|(byte, _)| byte).collect();
        let display_char_bytes: Vec<usize> =
            display_line.char_indices().map(|(byte, _)| byte).collect();
        let mut keys = Vec::with_capacity(raw_char_bytes.len());
        for (char_index, raw_byte) in raw_char_bytes.iter().enumerate() {
            let global_byte = byte_start + raw_byte;
            let char_end = raw_char_bytes
                .get(char_index + 1)
                .copied()
                .unwrap_or(line.len());
            let misspelled_char = !uppercase
                && misspelled
                    .iter()
                    .any(|span| span.0 < byte_start + char_end && span.1 > global_byte);
            let marks = paragraph
                .map(|paragraph| inline_marks_at(&paragraph.content, char_index))
                .unwrap_or_default();
            keys.push((marks, misspelled_char));
        }
        let mut first = true;
        let mut start_char = 0usize;
        while start_char < keys.len() {
            let key = keys[start_char].clone();
            let mut end_char = start_char + 1;
            while end_char < keys.len() && keys[end_char] == key {
                end_char += 1;
            }
            let start_byte = display_char_bytes[start_char];
            let end_byte = display_char_bytes
                .get(end_char)
                .copied()
                .unwrap_or(display_line.len());
            let mut run_format = format.clone();
            apply_inline_marks(&mut run_format, &key.0);
            if end_char == start_char + 1 {
                if let Some(image) = paragraph
                    .and_then(|paragraph| inline_image_at(&paragraph.content, start_char))
                {
                    let image_height = image.height.unwrap_or(20.0) * zoom;
                    run_format.font_id.size = image_height.max(font_size);
                    run_format.line_height = Some(image_height.max(font_size));
                    run_format.color = Color32::TRANSPARENT;
                }
            }
            if key.1 {
                run_format.underline = Stroke::new(1.4, Color32::from_rgb(196, 64, 64));
            }
            job.append(
                &display_line[start_byte..end_byte],
                if first { leading } else { 0.0 },
                run_format,
            );
            first = false;
            start_char = end_char;
        }
        byte_start += line.len();
    }
    job
}

fn inline_marks_at(
    content: &[portmanteau_core::InlineContent],
    char_index: usize,
) -> Vec<portmanteau_core::InlineMark> {
    let mut offset = 0usize;
    for item in content {
        match item {
            portmanteau_core::InlineContent::Text(run) => {
                let length = run.text.chars().count();
                if char_index < offset + length {
                    return run.marks.clone();
                }
                offset += length;
            }
            portmanteau_core::InlineContent::HardBreak
            | portmanteau_core::InlineContent::Image(_) => offset += 1,
        }
    }
    Vec::new()
}

fn inline_image_at(
    content: &[portmanteau_core::InlineContent],
    char_index: usize,
) -> Option<&portmanteau_core::ImageObject> {
    let mut offset = 0usize;
    for item in content {
        match item {
            portmanteau_core::InlineContent::Text(run) => {
                offset += run.text.chars().count();
            }
            portmanteau_core::InlineContent::HardBreak => offset += 1,
            portmanteau_core::InlineContent::Image(image) => {
                if offset == char_index {
                    return Some(image);
                }
                offset += 1;
            }
        }
    }
    None
}

fn apply_inline_marks(
    format: &mut egui::TextFormat,
    marks: &[portmanteau_core::InlineMark],
) {
    use portmanteau_core::InlineMark;
    if marks.contains(&InlineMark::Bold) {
        format.font_id.family = egui::FontFamily::Name("portmanteau-sans-bold".into());
    }
    format.italics = marks.contains(&InlineMark::Italic);
    format.underline = if marks.contains(&InlineMark::Underline) {
        Stroke::new(1.0, format.color)
    } else {
        Stroke::NONE
    };
    format.strikethrough = if marks.contains(&InlineMark::Strikethrough) {
        Stroke::new(1.0, format.color)
    } else {
        Stroke::NONE
    };
}

fn image_cache_key(project_id: &str, asset_id: &str) -> String {
    format!("{project_id}:{asset_id}")
}

pub(crate) fn load_rich_image_textures(
    app: &mut PortmanteauApp,
    ctx: &egui::Context,
    content: &portmanteau_core::RichDocument,
) -> HashMap<String, egui::TextureHandle> {
    let Some(root) = app.ws.path.clone() else {
        return HashMap::new();
    };
    let project_id = app.ws.project.id.clone();
    let images = content
        .paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.content.iter())
        .filter_map(|item| match item {
            portmanteau_core::InlineContent::Image(image) => Some(image.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut loaded = HashMap::new();
    for image in images {
        let key = image_cache_key(&project_id, &image.asset_id);
        if let Some(texture) = app.image_textures.get(&key) {
            loaded.insert(image.asset_id, texture.clone());
            continue;
        }
        let relative = std::path::Path::new(&image.asset_id);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            })
        {
            continue;
        }
        let path = root.join("assets").join(relative);
        let Ok(decoded) = image::open(path) else {
            continue;
        };
        let rgba = decoded.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let texture = ctx.load_texture(key.clone(), color, egui::TextureOptions::LINEAR);
        app.image_textures.insert(key, texture.clone());
        loaded.insert(image.asset_id, texture);
    }
    loaded
}

pub(crate) fn paint_rich_images(
    ui: &egui::Ui,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    widget_rect: egui::Rect,
    content: &portmanteau_core::RichDocument,
    textures: &HashMap<String, egui::TextureHandle>,
    zoom: f32,
) {
    let mut char_offset = 0usize;
    for paragraph in &content.paragraphs {
        for item in &paragraph.content {
            match item {
                portmanteau_core::InlineContent::Text(run) => {
                    char_offset += run.text.chars().count();
                }
                portmanteau_core::InlineContent::HardBreak => char_offset += 1,
                portmanteau_core::InlineContent::Image(image) => {
                    let cursor = galley.pos_from_cursor(egui::text::CCursor::new(char_offset));
                    let mut center = galley_pos + cursor.center().to_vec2();
                    let standalone = paragraph.style_id == "image";
                    if standalone {
                        center.x = widget_rect.center().x;
                    }
                    let width = image
                        .width
                        .unwrap_or(if standalone { 240.0 } else { 20.0 })
                        .max(12.0)
                        * zoom;
                    let height = image
                        .height
                        .unwrap_or(if standalone { 150.0 } else { 20.0 })
                        .max(12.0)
                        * zoom;
                    let rect = egui::Rect::from_center_size(center, Vec2::new(width, height));
                    if let Some(texture) = textures.get(&image.asset_id) {
                        ui.painter().image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(3),
                            Color32::from_rgb(227, 223, 214),
                        );
                        let label = if image.alt_text.is_empty() {
                            "Image"
                        } else {
                            image.alt_text.as_str()
                        };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            ui_small(),
                            PAPER_MUTED,
                        );
                    }
                    char_offset += 1;
                }
            }
        }
        char_offset += 1;
    }
}

fn rich_paragraph_start(content: &portmanteau_core::RichDocument, index: usize) -> usize {
    content
        .paragraphs
        .iter()
        .take(index)
        .map(|paragraph| paragraph.editor_text().chars().count() + 1)
        .sum()
}

pub(crate) struct InlineToolbarResult {
    pub changed: bool,
    pub insert_image: Option<Range<usize>>,
}

#[derive(Clone, Copy, Default)]
struct SelectionAnchor {
    rect: Option<egui::Rect>,
    focused: bool,
    popup_hovered: bool,
}

/// Remember where the current text selection sits on screen so the floating
/// format popup can be placed beside it on the next frame. Call after the
/// `TextEdit` has been shown.
pub(crate) fn remember_selection_anchor(
    ctx: &egui::Context,
    id: Id,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
) {
    let range = egui::text_edit::TextEditState::load(ctx, id)
        .and_then(|state| state.cursor.char_range());
    let key = Id::unique(("selection_anchor", id));
    let mut anchor: SelectionAnchor = ctx.data(|data| data.get_temp(key)).unwrap_or_default();
    anchor.focused = ctx.memory(|memory| memory.has_focus(id));
    anchor.rect = range.and_then(|range| {
        let start = range.primary.index.0.min(range.secondary.index.0);
        let end = range.primary.index.0.max(range.secondary.index.0);
        if start == end {
            return None;
        }
        let first = galley.pos_from_cursor(egui::text::CCursor::new(start));
        let last = galley.pos_from_cursor(egui::text::CCursor::new(end));
        Some(egui::Rect::from_two_pos(first.min, last.max).translate(galley_pos.to_vec2()))
    });
    ctx.data_mut(|data| data.insert_temp(key, anchor));
}

/// Floating bold / italic / underline / strike popup that appears above the
/// selected text. The text editors all share it.
pub(crate) fn inline_format_toolbar(
    _ui: &mut egui::Ui,
    ctx: &egui::Context,
    id: Id,
    content: &mut portmanteau_core::RichDocument,
) -> InlineToolbarResult {
    let key = Id::unique(("selection_anchor", id));
    let mut anchor: SelectionAnchor = ctx.data(|data| data.get_temp(key)).unwrap_or_default();
    let mut changed = false;
    let selection = egui::text_edit::TextEditState::load(ctx, id)
        .and_then(|state| state.cursor.char_range())
        .map(|range| {
            range.primary.index.0.min(range.secondary.index.0)
                ..range.primary.index.0.max(range.secondary.index.0)
        })
        .filter(|range| range.start < range.end);
    let visible = selection.is_some() && (anchor.focused || anchor.popup_hovered);
    let (Some(rect), true) = (anchor.rect, visible) else {
        anchor.popup_hovered = false;
        ctx.data_mut(|data| data.insert_temp(key, anchor));
        return InlineToolbarResult { changed, insert_image: None };
    };
    let screen = ctx.content_rect();
    let width = 4.0 * 30.0 + 14.0;
    let x = (rect.left() - 6.0).clamp(screen.left() + 4.0, (screen.right() - width - 4.0).max(0.0));
    let y = if rect.top() - 44.0 < screen.top() + 4.0 {
        rect.bottom() + 8.0
    } else {
        rect.top() - 44.0
    };
    let area = egui::Area::new(Id::unique(("format_popup", id)))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(x, y))
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(CornerRadius::same(8))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 14,
                    spread: 0,
                    color: Color32::from_black_alpha(36),
                })
                .inner_margin(Margin::symmetric(5, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for (label, mark, tip) in [
                            ("B", portmanteau_core::InlineMark::Bold, "Bold"),
                            ("I", portmanteau_core::InlineMark::Italic, "Italic"),
                            ("U", portmanteau_core::InlineMark::Underline, "Underline"),
                            ("S", portmanteau_core::InlineMark::Strikethrough, "Strikethrough"),
                        ] {
                            let mut text = RichText::new(label).font(ui_strong()).color(TEXT);
                            text = match mark {
                                portmanteau_core::InlineMark::Bold => text.strong(),
                                portmanteau_core::InlineMark::Italic => text.italics(),
                                portmanteau_core::InlineMark::Underline => text.underline(),
                                portmanteau_core::InlineMark::Strikethrough => text.strikethrough(),
                            };
                            let button = egui::Button::new(text)
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(5))
                                .min_size(Vec2::new(28.0, 26.0));
                            if ui.add(button).on_hover_text(tip).clicked() {
                                if let Some(range) = &selection {
                                    changed |= content.toggle_mark(range.clone(), mark);
                                }
                            }
                        }
                    });
                });
        });
    anchor.popup_hovered = ctx
        .pointer_hover_pos()
        .is_some_and(|pos| area.response.rect.expand(4.0).contains(pos));
    ctx.data_mut(|data| data.insert_temp(key, anchor));
    InlineToolbarResult { changed, insert_image: None }
}

/// Transitional reference implementation retained while the flow editor is
/// built out. The live editor above uses one shared TextEdit for the entire
/// screenplay document.
#[allow(dead_code)]
fn legacy_script_page(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    let mut dirty = false;
    let zoom = app.editor_zoom.clamp(0.7, 1.6) * SCRIPT_ZOOM_BASE;
    let mut add_at: Option<usize> = None;
    let mut remove_at: Option<usize> = None;
    let mut cycle_at: Option<(usize, bool)> = None;
    let mut new_character: Option<NewCharacterState> = None;
    let project_title = app.ws.project.title.clone();
    // Clone the element styles before the document is borrowed mutably. Each
    // block takes its type and indent from the matching script element style,
    // the same table the paged preview and compile use.
    let styles = app.ws.project.styles.clone();
    let style_sheet = app
        .ws
        .project
        .document(doc_id)
        .and_then(|doc| {
            app.ws
                .project
                .style_sheets
                .iter()
                .find(|sheet| sheet.id == doc.style_sheet_id)
                .cloned()
        })
        .unwrap_or_else(|| portmanteau_core::StyleSheet::film_screenplay(&styles));
    let spell_on = app.settings.spellcheck;
    // Disjoint borrow of the checker, taken before the document is borrowed
    // mutably, so every block's layouter and menu can consult it live.
    let checker = &app.spell;
    let mut spell_actions: Vec<SpellAction> = Vec::new();
    // Character names are read before the document is borrowed mutably: the
    // autocomplete only needs owned strings from the project.
    let character_names: Vec<String> = {
        let mut names: Vec<String> = app
            .ws
            .project
            .items
            .iter()
            .filter(|item| item.kind == ItemKind::Character)
            .map(|item| item.title.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect();
        names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        names
    };
    {
        let Some(doc) = app.ws.project.document_mut(doc_id) else { return };
        let mut blocks = doc.content.to_script_blocks(&style_sheet);
        paper_header(ui, &project_title, doc.medium, &doc.status, doc.words(), doc.pages());
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(doc.title.to_uppercase())
                    .font(mono(10.0))
                    .color(PAPER_MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let synopsis = doc.synopsis.plain_text();
                let snippet: String = synopsis.chars().take(58).collect();
                let meta = if synopsis.chars().count() > 58 {
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
        let selected_kind = blocks
            .get(app.selected_block)
            .map(|b| b.kind)
            .unwrap_or(ElementKind::Action);
        ui.horizontal_wrapped(|ui| {
            for (index, kind) in ElementKind::all(doc.medium).into_iter().enumerate() {
                let active = kind == selected_kind;
                let btn = egui::Button::new(
                    RichText::new(kind.short())
                        .font(ui_small())
                        .color(if active { Color32::WHITE } else { PAPER_MUTED }),
                )
                .fill(if active { ACCENT } else { PAPER_EDGE })
                .corner_radius(CornerRadius::same(10))
                .min_size(Vec2::new(0.0, 19.0));
                if ui
                    .add(btn)
                    .on_hover_text(format!("{} · Ctrl+{}", kind.label(), index + 1))
                    .clicked()
                {
                    if let Some(block) = blocks.get_mut(app.selected_block) {
                        block.kind = kind;
                        dirty = true;
                    }
                }
            }
            ui.add_space(6.0);
            let add = egui::Button::new(
                RichText::new("+ block").font(ui_small()).color(PAPER_MUTED),
            )
            .frame(false);
            if ui.add(add).clicked() {
                blocks.push(portmanteau_core::Block::new(
                    doc.medium.default_element(),
                    "",
                ));
                app.selected_block = blocks.len().saturating_sub(1);
                app.block_focus = Some(app.selected_block);
                dirty = true;
            }
        });
        ui.add_space(12.0);

        if blocks.is_empty() {
            blocks.push(portmanteau_core::Block::new(
                doc.medium.default_element(),
                "",
            ));
            app.block_focus = Some(0);
        }

        let first_scene_number = 1usize;
        let mut scene_no = first_scene_number;
        let content_width = ui.available_width();
        for i in 0..blocks.len() {
            let block_id = Id::unique(("script_block", doc_id, i));
            let kind = blocks[i].kind;
            if app.block_focus == Some(i) {
                ui.memory_mut(|m| m.request_focus(block_id));
            }
            // Keyboard: Enter starts a new element, Tab cycles its type,
            // Ctrl/Cmd+1…8 selects a type directly. Arrow keys stay bound to
            // the text caret.
            let focused = ui.memory(|m| m.has_focus(block_id));
            if focused {
                // egui resolved this pass's focus direction at `begin_pass`
                // from the event filter stored with the focused widget. A block
                // that has only just been given focus has no filter yet:
                // `set_focus_lock_filter` only takes effect once the widget has
                // held focus for a whole frame, so an unmodified arrow (or Tab)
                // on the first focused frames was already recorded as "move to
                // the next widget". The block owns those keys — the TextEdit
                // moves the caret and the type cycle below claims Tab — so drop
                // the pending directional move and keep focus here.
                ui.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                // A Character cue hands Enter to its CompletionPopup (see
                // below), which consumes the key before the TextEdit sees it.
                // Reading it here would steal the accept key, so a Character
                // block checks the leftover Enter after the popup has run.
                if kind != ElementKind::Character {
                    let enter = ui.input_mut(|inp| {
                        inp.consume_key(Modifiers::NONE, Key::Enter)
                    });
                    if enter {
                        add_at = Some(i);
                    }
                }
                // Shift+Tab has to be matched before plain Tab: `consume_key`
                // ignores extra Shift/Alt modifiers, so a `NONE` pattern would
                // otherwise swallow Shift+Tab and cycle forwards.
                let shift_tab = ui.input_mut(|inp| inp.consume_key(Modifiers::SHIFT, Key::Tab));
                let tab = ui.input_mut(|inp| inp.consume_key(Modifiers::NONE, Key::Tab));
                if tab || shift_tab {
                    cycle_at = Some((i, tab));
                }
                if let Some(kind) = ctrl_digit_element(ui, doc.medium) {
                    blocks[i].kind = kind;
                    app.selected_block = i;
                    dirty = true;
                }
                if ui.input(|inp| inp.key_pressed(Key::Backspace))
                    && blocks[i].text.is_empty()
                    && blocks.len() > 1
                {
                    remove_at = Some(i);
                }
            }

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
                        mono(7.5 * zoom),
                        PAPER_MUTED,
                    );
                }

                let style = script_element_style(&styles, kind);
                let indent = style.indent.max(0.0).min(content_width * 0.42);
                if indent > 0.0 {
                    ui.add_space(indent);
                }
                let width = (content_width - indent - 46.0).max(80.0);
                let family = family_for_font(&style.font);
                let font_size = style.size.max(6.0) * zoom;
                let rows = ((blocks[i].text.chars().count() as f32 * font_size * 0.62)
                    / width.max(80.0))
                .ceil()
                .clamp(1.0, 8.0) as usize;
                let mut text_color = if kind == ElementKind::SceneHeading {
                    PAPER_TEXT
                } else {
                    PAPER_TEXT
                };
                // Uppercase elements (scene headings, transitions, action cues)
                // get the darker ink; the layouter must use the same colour.
                if kind.is_uppercase() && kind != ElementKind::Character {
                    text_color = PAPER_TEXT;
                }
                let base_format = egui::TextFormat {
                    font_id: FontId::new(font_size, family.clone()),
                    color: text_color,
                    ..Default::default()
                };
                // One dictionary pass per block per frame, shared by the
                // layouter's measuring and drawing passes.
                let spans = if spell_on {
                    checker.misspelled_spans(&blocks[i].text)
                } else {
                    Vec::new()
                };
                let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                    let display_text =
                        transform_for_display(text.as_str(), style.uppercase || kind.is_uppercase());
                    let display_spans = if style.uppercase || kind.is_uppercase() {
                        Vec::new()
                    } else {
                        spans.clone()
                    };
                    let mut job = spell_layout_job(&display_text, &display_spans, &base_format);
                    job.wrap.max_width = wrap_width;
                    ui.ctx().fonts_mut(|f| f.layout_job(job))
                };
                let before = blocks[i].text.clone();
                let character_hint = if kind == ElementKind::Character {
                    next_speaker(&blocks, i)
                } else {
                    None
                };
                let mut edit = TextEdit::multiline(&mut blocks[i].text)
                    .font(FontId::new(font_size, family.clone()))
                    .layouter(&mut layouter)
                    .text_color(text_color)
                    .frame(Frame::NONE)
                    .desired_width(width)
                    .desired_rows(rows)
                    .margin(Margin::symmetric(2, 1))
                    .hint_text(
                        RichText::new(kind.label())
                            .font(FontId::new(font_size, family))
                            .color(PAPER_MUTED),
                    );
                if kind == ElementKind::Transition {
                    edit = edit.align(egui::Align2::RIGHT_TOP);
                }
                let mut accepted_dirty = false;
                let mut new_character_name: Option<String> = None;
                let output: egui::text_edit::TextEditOutput =
                    if kind == ElementKind::Character {
                        // The popup owns the TextEdit's id and its keyboard
                        // handling: unmodified Up/Down move the highlighted
                        // suggestion, Enter accepts, and Escape dismisses.
                        // Shift/Ctrl+arrows keep reaching the TextEdit, so
                        // word-processor selection and word jumps still work.
                        let mut typed_word = String::new();
                        let completion = egui::CompletionPopup::new(block_id)
                            // A cue is one line; newlines do not occur inside it,
                            // so the whole line is the word the popup filters on.
                            .word_boundary(|c| c == '\n')
                            // Tab is the element-type cycle, so only Enter accepts.
                            .accept_keys(vec![egui::Key::Enter])
                            .align(egui::RectAlign::BOTTOM_START)
                            .max_height(240.0)
                            .show(ui, edit, |query| {
                                typed_word = query.word.to_string();
                                let word = query.word.trim();
                                if word.is_empty() {
                                    // An empty cue only offers the seesaw's
                                    // next speaker; with no such speaker there
                                    // is nothing to suggest and the popup stays
                                    // closed, so Enter starts a new element.
                                    return character_hint
                                        .as_deref()
                                        .map(|speaker| {
                                            vec![egui::Suggestion::new(speaker)
                                                .content(format!("↺ {speaker}"))]
                                        })
                                        .unwrap_or_default();
                                }
                                let needle = word.to_lowercase();
                                // A complete, existing name means the writer is
                                // done with this cue: close the popup so Enter
                                // advances to the next element like every other
                                // kind instead of re-accepting the same name.
                                if character_names
                                    .iter()
                                    .any(|name| name.to_lowercase() == needle)
                                {
                                    return Vec::new();
                                }
                                let matches: Vec<egui::Suggestion> = character_names
                                    .iter()
                                    .filter(|name| name.to_lowercase().contains(&needle))
                                    .map(|name| egui::Suggestion::new(name.clone()))
                                    .collect();
                                if matches.is_empty() {
                                    // Nothing matches, so the only useful row
                                    // is the one that creates a new sheet.
                                    vec![egui::Suggestion::new(NEW_CHARACTER_SENTINEL)
                                        .content("+ New character…")]
                                } else {
                                    matches
                                }
                            });
                        if let Some(accepted) = &completion.accepted {
                            if accepted.insert == NEW_CHARACTER_SENTINEL {
                                // The popup has already replaced the cue with the
                                // sentinel; undo that and open the modal instead.
                                new_character_name = Some(typed_word.clone());
                            } else {
                                // The popup inserted the name itself, but the
                                // TextEdit did not process the key, so mark the
                                // document dirty here.
                                accepted_dirty = true;
                            }
                        }
                        completion.text_edit
                    } else {
                        edit.id(block_id).show(ui)
                    };
                if let Some(name) = new_character_name {
                    blocks[i].text = before;
                    new_character = Some(NewCharacterState {
                        doc_id: doc_id.to_string(),
                        block: i,
                        name,
                        role: String::new(),
                    });
                } else if accepted_dirty {
                    doc.touch();
                    dirty = true;
                }
                if kind == ElementKind::Character
                    && focused
                    && ui.input_mut(|inp| inp.consume_key(Modifiers::NONE, Key::Enter))
                {
                    // The popup was not open (a freshly focused block, or one
                    // dismissed with Escape), so Enter still starts a new
                    // element as it does on every other kind.
                    add_at = Some(i);
                }
                if spell_on {
                    let key = Id::unique(("spell_menu", doc_id, i));
                    let byte = spell_menu_byte(
                        ui.ctx(),
                        key,
                        &output.response,
                        &output.galley,
                        output.galley_pos,
                        blocks[i].text.as_str(),
                    );
                    let mut choice = None;
                    output.response.context_menu(|ui| {
                        choice = spell_menu_ui(ui, checker, &blocks[i].text, byte);
                    });
                    if let Some(choice) = choice {
                        spell_actions.push(match choice {
                            SpellChoice::Replace(range, replacement) => SpellAction::Replace {
                                doc_id: doc_id.to_string(),
                                block: Some(i),
                                range,
                                replacement,
                            },
                            SpellChoice::Ignore(word) => SpellAction::Ignore(word),
                            SpellChoice::Add(word) => SpellAction::Add(word),
                        });
                    }
                }
                let resp = &output.response;
                // Tab is screenplay input, not focus navigation. egui reads the
                // focused widget's `EventFilter` at the start of every pass, so
                // the lock must be installed after the TextEdit has drawn: the
                // widget installs its own filter (with `tab: false`) while it
                // draws and would otherwise overwrite this one.
                if resp.has_focus() {
                    ui.memory_mut(|mem| {
                        mem.set_focus_lock_filter(
                            block_id,
                            egui::EventFilter {
                                // Keep Tab/Shift+Tab for the type cycle.
                                tab: true,
                                // Caret movement inside the multiline widget.
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                escape: false,
                            },
                        )
                    });
                }
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
                RichText::new("+ Add element").font(ui_small()).color(PAPER_MUTED),
            )
            .frame(false);
            if ui.add(add).clicked() {
                let last_kind = blocks.last().map(|b| b.kind).unwrap_or(ElementKind::Action);
                blocks.push(portmanteau_core::Block::new(next_kind(last_kind, doc.medium), ""));
                app.selected_block = blocks.len() - 1;
                app.block_focus = Some(app.selected_block);
                dirty = true;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} pages", doc.pages()))
                        .font(ui_small())
                        .color(PAPER_MUTED),
                );
            });
        });
        doc.content = portmanteau_core::RichDocument::from_script_blocks(&blocks);
    }

    if let Some((i, forward)) = cycle_at {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            let mut blocks = doc.content.to_script_blocks(&style_sheet);
            if let Some(block) = blocks.get_mut(i) {
                block.kind = if forward {
                    block.kind.next(doc.medium)
                } else {
                    block.kind.prev(doc.medium)
                };
                doc.content = portmanteau_core::RichDocument::from_script_blocks(&blocks);
                app.selected_block = i;
                app.ws.dirty = true;
            }
        }
    }
    if let Some(i) = add_at {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            let mut blocks = doc.content.to_script_blocks(&style_sheet);
            if let Some(block) = blocks.get_mut(i) {
                block.text = block.text.trim_end_matches('\n').to_string();
                let next = next_kind(block.kind, doc.medium);
                blocks.insert(i + 1, portmanteau_core::Block::new(next, ""));
            }
            doc.content = portmanteau_core::RichDocument::from_script_blocks(&blocks);
            app.selected_block = i + 1;
            app.block_focus = Some(i + 1);
            app.ws.dirty = true;
        }
    }
    if let Some(i) = remove_at {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            let mut blocks = doc.content.to_script_blocks(&style_sheet);
            if blocks.len() > 1 {
                blocks.remove(i);
                doc.content = portmanteau_core::RichDocument::from_script_blocks(&blocks);
                let next = i.saturating_sub(1);
                app.selected_block = next;
                app.block_focus = Some(next);
                app.ws.dirty = true;
            }
        }
    }
    if let Some(state) = new_character {
        app.new_character = Some(state);
    }
    apply_spell_actions(app, spell_actions);
    if dirty {
        app.ws.dirty = true;
    }
}

/// Apply an all-caps paragraph style for display without changing the
/// canonical text. Avoid Unicode expansions that would desynchronise egui's
/// character-index caret positions from the stored buffer.
fn transform_for_display(text: &str, uppercase: bool) -> String {
    if !uppercase {
        return text.to_owned();
    }
    text.chars()
        .map(|ch| {
            let mut upper = ch.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(first), None) => first,
                _ => ch,
            }
        })
        .collect()
}

/// Consume Ctrl/Cmd+1…8 while an element has focus and map it onto the
/// medium's element list. Returning the kind keeps the borrow inside the block
/// loop, where `doc` is already mutably borrowed.
fn ctrl_digit_element(ui: &mut egui::Ui, medium: Medium) -> Option<ElementKind> {
    let all = ElementKind::all(medium);
    let keys = [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
    ];
    let ctrl_or_cmd = Modifiers::CTRL | Modifiers::COMMAND;
    for (index, key) in keys.into_iter().enumerate() {
        let Some(kind) = all.get(index).copied() else {
            break;
        };
        let hit = ui.input_mut(|input| {
            input.consume_key(ctrl_or_cmd, key) || input.consume_key(Modifiers::COMMAND, key)
        });
        if hit {
            return Some(kind);
        }
    }
    None
}

/// The `insert` text behind the popup's "New character" row. U+0001 cannot be
/// typed into a cue, so an accepted suggestion carrying it is never a name.
const NEW_CHARACTER_SENTINEL: &str = "\u{1}new-character";

/// In a back-and-forth, the next speaker is the other half of the last two
/// distinct Character names before `index`. Repeated or one-sided names give no
/// suggestion.
fn next_speaker(blocks: &[portmanteau_core::Block], index: usize) -> Option<String> {
    let mut recent: Vec<String> = Vec::new();
    for block in blocks[..index.min(blocks.len())].iter().rev() {
        if block.kind != ElementKind::Character {
            continue;
        }
        let name = block.text.trim();
        if name.is_empty() || recent.iter().any(|seen| seen.eq_ignore_ascii_case(name)) {
            continue;
        }
        recent.push(name.to_string());
        if recent.len() == 2 {
            break;
        }
    }
    if recent.len() < 2 {
        None
    } else {
        Some(recent[1].clone())
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
                .font(ui_small())
                .color(PAPER_MUTED)
                .extra_letter_spacing(1.4),
        );
        ui.label(RichText::new("·").font(ui_small()).color(PAPER_MUTED));
        ui.label(
            RichText::new(medium.label().to_uppercase())
                .font(ui_small())
                .color(PAPER_MUTED)
                .extra_letter_spacing(1.0),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{status} · {words} words · {pages} pp"))
                    .font(ui_small())
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

/// Page the open document with the active format's settings. The paging
/// primitives live in [`crate::ui::page`], shared with the Compile preview.
fn build_page_plan(app: &PortmanteauApp, doc_id: &str) -> Option<PagePlan> {
    let doc = app.ws.project.document(doc_id)?;
    let settings = app.ws.project.settings_for(app.ws.project.export.kind);
    Some(if doc.medium.is_script() {
        script_plan(doc, &settings)
    } else {
        prose_plan(&app.ws.project, doc, &settings)
    })
}

fn empty_state(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(120.0);
        ui.label(RichText::new("Nothing open").font(serif(30.0)).color(MUTED));
        ui.add_space(8.0);
        ui.label(
            RichText::new("Choose a document in the binder, or start a new chapter.")
                .font(ui_body())
                .color(FAINT),
        );
        ui.add_space(16.0);
        let btn = egui::Button::new(
            RichText::new("New chapter").font(ui_body()).color(Color32::WHITE),
        )
        .fill(ACCENT)
        .corner_radius(CornerRadius::same(6))
        .min_size(Vec2::new(120.0, 32.0));
        if ui.add(btn).clicked() {
            app.new_chapter_public();
        }
    });
}

/// Headless interaction tests for the screenplay surface.
///
/// They drive the real [`show`] function through `egui::Context::run`, so no
/// window, GPU or display is involved: `cargo test` runs them like any other
/// unit test. Frames are fed by hand with synthetic [`egui::RawInput`], which
/// is exactly what an eframe integration would deliver.
#[cfg(test)]
mod ui_tests {
    use super::*;
    use crate::app::{PortmanteauApp, View};
    use crate::ui::editor;
    use eframe::egui::accesskit::Role;
    use eframe::egui::{self, Event, Key, Modifiers, Pos2, RawInput, Rect, Vec2};
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use std::cell::RefCell;
    use std::rc::Rc;

    const SCREEN: Vec2 = Vec2::new(1400.0, 1000.0);

    /// A demo project opened on its first script document, with block 0
    /// selected and queued for focus. Spell-checking is switched off so the
    /// fixture does not depend on the writer's saved settings or on walking a
    /// dictionary for every frame.
    fn script_fixture() -> (PortmanteauApp, String) {
        let mut app = PortmanteauApp::new(None);
        app.settings.spellcheck = false;
        app.show_spelling = false;
        app.view = View::Write;
        app.write_mode = WriteMode::Continuous;
        let doc_id = app
            .ws
            .project
            .documents
            .iter()
            .find(|d| d.medium.is_script())
            .map(|d| d.id.clone())
            .expect("the demonstration project has a script document");
        app.selected = Some(doc_id.clone());
        app.selected_block = 0;
        app.block_focus = Some(0);
        (app, doc_id)
    }

    fn headless_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        // The editor uses the bundled families ("portmanteau-serif" and
        // friends), which have to be registered before the first frame.
        crate::theme::install(&ctx);
        ctx
    }

    fn frame(app: &mut PortmanteauApp, ctx: &egui::Context, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
            events,
            ..Default::default()
        };
        let output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                editor::show(app, ui);
            });
        });
        // No renderer applies the font atlas here; clear the deltas so egui
        // does not panic on drop.
        output.drop_without_applying_deltas();
    }

    /// Run enough empty frames for `request_focus` to land and for the
    /// TextEdit to install its own event filter before ours is put back on top
    /// of it, so the focus lock is active when a key is injected.
    fn settle(app: &mut PortmanteauApp, ctx: &egui::Context) {
        for _ in 0..4 {
            frame(app, ctx, Vec::new());
        }
    }

    fn key(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn block_id(doc_id: &str, index: usize) -> Id {
        let _ = index;
        Id::unique(("script_flow", doc_id))
    }

    fn block_kind(app: &PortmanteauApp, doc_id: &str, index: usize) -> ElementKind {
        let doc = app.ws
            .project
            .document(doc_id)
            .expect("document still exists");
        let sheet = app
            .ws
            .project
            .style_sheets
            .iter()
            .find(|sheet| sheet.id == doc.style_sheet_id)
            .expect("document stylesheet exists");
        doc.content.to_script_blocks(sheet)[index].kind
    }

    fn block_text(app: &PortmanteauApp, doc_id: &str, index: usize) -> String {
        let doc = app.ws
            .project
            .document(doc_id)
            .expect("document still exists");
        let sheet = app
            .ws
            .project
            .style_sheets
            .iter()
            .find(|sheet| sheet.id == doc.style_sheet_id)
            .expect("document stylesheet exists");
        doc.content.to_script_blocks(sheet)[index].text.clone()
    }

    fn set_script_block(
        doc: &mut portmanteau_core::Document,
        index: usize,
        kind: ElementKind,
        text: &str,
    ) {
        assert_eq!(index, 0, "single-paragraph fixtures use index zero");
        doc.set_script_blocks(vec![portmanteau_core::Block::new(kind, text)]);
    }

    #[test]
    fn tab_advances_and_shift_tab_reverses_the_element_type() {
        let (mut app, doc_id) = script_fixture();
        let ctx = headless_ctx();
        settle(&mut app, &ctx);
        assert!(
            ctx.memory(|m| m.has_focus(block_id(&doc_id, 0))),
            "block 0 should hold keyboard focus before keys are injected"
        );

        let medium = app
            .ws
            .project
            .document(&doc_id)
            .expect("document still exists")
            .medium;
        let before = block_kind(&app, &doc_id, 0);
        let forward = before.next(medium);
        assert_ne!(before, forward, "the demo medium has more than one kind");

        // Tab must cycle the type (and be kept from egui's focus navigation by
        // the focus-lock filter).
        frame(&mut app, &ctx, vec![key(Key::Tab, Modifiers::NONE)]);
        assert_eq!(
            block_kind(&app, &doc_id, 0),
            forward,
            "Tab should advance the element type"
        );
        assert!(
            ctx.memory(|m| m.has_focus(block_id(&doc_id, 0))),
            "Tab should not move focus to another widget"
        );

        // Shift+Tab must walk the same list backwards, not advance again.
        frame(&mut app, &ctx, vec![key(Key::Tab, Modifiers::SHIFT)]);
        assert_eq!(
            block_kind(&app, &doc_id, 0),
            before,
            "Shift+Tab should return to the previous element type"
        );
        assert!(
            ctx.memory(|m| m.has_focus(block_id(&doc_id, 0))),
            "Shift+Tab should keep focus on the block"
        );
    }

    #[test]
    fn character_style_displays_uppercase_without_mutating_canonical_text() {
        let (mut app, doc_id) = script_fixture();
        let ctx = headless_ctx();
        if let Some(doc) = app.ws.project.document_mut(&doc_id) {
            set_script_block(doc, 0, ElementKind::Character, "");
        }
        settle(&mut app, &ctx);
        assert!(ctx.memory(|m| m.has_focus(block_id(&doc_id, 0))));

        frame(&mut app, &ctx, vec![Event::Text("mara".into())]);
        assert_eq!(
            block_text(&app, &doc_id, 0),
            "mara",
            "the canonical character name should preserve its proper case"
        );
        assert_eq!(transform_for_display("mara", true), "MARA");
    }

    #[test]
    fn typing_into_a_character_cue_opens_the_character_popup() {
        let mut fixture = click_harness();
        assert_eq!(fixture.text(), "");

        fixture
            .harness
            .input_mut()
            .events
            .push(Event::Text("ma".into()));
        fixture.harness.run_steps(2);

        assert_eq!(fixture.text(), "ma", "the canonical cue preserves typed case");
        assert!(
            fixture
                .harness
                .query_by_role_and_label(Role::Button, "Mara Venn")
                .is_some(),
            "typing a character cue should open the completion popup with matching names"
        );
    }

    #[test]
    fn popup_arrow_down_then_enter_inserts_the_second_suggestion() {
        let mut fixture = click_harness();
        // An empty cue has nothing to suggest in this fixture (no preceding
        // speaker), so filter first. "r" keeps three demo names, ordered
        // Elias Rook, Mara Venn, Ruth Alder.
        fixture.type_word("r");
        assert!(
            fixture
                .harness
                .query_by_role_and_label(Role::Button, "Elias Rook")
                .is_some(),
            "the popup should start on the first suggestion"
        );

        fixture.harness.key_press(Key::ArrowDown);
        fixture.harness.run_steps(2);
        assert_eq!(
            fixture.text(),
            "r",
            "ArrowDown while the popup is open selects a row, it does not edit the text"
        );

        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(2);
        assert_eq!(
            fixture.text(),
            "Mara Venn",
            "ArrowDown moved the selection to the second suggestion, so Enter inserted it"
        );
    }

    #[test]
    fn popup_escape_closes_it_and_leaves_the_text_alone() {
        let mut fixture = click_harness();
        fixture.type_word("ma");
        assert!(
            fixture
                .harness
                .query_by_role_and_label(Role::Button, "Mara Venn")
                .is_some(),
            "the popup should be open before Escape"
        );

        fixture.harness.key_press(Key::Escape);
        fixture.harness.run_steps(2);

        assert_eq!(fixture.text(), "ma", "Escape must not change the cue");
        assert!(
            fixture
                .harness
                .query_by_role_and_label(Role::Button, "Mara Venn")
                .is_none(),
            "Escape should close the popup (no suggestion buttons in the tree)"
        );
    }

    #[test]
    fn popup_enter_on_a_filtered_single_name_inserts_it() {
        let mut fixture = click_harness();
        fixture
            .harness
            .input_mut()
            .events
            .push(Event::Text("ma".into()));
        fixture.harness.run_steps(2);
        assert_eq!(fixture.text(), "ma");

        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(2);
        assert_eq!(
            fixture.text(),
            "Mara Venn",
            "Enter accepts the only matching name"
        );
    }

    /// A complete, existing cue is finished: the popup must close so Enter
    /// keeps its script meaning (start the next element) instead of accepting
    /// a suggestion and rewriting the line.
    #[test]
    fn typing_a_full_character_name_closes_the_popup_and_enter_adds_an_element() {
        let mut fixture = click_harness();
        let blocks_before = fixture
            .app
            .borrow()
            .ws
            .project
            .document(&fixture.doc_id)
            .expect("document still exists")
            .content
            .paragraphs
            .len();

        fixture.type_word("Mara Venn");
        assert_eq!(fixture.text(), "Mara Venn", "the cue preserves canonical case");
        assert!(
            fixture
                .harness
                .query_by_role_and_label(Role::Button, "Mara Venn")
                .is_none(),
            "a complete name means the popup has nothing left to suggest"
        );
        assert!(
            fixture
                .harness
                .query_by_role_and_label(Role::Button, "+ New character…")
                .is_none(),
            "a complete name must not fall back to the new-character row"
        );

        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(2);

        assert!(
            fixture.app.borrow().new_character.is_none(),
            "Enter on a finished cue must not open the new-character modal"
        );
        assert_eq!(
            fixture.text(),
            "Mara Venn",
            "Enter must not replace the typed name with another casing"
        );
        let blocks_after = fixture
            .app
            .borrow()
            .ws
            .project
            .document(&fixture.doc_id)
            .expect("document still exists")
            .content
            .paragraphs
            .len();
        assert_eq!(
            blocks_after,
            blocks_before + 1,
            "Enter should add the next element like it does on every other kind"
        );
        assert_eq!(
            fixture.app.borrow().selected_block, 1,
            "the new element becomes the selected block"
        );
    }

    /// An unknown name offers only the new-character row; Enter opens the
    /// modal and leaves the typed cue untouched.
    #[test]
    fn typing_an_unknown_name_offers_only_new_character_and_enter_opens_the_modal() {
        use egui_kittest::kittest::NodeT;

        let mut fixture = click_harness();
        fixture.type_word("Zorbo");
        fixture.harness.run_steps(2);
        assert_eq!(fixture.text(), "Zorbo", "the cue preserves typed case");

        let labels: Vec<String> = fixture
            .harness
            .get_all_by_role(Role::Button)
            .filter_map(|node| node.accesskit_node().label())
            .collect();
        assert!(
            labels.iter().any(|label| label == "+ New character…"),
            "an unknown name should offer the new-character row; saw {labels:?}"
        );
        for name in ["Mara Venn", "Elias Rook", "Ruth Alder", "The Boy"] {
            assert!(
                !labels.iter().any(|label| label == name),
                "an unknown name must not list {name}; saw {labels:?}"
            );
        }

        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(2);

        let app = fixture.app.borrow();
        let state = app
            .new_character
            .as_ref()
            .expect("Enter on the new-character row should open the modal");
        assert_eq!(state.doc_id, fixture.doc_id);
        assert_eq!(state.block, 0);
        assert_eq!(
            fixture.text(),
            "Zorbo",
            "the sentinel must be undone before the modal opens"
        );
    }

    #[test]
    fn next_speaker_suggests_the_other_half_of_a_seesaw() {
        use portmanteau_core::Block;

        let seesaw = vec![
            Block::new(ElementKind::Character, "MARA"),
            Block::new(
                ElementKind::Dialogue,
                "The chart says there's a channel here.",
            ),
            Block::new(ElementKind::Character, "BOY"),
            Block::new(ElementKind::Dialogue, "The chart's wrong."),
        ];
        assert_eq!(
            next_speaker(&seesaw, 1),
            None,
            "one name so far is not a pair"
        );
        assert_eq!(
            next_speaker(&seesaw, 3).as_deref(),
            Some("MARA"),
            "after BOY the next speaker is MARA"
        );
        assert_eq!(
            next_speaker(&seesaw, 4).as_deref(),
            Some("MARA"),
            "the most recent distinct name is BOY, so MARA is due"
        );

        // A repeated name is not a second voice, so there is no clear pair.
        let repeated = vec![
            Block::new(ElementKind::Character, "MARA"),
            Block::new(ElementKind::Dialogue, "One."),
            Block::new(ElementKind::Character, "mara"),
            Block::new(ElementKind::Dialogue, "Two."),
        ];
        assert_eq!(next_speaker(&repeated, 4), None);

        // One-sided dialogue (and empty cues) give no suggestion either.
        let solo = vec![
            Block::new(ElementKind::Character, "MARA"),
            Block::new(ElementKind::Dialogue, "Alone."),
            Block::new(ElementKind::Character, "   "),
        ];
        assert_eq!(next_speaker(&solo, 3), None);
    }

    // ---- Click tests (egui_kittest / AccessKit) -------------------------

    /// Like `script_fixture`, but block 0 is an empty Character cue so typing
    /// opens the autocomplete. The app lives behind `Rc<RefCell<_>>` because
    /// `Harness` keeps the drawing closure and the test reads state between
    /// frames.
    fn click_fixture() -> (PortmanteauApp, String) {
        let (mut app, doc_id) = script_fixture();
        if let Some(doc) = app.ws.project.document_mut(&doc_id) {
            set_script_block(doc, 0, ElementKind::Character, "");
        }
        app.selected_block = 0;
        app.block_focus = Some(0);
        (app, doc_id)
    }

    struct ClickHarness {
        harness: Harness<'static>,
        app: Rc<RefCell<PortmanteauApp>>,
        doc_id: String,
    }

    fn click_harness() -> ClickHarness {
        let (app, doc_id) = click_fixture();
        let app = Rc::new(RefCell::new(app));
        let ui_app = Rc::clone(&app);
        // The editor uses the bundled families ("portmanteau-serif" and
        // friends), which have to be registered before the first layout.
        // `set_fonts` only takes effect at the start of the next pass, so the
        // very first harness frame installs fonts and draws nothing.
        let mut themed = false;
        let mut ready = false;
        let mut harness = Harness::builder()
            .with_size(SCREEN)
            .with_max_steps(64)
            .with_accessibility_check(false)
            .build_ui(move |ui| {
                if !themed {
                    crate::theme::install(ui.ctx());
                    themed = true;
                }
                if !ready {
                    ready = true;
                    return;
                }
                egui::CentralPanel::default().show(ui, |ui| {
                    editor::show(&mut ui_app.borrow_mut(), ui);
                });
            });
        // The first frame requests focus for block 0; a few more settle the
        // TextEdit's focus and event filter before a test types or clicks.
        harness.run_steps(4);
        ClickHarness {
            harness,
            app,
            doc_id,
        }
    }

    impl ClickHarness {
        fn text(&self) -> String {
            block_text(&self.app.borrow(), &self.doc_id, 0)
        }

        fn kind(&self) -> ElementKind {
            block_kind(&self.app.borrow(), &self.doc_id, 0)
        }

        /// Type `word` into the focused cue and let one frame settle so the
        /// popup has rebuilt its suggestions for the new word.
        fn type_word(&mut self, word: &str) {
            self.harness
                .input_mut()
                .events
                .push(Event::Text(word.into()));
            self.harness.run();
        }

        fn open_character_popup(&mut self) {
            self.type_word("ma");
            assert!(
                self.harness
                    .query_by_role_and_label(Role::Button, "Mara Venn")
                    .is_some(),
                "typing should open the character completion popup"
            );
        }
    }

    fn paragraph_summary(fixture: &ClickHarness) -> Vec<(String, String)> {
        let app = fixture.app.borrow();
        let doc = app.ws.project.document(&fixture.doc_id).unwrap();
        doc.content
            .paragraphs
            .iter()
            .map(|p| (p.style_id.clone(), p.editor_text()))
            .collect()
    }

    /// The whole writing rhythm: choose a name from the popup with Enter, press
    /// Enter again for dialogue, type a line, Enter for the next element.
    #[test]
    fn character_cue_flow_accept_name_then_dialogue_then_next_element() {
        let mut fixture = click_harness();
        fixture.open_character_popup();

        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(3);
        assert_eq!(
            paragraph_summary(&fixture),
            vec![("character".to_owned(), "Mara Venn".to_owned())],
            "Enter on a suggestion fills the name and does not add a paragraph"
        );

        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(3);
        let after_name = paragraph_summary(&fixture);
        assert_eq!(after_name.len(), 2, "second Enter adds a paragraph: {after_name:?}");
        assert_eq!(after_name[1].0, "dialogue");

        fixture.type_word("Hello there.");
        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(3);
        let after_line = paragraph_summary(&fixture);
        assert_eq!(after_line.len(), 3, "{after_line:?}");
        assert_eq!(after_line[1].1, "Hello there.");
        assert_eq!(fixture.app.borrow().selected_block, 2);
    }

    #[test]
    fn clicking_an_autocomplete_name_fills_the_block() {
        let mut fixture = click_harness();
        fixture.open_character_popup();

        fixture
            .harness
            .get_by_role_and_label(Role::Button, "Mara Venn")
            .click();
        fixture.harness.run();

        assert_eq!(
            fixture.text(),
            "Mara Venn",
            "clicking a suggested name should store it in the Character block"
        );
        assert!(
            fixture.app.borrow().new_character.is_none(),
            "choosing a name must not open the new-character modal"
        );
    }

    #[test]
    fn clicking_new_character_opens_the_modal() {
        let mut fixture = click_harness();
        // An unknown name leaves the sentinel as the only suggestion.
        fixture.type_word("Zorbo");
        let before = fixture.text();
        assert_eq!(before, "Zorbo", "the cue preserves typed case");
        assert!(
            fixture
                .harness
                .query_by_role_and_label(Role::Button, "Mara Venn")
                .is_none(),
            "a name that matches no sheet must not offer characters"
        );

        fixture
            .harness
            .get_by_role_and_label(Role::Button, "+ New character…")
            .click();
        fixture.harness.run();

        let app = fixture.app.borrow();
        let state = app
            .new_character
            .as_ref()
            .expect("clicking '+ New character…' should open the modal state");
        assert_eq!(state.doc_id, fixture.doc_id);
        assert_eq!(state.block, 0);
        assert_eq!(
            fixture.text(),
            before,
            "the sentinel the popup inserted must be undone before the modal opens"
        );
    }

    #[test]
    fn clicking_a_palette_element_changes_the_block_type() {
        let mut fixture = click_harness();
        assert_eq!(fixture.kind(), ElementKind::Character);

        fixture.harness.run();
        fixture
            .harness
            .get_all_by_role(Role::ComboBox)
            .next()
            .expect("the style dropdown should show the current style")
            .click();
        fixture.harness.run();
        fixture
            .harness
            .get_by_label("Dialogue")
            .click();
        fixture.harness.run();

        assert_eq!(
            fixture.kind(),
            ElementKind::Dialogue,
            "choosing a style from the toolbar dropdown should retype the selected block"
        );
        assert_eq!(fixture.app.borrow().selected_block, 0);
    }

    #[test]
    fn layouter_underlines_misspelled_words_and_not_correct_ones() {
        let checker = crate::spellcheck::load("en_US");
        let text = "the qik brown foxx";
        let spans = checker.misspelled_spans(text);
        assert_eq!(spans, vec![(4, 7), (14, 18)]);
        let job = spell_layout_job(text, &spans, &egui::TextFormat::default());
        assert_eq!(job.text, text, "the layouter must preserve the text");
        let underlined: Vec<&str> = job
            .sections
            .iter()
            .filter(|section| section.format.underline != Stroke::NONE)
            .map(|section| &text[section.byte_range.start.0..section.byte_range.end.0])
            .collect();
        assert_eq!(
            underlined,
            vec!["qik", "foxx"],
            "only misspelled words get the squiggle"
        );

        let clean = "the brown fox jumps";
        let spans = checker.misspelled_spans(clean);
        assert!(spans.is_empty());
        let job = spell_layout_job(clean, &spans, &egui::TextFormat::default());
        assert!(
            job.sections
                .iter()
                .all(|section| section.format.underline == Stroke::NONE),
            "correct prose must produce no underline sections"
        );

        // Empty text still yields one section, like `single_section`.
        let job = spell_layout_job("", &[], &egui::TextFormat::default());
        assert_eq!(job.sections.len(), 1);
        assert_eq!(job.text, "");
    }

    #[test]
    fn spell_menu_draws_without_a_click() {
        let checker = crate::spellcheck::load("en_US");
        let ctx = headless_ctx();
        let mut seen = Vec::new();
        ctx.run_ui(RawInput::default(), |ui| {
            seen.push(spell_menu_ui(ui, &checker, "helo", 2).is_none());
        })
        .drop_without_applying_deltas();
        // The menu was drawn without a click; no choice is made.
        assert_eq!(seen, vec![true]);
    }

    #[test]
    fn right_click_menu_ignores_a_word_for_the_session() {
        let mut app = PortmanteauApp::new(None);
        app.settings.spellcheck = true;
        app.show_spelling = false;
        app.view = View::Write;
        app.write_mode = WriteMode::Continuous;
        let doc_id = app
            .ws
            .project
            .documents
            .iter()
            .find(|d| d.kind == DocKind::Chapter)
            .map(|d| d.id.clone())
            .expect("the demo has a prose chapter");
        if let Some(doc) = app.ws.project.document_mut(&doc_id) {
            doc.set_plain_text("qik");
        }
        app.selected = Some(doc_id.clone());
        assert!(!app.spell.is_correct("qik"), "the fixture word is misspelled");

        let app = Rc::new(RefCell::new(app));
        let ui_app = Rc::clone(&app);
        let mut themed = false;
        let mut ready = false;
        let mut harness = Harness::builder()
            .with_size(SCREEN)
            .with_max_steps(64)
            .with_accessibility_check(false)
            .build_ui(move |ui| {
                if !themed {
                    crate::theme::install(ui.ctx());
                    themed = true;
                }
                if !ready {
                    ready = true;
                    return;
                }
                egui::CentralPanel::default().show(ui, |ui| {
                    editor::show(&mut ui_app.borrow_mut(), ui);
                });
            });
        harness.run_steps(4);

        // The prose body is the only multiline text input on the page.
        harness
            .get_by_role(Role::MultilineTextInput)
            .click_secondary();
        harness.run();

        harness
            .get_by_role_and_label(Role::Button, "Ignore for this session")
            .click();
        harness.run();

        assert!(
            app.borrow().spell.is_correct("qik"),
            "the right-click menu should ignore the word for this session"
        );
    }

    #[test]
    fn spell_actions_edit_prose_script_and_ignore_a_word() {
        let mut app = PortmanteauApp::new(None);
        let prose_id = app
            .ws
            .project
            .documents
            .iter()
            .find(|d| d.kind == DocKind::Chapter)
            .map(|d| d.id.clone())
            .expect("the demo has a prose chapter");
        let script_id = app
            .ws
            .project
            .documents
            .iter()
            .find(|d| d.medium.is_script())
            .map(|d| d.id.clone())
            .expect("the demo has a script document");

        if let Some(doc) = app.ws.project.document_mut(&prose_id) {
            doc.set_plain_text("the qik brown foxx");
        }
        if let Some(doc) = app.ws.project.document_mut(&script_id) {
            set_script_block(doc, 0, ElementKind::Action, "the qik brown foxx");
        }
        app.ws.dirty = false;

        apply_spell_actions(
            &mut app,
            vec![SpellAction::Replace {
                doc_id: prose_id.clone(),
                block: None,
                range: 4..7,
                replacement: "quick".into(),
            }],
        );
        assert_eq!(
            app.ws.project.document(&prose_id).unwrap().body_text(),
            "the quick brown foxx",
            "the prose body should take the suggestion"
        );
        assert!(app.ws.dirty, "a replacement marks the workspace dirty");

        apply_spell_actions(
            &mut app,
            vec![SpellAction::Replace {
                doc_id: script_id.clone(),
                block: Some(0),
                range: 4..7,
                replacement: "quick".into(),
            }],
        );
        assert_eq!(
            block_text(&app, &script_id, 0),
            "the quick brown foxx",
            "the screenplay block should take the suggestion"
        );

        // An out-of-range span is ignored rather than panicking or corrupting
        // the text.
        apply_spell_actions(
            &mut app,
            vec![SpellAction::Replace {
                doc_id: prose_id.clone(),
                block: None,
                range: 999..1002,
                replacement: "x".into(),
            }],
        );
        assert_eq!(
            app.ws.project.document(&prose_id).unwrap().body_text(),
            "the quick brown foxx"
        );

        apply_spell_actions(
            &mut app,
            vec![SpellAction::Ignore("zzzzqqq".into())],
        );
        assert!(app.spell.is_correct("zzzzqqq"), "ignored words are accepted");
    }

    /// Capability probes for the automated UI harness.
    ///
    /// These do not guard product behaviour in their own right; they prove
    /// that a headless `egui_kittest` test can simulate the mouse and keyboard
    /// and read widget state back out. When one fails it is evidence that the
    /// harness (or the product) cannot support the script-mode diagnostics we
    /// need, not necessarily that the fixture is wrong.
    #[cfg(test)]
    mod interaction_probe {
        use super::*;
        use egui_kittest::kittest::NodeT;

        /// Build a harness for `draw`, installing the bundled fonts on the
        /// first (discarded) frame and letting focus and layout settle.
        fn probe_harness(
            app: PortmanteauApp,
            draw: fn(&mut PortmanteauApp, &mut egui::Ui),
        ) -> (Harness<'static>, Rc<RefCell<PortmanteauApp>>) {
            let app = Rc::new(RefCell::new(app));
            let ui_app = Rc::clone(&app);
            let mut themed = false;
            let mut ready = false;
            let mut harness = Harness::builder()
                .with_size(SCREEN)
                .with_max_steps(64)
                .with_accessibility_check(false)
                .build_ui(move |ui| {
                    if !themed {
                        crate::theme::install(ui.ctx());
                        themed = true;
                    }
                    if !ready {
                        ready = true;
                        return;
                    }
                    egui::CentralPanel::default().show(ui, |ui| {
                        draw(&mut ui_app.borrow_mut(), ui);
                    });
                });
            harness.run_steps(4);
            (harness, app)
        }

        /// Read the caret's character index from the text edit's stored state.
        fn caret_index(harness: &Harness<'_>, id: Id) -> usize {
            egui::text_edit::TextEditState::load(&harness.ctx, id)
                .and_then(|state| state.cursor.char_range())
                .map(|range| range.primary.index.0)
                .expect("the text edit should have stored a caret")
        }

        /// Read the full selection range (primary and secondary cursors).
        fn caret_range(harness: &Harness<'_>, id: Id) -> egui::text::CCursorRange {
            egui::text_edit::TextEditState::load(&harness.ctx, id)
                .and_then(|state| state.cursor.char_range())
                .expect("the text edit should have stored a caret range")
        }

        #[test]
        fn mouse_opens_reads_and_selects_the_spelling_language_combo() {
            let mut app = PortmanteauApp::new(None);
            app.settings.spellcheck = false;
            // `new` reads the writer's saved defaults; pin the fixture so the
            // test does not depend on the machine it runs on.
            app.settings.language = "en_US".into();
            app.view = View::Settings;
            app.settings_tab = 0;
            let (mut harness, app) = probe_harness(app, crate::ui::views::show);

            // Open the ComboBox with a simulated pointer click.
            harness.get_by_role(Role::ComboBox).click();
            harness.run_steps(2);

            // Read every option label out of the open popup. This is the
            // capability the script-mode diagnostics need: the menu contents
            // must be visible in the accessibility tree, not just painted.
            let labels: Vec<String> = harness
                .get_all_by_role(Role::Button)
                .filter_map(|node| node.accesskit_node().label())
                .collect();
            assert!(
                labels.iter().any(|label| label == "English (US)"),
                "the open dropdown should expose English (US); saw {labels:?}"
            );
            assert!(
                labels.iter().any(|label| label == "English (UK)"),
                "the open dropdown should expose English (UK); saw {labels:?}"
            );

            // Select an item and read the resulting application state.
            harness
                .get_by_role_and_label(Role::Button, "English (UK)")
                .click();
            harness.run_steps(2);
            assert_eq!(
                app.borrow().settings.language,
                "en_GB",
                "clicking English (UK) should update the stored language"
            );
        }

        #[test]
        fn keyboard_types_into_a_script_block_and_arrow_left_moves_the_caret() {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                set_script_block(doc, 0, ElementKind::Action, "seed");
            }
            app.selected_block = 0;
            app.block_focus = Some(0);
            let (mut harness, app) = probe_harness(app, editor::show);

            let id = block_id(&doc_id, 0);
            // The value query also finds the widget's TextRun child, so pick
            // the editable node by role.
            harness
                .get_all_by_value("seed")
                .find(|node| node.accesskit_node().role() == Role::MultilineTextInput)
                .expect("block 0 should be a multiline input holding the seed text")
                .focus();
            harness.run_steps(2);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "the script block should accept focus from the accessibility tree"
            );

            // Send text through the node API. It appends at the caret, which
            // starts at the end of the focused multiline widget.
            harness
                .get_all_by_value("seed")
                .find(|node| node.accesskit_node().role() == Role::MultilineTextInput)
                .expect("block 0 should still be a multiline input holding the seed text")
                .type_text("hello");
            harness.run_steps(2);
            assert_eq!(
                block_text(&app.borrow(), &doc_id, 0),
                "seedhello",
                "typed text should reach the script block"
            );
            assert_eq!(caret_index(&harness, id), 9, "the caret follows the text");

            // A plain arrow key must move the text caret, not the widget focus.
            harness.key_press(Key::ArrowLeft);
            harness.run_steps(2);
            assert_eq!(
                caret_index(&harness, id),
                8,
                "ArrowLeft should move the caret one character left"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "ArrowLeft must not move focus off the block"
            );
        }

        #[test]
        fn script_character_dropdown_exposes_names_and_click_fills_the_block() {
            let mut fixture = click_harness();
            assert_eq!(fixture.text(), "", "the probe starts from an empty cue");

            // Find the empty multiline block (the fixture's block 0), focus it
            // and type a query that matches both demo characters. "e" matches
            // "Mara Venn" and "Elias Rook".
            fixture
                .harness
                .get_all_by_role(Role::MultilineTextInput)
                .find(|node| node.value().as_deref() == Some(""))
                .expect("the empty Character block should be a multiline input")
                .focus();
            fixture.harness.run_steps(2);
            fixture
                .harness
                .get_all_by_role(Role::MultilineTextInput)
                .find(|node| node.value().as_deref() == Some(""))
                .expect("the empty Character block should still be a multiline input")
                .type_text("e");
            fixture.harness.run_steps(2);

            // Read the accessible popup contents.
            let labels: Vec<String> = fixture
                .harness
                .get_all_by_role(Role::Button)
                .filter_map(|node| node.accesskit_node().label())
                .collect();
            assert!(
                labels.iter().any(|label| label == "Mara Venn"),
                "the character popup should expose Mara Venn; saw {labels:?}"
            );
            assert!(
                labels.iter().any(|label| label == "Elias Rook"),
                "the character popup should expose Elias Rook; saw {labels:?}"
            );
            assert!(
                !labels.iter().any(|label| label == "+ New character…"),
                "a filtered list must not also offer the new-character row; saw {labels:?}"
            );

            fixture
                .harness
                .get_by_role_and_label(Role::Button, "Mara Venn")
                .click();
            fixture.harness.run_steps(2);
            assert_eq!(
                fixture.text(),
                "Mara Venn",
                "clicking a name should fill the Character block"
            );
            assert!(
                fixture.app.borrow().new_character.is_none(),
                "choosing a name must not open the new-character modal"
            );
        }

        /// An empty cue in a back-and-forth offers the other half of the last
        /// two distinct speakers as the single suggestion.
        #[test]
        fn popup_offers_the_next_speaker_for_an_empty_cue() {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                doc.set_script_blocks(vec![
                    portmanteau_core::Block::new(ElementKind::Character, "MARA"),
                    portmanteau_core::Block::new(ElementKind::Dialogue, "One."),
                    portmanteau_core::Block::new(ElementKind::Character, "BOY"),
                    portmanteau_core::Block::new(ElementKind::Dialogue, "Two."),
                    portmanteau_core::Block::new(ElementKind::Character, ""),
                ]);
            }
            app.selected_block = 4;
            app.block_focus = Some(4);
            let (mut harness, app) = real_harness(app, SCREEN, None, 4, editor::show);

            let id = block_id(&doc_id, 4);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "the empty cue should hold focus"
            );
            assert!(
                harness
                    .query_by_role_and_label(Role::Button, "↺ MARA")
                    .is_some(),
                "an empty cue should offer the next speaker as its suggestion"
            );

            harness.key_press(Key::Enter);
            harness.run_steps(2);
            assert_eq!(
                block_text(&app.borrow(), &doc_id, 4),
                "MARA",
                "Enter on the next-speaker suggestion fills the cue"
            );
        }

        #[test]
        fn arrow_down_stays_inside_the_focused_script_block() {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                set_script_block(doc, 0, ElementKind::Action, "one\ntwo");
            }
            app.selected_block = 0;
            app.block_focus = Some(0);
            let (mut harness, app) = probe_harness(app, editor::show);

            let id = block_id(&doc_id, 0);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "block 0 should be focused before the arrow key"
            );
            let selected_before = app.borrow().selected_block;

            // Put the caret at the very start so ArrowDown has a line to enter.
            let mut state =
                egui::text_edit::TextEditState::load(&harness.ctx, id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(0),
            )));
            state.store(&harness.ctx, id);
            harness.run_steps(1);
            assert_eq!(caret_index(&harness, id), 0, "caret starts on line one");

            harness.key_press(Key::ArrowDown);
            harness.run_steps(2);

            // The bug we are probing: arrows must stay in the text, not move
            // the editor's block selection or the keyboard focus.
            assert_eq!(
                app.borrow().selected_block,
                selected_before,
                "ArrowDown must not change the selected block"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "ArrowDown must not move focus to another block"
            );
            assert_eq!(
                caret_index(&harness, id),
                4,
                "ArrowDown should move the caret to the start of the second line"
            );
        }

        // ---- Real-pointer probes ---------------------------------------
        //
        // The probes above use `Node::click()`. In egui_kittest 0.37 that is
        // *already* a real pointer sequence (PointerMoved, then a primary
        // button press, then the release), because `Harness::step` gives every
        // queued event its own frame. `Node::click_accesskit()` is the API that
        // sends `accesskit::Action::Click` and bypasses the pointer. This
        // version of kittest has no `simulate_click()`; the helpers below spell
        // the pointer sequence out event by event and, unlike `Node::click()`,
        // re-read the target's rect between frames so a popup that moves or
        // closes is observed rather than clicked at a stale position.

        /// Real app window: the shell is 1440x930.
        const REAL_SCREEN: Vec2 = Vec2::new(1440.0, 930.0);

        /// A harness that can also narrow the drawing column, mimicking the
        /// real shell where the rail, binder and inspector leave the editor
        /// roughly 800 px of the 1440 px window.
        fn real_harness(
            app: PortmanteauApp,
            size: Vec2,
            editor_width: Option<f32>,
            settle: usize,
            draw: fn(&mut PortmanteauApp, &mut egui::Ui),
        ) -> (Harness<'static>, Rc<RefCell<PortmanteauApp>>) {
            let app = Rc::new(RefCell::new(app));
            let ui_app = Rc::clone(&app);
            let mut themed = false;
            let mut ready = false;
            let mut harness = Harness::builder()
                .with_size(size)
                .with_max_steps(96)
                .with_accessibility_check(false)
                .build_ui(move |ui| {
                    if !themed {
                        crate::theme::install(ui.ctx());
                        themed = true;
                    }
                    if !ready {
                        ready = true;
                        return;
                    }
                    egui::CentralPanel::default().show(ui, |ui| {
                        let Some(width) = editor_width else {
                            draw(&mut ui_app.borrow_mut(), ui);
                            return;
                        };
                        // Centre the editor column, leaving the rest for the
                        // shell's rail/binder/inspector.
                        ui.vertical_centered(|ui| {
                            ui.set_max_width(width);
                            ui.allocate_ui_with_layout(
                                Vec2::new(width, ui.available_height()),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    draw(&mut ui_app.borrow_mut(), ui);
                                },
                            );
                        });
                    });
                });
            harness.run_steps(settle);
            (harness, app)
        }

        fn pointer_move(harness: &mut Harness<'_>, pos: Pos2) {
            harness.input_mut().events.push(Event::PointerMoved(pos));
            harness.run_steps(1);
        }

        fn pointer_button(harness: &mut Harness<'_>, pos: Pos2, pressed: bool) {
            harness.input_mut().events.push(Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.run_steps(1);
        }

        fn center_of(harness: &Harness<'_>, role: Role, label: &str) -> Option<Pos2> {
            harness
                .query_by_role_and_label(role, label)
                .map(|node| node.rect().center())
        }

        /// A real primary click: move, re-read the rect, press, hold for
        /// `hold_frames`, re-read, release. The re-reads are the point: a real
        /// pointer cannot click a node that has already disappeared.
        fn real_click(
            harness: &mut Harness<'_>,
            role: Role,
            label: &str,
            hold_frames: usize,
        ) {
            let pos = center_of(harness, role, label)
                .unwrap_or_else(|| panic!("{role:?} {label:?} is not in the tree before the click"));
            pointer_move(harness, pos);
            let down = center_of(harness, role, label).unwrap_or(pos);
            pointer_button(harness, down, true);
            if hold_frames > 0 {
                harness.run_steps(hold_frames);
            }
            let up = center_of(harness, role, label).unwrap_or(down);
            pointer_button(harness, up, false);
        }

        fn empty_multiline_center(harness: &Harness<'_>) -> Option<Pos2> {
            harness
                .get_all_by_role(Role::MultilineTextInput)
                .find(|node| node.value().as_deref() == Some(""))
                .map(|node| node.rect().center())
        }

        /// Focus a script block with a real click and type a character cue.
        fn click_into_empty_character_block(harness: &mut Harness<'_>, doc_id: &str) {
            let pos = empty_multiline_center(harness)
                .expect("the fixture should expose one empty Character input");
            pointer_move(harness, pos);
            let pos = empty_multiline_center(harness)
                .expect("the empty Character input should survive the pointer move");
            pointer_button(harness, pos, true);
            pointer_button(harness, pos, false);
            harness.run_steps(2);
            assert!(
                harness
                    .ctx
                    .memory(|memory| memory.has_focus(block_id(doc_id, 0))),
                "a real pointer click should focus the Character block"
            );

            harness.input_mut().events.push(Event::Text("ma".into()));
            harness.run_steps(2);
            assert!(
                harness
                    .query_by_role_and_label(Role::Button, "Mara Venn")
                    .is_some(),
                "typing into the clicked cue should open the completion popup"
            );
        }

        fn set_caret(harness: &Harness<'_>, id: Id, index: usize) {
            let mut state =
                egui::text_edit::TextEditState::load(&harness.ctx, id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(index),
            )));
            state.store(&harness.ctx, id);
        }

        #[test]
        fn real_pointer_click_on_a_character_name_fills_the_block() {
            let (app, doc_id) = click_fixture();
            let mut app = app;
            // No queued focus: the pointer click has to do the focusing.
            app.block_focus = None;
            let (mut harness, app) = real_harness(app, SCREEN, None, 4, editor::show);
            click_into_empty_character_block(&mut harness, &doc_id);

            real_click(&mut harness, Role::Button, "Mara Venn", 2);
            harness.run_steps(2);

            assert_eq!(
                block_text(&app.borrow(), &doc_id, 0),
                "Mara Venn",
                "a real pointer click on the suggestion should fill the cue"
            );
            assert!(
                app.borrow().new_character.is_none(),
                "choosing a name with the pointer must not open the new-character modal"
            );
        }

        #[test]
        fn real_pointer_click_on_new_character_opens_the_modal() {
            let (app, doc_id) = click_fixture();
            let mut app = app;
            app.block_focus = None;
            let (mut harness, app) = real_harness(app, SCREEN, None, 4, editor::show);
            click_into_empty_character_block(&mut harness, &doc_id);

            // Extend "MA" into a word that matches no sheet, so the popup's
            // only row is the new-character action.
            harness.input_mut().events.push(Event::Text("zz".into()));
            harness.run_steps(2);

            real_click(&mut harness, Role::Button, "+ New character…", 2);
            harness.run_steps(2);

            let borrowed = app.borrow();
            let state = borrowed
                .new_character
                .as_ref()
                .expect("a real pointer click should open the new-character modal");
            assert_eq!(state.doc_id, doc_id);
            assert_eq!(state.block, 0);
            assert_eq!(
                block_text(&borrowed, &doc_id, 0),
                "mazz",
                "the popup's sentinel insert must be undone before the modal opens"
            );
        }

        #[test]
        fn real_pointer_hover_does_not_dismiss_the_character_popup() {
            let (app, doc_id) = click_fixture();
            let mut app = app;
            app.block_focus = None;
            let (mut harness, app) = real_harness(app, SCREEN, None, 4, editor::show);
            click_into_empty_character_block(&mut harness, &doc_id);

            // A human moves the pointer onto the popup and dwells there before
            // pressing. CompletionPopup disables focus surrender while hovered,
            // so the widget must keep focus and the popup must stay open.
            let pos = center_of(&harness, Role::Button, "Mara Venn")
                .expect("Mara Venn should be in the open popup");
            pointer_move(&mut harness, pos);
            harness.run_steps(6);
            assert!(
                harness
                    .query_by_role_and_label(Role::Button, "Mara Venn")
                    .is_some(),
                "hovering the popup for several frames must not dismiss it"
            );

            // Finish the click from where the pointer already is.
            let pos = center_of(&harness, Role::Button, "Mara Venn")
                .expect("Mara Venn should still be in the open popup");
            pointer_button(&mut harness, pos, true);
            pointer_button(&mut harness, pos, false);
            harness.run_steps(2);
            assert_eq!(block_text(&app.borrow(), &doc_id, 0), "Mara Venn");
        }

        #[test]
        fn real_pointer_character_popup_works_at_the_real_editor_width() {
            let (app, doc_id) = click_fixture();
            let mut app = app;
            app.block_focus = None;
            // 1440x930 window with ~800 px left for the editor once the rail,
            // binder and inspector are drawn.
            let (mut harness, app) = real_harness(app, REAL_SCREEN, Some(800.0), 4, editor::show);
            click_into_empty_character_block(&mut harness, &doc_id);

            real_click(&mut harness, Role::Button, "Mara Venn", 2);
            harness.run_steps(2);
            assert_eq!(
                block_text(&app.borrow(), &doc_id, 0),
                "Mara Venn",
                "the narrowed editor column should behave like the full-width harness"
            );
        }

        #[test]
        fn real_pointer_selects_the_spelling_language() {
            let mut app = PortmanteauApp::new(None);
            app.settings.spellcheck = false;
            app.settings.language = "en_US".into();
            app.view = View::Settings;
            app.settings_tab = 0;
            let (mut harness, app) = real_harness(app, REAL_SCREEN, None, 4, crate::ui::views::show);

            // Open the ComboBox with the pointer, then pick an item with it.
            // The ComboBox node is unlabelled in older egui builds, so click
            // by role rather than by label.
            let combo = harness.get_by_role(Role::ComboBox).rect().center();
            pointer_move(&mut harness, combo);
            let combo = harness.get_by_role(Role::ComboBox).rect().center();
            pointer_button(&mut harness, combo, true);
            pointer_button(&mut harness, combo, false);
            harness.run_steps(2);
            real_click(&mut harness, Role::Button, "English (UK)", 2);
            harness.run_steps(2);

            assert_eq!(
                app.borrow().settings.language,
                "en_GB",
                "a real pointer click on English (UK) should update the stored language"
            );
        }

        /// Build the multiline fixture and settle it so the focus-lock filter
        /// has been installed by a prior frame.
        fn settled_multiline(
            text: &str,
        ) -> (Harness<'static>, Rc<RefCell<PortmanteauApp>>, String, Id) {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                set_script_block(doc, 0, ElementKind::Action, text);
            }
            app.selected_block = 0;
            app.block_focus = Some(0);
            let (harness, app) = real_harness(app, SCREEN, None, 4, editor::show);
            let id = block_id(&doc_id, 0);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "the block should hold focus after the settle frames"
            );
            (harness, app, doc_id, id)
        }

        #[test]
        fn arrow_down_and_up_stay_in_a_multiline_script_block() {
            let (mut harness, app, _doc_id, id) = settled_multiline("one\ntwo");
            set_caret(&harness, id, 0);
            harness.run_steps(1);
            assert_eq!(caret_index(&harness, id), 0, "caret starts on line one");

            harness.key_press(Key::ArrowDown);
            harness.run_steps(2);
            assert_eq!(
                app.borrow().selected_block,
                0,
                "ArrowDown must not change the selected block"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "ArrowDown must not move focus to another block"
            );
            assert_eq!(
                caret_index(&harness, id),
                4,
                "ArrowDown should move the caret to the start of the second line"
            );

            harness.key_press(Key::ArrowUp);
            harness.run_steps(2);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "ArrowUp must not move focus to another block"
            );
            assert_eq!(
                caret_index(&harness, id),
                0,
                "ArrowUp should return the caret to the start of the first line"
            );
            assert_eq!(app.borrow().selected_block, 0);
        }

        /// Word-processor modifiers must keep working in a script block:
        /// Shift+Arrow selects, Ctrl+Arrow jumps a whole word, and a plain
        /// arrow only moves the caret (never focus or the selected block).
        #[test]
        fn script_shift_ctrl_and_plain_arrows_move_the_caret_not_the_focus() {
            let (mut harness, app, _doc_id, id) = settled_multiline("one two three");
            set_caret(&harness, id, 0);
            harness.run_steps(1);
            assert_eq!(caret_index(&harness, id), 0);

            harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
            harness.run_steps(2);
            let range = caret_range(&harness, id);
            assert_eq!(
                (range.primary.index.0, range.secondary.index.0),
                (1, 0),
                "Shift+ArrowRight should extend a selection one character"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "Shift+ArrowRight must not move focus"
            );
            assert_eq!(app.borrow().selected_block, 0);

            set_caret(&harness, id, 0);
            harness.run_steps(1);
            harness.key_press_modifiers(Modifiers::CTRL, Key::ArrowRight);
            harness.run_steps(2);
            let range = caret_range(&harness, id);
            assert_eq!(
                (range.primary.index.0, range.secondary.index.0),
                (3, 3),
                "Ctrl+ArrowRight should jump to the end of the whole word"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "Ctrl+ArrowRight must not move focus"
            );
            assert_eq!(app.borrow().selected_block, 0);

            set_caret(&harness, id, 4);
            harness.run_steps(1);
            harness.key_press(Key::ArrowLeft);
            harness.run_steps(2);
            assert_eq!(
                caret_index(&harness, id),
                3,
                "a plain arrow moves the caret one character"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "a plain arrow must not move focus"
            );
            assert_eq!(app.borrow().selected_block, 0);

            harness.key_press(Key::ArrowRight);
            harness.run_steps(2);
            assert_eq!(caret_index(&harness, id), 4);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "a plain arrow must not move focus"
            );
            assert_eq!(app.borrow().selected_block, 0);
        }

        #[test]
        fn inline_format_toolbar_applies_bold_to_selected_prose() {
            let mut app = PortmanteauApp::new(None);
            app.settings.spellcheck = false;
            app.show_spelling = false;
            app.view = View::Write;
            let doc_id = app
                .ws
                .project
                .documents
                .iter()
                .find(|document| document.kind == DocKind::Chapter)
                .map(|document| document.id.clone())
                .expect("the demo has a prose chapter");
            app.ws
                .project
                .document_mut(&doc_id)
                .unwrap()
                .set_plain_text("hello world");
            app.selected = Some(doc_id.clone());
            let (mut harness, app) = real_harness(app, SCREEN, None, 4, editor::show);
            let id = Id::unique(("prose_body", doc_id.as_str()));

            harness
                .get_all_by_value("hello world")
                .find(|node| node.accesskit_node().role() == Role::MultilineTextInput)
                .expect("the prose flow should be in the accessibility tree")
                .focus();
            harness.run_steps(2);
            let mut state =
                egui::text_edit::TextEditState::load(&harness.ctx, id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(5),
            )));
            state.store(&harness.ctx, id);
            harness.run_steps(3);
            harness.get_by_role_and_label(Role::Button, "B").click();
            harness.run_steps(2);

            let borrowed = app.borrow();
            let content = &borrowed.ws.project.document(&doc_id).unwrap().content;
            let first = match &content.paragraphs[0].content[0] {
                portmanteau_core::InlineContent::Text(run) => run,
                _ => panic!("the first paragraph should begin with a text run"),
            };
            assert_eq!(first.text, "hello");
            assert!(
                first.marks.contains(&portmanteau_core::InlineMark::Bold),
                "the toolbar should apply Bold to the selected text range"
            );
        }

        #[test]
        fn arrow_down_navigates_to_the_next_styled_paragraph_in_one_flow() {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                doc.set_script_blocks(vec![
                    portmanteau_core::Block::new(ElementKind::Action, "first line"),
                    portmanteau_core::Block::new(ElementKind::Dialogue, "next paragraph"),
                ]);
            }
            app.block_focus = Some(0);
            let (mut harness, app) = probe_harness(app, editor::show);
            let id = block_id(&doc_id, 0);
            assert!(harness.ctx.memory(|memory| memory.has_focus(id)));
            set_caret(&harness, id, 0);
            harness.run_steps(1);

            harness.key_press(Key::ArrowDown);
            harness.run_steps(2);

            assert!(harness.ctx.memory(|memory| memory.has_focus(id)));
            assert!(
                caret_index(&harness, id) > 0,
                "ArrowDown should move through the shared styled-text flow"
            );
            assert!(app.borrow().selected_block < 2);
        }

        #[test]
        fn enter_applies_the_current_paragraphs_next_style() {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                doc.set_script_blocks(vec![portmanteau_core::Block::new(
                    ElementKind::Character,
                    "Mara Venn",
                )]);
            }
            app.block_focus = Some(0);
            let (mut harness, app) = probe_harness(app, editor::show);
            let id = block_id(&doc_id, 0);
            assert!(harness.ctx.memory(|memory| memory.has_focus(id)));

            harness.key_press(Key::Enter);
            harness.run_steps(2);

            let borrowed = app.borrow();
            let paragraph_styles: Vec<&str> = borrowed
                .ws
                .project
                .document(&doc_id)
                .unwrap()
                .content
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.style_id.as_str())
                .collect();
            assert_eq!(paragraph_styles, vec!["character", "dialogue"]);
        }

        /// The same word-processor keys on the prose body.
        #[test]
        fn prose_shift_ctrl_and_plain_arrows_move_the_caret_not_the_focus() {
            let mut app = PortmanteauApp::new(None);
            app.settings.spellcheck = false;
            app.show_spelling = false;
            app.view = View::Write;
            app.write_mode = WriteMode::Continuous;
            let doc_id = app
                .ws
                .project
                .documents
                .iter()
                .find(|d| d.kind == DocKind::Chapter)
                .map(|d| d.id.clone())
                .expect("the demo has a prose chapter");
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                doc.set_plain_text("one two three");
            }
            app.selected = Some(doc_id.clone());
            let (mut harness, app) = real_harness(app, SCREEN, None, 4, editor::show);
            let id = Id::unique(("prose_body", doc_id.as_str()));

            harness
                .get_all_by_value("one two three")
                .find(|node| node.accesskit_node().role() == Role::MultilineTextInput)
                .expect("the prose body should be a multiline input holding the seed text")
                .focus();
            harness.run_steps(2);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "the prose body should accept focus"
            );

            set_caret(&harness, id, 0);
            harness.run_steps(1);
            harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
            harness.run_steps(2);
            let range = caret_range(&harness, id);
            assert_eq!(
                (range.primary.index.0, range.secondary.index.0),
                (1, 0),
                "Shift+ArrowRight should extend a selection in prose"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "Shift+ArrowRight must not move focus"
            );

            set_caret(&harness, id, 0);
            harness.run_steps(1);
            harness.key_press_modifiers(Modifiers::CTRL, Key::ArrowRight);
            harness.run_steps(2);
            let range = caret_range(&harness, id);
            assert_eq!(
                (range.primary.index.0, range.secondary.index.0),
                (3, 3),
                "Ctrl+ArrowRight should jump to the end of the whole word in prose"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "Ctrl+ArrowRight must not move focus"
            );

            set_caret(&harness, id, 4);
            harness.run_steps(1);
            harness.key_press(Key::ArrowLeft);
            harness.run_steps(2);
            assert_eq!(
                caret_index(&harness, id),
                3,
                "a plain arrow moves the prose caret one character"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "a plain arrow must not move focus"
            );
            assert_eq!(app.borrow().selected_block, 0);
        }

        #[test]
        fn arrow_down_on_a_single_line_script_block_keeps_focus() {
            let (mut harness, app, _doc_id, id) = settled_multiline("solo");
            set_caret(&harness, id, 0);
            harness.run_steps(1);

            harness.key_press(Key::ArrowDown);
            harness.run_steps(2);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "ArrowDown at the only line of a one-line block must not move focus"
            );
            assert_eq!(
                app.borrow().selected_block,
                0,
                "a one-line block keeps the selection"
            );
            // egui puts a vertical move with no line below at the end of the
            // only line ("solo" is four bytes). It stayed in this widget.
            assert_eq!(
                caret_index(&harness, id),
                4,
                "ArrowDown on a one-line block moves the caret to the line end"
            );

            harness.key_press(Key::ArrowUp);
            harness.run_steps(2);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "ArrowUp at the only line must not move focus either"
            );
            assert_eq!(caret_index(&harness, id), 0);
            assert_eq!(app.borrow().selected_block, 0);
        }

        /// The frame that first focuses the block has no stored EventFilter
        /// yet: egui reads the filter at the *start* of a frame, and a freshly
        /// requested focus carries a default filter until the widget has held
        /// focus for one full frame. The editor cancels the pending directional
        /// move for a focused block, so an unmodified arrow in that frame stays
        /// in the text.
        #[test]
        fn arrow_down_in_the_first_focus_frame_stays_in_the_block() {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                set_script_block(doc, 0, ElementKind::Action, "one\ntwo");
            }
            app.selected_block = 0;
            app.block_focus = Some(0);

            // Use the hand-driven context so no settle frame runs first.
            let ctx = headless_ctx();
            frame(&mut app, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE)]);

            let id = block_id(&doc_id, 0);
            assert!(
                ctx.memory(|memory| memory.has_focus(id)),
                "an arrow in the frame that first focuses the block must leave \
                 focus in the block; it moved to another element instead"
            );
            assert_eq!(app.selected_block, 0);
        }

        #[test]
        fn tab_in_the_first_focus_frame_cycles_the_element() {
            let (mut app, doc_id) = script_fixture();
            app.block_focus = Some(0);
            let ctx = headless_ctx();
            let id = block_id(&doc_id, 0);
            let medium = app.ws.project.document(&doc_id).unwrap().medium;
            let before = block_kind(&app, &doc_id, 0);
            let forward = before.next(medium);
            assert_ne!(before, forward, "the demo medium has more than one kind");

            // The same stale-filter frame as the arrow probe: Tab must cycle the
            // element instead of handing focus to the next widget.
            frame(&mut app, &ctx, vec![key(Key::Tab, Modifiers::NONE)]);
            assert_eq!(
                block_kind(&app, &doc_id, 0),
                forward,
                "Tab in the first focus frame should advance the element type"
            );
            assert!(
                ctx.memory(|memory| memory.has_focus(id)),
                "Tab in the first focus frame must not move focus out of the block"
            );
        }

        /// The frame after the focus request is the second stale-filter frame:
        /// egui resolved the arrow from the default filter at `begin_pass`
        /// before the TextEdit could install its own, so the editor's
        /// cancellation has to cover it too.
        #[test]
        fn arrow_down_on_the_second_frame_after_focus_stays_in_the_block() {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                set_script_block(doc, 0, ElementKind::Action, "one\ntwo");
            }
            app.selected_block = 0;
            app.block_focus = Some(0);
            let ctx = headless_ctx();
            let id = block_id(&doc_id, 0);

            // Frame one requests focus; no event yet.
            frame(&mut app, &ctx, Vec::new());
            assert!(
                ctx.memory(|memory| memory.has_focus(id)),
                "the first frame should focus the block"
            );

            frame(&mut app, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE)]);
            assert!(
                ctx.memory(|memory| memory.has_focus(id)),
                "an arrow on the frame after the focus request must not move focus"
            );
            assert_eq!(app.selected_block, 0);
        }

        /// The realistic counterpart of the ignored probe above: a human clicks
        /// into the block and presses an arrow on a later frame. The click's
        /// frame installs the focus-lock filter, so the arrow is claimed by the
        /// TextEdit and focus does not move.
        #[test]
        fn arrow_down_after_a_real_pointer_click_stays_in_the_block() {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                set_script_block(doc, 0, ElementKind::Action, "one\ntwo");
            }
            app.block_focus = None;
            let (mut harness, app) = real_harness(app, SCREEN, None, 4, editor::show);

            let by_value = |harness: &Harness<'_>| {
                harness
                    .get_all_by_value("one\ntwo")
                    .find(|node| node.accesskit_node().role() == Role::MultilineTextInput)
                    .map(|node| node.rect().center())
            };
            let pos = by_value(&harness).expect("the multiline block should be in the tree");
            pointer_move(&mut harness, pos);
            let pos = by_value(&harness).expect("the block should survive the pointer move");
            pointer_button(&mut harness, pos, true);
            pointer_button(&mut harness, pos, false);
            harness.run_steps(2);

            let id = block_id(&doc_id, 0);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "the click should focus the block"
            );
            set_caret(&harness, id, 0);
            harness.run_steps(1);
            harness.key_press(Key::ArrowDown);
            harness.run_steps(2);

            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "an arrow after a mouse click must stay in the block"
            );
            assert_eq!(app.borrow().selected_block, 0);
            assert_eq!(caret_index(&harness, id), 4);
        }

        /// The other delivery path: `accesskit::Action::Click` with no pointer
        /// events at all. egui routes it to the same button handler, so the
        /// cue fills too — which is exactly why an AccessKit-driven probe
        /// cannot prove that a *mouse* click works. Kept for comparison.
        #[test]
        fn accesskit_action_click_also_fills_the_block() {
            let (app, doc_id) = click_fixture();
            let (mut harness, app) = real_harness(app, SCREEN, None, 4, editor::show);
            harness.input_mut().events.push(Event::Text("ma".into()));
            harness.run_steps(2);

            harness
                .get_by_role_and_label(Role::Button, "Mara Venn")
                .click_accesskit();
            harness.run_steps(2);

            assert_eq!(
                block_text(&app.borrow(), &doc_id, 0),
                "Mara Venn",
                "the AccessKit action reaches the same button handler"
            );
            assert!(app.borrow().new_character.is_none());
        }

        #[test]
        fn tab_and_shift_tab_cycle_the_element_without_moving_focus() {
            let (mut harness, app, doc_id, id) = settled_multiline("solo");
            let medium = app
                .borrow()
                .ws
                .project
                .document(&doc_id)
                .expect("document still exists")
                .medium;
            let before = block_kind(&app.borrow(), &doc_id, 0);
            let forward = before.next(medium);
            assert_ne!(before, forward, "the demo medium has more than one kind");

            harness.key_press(Key::Tab);
            harness.run_steps(2);
            assert_eq!(
                block_kind(&app.borrow(), &doc_id, 0),
                forward,
                "Tab should advance the element type"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "Tab should not move focus to another widget"
            );

            harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
            harness.run_steps(2);
            assert_eq!(
                block_kind(&app.borrow(), &doc_id, 0),
                before,
                "Shift+Tab should return to the previous element type"
            );
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(id)),
                "Shift+Tab should keep focus on the block"
            );
        }

        #[test]
        fn pointer_drag_selection_crosses_script_paragraph_styles() {
            let (mut app, doc_id) = script_fixture();
            if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                doc.set_script_blocks(vec![
                    portmanteau_core::Block::new(ElementKind::Action, "alpha beta"),
                    portmanteau_core::Block::new(ElementKind::Dialogue, "gamma delta"),
                ]);
            }
            app.block_focus = None;
            let (mut harness, _app) = real_harness(app, SCREEN, None, 4, editor::show);

            let input_rect = |harness: &Harness<'_>| {
                harness
                    .get_all_by_value("alpha beta\ngamma delta")
                    .find(|node| {
                        node.accesskit_node().role() == Role::MultilineTextInput
                    })
                    .map(|node| node.rect())
            };
            let rect = input_rect(&harness).expect("the rich flow input should be exposed");
            let start = Pos2::new(rect.left() + 12.0, rect.top() + 9.0);
            let end = Pos2::new(rect.left() + 210.0, rect.top() + 27.0);

            // Start in the first styled paragraph and release in the second.
            pointer_move(&mut harness, start);
            pointer_button(&mut harness, start, true);
            pointer_move(&mut harness, end);
            pointer_button(&mut harness, end, false);
            harness.run_steps(2);

            let selection = caret_range(&harness, block_id(&doc_id, 0));
            let low = selection
                .primary
                .index
                .0
                .min(selection.secondary.index.0);
            let high = selection
                .primary
                .index
                .0
                .max(selection.secondary.index.0);
            assert_ne!(
                low, high,
                "a real drag across screenplay paragraphs should select text"
            );
            assert!(
                low < "alpha beta".chars().count()
                    && high > "alpha beta\ngamma".chars().count() - "gamma".chars().count(),
                "the selection should include characters from both paragraphs; got {low}..{high}"
            );
        }
    }
}
