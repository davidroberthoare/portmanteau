//! Portmanteau desktop shell.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Id, Key, Layout, Margin, RichText,
    ScrollArea, Stroke, TextEdit, Vec2,
};

use portmanteau_api::Workspace;
use portmanteau_core::{
    AppSettings, DatabaseItem, DocKind, ElementKind, ExportKind, ItemKind, Medium, ProjectKind,
};

use crate::icons::{self, Icon};
use crate::theme::*;
use crate::ui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Write,
    Corkboard,
    Outline,
    Research,
    Database,
    History,
    Compile,
    Agent,
    Settings,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Write => "Write",
            View::Corkboard => "Corkboard",
            View::Outline => "Outline",
            View::Research => "Research",
            View::Database => "Database",
            View::History => "Versions",
            View::Compile => "Compile",
            View::Agent => "Command desk",
            View::Settings => "Settings",
        }
    }
    pub fn icon(self) -> Icon {
        match self {
            View::Write => Icon::Write,
            View::Corkboard => Icon::Corkboard,
            View::Outline => Icon::Outline,
            View::Research => Icon::Research,
            View::Database => Icon::Database,
            View::History => Icon::History,
            View::Compile => Icon::Compile,
            View::Agent => Icon::Agent,
            View::Settings => Icon::Settings,
        }
    }
    pub fn all() -> [View; 8] {
        [
            View::Write,
            View::Corkboard,
            View::Outline,
            View::Research,
            View::Database,
            View::History,
            View::Compile,
            View::Settings,
        ]
    }
    /// Development aid: `PORTMANTEAU_VIEW=corkboard` opens a workspace
    /// directly, which makes automated screenshots of each screen possible.
    pub fn from_key(key: &str) -> Option<View> {
        // The Command desk is intentionally absent from `all()` (it is opened
        // from the View menu), so resolve its keys explicitly.
        if key.eq_ignore_ascii_case("agent") || key.eq_ignore_ascii_case("command desk") {
            return Some(View::Agent);
        }
        View::all()
            .into_iter()
            .find(|v| format!("{v:?}").eq_ignore_ascii_case(key) || v.label().eq_ignore_ascii_case(key))
    }
}

fn rail_view_button(ui: &mut egui::Ui, view: View, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(46.0, 44.0), egui::Sense::click());
    let painter = ui.painter();
    if active {
        painter.rect_filled(rect, CornerRadius::same(8), ACCENT_SOFT);
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.left() - 8.0, rect.top() + 11.0),
            Vec2::new(2.5, 22.0),
        );
        painter.rect_filled(bar, CornerRadius::same(2), ACCENT);
    } else if resp.hovered() {
        painter.rect_filled(rect, CornerRadius::same(8), PANEL_2);
    }
    icons::draw(
        painter,
        rect.shrink(12.0),
        view.icon(),
        if active { ACCENT_INK } else { MUTED },
    );
    resp.on_hover_text(view.label()).clicked()
}

fn initial_view() -> View {
    std::env::var("PORTMANTEAU_VIEW")
        .ok()
        .and_then(|key| View::from_key(key.trim()))
        .unwrap_or(View::Write)
}

/// Which output format the Compile screen is showing. The Display/editor
/// settings now live in the Settings workspace.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompileTab {
    Format(ExportKind),
}

impl Default for CompileTab {
    fn default() -> Self {
        CompileTab::Format(ExportKind::Pdf)
    }
}

/// How the Write editor presents the page. Continuous is the editable paper;
/// Paged is a read-only print preview built from the active export target.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WriteMode {
    Continuous,
    Paged,
}

impl WriteMode {
    pub fn label(self) -> &'static str {
        match self {
            WriteMode::Continuous => "Continuous",
            WriteMode::Paged => "Paged",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Document,
    Notes,
    History,
}

pub struct Toast {
    pub message: String,
    pub detail: String,
    pub at: Instant,
    pub kind: ToastKind,
}

/// A prepared revision comparison, owned by the shell so the modal can be
/// drawn after the view that opened it has finished painting.
#[derive(Clone)]
pub struct DiffState {
    pub doc_id: String,
    pub doc_title: String,
    pub snapshot_id: String,
    pub snapshot_name: String,
    pub created: String,
    pub added: usize,
    pub removed: usize,
    pub words_before: u32,
    pub words_after: u32,
    pub lines: Vec<portmanteau_core::DiffLine>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PathDialog {
    Open,
    SaveAs,
    Import,
}

/// A character being created from the screenplay editor's "New character…"
/// entry. The document and block are remembered so the new name can be dropped
/// into the script when the modal closes.
#[derive(Clone)]
pub struct NewCharacterState {
    pub doc_id: String,
    pub block: usize,
    pub name: String,
    pub role: String,
}

#[derive(Clone, Debug)]
pub enum RichTextTarget {
    DocumentBody(String),
    DocumentSynopsis(String),
    DocumentNotes(String),
    ResearchBody(String),
    DatabaseNotes(String),
}

#[derive(Clone, Debug)]
pub struct ImageInsertState {
    pub target: RichTextTarget,
    pub range: Range<usize>,
    pub path: String,
    pub alt_text: String,
    pub caption: String,
    pub standalone: bool,
    pub error: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Ok,
    Error,
    Info,
}

pub struct PortmanteauApp {
    pub ws: Workspace,
    pub settings: AppSettings,
    pub view: View,
    /// Selected tab on the Compile screen.
    pub compile_tab: CompileTab,
    /// Selected tab on the Settings screen: 0 = Preferences, 1 = Display.
    pub settings_tab: usize,
    pub write_mode: WriteMode,
    /// The page currently at the top of the Paged preview, for the toolbar
    /// readout. Updated after the pages are drawn.
    pub paged_page: usize,
    /// Current page of the Compile screen's single-page preview. Reset when the
    /// selected output tab changes; clamped to the preview's page count.
    pub preview_page: usize,
    /// Current page of the Settings → Display manuscript preview.
    pub settings_preview_page: usize,
    pub selected: Option<String>,
    pub selected_block: usize,
    pub block_focus: Option<usize>,
    pub inspector_tab: InspectorTab,
    pub collapsed: HashSet<String>,
    pub search: String,
    pub focus_search: bool,
    pub focus_mode: bool,
    pub snapshot_name: String,
    pub checkpoint_label: String,
    pub selected_research: Option<String>,
    pub selected_item: Option<String>,
    /// Raw text of the database Tags field for `database_tags_id`. Keeping the
    /// unparsed text in view state stops a trailing comma or space being
    /// normalised away while the writer is still typing.
    pub database_tags_id: Option<String>,
    pub database_tags_text: String,
    pub toast: Option<Toast>,
    pub show_about: bool,
    pub show_new_project: bool,
    pub new_project_title: String,
    pub new_project_kind: ProjectKind,
    pub cmd_name: String,
    pub cmd_payload: String,
    pub cmd_output: String,
    pub cmd_selected: usize,
    pub path_dialog: Option<PathDialog>,
    pub path_draft: String,
    pub new_character: Option<NewCharacterState>,
    pub image_insert: Option<ImageInsertState>,
    pub image_textures: HashMap<String, egui::TextureHandle>,
    pub diff: Option<DiffState>,
    pub outline_filter: usize,
    pub screenshot: Option<PathBuf>,
    pub frame_no: u32,
    pub screenshot_requested: bool,
    /// Text zoom for the writing surface. Multiplies the prose type size and
    /// the screenplay element sizes; `−` / `+` in the editor toolbar change it
    /// within `0.7..=1.6`.
    pub editor_zoom: f32,
/// Corkboard card scale. 1.0 is the default card width; the `−` / `+`
    /// controls in the corkboard header change it within `0.6..=1.6`.
    pub corkboard_zoom: f32,
    pub spell: portmanteau_core::spell::SpellChecker,
    pub show_spelling: bool,
}

impl PortmanteauApp {
    pub fn new(screenshot: Option<PathBuf>) -> Self {
        let ws = Workspace::demo();
        let selected = ws
            .project
            .documents
            .iter()
            .find(|d| d.title.starts_with("Chapter One"))
            .map(|d| d.id.clone())
            .or_else(|| ws.project.documents.iter().find(|d| d.kind == DocKind::Chapter).map(|d| d.id.clone()));
        let selected = std::env::var("PORTMANTEAU_DOC")
            .ok()
            .and_then(|needle| {
                let needle = needle.to_lowercase();
                ws.project
                    .documents
                    .iter()
                    .find(|d| d.title.to_lowercase().contains(&needle))
                    .map(|d| d.id.clone())
            })
            .or(selected);
        let selected_item = ws.project.items.first().map(|c| c.id.clone());
        let selected_research = ws.project.research.first().map(|r| r.id.clone());
        let mut collapsed = HashSet::new();
        if let Some(draft) = ws.project.folder_id("Draft") {
            collapsed.remove(&draft);
        }
        let settings = crate::settings::load_settings();
        let mut spell = crate::spellcheck::load(&settings.language);
        spell.set_custom(settings.custom_words.clone());
        let mut app = Self {
            ws,
            settings,
            view: initial_view(),
            compile_tab: CompileTab::default(),
            settings_tab: 0,
            write_mode: WriteMode::Continuous,
            paged_page: 1,
            preview_page: 0,
            settings_preview_page: 0,
            selected,
            selected_block: 0,
            block_focus: None,
            inspector_tab: InspectorTab::Document,
            collapsed,
            search: String::new(),
            focus_search: false,
            focus_mode: std::env::var("PORTMANTEAU_FOCUS").is_ok(),
            snapshot_name: String::new(),
            checkpoint_label: String::new(),
            selected_research,
            selected_item,
            database_tags_id: None,
            database_tags_text: String::new(),
            toast: None,
            show_about: false,
            show_new_project: false,
            new_project_title: String::new(),
            new_project_kind: ProjectKind::Novel,
            cmd_name: "project.info".into(),
            cmd_payload: "{}".into(),
            cmd_output: String::new(),
            cmd_selected: 0,
            path_dialog: None,
            path_draft: String::new(),
            new_character: None,
            image_insert: None,
            image_textures: HashMap::new(),
            diff: None,
            outline_filter: 0,
            screenshot,
            frame_no: 0,
            screenshot_requested: false,
            editor_zoom: DEFAULT_EDITOR_ZOOM,
            corkboard_zoom: 1.0,
            spell,
            show_spelling: false,
        };
        // Development aid: open the first available comparison so the diff
        // screen can be reviewed (and screenshotted) without clicking.
        if std::env::var("PORTMANTEAU_DIFF").is_ok() {
            if let Some((doc_id, snap_id)) = app.ws.project.documents.iter().find_map(|d| {
                d.snapshots
                    .first()
                    .map(|s| (d.id.clone(), s.id.clone()))
            }) {
                app.open_diff(&doc_id, &snap_id);
            }
        }
        app
    }

    pub fn current_doc(&self) -> Option<&portmanteau_core::Document> {
        self.selected
            .as_ref()
            .and_then(|id| self.ws.project.document(id))
    }

    pub fn toast(&mut self, message: impl Into<String>, detail: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast {
            message: message.into(),
            detail: detail.into(),
            at: Instant::now(),
            kind,
        });
    }

    /// Switch the spelling dictionary and remember the choice as a global
    /// default.
    pub fn set_spell_language(&mut self, language: &str) {
        self.settings.language = language.to_string();
        self.spell = crate::spellcheck::load(&self.settings.language);
        self.spell.set_custom(self.settings.custom_words.clone());
        let _ = crate::settings::save_settings(&self.settings);
    }

    /// Add a word to the global personal dictionary. The word is normalised and
    /// deduplicated, then saved with the other defaults.
    pub fn add_to_dictionary(&mut self, word: &str) {
        let word = portmanteau_core::spell::normalise(word);
        if word.is_empty() {
            return;
        }
        if !self.settings.custom_words.contains(&word) {
            self.settings.custom_words.push(word.clone());
        }
        self.spell.add_custom(&word);
        let _ = crate::settings::save_settings(&self.settings);
    }

    pub fn open_doc(&mut self, id: String) {
        self.selected = Some(id);
        self.selected_block = 0;
        self.view = View::Write;
    }

    /// Build a line-level comparison between a saved snapshot and the current
    /// document. The same operation is available as `snapshot.diff` to agents.
    pub fn open_diff(&mut self, doc_id: &str, snap_id: &str) {
        let Some(doc) = self.ws.project.document(doc_id) else {
            return;
        };
        let Some(snap) = doc.snapshots.iter().find(|s| s.id == snap_id) else {
            return;
        };
        let lines = portmanteau_core::diff_lines(&snap.body_text(), &doc.body_text());
        let (added, removed) = portmanteau_core::diff_counts(&lines);
        self.diff = Some(DiffState {
            doc_id: doc_id.to_string(),
            doc_title: doc.title.clone(),
            snapshot_id: snap.id.clone(),
            snapshot_name: snap.name.clone(),
            created: snap.created.clone(),
            added,
            removed,
            words_before: snap.words,
            words_after: doc.words(),
            lines,
        });
    }

    /// Reorder the open document among its siblings. Returns true when the
    /// document actually moved.
    pub fn move_selected(&mut self, direction: i32) -> bool {
        let Some(id) = self.selected.clone() else {
            return false;
        };
        if self.ws.project.move_document(&id, direction) {
            self.ws.dirty = true;
            true
        } else {
            false
        }
    }

    fn save_project(&mut self) {
        match self.ws.save(None) {
            Ok(msg) => self.toast("Project saved", msg, ToastKind::Ok),
            Err(e) => self.toast("Could not save", e, ToastKind::Error),
        }
    }

    /// Public door for the editor and empty states, which cannot call private
    /// helpers from another module.
    pub fn take_snapshot_public(&mut self) {
        self.take_snapshot();
    }

    pub fn new_chapter_public(&mut self) {
        self.new_chapter();
    }

    fn new_chapter(&mut self) {
        let parent = self
            .selected
            .as_ref()
            .and_then(|id| self.ws.project.document(id))
            .and_then(|d| {
                if d.kind == DocKind::Folder {
                    Some(d.id.clone())
                } else {
                    d.parent.clone()
                }
            })
            .or_else(|| self.ws.project.folder_id("Draft"));
        let is_script = matches!(
            self.ws.project.kind,
            ProjectKind::Film | ProjectKind::Television | ProjectKind::Theatre
        );
        let (kind, medium, title) = if is_script {
            (DocKind::Scene, Medium::Screenplay, "Untitled scene")
        } else {
            (DocKind::Chapter, Medium::Prose, "Untitled chapter")
        };
        let id = self.ws.project.add_document(parent, title, kind, medium);
        if medium.is_script() {
            if let Some(doc) = self.ws.project.document_mut(&id) {
                doc.set_script_blocks(vec![portmanteau_core::Block::new(
                    ElementKind::SceneHeading,
                    "INT. LOCATION — DAY",
                )]);
            }
        }
        self.ws.dirty = true;
        self.selected = Some(id);
        self.selected_block = 0;
        self.block_focus = Some(0);
        self.toast("New document", format!("{} added to the binder", title), ToastKind::Ok);
    }

    fn take_snapshot(&mut self) {
        let Some(id) = self.selected.clone() else { return };
        let name = if self.snapshot_name.trim().is_empty() {
            format!("Snapshot · {}", chrono::Local::now().format("%d %b %H:%M"))
        } else {
            self.snapshot_name.clone()
        };
        if let Some(doc) = self.ws.project.document_mut(&id) {
            doc.snapshot(name.clone(), "Manual snapshot from the desktop");
            self.ws.dirty = true;
            self.snapshot_name.clear();
            self.toast("Version saved", name, ToastKind::Ok);
        }
    }

    fn checkpoint(&mut self, label: Option<String>) {
        let label = label.unwrap_or_else(|| {
            if self.checkpoint_label.trim().is_empty() {
                format!("Checkpoint · {}", chrono::Local::now().format("%d %b %H:%M"))
            } else {
                self.checkpoint_label.clone()
            }
        });
        self.ws.project.checkpoint(label.clone(), "manual");
        self.ws.dirty = true;
        self.checkpoint_label.clear();
        self.toast("Project checkpoint saved", label, ToastKind::Ok);
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar")
            .exact_size(54.0)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, BORDER_SOFT))
                    .inner_margin(Margin::symmetric(14, 9)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, CornerRadius::same(7), ACCENT);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "S",
                        serif(18.0),
                        Color32::WHITE,
                    );
                    ui.add_space(3.0);
                    ui.label(RichText::new("Portmanteau").font(serif(19.0)).color(TEXT));
                    ui.add_space(10.0);

                    self.menus(ui);
                    ui.add_space(8.0);
                    self.search_field(ui);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let compile = egui::Button::new(
                            RichText::new("Compile  ▸").font(ui_body()).color(Color32::WHITE),
                        )
                        .fill(ACCENT)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(96.0, 30.0));
                        if ui.add(compile).on_hover_text("Compile and export the manuscript").clicked() {
                            self.view = View::Compile;
                        }
                        ui.add_space(8.0);
                        let today = self.ws.project.today_words();
                        let target = self.ws.project.daily_target.max(1);
                        ui.vertical(|ui| {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(format!("{} / {} today", today, target))
                                    .font(ui_body())
                                    .color(MUTED),
                            );
                            ui.add_space(2.0);
                            let (r, _) = ui.allocate_exact_size(Vec2::new(88.0, 4.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, CornerRadius::same(2), PANEL_3);
                            let f = (today as f32 / target as f32).clamp(0.0, 1.0);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(r.min, Vec2::new(r.width() * f, 4.0)),
                                CornerRadius::same(2),
                                SAGE,
                            );
                        });
                        ui.add_space(10.0);
                    });
                });
            });
    }

    /// Focus mode replaces the normal chrome with one slim bar. It changes
    /// nothing about the document; the editor keeps its own state and layout
    /// rules.
    fn focus_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("focusbar")
            .exact_size(38.0)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, BORDER_SOFT))
                    .inner_margin(Margin::symmetric(14, 4)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new(&self.ws.project.title)
                            .font(serif(13.0))
                            .color(MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let exit = egui::Button::new(
                            RichText::new("Exit focus").font(ui_body()).color(MUTED),
                        )
                        .fill(PANEL_2)
                        .corner_radius(CornerRadius::same(5));
                        if ui
                            .add(exit)
                            .on_hover_text("Restore the full workspace (F11)")
                            .clicked()
                        {
                            self.focus_mode = false;
                        }
                        if let Some(doc) = self.current_doc() {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("{} · {} words", doc.title, doc.words()))
                                    .font(ui_body())
                                    .color(FAINT),
                            );
                        }
                    });
                });
            });
    }

    fn menus(&mut self, ui: &mut egui::Ui) {
        let menu_text = |s: &str| RichText::new(s).font(ui_body()).color(MUTED);
        ui.menu_button(menu_text("File"), |ui| {
            if ui.button("New project…").clicked() {
                self.new_project_title.clear();
                self.show_new_project = true;
                ui.close();
            }
            if ui.button("Save project").clicked() {
                self.save_project();
                ui.close();
            }
            if ui.button("Save project as…").clicked() {
                self.path_dialog = Some(PathDialog::SaveAs);
                self.path_draft = self
                    .ws
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| {
                        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                        format!("{home}/Portmanteau/{}.portmanteau/", slug(&self.ws.project.title))
                    });
                ui.close();
            }
            if ui.button("Open project…").clicked() {
                self.path_dialog = Some(PathDialog::Open);
                self.path_draft = self
                    .ws
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                ui.close();
            }
            if ui.button("Import Fountain, FDX, DOCX or Text…").clicked() {
                self.path_dialog = Some(PathDialog::Import);
                self.path_draft.clear();
                ui.close();
            }
            if ui.button("Save checkpoint").clicked() {
                self.checkpoint(None);
                ui.close();
            }
            ui.separator();
            if ui.button("Compile & export…").clicked() {
                self.view = View::Compile;
                ui.close();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button(menu_text("Edit"), |ui| {
            if ui.button("Search manuscript").clicked() {
                self.focus_search = true;
                ui.close();
            }
            if ui.button("Take snapshot").clicked() {
                self.take_snapshot();
                ui.close();
            }
            ui.separator();
            if ui.button("Move document up").clicked() {
                self.move_selected(-1);
                ui.close();
            }
            if ui.button("Move document down").clicked() {
                self.move_selected(1);
                ui.close();
            }
        });
        ui.menu_button(menu_text("View"), |ui| {
            for v in View::all() {
                if ui.selectable_label(self.view == v, v.label()).clicked() {
                    self.view = v;
                    ui.close();
                }
            }
            // The Command desk is deliberately kept out of the rail and
            // `View::all()`; it is reachable from this menu.
            if ui
                .selectable_label(self.view == View::Agent, View::Agent.label())
                .clicked()
            {
                self.view = View::Agent;
                ui.close();
            }
            ui.separator();
            if ui
                .checkbox(&mut self.settings.show_binder, "Show binder")
                .changed()
            {
                let _ = crate::settings::save_settings(&self.settings);
            }
            if ui
                .checkbox(&mut self.settings.show_inspector, "Show inspector")
                .changed()
            {
                let _ = crate::settings::save_settings(&self.settings);
            }
            ui.separator();
            if ui.button("Focus mode (F11)").clicked() {
                self.focus_mode = !self.focus_mode;
                ui.close();
            }
        });
        ui.menu_button(menu_text("Insert"), |ui| {
            if ui.button("Image…").clicked() {
                if let Some(id) = self.selected.clone() {
                    if let Some(doc) = self.ws.project.document(&id) {
                        let end = doc.content.char_count();
                        self.image_insert = Some(ImageInsertState {
                            target: RichTextTarget::DocumentBody(id),
                            range: end..end,
                            path: String::new(),
                            alt_text: String::new(),
                            caption: String::new(),
                            standalone: true,
                            error: String::new(),
                        });
                    }
                }
                ui.close();
            }
            if ui.button("New chapter").clicked() {
                self.new_chapter();
                ui.close();
            }
            if ui.button("New research note").clicked() {
                self.view = View::Research;
                ui.close();
            }
            if ui.button("New character").clicked() {
                self.view = View::Database;
                ui.close();
            }
        });
    }

    fn search_field(&mut self, ui: &mut egui::Ui) {
        Frame::new()
            .fill(PANEL_2)
            .stroke(Stroke::new(1.0, BORDER_SOFT))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(9, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.set_min_height(24.0);
                    let (icon, _) =
                        ui.allocate_exact_size(Vec2::new(13.0, 13.0), egui::Sense::hover());
                    let c = icon.center();
                    ui.painter()
                        .circle_stroke(c - Vec2::new(1.0, 1.0), 3.6, Stroke::new(1.2, FAINT));
                    ui.painter().line_segment(
                        [c + Vec2::new(1.8, 1.8), c + Vec2::new(5.0, 5.0)],
                        Stroke::new(1.2, FAINT),
                    );
                    let resp = ui.add(
                        TextEdit::singleline(&mut self.search)
                            .hint_text(RichText::new("Search manuscript…").font(ui_body()).color(FAINT))
                            .desired_width(190.0)
                            .vertical_align(Align::Center)
                            .frame(Frame::NONE)
                            .font(ui_body()),
                    );
                    if self.focus_search {
                        resp.request_focus();
                        self.focus_search = false;
                    }
                    if !self.search.is_empty() {
                        if ui.small_button("×").clicked() {
                            self.search.clear();
                        }
                    }
                });
            });
    }

    fn rail(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("rail")
            .exact_size(62.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(RAIL)
                    .inner_margin(Margin::symmetric(8, 12)),
            )
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(2.0);
                    let (logo, _) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), egui::Sense::hover());
                    ui.painter().rect_stroke(
                        logo,
                        CornerRadius::same(9),
                        Stroke::new(1.2, ACCENT.gamma_multiply(0.7)),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        logo.center(),
                        egui::Align2::CENTER_CENTER,
                        "S",
                        serif(20.0),
                        ACCENT_INK,
                    );
                    ui.add_space(18.0);
                    for view in View::all() {
                        if view == View::Settings {
                            continue;
                        }
                        let active = self.view == view;
                        if rail_view_button(ui, view, active) {
                            self.view = view;
                        }
                        ui.add_space(2.0);
                    }
                });

                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    let (av, _) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), egui::Sense::click());
                    ui.painter().circle_filled(av.center(), 14.0, SAGE_SOFT);
                    ui.painter().text(av.center(), egui::Align2::CENTER_CENTER, "AM", ui_small(), SAGE);
                    ui.add_space(6.0);
                    // Settings sits just above the avatar, pinned to the foot
                    // of the rail rather than in the workspace list.
                    let active = self.view == View::Settings;
                    if rail_view_button(ui, View::Settings, active) {
                        self.view = View::Settings;
                    }
                    ui.add_space(6.0);
                    let (help, hr) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), egui::Sense::click());
                    ui.painter().text(
                        help.center(),
                        egui::Align2::CENTER_CENTER,
                        "?",
                        serif(17.0),
                        if hr.hovered() { TEXT } else { FAINT },
                    );
                    if hr.clicked() {
                        self.show_about = true;
                    }
                });
            });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("statusbar")
            .exact_size(27.0)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, BORDER_SOFT))
                    .inner_margin(Margin::symmetric(14, 4)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let color = if self.ws.dirty { ACCENT } else { SAGE };
                    let (dot_rect, _) =
                        ui.allocate_exact_size(Vec2::new(7.0, 7.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 3.0, color);
                    ui.label(
                        RichText::new(if self.ws.dirty { "Unsaved changes" } else { "All changes saved" })
                            .font(ui_body())
                            .color(MUTED),
                    );
                    ui.add_space(8.0);
                    let path = self
                        .ws
                        .path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Demonstration project · in memory".into());
                    ui.label(RichText::new(path).font(mono(10.0)).color(FAINT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} words  ·  {} pages estimated",
                                self.ws.project.words(),
                                self.ws
                                    .project
                                    .manuscript_documents()
                                    .iter()
                                    .map(|d| d.pages())
                                    .sum::<u32>()
                            ))
                            .font(ui_body())
                            .color(MUTED),
                        );
                        ui.add_space(12.0);
                        if let Some(doc) = self.current_doc() {
                            ui.label(
                                RichText::new(format!("{} · {}", doc.title, doc.medium.label()))
                                    .font(ui_body())
                                    .color(FAINT),
                            );
                        }
                    });
                });
            });
    }

    fn toast_ui(&mut self, ctx: &egui::Context) {
        let Some(toast) = &self.toast else { return };
        if toast.at.elapsed() > Duration::from_secs(5) {
            self.toast = None;
            return;
        }
        let accent = match toast.kind {
            ToastKind::Ok => SAGE,
            ToastKind::Error => RED,
            ToastKind::Info => BLUE,
        };
        egui::Area::new(Id::unique("toast"))
            .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-22.0, -42.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(PANEL_2)
                    .stroke(Stroke::new(1.0, accent.gamma_multiply(0.6)))
                    .corner_radius(CornerRadius::same(9))
                    .inner_margin(Margin::same(13))
                    .show(ui, |ui| {
                        ui.set_max_width(360.0);
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
                            ui.painter().circle_filled(r.center(), 4.0, accent);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&toast.message).font(ui_body()).color(TEXT));
                                ui.label(RichText::new(&toast.detail).font(ui_body()).color(MUTED));
                            });
                        });
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(200));
    }

    fn modals(&mut self, ctx: &egui::Context) {
        if self.show_new_project {
            let mut open = self.show_new_project;
            egui::Window::new("New project")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .min_width(440.0)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    section_label(ui, "Working title");
                    ui.add(
                        TextEdit::singleline(&mut self.new_project_title)
                            .hint_text("e.g. The Salt Road")
                            .desired_width(f32::INFINITY)
                            .font(serif(19.0)),
                    );
                    ui.add_space(12.0);
                    section_label(ui, "What are you writing?");
                    ui.horizontal_wrapped(|ui| {
                        for kind in ProjectKind::all() {
                            let selected = self.new_project_kind == kind;
                            let btn = egui::Button::new(
                                RichText::new(kind.label()).font(ui_body())
                                    .color(if selected { Color32::WHITE } else { TEXT }),
                            )
                            .fill(if selected { ACCENT } else { PANEL_2 })
                            .corner_radius(CornerRadius::same(20));
                            if ui.add(btn).clicked() {
                                self.new_project_kind = kind;
                            }
                        }
                    });
                    ui.add_space(16.0);
                    let ready = !self.new_project_title.trim().is_empty();
                    let btn = egui::Button::new(
                        RichText::new("Create project").font(ui_body())
                            .color(if ready { Color32::WHITE } else { MUTED }),
                    )
                    .fill(if ready { ACCENT } else { PANEL_3 })
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(140.0, 32.0));
                    if ui.add_enabled(ready, btn).clicked() {
                        let title = self.new_project_title.trim().to_string();
                        self.ws = Workspace::new_project(&title, self.new_project_kind);
                        self.ws.project.styles = self.settings.element_styles_for_new_project();
                        self.ws.project.style_sheets =
                            self.settings.style_sheets_for_new_project();
                        let parent = self.ws.project.folder_id("Draft");
                        let id = self.ws.project.add_document(
                            parent,
                            "Chapter One",
                            DocKind::Chapter,
                            self.new_project_kind.default_medium(),
                        );
                        self.selected = Some(id);
                        self.view = View::Write;
                        self.show_new_project = false;
                        self.toast("Project created", title, ToastKind::Ok);
                    }
                });
            if !open {
                self.show_new_project = false;
            }
        }

        if self.show_about {
            let mut open = self.show_about;
            egui::Window::new("About Portmanteau")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .min_width(400.0)
                .show(ctx, |ui| {
                    ui.label(RichText::new("Portmanteau").font(serif(26.0)).color(TEXT));
                    ui.label(
                        RichText::new("A native writing studio for novels, film, television and theatre.")
                            .font(ui_body())
                            .color(MUTED),
                    );
                    ui.add_space(10.0);
                    hairline(ui);
                    ui.add_space(10.0);
                    for line in [
                        "Binder, corkboard and outline for long-form structure",
                        "Prose and screenplay editors with typed elements",
                        "Research, a story database and structured planning",
                        "Snapshots, checkpoints, version comparison and restore",
                        "PDF, EPUB, DOCX, Final Draft, Fountain, HTML, Markdown, RTF and text",
                        "A JSON command API and an MCP server under the desktop UI",
                    ] {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(5.0, 5.0), egui::Sense::hover());
                            ui.painter().circle_filled(r.center(), 2.5, ACCENT);
                            ui.label(RichText::new(line).font(ui_body()).color(TEXT));
                        });
                    }
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Version 0.1 · modular Rust workspace")
                            .font(mono(10.0))
                            .color(FAINT),
                    );
                });
            if !open {
                self.show_about = false;
            }
        }

        self.path_modal(ctx);
        self.diff_modal(ctx);
        self.new_character_modal(ctx);
        self.image_insert_modal(ctx);
    }

    /// The small "New character" form opened from a Character element's
    /// autocomplete. On create the sheet is added to the project and its name
    /// is written into the block that asked for it.
    fn new_character_modal(&mut self, ctx: &egui::Context) {
        let Some(mut state) = self.new_character.clone() else {
            return;
        };
        let mut open = true;
        let mut create = false;
        egui::Window::new("New character")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(400.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                section_label(ui, "Name");
                ui.add(
                    TextEdit::singleline(&mut state.name)
                        .hint_text("e.g. Mara Venn")
                        .desired_width(f32::INFINITY)
                        .font(serif(19.0)),
                );
                ui.add_space(12.0);
                section_label(ui, "Role");
                ui.add(
                    TextEdit::singleline(&mut state.role)
                        .hint_text("Protagonist, ally, antagonist…")
                        .desired_width(f32::INFINITY)
                        .font(ui_body()),
                );
                ui.add_space(16.0);
                let ready = !state.name.trim().is_empty();
                let btn = egui::Button::new(
                    RichText::new("Create").font(ui_body())
                        .color(if ready { Color32::WHITE } else { MUTED }),
                )
                .fill(if ready { ACCENT } else { PANEL_3 })
                .corner_radius(CornerRadius::same(6))
                .min_size(Vec2::new(110.0, 30.0));
                if ui.add_enabled(ready, btn).clicked() {
                    create = true;
                }
            });

        if create {
            let name = state.name.trim().to_string();
            let role = state.role.trim().to_string();
            let mut character = DatabaseItem::new(ItemKind::Character, name.clone());
            character.notes.set_plain_text(role, "body");
            // The Database view cycles through six record colours.
            character.color = (self.ws.project.items.len() % 6) as u8;
            self.ws.project.items.push(character);
            self.ws.dirty = true;
            if self.selected.as_deref() == Some(state.doc_id.as_str()) {
                if let Some(doc) = self.ws.project.document_mut(&state.doc_id) {
                    if let Some(paragraph) = doc.content.paragraphs.get_mut(state.block) {
                        *paragraph = portmanteau_core::RichParagraph {
                            style_id: paragraph.style_id.clone(),
                            content: vec![portmanteau_core::InlineContent::Text(
                                portmanteau_core::TextRun::plain(name.clone()),
                            )],
                        };
                    }
                    doc.touch();
                }
                self.selected_block = state.block;
                self.block_focus = Some(state.block);
            }
            self.new_character = None;
            self.toast("Character created", name, ToastKind::Ok);
            return;
        }

        // Keep the draft while the window is still open.
        if let Some(current) = self.new_character.as_mut() {
            current.name = state.name;
            current.role = state.role;
        }
        if !open {
            self.new_character = None;
        }
    }

    fn image_insert_modal(&mut self, ctx: &egui::Context) {
        let Some(mut state) = self.image_insert.clone() else {
            return;
        };
        let mut open = true;
        let mut insert = false;
        let mut cancel = false;
        egui::Window::new("Insert image")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(420.0)
            .show(ctx, |ui| {
                section_label(ui, "Image file path");
                ui.add(
                    TextEdit::singleline(&mut state.path)
                        .hint_text("/path/to/image.png")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);
                section_label(ui, "Alt text");
                ui.add(
                    TextEdit::singleline(&mut state.alt_text)
                        .hint_text("Describe the image")
                        .desired_width(f32::INFINITY),
                );
                ui.checkbox(&mut state.standalone, "Place on its own paragraph");
                ui.add_space(6.0);
                section_label(ui, "Caption (standalone image)");
                ui.add_enabled(
                    state.standalone,
                    TextEdit::singleline(&mut state.caption)
                        .hint_text("Caption")
                        .desired_width(f32::INFINITY),
                );
                if !state.error.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(&state.error).font(ui_small()).color(RED));
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if crate::ui::views::quiet_button(ui, "Cancel").clicked() {
                        cancel = true;
                    }
                    if crate::ui::views::primary_button(ui, "Insert image").clicked() {
                        insert = true;
                    }
                });
            });

        if cancel {
            open = false;
        }
        if !open {
            self.image_insert = None;
            return;
        }
        if insert {
            if let Err(error) = self.insert_image_from_state(&state) {
                state.error = error;
                self.image_insert = Some(state);
            } else {
                self.image_insert = None;
            }
        } else {
            self.image_insert = Some(state);
        }
    }

    fn insert_image_from_state(&mut self, state: &ImageInsertState) -> Result<(), String> {
        let root = self
            .ws
            .path
            .clone()
            .ok_or_else(|| "Save the project before inserting images.".to_string())?;
        let source = PathBuf::from(state.path.trim());
        if !source.is_file() {
            return Err("Choose an existing image file.".into());
        }
        let (pixel_width, pixel_height) = image::image_dimensions(&source)
            .map_err(|error| format!("Could not read image dimensions: {error}"))?;
        let extension = source
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| extension.chars().all(|ch| ch.is_ascii_alphanumeric()))
            .ok_or_else(|| "The image file needs a normal filename extension.".to_string())?;
        let file_name = format!("{}.{}", portmanteau_core::new_id("img"), extension);
        let image_dir = root.join("assets").join("images");
        std::fs::create_dir_all(&image_dir)
            .map_err(|error| format!("Could not create the project image folder: {error}"))?;
        std::fs::copy(&source, image_dir.join(&file_name))
            .map_err(|error| format!("Could not copy the image into the project: {error}"))?;

        let display_max = if state.standalone { 240.0 } else { 20.0 };
        let source_max = pixel_width.max(pixel_height).max(1) as f32;
        let image_scale = display_max / source_max;
        let width = pixel_width as f32 * image_scale;
        let height = pixel_height as f32 * image_scale;
        let image = portmanteau_core::ImageObject {
            asset_id: format!("images/{file_name}"),
            alt_text: state.alt_text.trim().to_owned(),
            caption: state.caption.trim().to_owned(),
            width: Some(width),
            height: Some(height),
        };
        let sheet = match &state.target {
            RichTextTarget::DocumentBody(doc_id)
            | RichTextTarget::DocumentSynopsis(doc_id)
            | RichTextTarget::DocumentNotes(doc_id) => self
                .ws
                .project
                .document(doc_id)
                .and_then(|doc| {
                    self.ws
                        .project
                        .style_sheets
                        .iter()
                        .find(|sheet| sheet.id == doc.style_sheet_id)
                        .cloned()
                })
                .unwrap_or_else(|| {
                    portmanteau_core::StyleSheet::novel_prose(&self.ws.project.styles)
                }),
            RichTextTarget::ResearchBody(_) | RichTextTarget::DatabaseNotes(_) => {
                portmanteau_core::StyleSheet::novel_prose(&self.ws.project.styles)
            }
        };
        let content = match &state.target {
            RichTextTarget::DocumentBody(doc_id) => self
                .ws
                .project
                .document_mut(doc_id)
                .map(|doc| {
                    doc.content.insert_image(
                        state.range.clone(),
                        image,
                        state.standalone,
                        &sheet,
                    );
                    doc.touch();
                    &mut doc.content
                }),
            RichTextTarget::DocumentSynopsis(doc_id) => self
                .ws
                .project
                .document_mut(doc_id)
                .map(|doc| {
                    doc.synopsis.insert_image(
                        state.range.clone(),
                        image,
                        state.standalone,
                        &sheet,
                    );
                    doc.touch();
                    &mut doc.synopsis
                }),
            RichTextTarget::DocumentNotes(doc_id) => self
                .ws
                .project
                .document_mut(doc_id)
                .map(|doc| {
                    doc.notes.insert_image(
                        state.range.clone(),
                        image,
                        state.standalone,
                        &sheet,
                    );
                    doc.touch();
                    &mut doc.notes
                }),
            RichTextTarget::ResearchBody(item_id) => self
                .ws
                .project
                .research
                .iter_mut()
                .find(|item| item.id == *item_id)
                .map(|item| {
                    item.body.insert_image(
                        state.range.clone(),
                        image,
                        state.standalone,
                        &sheet,
                    );
                    &mut item.body
                }),
            RichTextTarget::DatabaseNotes(item_id) => self
                .ws
                .project
                .items
                .iter_mut()
                .find(|item| item.id == *item_id)
                .map(|item| {
                    item.notes.insert_image(
                        state.range.clone(),
                        image,
                        state.standalone,
                        &sheet,
                    );
                    &mut item.notes
                }),
        };
        if content.is_none() {
            return Err("The target text field no longer exists.".into());
        }
        self.ws.project.touch();
        self.ws.dirty = true;
        self.toast("Image inserted", state.alt_text.clone(), ToastKind::Ok);
        Ok(())
    }

    fn diff_modal(&mut self, ctx: &egui::Context) {
        let Some(state) = self.diff.clone() else { return };
        let mut open = true;
        let mut close = false;
        let mut restore = false;
        egui::Window::new("Compare version")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(780.0)
            .max_width(980.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&state.doc_title)
                                .font(serif(20.0))
                                .color(TEXT),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} · saved {} · {} words then, {} now",
                                state.snapshot_name,
                                state.created,
                                state.words_before,
                                state.words_after
                            ))
                            .font(ui_small())
                            .color(FAINT),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                        if matches!(state.added + state.removed, 0) {
                            pill(ui, "No changes", SAGE, SAGE_SOFT);
                        } else {
                            if state.removed > 0 {
                                pill(
                                    ui,
                                    &format!("−{} removed", state.removed),
                                    RED,
                                    Color32::from_rgb(244, 225, 222),
                                );
                            }
                            if state.added > 0 {
                                pill(
                                    ui,
                                    &format!("+{} added", state.added),
                                    SAGE,
                                    SAGE_SOFT,
                                );
                            }
                        }
                    });
                });
                ui.add_space(8.0);
                hairline(ui);
                ui.add_space(4.0);

                Frame::new()
                    .fill(PAPER)
                    .stroke(Stroke::new(1.0, PAPER_EDGE))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("diff_scroll")
                            .auto_shrink([false, false])
                            .max_height(430.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                if state.lines.is_empty() {
                                    ui.label(
                                        RichText::new("Both versions are empty.")
                                            .font(mono(10.0))
                                            .color(PAPER_MUTED),
                                    );
                                }
                                for line in &state.lines {
                                    let (sigil, sigil_color, fill, text_color) = match line.kind {
                                        portmanteau_core::DiffKind::Same => (
                                            " ",
                                            PAPER_MUTED,
                                            PAPER,
                                            if line.text.trim().is_empty() {
                                                PAPER_MUTED
                                            } else {
                                                PAPER_TEXT
                                            },
                                        ),
                                        portmanteau_core::DiffKind::Add => (
                                            "+",
                                            Color32::from_rgb(72, 118, 86),
                                            Color32::from_rgb(225, 238, 227),
                                            Color32::from_rgb(32, 55, 39),
                                        ),
                                        portmanteau_core::DiffKind::Remove => (
                                            "−",
                                            Color32::from_rgb(158, 75, 66),
                                            Color32::from_rgb(245, 225, 220),
                                            Color32::from_rgb(66, 34, 30),
                                        ),
                                    };
                                    Frame::new()
                                        .fill(fill)
                                        .inner_margin(Margin::symmetric(4, 1))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.allocate_ui_with_layout(
                                                    Vec2::new(58.0, 14.0),
                                                    Layout::left_to_right(Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "{:>3} {:>3}",
                                                                line.old_line
                                                                    .map(|n| n.to_string())
                                                                    .unwrap_or_else(|| "·".into()),
                                                                line.new_line
                                                                    .map(|n| n.to_string())
                                                                    .unwrap_or_else(|| "·".into()),
                                                            ))
                                                            .font(mono(8.5))
                                                            .color(if line.kind
                                                                == portmanteau_core::DiffKind::Same
                                                            {
                                                                PAPER_MUTED
                                                            } else {
                                                                sigil_color
                                                            }),
                                                        );
                                                    },
                                                );
                                                ui.label(
                                                    RichText::new(sigil)
                                                        .font(mono(10.5))
                                                        .color(sigil_color),
                                                );
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(if line.text.is_empty() {
                                                            " "
                                                        } else {
                                                            &line.text
                                                        })
                                                        .font(mono(10.0))
                                                        .color(text_color),
                                                    )
                                                    .wrap(),
                                                );
                                            });
                                        });
                                }
                            });
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Restoring replaces the current text with this saved version.")
                            .font(ui_small())
                            .color(FAINT),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Restore this version").clicked() {
                            restore = true;
                            close = true;
                        }
                    });
                });
            });

        if restore {
            if let Some(doc) = self.ws.project.document_mut(&state.doc_id) {
                if doc.restore_snapshot(&state.snapshot_id) {
                    self.ws.dirty = true;
                    self.selected = Some(state.doc_id.clone());
                    self.toast(
                        "Version restored",
                        format!("Replaced with “{}”", state.snapshot_name),
                        ToastKind::Info,
                    );
                }
            }
        }
        if !open || close {
            self.diff = None;
        }
    }

    fn path_modal(&mut self, ctx: &egui::Context) {
        let Some(mode) = self.path_dialog else { return };
        let mut open = true;
        let mut commit = false;
        let (title, verb, hint) = match mode {
            PathDialog::Open => (
                "Open project",
                "Open",
                "Path to a .portmanteau project folder (or a legacy .json file)",
            ),
            PathDialog::SaveAs => (
                "Save project as",
                "Save",
                "Where the project folder should live (or a legacy .json file)",
            ),
            PathDialog::Import => (
                "Import manuscript",
                "Import",
                "Path to a .fountain, .fdx, .docx or .txt file",
            ),
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(520.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                section_label(ui, "Project folder or file");
                ui.add_space(3.0);
                let resp = ui.add(
                    TextEdit::singleline(&mut self.path_draft)
                        .hint_text(hint)
                        .font(mono(10.5))
                        .desired_width(f32::INFINITY),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    commit = true;
                }
                ui.add_space(5.0);
                ui.label(
                    RichText::new(match mode {
                        PathDialog::Open => "Unsaved changes in the current project are left untouched.",
                        PathDialog::SaveAs => "A new copy is written; the open project switches to it.",
                        PathDialog::Import => {
                            "Fountain and FDX become screenplays; DOCX and text become prose in the Draft folder."
                        }
                    })
                    .font(ui_small())
                    .color(FAINT),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let btn = egui::Button::new(
                        RichText::new(verb)
                            .font(ui_body())
                            .color(Color32::WHITE),
                    )
                    .fill(ACCENT)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(100.0, 30.0));
                    if ui
                        .add_enabled(!self.path_draft.trim().is_empty(), btn)
                        .clicked()
                    {
                        commit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.path_dialog = None;
                    }
                });
            });

        if commit {
            match mode {
                PathDialog::Open => {
                    let path = PathBuf::from(self.path_draft.trim());
                    match Workspace::open(&path) {
                        Ok(ws) => {
                            self.ws = ws;
                            self.selected = self
                                .ws
                                .project
                                .documents
                                .iter()
                                .find(|d| d.kind == DocKind::Chapter)
                                .map(|d| d.id.clone());
                            self.collapsed.clear();
                            self.view = View::Write;
                            self.path_dialog = None;
                            self.toast("Project opened", path.display().to_string(), ToastKind::Ok);
                        }
                        Err(e) => {
                            self.toast("Could not open project", e, ToastKind::Error);
                        }
                    }
                }
                PathDialog::SaveAs => {
                    let mut path = PathBuf::from(self.path_draft.trim());
                    if path.extension().is_none() {
                        // A bare name means a folder package, not a legacy
                        // single-file project: "MyBook" -> "MyBook.portmanteau/".
                        path.set_extension("portmanteau");
                    }
                    match self.ws.save(Some(path.clone())) {
                        Ok(msg) => {
                            self.path_dialog = None;
                            self.toast("Project saved", msg, ToastKind::Ok);
                        }
                        Err(e) => {
                            self.toast("Could not save project", e, ToastKind::Error);
                        }
                    }
                }
                PathDialog::Import => {
                    let path = self.path_draft.trim().to_owned();
                    let res = self.ws.execute("document.import", serde_json::json!({
                        "path": path,
                    }));
                    if res.ok {
                        let id = res.data["id"].as_str().map(|id| id.to_owned());
                        self.selected = id;
                        self.view = View::Write;
                        self.path_dialog = None;
                        self.toast("Manuscript imported", res.message, ToastKind::Ok);
                    } else {
                        self.toast("Could not import manuscript", res.message, ToastKind::Error);
                    }
                }
            }
        }
        if !open && self.path_dialog.is_some() {
            self.path_dialog = None;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (save, save_as, open, compile, focus_toggle) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(Key::S),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::S),
                i.modifiers.command && i.key_pressed(Key::O),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::E),
                i.key_pressed(Key::F11),
            )
        });
        if save && !save_as {
            self.save_project();
        }
        if save_as {
            self.path_dialog = Some(PathDialog::SaveAs);
            self.path_draft = self
                .ws
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                    format!("{home}/Portmanteau/{}.portmanteau/", slug(&self.ws.project.title))
                });
        }
        if open {
            self.path_dialog = Some(PathDialog::Open);
            self.path_draft = self
                .ws
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
        }
        if compile {
            self.view = View::Compile;
        }
        if focus_toggle {
            self.focus_mode = !self.focus_mode;
        }
    }

    fn screenshot_tick(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else { return };
        self.frame_no = self.frame_no.saturating_add(1);
        ctx.request_repaint();
        if self.frame_no >= 120 && !self.screenshot_requested {
            self.screenshot_requested = true;
            let shot_ctx = ctx.clone();
            ctx.request_screenshot(move |image| {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let [w, h] = image.size;
                let mut rgba = Vec::with_capacity(w * h * 4);
                for px in &image.pixels {
                    let [r, g, b, a] = px.to_array();
                    rgba.extend_from_slice(&[r, g, b, a]);
                }
                match image::save_buffer(&path, &rgba, w as u32, h as u32, image::ColorType::Rgba8) {
                    Ok(_) => eprintln!("screenshot saved to {}", path.display()),
                    Err(e) => eprintln!("could not save screenshot: {e}"),
                }
                shot_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
    }
}

fn slug(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if s.is_empty() { "untitled".into() } else { s }
}

impl eframe::App for PortmanteauApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_shortcuts(&ctx);

        let focus = self.focus_mode && self.view == View::Write;
        if focus {
            self.focus_bar(ui);
        } else {
            self.top_bar(ui);
            self.status_bar(ui);
            self.rail(ui);
        }
        self.toast_ui(&ctx);

        if self.view == View::Write {
            let focus = self.focus_mode;
            if !focus {
                if self.settings.show_binder {
                    self.binder(ui);
                } else {
                    self.binder_stub(ui);
                }
                if self.settings.show_inspector {
                    self.inspector(ui);
                } else {
                    self.inspector_stub(ui);
                }
            }
            egui::CentralPanel::default()
                .frame(Frame::new().fill(BG))
                .show(ui, |ui| {
                    ui::editor::show(self, ui);
                });
        } else {
            egui::CentralPanel::default()
                .frame(Frame::new().fill(BG))
                .show(ui, |ui| {
                    ui::views::show(self, ui);
                });
        }

        self.modals(&ctx);
        self.screenshot_tick(&ctx);
    }
}

impl PortmanteauApp {
    fn binder(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("binder")
            .exact_size(272.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(12, 10)),
            )
            .show(ui, |ui| {
                ui::binder::show(self, ui);
            });
    }

    fn inspector(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("inspector")
            .exact_size(304.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| {
                ui::inspector::show(self, ui);
            });
    }

    /// A slim edge handle so a hidden panel can be brought back without
    /// reaching for the menu. Focus mode hides the handles too.
    fn binder_stub(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("binder_stub")
            .exact_size(18.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(1, 6)),
            )
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    let show = egui::Button::new(
                        RichText::new("›").font(ui_body()).color(FAINT),
                    )
                    .frame(false)
                    .min_size(Vec2::new(16.0, 28.0));
                    if ui
                        .add(show)
                        .on_hover_text("Show the binder")
                        .clicked()
                    {
                        self.set_binder_visible(true);
                    }
                });
            });
    }

    fn inspector_stub(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("inspector_stub")
            .exact_size(18.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(1, 6)),
            )
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    let show = egui::Button::new(
                        RichText::new("‹").font(ui_body()).color(FAINT),
                    )
                    .frame(false)
                    .min_size(Vec2::new(16.0, 28.0));
                    if ui
                        .add(show)
                        .on_hover_text("Show the inspector")
                        .clicked()
                    {
                        self.set_inspector_visible(true);
                    }
                });
            });
    }

    pub fn set_binder_visible(&mut self, visible: bool) {
        self.settings.show_binder = visible;
        let _ = crate::settings::save_settings(&self.settings);
    }

    pub fn set_inspector_visible(&mut self, visible: bool) {
        self.settings.show_inspector = visible;
        let _ = crate::settings::save_settings(&self.settings);
    }
}

/// Capability probes that drive the *whole* shell, not just `editor::show`.
///
/// The editor probes in `src/ui/editor.rs` render the writing surface alone, so
/// they cannot see a regression caused by the real chrome: the top bar, rail,
/// status bar, binder and inspector panels, toasts, focus order between panels
/// or the narrower editor column. These probes reproduce the body of
/// [`eframe::App::ui`] inside an `egui_kittest` harness so the script
/// autocomplete and arrow-key focus can be checked against the shell a writer
/// actually uses.
///
/// Diagnostics only: they do not replace the behaviour tests next to the code
/// they exercise.
#[cfg(test)]
mod shell_probe {
    use super::*;
    use eframe::egui::accesskit::Role;
    use eframe::egui::{Event, Rect};
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The real app window: 1440x930.
    const SHELL: Vec2 = Vec2::new(1440.0, 930.0);

    /// The editor column sits between the 62 px rail + 272 px binder on the
    /// left and the 304 px inspector on the right. Anything inside this band is
    /// the writing surface, so the inspector's own text fields are excluded.
    fn in_editor_column(rect: Rect) -> bool {
        let x = rect.center().x;
        (340.0..1130.0).contains(&x)
    }

    /// A demo project open on its first script document, with view = Write and
    /// the first Character cue blanked so the probe types into a known empty
    /// box. Returns the document id and the cue's block index.
    fn shell_fixture() -> (PortmanteauApp, String, usize) {
        let mut app = PortmanteauApp::new(None);
        app.settings.spellcheck = false;
        app.show_spelling = false;
        app.view = View::Write;
        app.write_mode = WriteMode::Continuous;
        app.focus_mode = false;
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
        let cue = app
            .ws
            .project
            .document_mut(&doc_id)
            .and_then(|doc| {
                doc.content
                    .paragraphs
                    .iter()
                    .position(|paragraph| paragraph.style_id == "character")
                    .map(|index| {
                        doc.content.paragraphs[index].set_plain_text("");
                        index
                    })
            })
            .expect("the script fixture should keep its Character cue");
        (app, doc_id, cue)
    }

    /// The script block's widget id, matching the expression `editor::show`
    /// uses (`Id::unique(("script_block", doc_id, index))`).
    fn block_id(doc_id: &str, index: usize) -> Id {
        let _ = index;
        Id::unique(("script_flow", doc_id))
    }

    fn paragraph_start(document: &portmanteau_core::Document, index: usize) -> usize {
        document
            .content
            .paragraphs
            .iter()
            .take(index)
            .map(|paragraph| paragraph.editor_text().chars().count() + 1)
            .sum()
    }

    /// Draw the shell exactly as `eframe::App::ui` does, without the modal and
    /// screenshot calls (the fixture opens no modal and has no screenshot).
    /// Font installation happens on the first, discarded frame.
    fn shell_harness(app: PortmanteauApp) -> (Harness<'static>, Rc<RefCell<PortmanteauApp>>) {
        let app = Rc::new(RefCell::new(app));
        let ui_app = Rc::clone(&app);
        let mut themed = false;
        let mut ready = false;
        let mut harness = Harness::builder()
            .with_size(SHELL)
            .with_max_steps(96)
            .with_accessibility_check(false)
            // Keep the harness lenient about glyphs: the bundled families
            // cover the current symbol set, but a mark outside Liberation and
            // DejaVu would otherwise panic here and hide every shell-level
            // finding behind it (see AGENTS.md, "missing glyphs").
            .allow_missing_glyphs()
            .build_ui(move |ui| {
                if !themed {
                    crate::theme::install(ui.ctx());
                    themed = true;
                    return;
                }
                if !ready {
                    ready = true;
                    return;
                }
                let ctx = ui.ctx().clone();
                let mut app = ui_app.borrow_mut();
                app.top_bar(ui);
                app.status_bar(ui);
                app.rail(ui);
                app.toast_ui(&ctx);
                if app.view == View::Write {
                    if !app.focus_mode {
                        app.binder(ui);
                    }
                    if !app.focus_mode {
                        app.inspector(ui);
                    }
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(BG))
                        .show(ui, |ui| {
                            crate::ui::editor::show(&mut app, ui);
                        });
                }
            });
        harness.run_steps(6);
        (harness, app)
    }

    #[test]
    fn shell_character_popup_fills_the_block_from_a_pointer_click() {
        let (app, doc_id, cue) = shell_fixture();
        let (mut harness, app) = shell_harness(app);

        harness
            .get_all_by_role(Role::MultilineTextInput)
            .find(|node| in_editor_column(node.rect()) && node.value().is_some())
            .expect("the script flow should be a multiline input in the editor column")
            .click();
        harness.run_steps(2);

        let id = block_id(&doc_id, cue);
        assert!(
            harness.ctx.memory(|memory| memory.has_focus(id)),
            "a real pointer click in the full shell should focus the script block"
        );
        let caret = paragraph_start(
            app.borrow().ws.project.document(&doc_id).unwrap(),
            cue,
        );
        let mut state =
            egui::text_edit::TextEditState::load(&harness.ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
            egui::text::CCursor::new(caret),
        )));
        state.store(&harness.ctx, id);
        harness.run_steps(1);

        harness.input_mut().events.push(Event::Text("ma".into()));
        harness.run_steps(2);
        let body_after_type = app
            .borrow()
            .ws
            .project
            .document(&doc_id)
            .unwrap()
            .content
            .editor_text();
        let selected_after_type = app.borrow().selected_block;
        assert!(
            harness
                .query_by_role_and_label(Role::Button, "Mara Venn")
                .is_some(),
            "typing into the clicked cue should open the completion popup in the full shell; \
             selected={selected_after_type}, body={body_after_type:?}"
        );

        harness
            .get_by_role_and_label(Role::Button, "Mara Venn")
            .click();
        harness.run_steps(2);
        let doc = app.borrow();
        let document = doc.ws.project.document(&doc_id).unwrap();
        let sheet = doc
            .ws
            .project
            .style_sheets
            .iter()
            .find(|sheet| sheet.id == document.style_sheet_id)
            .unwrap();
        let text = document.content.to_script_blocks(sheet)[cue].text.clone();
        assert_eq!(
            text, "Mara Venn",
            "clicking a name in the full shell should fill the Character block"
        );
        assert!(
            app.borrow().new_character.is_none(),
            "choosing a name must not open the new-character modal"
        );
    }

    #[test]
    fn shell_arrows_stay_inside_the_clicked_block() {
        let (mut app, doc_id, _cue) = shell_fixture();
        app.ws
            .project
            .document_mut(&doc_id)
            .unwrap()
            .set_script_blocks(vec![
                portmanteau_core::Block::new(ElementKind::Action, "one\ntwo"),
                portmanteau_core::Block::new(ElementKind::Dialogue, "three"),
            ]);
        let (mut harness, app) = shell_harness(app);

        harness
            .get_all_by_role(Role::MultilineTextInput)
            .find(|node| in_editor_column(node.rect()) && node.value().as_deref().is_some())
            .expect("the script should expose a multiline block in the editor column")
            .click();
        harness.run_steps(2);

        let id = block_id(&doc_id, 0);
        assert!(
            harness.ctx.memory(|memory| memory.has_focus(id)),
            "the click should focus the script flow in the full shell"
        );
        let mut state =
            egui::text_edit::TextEditState::load(&harness.ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
            egui::text::CCursor::new(0),
        )));
        state.store(&harness.ctx, id);
        harness.run_steps(1);

        harness.key_press(Key::ArrowDown);
        harness.run_steps(2);
        assert!(
            harness.ctx.memory(|memory| memory.has_focus(id)),
            "ArrowDown in the full shell must not move focus out of the flow"
        );
        let down = egui::text_edit::TextEditState::load(&harness.ctx, id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| range.primary.index.0)
            .expect("the flow editor should expose its caret");
        assert_eq!(down, 4, "ArrowDown should enter the second visual line");

        harness.key_press(Key::ArrowUp);
        harness.run_steps(2);
        assert!(
            harness.ctx.memory(|memory| memory.has_focus(id)),
            "ArrowUp in the full shell must not move focus out of the flow"
        );
        let up = egui::text_edit::TextEditState::load(&harness.ctx, id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| range.primary.index.0)
            .expect("the flow editor should expose its caret");
        assert_eq!(up, 0, "ArrowUp should return to the first visual line");
        assert!(app.borrow().selected_block < 2);
    }

    /// The shell-level version of the first-focus-frame bug: the app asks for
    /// paragraph 0's focus (as `+ Add element`, a click into a cue or a menu action
    /// does) in the same frame an unmodified arrow arrives. egui reads the
    /// focused widget's event filter at the start of the pass, so the freshly
    /// requested focus has none yet; without the editor's cancellation the
    /// arrow would hand focus to the next widget in the shell's focus order.
    #[test]
    fn shell_arrow_in_the_first_focus_frame_stays_in_the_block() {
        let (app, doc_id, _cue) = shell_fixture();
        let (mut harness, app) = shell_harness(app);
        let id = block_id(&doc_id, 0);
        assert!(
            !harness.ctx.memory(|memory| memory.has_focus(id)),
            "the fixture should start with no block focused"
        );

        app.borrow_mut().block_focus = Some(0);
        harness.key_press(Key::ArrowDown);
        harness.run_steps(1);

        assert!(
            harness.ctx.memory(|memory| memory.has_focus(id)),
            "an arrow in the frame that first focuses the flow must stay in it \
             even with the full shell's focus order"
        );
        assert!(
            app.borrow().selected_block < app.borrow().current_doc().unwrap().content.paragraphs.len(),
            "the caret should remain on a paragraph in the flow"
        );
    }

    #[test]
    fn inserting_an_image_copies_asset_and_preserves_rich_document_on_save() {
        let source_dir = std::env::temp_dir().join(portmanteau_core::new_id("image-source"));
        let project_dir = std::env::temp_dir().join(portmanteau_core::new_id("image-project"));
        std::fs::create_dir_all(&source_dir).expect("create source dir");
        let source = source_dir.join("harbor.png");
        image::RgbaImage::new(4, 3)
            .save(&source)
            .expect("write a tiny test image");

        let mut app = PortmanteauApp::new(None);
        let doc_id = app.ws.project.add_document(
            None,
            "Image test",
            DocKind::Chapter,
            Medium::Prose,
        );
        app.ws
            .project
            .document_mut(&doc_id)
            .unwrap()
            .set_plain_text("before after");
        app.ws.path = Some(project_dir.clone());
        app.insert_image_from_state(&ImageInsertState {
            target: RichTextTarget::DocumentBody(doc_id.clone()),
            range: 7..7,
            path: source.display().to_string(),
            alt_text: "Harbor map".into(),
            caption: "Low tide".into(),
            standalone: true,
            error: String::new(),
        })
        .expect("insert image");

        let document = app.ws.project.document(&doc_id).unwrap();
        let image = document
            .content
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.content.iter())
            .find_map(|item| match item {
                portmanteau_core::InlineContent::Image(image) => Some(image),
                _ => None,
            })
            .expect("rich content references the image object");
        let asset_id = image.asset_id.clone();
        assert_eq!(image.alt_text, "Harbor map");
        assert_eq!(image.caption, "Low tide");
        assert!(project_dir.join("assets").join(&asset_id).is_file());

        portmanteau_api::storage::save_folder(&app.ws.project, &project_dir)
            .expect("save image project");
        let reopened = portmanteau_api::storage::load_folder(&project_dir)
            .expect("reopen image project");
        let reopened_doc = reopened.document(&doc_id).unwrap();
        assert!(reopened_doc.content.paragraphs.iter().any(|paragraph| {
            paragraph.content.iter().any(
                |item| matches!(item, portmanteau_core::InlineContent::Image(image) if image.asset_id == asset_id),
            )
        }));
        assert!(project_dir.join("assets").join(asset_id).is_file());

        let _ = std::fs::remove_dir_all(&source_dir);
        let _ = std::fs::remove_dir_all(&project_dir);
    }
}
