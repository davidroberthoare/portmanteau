//! The editor's find bar (Ctrl/Cmd+F): a slim find/replace strip above the
//! page that works on the open document only. The Search page
//! (`views/search.rs`) is the project-wide version; both use `Finder`.
//!
//! Matches are shaded in the text by `paint_matches`, called by the prose and
//! script editors after they draw. Moving between matches sends the editor a
//! `CaretRequest` that selects the match without stealing the keyboard from
//! the find box. Replacing goes through the `search.replace` command.

use std::ops::Range;

use eframe::egui::{self, Color32, CornerRadius, Frame, Key, Margin, Rect, RichText, Stroke, TextEdit};
use portmanteau_core::{FindOptions, Finder};
use serde_json::json;

use crate::app::{CaretRequest, PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::find_ui;

pub struct FindBar {
    pub open: bool,
    pub query: String,
    pub replacement: String,
    pub options: FindOptions,
    pub show_replace: bool,
    /// Take keyboard focus on the next frame.
    pub focus: bool,
    /// Matches in the open document (character ranges in its editor flow).
    pub matches: Vec<Range<usize>>,
    pub current: usize,
    pub error: Option<String>,
    /// What the matches were found for, to notice a changed query.
    seen: Option<(String, FindOptions, String)>,
}

impl Default for FindBar {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            replacement: String::new(),
            options: FindOptions::default(),
            show_replace: false,
            focus: false,
            matches: Vec::new(),
            current: 0,
            error: None,
            seen: None,
        }
    }
}

const QUERY_ID: &str = "find_bar_query";
const REPLACE_ID: &str = "find_bar_replace";

/// Open (or refocus) the bar.
pub fn open(app: &mut PortmanteauApp, with_replace: bool) {
    app.find_bar.open = true;
    app.find_bar.focus = true;
    app.find_bar.show_replace |= with_replace;
}

/// Draw the bar above the page and keep its matches current.
pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    if !app.find_bar.open {
        return;
    }
    let mut goto: Option<usize> = None;
    let mut replace_one = false;
    let mut replace_all = false;
    let mut close = false;
    let mut whole_project = false;

    // Work out the matches first so the label is right this frame.
    let content = app.ws.project.document(doc_id).map(|doc| doc.content.clone());
    let bar = &mut app.find_bar;
    let key = (bar.query.clone(), bar.options, doc_id.to_string());
    let query_changed = bar.seen.as_ref() != Some(&key);
    bar.error = None;
    bar.matches = match (&content, bar.query.is_empty()) {
        (Some(content), false) => match Finder::new(&bar.query, &bar.options) {
            Ok(finder) => finder.find_in(content),
            Err(error) => {
                bar.error = Some(error);
                Vec::new()
            }
        },
        _ => Vec::new(),
    };
    if query_changed {
        bar.seen = Some(key);
        bar.current = 0;
        goto = (!bar.matches.is_empty()).then_some(0);
    }
    bar.current = bar.current.min(bar.matches.len().saturating_sub(1));

    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 6))
        .outer_margin(Margin::symmetric(14, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let arrow = if bar.show_replace { "▾" } else { "▸" };
                if ui.add(egui::Button::new(RichText::new(arrow).color(MUTED)).frame(false)).on_hover_text("Show or hide Replace").clicked() {
                    bar.show_replace = !bar.show_replace;
                }
                let find = ui.add(
                    TextEdit::singleline(&mut bar.query)
                        .id(egui::Id::unique(QUERY_ID))
                        .hint_text("Find")
                        .font(ui_body())
                        .desired_width(220.0),
                );
                if bar.focus {
                    find.request_focus();
                    // Select what is there so typing replaces it.
                    if let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), find.id) {
                        let end = egui::text::CCursor::new(bar.query.chars().count());
                        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), end)));
                        state.store(ui.ctx(), find.id);
                    }
                    bar.focus = false;
                }
                if find.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    let backwards = ui.input(|i| i.modifiers.shift);
                    step(bar, backwards, &mut goto);
                    find.request_focus();
                }
                find_ui::option_toggles(ui, &mut bar.options);
                let label = if let Some(error) = &bar.error {
                    RichText::new(error).font(ui_small()).color(RED_INK)
                } else if bar.query.is_empty() {
                    RichText::new("").font(ui_small())
                } else if bar.matches.is_empty() {
                    RichText::new("No results").font(ui_small()).color(FAINT)
                } else {
                    RichText::new(format!("{} of {}", bar.current + 1, bar.matches.len())).font(ui_small()).color(MUTED)
                };
                ui.add(egui::Label::new(label).truncate());
                let any = !bar.matches.is_empty();
                if ui.add_enabled(any, egui::Button::new("↑").frame(false)).on_hover_text("Previous match (Shift+Enter)").clicked() {
                    step(bar, true, &mut goto);
                }
                if ui.add_enabled(any, egui::Button::new("↓").frame(false)).on_hover_text("Next match (Enter)").clicked() {
                    step(bar, false, &mut goto);
                }
                if ui
                    .link(RichText::new("Search whole project").font(ui_small()).color(ACCENT))
                    .on_hover_text("Look for this in every document, notes and the database")
                    .clicked()
                {
                    whole_project = true;
                }
                if ui.add(egui::Button::new("×").frame(false)).on_hover_text("Close (Esc)").clicked() {
                    close = true;
                }
            });
            if bar.show_replace {
                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    let replace = ui.add(
                        TextEdit::singleline(&mut bar.replacement)
                            .id(egui::Id::unique(REPLACE_ID))
                            .hint_text("Replace")
                            .font(ui_body())
                            .desired_width(220.0),
                    );
                    if replace.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        replace_one = true;
                        replace.request_focus();
                    }
                    let any = !bar.matches.is_empty();
                    if ui.add_enabled(any, egui::Button::new("Replace")).on_hover_text("Replace this match and move to the next").clicked() {
                        replace_one = true;
                    }
                    if ui.add_enabled(any, egui::Button::new("Replace all")).on_hover_text("Replace every match in this document (a snapshot is saved first)").clicked() {
                        replace_all = true;
                    }
                });
            }
        });

    // Esc closes the bar wherever the focus is (the @ list takes its own Esc first).
    if ui.input(|i| i.key_pressed(Key::Escape)) {
        close = true;
    }
    if whole_project {
        // Hand the search, with its options, to the Search page, which runs it.
        let query = app.find_bar.query.clone();
        app.search_page.options = app.find_bar.options;
        app.open_search(Some(query));
        close = true;
    }
    if close {
        app.find_bar.open = false;
        app.find_bar.matches.clear();
        app.find_bar.seen = None;
        return;
    }

    if replace_one || replace_all {
        let bar = &app.find_bar;
        let mut payload = json!({
            "query": bar.query,
            "replacement": bar.replacement,
            "case_sensitive": bar.options.case_sensitive,
            "whole_word": bar.options.whole_word,
            "regex": bar.options.regex,
            "id": doc_id,
            "field": "Text",
        });
        if replace_one {
            if let Some(range) = bar.matches.get(bar.current) {
                payload["start"] = json!(range.start);
            }
        }
        let response = app.ws.execute("search.replace", payload);
        if replace_all {
            app.toast(if response.ok { "Replaced" } else { "Nothing replaced" }, response.message, if response.ok { ToastKind::Ok } else { ToastKind::Error });
        }
        // After replacing one, the next match slides into this one's place.
        app.find_bar.seen = None;
        if replace_one {
            let keep = app.find_bar.current;
            let content = app.ws.project.document(doc_id).map(|doc| doc.content.clone());
            if let (Some(content), Ok(finder)) = (content, Finder::new(&app.find_bar.query, &app.find_bar.options)) {
                let matches = finder.find_in(&content);
                app.find_bar.current = keep.min(matches.len().saturating_sub(1));
                goto = (!matches.is_empty()).then_some(app.find_bar.current);
                app.find_bar.matches = matches;
                app.find_bar.seen = Some((app.find_bar.query.clone(), app.find_bar.options, doc_id.to_string()));
            }
        }
    }

    if let Some(index) = goto {
        select_match(app, doc_id, index);
    }
}

fn step(bar: &mut FindBar, backwards: bool, goto: &mut Option<usize>) {
    let n = bar.matches.len();
    if n == 0 {
        return;
    }
    bar.current = if backwards { (bar.current + n - 1) % n } else { (bar.current + 1) % n };
    *goto = Some(bar.current);
}

/// Select match `index` in the editor, scrolling to it, without taking focus.
fn select_match(app: &mut PortmanteauApp, doc_id: &str, index: usize) {
    let Some(range) = app.find_bar.matches.get(index).cloned() else { return };
    let Some(doc) = app.ws.project.document(doc_id) else { return };
    let at = doc.content.position_at(range.start);
    app.caret_request = Some(CaretRequest {
        doc_id: doc_id.to_string(),
        paragraph: at.paragraph,
        offset: at.offset,
        select: range.end - range.start,
        focus: false,
    });
}

/// Shade the matches in a drawn text (the current one stronger). Call after the
/// `TextEdit` is shown, with its galley.
pub fn paint_matches(ui: &egui::Ui, bar: &FindBar, galley: &egui::Galley, galley_pos: egui::Pos2) {
    if !bar.open || bar.matches.is_empty() {
        return;
    }
    let clip = ui.clip_rect();
    let others = Color32::from_rgba_unmultiplied(250, 214, 70, 110);
    let current = Color32::from_rgba_unmultiplied(240, 150, 40, 150);
    for (index, range) in bar.matches.iter().enumerate().take(4000) {
        let from = galley.pos_from_cursor(egui::text::CCursor::new(range.start)).translate(galley_pos.to_vec2());
        let to = galley.pos_from_cursor(egui::text::CCursor::new(range.end)).translate(galley_pos.to_vec2());
        if from.max.y < clip.min.y || from.min.y > clip.max.y {
            continue;
        }
        let colour = if index == bar.current { current } else { others };
        let shade = if (from.min.y - to.min.y).abs() < 1.0 {
            Rect::from_min_max(egui::pos2(from.min.x, from.min.y), egui::pos2(to.min.x.max(from.min.x + 2.0), from.max.y))
        } else {
            // Wrapped across two lines: shade the first part to the edge.
            Rect::from_min_max(egui::pos2(from.min.x, from.min.y), egui::pos2(galley_pos.x + galley.rect.max.x, from.max.y))
        };
        ui.painter().rect_filled(shade, CornerRadius::same(2), colour);
    }
}
