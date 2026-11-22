//! Research: clippings, interviews, data and field notes beside the manuscript.
//!
//! A research note is an ordinary binder document inside the Research folder
//! that also carries research details (`ResearchMeta`): a kind, a source and
//! attached files. This page is a lens on those documents: find them (search,
//! kind filter, sort), read and write them, keep files with them, and see
//! which manuscript documents mention them. Everything that changes the
//! project goes through the `research.*` and `document.*` commands.

use std::sync::Arc;

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};
use portmanteau_core::{Document, StyleSheet, RESEARCH_KINDS};
use serde_json::json;

use crate::app::{ImageInsertState, PortmanteauApp, RichTextTarget, ToastKind};
use crate::theme::*;
use crate::ui::editor::{
    inline_format_toolbar, load_rich_image_textures, paint_rich_images, script_flow_layout_job,
};
use crate::ui::views::{content_frame, meta_line, page_head, primary_button, quiet_button};

// ---------------------------------------------------------------------------
// What a kind means
// ---------------------------------------------------------------------------

/// The colour a kind is drawn in, in the list and the editor.
pub fn kind_color(kind: &str) -> Color32 {
    match kind {
        "Interview" => PURPLE,
        "Data" => BLUE,
        "Archive" => ACCENT,
        "Image" => RED,
        "Place" => Color32::from_rgb(166, 124, 52),
        _ => SAGE,
    }
}

/// What the "source" line asks for, by kind: an interview wants who and when,
/// a place wants where. The same field, said the way that kind needs it.
pub fn source_label(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "Interview" => ("Who, and when", "Name, role, date and where it was recorded"),
        "Data" => ("Dataset", "Where the figures come from, and the period they cover"),
        "Archive" => ("Archive & reference", "Collection, box, folio or catalogue number"),
        "Image" => ("Credit", "Photographer, collection, licence"),
        "Place" => ("Where", "Address, coordinates or a description of the spot"),
        _ => ("Source", "Where it came from"),
    }
}

// ---------------------------------------------------------------------------
// The list: filter, search, sort
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Sort {
    /// The order of the binder's Research folder.
    #[default]
    Binder,
    Title,
    Kind,
    Recent,
}

impl Sort {
    fn label(self) -> &'static str {
        match self {
            Sort::Binder => "Binder order",
            Sort::Title => "Title",
            Sort::Kind => "Kind",
            Sort::Recent => "Recently edited",
        }
    }
}

/// What the list shows right now. Kept in egui's memory for the session.
#[derive(Clone, Default)]
struct ListState {
    search: String,
    kind: Option<String>,
    sort: Sort,
    /// A note the writer has asked to delete and must confirm.
    confirm_delete: Option<String>,
}

pub fn kind_of(doc: &Document) -> &str {
    doc.research.as_ref().map_or("Note", |meta| meta.kind.as_str())
}

/// The notes to list, filtered and sorted. Search looks at the title, the
/// source and the text, without regard to case.
pub fn visible_notes<'a>(
    notes: Vec<&'a Document>,
    search: &str,
    kind: Option<&str>,
    sort: Sort,
) -> Vec<&'a Document> {
    let needle = search.trim().to_lowercase();
    let mut out: Vec<&Document> = notes
        .into_iter()
        .filter(|doc| kind.is_none_or(|k| kind_of(doc) == k))
        .filter(|doc| {
            needle.is_empty()
                || doc.title.to_lowercase().contains(&needle)
                || doc.research.as_ref().is_some_and(|m| m.source.to_lowercase().contains(&needle))
                || doc.content.plain_text().to_lowercase().contains(&needle)
        })
        .collect();
    match sort {
        Sort::Binder => {}
        Sort::Title => out.sort_by_key(|doc| doc.title.to_lowercase()),
        Sort::Kind => out.sort_by(|a, b| (kind_of(a), a.title.to_lowercase()).cmp(&(kind_of(b), b.title.to_lowercase()))),
        Sort::Recent => out.sort_by(|a, b| b.updated.cmp(&a.updated)),
    }
    out
}

/// Manuscript documents whose text or notes mention this note by title: where
/// the research is already being used. Short titles are ignored (too common).
pub fn mentioned_in<'a>(project: &'a portmanteau_core::Project, note: &Document) -> Vec<&'a Document> {
    let title = note.title.trim().to_lowercase();
    if title.chars().count() < 5 {
        return Vec::new();
    }
    project
        .ordered_documents()
        .into_iter()
        .filter(|doc| !doc.kind.is_folder() && !project.in_research(doc) && doc.id != note.id)
        .filter(|doc| {
            doc.content.plain_text().to_lowercase().contains(&title)
                || doc.notes.plain_text().to_lowercase().contains(&title)
                || doc.synopsis.plain_text().to_lowercase().contains(&title)
        })
        .collect()
}

/// Research notes a manuscript document names by title: the other direction of
/// `mentioned_in`, for the inspector.
pub fn notes_mentioned_by<'a>(project: &'a portmanteau_core::Project, doc: &Document) -> Vec<&'a Document> {
    let text = format!(
        "{}\n{}\n{}",
        doc.content.plain_text(),
        doc.notes.plain_text(),
        doc.synopsis.plain_text()
    )
    .to_lowercase();
    project
        .research_docs()
        .into_iter()
        .filter(|note| note.id != doc.id)
        .filter(|note| {
            let title = note.title.trim().to_lowercase();
            title.chars().count() >= 5 && text.contains(&title)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The page
// ---------------------------------------------------------------------------

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    egui_extras::install_image_loaders(ui.ctx());
    let mut add = false;
    let mut clip = false;
    page_head(
        ui,
        "Sources",
        "Research",
        "Keep the world of the story close: interviews, archives, images, arguments with yourself.",
        |ui| {
            if primary_button(ui, "+ New note").clicked() {
                add = true;
            }
            if quiet_button(ui, "Paste clipping").on_hover_text("Make a note from what is on the clipboard").clicked() {
                clip = true;
            }
        },
    );

    let state_id = ui.make_persistent_id("research-list-state");
    let mut state: ListState = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();

    let notes = app.ws.project.research_docs();
    let all_ids: Vec<String> = notes.iter().map(|d| d.id.clone()).collect();
    if !all_ids.is_empty() && !app.selected_research.as_ref().is_some_and(|id| all_ids.contains(id)) {
        app.selected_research = all_ids.first().cloned();
    }

    content_frame().show(ui, |ui| {
        if all_ids.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No research yet").font(serif(20.0)).color(TEXT));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("A folder of small, useful things. Start with one note, or paste a clipping.")
                        .font(ui_body())
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
            ui.allocate_ui_with_layout(Vec2::new(300.0, height), Layout::top_down(Align::Min), |ui| {
                list_panel(app, ui, &mut state, height);
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                if let Some(id) = app.selected_research.clone() {
                    editor(app, ui, &id, &mut state, height);
                }
            });
        });
    });

    ui.data_mut(|d| d.insert_temp(state_id, state));

    if add {
        run_add(app, "Untitled note", "", None);
    }
    if clip {
        paste_clipping(app);
    }
    accept_dropped_files(app, ui.ctx());
}

fn run_add(app: &mut PortmanteauApp, title: &str, body: &str, source: Option<&str>) {
    let mut payload = json!({"title": title, "body": body, "kind": "Note"});
    if let Some(source) = source {
        payload["source"] = json!(source);
    }
    let res = app.ws.execute("research.add", payload);
    if res.ok {
        if let Some(id) = res.data.get("id").and_then(|v| v.as_str()) {
            app.selected_research = Some(id.to_string());
        }
    } else {
        app.toast("Could not add the note", res.message, ToastKind::Error);
    }
}

/// A note from whatever text is on the clipboard: the first line (or the web
/// address, if that is all there is) names it.
fn paste_clipping(app: &mut PortmanteauApp) {
    let text = match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) => {
            app.toast("Clipboard is empty", "Copy some text first.", ToastKind::Info);
            return;
        }
        Err(e) => {
            app.toast("No text on the clipboard", e.to_string(), ToastKind::Info);
            return;
        }
    };
    let first = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Clipping");
    let title: String = first.chars().take(70).collect();
    let source = (first.starts_with("http://") || first.starts_with("https://")).then_some(first);
    run_add(app, &title, text.trim(), source);
    // Web addresses in a clipping become links.
    if let Some(doc) = app.selected_research.clone().and_then(|id| app.ws.project.document_mut(&id)) {
        doc.content.autolink_urls();
    }
}

/// Files dropped on the window become attachments of the open note.
fn accept_dropped_files(app: &mut PortmanteauApp, ctx: &egui::Context) {
    let dropped: Vec<std::path::PathBuf> = ctx
        .input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| p.is_file()).collect());
    let Some(id) = app.selected_research.clone() else { return };
    for path in dropped {
        attach(app, &id, &path);
    }
}

fn attach(app: &mut PortmanteauApp, id: &str, path: &std::path::Path) {
    let res = app.ws.execute("research.attach", json!({"id": id, "path": path.display().to_string()}));
    if res.ok {
        app.toast("File attached", res.message, ToastKind::Ok);
    } else {
        app.toast("Could not attach the file", res.message, ToastKind::Error);
    }
}

// ---------------------------------------------------------------------------
// Left: the sourcebook
// ---------------------------------------------------------------------------

fn list_panel(app: &mut PortmanteauApp, ui: &mut egui::Ui, state: &mut ListState, height: f32) {
    let notes = app.ws.project.research_docs();
    let total = notes.len();
    let counts: Vec<(&str, usize)> = RESEARCH_KINDS
        .iter()
        .map(|kind| (*kind, notes.iter().filter(|d| kind_of(d) == *kind).count()))
        .collect();
    let shown = visible_notes(notes, &state.search, state.kind.as_deref(), state.sort);
    let shown: Vec<(String, String, String, String, usize, usize, bool)> = shown
        .iter()
        .map(|d| {
            let meta = d.research.clone().unwrap_or_default();
            (d.id.clone(), d.title.clone(), meta.kind, meta.source, meta.attachments.len(), d.words() as usize, false)
        })
        .collect();

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
                    let label = if shown.len() == total {
                        format!("{total} notes")
                    } else {
                        format!("{} of {total}", shown.len())
                    };
                    meta_line(ui, &[&label]);
                });
            });
            ui.add_space(4.0);
            ui.add(
                TextEdit::singleline(&mut state.search)
                    .hint_text("Search titles, sources and text")
                    .font(ui_small())
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                if chip(ui, &format!("All {total}"), state.kind.is_none(), TEXT) {
                    state.kind = None;
                }
                for (kind, count) in counts.iter().filter(|(_, n)| *n > 0) {
                    let active = state.kind.as_deref() == Some(*kind);
                    if chip(ui, &format!("{kind} {count}"), active, kind_color(kind)) {
                        state.kind = if active { None } else { Some(kind.to_string()) };
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Sort").font(ui_small()).color(FAINT));
                egui::ComboBox::from_id_salt("research_sort")
                    .selected_text(RichText::new(state.sort.label()).font(ui_small()).color(MUTED))
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for sort in [Sort::Binder, Sort::Title, Sort::Kind, Sort::Recent] {
                            ui.selectable_value(&mut state.sort, sort, sort.label());
                        }
                    });
            });
            ui.add_space(6.0);
            if shown.is_empty() {
                ui.add_space(10.0);
                ui.label(RichText::new("Nothing matches. Clear the search or the kind filter.").font(ui_small()).color(FAINT));
            }
            ScrollArea::vertical()
                .id_salt("research_list")
                .auto_shrink([false, false])
                .max_height((height - 170.0).max(120.0))
                .show(ui, |ui| {
                    for (id, title, kind, source, files, words, _) in &shown {
                        let selected = app.selected_research.as_deref() == Some(id.as_str());
                        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 50.0), Sense::click());
                        // The row is painted by hand, so say what it is for screen readers and tests.
                        resp.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::Role::Button,
                                true,
                                selected,
                                format!("{title} · {kind}"),
                            )
                        });
                        if selected {
                            ui.painter().rect_filled(rect, CornerRadius::same(5), ACCENT_SOFT);
                        } else if resp.hovered() {
                            ui.painter().rect_filled(rect, CornerRadius::same(5), PANEL_2);
                        }
                        let color = kind_color(kind);
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(rect.min + Vec2::new(3.0, 8.0), Vec2::new(3.0, rect.height() - 16.0)),
                            CornerRadius::same(2),
                            color,
                        );
                        let text_rect = egui::Rect::from_min_max(
                            egui::pos2(rect.left() + 14.0, rect.top()),
                            egui::pos2(rect.right() - 6.0, rect.bottom()),
                        );
                        let clip = ui.painter().with_clip_rect(text_rect);
                        clip.text(
                            text_rect.left_top() + Vec2::new(0.0, 7.0),
                            egui::Align2::LEFT_TOP,
                            title,
                            ui_body(),
                            if selected { TEXT } else { MUTED },
                        );
                        let mut detail = kind.clone();
                        if !source.is_empty() {
                            detail.push_str(&format!(" · {source}"));
                        }
                        clip.text(text_rect.left_top() + Vec2::new(0.0, 25.0), egui::Align2::LEFT_TOP, detail, ui_small(), FAINT);
                        let mut badges = format!("{words} w");
                        if *files > 0 {
                            badges = format!("📎{files}  {badges}");
                        }
                        ui.painter().text(
                            egui::pos2(rect.right() - 8.0, rect.top() + 9.0),
                            egui::Align2::RIGHT_TOP,
                            badges,
                            mono(8.5),
                            FAINT,
                        );
                        if resp.clicked() {
                            app.selected_research = Some(id.clone());
                            state.confirm_delete = None;
                        }
                    }
                });
        });
}

fn chip(ui: &mut egui::Ui, text: &str, active: bool, color: Color32) -> bool {
    let button = egui::Button::new(RichText::new(text).font(ui_small()).color(if active { TEXT } else { MUTED }))
        .fill(if active { color.gamma_multiply(0.22) } else { PANEL_2 })
        .stroke(Stroke::new(1.0, if active { color } else { BORDER_SOFT }))
        .corner_radius(CornerRadius::same(10))
        .min_size(Vec2::new(0.0, 22.0));
    ui.add(button).clicked()
}

// ---------------------------------------------------------------------------
// Right: the note
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Action {
    AddFiles,
    Open(std::path::PathBuf),
    Reveal(std::path::PathBuf),
    Remove(String),
    OpenDoc(String),
    Delete,
}

fn editor(app: &mut PortmanteauApp, ui: &mut egui::Ui, id: &str, state: &mut ListState, height: f32) {
    let Some(doc) = app.ws.project.document(id) else { return };
    let old_title = doc.title.clone();
    let mut rich_body = doc.content.clone();
    let old_body = rich_body.editor_text();
    let meta = doc.research.clone().unwrap_or_default();
    let used_in: Vec<(String, String)> = mentioned_in(&app.ws.project, doc)
        .into_iter()
        .take(8)
        .map(|d| (d.id.clone(), d.title.clone()))
        .collect();
    let body_images = load_rich_image_textures(app, ui.ctx(), &rich_body);
    let mut title = old_title.clone();
    let mut body = old_body.clone();
    let mut kind = meta.kind.clone();
    let mut source = meta.source.clone();
    let mut changed = false;
    let mut actions: Vec<Action> = Vec::new();
    let mut styles = app.ws.project.styles.clone();
    styles.body.size = 13.5;
    let sheet = StyleSheet::novel_prose(&styles);
    // Each file's real path, checked to be inside the project (see
    // `Workspace::research_file_path`); `None` for a missing or unsafe one.
    let files: Vec<(String, Option<std::path::PathBuf>)> = meta
        .attachments
        .iter()
        .map(|name| (name.clone(), app.ws.research_file_path(id, name)))
        .collect();
    let saved = app.ws.path.is_some();
    let hovering_files = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());

    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(if hovering_files { 2.0 } else { 1.0 }, if hovering_files { ACCENT } else { BORDER_SOFT }))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ScrollArea::vertical()
                .id_salt(("research_note", id))
                .auto_shrink([false, false])
                .max_height((height - 40.0).max(260.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // ---- Header ----------------------------------------
                    ui.horizontal(|ui| {
                        let color = kind_color(&kind);
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(9.0), Sense::hover());
                        ui.painter().circle_filled(dot.center(), 4.0, color);
                        section_label(ui, "Research note");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            meta_line(ui, &[&format!("{} words", portmanteau_core::count_words(&body))]);
                        });
                    });
                    ui.add_space(5.0);
                    if ui
                        .add(
                            TextEdit::singleline(&mut title)
                                .font(serif(22.0))
                                .frame(Frame::NONE)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.add_space(6.0);
                    let (source_name, source_hint) = source_label(&kind);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Kind").font(ui_small()).color(FAINT));
                        egui::ComboBox::from_id_salt("research_kind")
                            .selected_text(RichText::new(&kind).font(ui_small()).color(kind_color(&kind)))
                            .width(118.0)
                            .show_ui(ui, |ui| {
                                for option in RESEARCH_KINDS {
                                    if ui
                                        .selectable_value(&mut kind, option.to_string(), RichText::new(option).color(kind_color(option)))
                                        .clicked()
                                    {
                                        changed = true;
                                    }
                                }
                            });
                        ui.add_space(8.0);
                        ui.label(RichText::new(source_name).font(ui_small()).color(FAINT));
                        if ui
                            .add(
                                TextEdit::singleline(&mut source)
                                    .hint_text(source_hint)
                                    .font(ui_small())
                                    .desired_width(ui.available_width().max(160.0)),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    });
                    ui.add_space(8.0);
                    hairline(ui);
                    ui.add_space(10.0);

                    // ---- Text ------------------------------------------
                    let rows = ((body.chars().count() as f32) / 75.0).ceil().max(12.0) as usize;
                    let checker = &app.spell;
                    let spell_on = app.settings.spellcheck;
                    let body_spans = crate::ui::editor::field_spans(checker, spell_on, &body);
                    let body_id = egui::Id::unique(("research_body", id));
                    let ctx = ui.ctx().clone();
                    let toolbar = inline_format_toolbar(ui, &ctx, body_id, &mut rich_body);
                    if toolbar.changed {
                        changed = true;
                    }
                    if let Some(range) = toolbar.insert_image {
                        app.image_insert = Some(ImageInsertState {
                            target: RichTextTarget::DocumentBody(id.to_owned()),
                            range,
                            path: String::new(),
                            alt_text: String::new(),
                            caption: String::new(),
                            standalone: false,
                            error: String::new(),
                        });
                    }
                    let (rich_layout, styles_layout, sheet_layout) = (rich_body.clone(), styles.clone(), sheet.clone());
                    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                        let job = script_flow_layout_job(
                            text.as_str(),
                            &rich_layout,
                            &sheet_layout,
                            &styles_layout,
                            field_text_zoom(),
                            wrap_width,
                            &body_spans,
                        );
                        ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
                    };
                    let body_output = TextEdit::multiline(&mut body)
                        .id(body_id)
                        .layouter(&mut layouter)
                        .font(serif(13.5))
                        .desired_rows(rows.min(40))
                        .desired_width(f32::INFINITY)
                        .frame(Frame::NONE)
                        .hint_text("What did you find, and why does it matter to the story?")
                        .show(ui);
                    if body_output.response.changed() {
                        rich_body.sync_editor_text(&body, &sheet);
                        crate::ui::editor::autolink_pasted(ui.ctx(), &mut rich_body, body_output.cursor_range.map(|r| r.primary.index.0));
                        changed = true;
                    }
                    crate::ui::editor::remember_selection_anchor(ui.ctx(), body_output.response.id, &body_output.galley, body_output.galley_pos);
                    let fixed = crate::ui::editor::field_spell_menu(
                        ui,
                        checker,
                        spell_on,
                        egui::Id::unique(("spell_menu_research", id)),
                        &body_output,
                        &body,
                        &mut rich_body,
                        &sheet,
                    );
                    if fixed.replaced {
                        changed = true;
                    }
                    crate::ui::editor::apply_field_spell(app, vec![fixed]);
                    paint_rich_images(
                        ui,
                        &body_output.galley,
                        body_output.galley_pos,
                        body_output.response.rect,
                        &rich_body,
                        &body_images,
                        field_text_zoom(),
                    );

                    // ---- Files -----------------------------------------
                    ui.add_space(14.0);
                    hairline(ui);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        section_label(ui, "Files");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if quiet_button(ui, "Add files…").clicked() {
                                actions.push(Action::AddFiles);
                            }
                            meta_line(
                                ui,
                                &[&format!("{} file{}", meta.attachments.len(), if meta.attachments.len() == 1 { "" } else { "s" })],
                            );
                        });
                    });
                    ui.add_space(6.0);
                    if !saved {
                        ui.label(
                            RichText::new("Save the project to attach files: they are copied into its folder.")
                                .font(ui_small())
                                .color(FAINT),
                        );
                    } else if meta.attachments.is_empty() {
                        ui.label(
                            RichText::new("Add files with the button, or drop them on this window. They are copied into the project; nothing leaves this machine.")
                                .font(ui_small())
                                .color(FAINT),
                        );
                    }
                    if !meta.attachments.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                            for (name, path) in &files {
                                file_card(ui, id, name, path.as_deref(), &mut actions);
                            }
                        });
                    }

                    // ---- Where it is used -------------------------------
                    if !used_in.is_empty() {
                        ui.add_space(14.0);
                        hairline(ui);
                        ui.add_space(10.0);
                        section_label(ui, "Mentioned in");
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            for (doc_id, doc_title) in &used_in {
                                if ui
                                    .add(egui::Button::new(RichText::new(doc_title).font(ui_small()).color(BLUE)).frame(false))
                                    .on_hover_text("Open this document")
                                    .clicked()
                                {
                                    actions.push(Action::OpenDoc(doc_id.clone()));
                                }
                            }
                        });
                    }

                    // ---- Delete ----------------------------------------
                    ui.add_space(14.0);
                    hairline(ui);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if state.confirm_delete.as_deref() == Some(id) {
                            ui.label(RichText::new("Delete this note and its files?").font(ui_small()).color(RED_INK));
                            if ui.button("Delete").clicked() {
                                actions.push(Action::Delete);
                            }
                            if ui.button("Keep").clicked() {
                                state.confirm_delete = None;
                            }
                        } else if quiet_button(ui, "Delete note").clicked() {
                            state.confirm_delete = Some(id.to_string());
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            meta_line(ui, &["It is also in the binder, under Research"]);
                        });
                    });
                });
        });

    if changed {
        rich_body.sync_editor_text(&body, &sheet);
        if let Some(doc) = app.ws.project.document_mut(id) {
            doc.title = title;
            doc.content = rich_body;
            let meta = doc.research_meta();
            meta.kind = kind;
            meta.source = source;
            doc.touch();
        }
        app.ws.dirty = true;
    }

    for action in actions {
        match action {
            Action::AddFiles => {
                if let Some(paths) = rfd::FileDialog::new().set_title("Add files to this note").pick_files() {
                    for path in paths {
                        attach(app, id, &path);
                    }
                }
            }
            Action::Open(path) => crate::app::open_path(&path),
            Action::Reveal(path) => {
                if let Some(parent) = path.parent() {
                    crate::app::open_path(parent);
                }
            }
            Action::Remove(name) => {
                let res = app.ws.execute("research.detach", json!({"id": id, "name": name}));
                if !res.ok {
                    app.toast("Could not remove the file", res.message, ToastKind::Error);
                }
            }
            Action::OpenDoc(doc) => app.open_doc(doc),
            Action::Delete => {
                let res = app.ws.execute("research.delete", json!({"id": id}));
                if !res.ok {
                    app.toast("Could not delete the note", res.message, ToastKind::Error);
                }
                state.confirm_delete = None;
                app.selected_research = app.ws.project.research_docs().first().map(|d| d.id.clone());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

const CARD_W: f32 = 168.0;

/// One attached file: a thumbnail for pictures, an icon otherwise, its name
/// and what you can do with it.
fn file_card(ui: &mut egui::Ui, id: &str, name: &str, path: Option<&std::path::Path>, actions: &mut Vec<Action>) {
    let kind = attachment_kind(name);
    let path = path.map(std::path::Path::to_path_buf);
    let size = path.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len());
    Frame::new()
        .fill(PANEL_2)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(CARD_W);
            ui.set_max_width(CARD_W);
            let (area, _) = ui.allocate_exact_size(Vec2::new(CARD_W, 96.0), Sense::hover());
            let bytes = (kind == "Image").then(|| path.as_ref().and_then(|p| cached_bytes(ui.ctx(), p))).flatten();
            match (&path, bytes) {
                (Some(path), Some(bytes)) => {
                    let uri = format!("bytes://research/{id}/{name}?v={}", stamp(path));
                    egui::Image::from_bytes(uri, egui::load::Bytes::from(bytes.to_vec()))
                        .fit_to_exact_size(area.size())
                        .maintain_aspect_ratio(true)
                        .corner_radius(4)
                        .paint_at(ui, area);
                }
                _ => {
                    ui.painter().rect_filled(area, CornerRadius::same(4), BG);
                    let ext = std::path::Path::new(name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("file")
                        .to_uppercase();
                    ui.painter().text(area.center(), egui::Align2::CENTER_CENTER, ext, serif(22.0), MUTED);
                }
            }
            ui.add_space(2.0);
            ui.add(egui::Label::new(RichText::new(name).font(ui_small()).color(TEXT)).truncate());
            let detail = match size {
                Some(bytes) => format!("{kind} · {}", human_size(bytes)),
                None => format!("{kind} · missing from the project folder"),
            };
            ui.label(RichText::new(detail).font(mono(8.5)).color(FAINT));
            ui.horizontal(|ui| {
                if let Some(path) = &path {
                    if size.is_some() {
                        if ui.add(egui::Button::new(RichText::new("Open").font(ui_small()).color(BLUE)).frame(false)).clicked() {
                            actions.push(Action::Open(path.clone()));
                        }
                        if ui.add(egui::Button::new(RichText::new("Show").font(ui_small()).color(BLUE)).frame(false)).on_hover_text("Show in the file manager").clicked() {
                            actions.push(Action::Reveal(path.clone()));
                        }
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new(RichText::new("Remove").font(ui_small()).color(MUTED)).frame(false)).clicked() {
                        actions.push(Action::Remove(name.to_string()));
                    }
                });
            });
        });
}

fn stamp(path: &std::path::Path) -> u128 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos())
}

/// A file's bytes, read once and kept until the file changes.
fn cached_bytes(ctx: &egui::Context, path: &std::path::Path) -> Option<Arc<Vec<u8>>> {
    let key = egui::Id::unique(("research-bytes", path.display().to_string()));
    let version = stamp(path);
    if let Some((cached, bytes)) = ctx.data(|d| d.get_temp::<(u128, Arc<Vec<u8>>)>(key)) {
        if cached == version {
            return Some(bytes);
        }
    }
    let bytes = Arc::new(std::fs::read(path).ok()?);
    ctx.data_mut(|d| d.insert_temp(key, (version, bytes.clone())));
    Some(bytes)
}

pub fn human_size(bytes: u64) -> String {
    match bytes {
        0..=1023 => format!("{bytes} B"),
        1024..=1_048_575 => format!("{:.0} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

pub fn attachment_kind(name: &str) -> &'static str {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "Image",
        "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" => "Audio",
        "mp4" | "mov" | "mkv" | "webm" | "avi" => "Video",
        "pdf" => "PDF",
        "doc" | "docx" | "rtf" | "txt" | "md" => "Document",
        "html" | "htm" => "Web page",
        _ => "File",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use portmanteau_core::sample;

    #[test]
    fn search_and_kind_filter_narrow_the_list_and_sorts_reorder_it() {
        let project = sample::demo();
        let all = project.research_docs();
        assert!(all.len() >= 4);
        let titles = |notes: &[&Document]| notes.iter().map(|d| d.title.clone()).collect::<Vec<_>>();

        let interviews = visible_notes(all.clone(), "", Some("Interview"), Sort::Binder);
        assert!(!interviews.is_empty() && interviews.iter().all(|d| kind_of(d) == "Interview"));

        // Search reaches the text and the source, not only the title.
        let by_text = visible_notes(all.clone(), "ballast", None, Sort::Binder);
        assert_eq!(titles(&by_text), ["Tide tables, 1987–present"]);
        let by_source = visible_notes(all.clone(), "county records", None, Sort::Binder);
        assert_eq!(titles(&by_source), ["Wreck of the Anna Rose"]);
        assert!(visible_notes(all.clone(), "zzzz-no-such", None, Sort::Binder).is_empty());

        let sorted = titles(&visible_notes(all.clone(), "", None, Sort::Title));
        let mut expected = sorted.clone();
        expected.sort_by_key(|t| t.to_lowercase());
        assert_eq!(sorted, expected);
    }

    #[test]
    fn each_kind_asks_for_its_source_in_its_own_words() {
        let labels: Vec<_> = RESEARCH_KINDS.iter().map(|k| source_label(k).0).collect();
        let mut unique = labels.clone();
        unique.sort();
        unique.dedup();
        assert!(unique.len() >= 5, "kinds read differently: {labels:?}");
        assert_eq!(source_label("Interview").0, "Who, and when");
        assert_eq!(source_label("nonsense").0, "Source");
    }

    #[test]
    fn a_note_is_mentioned_where_the_manuscript_names_it() {
        let mut project = sample::demo();
        let note = project.research_docs().into_iter().find(|d| d.title.starts_with("Wreck")).unwrap().clone();
        assert!(mentioned_in(&project, &note).is_empty());
        let chapter = project.documents.iter().find(|d| d.kind == portmanteau_core::DocKind::Chapter).unwrap().id.clone();
        project.document_mut(&chapter).unwrap().notes =
            portmanteau_core::RichDocument::from_plain_text("Check the wreck of the anna rose before drafting.", "body");
        let found = mentioned_in(&project, &note);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, chapter);
        // Research notes never count as mentions, and tiny titles are ignored.
        let mut tiny = note.clone();
        tiny.title = "Tide".into();
        assert!(mentioned_in(&project, &tiny).is_empty());
    }

    #[test]
    fn sizes_read_naturally() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2 KB");
        assert_eq!(human_size(5 * 1_048_576), "5.0 MB");
    }

    fn research_harness(app: PortmanteauApp) -> (egui_kittest::Harness<'static>, std::rc::Rc<std::cell::RefCell<PortmanteauApp>>) {
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
    fn the_research_page_lists_the_notes_and_a_filter_chip_narrows_them() {
        use egui_kittest::kittest::Queryable;
        let (mut harness, app) = research_harness(PortmanteauApp::new(None));
        harness.run_steps(4);
        assert!(harness.query_by_label("SOURCEBOOK").is_some());
        assert!(harness.query_by_label_contains("Wreck of the Anna Rose").is_some());
        harness.get_by_label_contains("Interview 1").click();
        harness.run_steps(3);
        assert!(harness.query_by_label_contains("Wreck of the Anna Rose").is_none(), "filtered out");
        assert!(harness.query_by_label_contains("Interview · harbour master").is_some());
        assert!(app.borrow().selected_research.is_some());
    }

    #[test]
    fn an_empty_research_folder_invites_the_first_note() {
        use egui_kittest::kittest::Queryable;
        let mut app = PortmanteauApp::new(None);
        let ids: Vec<String> = app.ws.project.research_docs().iter().map(|d| d.id.clone()).collect();
        for id in ids {
            app.ws.project.delete_document(&id);
        }
        let (mut harness, app) = research_harness(app);
        harness.run_steps(3);
        assert!(harness.query_by_label("No research yet").is_some());
        harness.get_by_label("Add the first note").click();
        harness.run_steps(3);
        assert_eq!(app.borrow().ws.project.research_docs().len(), 1);
    }

    /// Select some text in a note, press Ctrl/Cmd+K, give an address, and the
    /// selection becomes a link in the note itself.
    #[test]
    fn ctrl_k_links_the_selected_text_of_a_research_note() {
        use egui_kittest::kittest::Queryable;
        let (mut harness, app) = research_harness(PortmanteauApp::new(None));
        harness.run_steps(4);
        let id = app.borrow().selected_research.clone().unwrap();
        let body_id = egui::Id::unique(("research_body", id.as_str()));
        // Focus the body, select "Mean low water" (the first 14 characters), then Ctrl/Cmd+K.
        harness.ctx.memory_mut(|m| m.request_focus(body_id));
        let mut state = egui::text_edit::TextEditState::load(&harness.ctx, body_id).unwrap_or_default();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(14),
        )));
        state.store(&harness.ctx, body_id);
        harness.run_steps(2);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.run_steps(3);
        assert!(harness.query_by_label("Address").is_some(), "the link dialog opened");
        // The dialog's address field is the last text box on screen.
        harness.get_all_by_role(egui::accesskit::Role::TextInput).last().expect("address field").type_text("example.org/tides");
        harness.run_steps(2);
        harness.get_by_label("Apply link").click();
        harness.run_steps(3);
        let app = app.borrow();
        let links = app.ws.project.document(&id).unwrap().content.links();
        assert_eq!(links, [(0..14, "https://example.org/tides".to_string())]);
        assert!(app.ws.dirty);
    }
}
