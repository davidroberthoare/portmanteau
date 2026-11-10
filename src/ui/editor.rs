//! The writing surface: a paper-like prose editor and a typed screenplay editor.

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, Frame, Id, Key, Layout, Margin, Modifiers,
    RichText, ScrollArea, Sense, Stroke, TextEdit, Vec2,
};
use portmanteau_core::{DocKind, ElementKind, ItemKind, Medium};

use crate::app::{NewCharacterState, PortmanteauApp, WriteMode};
use crate::theme::*;
use crate::ui::page::{empty_page_plan, paged_view, prose_plan, script_plan, PagePlan};
use crate::ui::views::{content_frame, page_head};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let doc_id = app
        .selected
        .clone()
        .filter(|id| app.ws.project.document(id).is_some());

    let Some(doc_id) = doc_id else {
        toolbar(app, ui, None, 0);
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
    toolbar(
        app,
        ui,
        page_plan.as_ref().map(|plan| plan.pages.len()),
        miss.len(),
    );

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

/// Folders are pure containers, so the Write view shows this panel rather than
/// a paper page: an editable name and synopsis, then the documents inside.
fn folder_panel(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    let Some(doc) = app.ws.project.document(doc_id) else {
        return;
    };
    let original_title = doc.title.clone();
    let original_synopsis = doc.synopsis.clone();
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
                            .font(sans(11.0))
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
                    .font(sans(10.0))
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
                doc.synopsis = synopsis;
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
        DocKind::Chapter => ("¶", ACCENT),
        DocKind::Scene if medium.is_script() => ("▶", PURPLE),
        DocKind::Scene => ("◆", SAGE),
        DocKind::Research => ("⌕", BLUE),
        DocKind::Character => ("☺", PURPLE),
        DocKind::FrontMatter | DocKind::BackMatter => ("▤", FAINT),
        DocKind::Text => ("◆", FAINT),
    };
    ui.painter().text(
        egui::pos2(rect.left() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        glyph,
        sans(10.5),
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
        sans(11.0),
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

/// Compact `−` / percentage / `+` control for the writing surface. It changes
/// only view state; the project is untouched and nothing is saved.
fn editor_zoom_control(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let zoom = app.editor_zoom.clamp(0.7, 1.6);
    let mut next = zoom;
    // A fixed-width allocation keeps the group compact inside the
    // right-aligned toolbar row instead of stretching across it.
    ui.allocate_ui_with_layout(
        Vec2::new(88.0, 21.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            Frame::new()
                .fill(PANEL_2)
                .corner_radius(CornerRadius::same(5))
                .inner_margin(Margin::symmetric(2, 1))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    ui.spacing_mut().button_padding = Vec2::new(4.0, 2.0);
                    if zoom_button(ui, "−").on_hover_text("Zoom out").clicked() {
                        next = step_zoom(zoom, -1);
                    }
                    ui.add_sized(
                        Vec2::new(42.0, 19.0),
                        egui::Label::new(
                            RichText::new(format!("{}%", (zoom * 100.0).round() as i32))
                                .font(sans(9.5))
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
        egui::Button::new(RichText::new(glyph).font(sans(10.5)).color(MUTED))
            .fill(PANEL_2)
            .stroke(Stroke::new(1.0, BORDER_SOFT))
            .corner_radius(CornerRadius::same(4))
            .min_size(Vec2::new(20.0, 19.0)),
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
    let zoom = app.editor_zoom.clamp(0.7, 1.6);
    let prose_size = app.settings.display.prose_size;
    let prose_leading = app.settings.display.prose_leading;
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
        let text_size = prose_size * zoom;
        let line_height = (prose_size * prose_leading * zoom).max(text_size);
        let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
            let mut job = egui::text::LayoutJob::single_section(
                text.to_owned(),
                egui::TextFormat {
                    font_id: serif(text_size),
                    color: PAPER_TEXT,
                    line_height: Some(line_height),
                    ..Default::default()
                },
            );
            job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(job))
        };
        let edit = TextEdit::multiline(&mut doc.text)
            .font(serif(text_size))
            .layouter(&mut layouter)
            .text_color(PAPER_TEXT)
            .frame(false)
            .desired_width(f32::INFINITY)
            .desired_rows(rows.min(120))
            .margin(Margin::same(0))
            .hint_text(
                RichText::new("Begin at the beginning…")
                    .font(serif(text_size))
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
    let zoom = app.editor_zoom.clamp(0.7, 1.6);
    let mut add_at: Option<usize> = None;
    let mut remove_at: Option<usize> = None;
    let mut cycle_at: Option<usize> = None;
    let mut set_text_at: Option<(usize, String)> = None;
    let mut new_character: Option<NewCharacterState> = None;
    let project_title = app.ws.project.title.clone();
    let script_size = app.settings.display.script_size;
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
            for (index, kind) in ElementKind::all(doc.medium).into_iter().enumerate() {
                let active = kind == selected_kind;
                let btn = egui::Button::new(
                    RichText::new(kind.short())
                        .font(sans(8.5))
                        .color(if active { Color32::from_rgb(30, 26, 19) } else { PAPER_MUTED }),
                )
                .fill(if active { ACCENT } else { Color32::from_rgb(235, 229, 216) })
                .corner_radius(CornerRadius::same(10))
                .min_size(Vec2::new(0.0, 19.0));
                if ui
                    .add(btn)
                    .on_hover_text(format!("{} · Ctrl+{}", kind.label(), index + 1))
                    .clicked()
                {
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
            // Keyboard: Enter starts a new element, Tab cycles its type,
            // Ctrl/Cmd+1…8 selects a type directly. Arrow keys stay bound to
            // the text caret.
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
                if let Some(kind) = ctrl_digit_element(ui, doc.medium) {
                    doc.blocks[i].kind = kind;
                    app.selected_block = i;
                    dirty = true;
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
                        mono(7.5 * zoom),
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
                    ElementKind::SceneHeading => script_size * 1.05,
                    ElementKind::Character => script_size,
                    ElementKind::Parenthetical => script_size * 0.95,
                    ElementKind::Dialogue => script_size,
                    ElementKind::Transition => script_size * 0.95,
                    _ => script_size,
                } * zoom;
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
                if kind == ElementKind::Character && (focused || resp.has_focus()) {
                    character_assist(
                        ui,
                        doc_id,
                        i,
                        resp.rect,
                        &character_names,
                        doc.blocks[i].text.as_str(),
                        &doc.blocks,
                        &mut set_text_at,
                        &mut new_character,
                    );
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
    if let Some((i, text)) = set_text_at {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            let mut changed = false;
            if let Some(block) = doc.blocks.get_mut(i) {
                if block.text != text {
                    block.text = text;
                    changed = true;
                }
            }
            if changed {
                doc.touch();
                app.selected_block = i;
                app.block_focus = Some(i);
                app.ws.dirty = true;
            }
        }
    }
    if let Some(state) = new_character {
        app.new_character = Some(state);
    }
    if dirty {
        app.ws.dirty = true;
    }
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

/// The autocomplete dropdown and next-speaker hint for one focused Character
/// element. Anything that needs a mutation is collected into the two `Option`
/// out-parameters, because `doc` is borrowed for the draw.
#[allow(clippy::too_many_arguments)]
fn character_assist(
    ui: &mut egui::Ui,
    doc_id: &str,
    index: usize,
    rect: egui::Rect,
    names: &[String],
    text: &str,
    blocks: &[portmanteau_core::Block],
    set_text_at: &mut Option<(usize, String)>,
    new_character: &mut Option<NewCharacterState>,
) {
    let query = text.trim().to_lowercase();
    let matches: Vec<&String> = names
        .iter()
        .filter(|name| {
            if query.is_empty() {
                true
            } else {
                let lower = name.to_lowercase();
                lower.contains(&query) && lower != query
            }
        })
        .take(6)
        .collect();

    let area_pos = egui::pos2(rect.left(), rect.bottom() + 3.0);
    egui::Area::new(Id::new(("character_autocomplete", doc_id, index)))
        .order(egui::Order::Foreground)
        .fixed_pos(area_pos)
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(Color32::from_rgb(247, 243, 233))
                .stroke(Stroke::new(1.0, PAPER_EDGE))
                .corner_radius(CornerRadius::same(5))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 0,
                    color: Color32::from_black_alpha(60),
                })
                .inner_margin(Margin::symmetric(5, 5))
                .show(ui, |ui| {
                    ui.set_min_width(190.0);
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for name in &matches {
                        let btn = egui::Button::new(
                            RichText::new(name.as_str())
                                .font(sans(10.5))
                                .color(PAPER_TEXT),
                        )
                        .frame(false)
                        .min_size(Vec2::new(180.0, 20.0));
                        if ui.add(btn).clicked() {
                            *set_text_at = Some((index, (*name).clone()));
                        }
                    }
                    if !matches.is_empty() {
                        ui.add_space(2.0);
                        let (rule, _) =
                            ui.allocate_exact_size(Vec2::new(180.0, 1.0), Sense::hover());
                        ui.painter().line_segment(
                            [rule.left_center(), rule.right_center()],
                            Stroke::new(1.0, PAPER_EDGE),
                        );
                        ui.add_space(2.0);
                    }
                    let new = egui::Button::new(
                        RichText::new("+ New character…")
                            .font(sans(10.0))
                            .color(PAPER_MUTED),
                    )
                    .frame(false)
                    .min_size(Vec2::new(180.0, 20.0));
                    if ui.add(new).clicked() {
                        *new_character = Some(NewCharacterState {
                            doc_id: doc_id.to_string(),
                            block: index,
                            name: text.trim().to_string(),
                            role: String::new(),
                        });
                    }
                });
        });

    if text.trim().is_empty() {
        if let Some(speaker) = next_speaker(blocks, index) {
            let pos = egui::pos2(rect.right() + 8.0, rect.top() + 1.0);
            egui::Area::new(Id::new(("next_speaker", doc_id, index)))
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    let hint = egui::Button::new(
                        RichText::new(format!("↺ {speaker}"))
                            .font(mono(8.5))
                            .color(PAPER_MUTED),
                    )
                    .frame(false);
                    if ui
                        .add(hint)
                        .on_hover_text("Fill the next speaker")
                        .clicked()
                    {
                        *set_text_at = Some((index, speaker.clone()));
                    }
                });
        }
    }
}

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

/// Page the open document with the active format's settings. The paging
/// primitives live in [`crate::ui::page`], shared with the Compile preview.
fn build_page_plan(app: &PortmanteauApp, doc_id: &str) -> Option<PagePlan> {
    let doc = app.ws.project.document(doc_id)?;
    let settings = app.ws.project.settings_for(app.ws.project.export.kind);
    Some(if doc.medium.is_script() {
        script_plan(doc, &settings)
    } else {
        prose_plan(doc, &settings)
    })
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
