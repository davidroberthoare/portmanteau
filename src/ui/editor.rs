//! The writing surface: a paper-like prose editor and a typed screenplay editor.

use std::collections::HashMap;

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, FontId, Frame, Id, Key, Layout, Margin,
    Modifiers, RichText, ScrollArea, Sense, Stroke, TextEdit, Vec2,
};
use portmanteau_core::spell::{byte_offset, word_at, SpellChecker};
use portmanteau_core::{DocKind, ElementKind, ItemKind, Medium};

use crate::app::{
    NewCharacterState, PortmanteauApp, WriteMode,
};
use crate::theme::*;
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
    // The Paged view shows this document's pages from the real export layout.
    let paged = (app.write_mode == WriteMode::Paged)
        .then(|| crate::ui::preview::paged_document(ui, &app.ws.project, &doc_id));
    if !app.focus_mode {
        let paging = paged.as_ref().and_then(|paged| paged.as_ref().ok()).map(|paged| {
            // (number of the document's first page, pages it touches, pages in the manuscript)
            (paged.first + 1, paged.count, paged.layout.page_count())
        });
        toolbar(app, ui, paging, miss.len());

        if app.settings.spellcheck && app.show_spelling {
            spelling_panel(app, ui, &doc_id, &miss);
        }
    }

    if let Some(paged) = &paged {
        crate::ui::preview::paged_view(app, ui, paged, &doc_id);
        return;
    }
    crate::ui::find_bar::show(app, ui, &doc_id);

    let wide = if app.focus_mode { 860.0 } else { 780.0 };
    let is_script_view = app
        .ws
        .project
        .document(&doc_id)
        .is_some_and(|document| document.medium.is_script());
    // Screenplays are drawn as a true US Letter sheet (612 x 792 pt) with a
    // 1.5" left and 1" right margin, so 12 pt Courier gives 60 characters.
    let margins = script_margins(app, &doc_id);
    let text_pt = (612.0 - margins.left - margins.right).max(144.0);
    let script_scale = (app.editor_zoom.clamp(0.7, 1.6) * SCRIPT_ZOOM_BASE)
        .min((ui.available_width() - 72.0) / 612.0)
        .max(0.3);
    let page_w = if is_script_view {
        612.0 * script_scale
    } else {
        (ui.available_width() - 72.0).clamp(420.0, wide)
    };
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
                    .inner_margin(if is_script {
                        Margin::ZERO
                    } else {
                        let side = if app.focus_mode { 64 } else { 54 };
                        Margin {
                            left: side,
                            right: side,
                            top: 44,
                            bottom: 34,
                        }
                    })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        if is_script {
                            ui.add_space(margins.top * script_scale);
                            ui.horizontal_top(|ui| {
                                let spacing = ui.spacing().item_spacing.x;
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.add_space(margins.left * script_scale);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.x = spacing;
                                    ui.set_width(text_pt * script_scale);
                                    script_page(app, ui, &doc_id);
                                });
                            });
                            ui.add_space(margins.bottom * script_scale);
                        } else {
                            prose_page(app, ui, &doc_id, &mut take_snapshot);
                        }
                    });
            });
            // Room to scroll the caret line up off the window edge, so the
            // cue popup and the clearance below the line fit at the end of
            // the script too.
            ui.add_space(CARET_BOTTOM_CLEARANCE + 80.0);
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
    let checker = &app.spell;
    let spell_on = app.settings.spellcheck;
    let mut spelling: Vec<FieldSpell> = Vec::new();

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
                let (_, fixed) = spelled_text_edit(
                    ui,
                    &mut synopsis,
                    checker,
                    spell_on,
                    Id::unique(("folder_synopsis", doc_id)),
                    serif(13.0),
                    TEXT,
                    |edit| {
                        edit.desired_width(f32::INFINITY).desired_rows(3).hint_text(
                            RichText::new("What belongs in this folder?")
                                .font(serif(13.0))
                                .color(FAINT),
                        )
                    },
                );
                spelling.push(fixed);
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

    apply_field_spell(app, spelling);
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
    paging: Option<(usize, usize, usize)>,
    miss_count: usize,
) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .inner_margin(Margin::symmetric(16, 7))
        .show(ui, |ui| {
            let row_width = ui.available_width();
            ui.allocate_ui_with_layout(
                Vec2::new(row_width, TOOLBAR_ROW_H),
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
                        let galley = ui.fonts_mut(|fonts| {
                            fonts.layout_no_wrap(label, ui_body(), color)
                        });
                        let size = Vec2::new(galley.size().x + 20.0, TOOLBAR_ROW_H);
                        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                        let fill = if app.show_spelling { ACCENT_SOFT } else { PANEL_2 };
                        ui.painter().rect_filled(rect, CornerRadius::same(5), fill);
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::same(5),
                            Stroke::new(1.0, BORDER_SOFT),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().galley(
                            egui::pos2(
                                rect.center().x - galley.size().x * 0.5,
                                rect.center().y - galley.size().y * 0.5,
                            ),
                            galley,
                            color,
                        );
                        if response
                            .on_hover_text("Review misspellings and apply suggestions")
                            .clicked()
                        {
                            app.show_spelling = !app.show_spelling;
                        }
                    }

                    // Continuous vs Paged. Paged is a read-only print preview.
                    if app.write_mode == WriteMode::Paged {
                        if let Some((first, count, total)) = paging {
                            // The numbers printed on the pages, not counted
                            // from this document: page 12 here is page 12 in
                            // the exported PDF.
                            let current = first - 1 + app.paged_page.clamp(1, count.max(1));
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
    // A link is an address, not prose: offer its own actions, never spelling.
    if let Some((range, link)) = link_in_text(text, byte) {
        link_actions_ui(ui, range, &link);
        return None;
    }
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

/// Leave out the word being typed: while a field has the keyboard and its caret
/// sits at the end of a flagged word, that word is not underlined yet (it would
/// flash red on every half-typed word). It is flagged once the caret moves on.
pub(crate) fn without_word_being_typed(ctx: &egui::Context, id: Id, text: &str, mut spans: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if spans.is_empty() || !ctx.memory(|m| m.has_focus(id)) {
        return spans;
    }
    let Some(range) = egui::text_edit::TextEditState::load(ctx, id).and_then(|state| state.cursor.char_range()) else {
        return spans;
    };
    if range.primary != range.secondary {
        return spans;
    }
    let caret = text.char_indices().nth(range.primary.index.0).map_or(text.len(), |(byte, _)| byte);
    spans.retain(|&(start, end)| !(start < caret && caret == end));
    spans
}

/// Apply the menu's choices after the document borrow has ended, so the
/// document is never borrowed while a widget runs (AGENTS.md §8).
/// Misspelled byte ranges of a field's text, for its layouter (empty when
/// spell-checking is off). One dictionary pass per frame; the layouter reuses it.
pub(crate) fn field_spans(checker: &SpellChecker, enabled: bool, text: &str) -> Vec<(usize, usize)> {
    if enabled {
        checker.misspelled_spans(text)
    } else {
        Vec::new()
    }
}

/// What the spelling menu on a secondary field asked for, beyond the fix it
/// already applied to the field itself.
#[derive(Default)]
pub(crate) struct FieldSpell {
    /// A suggestion was applied to the field; the caller marks it edited.
    pub replaced: bool,
    ignore: Option<String>,
    add: Option<String>,
}

/// The right-click spelling menu for a synopsis, notes, research or database
/// field. A suggestion is applied to `field` at once; "Ignore" and "Add to
/// dictionary" change the checker, which the caller may be holding, so they are
/// returned for `apply_field_spell` to run after its borrows end.
pub(crate) fn field_spell_menu(
    ui: &mut egui::Ui,
    checker: &SpellChecker,
    enabled: bool,
    key: Id,
    output: &egui::text_edit::TextEditOutput,
    text: &str,
    field: &mut portmanteau_core::RichDocument,
    sheet: &portmanteau_core::StyleSheet,
) -> FieldSpell {
    let mut outcome = FieldSpell::default();
    if !enabled {
        return outcome;
    }
    let byte = spell_menu_byte(
        ui.ctx(),
        key,
        &output.response,
        &output.galley,
        output.galley_pos,
        text,
    );
    let mut choice = None;
    output.response.context_menu(|ui| {
        choice = spell_menu_ui(ui, checker, text, byte);
    });
    match choice {
        Some(SpellChoice::Replace(range, replacement))
            if text.is_char_boundary(range.start) && text.is_char_boundary(range.end) =>
        {
            let start = text[..range.start].chars().count();
            let end = text[..range.end].chars().count();
            field.replace_range(start..end, &replacement, sheet);
            outcome.replaced = true;
        }
        Some(SpellChoice::Ignore(word)) => outcome.ignore = Some(word),
        Some(SpellChoice::Add(word)) => outcome.add = Some(word),
        _ => {}
    }
    outcome
}

/// A multiline plain-text field with spell-checking: misspelled words are
/// underlined and the right-click menu offers suggestions. A suggestion is
/// applied to `text` at once; "Ignore" and "Add to dictionary" are returned
/// for `apply_field_spell`, as for the rich fields. `configure` styles the
/// `TextEdit` (font, hint, rows, frame); the layouter is supplied here.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spelled_text_edit(
    ui: &mut egui::Ui,
    text: &mut String,
    checker: &SpellChecker,
    enabled: bool,
    id: Id,
    font: egui::FontId,
    color: Color32,
    configure: impl for<'l> FnOnce(TextEdit<'l>) -> TextEdit<'l>,
) -> (egui::text_edit::TextEditOutput, FieldSpell) {
    let spans = without_word_being_typed(ui.ctx(), id, text, field_spans(checker, enabled, text));
    let base = egui::TextFormat {
        font_id: font,
        color,
        ..Default::default()
    };
    let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut job = spell_layout_job(buffer.as_str(), &spans, &base);
        job.wrap.max_width = wrap_width;
        ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
    };
    let output = configure(TextEdit::multiline(&mut *text).id(id).layouter(&mut layouter)).show(ui);
    mark_rich(ui.ctx(), id);
    let mut outcome = FieldSpell::default();
    if enabled {
        let byte = spell_menu_byte(
            ui.ctx(),
            Id::unique(("spell_menu_plain", id)),
            &output.response,
            &output.galley,
            output.galley_pos,
            text,
        );
        let mut choice = None;
        output.response.context_menu(|ui| {
            choice = spell_menu_ui(ui, checker, text, byte);
        });
        match choice {
            Some(SpellChoice::Replace(range, replacement))
                if text.is_char_boundary(range.start) && text.is_char_boundary(range.end) =>
            {
                text.replace_range(range, &replacement);
                outcome.replaced = true;
            }
            Some(SpellChoice::Ignore(word)) => outcome.ignore = Some(word),
            Some(SpellChoice::Add(word)) => outcome.add = Some(word),
            _ => {}
        }
    }
    (output, outcome)
}

/// Run the checker changes a field's spelling menu asked for.
pub(crate) fn apply_field_spell(app: &mut PortmanteauApp, outcomes: Vec<FieldSpell>) {
    for outcome in outcomes {
        if let Some(word) = outcome.ignore {
            app.spell.ignore(&word);
        }
        if let Some(word) = outcome.add {
            app.add_to_dictionary(&word);
        }
    }
}

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
    // The page count the exporter will produce; recomputed a moment after typing
    // pauses, never on every keystroke.
    let pages = crate::ui::preview::page_counts(ui, &app.ws.project).label(doc_id);
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
        paper_header(ui, &project_title, doc.medium, &doc.status, doc.words(), &pages);

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

        let mut edit_text = doc.content.editor_text();
        let rows = ((edit_text.chars().count() as f32) / 72.0).ceil().max(18.0) as usize;
        // One dictionary pass per frame, reused by every layouter call.
        let spans = if spell_on {
            without_word_being_typed(ui.ctx(), prose_id, &edit_text, checker.misspelled_spans(&edit_text))
        } else {
            Vec::new()
        };
        let content_for_layout = doc.content.clone();
        let sheet_for_layout = sheet.clone();
        let styles_for_layout = styles.clone();
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
            script_flow_galley(
                ui,
                text.as_str(),
                &content_for_layout,
                &sheet_for_layout,
                &styles_for_layout,
                zoom,
                wrap_width,
                &spans,
            )
        };
        let reveal = take_caret_request(&mut app.caret_request, ui, doc_id, &doc.content, prose_id);
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
        reveal_caret(ui, &output, reveal);
        if output.response.changed() {
            let caret = output.cursor_range.map(|range| range.primary.index.0);
            doc.content
                .sync_editor_text_with_caret(&edit_text, &sheet, caret);
            autolink_pasted(ui.ctx(), &mut doc.content, caret);
            doc.touch();
            dirty = true;
            if let Some(caret) = caret {
                keep_caret_clear_of_bottom(ui, &output, caret);
            }
        }
        crate::ui::editor::remember_selection_anchor(ui.ctx(), output.response.id, &output.galley, output.galley_pos);
        let picture_rects = paint_rich_images(
            ui,
            &output.galley,
            output.galley_pos,
            output.response.rect,
            &doc.content,
            &image_textures,
            zoom,
        );
        if ui.input(|i| i.pointer.secondary_clicked()) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                if let Some((paragraph, _)) = picture_rects.iter().find(|(_, rect)| rect.contains(pos)) {
                    app.picture_menu = Some(crate::app::PictureMenu {
                        doc_id: doc_id.to_owned(),
                        paragraph: *paragraph,
                        pos,
                        serial: ui.input(|i| i.time).to_bits(),
                    });
                }
            }
        }
        crate::ui::find_bar::paint_matches(ui, &app.find_bar, &output.galley, output.galley_pos);
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
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} words  ·  {} min read", words, portmanteau_core::reading_minutes(words)))
                    .font(ui_small())
                    .color(PAPER_MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(pages.as_str()).font(serif(9.0)).color(PAPER_MUTED));
            });
        });
        let _ = take_snapshot;
    }
    apply_spell_actions(app, spell_actions);
    if dirty {
        app.ws.dirty = true;
    }
}

/// Page margins of the stylesheet that owns a script document.
fn script_margins(app: &PortmanteauApp, doc_id: &str) -> portmanteau_core::rich_text::PageMargins {
    app.ws
        .project
        .document(doc_id)
        .map(|doc| app.ws.project.style_sheet_for(doc).margins)
        .unwrap_or_default()
}

fn script_page(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    script_flow_page(app, ui, doc_id);
}

/// Screenplay view over the same single flowing rich-text buffer used by
/// prose. Paragraph style ids carry screenplay semantics; the one TextEdit
/// owns the caret and selection for the complete document.
fn script_flow_page(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    // The page count the exporter will produce; recomputed a moment after typing
    // pauses, never on every keystroke.
    let pages = crate::ui::preview::page_counts(ui, &app.ws.project).label(doc_id);
    let margins = script_margins(app, doc_id);
    let zoom = (app.editor_zoom.clamp(0.7, 1.6) * SCRIPT_ZOOM_BASE)
        .min(ui.available_width() / (612.0 - margins.left - margins.right).max(144.0))
        .max(0.3);
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
        paper_header(ui, &project_title, doc.medium, &doc.status, doc.words(), &pages);
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
        ui.add_space(4.0);
        let before = doc.content.editor_text();
        let mut edit_text = before.clone();
        let spans = if spell_on {
            without_word_being_typed(ui.ctx(), flow_id, &edit_text, checker.misspelled_spans(&edit_text))
        } else {
            Vec::new()
        };
        let content_for_layout = doc.content.clone();
        let sheet_for_layout = sheet.clone();
        let styles_for_layout = styles.clone();
        let desired_rows = edit_text.lines().count().clamp(1, 2000);
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
            script_flow_galley(
                ui,
                text.as_str(),
                &content_for_layout,
                &sheet_for_layout,
                &styles_for_layout,
                zoom,
                wrap_width,
                &spans,
            )
        };
        let edit = TextEdit::multiline(&mut edit_text)
            .id(flow_id)
            .font(mono(12.0 * zoom))
            .layouter(&mut layouter)
            .text_color(PAPER_TEXT)
            .frame(Frame::NONE)
            .desired_width(f32::INFINITY)
            .desired_rows(desired_rows)
            .margin(Margin::ZERO)
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

        // ---- Character cue popup -------------------------------------
        // Our own popup rather than a stock completion list: it opens only
        // while the writer is working on a cue (typing in it, or sitting in a
        // fresh empty one), sits under the caret, and never appears just
        // because the caret passed over an existing cue.
        let cue_key = Id::unique(("cue_popup", doc_id));
        let mut cue: CueState = ui.data(|data| data.get_temp(cue_key)).unwrap_or_default();
        let mut accept_cue: Option<usize> = None;
        if cue.visible && ui.memory(|memory| memory.has_focus(flow_id)) {
            ui.input_mut(|input| {
                let count = cue.suggestions.len().max(1);
                if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    cue.selected = (cue.selected + 1) % count;
                    cue.selection_moved = true;
                }
                if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    cue.selected = (cue.selected + count - 1) % count;
                    cue.selection_moved = true;
                }
                if input.consume_key(Modifiers::NONE, Key::Enter) {
                    accept_cue = Some(cue.selected);
                }
                if input.consume_key(Modifiers::NONE, Key::Escape) {
                    cue.dismissed = cue.armed;
                    cue.visible = false;
                }
            });
        }

        // Clicking a suggestion must not take focus from the text.
        let pointer_on_popup = cue.visible
            && cue.popup_rect.is_some_and(|rect| {
                ui.ctx()
                    .pointer_interact_pos()
                    .is_some_and(|pos| rect.contains(pos))
            });
        let surrender = ui
            .ctx()
            .options(|options| options.input_options.surrender_focus_on);
        if pointer_on_popup {
            ui.ctx().options_mut(|options| {
                options.input_options.surrender_focus_on = egui::SurrenderFocusOn::Never;
            });
        }
        let reveal = take_caret_request(&mut app.caret_request, ui, doc_id, &doc.content, flow_id);
        let output = edit.show(ui);
        reveal_caret(ui, &output, reveal);
        ui.ctx()
            .options_mut(|options| options.input_options.surrender_focus_on = surrender);
        let caret = output
            .cursor_range
            .map(|range| range.primary.index.0);
        let text_changed = output.response.changed();
        if text_changed {
            doc.content
                .sync_editor_text_with_caret(&edit_text, &sheet, caret);
            doc.touch();
            dirty = true;
        }
        if focused_after_edit(&output) && (text_changed || cue.last_caret != caret) {
            if let Some(caret) = caret {
                keep_caret_clear_of_bottom(ui, &output, caret);
            }
        }
        cue.last_caret = caret;
        let caret_paragraph = caret
            .map(|index| doc.content.position_at(index).paragraph)
            .unwrap_or(app.selected_block);
        let paragraph_text = doc
            .content
            .paragraphs
            .get(caret_paragraph)
            .map(|paragraph| paragraph.editor_text())
            .unwrap_or_default();
        let is_cue = doc
            .content
            .paragraphs
            .get(caret_paragraph)
            .and_then(|paragraph| sheet.style(&paragraph.style_id))
            .and_then(|style| style.semantic_kind)
            == Some(ElementKind::Character);
        let focused = output.response.has_focus();
        if text_changed || cue.last_paragraph != Some(caret_paragraph) {
            cue.dismissed = None;
        }
        cue.last_paragraph = Some(caret_paragraph);
        let engaged = is_cue
            && (text_changed
                || paragraph_text.trim().is_empty()
                || cue.armed == Some(caret_paragraph));
        cue.armed = engaged.then_some(caret_paragraph);

        let word = paragraph_text.trim().to_owned();
        if cue.last_word != word {
            cue.selected = 0;
            cue.last_word = word.clone();
        }
        cue.suggestions = if cue.armed.is_some() && cue.dismissed.is_none() {
            cue_suggestions(
                &doc.content.to_script_blocks(&sheet),
                caret_paragraph,
                &word,
                &character_names,
            )
        } else {
            Vec::new()
        };
        cue.selected = cue.selected.min(cue.suggestions.len().saturating_sub(1));
        cue.visible = focused && !cue.suggestions.is_empty() && accept_cue.is_none();

        let mut clicked_cue = None;
        cue.popup_rect = None;
        if cue.visible {
            let caret_rect = output
                .galley
                .pos_from_cursor(egui::text::CCursor::new(caret.unwrap_or(0)))
                .translate(output.galley_pos.to_vec2());
            let row_height = ui_body().size + 22.0;
            let popup_height = cue.suggestions.len().min(CUE_POPUP_ROWS as usize) as f32
                * row_height
                + 14.0;
            let below = caret_rect.bottom() + 4.0;
            let screen_bottom = ui.ctx().content_rect().bottom();
            let top = if below + popup_height > screen_bottom - 8.0
                && caret_rect.top() - 4.0 - popup_height > 0.0
            {
                caret_rect.top() - 4.0 - popup_height
            } else {
                below
            };
            let anchor = egui::pos2(
                caret_rect.left().max(output.response.rect.left()),
                top,
            );
            let area = egui::Area::new(Id::unique(("cue_popup_area", doc_id)))
                .order(egui::Order::Foreground)
                .fixed_pos(anchor)
                .show(ui.ctx(), |ui| {
                    Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_width(260.0);
                        ScrollArea::vertical()
                            .max_height(row_height * CUE_POPUP_ROWS)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (index, suggestion) in cue.suggestions.iter().enumerate() {
                                    let response = ui.add(
                                        egui::Button::selectable(
                                            index == cue.selected,
                                            RichText::new(&suggestion.label).font(ui_body()),
                                        )
                                        .min_size(Vec2::new(ui.available_width(), 0.0)),
                                    );
                                    if index == cue.selected && cue.selection_moved {
                                        response.scroll_to_me(None);
                                    }
                                    if response.clicked() {
                                        clicked_cue = Some(index);
                                    }
                                }
                            });
                        cue.selection_moved = false;
                    });
                });
            cue.popup_rect = Some(area.response.rect);
        }

        if let Some(index) = accept_cue.or(clicked_cue) {
            if let Some(suggestion) = cue.suggestions.get(index).cloned() {
                if suggestion.insert == NEW_CHARACTER_SENTINEL {
                    new_character = Some(NewCharacterState {
                        doc_id: doc_id.to_owned(),
                        block: caret_paragraph,
                        name: word.clone(),
                        role: String::new(),
                    });
                } else if let Some(paragraph) = doc.content.paragraphs.get_mut(caret_paragraph) {
                    paragraph.set_plain_text(suggestion.insert.clone());
                    doc.touch();
                    dirty = true;
                    let end = rich_paragraph_start(&doc.content, caret_paragraph)
                        + suggestion.insert.chars().count();
                    let mut state = egui::text_edit::TextEditState::load(ui.ctx(), flow_id)
                        .unwrap_or_default();
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                        egui::text::CCursor::new(end),
                    )));
                    state.store(ui.ctx(), flow_id);
                }
                ui.memory_mut(|memory| memory.request_focus(flow_id));
                cue.dismissed = Some(caret_paragraph);
                cue.visible = false;
                cue.suggestions.clear();
            }
        }
        ui.data_mut(|data| data.insert_temp(cue_key, cue));
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
        crate::ui::find_bar::paint_matches(ui, &app.find_bar, &output.galley, output.galley_pos);
        if output.response.has_focus() {
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
        app.selected_block = caret_paragraph;
        if spell_on {
            let key = Id::unique(("script_spell_menu", doc_id));
            let byte = spell_menu_byte(
                ui.ctx(),
                key,
                &output.response,
                &output.galley,
                output.galley_pos,
                &edit_text,
            );
            output.response.context_menu(|ui| {
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

fn script_paragraph_style(
    content: &portmanteau_core::RichDocument,
    sheet: &portmanteau_core::StyleSheet,
    styles: &portmanteau_core::ElementStyles,
    index: usize,
) -> (ElementKind, portmanteau_core::ElementStyle, bool) {
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
    let uppercase = paragraph_style
        .is_some_and(|style| style.text_transform == portmanteau_core::TextTransform::Uppercase)
        || element_style.uppercase;
    (kind, element_style, uppercase)
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

    remember_link_spans(text, content);
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    let mut byte_start = 0usize;
    let lines = if text.is_empty() {
        vec![""]
    } else {
        text.split_inclusive('\n').collect::<Vec<_>>()
    };
    for (index, line) in lines.into_iter().enumerate() {
        let (_, element_style, _) = script_paragraph_style(content, sheet, styles, index);
        let font_size = element_style.size.max(6.0) * zoom;
        let leading = paragraph_leading(
            &element_style,
            zoom,
            wrap_width,
            line.trim_end_matches('\n').chars().count(),
            font_size,
        );
        append_script_paragraph(
            &mut job, line, index, byte_start, content, sheet, styles, zoom, misspelled, leading,
        );
        byte_start += line.len();
    }
    job
}

/// Lay the screenplay out paragraph by paragraph, each wrapped to its own
/// column (left and right indents), then stack the rows into one galley with
/// the styles' space before/after and alignment. One TextEdit still owns the
/// caret and selection; only the row positions differ from a plain job.
pub(crate) fn script_flow_galley(
    ui: &egui::Ui,
    text: &str,
    content: &portmanteau_core::RichDocument,
    sheet: &portmanteau_core::StyleSheet,
    styles: &portmanteau_core::ElementStyles,
    zoom: f32,
    wrap_width: f32,
    misspelled: &[(usize, usize)],
) -> std::sync::Arc<egui::Galley> {
    use egui::text::LayoutJob;

    let ppp = ui.ctx().pixels_per_point();
    let snap = |v: f32| (v * ppp).round() / ppp;
    let full = std::sync::Arc::new(script_flow_layout_job(
        text, content, sheet, styles, zoom, wrap_width, misspelled,
    ));
    let paragraphs: Vec<&str> = text.split('\n').collect();
    let count = paragraphs.len();
    let mut template: Option<egui::Galley> = None;
    let mut rows: Vec<egui::epaint::text::PlacedRow> = Vec::new();
    let mut y = 0.0f32;
    let mut prev_after = 0.0f32;
    let mut byte_start = 0usize;
    let mut right_edge = wrap_width;
    let mut float: Option<FloatBox> = None;
    for (index, line) in paragraphs.iter().enumerate() {
        let (kind, style, _) = script_paragraph_style(content, sheet, styles, index);
        // "FADE IN:" opens the script flush left rather than as a right-hand cue.
        let flush_left = kind == ElementKind::Transition
            && line.trim_start().to_uppercase().starts_with("FADE IN");
        let indent = style.indent.max(0.0) * zoom;
        let right_indent = style.right_indent.max(0.0) * zoom;
        let column = (wrap_width - indent - right_indent).max(24.0 * zoom);
        if index > 0 {
            y += prev_after + style.before.max(0.0) * zoom;
        }
        // A picture that text runs around does not take a line of its own: it
        // becomes a box the following paragraphs flow beside, like a float in
        // HTML. Its own line is almost no height.
        let wrapped = content.paragraphs.get(index).and_then(wrapped_picture);
        if let Some(image) = wrapped {
            let clear = float.as_ref().map_or(0.0, |f: &FloatBox| f.bottom);
            y = y.max(clear);
            let width = image.width.unwrap_or(240.0).max(12.0) * zoom;
            let height = image.height.unwrap_or(150.0).max(12.0) * zoom;
            let room = if image.caption.trim().is_empty() { 0.0 } else { CAPTION_ROOM * zoom };
            let padding = image.padding.max(0.0) * zoom;
            float = Some(FloatBox {
                on_left: image.align == portmanteau_core::PictureAlign::Left,
                width: width + padding,
                bottom: y + height + room + padding,
            });
        }
        // Rows that start above the bottom of a float run beside it.
        let beside = float.as_ref().filter(|f| f.bottom > y + 0.5 && wrapped.is_none()).cloned();
        let narrow = beside.as_ref().map_or(0.0, |f| f.width);
        let mut job = LayoutJob::default();
        job.wrap.max_width = (column - narrow).max(24.0 * zoom);
        append_script_paragraph(
            &mut job, line, index, byte_start, content, sheet, styles, zoom, misspelled, 0.0,
        );
        byte_start += line.len() + 1;
        let galley = ui.ctx().fonts_mut(|fonts| fonts.layout_job(job.clone()));
        let first_indent = style.first_line_indent.max(0.0) * zoom;
        let align = if flush_left {
            portmanteau_core::Alignment::Left
        } else {
            style.align
        };
        // How many rows sit beside the float; the rest are laid out again at
        // the full width.
        let beside_rows = match &beside {
            Some(f) => galley
                .rows
                .iter()
                .take_while(|placed| y + placed.pos.y < f.bottom)
                .count()
                .max(1),
            None => galley.rows.len(),
        };
        let split = beside_rows < galley.rows.len();
        let mut place = |rows: &mut Vec<egui::epaint::text::PlacedRow>,
                         placed: &egui::epaint::text::PlacedRow,
                         row_index: usize,
                         top: f32,
                         width_available: f32,
                         shift: f32,
                         last_in_paragraph: bool| {
            let mut placed = placed.clone();
            let width = placed.row.size.x;
            let x = match align {
                portmanteau_core::Alignment::Right => {
                    indent + shift + (width_available - width).max(0.0)
                }
                portmanteau_core::Alignment::Center => {
                    indent + shift + ((width_available - width) * 0.5).max(0.0)
                }
                _ => indent + shift + if row_index == 0 { first_indent } else { 0.0 },
            };
            placed.pos = egui::pos2(snap(x), snap(top + placed.pos.y));
            placed.ends_with_newline = index + 1 < count && last_in_paragraph;
            right_edge = right_edge.max(placed.pos.x + width);
            rows.push(placed);
        };
        let shift = match &beside {
            Some(f) if f.on_left => f.width,
            _ => 0.0,
        };
        for (row_index, placed) in galley.rows.iter().take(beside_rows).enumerate() {
            place(&mut rows, placed, row_index, y, column - narrow, shift, !split && row_index + 1 == galley.rows.len());
        }
        let mut height = galley.rect.height();
        if split {
            let head_bottom = galley
                .rows
                .get(beside_rows - 1)
                .map_or(0.0, |r| r.pos.y + r.row.size.y);
            let head_chars: usize = galley
                .rows
                .iter()
                .take(beside_rows)
                .map(|r| r.row.char_count_excluding_newline().0)
                .sum();
            let tail_job = job_from_char(&job, head_chars, column);
            let tail = ui.ctx().fonts_mut(|fonts| fonts.layout_job(tail_job));
            for (row_index, placed) in tail.rows.iter().enumerate() {
                place(&mut rows, placed, row_index + 1, y + head_bottom, column, 0.0, row_index + 1 == tail.rows.len());
            }
            height = head_bottom + tail.rect.height();
        }
        y += height;
        prev_after = style.after.max(0.0) * zoom;
        if template.is_none() {
            template = Some((*galley).clone());
        }
    }
    if let Some(f) = &float {
        y = y.max(f.bottom);
    }
    let mut out = template.unwrap_or_else(|| {
        (*ui.ctx().fonts_mut(|fonts| fonts.layout_job(LayoutJob::default()))).clone()
    });
    let mut bounds = egui::Rect::NOTHING;
    let (mut vertices, mut indices) = (0usize, 0usize);
    for placed in &rows {
        let b = placed.row.visuals.mesh_bounds;
        if b.is_positive() {
            bounds |= b.translate(placed.pos.to_vec2());
        }
        vertices += placed.row.visuals.mesh.vertices.len();
        indices += placed.row.visuals.mesh.indices.len();
    }
    out.rows = rows;
    out.rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(right_edge, y));
    out.mesh_bounds = bounds;
    out.num_vertices = vertices;
    out.num_indices = indices;
    out.job = full;
    std::sync::Arc::new(out)
}

/// The space a wrapped picture takes at one side of the page while the text
/// that follows runs beside it.
#[derive(Clone)]
struct FloatBox {
    on_left: bool,
    /// Picture width plus the space kept around it.
    width: f32,
    /// Where the box ends, down the page.
    bottom: f32,
}

/// The picture of a paragraph that holds one picture and nothing else, when
/// text is set to wrap around it (it sits left or right, never centred).
pub(crate) fn wrapped_picture(paragraph: &portmanteau_core::RichParagraph) -> Option<&portmanteau_core::ImageObject> {
    use portmanteau_core::InlineContent;
    let mut picture = None;
    for item in &paragraph.content {
        match item {
            InlineContent::Image(image) if picture.is_none() => picture = Some(image),
            InlineContent::Text(run) if run.text.trim().is_empty() => {}
            _ => return None,
        }
    }
    picture.filter(|image| image.wrap && image.align != portmanteau_core::PictureAlign::Center)
}

/// The same job from the given character on, at another width: the rest of a
/// paragraph that has passed the bottom of a picture.
fn job_from_char(job: &egui::text::LayoutJob, chars: usize, width: f32) -> egui::text::LayoutJob {
    use egui::epaint::text::ByteRangeExt;
    let start = job.text.char_indices().nth(chars).map_or(job.text.len(), |(byte, _)| byte);
    let mut out = egui::text::LayoutJob::default();
    out.wrap = job.wrap.clone();
    out.wrap.max_width = width;
    for section in &job.sections {
        let range = section.byte_range.as_usize();
        if range.end <= start {
            continue;
        }
        out.append(&job.text[range.start.max(start)..range.end], 0.0, section.format.clone());
    }
    out
}

/// Whether a paragraph style is a heading: the chapter heading, or any style
/// the sheet names as one. The exporters make the same call.
pub(crate) fn is_heading_style(style_id: &str, sheet: &portmanteau_core::StyleSheet) -> bool {
    style_id == "chapter-heading"
        || sheet
            .style(style_id)
            .is_some_and(|style| style.name.to_ascii_lowercase().contains("heading"))
}

fn append_script_paragraph(
    job: &mut egui::text::LayoutJob,
    line: &str,
    index: usize,
    byte_start: usize,
    content: &portmanteau_core::RichDocument,
    sheet: &portmanteau_core::StyleSheet,
    styles: &portmanteau_core::ElementStyles,
    zoom: f32,
    misspelled: &[(usize, usize)],
    leading: f32,
) {
    let (kind, element_style, uppercase) = script_paragraph_style(content, sheet, styles, index);
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
        italics: element_style.italic,
        ..Default::default()
    };
    let display_line = transform_for_display(line, uppercase);
    let paragraph = content.paragraphs.get(index);
    let heading = paragraph.is_some_and(|p| is_heading_style(&p.style_id, sheet));
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
        let marks = paragraph
            .map(|paragraph| inline_marks_at(&paragraph.content, char_index))
            .unwrap_or_default();
        let linked = paragraph.is_some_and(|paragraph| inline_linked_at(&paragraph.content, char_index));
        // An address is not prose: linked text is never flagged as misspelled.
        let misspelled_char = !uppercase
            && !linked
            && misspelled
                .iter()
                .any(|span| span.0 < byte_start + char_end && span.1 > global_byte);
        keys.push((marks, misspelled_char, linked));
    }
    if keys.is_empty() {
        job.append("", leading, format);
        return;
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
        // A heading is set bold, as every export sets it.
        let mut marks = key.0.clone();
        if heading && !marks.contains(&portmanteau_core::InlineMark::Bold) {
            marks.push(portmanteau_core::InlineMark::Bold);
        }
        apply_inline_marks(&mut run_format, &marks);
        if end_char == start_char + 1 {
            if let Some(image) = paragraph
                .and_then(|paragraph| inline_image_at(&paragraph.content, start_char))
            {
                let image_height = image.height.unwrap_or(20.0) * zoom;
                run_format.font_id.size = image_height.max(font_size);
                // Room under a standalone picture for its caption.
                let room = if image.caption.trim().is_empty() { 0.0 } else { CAPTION_ROOM * zoom };
                if paragraph.and_then(wrapped_picture).is_some() {
                    // Text runs beside it, so its own line takes almost no room.
                    run_format.font_id.size = 2.0 * zoom;
                    run_format.line_height = Some(2.0 * zoom);
                } else {
                    run_format.line_height = Some(image_height.max(font_size) + room);
                }
                run_format.color = Color32::TRANSPARENT;
            }
        }
        if key.2 {
            // A link reads as one: blue and underlined.
            run_format.color = LINK_COLOR;
            run_format.underline = Stroke::new(1.0, LINK_COLOR);
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
}

/// Left offset of a paragraph's first line: its indent, or the centring /
/// right-alignment offset for a line of `chars` characters.
fn paragraph_leading(
    style: &portmanteau_core::ElementStyle,
    zoom: f32,
    wrap_width: f32,
    chars: usize,
    font_size: f32,
) -> f32 {
    let measured_width = chars as f32 * font_size * 0.6;
    let aligned_indent = match style.align {
        portmanteau_core::Alignment::Left | portmanteau_core::Alignment::Justify => 0.0,
        portmanteau_core::Alignment::Center => (wrap_width - measured_width) * 0.5,
        portmanteau_core::Alignment::Right => wrap_width - measured_width,
    };
    (style.indent + style.first_line_indent)
        .max(aligned_indent)
        .max(0.0)
        * zoom
}

fn focused_after_edit(output: &egui::text_edit::TextEditOutput) -> bool {
    output.response.has_focus()
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

/// The colour links are drawn in, in every rich-text field.
const LINK_COLOR: Color32 = Color32::from_rgb(40, 96, 168);

fn inline_linked_at(content: &[portmanteau_core::InlineContent], char_index: usize) -> bool {
    let mut offset = 0usize;
    for item in content {
        match item {
            portmanteau_core::InlineContent::Text(run) => {
                let length = run.text.chars().count();
                if char_index < offset + length {
                    return run.link.is_some();
                }
                offset += length;
            }
            portmanteau_core::InlineContent::HardBreak
            | portmanteau_core::InlineContent::Image(_) => offset += 1,
        }
    }
    false
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
        // The bold face of the family the text is already in (a serif stays a
        // serif); these families are registered in `theme::install`.
        let family = match &format.font_id.family {
            egui::FontFamily::Monospace => "page-mono-bold",
            egui::FontFamily::Name(name) if name.contains("sans") => "page-sans-bold",
            _ => "page-serif-bold",
        };
        format.font_id.family = egui::FontFamily::Name(family.into());
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

/// A tiny anchor: a ring, a stem, a crossbar and two curved flukes.
fn paint_anchor(painter: &egui::Painter, center: egui::Pos2, size: f32, color: Color32) {
    let stroke = Stroke::new(1.3, color);
    let top = center - Vec2::new(0.0, size);
    let bottom = center + Vec2::new(0.0, size);
    painter.circle_stroke(top + Vec2::new(0.0, size * 0.25), size * 0.25, stroke);
    painter.line_segment([top + Vec2::new(0.0, size * 0.5), bottom], stroke);
    painter.line_segment(
        [center - Vec2::new(size * 0.4, size * 0.4), center + Vec2::new(size * 0.4, -size * 0.4)],
        stroke,
    );
    let fluke = |sign: f32| {
        [
            bottom,
            bottom + Vec2::new(sign * size * 0.35, -size * 0.15),
            bottom + Vec2::new(sign * size * 0.6, -size * 0.5),
        ]
    };
    painter.add(egui::Shape::line(fluke(-1.0).to_vec(), stroke));
    painter.add(egui::Shape::line(fluke(1.0).to_vec(), stroke));
}

/// Height kept under a picture for its caption line, in points before zoom.
const CAPTION_ROOM: f32 = 26.0;

/// Draws every picture over the text. Returns the paragraph number and screen
/// rectangle of each picture that sits alone on its line, for hit-testing
/// (the right-click menu).
pub(crate) fn paint_rich_images(
    ui: &egui::Ui,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    widget_rect: egui::Rect,
    content: &portmanteau_core::RichDocument,
    textures: &HashMap<String, egui::TextureHandle>,
    zoom: f32,
) -> Vec<(usize, egui::Rect)> {
    let mut hits = Vec::new();
    let mut char_offset = 0usize;
    for (paragraph_index, paragraph) in content.paragraphs.iter().enumerate() {
        for item in &paragraph.content {
            match item {
                portmanteau_core::InlineContent::Text(run) => {
                    char_offset += run.text.chars().count();
                }
                portmanteau_core::InlineContent::HardBreak => char_offset += 1,
                portmanteau_core::InlineContent::Image(image) => {
                    let cursor = galley.pos_from_cursor(egui::text::CCursor::new(char_offset));
                    let mut center = galley_pos + cursor.center().to_vec2();
                    let standalone = paragraph.style_id == "image" || wrapped_picture(paragraph).is_some();
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
                    if standalone {
                        let margin = (galley_pos.x - widget_rect.left()).max(0.0);
                        center.x = match image.align {
                            portmanteau_core::PictureAlign::Left => galley_pos.x + width * 0.5,
                            portmanteau_core::PictureAlign::Center => widget_rect.center().x,
                            portmanteau_core::PictureAlign::Right => {
                                widget_rect.right() - margin - width * 0.5
                            }
                        };
                    }
                    let has_caption = standalone && !image.caption.trim().is_empty();
                    let floating = wrapped_picture(paragraph).is_some();
                    if has_caption || floating {
                        // The line is taller than the picture; sit it at the top.
                        center.y = galley_pos.y + cursor.top() + height * 0.5;
                    }
                    let rect = egui::Rect::from_center_size(center, Vec2::new(width, height));
                    if standalone {
                        hits.push((paragraph_index, rect));
                        // A small anchor in the margin shows where the picture is
                        // tied to the text; delete that spot and the picture goes.
                        let size = 7.0 * zoom;
                        let at = egui::pos2(galley_pos.x - 12.0 * zoom, rect.top() + 9.0 * zoom);
                        let spot = egui::Rect::from_center_size(at, Vec2::splat(size * 2.4));
                        let response = ui
                            .interact(spot, egui::Id::unique(("picture_anchor", paragraph_index)), egui::Sense::hover())
                            .on_hover_text("This picture is anchored to the text here. Right-click the picture for its settings.");
                        let color = if response.hovered() { Color32::from_rgb(70, 120, 110) } else { PAPER_MUTED };
                        paint_anchor(ui.painter(), at, size, color);
                    }
                    if has_caption {
                        ui.painter().text(
                            egui::pos2(rect.center().x, rect.bottom() + 4.0 * zoom),
                            egui::Align2::CENTER_TOP,
                            image.caption.trim(),
                            serif(12.0 * zoom),
                            PAPER_MUTED,
                        );
                    }
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
    hits
}

/// After a paste into a rich-text field, turn the web addresses it brought in
/// into links. Only the pasted stretch (which ends at the caret) is looked at,
/// so a plain address typed earlier stays as the writer left it.
pub(crate) fn autolink_pasted(
    ctx: &egui::Context,
    content: &mut portmanteau_core::RichDocument,
    caret: Option<usize>,
) {
    let pasted: usize = ctx.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Paste(text) => Some(text.chars().count()),
                _ => None,
            })
            .sum()
    });
    if pasted == 0 {
        return;
    }
    let end = caret.unwrap_or_else(|| content.char_count());
    content.autolink_urls_in(Some(end.saturating_sub(pasted)..end));
}

/// Note that this widget is a rich-text field the app's own undo covers.
/// Called by every such field each frame it is drawn.
pub(crate) fn mark_rich(ctx: &egui::Context, id: Id) {
    ctx.data_mut(|data| {
        let known = data.get_temp_mut_or_default::<std::collections::HashSet<Id>>(Id::unique("rich-fields"));
        known.insert(id);
    });
}

/// Whether `id` is a rich-text field (see `mark_rich`).
pub(crate) fn is_rich_field(ctx: &egui::Context, id: Id) -> bool {
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<std::collections::HashSet<Id>>(Id::unique("rich-fields"))
            .contains(&id)
    })
}

/// A request from the Paged view for this document: put the caret where the
/// writer clicked and let the text box take focus. Returns the character to
/// bring into view once the text is laid out.
pub(crate) fn take_caret_request(
    request: &mut Option<crate::app::CaretRequest>,
    ui: &egui::Ui,
    doc_id: &str,
    content: &portmanteau_core::RichDocument,
    text_id: Id,
) -> Option<usize> {
    let request = request.take_if(|request| request.doc_id == doc_id)?;
    let length = content
        .paragraphs
        .get(request.paragraph)
        .map(|paragraph| paragraph.editor_text().chars().count())
        .unwrap_or(0);
    let caret = rich_paragraph_start(content, request.paragraph) + request.offset.min(length);
    let selected_to = caret + request.select.min(length.saturating_sub(request.offset.min(length)));
    if request.focus {
        ui.memory_mut(|memory| memory.request_focus(text_id));
    }
    let mut state = egui::text_edit::TextEditState::load(ui.ctx(), text_id).unwrap_or_default();
    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
        egui::text::CCursor::new(caret),
        egui::text::CCursor::new(selected_to),
    )));
    state.store(ui.ctx(), text_id);
    Some(caret)
}

/// Scroll the character a caret request chose into the middle of the view.
pub(crate) fn reveal_caret(ui: &egui::Ui, output: &egui::text_edit::TextEditOutput, caret: Option<usize>) {
    let Some(caret) = caret else { return };
    let rect = output
        .galley
        .pos_from_cursor(egui::text::CCursor::new(caret))
        .translate(output.galley_pos.to_vec2());
    ui.scroll_to_rect(rect.expand2(Vec2::new(0.0, 60.0)), Some(Align::Center));
}

pub(crate) fn rich_paragraph_start(content: &portmanteau_core::RichDocument, index: usize) -> usize {
    content
        .paragraphs
        .iter()
        .take(index)
        .map(|paragraph| paragraph.editor_text().chars().count() + 1)
        .sum()
}

pub(crate) struct InlineToolbarResult {
    pub changed: bool,
}

#[derive(Clone, Copy, Default)]
struct SelectionAnchor {
    rect: Option<egui::Rect>,
    focused: bool,
    popup_hovered: bool,
}

thread_local! {
    /// The link spans of the text each layouter was last asked to lay out,
    /// keyed by a hash of that text, so a click on the finished galley can
    /// tell whether it landed on a link.
    static LINK_SPANS: std::cell::RefCell<std::collections::HashMap<u64, Vec<(std::ops::Range<usize>, String)>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn text_key(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn remember_link_spans(text: &str, content: &portmanteau_core::RichDocument) {
    let spans = content.links();
    LINK_SPANS.with(|spans_by_text| {
        let mut map = spans_by_text.borrow_mut();
        if map.len() > 64 {
            map.clear();
        }
        if spans.is_empty() {
            map.remove(&text_key(text));
        } else {
            map.insert(text_key(text), spans);
        }
    });
}

/// Ctrl/Cmd+click on a link opens it: the target is left for `app::draw` to
/// act on (see `take_clicked_link`). While the modifier is held the pointer
/// shows it is a link.
fn handle_link_click(ctx: &egui::Context, field: Id, galley: &egui::Galley, galley_pos: egui::Pos2) {
    let Some(pointer) = ctx.pointer_hover_pos() else { return };
    let spans = LINK_SPANS.with(|map| map.borrow().get(&text_key(&galley.job.text)).cloned());
    let Some(spans) = spans else { return };
    let local = pointer - galley_pos.to_vec2();
    if !galley.rect.expand(2.0).contains(local) {
        return;
    }
    let index = galley.cursor_from_pos(local.to_vec2()).index.0;
    let Some((range, link)) = spans.iter().find(|(range, _)| range.contains(&index)) else { return };
    // Right-click on a link: a small menu with Edit link… and Open link.
    if ctx.input(|i| i.pointer.secondary_clicked()) {
        ctx.data_mut(|data| data.insert_temp(Id::unique(("link_menu", field)), LinkMenu { pos: pointer, range: range.clone(), link: link.clone() }));
    }
    if ctx.input(|i| i.modifiers.command) {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        if ctx.input(|i| i.pointer.primary_clicked()) {
            ctx.data_mut(|data| data.insert_temp(Id::unique("clicked-link"), link.clone()));
        }
    }
}

/// The little menu a right-click on a link opens.
#[derive(Clone)]
struct LinkMenu {
    pos: egui::Pos2,
    range: std::ops::Range<usize>,
    link: String,
}

/// The link a right-click landed on in `text` (the field's editor text), as a
/// character range and its target, found from what the layout registered.
fn link_in_text(text: &str, byte: usize) -> Option<(std::ops::Range<usize>, String)> {
    let index = text.get(..byte.min(text.len()))?.chars().count();
    LINK_SPANS.with(|map| {
        map.borrow()
            .get(&text_key(text))?
            .iter()
            .find(|(range, _)| range.contains(&index))
            .cloned()
    })
}

/// Ask whichever field holds this link to open its edit dialog. The field is
/// found by the link itself (`link_support` checks its own content), so the
/// menu does not need to know which field it was opened from.
fn request_link_edit(ctx: &egui::Context, range: std::ops::Range<usize>, url: String) {
    let edit = LinkEdit { range, url, existing: true, first_frame: true };
    ctx.data_mut(|data| data.insert_temp(Id::unique("link-edit-request"), edit));
}

/// What a right-click on a link offers, shared by the spelling menu and the
/// plain link menu. Returns whether the writer chose something (close the menu).
fn link_actions_ui(ui: &mut egui::Ui, range: std::ops::Range<usize>, link: &str) -> bool {
    ui.set_min_width(150.0);
    let shown: String = link.chars().take(38).collect();
    ui.label(RichText::new(if shown.len() < link.len() { format!("{shown}…") } else { shown }).font(ui_small()).color(FAINT));
    let mut chosen = false;
    if ui.button("Edit link…").clicked() {
        request_link_edit(ui.ctx(), range, link.to_string());
        chosen = true;
    }
    if ui.button("Open link").clicked() {
        ui.ctx().data_mut(|data| data.insert_temp(Id::unique("clicked-link"), link.to_string()));
        chosen = true;
    }
    if chosen {
        ui.close();
    }
    chosen
}

/// Show the right-click menu for this field's link, if one is open. "Edit
/// link…" opens the same dialog as making a link (see `inline_format_toolbar`).
fn link_menu(ctx: &egui::Context, field: Id) {
    let key = Id::unique(("link_menu", field));
    let Some(menu) = ctx.data(|data| data.get_temp::<LinkMenu>(key)) else { return };
    // With spell-checking on, the field's own right-click menu already carries
    // the link actions (see `spell_menu_ui`); two menus would fight.
    if ctx.data(|data| data.get_temp::<bool>(Id::unique("spell-on"))).unwrap_or(false) {
        ctx.data_mut(|data| data.remove::<LinkMenu>(key));
        return;
    }
    let mut close = false;
    let area = egui::Area::new(Id::unique(("link_menu_area", field)))
        .order(egui::Order::Foreground)
        .fixed_pos(menu.pos)
        .show(ctx, |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                close = link_actions_ui(ui, menu.range.clone(), &menu.link);
            });
        });
    let clicked_elsewhere = ctx.input(|i| i.pointer.any_pressed())
        && ctx.pointer_interact_pos().is_some_and(|p| !area.response.rect.contains(p));
    if close || clicked_elsewhere || ctx.input(|i| i.key_pressed(Key::Escape)) {
        ctx.data_mut(|data| data.remove::<LinkMenu>(key));
    }
}

/// The link the writer Ctrl/Cmd-clicked this frame, if any.
pub(crate) fn take_clicked_link(ctx: &egui::Context) -> Option<String> {
    ctx.data_mut(|data| data.remove_temp::<String>(Id::unique("clicked-link")))
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
    handle_link_click(ctx, id, galley, galley_pos);
    link_menu(ctx, id);
    // Every rich-text field reports its selection here, which is also how the
    // app knows Cmd+Z is its own to handle while such a field has the focus.
    mark_rich(ctx, id);
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
    let caret_range = egui::text_edit::TextEditState::load(ctx, id).and_then(|state| state.cursor.char_range());
    let selection = caret_range
        .map(|range| {
            range.primary.index.0.min(range.secondary.index.0)
                ..range.primary.index.0.max(range.secondary.index.0)
        })
        .filter(|range| range.start < range.end);

    changed |= link_support(ctx, id, content);
    let link_key = Id::unique(("link_edit", id));
    let mut start_link: Option<std::ops::Range<usize>> = None;
    let visible = selection.is_some() && (anchor.focused || anchor.popup_hovered);
    let (Some(rect), true) = (anchor.rect, visible) else {
        anchor.popup_hovered = false;
        ctx.data_mut(|data| data.insert_temp(key, anchor));
        return InlineToolbarResult { changed };
    };
    let screen = ctx.content_rect();
    let width = 4.0 * 30.0 + 58.0;
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
                        let link_button = egui::Button::new(RichText::new("Link").font(ui_small()).color(LINK_COLOR))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(5))
                            .min_size(Vec2::new(40.0, 26.0));
                        if ui.add(link_button).on_hover_text("Make the selection a link (Ctrl/Cmd+K)").clicked() {
                            start_link = selection.clone();
                        }
                    });
                });
        });
    anchor.popup_hovered = ctx
        .pointer_hover_pos()
        .is_some_and(|pos| area.response.rect.expand(4.0).contains(pos));
    ctx.data_mut(|data| data.insert_temp(key, anchor));
    if let Some(range) = start_link {
        ctx.data_mut(|data| data.insert_temp(link_key, LinkEdit::for_range(content, range)));
    }
    InlineToolbarResult { changed }
}

/// Everything about links that belongs to a field rather than to one kind of
/// toolbar: Ctrl/Cmd+K, an edit requested from a right-click menu, and the
/// dialog itself. Both toolbars call this once per frame for their field.
/// Returns whether the dialog changed the text.
pub(crate) fn link_support(ctx: &egui::Context, id: Id, content: &mut portmanteau_core::RichDocument) -> bool {
    let mut changed = false;
    let caret_range = egui::text_edit::TextEditState::load(ctx, id).and_then(|state| state.cursor.char_range());
    let selection = caret_range
        .map(|range| {
            range.primary.index.0.min(range.secondary.index.0)..range.primary.index.0.max(range.secondary.index.0)
        })
        .filter(|range| range.start < range.end);
    let link_key = Id::unique(("link_edit", id));
    let mut link_edit: Option<LinkEdit> = ctx.data(|data| data.get_temp(link_key));
    if ctx.memory(|memory| memory.has_focus(id))
        && ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::K))
    {
        let caret = caret_range.map_or(0, |range| range.primary.index.0);
        link_edit = match &selection {
            Some(range) => Some(LinkEdit::for_range(content, range.clone())),
            // No selection: edit the link the caret is in, if it is in one.
            None => content.link_at(caret, true).map(|(_, range)| LinkEdit::for_range(content, range)),
        };
    }
    // An edit asked for by a right-click menu belongs to the field that holds
    // that very link.
    let requested = ctx.data(|data| data.get_temp::<LinkEdit>(Id::unique("link-edit-request")));
    if let Some(request) = requested {
        if content.link_at(request.range.start, false).is_some_and(|(url, range)| url == request.url && range == request.range) {
            ctx.data_mut(|data| data.remove::<LinkEdit>(Id::unique("link-edit-request")));
            link_edit = Some(request);
        }
    }
    if let Some(edit) = link_edit.take() {
        match link_editor_window(ctx, id, edit, content) {
            LinkOutcome::Open(edit) => link_edit = Some(edit),
            LinkOutcome::Changed => changed = true,
            LinkOutcome::Closed => {}
        }
    }
    ctx.data_mut(|data| match &link_edit {
        Some(edit) => {
            data.insert_temp(link_key, edit.clone());
        }
        None => data.remove::<LinkEdit>(link_key),
    });
    changed
}

/// Open the link dialog for `range` of this field's text (the toolbars' Link
/// button). The dialog itself is shown by `link_support`.
pub(crate) fn begin_link_edit(
    ctx: &egui::Context,
    field: Id,
    content: &portmanteau_core::RichDocument,
    range: std::ops::Range<usize>,
) {
    ctx.data_mut(|data| data.insert_temp(Id::unique(("link_edit", field)), LinkEdit::for_range(content, range)));
}

/// A link being made or changed: the characters it covers and its address.
#[derive(Clone, Debug)]
struct LinkEdit {
    range: std::ops::Range<usize>,
    url: String,
    /// The text already links somewhere (so "Remove link" is offered).
    existing: bool,
    first_frame: bool,
}

impl LinkEdit {
    fn for_range(content: &portmanteau_core::RichDocument, range: std::ops::Range<usize>) -> Self {
        let existing = content.link_at(range.start, false).map(|(url, _)| url);
        Self { range, existing: existing.is_some(), url: existing.unwrap_or_default(), first_frame: true }
    }
}

enum LinkOutcome {
    Open(LinkEdit),
    Changed,
    Closed,
}

/// What the writer typed, made into something a link can point at: a bare
/// domain gets `https://`; an address with no recognised scheme (an email
/// address included: email links are not supported) is refused.
pub(crate) fn normalise_link(typed: &str) -> Option<String> {
    let typed = typed.trim();
    if typed.is_empty() || typed.chars().any(char::is_whitespace) {
        return None;
    }
    let lower = typed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some(typed.to_string());
    }
    if lower.starts_with(portmanteau_core::rich_text::NOTE_LINK) || lower.starts_with(crate::ui::mention::ITEM_LINK) {
        return Some(typed.to_string());
    }
    if typed.contains("://") || typed.contains(':') && !typed.contains('.') {
        return None; // some other scheme (file:, javascript:, …)
    }
    if typed.contains('@') {
        return None;
    }
    typed.contains('.').then(|| format!("https://{typed}"))
}

fn link_editor_window(
    ctx: &egui::Context,
    id: Id,
    mut edit: LinkEdit,
    content: &mut portmanteau_core::RichDocument,
) -> LinkOutcome {
    let targets = crate::ui::mention::published_targets(ctx);
    let mut outcome = None;
    let mut open = true;
    egui::Window::new("Link")
        .id(Id::unique(("link_window", id)))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .min_width(420.0)
        .show(ctx, |ui| {
            // A link to something in the project (made with @) shows what it
            // points at; the writer can swap it for a web address.
            let internal = edit.url.starts_with(portmanteau_core::rich_text::NOTE_LINK)
                || edit.url.starts_with(crate::ui::mention::ITEM_LINK);
            let mut field = None;
            if internal {
                let what = targets
                    .iter()
                    .find(|t| t.link == edit.url)
                    .map_or("something that no longer exists".to_string(), |t| format!("{} ({})", t.title, t.group));
                ui.label(RichText::new(format!("Links to {what}")).font(ui_body()).color(TEXT));
                ui.label(RichText::new("Type @ in the text to link to anything in the project.").font(ui_small()).color(FAINT));
                if ui.button("Use a web address instead").clicked() {
                    edit.url.clear();
                    edit.first_frame = true;
                }
            } else {
                ui.label(RichText::new("Address").font(ui_small()).color(FAINT));
                let response = ui.add(
                    TextEdit::singleline(&mut edit.url)
                        .hint_text("https://…  (type @ in the text to link to something in the project)")
                        .font(mono(10.5))
                        .desired_width(f32::INFINITY),
                );
                if edit.first_frame {
                    response.request_focus();
                    edit.first_frame = false;
                }
                field = Some(response);
            }
            ui.add_space(6.0);
            let normalised = if internal { None } else { normalise_link(&edit.url) };
            if !internal && !edit.url.trim().is_empty() && normalised.is_none() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Use a web address (https://…).")
                        .font(ui_small())
                        .color(RED_INK),
                );
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let apply = ui.add_enabled(normalised.is_some(), egui::Button::new("Apply link"));
                let enter = field.as_ref().is_some_and(|f| f.lost_focus()) && ui.input(|i| i.key_pressed(Key::Enter));
                if (apply.clicked() || (enter && normalised.is_some())) && normalised.is_some() {
                    let changed = content.set_link(edit.range.clone(), normalised.as_deref());
                    outcome = Some(if changed { LinkOutcome::Changed } else { LinkOutcome::Closed });
                }
                if edit.existing && ui.button("Remove link").clicked() {
                    let changed = content.set_link(edit.range.clone(), None);
                    outcome = Some(if changed { LinkOutcome::Changed } else { LinkOutcome::Closed });
                }
                if ui.button("Cancel").clicked() {
                    outcome = Some(LinkOutcome::Closed);
                }
            });
        });
    if !open {
        return LinkOutcome::Closed;
    }
    outcome.unwrap_or(LinkOutcome::Open(edit))
}

/// Transitional reference implementation retained while the flow editor is
/// built out. The live editor above uses one shared TextEdit for the entire
/// screenplay document.
#[allow(dead_code)]
fn legacy_script_page(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    // The page count the exporter will produce; recomputed a moment after typing
    // pauses, never on every keystroke.
    let pages = crate::ui::preview::page_counts(ui, &app.ws.project).label(doc_id);
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
        paper_header(ui, &project_title, doc.medium, &doc.status, doc.words(), &pages);
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

                let style = styles.style_for_kind(kind);
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
                    RichText::new(format!("{pages} pages"))
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
#[derive(Clone, Default)]
struct CueSuggestion {
    insert: String,
    label: String,
}

/// Per-document state of the character-cue popup.
#[derive(Clone, Default)]
struct CueState {
    /// Paragraph the popup is working on, once the writer has typed in a cue
    /// or entered an empty one.
    armed: Option<usize>,
    dismissed: Option<usize>,
    last_paragraph: Option<usize>,
    last_word: String,
    selected: usize,
    visible: bool,
    suggestions: Vec<CueSuggestion>,
    popup_rect: Option<egui::Rect>,
    last_caret: Option<usize>,
    selection_moved: bool,
}

/// Rows of the cue popup visible before it scrolls.
const CUE_POPUP_ROWS: f32 = 4.0;
/// Space kept clear below the line being typed (about two inches).
const CARET_BOTTOM_CLEARANCE: f32 = 200.0;

/// Scroll the writing surface so the caret line keeps a couple of inches of
/// paper below it, instead of sinking to the window's edge as text is added.
fn keep_caret_clear_of_bottom(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    caret: usize,
) {
    let rect = output
        .galley
        .pos_from_cursor(egui::text::CCursor::new(caret))
        .translate(output.galley_pos.to_vec2());
    ui.scroll_to_rect(
        egui::Rect::from_min_max(
            rect.left_top(),
            egui::pos2(rect.right(), rect.bottom() + CARET_BOTTOM_CLEARANCE),
        ),
        None,
    );
}

/// Names to offer for the cue at `paragraph`. An empty cue lists the other
/// half of the current exchange first, then recent speakers, then the cast; a
/// partly typed one filters the cast, and an unknown name offers to create a
/// character. A complete, known name offers nothing, so Enter moves on.
fn cue_suggestions(
    blocks: &[portmanteau_core::Block],
    paragraph: usize,
    word: &str,
    cast: &[String],
) -> Vec<CueSuggestion> {
    // Script cues are often short forms ("MARA", "BOY") of a database
    // character ("Mara Venn", "The Boy"). Resolve them to the database name
    // when exactly one character fits, so the list shows one entry per person.
    let canonical = |name: &str| {
        if let Some(known) = cast.iter().find(|known| known.eq_ignore_ascii_case(name)) {
            return known.clone();
        }
        let needle = name.trim().to_lowercase();
        let fits: Vec<&String> = cast
            .iter()
            .filter(|known| {
                known
                    .to_lowercase()
                    .split_whitespace()
                    .any(|part| part == needle)
            })
            .collect();
        match fits.as_slice() {
            [only] => (*only).clone(),
            _ => name.to_owned(),
        }
    };
    if word.is_empty() {
        let mut names: Vec<String> = Vec::new();
        let push = |name: String, names: &mut Vec<String>| {
            if !name.trim().is_empty()
                && !names.iter().any(|seen| seen.eq_ignore_ascii_case(&name))
            {
                names.push(name);
            }
        };
        let mut result = Vec::new();
        if let Some(speaker) = next_speaker(blocks, paragraph) {
            push(canonical(&speaker), &mut names);
        }
        for block in blocks[..paragraph.min(blocks.len())].iter().rev() {
            if block.kind == ElementKind::Character {
                push(canonical(block.text.trim()), &mut names);
            }
        }
        for name in cast {
            push(name.clone(), &mut names);
        }
        for (index, name) in names.into_iter().take(8).enumerate() {
            result.push(CueSuggestion {
                label: if index == 0 { format!("↺ {name}") } else { name.clone() },
                insert: name,
            });
        }
        return result;
    }
    let needle = word.to_lowercase();
    if cast.iter().any(|name| name.to_lowercase() == needle) {
        return Vec::new();
    }
    let matches: Vec<CueSuggestion> = cast
        .iter()
        .filter(|name| name.to_lowercase().contains(&needle))
        .map(|name| CueSuggestion {
            insert: name.clone(),
            label: name.clone(),
        })
        .collect();
    if matches.is_empty() {
        vec![CueSuggestion {
            insert: NEW_CHARACTER_SENTINEL.to_owned(),
            label: "+ New character…".to_owned(),
        }]
    } else {
        matches
    }
}

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
    pages: &str,
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
mod link_tests {
    use super::*;

    #[test]
    fn typed_addresses_become_safe_links_or_are_refused() {
        assert_eq!(normalise_link(" https://example.org/x "), Some("https://example.org/x".into()));
        assert_eq!(normalise_link("example.org/page"), Some("https://example.org/page".into()));
        assert_eq!(normalise_link("note:doc-123"), Some("note:doc-123".into()));
        for refused in ["", "ada@example.org", "mailto:ada@example.org", "javascript:alert(1)", "file:///etc/passwd", "two words.org", "ftp://x.test", "localhost"] {
            assert_eq!(normalise_link(refused), None, "{refused:?}");
        }
    }

    /// A heading is set bold, in the same family as the text around it.
    #[test]
    fn headings_are_bold_and_bold_keeps_the_serif() {
        let styles = portmanteau_core::ElementStyles::default();
        let sheet = portmanteau_core::StyleSheet::novel_prose(&styles);
        let mut content = portmanteau_core::RichDocument::from_plain_text("A heading\nBody text here", "body");
        content.set_paragraph_style(0..0, "chapter-heading");
        assert!(is_heading_style("chapter-heading", &sheet) && !is_heading_style("body", &sheet));
        let text = content.editor_text();
        let job = script_flow_layout_job(&text, &content, &sheet, &styles, 1.0, 400.0, &[]);
        let family_of = |needle: &str| {
            job.sections
                .iter()
                .find(|s| job.text[s.byte_range.start.0..s.byte_range.end.0].contains(needle))
                .map(|s| s.format.font_id.family.clone())
                .unwrap()
        };
        assert_eq!(family_of("A heading"), egui::FontFamily::Name("page-serif-bold".into()));
        assert_ne!(family_of("Body text"), egui::FontFamily::Name("page-serif-bold".into()));
        // A Bold mark in prose no longer turns the text into sans.
        let mut format = egui::TextFormat { font_id: FontId::new(14.0, egui::FontFamily::Name("portmanteau-serif".into())), ..Default::default() };
        apply_inline_marks(&mut format, &[portmanteau_core::InlineMark::Bold]);
        assert_eq!(format.font_id.family, egui::FontFamily::Name("page-serif-bold".into()));
    }

    #[test]
    fn linked_text_is_not_spell_checked() {
        let mut content = portmanteau_core::RichDocument::from_plain_text("see wrrodd and wrrodd", "body");
        content.set_link(4..10, Some("https://example.org"));
        let styles = portmanteau_core::ElementStyles::default();
        let sheet = portmanteau_core::StyleSheet::novel_prose(&styles);
        let text = content.editor_text();
        // Both words are "misspelled"; only the unlinked one may be underlined.
        let misspelled = [(4, 10), (15, 21)];
        let job = script_flow_layout_job(&text, &content, &sheet, &styles, 1.0, 400.0, &misspelled);
        let underlined: Vec<&str> = job
            .sections
            .iter()
            .filter(|s| s.format.underline.color == Color32::from_rgb(196, 64, 64))
            .map(|s| &job.text[s.byte_range.start.0..s.byte_range.end.0])
            .collect();
        assert_eq!(underlined, ["wrrodd"]);
        assert_eq!(job.text[15..21].to_string(), "wrrodd");
    }

    #[test]
    fn right_clicking_a_link_offers_edit_link_which_opens_the_same_dialog() {
        use egui_kittest::kittest::Queryable;
        let field = Id::unique("some-field");
        let mut harness = egui_kittest::Harness::builder()
            .with_size(Vec2::new(400.0, 300.0))
            .with_accessibility_check(false)
            .build_ui({
                let mut themed = false;
                move |ui| {
                    if !themed {
                        crate::theme::install(ui.ctx());
                        themed = true;
                        return;
                    }
                    link_menu(ui.ctx(), field)
                }
            });
        // With spell-checking off this plain menu is the only one.
        harness.run_steps(1);
        harness.ctx.data_mut(|data| data.insert_temp(Id::unique("spell-on"), false));
        harness.ctx.data_mut(|data| {
            data.insert_temp(
                Id::unique(("link_menu", field)),
                LinkMenu { pos: egui::pos2(40.0, 40.0), range: 4..13, link: "https://example.org".into() },
            )
        });
        harness.run_steps(2);
        harness.get_by_label("Edit link…").click();
        harness.run_steps(2);
        let edit: LinkEdit = harness
            .ctx
            .data(|data| data.get_temp(Id::unique("link-edit-request")))
            .expect("the edit dialog was requested");
        assert_eq!((edit.range, edit.url.as_str(), edit.existing), (4..13, "https://example.org", true));
        assert!(harness.query_by_label("Edit link…").is_none(), "the menu closed");
    }

    /// The spelling menu is the field's one right-click menu when spell-checking
    /// is on: on a link it offers the link actions and never a spelling suggestion.
    #[test]
    fn the_spelling_menu_on_a_link_offers_link_actions_not_corrections() {
        use egui_kittest::kittest::Queryable;
        let ctx_content = {
            let mut c = portmanteau_core::RichDocument::from_plain_text("go to https://pitpal.example.net/ now", "body");
            c.set_link(6..33, Some("https://pitpal.example.net/"));
            c
        };
        let styles = portmanteau_core::ElementStyles::default();
        let sheet = portmanteau_core::StyleSheet::novel_prose(&styles);
        let text = ctx_content.editor_text();
        let _ = script_flow_layout_job(&text, &ctx_content, &sheet, &styles, 1.0, 400.0, &[]);
        let checker = crate::spellcheck::load("en_US");
        let (t, c) = (text.clone(), std::rc::Rc::new(checker));
        let mut harness = egui_kittest::Harness::builder()
            .with_size(Vec2::new(400.0, 300.0))
            .with_accessibility_check(false)
            .build_ui({
                let mut themed = false;
                move |ui| {
                    if !themed {
                        crate::theme::install(ui.ctx());
                        themed = true;
                        return;
                    }
                    // The click landed inside "pitpal".
                    let _ = spell_menu_ui(ui, &c, &t, 12);
                }
            });
        harness.run_steps(2);
        assert!(harness.query_by_label("Edit link…").is_some());
        assert!(harness.query_by_label("Open link").is_some());
        assert!(harness.query_by_label_contains("Correct").is_none(), "no spelling help for an address");
        // Off the link, ordinary words still get the spelling menu.
        let (t2, c2) = (text.clone(), std::rc::Rc::new(crate::spellcheck::load("en_US")));
        let mut other = egui_kittest::Harness::builder()
            .with_size(Vec2::new(400.0, 300.0))
            .with_accessibility_check(false)
            .build_ui({
                let mut themed = false;
                move |ui| {
                    if !themed {
                        crate::theme::install(ui.ctx());
                        themed = true;
                        return;
                    }
                    let _ = spell_menu_ui(ui, &c2, &t2, 1);
                }
            });
        other.run_steps(2);
        assert!(other.query_by_label("Edit link…").is_none());
    }

    #[test]
    fn a_paste_links_the_addresses_it_brought_and_nothing_else() {
        let ctx = egui::Context::default();
        let pasted = "see https://example.org/map now";
        let mut content = portmanteau_core::RichDocument::from_plain_text(
            &format!("typed https://typed.test earlier. {pasted}"),
            "body",
        );
        let caret = content.char_count();
        let input = egui::RawInput { events: vec![egui::Event::Paste(pasted.into())], ..Default::default() };
        ctx.run_ui(input, |ui| {
            autolink_pasted(ui.ctx(), &mut content, Some(caret));
        })
        .drop_without_applying_deltas();
        let linked: Vec<_> = content.links().into_iter().map(|(_, l)| l).collect();
        assert_eq!(linked, ["https://example.org/map"], "the earlier typed address is left alone");
        // With no paste this frame nothing is linked.
        let mut other = portmanteau_core::RichDocument::from_plain_text("https://plain.test", "body");
        ctx.run_ui(egui::RawInput::default(), |ui| autolink_pasted(ui.ctx(), &mut other, None))
            .drop_without_applying_deltas();
        assert!(other.links().is_empty());
    }

    #[test]
    fn linked_text_is_drawn_as_a_link_and_a_ctrl_click_is_noted() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut content = portmanteau_core::RichDocument::from_plain_text("See the chart now.", "body");
        content.set_link(4..13, Some("https://example.org"));
        let styles = portmanteau_core::ElementStyles::default();
        let sheet = portmanteau_core::StyleSheet::novel_prose(&styles);
        let text = content.editor_text();
        let job = script_flow_layout_job(&text, &content, &sheet, &styles, 1.0, 400.0, &[]);
        let linked: Vec<_> = job.sections.iter().filter(|s| s.format.color == LINK_COLOR).collect();
        assert_eq!(linked.len(), 1, "exactly the linked words are link-coloured");
        assert_eq!(&job.text[linked[0].byte_range.start.0..linked[0].byte_range.end.0], "the chart");
        // The layout remembered where the link is, for click handling.
        let spans = LINK_SPANS.with(|map| map.borrow().get(&text_key(&text)).cloned()).expect("spans registered");
        assert_eq!(spans, [(4..13, "https://example.org".to_string())]);
        assert_eq!(take_clicked_link(&ctx), None);
    }
}

#[cfg(test)]
mod caret_request_tests {
    use super::*;

    #[test]
    fn a_caret_request_for_this_document_places_the_caret_in_its_paragraph() {
        let ctx = egui::Context::default();
        let content = portmanteau_core::RichDocument::from_plain_text("First paragraph.\n\nSecond paragraph here.", "body");
        // Paragraph indices count the blank one between the two lines of text.
        let target = content.paragraphs.iter().position(|p| p.plain_text().starts_with("Second")).unwrap();
        let mut request = Some(crate::app::CaretRequest { doc_id: "doc".into(), paragraph: target, offset: 7, select: 0, focus: true });
        let id = Id::unique("caret-test");
        let mut other = None;
        let mut placed = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            // Someone else's request is left alone.
            let mut theirs = Some(crate::app::CaretRequest { doc_id: "other".into(), paragraph: 0, offset: 0, select: 0, focus: true });
            other = take_caret_request(&mut theirs, ui, "doc", &content, id);
            assert!(theirs.is_some());
            placed = take_caret_request(&mut request, ui, "doc", &content, id);
        })
        .drop_without_applying_deltas();
        assert_eq!(other, None);
        let expected = rich_paragraph_start(&content, target) + 7;
        assert_eq!(placed, Some(expected));
        assert!(request.is_none(), "the request is consumed");
        let state = egui::text_edit::TextEditState::load(&ctx, id).expect("state stored");
        assert_eq!(state.cursor.char_range().unwrap().primary.index.0, expected);
    }
}

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
        click_harness_from(app, doc_id)
    }

    fn click_harness_from(app: PortmanteauApp, doc_id: String) -> ClickHarness {
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

    fn harness_with_blocks(blocks: Vec<portmanteau_core::Block>, focus: usize) -> ClickHarness {
        let (mut app, doc_id) = script_fixture();
        app.ws
            .project
            .document_mut(&doc_id)
            .unwrap()
            .set_script_blocks(blocks);
        app.selected_block = focus;
        app.block_focus = Some(focus);
        click_harness_from(app, doc_id)
    }

    fn popup_labels(fixture: &ClickHarness) -> Vec<String> {
        ["Mara Venn", "Elias Rook", "↺ Elias Rook", "↺ Mara Venn", "The Boy"]
            .iter()
            .filter(|label| {
                fixture
                    .harness
                    .query_by_role_and_label(Role::Button, label)
                    .is_some()
            })
            .map(|label| label.to_string())
            .collect()
    }

    /// Resting the caret on an existing cue must not pop the list open.
    #[test]
    fn caret_on_an_existing_cue_does_not_open_the_popup() {
        use portmanteau_core::Block;
        let fixture = harness_with_blocks(
            vec![
                Block::new(ElementKind::Action, "A boy sits."),
                Block::new(ElementKind::Character, "Boy"),
                Block::new(ElementKind::Dialogue, "Hi."),
            ],
            1,
        );
        assert!(popup_labels(&fixture).is_empty(), "{:?}", popup_labels(&fixture));
    }

    /// Enter at the end of a cue that is followed by another paragraph must
    /// split the cue, not the paragraph after it.
    #[test]
    fn enter_in_the_middle_of_a_script_gives_dialogue_not_the_next_paragraphs_style() {
        use portmanteau_core::Block;
        let mut fixture = harness_with_blocks(
            vec![
                Block::new(ElementKind::Character, "Mara Venn"),
                Block::new(ElementKind::Dialogue, "Then we have a problem."),
                Block::new(ElementKind::Character, "Elias Rook"),
                Block::new(ElementKind::SceneHeading, ""),
                Block::new(ElementKind::Transition, "SMASH CUT TO BLACK."),
            ],
            2,
        );
        assert_eq!(fixture.harness.ctx.memory(|m| m.focused().is_some()), true);
        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(3);
        let paragraphs = paragraph_summary(&fixture);
        let styles: Vec<&str> = paragraphs.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(
            styles,
            vec!["character", "dialogue", "character", "dialogue", "scene-heading", "transition"],
            "{paragraphs:?}"
        );
        assert_eq!(fixture.app.borrow().selected_block, 3);
    }

    /// After a line of dialogue, Enter starts a cue and offers the other
    /// speaker first.
    #[test]
    fn enter_after_dialogue_opens_a_cue_with_the_other_speaker_on_top() {
        use portmanteau_core::Block;
        let mut fixture = harness_with_blocks(
            vec![
                Block::new(ElementKind::Character, "Elias Rook"),
                Block::new(ElementKind::Dialogue, "Where is she?"),
                Block::new(ElementKind::Character, "Mara Venn"),
                Block::new(ElementKind::Dialogue, "Then we have a problem."),
            ],
            3,
        );
        let end = {
            let app = fixture.app.borrow();
            app.ws
                .project
                .document(&fixture.doc_id)
                .unwrap()
                .content
                .char_count()
        };
        {
            let id = block_id(&fixture.doc_id, 0);
            let mut state = egui::text_edit::TextEditState::load(&fixture.harness.ctx, id)
                .unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(end),
            )));
            state.store(&fixture.harness.ctx, id);
        }
        fixture.harness.run_steps(2);
        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(4);
        let paragraphs = paragraph_summary(&fixture);
        assert_eq!(paragraphs.last().unwrap().0, "character", "{paragraphs:?}");
        assert!(
            fixture
                .harness
                .query_by_role_and_label(Role::Button, "↺ Elias Rook")
                .is_some(),
            "the other speaker should head the list: {:?}",
            popup_labels(&fixture)
        );
        fixture.harness.key_press(Key::Enter);
        fixture.harness.run_steps(3);
        let paragraphs = paragraph_summary(&fixture);
        assert_eq!(paragraphs.last().unwrap().1, "Elias Rook", "{paragraphs:?}");
        assert_eq!(paragraphs.len(), 5);
    }

    /// An empty paragraph still reports its style's margin, which the editor
    /// uses to draw the caret there.
    #[test]
    fn empty_paragraph_leading_follows_the_style() {
        let sheet = portmanteau_core::StyleSheet::film_screenplay(
            &portmanteau_core::ElementStyles::default(),
        );
        let dialogue = &sheet.style("dialogue").unwrap().format;
        assert!(paragraph_leading(dialogue, 1.0, 600.0, 0, 12.0) > 40.0);
        let character = &sheet.style("character").unwrap().format;
        assert!(paragraph_leading(character, 1.0, 600.0, 0, 12.0) > 100.0);
        let action = &sheet.style("action").unwrap().format;
        assert_eq!(paragraph_leading(action, 1.0, 600.0, 0, 12.0), 0.0);
    }

    #[test]
    fn cue_list_merges_script_short_forms_into_database_names() {
        use portmanteau_core::Block;
        let blocks = vec![
            Block::new(ElementKind::Character, "ELIAS"),
            Block::new(ElementKind::Dialogue, "Hm."),
            Block::new(ElementKind::Character, "MARA"),
            Block::new(ElementKind::Dialogue, "Yes."),
            Block::new(ElementKind::Character, "BOY"),
            Block::new(ElementKind::Dialogue, "No."),
            Block::new(ElementKind::Character, ""),
        ];
        let cast: Vec<String> = ["Mara Venn", "Elias Rook", "The Boy", "Ruth Alder"]
            .iter()
            .map(|name| name.to_string())
            .collect();
        let labels: Vec<String> = cue_suggestions(&blocks, 6, "", &cast)
            .into_iter()
            .map(|suggestion| suggestion.insert)
            .collect();
        assert_eq!(labels, vec!["Mara Venn", "The Boy", "Elias Rook", "Ruth Alder"]);
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
                    .query_by_role_and_label(Role::Button, "↺ Mara Venn")
                    .is_some(),
                "an empty cue should offer the next speaker as its suggestion"
            );

            harness.key_press(Key::Enter);
            harness.run_steps(2);
            assert_eq!(
                block_text(&app.borrow(), &doc_id, 4),
                "Mara Venn",
                "Enter on the next-speaker suggestion fills the cue with the database name"
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

    /// Every prose field the writer types into must underline misspellings:
    /// the helpers the secondary fields share find them and respect the switch.
    #[test]
    fn secondary_fields_find_misspellings_and_honour_the_spelling_switch() {
        let app = PortmanteauApp::new(None);
        let text = "A perfectly good sentence with one wrrod wrong.";
        let spans = field_spans(&app.spell, true, text);
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert_eq!(&text[spans[0].0..spans[0].1], "wrrod");
        assert!(field_spans(&app.spell, false, text).is_empty(), "off means off");
    }

    /// The plain-text field underlines a misspelling in its own layout.
    #[test]
    fn spelled_text_edit_underlines_in_a_plain_field() {
        let app = PortmanteauApp::new(None);
        let checker = std::rc::Rc::new(app.spell);
        let text = std::rc::Rc::new(std::cell::RefCell::new(String::from("Fine words and a wrrod.")));
        let (c, t) = (checker.clone(), text.clone());
        let mut harness = egui_kittest::Harness::builder()
            .with_size(Vec2::new(500.0, 200.0))
            .with_accessibility_check(false)
            .allow_missing_glyphs()
            .build_ui(move |ui| {
                let (output, _) = spelled_text_edit(
                    ui,
                    &mut t.borrow_mut(),
                    &c,
                    true,
                    Id::unique("plain_spell_test"),
                    egui::FontId::proportional(14.0),
                    Color32::BLACK,
                    |edit| edit,
                );
                let has_underline = output.galley.job.sections.iter().any(|section| {
                    section.format.underline.width > 0.0
                });
                ui.ctx().data_mut(|d| d.insert_temp(Id::unique("underlined"), has_underline));
            });
        harness.run_steps(3);
        let underlined: bool = harness.ctx.data(|d| d.get_temp(Id::unique("underlined"))).unwrap_or(false);
        assert!(underlined, "the misspelled word should carry an underline section");
        assert_eq!(&*text.borrow(), "Fine words and a wrrod.", "drawing does not alter the text");
    }

    #[test]
    fn the_word_being_typed_is_not_underlined_until_the_caret_moves_on() {
        let id = Id::unique("typing-test");
        let spans = vec![(10usize, 13usize)];
        let result = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let out = std::rc::Rc::clone(&result);
        let mut harness = egui_kittest::Harness::new_ui(move |ui| {
            let ctx = ui.ctx().clone();
            ctx.memory_mut(|m| m.request_focus(id));
            let at = |caret: usize| {
                let mut state = egui::text_edit::TextEditState::default();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(caret))));
                state.store(&ctx, id);
            };
            let mut out = out.borrow_mut();
            out.clear();
            at(13);
            out.push(without_word_being_typed(&ctx, id, "One day I sta", spans.clone()));
            at(11);
            out.push(without_word_being_typed(&ctx, id, "One day I sta", spans.clone()));
            at(14);
            out.push(without_word_being_typed(&ctx, id, "One day I sta ", spans.clone()));
        });
        harness.run_steps(3);
        let result = result.borrow();
        assert!(result[0].is_empty(), "caret at the end of the word: not flagged yet");
        assert_eq!(result[1], vec![(10, 13)], "caret elsewhere in the word is still flagged");
        assert_eq!(result[2], vec![(10, 13)], "once the caret moves on, the word is flagged");
    }
}
