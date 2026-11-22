//! The Search page: find (and replace) across the whole project, with the
//! same options as the editor's find bar, filtered by where the text lives.
//! Clicking a result opens the document at that place.
//!
//! The work is done by `Project::find_all` / the `search.replace` command;
//! this page only holds the query and draws the answer.

use std::collections::HashSet;

use eframe::egui::{
    self, text::LayoutJob, Color32, CornerRadius, Frame, Margin, RichText, ScrollArea, Sense, Stroke, TextEdit,
    TextFormat, Vec2,
};
use portmanteau_core::{FindHit, FindOptions, FindQuery, FindSource, SearchGroup};
use serde_json::json;

use crate::app::{PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::find_ui;
use crate::ui::views::{content_frame, meta_line, page_head};

pub struct SearchPage {
    pub query: String,
    pub replacement: String,
    pub options: FindOptions,
    /// Chosen groups; none chosen means everywhere.
    pub groups: Vec<SearchGroup>,
    pub asides: bool,
    pub show_replace: bool,
    /// Ask for the query box to take focus on the next frame.
    pub focus: bool,
    hits: Vec<FindHit>,
    error: Option<String>,
    /// What `hits` was computed for; None means "compute again".
    ran: Option<FindQuery>,
    folded: HashSet<String>,
}

impl Default for SearchPage {
    fn default() -> Self {
        Self {
            query: String::new(),
            replacement: String::new(),
            options: FindOptions::default(),
            groups: Vec::new(),
            asides: true,
            show_replace: false,
            focus: false,
            hits: Vec::new(),
            error: None,
            ran: None,
            folded: HashSet::new(),
        }
    }
}

impl SearchPage {
    pub fn query(&self) -> FindQuery {
        FindQuery { text: self.query.clone(), options: self.options, groups: self.groups.clone(), asides: self.asides }
    }
    /// Forget the answer so it is worked out again (the project changed).
    pub fn stale(&mut self) {
        self.ran = None;
    }
    #[cfg(test)]
    pub fn hits(&self) -> &[FindHit] {
        &self.hits
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

enum Act {
    Open(FindHit),
    ReplaceHit(FindHit),
    ReplaceIn(FindSource),
    ReplaceAll,
}

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    refresh(app);
    let total = app.search_page.hits.len();
    let documents = {
        let mut seen: Vec<&str> = Vec::new();
        for hit in &app.search_page.hits {
            if !seen.contains(&hit.source.id()) {
                seen.push(hit.source.id());
            }
        }
        seen.len()
    };
    page_head(ui, "Find", "Search", "Look through the manuscript, research and database. Click a result to go there.", |ui| {
        if app.search_page.query.is_empty() {
            meta_line(ui, &["Type to search"]);
        } else if let Some(error) = app.search_page.error() {
            ui.label(RichText::new(error).font(ui_small()).color(RED_INK));
        } else {
            meta_line(ui, &[&format!("{total} result{} in {documents} place{}", plural(total), plural(documents))]);
        }
    });

    let mut act: Option<Act> = None;
    content_frame().show(ui, |ui| {
        let page = &mut app.search_page;
        let mut changed = false;
        Frame::new()
            .fill(PANEL)
            .stroke(Stroke::new(1.0, BORDER_SOFT))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let arrow = if page.show_replace { "▾" } else { "▸" };
                    if ui.add(egui::Button::new(RichText::new(arrow).font(ui_body()).color(MUTED)).frame(false)).on_hover_text("Show or hide Replace").clicked() {
                        page.show_replace = !page.show_replace;
                    }
                    let find = ui.add(
                        TextEdit::singleline(&mut page.query)
                            .hint_text("Find")
                            .font(ui_body())
                            .desired_width((ui.available_width() - 110.0).max(120.0)),
                    );
                    if page.focus {
                        find.request_focus();
                        page.focus = false;
                    }
                    changed |= find_ui::option_toggles(ui, &mut page.options);
                });
                if page.show_replace {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.add(
                            TextEdit::singleline(&mut page.replacement)
                                .hint_text("Replace")
                                .font(ui_body())
                                .desired_width((ui.available_width() - 130.0).max(120.0)),
                        );
                        let ready = !page.hits.is_empty() && page.error.is_none();
                        if ui.add_enabled(ready, egui::Button::new(RichText::new("Replace all").font(ui_body()))).on_hover_text("Replace every result below. Each changed document is snapshotted first.").clicked() {
                            act = Some(Act::ReplaceAll);
                        }
                    });
                }
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(5.0, 4.0);
                    ui.label(RichText::new("Look in").font(ui_small()).color(FAINT));
                    for group in SearchGroup::all() {
                        let on = page.groups.contains(&group);
                        if chip(ui, group.label(), on, "Show only this kind of result (choose several, or none for everything)") {
                            if on {
                                page.groups.retain(|g| *g != group);
                            } else {
                                page.groups.push(group);
                            }
                            changed = true;
                        }
                    }
                    if chip(ui, "Synopsis & notes", page.asides, "Also search each document's synopsis and notes") {
                        page.asides = !page.asides;
                        changed = true;
                    }
                });
            });
        if changed {
            page.stale();
        }
        ui.add_space(10.0);

        ScrollArea::vertical().id_salt("search_scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let page = &mut app.search_page;
            if page.query.is_empty() {
                return;
            }
            if page.hits.is_empty() && page.error.is_none() {
                ui.label(RichText::new("No results.").font(ui_body()).color(FAINT));
                return;
            }
            let replacing = page.show_replace;
            let mut index = 0;
            while index < page.hits.len() {
                let source = page.hits[index].source.clone();
                let end = page.hits[index..].iter().position(|h| h.source != source).map_or(page.hits.len(), |n| index + n);
                let group_hits = &page.hits[index..end];
                let id = source.id().to_string();
                let folded = page.folded.contains(&id);
                ui.horizontal(|ui| {
                    let arrow = if folded { "▸" } else { "▾" };
                    let head = ui.add(egui::Button::new(RichText::new(format!("{arrow}  {}", group_hits[0].title)).font(ui_body()).color(TEXT)).frame(false));
                    if head.clicked() {
                        if folded {
                            page.folded.remove(&id);
                        } else {
                            page.folded.insert(id.clone());
                        }
                    }
                    ui.label(RichText::new(group_hits[0].group.label()).font(ui_small()).color(FAINT));
                    ui.label(
                        RichText::new(format!(" {} ", group_hits.len()))
                            .font(ui_small())
                            .color(ACCENT_INK)
                            .background_color(ACCENT_SOFT),
                    );
                    if replacing && ui.small_button("Replace in this one").on_hover_text("Replace every result in this document").clicked() {
                        act = Some(Act::ReplaceIn(source.clone()));
                    }
                });
                if !folded {
                    for hit in group_hits {
                        ui.horizontal(|ui| {
                            ui.add_space(22.0);
                            let resp = ui.add(egui::Label::new(hit_line(hit)).truncate().sense(Sense::click()));
                            if resp.hovered() {
                                ui.painter().rect_filled(resp.rect.expand2(Vec2::new(4.0, 1.0)), CornerRadius::same(3), Color32::from_black_alpha(10));
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if resp.clicked() {
                                act = Some(Act::Open(hit.clone()));
                            }
                            if replacing && ui.small_button("↳ Replace").on_hover_text("Replace just this one").clicked() {
                                act = Some(Act::ReplaceHit(hit.clone()));
                            }
                        });
                    }
                }
                ui.add_space(6.0);
                index = end;
            }
        });
    });

    match act {
        Some(Act::Open(hit)) => app.open_hit(&hit),
        Some(Act::ReplaceHit(hit)) => {
            let field = hit.field.label();
            replace(app, json!({"id": hit.source.id(), "field": field, "start": hit.start}));
        }
        Some(Act::ReplaceIn(source)) => replace(app, json!({"id": source.id()})),
        Some(Act::ReplaceAll) => replace(app, json!({})),
        None => {}
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Work out the answer again if the query changed (or the page was just opened).
fn refresh(app: &mut PortmanteauApp) {
    let query = app.search_page.query();
    if app.search_page.ran.as_ref() == Some(&query) {
        return;
    }
    app.search_page.ran = Some(query.clone());
    if query.text.is_empty() {
        app.search_page.hits.clear();
        app.search_page.error = None;
        return;
    }
    match app.ws.project.find_all(&query) {
        Ok(hits) => {
            app.search_page.hits = hits;
            app.search_page.error = None;
        }
        Err(error) => {
            app.search_page.hits.clear();
            app.search_page.error = Some(error);
        }
    }
}

/// Replace through the command layer, then report.
fn replace(app: &mut PortmanteauApp, mut extra: serde_json::Value) {
    let page = &app.search_page;
    let query = page.query();
    let object = extra.as_object_mut().expect("an object");
    object.insert("query".into(), json!(query.text));
    object.insert("replacement".into(), json!(page.replacement));
    object.insert("case_sensitive".into(), json!(query.options.case_sensitive));
    object.insert("whole_word".into(), json!(query.options.whole_word));
    object.insert("regex".into(), json!(query.options.regex));
    object.insert("asides".into(), json!(query.asides));
    object.insert("groups".into(), json!(query.groups.iter().map(|g| format!("{g:?}")).collect::<Vec<_>>()));
    let response = app.ws.execute("search.replace", extra);
    if response.ok {
        app.toast("Replaced", response.message, ToastKind::Ok);
    } else {
        app.toast("Nothing replaced", response.message, ToastKind::Error);
    }
    app.search_page.stale();
}

fn chip(ui: &mut egui::Ui, text: &str, on: bool, hover: &str) -> bool {
    let button = egui::Button::new(RichText::new(text).font(ui_small()).color(if on { TEXT } else { MUTED }))
        .fill(if on { ACCENT_SOFT } else { PANEL_2 })
        .stroke(Stroke::new(1.0, if on { ACCENT } else { BORDER_SOFT }))
        .corner_radius(CornerRadius::same(10))
        .min_size(Vec2::new(0.0, 20.0));
    ui.add(button).on_hover_text(hover).clicked()
}

/// One result: where in the document ("Notes ·" for a synopsis or notes hit)
/// and the line with the match highlighted.
fn hit_line(hit: &FindHit) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat { font_id: ui_body(), color: MUTED, ..Default::default() };
    if hit.field != portmanteau_core::HitField::Body {
        job.append(&format!("{} · ", hit.field.label()), 0.0, TextFormat { font_id: ui_small(), color: FAINT, ..Default::default() });
    }
    let chars: Vec<char> = hit.excerpt.chars().collect();
    let take = |range: std::ops::Range<usize>| chars[range.start.min(chars.len())..range.end.min(chars.len())].iter().collect::<String>();
    job.append(&take(0..hit.mark.start), 0.0, plain.clone());
    job.append(
        &take(hit.mark.clone()),
        0.0,
        TextFormat { font_id: ui_body(), color: TEXT, background: Color32::from_rgb(250, 226, 140), ..Default::default() },
    );
    job.append(&take(hit.mark.end..chars.len()), 0.0, plain);
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    fn harness(app: PortmanteauApp) -> (egui_kittest::Harness<'static>, std::rc::Rc<std::cell::RefCell<PortmanteauApp>>) {
        let app = std::rc::Rc::new(std::cell::RefCell::new(app));
        let ui_app = std::rc::Rc::clone(&app);
        let mut themed = false;
        let harness = egui_kittest::Harness::builder()
            .with_size(Vec2::new(1440.0, 930.0))
            .with_accessibility_check(false)
            .allow_missing_glyphs()
            .build_ui(move |ui| {
                if !themed {
                    crate::theme::install(ui.ctx());
                    themed = true;
                    return;
                }
                show(&mut ui_app.borrow_mut(), ui);
            });
        (harness, app)
    }

    #[test]
    fn typing_finds_results_and_clicking_one_opens_the_document_with_the_word_selected() {
        let mut app = PortmanteauApp::new(None);
        app.view = crate::app::View::Search;
        app.search_page.query = "compass".into();
        let (mut harness, app) = harness(app);
        harness.run_steps(4);
        assert!(app.borrow().search_page.hits().len() >= 2, "the demo mentions the compass more than once");
        let hit = app.borrow().search_page.hits()[0].clone();
        harness.get_by_label_contains("brass compass").click_accesskit();
        harness.run_steps(3);
        let app = app.borrow();
        assert_eq!(app.view, crate::app::View::Write);
        assert_eq!(app.selected.as_deref(), Some(hit.source.id()));
        let request = app.caret_request.as_ref().expect("the caret is placed");
        assert_eq!((request.paragraph, request.offset, request.select), (hit.paragraph, hit.offset, "compass".len()));
    }

    #[test]
    fn a_bad_pattern_is_explained_and_group_chips_filter() {
        let mut app = PortmanteauApp::new(None);
        app.search_page.query = "(".into();
        app.search_page.options.regex = true;
        let (mut harness, app) = harness(app);
        harness.run_steps(3);
        assert!(app.borrow().search_page.error().is_some());
        assert!(harness.query_by_label_contains("Not a valid pattern").is_some());
        {
            let mut app = app.borrow_mut();
            app.search_page.query = "harbour".into();
            app.search_page.options.regex = false;
            app.search_page.stale();
        }
        harness.run_steps(3);
        let all = app.borrow().search_page.hits().len();
        // The chip comes first on the page; later "Research" labels are result headings.
        harness.get_all_by_label("Research").next().expect("a Research chip").click();
        harness.run_steps(3);
        let research = app.borrow().search_page.hits().len();
        assert!(research > 0 && research < all);
        assert!(app.borrow().search_page.hits().iter().all(|h| h.group == SearchGroup::Research));
    }

    #[test]
    fn replace_all_goes_through_the_command_and_updates_the_results() {
        let mut app = PortmanteauApp::new(None);
        app.search_page.query = "Mara Venn".into();
        app.search_page.replacement = "Mara Vane".into();
        app.search_page.show_replace = true;
        app.search_page.groups = vec![SearchGroup::Manuscript];
        let (mut harness, app) = harness(app);
        harness.run_steps(3);
        assert!(!app.borrow().search_page.hits().is_empty());
        harness.get_by_label("Replace all").click();
        harness.run_steps(4);
        assert!(app.borrow().search_page.hits().is_empty(), "nothing left to find");
        assert!(app.borrow().ws.dirty);
    }
}
