//! Portmanteau desktop shell.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};
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
    Search,
    Database,
    History,
    Project,
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
            View::Search => "Search",
            View::Database => "Database",
            View::History => "Versions",
            View::Project => "Project",
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
            View::Search => Icon::Search,
            View::Database => Icon::Database,
            View::History => Icon::History,
            View::Project => Icon::Project,
            View::Compile => Icon::Compile,
            View::Agent => Icon::Agent,
            View::Settings => Icon::Settings,
        }
    }
    pub fn all() -> [View; 10] {
        [
            View::Write,
            View::Corkboard,
            View::Outline,
            View::Research,
            View::Search,
            View::Database,
            View::History,
            View::Project,
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

/// A path shortened for display: the home folder becomes `~`.
pub(crate) fn short_path(path: &Path) -> String {
    let full = path.display().to_string();
    if let Ok(home) = std::env::var("HOME") {
        let home = home.trim_end_matches('/');
        if let Some(rest) = full.strip_prefix(&format!("{home}/")) {
            return format!("~/{rest}");
        }
    }
    full
}

/// The system file dialog for one of the path dialogs, starting near what is
/// already typed. `None` when the writer cancels.
fn browse_for(mode: PathDialog, current: &str) -> Option<String> {
    let start = Path::new(current.trim());
    let start_dir = if start.is_dir() { Some(start) } else { start.parent().filter(|p| p.is_dir()) };
    let mut dialog = rfd::FileDialog::new();
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    let picked = match mode {
        PathDialog::Open => dialog.set_title("Open project").pick_folder(),
        PathDialog::SaveAs => dialog
            .set_title("Save project as")
            .set_file_name("My Project.portmanteau")
            .save_file(),
    };
    picked.map(|path| path.display().to_string())
}

/// The image's stored description: displayed at most 240 points on a side as
/// a figure on its own paragraph, 20 as a glyph within a line of text.
fn image_object(
    file_name: &str,
    pixel_width: u32,
    pixel_height: u32,
    standalone: bool,
    alt: &str,
    caption: &str,
) -> portmanteau_core::ImageObject {
    let display_max = if standalone { 240.0 } else { 20.0 };
    let scale = display_max / pixel_width.max(pixel_height).max(1) as f32;
    portmanteau_core::ImageObject {
        asset_id: format!("images/{file_name}"),
        alt_text: alt.trim().to_owned(),
        caption: caption.trim().to_owned(),
        width: Some(pixel_width as f32 * scale),
        height: Some(pixel_height as f32 * scale),
    }
}

/// A link the desktop may be asked to open: web addresses with a host, and
/// nothing else. A project from elsewhere could carry any link it likes
/// (`file:`, a custom scheme, a command-looking string), so nothing but a plain
/// `http` or `https` address ever reaches the system's opener. (Email links are
/// deliberately not followed: `mailto:` can carry attachment and header
/// parameters that mail programs act on.) Whitespace, control characters,
/// backslashes, quotes, angle brackets and the characters shells and URL
/// handlers treat specially are refused too.
pub(crate) fn safe_external_url(link: &str) -> Option<&str> {
    let link = link.trim();
    if link.chars().any(|c| c.is_whitespace() || c.is_control() || "\\\"<>|^`".contains(c)) {
        return None;
    }
    let lower = link.to_ascii_lowercase();
    let rest = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("");
    let host = host
        .rsplit_once(':')
        .map_or(host, |(h, port)| if port.chars().all(|c| c.is_ascii_digit()) { h } else { host });
    (!host.is_empty() && host != "." && !host.starts_with('-')).then_some(link)
}

/// Ask the desktop to open a web address (already vetted by
/// `safe_external_url`). The address is one argument to the opener, never part
/// of a shell command line, so `&` or `;` in it mean nothing.
fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    // Not `explorer`: it treats its argument as a path or command, while the
    // protocol handler takes a URL and nothing else.
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("rundll32");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = std::process::Command::new("xdg-open");
    let _ = command.arg(url).spawn();
}

/// Ask the desktop to open a folder itself (not the one above it).
pub(crate) fn open_folder(folder: &Path) {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = std::process::Command::new("explorer");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = std::process::Command::new("xdg-open");
    let _ = command.arg(folder).spawn();
}

/// Ask the desktop to open a file. When the file has been moved or deleted,
/// open its folder instead so the click still leads somewhere useful.
pub(crate) fn open_path(path: &Path) {
    let target = if path.is_file() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    };
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = std::process::Command::new("explorer");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = std::process::Command::new("xdg-open");
    let _ = command.arg(&target).spawn();
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Document,
    Notes,
    History,
}

/// A place in a document's text to put the caret.
pub struct CaretRequest {
    pub doc_id: String,
    pub paragraph: usize,
    /// Characters into the paragraph.
    pub offset: usize,
    /// Characters to select from there (a found word); 0 places a bare caret.
    pub select: usize,
    /// Move the keyboard focus into the text (false while typing in the find bar).
    pub focus: bool,
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
    /// Set by the Paged view when a paragraph is clicked: the editor opens that
    /// document in Continuous with the caret there, then clears it.
    pub caret_request: Option<CaretRequest>,
    pub inspector_tab: InspectorTab,
    /// Undo and redo for the selected document's text (see `history.rs`).
    pub history: crate::history::History,
    /// A burst of typing is open and needs a repaint to close on a pause.
    history_pending: bool,
    pub collapsed: HashSet<String>,
    pub search: String,
    pub focus_search: bool,
    pub search_page: ui::views::search::SearchPage,
    pub find_bar: ui::find_bar::FindBar,
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
    /// The import review window, while it is open.
    pub import_review: Option<crate::ui::import_review::ImportReview>,
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
    /// What the config file last held, so changes can be saved once they settle.
    settings_saved: portmanteau_core::AppSettings,
    settings_changed_at: Option<Instant>,
    /// When the writer last touched the keyboard or mouse (for autosave).
    last_input: Instant,
    pub spell: portmanteau_core::spell::SpellChecker,
    pub show_spelling: bool,
}

impl PortmanteauApp {
    pub fn new(screenshot: Option<PathBuf>) -> Self {
        let settings = crate::settings::load_settings();
        // Reopen the project the writer left open. Screenshots and the test
        // suite always start from the demonstration project, so they stay
        // repeatable whatever is on this machine.
        // PORTMANTEAU_PROJECT opens a given project even for a screenshot (a
        // development aid, like PORTMANTEAU_VIEW).
        let dev_project = std::env::var("PORTMANTEAU_PROJECT").ok();
        let reopened = (!cfg!(test) && (screenshot.is_none() || dev_project.is_some()))
            .then(|| dev_project.as_deref().or(settings.last_project.as_deref()))
            .flatten()
            .and_then(|path| Workspace::open(Path::new(path)).ok());
        let ws = reopened.unwrap_or_else(Workspace::demo);
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
        let selected_research = ws.project.research_docs().first().map(|d| d.id.clone());
        let mut collapsed = HashSet::new();
        if let Some(draft) = ws.project.folder_id("Draft") {
            collapsed.remove(&draft);
        }
        let mut spell = crate::spellcheck::load(&settings.language);
        spell.set_custom(settings.custom_words.clone());
        let compile_tab = ExportKind::all()
            .into_iter()
            .find(|kind| kind.short() == settings.compile_format)
            .map_or_else(CompileTab::default, CompileTab::Format);
        let start_paged = settings.paged_view;
        let editor_zoom = settings.editor_zoom.clamp(0.7, 1.6);
        let corkboard_zoom = settings.corkboard_zoom.clamp(0.6, 1.6);
        let settings_saved = settings.clone();
        let import_prefs = settings.import.clone();
        let mut app = Self {
            ws,
            settings,
            settings_saved,
            settings_changed_at: None,
            last_input: Instant::now(),
            view: initial_view(),
            compile_tab,
            settings_tab: 0,
            // Development aid, like PORTMANTEAU_VIEW: open Write in Paged mode.
            write_mode: if std::env::var("PORTMANTEAU_PAGED").is_ok() || start_paged {
                WriteMode::Paged
            } else {
                WriteMode::Continuous
            },
            paged_page: 1,
            preview_page: std::env::var("PORTMANTEAU_PREVIEW_PAGE").ok().and_then(|v| v.parse().ok()).unwrap_or(0),
            settings_preview_page: 0,
            selected,
            selected_block: 0,
            block_focus: None,
            caret_request: None,
            inspector_tab: InspectorTab::Document,
            history: crate::history::History::default(),
            history_pending: false,
            collapsed,
            search: String::new(),
            focus_search: false,
            // Development aid: PORTMANTEAU_FIND=text opens the editor's find bar with that text.
            find_bar: {
                let mut bar = ui::find_bar::FindBar::default();
                if let Ok(text) = std::env::var("PORTMANTEAU_FIND") {
                    bar.open = true;
                    bar.query = text;
                    bar.show_replace = true;
                }
                bar
            },
            // Development aid: PORTMANTEAU_SEARCH=text opens the Search page with that query.
            search_page: {
                let mut page = ui::views::search::SearchPage::default();
                if let Ok(text) = std::env::var("PORTMANTEAU_SEARCH") {
                    page.query = text;
                    page.show_replace = true;
                }
                page
            },
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
            // Development aid, like PORTMANTEAU_VIEW: open the import review on a file.
            import_review: std::env::var("PORTMANTEAU_IMPORT")
                .ok()
                .map(|path| crate::ui::import_review::ImportReview::for_path(&import_prefs, &path)),
            path_draft: String::new(),
            new_character: None,
            image_insert: None,
            image_textures: HashMap::new(),
            diff: None,
            outline_filter: 0,
            screenshot,
            frame_no: 0,
            screenshot_requested: false,
            editor_zoom,
            corkboard_zoom,
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
        let parent = self.ws.project.new_document_home(self.selected.as_deref());
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
                        // Removed: a "Compile ▸" button that only switched to the
                        // Compile screen. Navigation is the rail and the View
                        // menu; a one-click "compile with the last settings"
                        // action would be worth adding here when it can run the
                        // export itself and report the file.
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
            let recent = self.settings.recent_projects.clone();
            ui.add_enabled_ui(!recent.is_empty(), |ui| {
                ui.menu_button("Open recent", |ui| {
                    for path in &recent {
                        if ui.button(short_path(Path::new(path))).clicked() {
                            self.open_project_path(Path::new(path));
                            ui.close();
                        }
                    }
                });
            });
            if ui.button("Import Fountain, FDX, DOCX or Text…").clicked() {
                self.import_review = Some(crate::ui::import_review::ImportReview::new(&self.settings.import));
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
            let ctx = ui.ctx().clone();
            if ui
                .add_enabled(self.can_undo(), egui::Button::new("Undo    ⌘Z"))
                .clicked()
            {
                self.step_history(&ctx, true);
                ui.close();
            }
            if ui
                .add_enabled(self.can_redo(), egui::Button::new("Redo    ⇧⌘Z"))
                .clicked()
            {
                self.step_history(&ctx, false);
                ui.close();
            }
            ui.separator();
            if ui.button("Search project… (Ctrl+Shift+F)").clicked() {
                self.open_search(None);
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
                        // At the caret if the document has been clicked into, else at the end.
                        let end = doc.content.char_count();
                        let caret = egui::text_edit::TextEditState::load(ui.ctx(), Id::unique(("prose_body", id.as_str())))
                            .and_then(|state| state.cursor.char_range())
                            .map_or(end, |range| range.primary.index.0.min(end));
                        self.image_insert = Some(ImageInsertState {
                            target: RichTextTarget::DocumentBody(id),
                            range: caret..caret,
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
                            .align(egui::Align2::LEFT_CENTER)
                            .frame(Frame::NONE)
                            .font(ui_body()),
                    );
                    if self.focus_search {
                        resp.request_focus();
                        self.focus_search = false;
                    }
                    // Enter hands what was typed to the Search page.
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) && !self.search.trim().is_empty() {
                        let text = std::mem::take(&mut self.search);
                        self.open_search(Some(text));
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
                    // The brand mark is drawn once, in the top bar; the rail used
                    // to repeat it here.
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
                    // Removed: an "AM" avatar circle that did nothing. It should
                    // become the author's profile (name, byline, default author
                    // for new projects) once the app has a notion of a person;
                    // until then Settings is the only place that applies.
                    // Settings is pinned to the foot of the rail rather than listed
                    // with the workspaces.
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
                        // Words only. This bar used to add a page estimate and the
                        // open document's name; the estimate disagreed with the
                        // Compile screen's, and the name is already in the editor.
                        ui.label(
                            RichText::new(format!("{} words", self.ws.project.words()))
                                .font(ui_body())
                                .color(MUTED),
                        );
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
                section_label(ui, "Image file");
                ui.horizontal(|ui| {
                    ui.add(
                        TextEdit::singleline(&mut state.path)
                            .hint_text("/path/to/image.png")
                            .desired_width(ui.available_width() - 90.0),
                    );
                    if ui.button("Browse…").clicked() {
                        if let Some(file) = rfd::FileDialog::new()
                            .set_title("Choose an image")
                            .add_filter("Pictures", &["png", "jpg", "jpeg", "gif", "webp"])
                            .pick_file()
                        {
                            state.path = file.display().to_string();
                        }
                    }
                });
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

    /// Copy an image file into the project and describe it.
    fn store_image_file(&mut self, source: &Path, standalone: bool, alt: &str, caption: &str) -> Result<portmanteau_core::ImageObject, String> {
        let root = self
            .ws
            .path
            .clone()
            .ok_or_else(|| "Save the project before inserting images.".to_string())?;
        if !source.is_file() {
            return Err("Choose an existing image file.".into());
        }
        let (pixel_width, pixel_height) = image::image_dimensions(source)
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
        std::fs::copy(source, image_dir.join(&file_name))
            .map_err(|error| format!("Could not copy the image into the project: {error}"))?;
        Ok(image_object(&file_name, pixel_width, pixel_height, standalone, alt, caption))
    }

    /// Store pixels (a pasted picture) in the project as a PNG and describe it.
    fn store_image_pixels(&mut self, width: u32, height: u32, rgba: Vec<u8>, standalone: bool) -> Result<portmanteau_core::ImageObject, String> {
        let root = self
            .ws
            .path
            .clone()
            .ok_or_else(|| "Save the project before pasting images: they are kept in its folder.".to_string())?;
        let buffer = image::RgbaImage::from_raw(width, height, rgba)
            .ok_or_else(|| "The clipboard image did not have the expected size.".to_string())?;
        let file_name = format!("{}.png", portmanteau_core::new_id("img"));
        let image_dir = root.join("assets").join("images");
        std::fs::create_dir_all(&image_dir)
            .map_err(|error| format!("Could not create the project image folder: {error}"))?;
        image::DynamicImage::ImageRgba8(buffer)
            .save(image_dir.join(&file_name))
            .map_err(|error| format!("Could not store the image: {error}"))?;
        Ok(image_object(&file_name, width, height, standalone, "", ""))
    }

    fn insert_image_from_state(&mut self, state: &ImageInsertState) -> Result<(), String> {
        let image = self.store_image_file(
            &PathBuf::from(state.path.trim()),
            state.standalone,
            &state.alt_text,
            &state.caption,
        )?;
        self.insert_image_object(&state.target, state.range.clone(), image, state.standalone)
    }

    /// Put a stored image into a text field at `range`.
    fn insert_image_object(
        &mut self,
        target: &RichTextTarget,
        range: std::ops::Range<usize>,
        image: portmanteau_core::ImageObject,
        standalone: bool,
    ) -> Result<(), String> {
        let alt_text = image.alt_text.clone();
        let state = ImageInsertState {
            target: target.clone(),
            range,
            path: String::new(),
            alt_text: alt_text.clone(),
            caption: String::new(),
            standalone,
            error: String::new(),
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
        };
        if content.is_none() {
            return Err("The target text field no longer exists.".into());
        }
        self.ws.project.touch();
        self.ws.dirty = true;
        self.toast("Image inserted", alt_text, ToastKind::Ok);
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
                ui.add_space(4.0);
                if ui
                    .button("Browse…")
                    .on_hover_text("Choose with the system file dialog")
                    .clicked()
                {
                    if let Some(chosen) = browse_for(mode, &self.path_draft) {
                        self.path_draft = chosen;
                    }
                }
                ui.add_space(5.0);
                ui.label(
                    RichText::new(match mode {
                        PathDialog::Open => "Unsaved changes in the current project are left untouched.",
                        PathDialog::SaveAs => "A new copy is written; the open project switches to it.",
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
                    if self.open_project_path(&path) {
                        self.path_dialog = None;
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
            }
        }
        if !open && self.path_dialog.is_some() {
            self.path_dialog = None;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.view != View::Search {
            // Results are worked out again each time the page is opened.
            self.search_page.stale();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, Key::F)) {
            self.open_search(None);
        }
        // Ctrl/Cmd+F: the find bar over the open document (Ctrl/Cmd+H opens it with Replace).
        let can_find = self.view == View::Write
            && self.write_mode == WriteMode::Continuous
            && self.selected.as_deref().and_then(|id| self.ws.project.document(id)).is_some_and(|d| !d.kind.is_folder());
        if can_find {
            let (find, replace) = ctx.input_mut(|i| {
                (i.consume_key(egui::Modifiers::COMMAND, Key::F), i.consume_key(egui::Modifiers::COMMAND, Key::H))
            });
            if find || replace {
                ui::find_bar::open(self, replace);
            }
        }
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
        self.draw(ui);
    }

    /// Closing the window must not lose the last couple of seconds of work, or
    /// the preferences that were still settling.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.ws.dirty && self.ws.path.is_some() {
            let _ = self.ws.save(None);
        }
        let _ = crate::settings::save_settings(&self.settings);
    }
}

impl PortmanteauApp {
    /// One frame of the whole window. Separate from `eframe::App::ui` so tests
    /// can drive the real thing without an `eframe::Frame`.
    pub(crate) fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.handle_shortcuts(&ctx);
        self.undo_redo_shortcuts(&ctx);
        self.paste_image_shortcut(&ctx);
        self.insert_dropped_images(&ctx);
        crate::ui::mention::publish_targets(&ctx, &self.ws.project);
        ctx.data_mut(|data| data.insert_temp(Id::unique("spell-on"), self.settings.spellcheck));
        if let Some(link) = crate::ui::editor::take_clicked_link(&ctx) {
            self.open_link(&link);
        }

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
        crate::ui::import_review::show(self, &ctx);
        self.track_history(&ctx);
        self.persist_session(&ctx);
        self.autosave(&ctx);
        self.screenshot_tick(&ctx);
    }
}

/// How long the writer must be idle before a saved project is saved again.
const AUTOSAVE_IDLE: Duration = Duration::from_millis(2500);
/// How long a preference must hold still before it is written to the config.
const SETTINGS_SETTLE: Duration = Duration::from_millis(500);

impl PortmanteauApp {
    /// Keep what the writer chose (zoom, view mode, Compile format, window
    /// size, the open project) in the config file, so the next session starts
    /// where this one stopped. Written once a change has settled, not on every
    /// frame of a window being dragged.
    fn persist_session(&mut self, ctx: &egui::Context) {
        let CompileTab::Format(kind) = self.compile_tab;
        self.settings.editor_zoom = self.editor_zoom;
        self.settings.corkboard_zoom = self.corkboard_zoom;
        self.settings.paged_view = self.write_mode == WriteMode::Paged;
        self.settings.compile_format = kind.short().to_string();
        let (fullscreen, maximized) =
            ctx.input(|i| (i.viewport().fullscreen, i.viewport().maximized));
        if fullscreen != Some(true) && maximized != Some(true) {
            if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
                self.settings.window_size = Some([rect.width().round(), rect.height().round()]);
            }
        }
        if let Some(path) = self.ws.path.as_ref().map(|p| p.display().to_string()) {
            if self.settings.last_project.as_deref() != Some(path.as_str()) {
                self.settings.remember_project(&path);
            }
        }
        if self.settings == self.settings_saved {
            self.settings_changed_at = None;
            return;
        }
        let since = *self.settings_changed_at.get_or_insert_with(Instant::now);
        if since.elapsed() >= SETTINGS_SETTLE {
            if crate::settings::save_settings(&self.settings).is_ok() {
                self.settings_saved = self.settings.clone();
            }
            self.settings_changed_at = None;
        } else {
            ctx.request_repaint_after(SETTINGS_SETTLE);
        }
    }

    /// Save a project that already has a home once the writer pauses, so
    /// changes (text, Compile settings, details) survive without pressing
    /// Save. A project that has never been saved is left alone: choosing where
    /// it lives is the writer's decision.
    fn autosave(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| !i.events.is_empty() || i.pointer.any_down()) {
            self.last_input = Instant::now();
        }
        if !self.ws.dirty || self.ws.path.is_none() {
            return;
        }
        let idle = self.last_input.elapsed();
        if idle < AUTOSAVE_IDLE {
            ctx.request_repaint_after(AUTOSAVE_IDLE - idle);
            return;
        }
        if let Err(error) = self.ws.save(None) {
            // Say so once, then wait for the next change rather than retry
            // (and complain) every frame.
            self.ws.dirty = false;
            self.toast("Autosave failed", error, ToastKind::Error);
        }
    }

    /// The text field a paste or drop should reach: the document body (or
    /// research note) being written in, and where its caret is. Scripts take no
    /// pictures.
    fn image_target(&self, ctx: &egui::Context) -> Option<(RichTextTarget, std::ops::Range<usize>, Option<Id>)> {
        let doc_id = match self.view {
            View::Research => self.selected_research.clone()?,
            View::Write => self.selected.clone()?,
            _ => return None,
        };
        let doc = self.ws.project.document(&doc_id)?;
        if doc.medium.is_script() || doc.kind.is_folder() {
            return None;
        }
        let ids = [
            Id::unique(("prose_body", doc_id.as_str())),
            Id::unique(("research_body", doc_id.as_str())),
        ];
        let focused = ctx.memory(|memory| memory.focused());
        let field = ids.into_iter().find(|id| focused == Some(*id));
        let probe = field.or_else(|| ids.into_iter().find(|id| egui::text_edit::TextEditState::load(ctx, *id).is_some()));
        let end = doc.content.char_count();
        let caret = probe
            .and_then(|id| egui::text_edit::TextEditState::load(ctx, id))
            .and_then(|state| state.cursor.char_range())
            .map_or(end, |range| range.primary.index.0.min(end));
        Some((RichTextTarget::DocumentBody(doc_id), caret..caret, field))
    }

    /// Ctrl/Cmd+V with a picture on the clipboard puts it in the text at the
    /// caret, as its own paragraph. (egui only passes text through a paste, so
    /// the key itself is watched; a paste that carries text is left to the
    /// field.) The picture is kept in the project's `assets/images`.
    fn paste_image_shortcut(&mut self, ctx: &egui::Context) {
        let pressed = ctx.input(|i| {
            i.modifiers.command
                && !i.modifiers.shift
                && i.key_pressed(Key::V)
                && !i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)))
        });
        if !pressed {
            return;
        }
        let Some((target, range, Some(_focused))) = self.image_target(ctx) else { return };
        let Ok(image) = arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_image()) else {
            return; // no picture there: nothing to do
        };
        let stored = self.store_image_pixels(image.width as u32, image.height as u32, image.bytes.into_owned(), true);
        match stored.and_then(|object| self.insert_image_object(&target, range, object, true)) {
            Ok(()) => ctx.request_repaint(),
            Err(error) => self.toast("Could not paste the picture", error, ToastKind::Error),
        }
    }

    /// Picture files dropped on the writing surface are inserted at the caret.
    /// (In Research, dropped files are attached to the note instead.)
    fn insert_dropped_images(&mut self, ctx: &egui::Context) {
        if self.view != View::Write {
            return;
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| {
                    p.is_file()
                        && p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                            ["png", "jpg", "jpeg", "gif", "webp"].contains(&e.to_ascii_lowercase().as_str())
                        })
                })
                .collect()
        });
        for path in dropped {
            let Some((target, range, _)) = self.image_target(ctx) else { return };
            let result = self
                .store_image_file(&path, true, "", "")
                .and_then(|object| self.insert_image_object(&target, range, object, true));
            if let Err(error) = result {
                self.toast("Could not insert the picture", error, ToastKind::Error);
            }
        }
    }

    /// Show the Search page, optionally with a new query, ready to type in.
    pub fn open_search(&mut self, text: Option<String>) {
        if let Some(text) = text {
            self.search_page.query = text;
        }
        self.search_page.focus = true;
        self.search_page.stale();
        self.view = View::Search;
    }

    /// Go to a search result: the document (or note, or item) with the found
    /// text selected where it can be.
    pub fn open_hit(&mut self, hit: &portmanteau_core::FindHit) {
        use portmanteau_core::{FindSource, HitField};
        match &hit.source {
            FindSource::Item(id) => {
                self.selected_item = Some(id.clone());
                self.view = View::Database;
            }
            FindSource::Document(id) if hit.group == portmanteau_core::SearchGroup::Research => {
                self.selected_research = Some(id.clone());
                self.view = View::Research;
                if hit.field == HitField::Body {
                    self.caret_request = Some(CaretRequest { doc_id: id.clone(), paragraph: hit.paragraph, offset: hit.offset, select: hit.end - hit.start, focus: true });
                }
            }
            FindSource::Document(id) => {
                self.open_doc(id.clone());
                self.write_mode = WriteMode::Continuous;
                if hit.field == HitField::Body {
                    self.caret_request = Some(CaretRequest { doc_id: id.clone(), paragraph: hit.paragraph, offset: hit.offset, select: hit.end - hit.start, focus: true });
                }
            }
        }
    }

    /// Open a document at the first place a database item appears in it. A
    /// research note opens in Research (it has no caret target); an appearance
    /// only in a synopsis or notes just opens the document.
    pub fn open_appearance(&mut self, appearance: &portmanteau_core::Appearance) {
        if appearance.in_research {
            self.selected_research = Some(appearance.doc_id.clone());
            self.view = View::Research;
            if let Some((paragraph, offset)) = appearance.place {
                self.caret_request = Some(CaretRequest { doc_id: appearance.doc_id.clone(), paragraph, offset, select: 0, focus: true });
            }
            return;
        }
        self.open_doc(appearance.doc_id.clone());
        self.write_mode = WriteMode::Continuous;
        if let Some((paragraph, offset)) = appearance.place {
            self.caret_request = Some(CaretRequest { doc_id: appearance.doc_id.clone(), paragraph, offset, select: 0, focus: true });
        }
    }

    /// Follow a link the writer Ctrl/Cmd-clicked: another document in the
    /// project (a research note opens in Research), or a web address.
    pub fn open_link(&mut self, link: &str) {
        if let Some(id) = link.strip_prefix(crate::ui::mention::ITEM_LINK) {
            if self.ws.project.items.iter().any(|item| item.id == id) {
                self.selected_item = Some(id.to_string());
                self.view = View::Database;
            } else {
                self.toast("That item no longer exists", "The link points at something that was deleted.", ToastKind::Info);
            }
        } else if let Some(id) = link.strip_prefix(portmanteau_core::rich_text::NOTE_LINK) {
            match self.ws.project.document(id) {
                Some(doc) if self.ws.project.in_research(doc) && !doc.kind.is_folder() => {
                    self.selected_research = Some(id.to_string());
                    self.view = View::Research;
                }
                Some(_) => self.open_doc(id.to_string()),
                None => self.toast("That document no longer exists", "The link points at something that was deleted.", ToastKind::Info),
            }
        } else if let Some(url) = safe_external_url(link) {
            open_url(url);
        } else {
            self.toast("Not opened", "Only web links are opened from here.", ToastKind::Info);
        }
    }

    /// Open a project folder, make it the one on screen and remember it.
    pub fn open_project_path(&mut self, path: &Path) -> bool {
        match Workspace::open(path) {
            Ok(ws) => {
                self.ws = ws;
                self.selected = self
                    .ws
                    .project
                    .documents
                    .iter()
                    .find(|d| d.kind == DocKind::Chapter)
                    .or_else(|| self.ws.project.documents.iter().find(|d| !d.kind.is_folder()))
                    .map(|d| d.id.clone());
                self.selected_research = self.ws.project.research_docs().first().map(|d| d.id.clone());
                self.selected_item = self.ws.project.items.first().map(|c| c.id.clone());
                self.collapsed.clear();
                self.history = crate::history::History::default();
                self.view = View::Write;
                self.toast("Project opened", path.display().to_string(), ToastKind::Ok);
                true
            }
            Err(e) => {
                self.toast("Could not open project", e, ToastKind::Error);
                false
            }
        }
    }
}

impl PortmanteauApp {
    /// Cmd/Ctrl+Z undoes, Shift+Cmd+Z (or Ctrl+Y) redoes. The keys are consumed
    /// here, before the editor draws, so egui's own per-widget undo (which
    /// would rewind a rebuilt text buffer and lose marks) never sees them.
    fn undo_redo_shortcuts(&mut self, ctx: &egui::Context) {
        if self.history_target().is_none() {
            return;
        }
        // Cmd+Z belongs to egui's own undo while a plain field (a title, a
        // path) has the focus; it is ours only when a rich-text field, or
        // nothing, is focused. Rich fields rebuild their buffer every frame, so
        // egui's per-widget undo would rewind it and lose the formatting.
        if let Some(focused) = ctx.memory(|memory| memory.focused()) {
            if !crate::ui::editor::is_rich_field(ctx, focused) {
                return;
            }
        }
        use egui::Modifiers;
        let (redo, undo) = ctx.input_mut(|input| {
            let redo = input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
                || input.consume_key(Modifiers::COMMAND, Key::Y);
            let undo = !redo && input.consume_key(Modifiers::COMMAND, Key::Z);
            (redo, undo)
        });
        if undo {
            self.step_history(ctx, true);
        } else if redo {
            self.step_history(ctx, false);
        }
    }

    /// What Undo and Redo act on: the research item or database item open in
    /// those workspaces, otherwise the selected document.
    fn history_target(&self) -> Option<HistoryTarget> {
        match self.view {
            View::Research => self.selected_research.clone().map(HistoryTarget::Doc),
            View::Database => self.selected_item.clone().map(HistoryTarget::Item),
            View::Settings | View::Compile | View::Agent | View::Project | View::History | View::Search => None,
            _ => self.selected.clone().map(HistoryTarget::Doc),
        }
    }

    /// Undo (or redo) one step in what is being edited and, for a document's
    /// body, put the caret where the change was.
    pub fn step_history(&mut self, ctx: &egui::Context, undo: bool) {
        let Some(target) = self.history_target() else { return };
        // Close any open burst first, so the very latest typing is undoable.
        self.observe_history(ctx);
        let applied = {
            let history = &mut self.history;
            let project = &mut self.ws.project;
            let mut step = |item: &mut dyn crate::history::Tracked| {
                if undo { history.undo(item) } else { history.redo(item) }
            };
            match &target {
                HistoryTarget::Doc(id) => project.document_mut(id).map(|doc| {
                    let applied = step(doc);
                    if applied.is_some() {
                        doc.touch();
                    }
                    applied
                }),
                HistoryTarget::Item(id) => project
                    .items
                    .iter_mut()
                    .find(|item| &item.id == id)
                    .map(|item| step(item)),
            }
            .flatten()
        };
        let Some(applied) = applied else { return };
        self.ws.dirty = true;
        if let (HistoryTarget::Doc(id), true) = (&target, applied.field == crate::history::Field::BODY) {
            // The editor keeps its caret in egui's text state under these ids.
            for key in ["prose_body", "script_flow"] {
                let edit_id = Id::unique((key, id.as_str()));
                let mut state =
                    egui::text_edit::TextEditState::load(ctx, edit_id).unwrap_or_default();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(applied.caret),
                )));
                state.store(ctx, edit_id);
            }
        }
        ctx.request_repaint();
    }

    pub fn can_undo(&self) -> bool {
        self.history_key().is_some_and(|key| self.history.can_undo(&key))
    }

    pub fn can_redo(&self) -> bool {
        self.history_key().is_some_and(|key| self.history.can_redo(&key))
    }

    fn history_key(&self) -> Option<String> {
        Some(match self.history_target()? {
            HistoryTarget::Doc(id) => format!("doc:{id}"),
            HistoryTarget::Item(id) => format!("item:{id}"),
        })
    }

    /// Show the history what is being edited, on frames where something was
    /// typed or clicked and while a burst of typing waits to close.
    fn track_history(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| !i.events.is_empty() || i.pointer.any_released());
        if input || self.history_pending {
            self.observe_history(ctx);
        }
    }

    fn observe_history(&mut self, ctx: &egui::Context) {
        let Some(target) = self.history_target() else { return };
        let project = &self.ws.project;
        let tracked: Option<&dyn crate::history::Tracked> = match &target {
            HistoryTarget::Doc(id) => project.document(id).map(|d| d as &dyn crate::history::Tracked),
            HistoryTarget::Item(id) => project
                .items
                .iter()
                .find(|item| &item.id == id)
                .map(|item| item as &dyn crate::history::Tracked),
        };
        let Some(tracked) = tracked else { return };
        match self.history.observe(tracked, Instant::now()) {
            Some(left) => {
                self.history_pending = true;
                ctx.request_repaint_after(left + Duration::from_millis(30));
            }
            None => self.history_pending = false,
        }
        self.history.retain(|key| {
            let (kind, id) = key.split_once(':').unwrap_or(("", key));
            match kind {
                "doc" => project.document(id).is_some(),
                "item" => project.items.iter().any(|item| item.id == id),
                _ => false,
            }
        });
    }
}

/// What the history is currently watching.
enum HistoryTarget {
    Doc(String),
    Item(String),
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

    /// The chrome used to carry controls that did nothing or repeated another:
    /// a rail logo, an "AM" avatar, a top-bar Compile button and snapshot
    /// controls in two inspector tabs. They are gone, and this keeps them gone.
    #[test]
    fn the_shell_has_no_dead_or_repeated_controls() {
        let (mut harness, app) = shell_harness(PortmanteauApp::new(None));
        assert!(
            harness.query_by_label("Compile  ▸").is_none(),
            "navigation to Compile is the rail and the View menu, not a top-bar button"
        );
        assert!(harness.query_by_label("AM").is_none(), "no placeholder avatar");
        assert!(
            harness.query_by_label("Session target").is_none(),
            "the daily target is shown once, in the top bar"
        );
        assert!(
            harness.query_by_label("Take snapshot").is_none(),
            "snapshots are made on the Versions tab, not the Document tab"
        );

        app.borrow_mut().inspector_tab = InspectorTab::History;
        harness.run_steps(3);
        assert!(
            harness.query_by_label("Take snapshot").is_some(),
            "the Versions tab is where a snapshot is taken"
        );
    }

    /// A harness that draws the app through `draw`, exactly as a frame of the
    /// running program does (keys, history tracking and all).
    fn app_harness(app: PortmanteauApp) -> (Harness<'static>, Rc<RefCell<PortmanteauApp>>) {
        let app = Rc::new(RefCell::new(app));
        let ui_app = Rc::clone(&app);
        let mut themed = false;
        let mut harness = Harness::builder()
            .with_size(SHELL)
            .with_max_steps(96)
            .with_accessibility_check(false)
            .allow_missing_glyphs()
            .build_ui(move |ui| {
                if !themed {
                    crate::theme::install(ui.ctx());
                    themed = true;
                    return;
                }
                ui_app.borrow_mut().draw(ui);
            });
        harness.run_steps(6);
        (harness, app)
    }

    fn press(harness: &mut Harness<'static>, modifiers: egui::Modifiers, key: Key) {
        harness.input_mut().events.push(Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        });
        harness.run_steps(3);
    }

    fn body_of(app: &Rc<RefCell<PortmanteauApp>>, doc_id: &str) -> String {
        app.borrow()
            .ws
            .project
            .document(doc_id)
            .unwrap()
            .content
            .editor_text()
    }

    /// Ctrl+F opens the find bar over the open document; typing finds matches,
    /// Enter steps through them and Replace all goes through the command layer.
    #[test]
    fn ctrl_f_finds_in_the_open_document_and_replace_all_changes_it() {
        let app = PortmanteauApp::new(None);
        let doc_id = app.selected.clone().expect("the demo opens on a chapter");
        let (mut harness, app) = app_harness(app);
        press(&mut harness, egui::Modifiers::COMMAND, Key::F);
        assert!(app.borrow().find_bar.open, "Ctrl+F opens the bar");

        harness.input_mut().events.push(Event::Text("harbour".into()));
        harness.run_steps(4);
        let total = app.borrow().find_bar.matches.len();
        assert!(total >= 2, "the first chapter mentions the harbour more than once");
        assert!(harness.query_by_label_contains(&format!("1 of {total}")).is_some());

        press(&mut harness, egui::Modifiers::NONE, Key::Enter);
        assert_eq!(app.borrow().find_bar.current, 1, "Enter goes to the next match");

        {
            let mut app = app.borrow_mut();
            app.find_bar.show_replace = true;
            app.find_bar.replacement = "port".into();
        }
        harness.run_steps(3);
        let before = body_of(&app, &doc_id);
        harness.get_by_label("Replace all").click();
        harness.run_steps(4);
        let after = body_of(&app, &doc_id);
        assert_ne!(before, after);
        assert!(!after.to_lowercase().contains("harbour"));
        assert!(app.borrow().find_bar.matches.is_empty());

        press(&mut harness, egui::Modifiers::NONE, Key::Escape);
        assert!(!app.borrow().find_bar.open, "Esc closes the bar");
    }

    /// The whole feature through the real window: typing in the prose editor,
    /// Cmd+Z, then Cmd+Shift+Z. egui's own text undo must not interfere.
    #[test]
    fn cmd_z_undoes_typing_in_the_real_window_and_cmd_shift_z_redoes_it() {
        let app = PortmanteauApp::new(None);
        let doc_id = app.selected.clone().expect("the demo opens on a chapter");
        let (mut harness, app) = app_harness(app);
        let original = body_of(&app, &doc_id);

        harness
            .get_all_by_role(Role::MultilineTextInput)
            .find(|node| in_editor_column(node.rect()) && node.value().is_some())
            .expect("the prose editor")
            .click();
        harness.run_steps(2);
        let edit_id = Id::unique(("prose_body", doc_id.as_str()));
        let mut state = egui::text_edit::TextEditState::load(&harness.ctx, edit_id).unwrap_or_default();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(0))));
        state.store(&harness.ctx, edit_id);
        harness.run_steps(1);

        harness.input_mut().events.push(Event::Text("ZZZ ".into()));
        harness.run_steps(3);
        let typed = body_of(&app, &doc_id);
        assert!(typed.starts_with("ZZZ "), "typing reached the document: {:?}", &typed[..20]);
        assert!(app.borrow().can_undo(), "the typing is undoable");

        press(&mut harness, egui::Modifiers::COMMAND, Key::Z);
        assert_eq!(body_of(&app, &doc_id), original, "Cmd+Z removes the typing");
        assert!(app.borrow().can_redo());

        press(&mut harness, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, Key::Z);
        assert_eq!(body_of(&app, &doc_id), typed, "Cmd+Shift+Z puts it back");

        // And once more down and up, to be sure nothing was left half-applied.
        press(&mut harness, egui::Modifiers::COMMAND, Key::Z);
        assert_eq!(body_of(&app, &doc_id), original);
        press(&mut harness, egui::Modifiers::COMMAND, Key::Y);
        assert_eq!(body_of(&app, &doc_id), typed, "Ctrl+Y also redoes");
    }

    /// What the writer chose (zoom, view mode, Compile format, open project) is
    /// carried into the config so the next session starts there.
    #[test]
    fn the_session_is_mirrored_into_the_config_and_survives_a_restart() {
        let mut app = PortmanteauApp::new(None);
        app.editor_zoom = 1.3;
        app.corkboard_zoom = 0.8;
        app.write_mode = WriteMode::Paged;
        app.compile_tab = CompileTab::Format(ExportKind::Epub);
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("session"));
        app.ws.save(Some(dir.clone())).expect("save");
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |_ui| {}).drop_without_applying_deltas();
        app.persist_session(&ctx);
        let settings = app.settings.clone();
        assert_eq!(settings.editor_zoom, 1.3);
        assert_eq!(settings.corkboard_zoom, 0.8);
        assert!(settings.paged_view);
        assert_eq!(settings.compile_format, "EPUB");
        assert_eq!(settings.last_project.as_deref(), Some(dir.display().to_string().as_str()));
        // The config is plain JSON: it round-trips, and an older file without
        // the new keys still loads.
        let json = serde_json::to_string(&settings).unwrap();
        assert_eq!(serde_json::from_str::<portmanteau_core::AppSettings>(&json).unwrap(), settings);
        let older: portmanteau_core::AppSettings = serde_json::from_str("{\"spellcheck\":false}").unwrap();
        assert!(older.last_project.is_none() && older.editor_zoom == 1.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn recent_projects_keep_the_newest_first_without_repeats_and_cap_at_eight() {
        let mut settings = portmanteau_core::AppSettings::default();
        for n in 0..10 {
            settings.remember_project(&format!("/p/{n}"));
        }
        settings.remember_project("/p/4");
        assert_eq!(settings.recent_projects.len(), 8);
        assert_eq!(settings.recent_projects[0], "/p/4");
        assert_eq!(settings.recent_projects.iter().filter(|p| *p == "/p/4").count(), 1);
        assert_eq!(settings.last_project.as_deref(), Some("/p/4"));
    }

    /// A saved project is saved again once the writer pauses; a project that
    /// has never been saved is left alone.
    #[test]
    fn autosave_writes_a_saved_project_after_a_pause_and_leaves_an_unsaved_one_alone() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |_ui| {}).drop_without_applying_deltas();

        let mut unsaved = PortmanteauApp::new(None);
        unsaved.ws.dirty = true;
        unsaved.last_input = Instant::now() - Duration::from_secs(30);
        unsaved.autosave(&ctx);
        assert!(unsaved.ws.dirty && unsaved.ws.path.is_none(), "no home yet, so no autosave");

        let mut app = PortmanteauApp::new(None);
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("autosave"));
        app.ws.save(Some(dir.clone())).expect("save");
        app.ws.execute("project.update", serde_json::json!({"author": "Changed Author"}));
        assert!(app.ws.dirty);
        app.last_input = Instant::now();
        app.autosave(&ctx);
        assert!(app.ws.dirty, "still typing: not yet");
        app.last_input = Instant::now() - Duration::from_secs(30);
        app.autosave(&ctx);
        assert!(!app.ws.dirty, "idle: saved");
        let reopened = Workspace::open(&dir).expect("open");
        assert_eq!(reopened.project.author, "Changed Author");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Undo and Redo follow the workspace: in Research they act on the open
    /// research item's text, in Database on the open item's notes.
    #[test]
    fn undo_redo_cover_research_and_database_text() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |_ui| {}).drop_without_applying_deltas();
        let mut app = PortmanteauApp::new(None);

        app.view = View::Research;
        let id = app.selected_research.clone().expect("the demo has research");
        let original = app.ws.project.document(&id).unwrap().content.clone();
        app.observe_history(&ctx);
        app.ws.project.document_mut(&id).unwrap().content =
            portmanteau_core::RichDocument::from_plain_text("Rewritten research note.", "body");
        app.observe_history(&ctx);
        assert!(app.can_undo());
        app.step_history(&ctx, true);
        assert_eq!(app.ws.project.document(&id).unwrap().content, original);
        assert!(app.can_redo());
        app.step_history(&ctx, false);
        assert_eq!(
            app.ws.project.document(&id).unwrap().content.plain_text(),
            "Rewritten research note."
        );

        app.view = View::Database;
        let item = app.selected_item.clone().expect("the demo has database items");
        let before = app.ws.project.items.iter().find(|i| i.id == item).unwrap().notes.clone();
        app.observe_history(&ctx);
        app.ws.project.items.iter_mut().find(|i| i.id == item).unwrap().notes =
            portmanteau_core::RichDocument::from_plain_text("New notes about the character.", "body");
        app.observe_history(&ctx);
        app.step_history(&ctx, true);
        assert_eq!(app.ws.project.items.iter().find(|i| i.id == item).unwrap().notes, before);
        // The research item's history is separate and untouched.
        app.view = View::Research;
        assert!(app.can_undo() && !app.can_redo(), "research keeps its own, separate history");
    }

    /// Plain fields (a title) keep egui's own undo; rich fields are ours.
    #[test]
    fn only_rich_fields_hand_undo_to_the_app() {
        let ctx = egui::Context::default();
        let rich = Id::unique("a-rich-field");
        let plain = Id::unique("a-plain-field");
        crate::ui::editor::mark_rich(&ctx, rich);
        assert!(crate::ui::editor::is_rich_field(&ctx, rich));
        assert!(!crate::ui::editor::is_rich_field(&ctx, plain));
    }

    /// Following a link: a research note opens in Research, a manuscript
    /// document in Write, and only web addresses ever reach the
    /// system's opener.
    #[test]
    fn links_open_the_right_place_and_only_safe_addresses_leave_the_app() {
        let mut app = PortmanteauApp::new(None);
        let note = app.ws.project.research_docs()[0].id.clone();
        app.open_link(&format!("note:{note}"));
        assert_eq!((app.view, app.selected_research.as_deref()), (View::Research, Some(note.as_str())));
        let chapter = app
            .ws
            .project
            .documents
            .iter()
            .find(|d| d.kind == DocKind::Chapter)
            .unwrap()
            .id
            .clone();
        app.open_link(&format!("note:{chapter}"));
        assert_eq!((app.view, app.selected.as_deref()), (View::Write, Some(chapter.as_str())));
        app.open_link("note:no-such-document");
        assert!(app.toast.as_ref().is_some_and(|t| t.message.contains("no longer exists")));

        assert_eq!(safe_external_url("https://example.org/a?b=1"), Some("https://example.org/a?b=1"));
        assert_eq!(safe_external_url("https://user@host.test:8080/p?q=1,2#f"), Some("https://user@host.test:8080/p?q=1,2#f"));
        for hostile in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ssh://host",
            "https://x.test/a b",
            "https://x.test/\nrm -rf",
            "-oProxyCommand=evil",
            "https://",
            "https:///path-without-host",
            "https://-evil.test",
            "https://x.test\\evil",
            "https://x.test/\"q",
            "mailto:a@b.test",
            "MAILTO:a@b.test?subject=Hello",
            "mailto:a@b.test?attach=/etc/passwd",
            "",
        ] {
            assert_eq!(safe_external_url(hostile), None, "{hostile:?}");
        }
    }

    /// A picture from the clipboard (or a dropped file) is stored in the project's
    /// assets and put into the text where the caret is, on its own paragraph.
    #[test]
    fn pasted_pixels_are_stored_in_the_project_and_inserted_at_the_caret() {
        let mut app = PortmanteauApp::new(None);
        let doc_id = app.selected.clone().unwrap();
        let target = RichTextTarget::DocumentBody(doc_id.clone());
        let rgba = vec![200u8; 4 * 6 * 4];

        // An unsaved project has nowhere to keep it: say so, change nothing.
        let before = app.ws.project.document(&doc_id).unwrap().content.clone();
        let err = app.store_image_pixels(6, 4, rgba.clone(), true).unwrap_err();
        assert!(err.contains("Save the project"), "{err}");
        assert_eq!(app.ws.project.document(&doc_id).unwrap().content, before);

        let dir = std::env::temp_dir().join(portmanteau_core::new_id("paste"));
        app.ws.save(Some(dir.clone())).unwrap();
        let caret = 12;
        let object = app.store_image_pixels(6, 4, rgba, true).unwrap();
        assert!(dir.join("assets").join(&object.asset_id).is_file(), "the PNG is in the project");
        assert_eq!(object.width.map(|w| w.round()), Some(240.0), "shown at most 240 points wide as a figure");
        app.insert_image_object(&target, caret..caret, object.clone(), true).unwrap();
        let content = &app.ws.project.document(&doc_id).unwrap().content;
        let images: Vec<_> = content
            .paragraphs
            .iter()
            .flat_map(|p| p.content.iter())
            .filter(|item| matches!(item, portmanteau_core::InlineContent::Image(_)))
            .collect();
        assert_eq!(images.len(), 1);
        assert!(content.paragraphs.iter().any(|p| p.style_id == "image"), "on its own paragraph");
        assert!(app.ws.dirty);
        // The text either side of the caret is untouched.
        assert!(content.plain_text().starts_with(&before.plain_text()[..caret]));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Where a paste lands: the focused note body or document body, never a script.
    #[test]
    fn pictures_target_the_document_or_note_being_written_in_but_not_scripts() {
        let ctx = egui::Context::default();
        let mut app = PortmanteauApp::new(None);
        app.view = View::Research;
        let note = app.selected_research.clone().unwrap();
        let target = app.image_target(&ctx).expect("a research note takes pictures");
        assert!(matches!(&target.0, RichTextTarget::DocumentBody(id) if *id == note));
        app.view = View::Write;
        assert!(app.image_target(&ctx).is_some());
        app.selected = app.ws.project.documents.iter().find(|d| d.medium.is_script()).map(|d| d.id.clone());
        assert!(app.image_target(&ctx).is_none(), "scripts take no pictures");
        app.view = View::Compile;
        assert!(app.image_target(&ctx).is_none());
    }
}
