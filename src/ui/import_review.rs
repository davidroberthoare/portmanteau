//! The import review: see what a file will become before anything is added.
//!
//! Choose a file and the importer reads it (`document.import_preview`, which
//! changes nothing). The review lists the documents it would create, the title
//! page details it found, and for Word and Final Draft files every paragraph
//! style with what it becomes. Everything can be edited; the choices that are
//! about the *format* (split, details, style mappings) are remembered, so the
//! next file from the same program arrives already mapped.
//!
//! All the work goes through the command layer: this module builds payloads
//! and reads responses.

use std::collections::BTreeMap;
use std::path::Path;

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, TextEdit, Vec2,
};
use portmanteau_api::Workspace;
use portmanteau_core::{ImportPrefs, Medium, StyleSheet};
use serde_json::{json, Value};

use crate::app::{PortmanteauApp, ToastKind, View};
use crate::theme::*;

/// The writer's choice for one document the file would create.
#[derive(Clone, Debug, PartialEq)]
pub struct DocEdit {
    pub include: bool,
    pub title: String,
    pub kind: String,
}

/// A title-page detail the file carries, and whether to apply it.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailEdit {
    pub key: &'static str,
    pub label: &'static str,
    pub value: String,
    pub apply: bool,
}

#[derive(Clone, Debug)]
pub struct ImportReview {
    pub path: String,
    /// "auto", or a medium name from `Medium::label`.
    pub medium: String,
    pub split: bool,
    /// Where the documents go; `None` is the Draft folder.
    pub parent: Option<String>,
    /// Source style name to target token, as the writer has set them.
    pub overrides: BTreeMap<String, String>,
    pub preview: Option<Value>,
    pub error: Option<String>,
    pub docs: Vec<DocEdit>,
    pub details: Vec<DetailEdit>,
    /// The saved mapping for this format has been applied (once per file).
    saved_map_applied: bool,
    /// The file or an option changed; read it again.
    stale: bool,
}

impl ImportReview {
    pub fn new(prefs: &ImportPrefs) -> Self {
        Self {
            path: String::new(),
            medium: "auto".into(),
            split: prefs.split,
            parent: None,
            overrides: BTreeMap::new(),
            preview: None,
            error: None,
            docs: Vec::new(),
            details: Vec::new(),
            saved_map_applied: false,
            stale: false,
        }
    }

    /// Start a review of `path` straight away (drag-and-drop, command line).
    pub fn for_path(prefs: &ImportPrefs, path: &str) -> Self {
        let mut review = Self::new(prefs);
        review.set_path(path);
        review
    }

    pub fn set_path(&mut self, path: &str) {
        if self.path != path {
            self.path = path.to_string();
            self.overrides.clear();
            self.saved_map_applied = false;
            self.stale = !path.trim().is_empty();
        }
    }

    /// An option changed: the preview must be read again before the next draw.
    pub fn mark_stale(&mut self) {
        self.stale = true;
    }

    pub fn is_stale(&self) -> bool {
        self.stale
    }

    fn format_key(&self) -> Option<String> {
        self.preview.as_ref()?.get("format_key")?.as_str().map(str::to_string)
    }

    /// The command payload for a preview (`for_import` false) or the import.
    pub fn payload(&self, for_import: bool) -> Value {
        let mut payload = json!({
            "path": self.path.trim(),
            "split": self.split,
            "style_map": self.overrides,
        });
        if self.medium != "auto" {
            payload["medium"] = json!(self.medium);
        }
        if for_import {
            if let Some(parent) = &self.parent {
                payload["parent"] = json!(parent);
            }
            payload["documents"] = json!(self
                .docs
                .iter()
                .map(|d| json!({"include": d.include, "title": d.title, "kind": d.kind}))
                .collect::<Vec<_>>());
            let details: serde_json::Map<String, Value> = self
                .details
                .iter()
                .filter(|d| d.apply)
                .map(|d| (d.key.to_string(), json!(d.value)))
                .collect();
            payload["details"] = Value::Object(details);
        }
        payload
    }

    /// Read the file with the current options and rebuild what the review
    /// shows, keeping the writer's edits where the document list is unchanged.
    pub fn refresh(&mut self, ws: &mut Workspace, prefs: &ImportPrefs) {
        self.stale = false;
        let response = ws.execute("document.import_preview", self.payload(false));
        if !response.ok {
            self.preview = None;
            self.error = Some(response.message);
            self.docs.clear();
            self.details.clear();
            return;
        }
        self.error = None;
        let data = response.data;

        // The file's format has a saved mapping the writer has not overridden:
        // apply it and read again so the preview reflects it.
        if !self.saved_map_applied {
            self.saved_map_applied = true;
            if let Some(key) = data.get("format_key").and_then(|v| v.as_str()) {
                if let Some(saved) = prefs.style_maps.get(key) {
                    if !saved.is_empty() && self.overrides.is_empty() {
                        self.overrides = saved.clone();
                        self.preview = Some(data);
                        return self.refresh(ws, prefs);
                    }
                }
            }
        }

        let listed = data["documents"].as_array().cloned().unwrap_or_default();
        let fresh: Vec<DocEdit> = listed
            .iter()
            .map(|d| DocEdit {
                include: true,
                title: d["title"].as_str().unwrap_or_default().to_string(),
                kind: d["kind"].as_str().unwrap_or("Chapter").to_string(),
            })
            .collect();
        if self.docs.len() != fresh.len() {
            self.docs = fresh;
        } else {
            // Same shape: keep inclusion and kind, but follow titles the
            // writer did not edit.
            for (edit, new) in self.docs.iter_mut().zip(fresh) {
                if edit.title.is_empty() {
                    edit.title = new.title;
                }
            }
        }

        let found = |key: &str| data[key].as_str().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
        let mut details = Vec::new();
        for (key, label) in [("title", "Title"), ("author", "Author"), ("contact", "Contact"), ("draft_date", "Draft date")] {
            if let Some(value) = found(key) {
                let kept = self.details.iter().find(|d| d.key == key);
                details.push(DetailEdit {
                    key,
                    label,
                    value: kept.map_or(value, |d| d.value.clone()),
                    apply: kept.map_or(prefs.apply_details, |d| d.apply),
                });
            }
        }
        self.details = details;
        self.preview = Some(data);
    }

    pub fn included_count(&self) -> usize {
        self.docs.iter().filter(|d| d.include).count()
    }

    /// Fold this review's reusable choices into the preferences.
    pub fn remember(&self, prefs: &mut ImportPrefs) {
        prefs.split = self.split;
        if !self.details.is_empty() {
            prefs.apply_details = self.details.iter().any(|d| d.apply);
        }
        if let Some(key) = self.format_key() {
            if self.overrides.is_empty() {
                prefs.style_maps.remove(&key);
            } else {
                prefs.style_maps.insert(key, self.overrides.clone());
            }
        }
        if let Some(folder) = Path::new(self.path.trim()).parent().and_then(|p| p.to_str()) {
            if !folder.is_empty() {
                prefs.folder = Some(folder.to_string());
            }
        }
    }
}

/// The medium a file would be imported as, for choosing style targets.
fn effective_medium(review: &ImportReview) -> Medium {
    let named = Medium::all().into_iter().find(|m| m.label() == review.medium);
    named.unwrap_or_else(|| {
        match review.preview.as_ref().and_then(|p| p["medium"].as_str()) {
            Some("Screenplay") => Medium::Screenplay,
            Some("TvScript") => Medium::TvScript,
            Some("StagePlay") => Medium::StagePlay,
            _ => Medium::Prose,
        }
    })
}

/// A readable name for a target token.
fn target_label(token: &str, sheet: &StyleSheet) -> String {
    if let Some(level) = token.strip_prefix("heading:") {
        return format!("Heading {level} (starts a document)");
    }
    if let Some(id) = token.strip_prefix("style:") {
        return sheet
            .styles
            .iter()
            .find(|style| style.id == id)
            .map_or_else(|| id.to_string(), |style| style.name.clone());
    }
    match token {
        "title" => "Work title".into(),
        "auto" => "Detect from the text".into(),
        "skip" => "Skip".into(),
        other => other.to_string(),
    }
}

/// The targets a style can be sent to, in the order the dropdown lists them.
fn target_choices(format_key: &str, medium: Medium, sheet: &StyleSheet) -> Vec<String> {
    let mut choices = Vec::new();
    if format_key == "docx" {
        choices.extend(["heading:1", "heading:2", "heading:3", "title"].map(String::from));
        if medium.is_script() {
            choices.push("auto".into());
        }
    }
    choices.extend(sheet.styles.iter().map(|style| format!("style:{}", style.id)));
    choices.push("skip".into());
    choices
}

pub fn show(app: &mut PortmanteauApp, ctx: &egui::Context) {
    let Some(mut review) = app.import_review.take() else { return };
    if review.is_stale() {
        review.refresh(&mut app.ws, &app.settings.import);
    }
    let mut open = true;
    let mut action = Action::None;
    let height = (ctx.content_rect().height() - 90.0).clamp(380.0, 760.0);
    egui::Window::new("Import manuscript")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .fixed_size(Vec2::new(760.0, height))
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .id_salt("import_review")
                .auto_shrink([false, false])
                .max_height(height - 70.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    body(app, ui, &mut review);
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let count = review.included_count();
                let label = match count {
                    0 => "Import".to_string(),
                    1 => "Import 1 document".to_string(),
                    n => format!("Import {n} documents"),
                };
                let ready = review.preview.is_some() && count > 0;
                let button = egui::Button::new(RichText::new(label).font(ui_body()).color(Color32::WHITE))
                    .fill(if ready { ACCENT } else { FAINT })
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(150.0, 30.0));
                if ui.add_enabled(ready, button).clicked() {
                    action = Action::Import;
                }
                if ui.button("Cancel").clicked() {
                    action = Action::Cancel;
                }
            });
        });
    if !open {
        action = Action::Cancel;
    }
    match action {
        Action::Cancel => {}
        Action::Import => {
            let response = app.ws.execute("document.import", review.payload(true));
            if response.ok {
                review.remember(&mut app.settings.import);
                app.selected = response.data["id"].as_str().map(str::to_string);
                app.view = View::Write;
                let kind = if response.data["warnings"].as_array().is_some_and(|w| !w.is_empty()) {
                    ToastKind::Info
                } else {
                    ToastKind::Ok
                };
                app.toast("Manuscript imported", response.message, kind);
            } else {
                app.toast("Could not import manuscript", response.message, ToastKind::Error);
                app.import_review = Some(review);
            }
        }
        Action::None => app.import_review = Some(review),
    }
}

enum Action {
    None,
    Import,
    Cancel,
}

fn card(ui: &mut egui::Ui, heading: &str, body: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, heading);
            ui.add_space(4.0);
            body(ui);
        });
    ui.add_space(8.0);
}

fn body(app: &mut PortmanteauApp, ui: &mut egui::Ui, review: &mut ImportReview) {
    // ---- File ---------------------------------------------------------
    card(ui, "File", |ui| {
        ui.horizontal(|ui| {
            let mut path = review.path.clone();
            let edit = ui.add(
                TextEdit::singleline(&mut path)
                    .hint_text("Path to a .fountain, .fdx, .docx or .txt file")
                    .font(mono(10.5))
                    .desired_width(ui.available_width() - 90.0),
            );
            let committed = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if committed || (edit.changed() && !edit.has_focus()) {
                review.set_path(&path);
            } else if edit.changed() {
                review.path = path;
            }
            if ui.button("Browse…").clicked() {
                if let Some(chosen) = browse(app.settings.import.folder.as_deref()) {
                    review.set_path(&chosen);
                }
            }
        });
        if let Some(error) = &review.error {
            ui.add_space(4.0);
            ui.label(RichText::new(error).font(ui_small()).color(RED_INK));
        } else if review.preview.is_none() {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Fountain and Final Draft become screenplays; Word and text become prose. Nothing is added until you press Import.")
                    .font(ui_small())
                    .color(FAINT),
            );
        }
    });
    let Some(preview) = review.preview.clone() else { return };

    // ---- How to read it ----------------------------------------------
    card(ui, "How to read it", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(preview_summary(&preview)).font(ui_small()).color(MUTED));
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("As").font(ui_small()).color(FAINT));
            let before = review.medium.clone();
            egui::ComboBox::from_id_salt("import_medium")
                .selected_text(if review.medium == "auto" { "Automatic".to_string() } else { review.medium.clone() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut review.medium, "auto".to_string(), "Automatic");
                    for medium in [Medium::Prose, Medium::Screenplay, Medium::TvScript, Medium::StagePlay] {
                        ui.selectable_value(&mut review.medium, medium.label().to_string(), medium.label());
                    }
                });
            if review.medium != before {
                review.mark_stale();
            }
            ui.add_space(10.0);
            let before = review.split;
            ui.checkbox(
                &mut review.split,
                RichText::new("Split into several documents at headings").font(ui_small()).color(MUTED),
            );
            if review.split != before {
                review.mark_stale();
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Add to").font(ui_small()).color(FAINT));
            let folders: Vec<(String, String)> = app
                .ws
                .project
                .ordered_documents()
                .into_iter()
                .filter(|d| d.kind.is_folder() && d.parent.is_none())
                .map(|d| (d.id.clone(), d.title.clone()))
                .collect();
            let current = review
                .parent
                .as_ref()
                .and_then(|id| folders.iter().find(|(f, _)| f == id))
                .map_or("Draft".to_string(), |(_, title)| title.clone());
            egui::ComboBox::from_id_salt("import_parent")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut review.parent, None, "Draft");
                    for (id, title) in folders.iter().filter(|(_, t)| t != "Draft") {
                        ui.selectable_value(&mut review.parent, Some(id.clone()), title);
                    }
                });
        });
    });

    // ---- Title page details ------------------------------------------
    if !review.details.is_empty() {
        card(ui, "Details found in the file", |ui| {
            ui.label(
                RichText::new("Ticked details are set on the project, replacing what it has. Edit them first if the file got one wrong.")
                    .font(ui_small())
                    .color(FAINT),
            );
            ui.add_space(4.0);
            for detail in &mut review.details {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut detail.apply, "");
                    ui.allocate_ui_with_layout(Vec2::new(86.0, 22.0), Layout::left_to_right(Align::Center), |ui| {
                        ui.set_min_width(86.0);
                        ui.label(RichText::new(detail.label).font(ui_small()).color(FAINT));
                    });
                    let multi = detail.key == "contact";
                    let edit = if multi {
                        TextEdit::multiline(&mut detail.value).desired_rows(2)
                    } else {
                        TextEdit::singleline(&mut detail.value)
                    };
                    ui.add(edit.font(ui_body()).desired_width(ui.available_width()));
                });
            }
        });
    }

    // ---- Paragraph styles --------------------------------------------
    let styles = preview["styles"].as_array().cloned().unwrap_or_default();
    if !styles.is_empty() {
        let format_key = review.format_key().unwrap_or_default();
        let medium = effective_medium(review);
        let sheet = StyleSheet::for_medium(medium, &app.ws.project.styles);
        let choices = target_choices(&format_key, medium, &sheet);
        card(ui, "Paragraph styles", |ui| {
            ui.label(
                RichText::new("What each style in the file becomes. Your choices are remembered for the next file from the same program.")
                    .font(ui_small())
                    .color(FAINT),
            );
            ui.add_space(4.0);
            egui::Grid::new("import_styles").num_columns(3).spacing([14.0, 5.0]).show(ui, |ui| {
                for style in &styles {
                    let name = style["name"].as_str().unwrap_or_default();
                    let default = style["default"].as_str().unwrap_or("auto");
                    let current = style["target"].as_str().unwrap_or(default).to_string();
                    ui.label(RichText::new(name).font(ui_body()).color(TEXT));
                    ui.label(RichText::new(format!("{}", style["count"].as_u64().unwrap_or(0))).font(mono(10.0)).color(FAINT));
                    let mut chosen = current.clone();
                    egui::ComboBox::from_id_salt(("import_style", name))
                        .width(260.0)
                        .selected_text(target_label(&current, &sheet))
                        .show_ui(ui, |ui| {
                            // Offering the importer's own choice first makes
                            // "undo my change" one click.
                            ui.selectable_value(
                                &mut chosen,
                                default.to_string(),
                                format!("{} (default)", target_label(default, &sheet)),
                            );
                            for choice in choices.iter().filter(|c| c.as_str() != default) {
                                ui.selectable_value(&mut chosen, choice.clone(), target_label(choice, &sheet));
                            }
                        });
                    if chosen != current {
                        if chosen == default {
                            review.overrides.remove(name);
                            review.overrides.retain(|source, _| !source.eq_ignore_ascii_case(name));
                        } else {
                            review.overrides.insert(name.to_string(), chosen);
                        }
                        review.mark_stale();
                    }
                    ui.end_row();
                }
            });
        });
    }

    // ---- Documents ---------------------------------------------------
    card(ui, "Documents to create", |ui| {
        let listed = preview["documents"].as_array().cloned().unwrap_or_default();
        egui::Grid::new("import_docs").num_columns(4).spacing([10.0, 5.0]).show(ui, |ui| {
            for (index, edit) in review.docs.iter_mut().enumerate() {
                let words = listed.get(index).and_then(|d| d["words"].as_u64()).unwrap_or(0);
                ui.checkbox(&mut edit.include, "");
                ui.add_sized(Vec2::new(330.0, 26.0), TextEdit::singleline(&mut edit.title).font(ui_body()));
                egui::ComboBox::from_id_salt(("import_kind", index))
                    .width(96.0)
                    .selected_text(edit.kind.clone())
                    .show_ui(ui, |ui| {
                        for kind in ["Chapter", "Scene", "Text"] {
                            ui.selectable_value(&mut edit.kind, kind.to_string(), kind);
                        }
                    });
                ui.label(RichText::new(format!("{words} words")).font(mono(10.0)).color(FAINT));
                ui.end_row();
            }
        });
    });

    // ---- Warnings ----------------------------------------------------
    let warnings = preview["warnings"].as_array().cloned().unwrap_or_default();
    if !warnings.is_empty() {
        card(ui, "Not carried across", |ui| {
            for warning in warnings {
                ui.label(RichText::new(format!("• {}", warning.as_str().unwrap_or_default())).font(ui_small()).color(MUTED));
            }
        });
    }
}

fn preview_summary(preview: &Value) -> String {
    let docs = preview["documents"].as_array().map_or(0, Vec::len);
    let words: u64 = preview["documents"]
        .as_array()
        .map(|d| d.iter().filter_map(|d| d["words"].as_u64()).sum())
        .unwrap_or(0);
    format!(
        "{} · {} document{} · {} words",
        preview["format"].as_str().unwrap_or("File"),
        docs,
        if docs == 1 { "" } else { "s" },
        words
    )
}

fn browse(start: Option<&str>) -> Option<String> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Import manuscript")
        .add_filter("Manuscripts", &["fountain", "spmd", "fdx", "docx", "txt", "text", "md", "markdown"]);
    if let Some(dir) = start.filter(|d| Path::new(d).is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_file().map(|path| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp(name: &str, bytes: &[u8]) -> String {
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("review"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path.display().to_string()
    }

    const SCRIPT: &str = "Title: Tidewrack\nAuthor: Ada Marlowe\n\n# ACT ONE\n\nINT. HARBOR - DAWN\n\nMARA\nWe missed the tide.\n\n# ACT TWO\n\nEXT. PIER - DAY\n\nShe waits.\n";

    #[test]
    fn a_review_reads_the_file_and_offers_its_documents_and_details() {
        let mut ws = Workspace::demo();
        let prefs = ImportPrefs::default();
        let mut review = ImportReview::for_path(&prefs, &write_temp("a.fountain", SCRIPT.as_bytes()));
        assert!(review.is_stale());
        review.refresh(&mut ws, &prefs);
        assert!(review.error.is_none(), "{:?}", review.error);
        assert_eq!(review.docs.len(), 2);
        assert!(review.docs.iter().all(|d| d.include));
        let keys: Vec<_> = review.details.iter().map(|d| d.key).collect();
        assert_eq!(keys, ["title", "author"]);
        assert!(review.details.iter().all(|d| d.apply), "details are ticked by default");
        // Nothing has been added to the project yet.
        assert!(ws.project.documents.iter().all(|d| d.title != "Tidewrack"));
    }

    #[test]
    fn the_payload_carries_the_writers_edits() {
        let mut ws = Workspace::demo();
        ws.project.author = "Original Author".into();
        let prefs = ImportPrefs::default();
        let mut review = ImportReview::for_path(&prefs, &write_temp("b.fountain", SCRIPT.as_bytes()));
        review.refresh(&mut ws, &prefs);
        review.docs[0].title = "Cold open".into();
        review.docs[1].include = false;
        review.details[1].apply = false;
        review.details[0].value = "Edited Title".into();
        let payload = review.payload(true);
        assert_eq!(payload["documents"][0]["title"], "Cold open");
        assert_eq!(payload["documents"][1]["include"], false);
        assert_eq!(payload["details"]["title"], "Edited Title");
        assert!(payload["details"].get("author").is_none(), "an unticked detail is not sent");
        assert_eq!(review.included_count(), 1);

        let response = ws.execute("document.import", payload);
        assert!(response.ok, "{}", response.message);
        assert_eq!(ws.project.title, "Edited Title");
        assert_eq!(ws.project.author, "Original Author", "the unticked author is left alone");
        assert!(ws.project.documents.iter().any(|d| d.title == "Cold open"));
        assert!(!ws.project.documents.iter().any(|d| d.title.contains("ACT TWO") || d.title.contains("Act Two")));
    }

    #[test]
    fn style_mappings_are_remembered_and_applied_to_the_next_file_of_that_format() {
        let mut ws = Workspace::demo();
        let mut prefs = ImportPrefs::default();
        let fdx = r#"<?xml version="1.0"?><FinalDraft><Content>
            <Paragraph Type="Action"><Text>The sea.</Text></Paragraph>
            <Paragraph Type="Transition"><Text>CUT TO:</Text></Paragraph></Content></FinalDraft>"#;

        let mut review = ImportReview::for_path(&prefs, &write_temp("one.fdx", fdx.as_bytes()));
        review.refresh(&mut ws, &prefs);
        review.overrides.insert("Transition".into(), "skip".into());
        review.mark_stale();
        review.refresh(&mut ws, &prefs);
        let kept: Vec<_> = review.preview.as_ref().unwrap()["documents"][0]["paragraphs"].as_u64().into_iter().collect();
        assert_eq!(kept, [1], "the transition is skipped");
        review.split = false;
        review.remember(&mut prefs);
        assert_eq!(prefs.style_maps["fdx"]["Transition"], "skip");
        assert!(!prefs.split);

        // A new review of another FDX file starts already mapped.
        let mut next = ImportReview::for_path(&prefs, &write_temp("two.fdx", fdx.as_bytes()));
        next.refresh(&mut ws, &prefs);
        assert_eq!(next.overrides.get("Transition").map(String::as_str), Some("skip"));
        assert_eq!(next.preview.as_ref().unwrap()["documents"][0]["paragraphs"], 1);
        assert!(!next.split, "split is remembered too");

        // Clearing every override forgets the mapping.
        next.overrides.clear();
        next.remember(&mut prefs);
        assert!(prefs.style_maps.get("fdx").is_none());
    }

    #[test]
    fn an_unreadable_file_reports_why_and_offers_nothing() {
        let mut ws = Workspace::demo();
        let prefs = ImportPrefs::default();
        let mut review = ImportReview::for_path(&prefs, "/definitely/not/here.docx");
        review.refresh(&mut ws, &prefs);
        assert!(review.error.as_deref().is_some_and(|e| e.contains("Could not read")));
        assert!(review.preview.is_none() && review.docs.is_empty());
    }

    #[test]
    fn the_review_draws_without_panicking_for_each_format() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        for (name, bytes) in [
            ("c.fountain", SCRIPT.as_bytes().to_vec()),
            ("c.txt", b"Chapter 1\n\nOne.\n\nChapter 2\n\nTwo.".to_vec()),
        ] {
            let mut app = PortmanteauApp::new(None);
            app.import_review = Some(ImportReview::for_path(&app.settings.import.clone(), &write_temp(name, &bytes)));
            for _ in 0..2 {
                ctx.run_ui(egui::RawInput::default(), |ui| {
                    let ctx = ui.ctx().clone();
                    show(&mut app, &ctx);
                })
                .drop_without_applying_deltas();
            }
            let review = app.import_review.as_ref().expect("still open");
            assert!(review.preview.is_some(), "{name}: {:?}", review.error);
        }
    }
}
