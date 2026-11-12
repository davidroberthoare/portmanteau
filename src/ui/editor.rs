//! The writing surface: a paper-like prose editor and a typed screenplay editor.

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, FontId, Frame, Id, Key, Layout, Margin,
    Modifiers, RichText, ScrollArea, Sense, Stroke, TextEdit, Vec2,
};
use portmanteau_core::{DocKind, ElementKind, ItemKind, Medium};

use crate::app::{NewCharacterState, PortmanteauApp, WriteMode};
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
    // The prose surface takes its type from the project's body element style,
    // the same style the compiled page and preview use.
    let body = app.ws.project.styles.body.clone();
    let family = family_for_font(&body.font);
    let text_size = body.size.max(6.0) * zoom;
    let line_height = (body.size.max(6.0) * body.leading.max(0.85) * zoom).max(text_size);
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

        let rows = ((doc.text.chars().count() as f32) / 72.0).ceil().max(18.0) as usize;
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut job = egui::text::LayoutJob::single_section(
                text.as_str().to_owned(),
                egui::TextFormat {
                    font_id: FontId::new(text_size, family.clone()),
                    color: PAPER_TEXT,
                    line_height: Some(line_height),
                    ..Default::default()
                },
            );
            job.wrap.max_width = wrap_width;
            ui.ctx().fonts_mut(|f| f.layout_job(job))
        };
        let edit = TextEdit::multiline(&mut doc.text)
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
    let mut cycle_at: Option<(usize, bool)> = None;
    let mut set_text_at: Option<(usize, String)> = None;
    let mut new_character: Option<NewCharacterState> = None;
    let project_title = app.ws.project.title.clone();
    // Clone the element styles before the document is borrowed mutably. Each
    // block takes its type and indent from the matching script element style,
    // the same table the paged preview and compile use.
    let styles = app.ws.project.styles.clone();
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
    // Escape closes the character popup even though it also surrenders the
    // text widget's focus. The filter below leaves `escape: false`, so egui
    // clears the focus before the widget sees the key; reading the raw input
    // here is what keeps the popup's own dismissal working.
    if ui.input(|i| i.key_pressed(Key::Escape)) {
        app.character_menu = None;
    }
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
            let block_id = Id::unique(("script_block", doc_id, i));
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
                // Shift+Tab has to be matched before plain Tab: `consume_key`
                // ignores extra Shift/Alt modifiers, so a `NONE` pattern would
                // otherwise swallow Shift+Tab and cycle forwards.
                let shift_tab = ui.input_mut(|inp| inp.consume_key(Modifiers::SHIFT, Key::Tab));
                let tab = ui.input_mut(|inp| inp.consume_key(Modifiers::NONE, Key::Tab));
                if tab || shift_tab {
                    cycle_at = Some((i, tab));
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

                let style = script_element_style(&styles, kind);
                let indent = style.indent.max(0.0).min(content_width * 0.42);
                if indent > 0.0 {
                    ui.add_space(indent);
                }
                let width = (content_width - indent - 46.0).max(80.0);
                let family = family_for_font(&style.font);
                let font_size = style.size.max(6.0) * zoom;
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
                    .font(FontId::new(font_size, family.clone()))
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
                if kind.is_uppercase() && kind != ElementKind::Character {
                    edit = edit.text_color(Color32::from_rgb(44, 40, 33));
                }
                let resp = ui.add(edit);
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
                    // Clicking a block owns the popup: open it on a Character
                    // element, close it for anything else.
                    app.character_menu = if kind == ElementKind::Character {
                        Some((doc_id.to_string(), i))
                    } else {
                        None
                    };
                }
                if resp.changed() {
                    if style.uppercase {
                        doc.blocks[i].text = doc.blocks[i].text.to_uppercase();
                    }
                    doc.touch();
                    dirty = true;
                    if kind == ElementKind::Character {
                        app.character_menu = Some((doc_id.to_string(), i));
                    }
                }
                // The popup is drawn from view state, not from text focus, so
                // it stays on screen while the pointer travels to it. Clicking
                // a name then registers instead of unmounting the widget first.
                let menu_open = matches!(
                    app.character_menu.as_ref(),
                    Some((id, index)) if id == doc_id && *index == i
                );
                if kind == ElementKind::Character && menu_open {
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

    if let Some((i, forward)) = cycle_at {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            if let Some(block) = doc.blocks.get_mut(i) {
                block.kind = if forward {
                    block.kind.next(doc.medium)
                } else {
                    block.kind.prev(doc.medium)
                };
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
        app.character_menu = None;
    }
    if let Some(state) = new_character {
        app.new_character = Some(state);
        app.character_menu = None;
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
    egui::Area::new(Id::unique(("character_autocomplete", doc_id, index)))
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
            egui::Area::new(Id::unique(("next_speaker", doc_id, index)))
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
        Id::unique(("script_block", doc_id, index))
    }

    fn block_kind(app: &PortmanteauApp, doc_id: &str, index: usize) -> ElementKind {
        app.ws
            .project
            .document(doc_id)
            .expect("document still exists")
            .blocks[index]
            .kind
    }

    fn block_text(app: &PortmanteauApp, doc_id: &str, index: usize) -> String {
        app.ws
            .project
            .document(doc_id)
            .expect("document still exists")
            .blocks[index]
            .text
            .clone()
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
    fn typing_into_a_character_cue_uppercases_the_stored_text() {
        let (mut app, doc_id) = script_fixture();
        let ctx = headless_ctx();
        if let Some(doc) = app.ws.project.document_mut(&doc_id) {
            doc.blocks[0].kind = ElementKind::Character;
            doc.blocks[0].text.clear();
        }
        settle(&mut app, &ctx);
        assert!(ctx.memory(|m| m.has_focus(block_id(&doc_id, 0))));

        frame(&mut app, &ctx, vec![Event::Text("mara".into())]);
        assert_eq!(
            block_text(&app, &doc_id, 0),
            "MARA",
            "a Character block should store all-caps text"
        );
    }

    #[test]
    fn typing_into_a_character_cue_opens_the_character_menu() {
        let (mut app, doc_id) = script_fixture();
        let ctx = headless_ctx();
        if let Some(doc) = app.ws.project.document_mut(&doc_id) {
            doc.blocks[0].kind = ElementKind::Character;
            doc.blocks[0].text.clear();
        }
        settle(&mut app, &ctx);
        assert_eq!(app.character_menu, None, "the menu starts closed");

        frame(&mut app, &ctx, vec![Event::Text("ma".into())]);
        assert_eq!(
            app.character_menu,
            Some((doc_id.clone(), 0)),
            "typing a character cue should open the autocomplete for that block"
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
            doc.blocks[0].kind = ElementKind::Character;
            doc.blocks[0].text.clear();
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

        fn open_character_menu(&mut self) {
            self.harness
                .input_mut()
                .events
                .push(Event::Text("ma".into()));
            self.harness.run();
            assert_eq!(
                self.app.borrow().character_menu,
                Some((self.doc_id.clone(), 0)),
                "typing should open the character autocomplete"
            );
        }
    }

    #[test]
    fn clicking_an_autocomplete_name_fills_the_block() {
        let mut fixture = click_harness();
        fixture.open_character_menu();

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
        assert_eq!(
            fixture.app.borrow().character_menu,
            None,
            "the autocomplete should close after a name is chosen"
        );
    }

    #[test]
    fn clicking_new_character_opens_the_modal() {
        let mut fixture = click_harness();
        fixture.open_character_menu();

        fixture
            .harness
            .get_by_role_and_label(Role::Button, "+ New character…")
            .click();
        fixture.harness.run();

        let app = fixture.app.borrow();
        assert_eq!(
            app.character_menu, None,
            "the autocomplete should close when the new-character modal opens"
        );
        let state = app
            .new_character
            .as_ref()
            .expect("clicking '+ New character…' should open the modal state");
        assert_eq!(state.doc_id, fixture.doc_id);
        assert_eq!(state.block, 0);
    }

    #[test]
    fn clicking_a_palette_element_changes_the_block_type() {
        let mut fixture = click_harness();
        assert_eq!(fixture.kind(), ElementKind::Character);

        fixture
            .harness
            .get_by_role_and_label(Role::Button, "Dialogue")
            .click();
        fixture.harness.run();

        assert_eq!(
            fixture.kind(),
            ElementKind::Dialogue,
            "clicking a palette element should retype the selected block"
        );
        assert_eq!(fixture.app.borrow().selected_block, 0);
    }
}
