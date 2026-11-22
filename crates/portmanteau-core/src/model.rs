//! Portmanteau domain model: projects, binder, documents, script elements,
//! research, story development, versions and export settings.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{ParagraphStyle, RichDocument, StyleSheet};

pub use crate::script::ElementKind;
pub use crate::text::{count_words, reading_minutes, truncate_words};

pub fn new_id(prefix: &str) -> String {
    format!("{}-{}", prefix, &Uuid::new_v4().simple().to_string()[..8])
}

fn default_ui_scale() -> f32 {
    1.0
}

fn default_prose_stylesheet() -> String {
    "prose".into()
}

fn default_style_sheets() -> Vec<StyleSheet> {
    StyleSheet::industry_defaults(&ElementStyles::default())
}

fn screenplay_export_defaults(mut settings: ExportSettings) -> ExportSettings {
    settings.paper = PaperSize::Letter;
    settings.header_footer.number_location = HfLocation::Header;
    settings.header_footer.number_align = Alignment::Right;
    settings.header_footer.number_spacing = 36.0;
    settings
}

pub fn now_stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

// ---------------------------------------------------------------------------
// Enumerations
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProjectKind {
    Novel,
    Film,
    Television,
    Theatre,
    Mixed,
}

impl ProjectKind {
    pub fn label(self) -> &'static str {
        match self {
            ProjectKind::Novel => "Novel",
            ProjectKind::Film => "Film",
            ProjectKind::Television => "Television",
            ProjectKind::Theatre => "Theatre",
            ProjectKind::Mixed => "Mixed media",
        }
    }
    pub fn all() -> [ProjectKind; 5] {
        [
            ProjectKind::Novel,
            ProjectKind::Film,
            ProjectKind::Television,
            ProjectKind::Theatre,
            ProjectKind::Mixed,
        ]
    }
    pub fn default_medium(self) -> Medium {
        match self {
            ProjectKind::Novel => Medium::Prose,
            ProjectKind::Film => Medium::Screenplay,
            ProjectKind::Television => Medium::TvScript,
            ProjectKind::Theatre => Medium::StagePlay,
            ProjectKind::Mixed => Medium::Prose,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Medium {
    Prose,
    Screenplay,
    StagePlay,
    TvScript,
    Note,
}

impl Medium {
    pub fn label(self) -> &'static str {
        match self {
            Medium::Prose => "Prose",
            Medium::Screenplay => "Screenplay",
            Medium::StagePlay => "Stage play",
            Medium::TvScript => "TV script",
            Medium::Note => "Note",
        }
    }
    pub fn is_script(self) -> bool {
        matches!(
            self,
            Medium::Screenplay | Medium::StagePlay | Medium::TvScript
        )
    }
    pub fn all() -> [Medium; 5] {
        [
            Medium::Prose,
            Medium::Screenplay,
            Medium::StagePlay,
            Medium::TvScript,
            Medium::Note,
        ]
    }
    /// The id of the built-in stylesheet a document of this medium starts with.
    pub fn default_stylesheet_id(self) -> &'static str {
        match self {
            Medium::Screenplay => "film-screenplay",
            Medium::TvScript => "tv-script",
            Medium::StagePlay => "stage-play",
            Medium::Prose | Medium::Note => "prose",
        }
    }

    pub fn default_element(self) -> ElementKind {
        match self {
            Medium::StagePlay => ElementKind::StageDirection,
            Medium::Screenplay | Medium::TvScript => ElementKind::Action,
            _ => ElementKind::General,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DocKind {
    Folder,
    Chapter,
    Scene,
    Text,
    Research,
    Character,
    FrontMatter,
    BackMatter,
}

impl DocKind {
    pub fn label(self) -> &'static str {
        match self {
            DocKind::Folder => "Folder",
            DocKind::Chapter => "Chapter",
            DocKind::Scene => "Scene",
            DocKind::Text => "Text",
            DocKind::Research => "Research",
            DocKind::Character => "Character",
            DocKind::FrontMatter => "Front matter",
            DocKind::BackMatter => "Back matter",
        }
    }
    pub fn is_folder(self) -> bool {
        matches!(self, DocKind::Folder)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Block {
    pub kind: ElementKind,
    pub text: String,
}

impl Block {
    pub fn new(kind: ElementKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Document {
    pub id: String,
    pub parent: Option<String>,
    pub order: i32,
    pub title: String,
    pub kind: DocKind,
    pub medium: Medium,
    /// Project stylesheet selected for this document. Medium chooses an initial
    /// default, but the content representation is shared across all media.
    #[serde(default = "default_prose_stylesheet")]
    pub style_sheet_id: String,
    pub content: RichDocument,
    pub synopsis: RichDocument,
    pub notes: RichDocument,
    pub label: String,
    pub status: String,
    pub include_in_export: bool,
    pub word_target: u32,
    pub created: String,
    pub updated: String,
    pub snapshots: Vec<Snapshot>,
    pub pinned: bool,
    /// Present on research notes (documents in the Research folder).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub research: Option<ResearchMeta>,
}

impl Document {
    pub fn new(
        parent: Option<String>,
        order: i32,
        title: impl Into<String>,
        kind: DocKind,
        medium: Medium,
    ) -> Self {
        let stamp = now_stamp();
        Self {
            id: new_id("doc"),
            parent,
            order,
            title: title.into(),
            kind,
            medium,
            style_sheet_id: medium.default_stylesheet_id().into(),
            content: RichDocument::empty(if medium.is_script() {
                "action"
            } else {
                "body"
            }),
            synopsis: RichDocument::empty("body"),
            notes: RichDocument::empty("body"),
            label: String::new(),
            status: "To do".into(),
            include_in_export: true,
            word_target: 0,
            created: stamp.clone(),
            updated: stamp,
            snapshots: Vec::new(),
            pinned: false,
            research: None,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.set_plain_text(text);
        self
    }

    pub fn with_script(mut self, blocks: Vec<Block>) -> Self {
        self.set_script_blocks(blocks);
        self
    }

    pub fn set_plain_text(&mut self, text: impl Into<String>) {
        let default_style = if self.medium.is_script() {
            "action"
        } else {
            "body"
        };
        let text = text.into();
        self.content = RichDocument::from_plain_text(&text, default_style);
    }

    pub fn set_script_blocks(&mut self, blocks: Vec<Block>) {
        self.content = RichDocument::from_script_blocks(&blocks);
    }

    pub fn with_synopsis(mut self, synopsis: impl Into<String>) -> Self {
        self.set_synopsis_text(synopsis);
        self
    }

    pub fn set_synopsis_text(&mut self, synopsis: impl Into<String>) {
        self.synopsis = RichDocument::from_plain_text(&synopsis.into(), "body");
    }

    pub fn set_notes_text(&mut self, notes: impl Into<String>) {
        self.notes = RichDocument::from_plain_text(&notes.into(), "body");
    }

    pub fn with_meta(
        mut self,
        label: impl Into<String>,
        status: impl Into<String>,
        target: u32,
    ) -> Self {
        self.label = label.into();
        self.status = status.into();
        self.word_target = target;
        self
    }

    /// Authoritative body text, whatever the medium.
    pub fn body_text(&self) -> String {
        self.content.plain_text()
    }

    pub fn words(&self) -> u32 {
        count_words(&self.content.text_only())
    }

    pub fn touch(&mut self) {
        self.updated = now_stamp();
    }

    /// This note's research details, created on first use.
    pub fn research_meta(&mut self) -> &mut ResearchMeta {
        self.research.get_or_insert_with(ResearchMeta::default)
    }

    pub fn snapshot(&mut self, name: impl Into<String>, note: impl Into<String>) {
        let snap = Snapshot {
            id: new_id("snap"),
            name: name.into(),
            created: now_stamp(),
            note: note.into(),
            content: self.content.clone(),
            words: self.words(),
        };
        self.snapshots.insert(0, snap);
        if self.snapshots.len() > 60 {
            self.snapshots.truncate(60);
        }
    }

    pub fn restore_snapshot(&mut self, snapshot_id: &str) -> bool {
        if let Some(snap) = self.snapshots.iter().find(|s| s.id == snapshot_id).cloned() {
            self.content = snap.content;
            self.touch();
            true
        } else {
            false
        }
    }

    /// Turn a prose document into blocks, or blocks back into prose.
    pub fn convert_medium(&mut self, medium: Medium) {
        if medium == self.medium {
            return;
        }
        let body = self.body_text();
        self.medium = medium;
        self.style_sheet_id = medium.default_stylesheet_id().into();
        if medium.is_script() {
            let blocks = crate::script::parse_loose(&body);
            self.content = RichDocument::from_script_blocks(&blocks);
        } else {
            self.content = RichDocument::from_plain_text(&body, "body");
        }
        self.touch();
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Snapshot {
    pub id: String,
    pub name: String,
    pub created: String,
    pub note: String,
    pub content: RichDocument,
    pub words: u32,
}

impl Snapshot {
    /// Reconstruct the saved body for comparison with the current document.
    /// A script snapshot stores typed blocks; a prose snapshot stores text.
    pub fn body_text(&self) -> String {
        self.content.plain_text()
    }
}

/// The kind of record held in the story database.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemKind {
    Character,
    Place,
    Event,
    Prop,
    Custom,
}

impl ItemKind {
    pub fn label(self) -> &'static str {
        match self {
            ItemKind::Character => "Character",
            ItemKind::Place => "Place",
            ItemKind::Event => "Event",
            ItemKind::Prop => "Prop",
            ItemKind::Custom => "Custom",
        }
    }

    pub fn all() -> [ItemKind; 5] {
        [
            ItemKind::Character,
            ItemKind::Place,
            ItemKind::Event,
            ItemKind::Prop,
            ItemKind::Custom,
        ]
    }
}

/// A story-database record: a character, place, event, prop or custom item.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DatabaseItem {
    pub id: String,
    pub kind: ItemKind,
    pub title: String,
    pub notes: RichDocument,
    pub tags: Vec<String>,
    pub color: u8,
}

impl DatabaseItem {
    pub fn new(kind: ItemKind, title: impl Into<String>) -> Self {
        Self {
            id: new_id("item"),
            kind,
            title: title.into(),
            notes: RichDocument::empty("body"),
            tags: Vec::new(),
            color: 0,
        }
    }
}

/// One place a database item is named or linked in the writing.
#[derive(Clone, Debug, PartialEq)]
pub struct Appearance {
    pub doc_id: String,
    pub title: String,
    /// The first place in the document's text: its paragraph and the character
    /// offset in it. `None` when the item is only named in the synopsis or notes.
    pub place: Option<(usize, usize)>,
    /// The words around it.
    pub excerpt: String,
    /// Found by an `@` link rather than by its name.
    pub by_link: bool,
    pub in_research: bool,
}

/// The names to look for: the whole title, and a character's first name.
fn item_names(item: &DatabaseItem) -> Vec<Vec<char>> {
    let title = item.title.trim();
    let mut names = Vec::new();
    if !title.is_empty() {
        names.push(title.to_lowercase().chars().collect());
        if item.kind == ItemKind::Character {
            if let Some(first) = title.split_whitespace().next().filter(|w| w.chars().count() >= 3 && w.len() < title.len()) {
                names.push(first.to_lowercase().chars().collect());
            }
        }
    }
    names
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '’'
}

/// The first whole-word, case-insensitive occurrence of any name, as a char offset.
fn find_name(text: &[char], names: &[Vec<char>]) -> Option<usize> {
    let lower: Vec<char> = text.iter().flat_map(|c| c.to_lowercase()).collect();
    if lower.len() != text.len() {
        return None; // case mapping changed the length; offsets would be wrong
    }
    let mut best: Option<usize> = None;
    for name in names.iter().filter(|n| !n.is_empty()) {
        let mut i = 0;
        while i + name.len() <= lower.len() {
            if lower[i..i + name.len()] == name[..]
                && (i == 0 || !is_word_char(lower[i - 1]))
                && (i + name.len() == lower.len() || !is_word_char(lower[i + name.len()]))
            {
                best = Some(best.map_or(i, |b| b.min(i)));
                break;
            }
            i += 1;
        }
    }
    best
}

fn excerpt_around(text: &[char], at: usize) -> String {
    let start = at.saturating_sub(28);
    let end = (at + 48).min(text.len());
    let body: String = text[start..end].iter().collect::<String>().replace('\n', " ");
    format!("{}{}{}", if start > 0 { "…" } else { "" }, body.trim(), if end < text.len() { "…" } else { "" })
}

/// What makes a binder document a research note: a category, where it came
/// from, and the files kept with it. The note's text is the document's own.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ResearchMeta {
    /// "Note", "Interview", "Data", "Archive", "Image" or "Place" (see
    /// `RESEARCH_KINDS`): used to filter, colour and label notes.
    pub kind: String,
    /// Where it came from: a reference, a person and date, a place.
    #[serde(default)]
    pub source: String,
    /// Attachment basenames. The bytes live beside the project in
    /// `assets/research/<document-id>/<basename>`; the model only remembers names.
    #[serde(default)]
    pub attachments: Vec<String>,
}

impl Default for ResearchMeta {
    fn default() -> Self {
        Self { kind: "Note".into(), source: String::new(), attachments: Vec::new() }
    }
}

/// The categories a research note can have, in the order they are offered.
pub const RESEARCH_KINDS: [&str; 6] = ["Note", "Interview", "Data", "Archive", "Image", "Place"];

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SessionEntry {
    pub date: String,
    pub words: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Checkpoint {
    pub id: String,
    pub label: String,
    pub created: String,
    pub kind: String,
    pub words: u32,
    pub payload: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ExportFormat;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportKind {
    Pdf,
    Epub,
    Docx,
    Fdx,
    Fountain,
    Html,
    Markdown,
    Rtf,
    Txt,
    Bundle,
}

impl ExportKind {
    /// Whether this format can carry a link in the text. (PDF would need link
    /// annotations; plain text, Fountain and Final Draft have no links.)
    pub fn writes_hyperlinks(self) -> bool {
        matches!(
            self,
            ExportKind::Html | ExportKind::Epub | ExportKind::Markdown | ExportKind::Docx | ExportKind::Rtf
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            ExportKind::Pdf => "PDF",
            ExportKind::Epub => "EPUB",
            ExportKind::Docx => "Word (.docx)",
            ExportKind::Fdx => "Final Draft (.fdx)",
            ExportKind::Fountain => "Fountain",
            ExportKind::Html => "HTML",
            ExportKind::Markdown => "Markdown",
            ExportKind::Rtf => "Rich Text (.rtf)",
            ExportKind::Txt => "Plain text",
            ExportKind::Bundle => "Project bundle",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            ExportKind::Docx => "DOCX",
            ExportKind::Fdx => "FDX",
            ExportKind::Markdown => "MD",
            ExportKind::Txt => "TXT",
            ExportKind::Html => "HTML",
            ExportKind::Bundle => "ZIP",
            _ => self.label(),
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            ExportKind::Pdf => "pdf",
            ExportKind::Epub => "epub",
            ExportKind::Docx => "docx",
            ExportKind::Fdx => "fdx",
            ExportKind::Fountain => "fountain",
            ExportKind::Html => "html",
            ExportKind::Markdown => "md",
            ExportKind::Rtf => "rtf",
            ExportKind::Txt => "txt",
            ExportKind::Bundle => "zip",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            ExportKind::Pdf => "Print-ready manuscript or screenplay",
            ExportKind::Epub => "Reflowable ebook for readers and stores",
            ExportKind::Docx => "Editable manuscript for editors",
            ExportKind::Fdx => "Industry-standard screenplay exchange",
            ExportKind::Fountain => "Plain-text screenplay with formatting",
            ExportKind::Html => "Single web page with clean CSS",
            ExportKind::Markdown => "Portable plain text with structure",
            ExportKind::Rtf => "Rich text for any word processor",
            ExportKind::Txt => "UTF-8 plain text, unformatted",
            ExportKind::Bundle => "Project, snapshots and notes in one archive",
        }
    }
    pub fn all() -> [ExportKind; 10] {
        [
            ExportKind::Pdf,
            ExportKind::Epub,
            ExportKind::Docx,
            ExportKind::Fdx,
            ExportKind::Fountain,
            ExportKind::Html,
            ExportKind::Markdown,
            ExportKind::Rtf,
            ExportKind::Txt,
            ExportKind::Bundle,
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaperSize {
    A4,
    Letter,
    A5,
    UsTrade,
}

impl PaperSize {
    pub fn label(self) -> &'static str {
        match self {
            PaperSize::A4 => "A4",
            PaperSize::Letter => "US Letter",
            PaperSize::A5 => "A5",
            PaperSize::UsTrade => "Trade paperback",
        }
    }
    pub fn mm(self) -> (f32, f32) {
        match self {
            PaperSize::A4 => (210.0, 297.0),
            PaperSize::Letter => (215.9, 279.4),
            PaperSize::A5 => (148.0, 210.0),
            PaperSize::UsTrade => (152.4, 228.6),
        }
    }
    pub fn all() -> [PaperSize; 4] {
        [
            PaperSize::A4,
            PaperSize::Letter,
            PaperSize::A5,
            PaperSize::UsTrade,
        ]
    }
}

/// Horizontal alignment for a paragraph, modelled after a word processor's
/// paragraph dialog. Serialises as `"Left"`, `"Center"`, `"Right"` or
/// `"Justify"`.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

impl Alignment {
    pub fn label(self) -> &'static str {
        match self {
            Alignment::Left => "Left",
            Alignment::Center => "Center",
            Alignment::Right => "Right",
            Alignment::Justify => "Justify",
        }
    }

    pub fn all() -> [Alignment; 4] {
        [
            Alignment::Left,
            Alignment::Center,
            Alignment::Right,
            Alignment::Justify,
        ]
    }
}

impl Default for Alignment {
    fn default() -> Self {
        Alignment::Left
    }
}

/// Type for one kind of manuscript element. Every output format starts from
/// the project's element styles when it has no saved settings of its own.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ElementStyle {
    pub font: String,     // family name ("" means inherit/unset)
    pub size: f32,        // points
    pub leading: f32,     // line-height multiple
    pub indent: f32,      // left indent, points
    #[serde(default)]
    pub first_line_indent: f32, // first-line indent, points
    #[serde(default)]
    pub right_indent: f32, // right indent, points
    pub before: f32,      // space before, points
    pub after: f32,       // space after, points
    #[serde(default)]
    pub align: Alignment,
    /// Render this element in all caps. Conventional for character cues,
    /// scene headings, transitions and act markers; off for action, dialogue
    /// and prose.
    #[serde(default)]
    pub uppercase: bool,
    /// Render this element in italics (Samuel French stage directions).
    #[serde(default)]
    pub italic: bool,
}

impl Default for ElementStyle {
    fn default() -> Self {
        Self {
            font: String::new(),
            size: 11.0,
            leading: 1.4,
            indent: 0.0,
            first_line_indent: 0.0,
            right_indent: 0.0,
            before: 0.0,
            after: 0.0,
            align: Alignment::Left,
            uppercase: false,
            italic: false,
        }
    }
}

fn centred(style: ElementStyle) -> ElementStyle {
    ElementStyle {
        align: Alignment::Center,
        ..style
    }
}

fn base_element(font: &str, size: f32, leading: f32) -> ElementStyle {
    ElementStyle {
        font: font.into(),
        size,
        leading,
        ..ElementStyle::default()
    }
}

/// A writer-defined element type stored beside the built-in ones. The name is
/// shown in the style editor and the preview picker; the style is the same
/// structure every built-in element uses.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CustomStyle {
    pub name: String,
    pub style: ElementStyle,
}

/// The project-wide base type for every kind of manuscript element. Formats
/// that have no saved settings of their own are seeded from `body` (prose) or
/// `action` (script).
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ElementStyles {
    pub title_page_title: ElementStyle,
    pub title_page_subtitle: ElementStyle,
    pub title_page_author: ElementStyle,
    pub chapter_heading: ElementStyle,
    pub body: ElementStyle,
    pub scene_heading: ElementStyle,
    pub action: ElementStyle,
    pub character: ElementStyle,
    pub parenthetical: ElementStyle,
    pub dialogue: ElementStyle,
    pub transition: ElementStyle,
    pub act: ElementStyle,
    pub scene: ElementStyle,
    pub stage_direction: ElementStyle,
    pub general: ElementStyle,
    /// Common U.S. screenplay paragraph types beyond the core dialogue trio.
    #[serde(default)]
    pub extension: ElementStyle,
    #[serde(default)]
    pub intercut: ElementStyle,
    #[serde(default)]
    pub subheading: ElementStyle,
    /// Writer-defined styles ("Epigraph", "Pull quote", …). Stored with the
    /// built-ins so they persist in global settings, the project and each
    /// format's settings through serde.
    #[serde(default)]
    pub custom: Vec<CustomStyle>,
}

impl Default for ElementStyles {
    fn default() -> Self {
        let serif = "Liberation Serif";
        let mono = "Courier New";
        Self {
            // A title block is centred in every format; the writer can change
            // it per format in Compile.
            title_page_title: ElementStyle {
                after: 18.0,
                ..centred(base_element(serif, 24.0, 1.2))
            },
            title_page_subtitle: ElementStyle {
                after: 24.0,
                ..centred(base_element(serif, 14.0, 1.3))
            },
            title_page_author: centred(base_element(serif, 12.0, 1.4)),
            chapter_heading: ElementStyle {
                before: 12.0,
                after: 8.0,
                ..base_element(serif, 16.0, 1.2)
            },
            body: ElementStyle {
                first_line_indent: 14.0,
                ..base_element(serif, 11.0, 1.4)
            },
            // US Letter, 12 pt Courier, 6 lines per inch, 1.5" left / 1" right
            // margins. Indents are measured from the left text margin (1.5"),
            // so the character cue sits at 3.7" and dialogue at 2.5".
            scene_heading: ElementStyle {
                before: 24.0,
                uppercase: true,
                ..base_element(mono, 12.0, 1.0)
            },
            action: ElementStyle {
                before: 12.0,
                ..base_element(mono, 12.0, 1.0)
            },
            character: ElementStyle {
                indent: 158.4,
                before: 12.0,
                uppercase: true,
                ..base_element(mono, 12.0, 1.0)
            },
            parenthetical: ElementStyle {
                indent: 115.2,
                right_indent: 136.8,
                ..base_element(mono, 12.0, 1.0)
            },
            dialogue: ElementStyle {
                indent: 72.0,
                right_indent: 108.0,
                ..base_element(mono, 12.0, 1.0)
            },
            transition: ElementStyle {
                before: 12.0,
                uppercase: true,
                align: Alignment::Right,
                ..base_element(mono, 12.0, 1.0)
            },
            act: ElementStyle {
                before: 12.0,
                uppercase: true,
                ..base_element(mono, 12.0, 1.0)
            },
            scene: base_element(mono, 12.0, 1.0),
            stage_direction: base_element(mono, 12.0, 1.0),
            general: base_element(mono, 12.0, 1.0),
            extension: ElementStyle {
                indent: 158.4,
                uppercase: true,
                ..base_element(mono, 12.0, 1.0)
            },
            intercut: ElementStyle {
                before: 12.0,
                uppercase: true,
                ..base_element(mono, 12.0, 1.0)
            },
            subheading: ElementStyle {
                before: 12.0,
                uppercase: true,
                ..base_element(mono, 12.0, 1.0)
            },
            custom: Vec::new(),
        }
    }
}

impl ElementStyles {
    /// Look up a built-in element by its JSON/model name.
    pub fn element(&self, name: &str) -> Option<&ElementStyle> {
        Some(match name {
            "title_page_title" => &self.title_page_title,
            "title_page_subtitle" => &self.title_page_subtitle,
            "title_page_author" => &self.title_page_author,
            "chapter_heading" => &self.chapter_heading,
            "body" => &self.body,
            "scene_heading" => &self.scene_heading,
            "action" => &self.action,
            "character" => &self.character,
            "parenthetical" => &self.parenthetical,
            "dialogue" => &self.dialogue,
            "transition" => &self.transition,
            "act" => &self.act,
            "scene" => &self.scene,
            "stage_direction" => &self.stage_direction,
            "general" => &self.general,
            "extension" => &self.extension,
            "intercut" => &self.intercut,
            "subheading" => &self.subheading,
            _ => return None,
        })
    }

    /// Look up a writer-defined style by its display name.
    pub fn custom_named(&self, name: &str) -> Option<&CustomStyle> {
        self.custom.iter().find(|style| style.name == name)
    }

    /// The style backing one screenplay element kind. This is the single
    /// mapping the editor, preview and every export writer share.
    pub fn style_for_kind(&self, kind: ElementKind) -> &ElementStyle {
        match kind {
            ElementKind::SceneHeading => &self.scene_heading,
            ElementKind::Action | ElementKind::Shot => &self.action,
            ElementKind::Character => &self.character,
            ElementKind::Parenthetical => &self.parenthetical,
            ElementKind::Dialogue | ElementKind::Lyric => &self.dialogue,
            ElementKind::Transition => &self.transition,
            ElementKind::Act => &self.act,
            ElementKind::Scene => &self.scene,
            ElementKind::StageDirection => &self.stage_direction,
            ElementKind::General => &self.general,
            ElementKind::Extension => &self.extension,
            ElementKind::Intercut => &self.intercut,
            ElementKind::Subheading => &self.subheading,
        }
    }

    pub fn style_for_kind_mut(&mut self, kind: ElementKind) -> &mut ElementStyle {
        match kind {
            ElementKind::SceneHeading => &mut self.scene_heading,
            ElementKind::Action | ElementKind::Shot => &mut self.action,
            ElementKind::Character => &mut self.character,
            ElementKind::Parenthetical => &mut self.parenthetical,
            ElementKind::Dialogue | ElementKind::Lyric => &mut self.dialogue,
            ElementKind::Transition => &mut self.transition,
            ElementKind::Act => &mut self.act,
            ElementKind::Scene => &mut self.scene,
            ElementKind::StageDirection => &mut self.stage_direction,
            ElementKind::General => &mut self.general,
            ElementKind::Extension => &mut self.extension,
            ElementKind::Intercut => &mut self.intercut,
            ElementKind::Subheading => &mut self.subheading,
        }
    }

    /// Project/export styles inherit the visible formatting from the active
    /// writing sheets, so preview and exported pages agree with the editor.
    pub fn update_from_sheets(&mut self, sheets: &[StyleSheet]) {
        for sheet in sheets {
            if StyleSheet::is_independent_script_sheet(&sheet.id) {
                continue;
            }
            for style in &sheet.styles {
                if let Some(kind) = style.semantic_kind {
                    *self.style_for_kind_mut(kind) = style.format.clone();
                } else if sheet.id == "prose" {
                    match style.id.as_str() {
                        "body" => self.body = style.format.clone(),
                        "chapter-heading" => self.chapter_heading = style.format.clone(),
                        _ => {}
                    }
                } else if !matches!(style.id.as_str(), "image" | "image-caption") {
                    if let Some(custom) = self.custom.iter_mut().find(|custom| custom.name == style.name) {
                        custom.style = style.format.clone();
                    } else if !self.custom.iter().any(|custom| custom.name == style.name) {
                        self.custom.push(CustomStyle {
                            name: style.name.clone(),
                            style: style.format.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Look up one element by its JSON/model name.
    pub fn element_mut(&mut self, name: &str) -> Option<&mut ElementStyle> {
        Some(match name {
            "title_page_title" => &mut self.title_page_title,
            "title_page_subtitle" => &mut self.title_page_subtitle,
            "title_page_author" => &mut self.title_page_author,
            "chapter_heading" => &mut self.chapter_heading,
            "body" => &mut self.body,
            "scene_heading" => &mut self.scene_heading,
            "action" => &mut self.action,
            "character" => &mut self.character,
            "parenthetical" => &mut self.parenthetical,
            "dialogue" => &mut self.dialogue,
            "transition" => &mut self.transition,
            "act" => &mut self.act,
            "scene" => &mut self.scene,
            "stage_direction" => &mut self.stage_direction,
            "general" => &mut self.general,
            "extension" => &mut self.extension,
            "intercut" => &mut self.intercut,
            "subheading" => &mut self.subheading,
            _ => return None,
        })
    }
}

/// Global defaults that live outside any one project. The Display tab of the
/// Settings screen edits these and can copy them into the open project. The
/// editors themselves draw from the default element styles, so there is no
/// separate on-screen size preference.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AppSettings {
    #[serde(default)]
    pub styles: ElementStyles,
    /// Program-wide stylesheet templates copied into projects when they are
    /// created. Existing project sheets remain independent.
    #[serde(default = "default_style_sheets")]
    pub style_sheets: Vec<StyleSheet>,
    /// Show the spelling panel and check words in the editors.
    #[serde(default = "default_true")]
    pub spellcheck: bool,
    /// The writer's personal dictionary, shared by every project.
    #[serde(default)]
    pub custom_words: Vec<String>,
    /// Spelling dictionary code, matching `spellcheck::LANGUAGES`.
    #[serde(default = "default_language")]
    pub language: String,
    /// Whether the binder panel is visible in the Write workspace. Remembered
    /// across launches so the writer's layout survives a restart.
    #[serde(default = "default_true")]
    pub show_binder: bool,
    /// Whether the inspector panel is visible in the Write workspace.
    #[serde(default = "default_true")]
    pub show_inspector: bool,
    /// Interface text scale (1.0 = default). Affects chrome, not manuscripts.
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    /// The project that was open when the app last closed, reopened at launch.
    #[serde(default)]
    pub last_project: Option<String>,
    /// Recently opened projects, newest first.
    #[serde(default)]
    pub recent_projects: Vec<String>,
    /// Text zoom of the writing surface.
    #[serde(default = "default_ui_scale")]
    pub editor_zoom: f32,
    /// Corkboard card scale.
    #[serde(default = "default_ui_scale")]
    pub corkboard_zoom: f32,
    /// Write opens in the read-only Paged view rather than Continuous.
    #[serde(default)]
    pub paged_view: bool,
    /// The output format the Compile screen last showed ("PDF", "EPUB", …).
    #[serde(default)]
    pub compile_format: String,
    /// The window's inner size in points, restored at launch.
    #[serde(default)]
    pub window_size: Option<[f32; 2]>,
    /// What the writer chose last time in the import review.
    #[serde(default)]
    pub import: ImportPrefs,
}

/// Import choices remembered for next time.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ImportPrefs {
    /// Cut a long file into several documents at its headings.
    #[serde(default = "default_true")]
    pub split: bool,
    /// Offer to put the file's title page details into the project.
    #[serde(default = "default_true")]
    pub apply_details: bool,
    /// Paragraph-style mappings, by import format ("docx", "fdx"): the source
    /// style name to a target token (`heading:1`, `style:body`, `skip`, …).
    /// Applied automatically the next time a file of that format is reviewed.
    #[serde(default)]
    pub style_maps: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    /// The folder the file browser last used.
    #[serde(default)]
    pub folder: Option<String>,
}

impl Default for ImportPrefs {
    fn default() -> Self {
        Self { split: true, apply_details: true, style_maps: Default::default(), folder: None }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        let styles = ElementStyles::default();
        Self {
            styles: styles.clone(),
            style_sheets: StyleSheet::industry_defaults(&styles),
            spellcheck: true,
            custom_words: Vec::new(),
            language: default_language(),
            show_binder: true,
            show_inspector: true,
            ui_scale: default_ui_scale(),
            last_project: None,
            recent_projects: Vec::new(),
            editor_zoom: default_ui_scale(),
            corkboard_zoom: default_ui_scale(),
            paged_view: false,
            compile_format: String::new(),
            window_size: None,
            import: ImportPrefs::default(),
        }
    }
}

impl AppSettings {
    /// Note that a project was opened or saved: it becomes the one to reopen
    /// at launch and moves to the front of the recent list (at most eight).
    pub fn remember_project(&mut self, path: &str) {
        self.last_project = Some(path.to_string());
        self.recent_projects.retain(|existing| existing != path);
        self.recent_projects.insert(0, path.to_string());
        self.recent_projects.truncate(8);
    }

    /// Add any built-in stylesheet that a config saved by an older build lacks.
    pub fn ensure_style_sheets(&mut self) {
        StyleSheet::ensure_industry_defaults(&mut self.style_sheets, &self.styles);
    }

    /// Build the style sheets a newly created project receives. The existing
    /// program-wide style sheets are copied into the project; project edits
    /// never write back to this template.
    pub fn style_sheets_for_new_project(&self) -> Vec<StyleSheet> {
        if self.style_sheets.is_empty() {
            default_style_sheets()
        } else {
            self.style_sheets.clone()
        }
    }

    pub fn element_styles_for_new_project(&self) -> ElementStyles {
        let mut styles = self.styles.clone();
        styles.update_from_sheets(&self.style_sheets_for_new_project());
        styles
    }

    /// Keep legacy export/base-style controls reflected in the built-in
    /// stylesheet entries while preserving each style's name and next-style
    /// rule.
    pub fn sync_sheet_formats_from_element_styles(&mut self) {
        for sheet in &mut self.style_sheets {
            // The TV and stage sheets are separate formats with their own
            // look; the shared element styles only describe film.
            if StyleSheet::is_independent_script_sheet(&sheet.id) {
                continue;
            }
            for style in &mut sheet.styles {
                if let Some(kind) = style.semantic_kind {
                    style.format = self.styles.style_for_kind(kind).clone();
                } else if sheet.id == "prose" {
                    match style.id.as_str() {
                        "body" => style.format = self.styles.body.clone(),
                        "chapter-heading" => {
                            style.format = self.styles.chapter_heading.clone()
                        }
                        _ => {}
                    }
                } else if let Some(custom) = self.styles.custom_named(&style.name) {
                    style.format = custom.style.clone();
                }
            }
        }
    }

    pub fn sync_element_styles_from_sheets(&mut self) {
        self.styles.update_from_sheets(&self.style_sheets);
    }
}

fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "en_US".to_string()
}

/// How page numbers are written in a compiled PDF.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum NumberFormat {
    Arabic,
    RomanLower,
    RomanUpper,
}

impl NumberFormat {
    pub fn label(self) -> &'static str {
        match self {
            NumberFormat::Arabic => "1, 2, 3",
            NumberFormat::RomanLower => "i, ii, iii",
            NumberFormat::RomanUpper => "I, II, III",
        }
    }
    pub fn all() -> [NumberFormat; 3] {
        [
            NumberFormat::Arabic,
            NumberFormat::RomanLower,
            NumberFormat::RomanUpper,
        ]
    }
    pub fn format(self, n: u32) -> String {
        match self {
            NumberFormat::Arabic => n.to_string(),
            NumberFormat::RomanLower => roman_numeral(n).to_lowercase(),
            NumberFormat::RomanUpper => roman_numeral(n),
        }
    }
}

/// Small classical Roman numeral helper for page numbers. Values outside the
/// classical 1–3999 range fall back to the Arabic digits so the writer never
/// loses a number.
fn roman_numeral(mut n: u32) -> String {
    if n == 0 || n > 3999 {
        return n.to_string();
    }
    const VALUES: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (value, glyph) in VALUES {
        while n >= value {
            out.push_str(glyph);
            n -= value;
        }
    }
    out
}

/// Where a page number sits on the page.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum HfLocation {
    Header,
    Footer,
}

impl HfLocation {
    pub fn label(self) -> &'static str {
        match self {
            HfLocation::Header => "Header",
            HfLocation::Footer => "Footer",
        }
    }
    pub fn all() -> [HfLocation; 2] {
        [HfLocation::Header, HfLocation::Footer]
    }
}

/// Header and footer controls for the compiled PDF. `header_text` blank means
/// the running header repeats the project title.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct HeaderFooter {
    pub page_numbers: bool,
    pub number_on_first_page: bool,
    pub number_format: NumberFormat,
    pub number_location: HfLocation,
    pub number_align: Alignment,
    pub number_spacing: f32,
    pub start_number: u32,
    pub running_header: bool,
    pub header_text: String,
    pub header_align: Alignment,
    pub header_spacing: f32,
}

impl Default for HeaderFooter {
    fn default() -> Self {
        Self {
            page_numbers: true,
            number_on_first_page: false,
            number_format: NumberFormat::Arabic,
            number_location: HfLocation::Footer,
            number_align: Alignment::Center,
            number_spacing: 30.0,
            start_number: 1,
            running_header: false,
            header_text: String::new(),
            header_align: Alignment::Center,
            header_spacing: 36.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ExportSettings {
    pub kind: ExportKind,
    pub paper: PaperSize,
    pub font: String,
    pub font_size: f32,
    pub line_spacing: f32,
    #[serde(default)]
    pub header_footer: HeaderFooter,
    #[serde(default)]
    pub cover_image: String,
    pub include_title_page: bool,
    pub include_toc: bool,
    pub include_chapter_numbers: bool,
    pub include_scene_numbers: bool,
    pub include_synopsis: bool,
    pub include_notes: bool,
    /// Write links in the text as links, in the formats that can carry them
    /// (see `ExportKind::writes_hyperlinks`).
    #[serde(default = "default_true")]
    pub include_hyperlinks: bool,
    pub output_path: String,
    /// Per-format output filename, joined to `output_path` on compile. Blank
    /// falls back to the export engine's standard name.
    #[serde(default)]
    pub output_filename: String,
    /// Per-element styles for this output format. A format that has never been
    /// configured is seeded from the project's base styles, then diverges.
    #[serde(default)]
    pub styles: ElementStyles,
    /// Which kind of manuscript to compile; `None` follows the first included
    /// document.
    #[serde(default)]
    pub compile_type: Option<crate::CompileType>,
    #[serde(default)]
    pub scope: crate::CompileScope,
    /// Script types only: write one file per script instead of one combined file.
    #[serde(default)]
    pub each_separately: bool,
    /// Page margins in points for this output format. `None` is automatic:
    /// prose uses the book default and scripts use their style sheet's margins
    /// (scaled to the paper). Every writer resolves this through
    /// `portmanteau_export::layout::page_margins`, so no writer owns a margin
    /// constant of its own.
    #[serde(default)]
    pub margins: Option<crate::rich_text::PageMargins>,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            kind: ExportKind::Pdf,
            // Portmanteau is written for a North American desk: US Letter is
            // the default page for every new output format. A5, A4 and the
            // trade paperback trim remain available per format.
            paper: PaperSize::Letter,
            font: "Liberation Serif".into(),
            font_size: 11.0,
            line_spacing: 1.4,
            header_footer: HeaderFooter::default(),
            cover_image: String::new(),
            include_title_page: true,
            include_toc: true,
            include_chapter_numbers: true,
            include_scene_numbers: true,
            include_synopsis: false,
            include_notes: false,
            include_hyperlinks: true,
            output_path: String::new(),
            output_filename: String::new(),
            styles: ElementStyles::default(),
            compile_type: None,
            scope: crate::CompileScope::Draft,
            each_separately: false,
            margins: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Project {
    pub schema: u32,
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub author: String,
    pub genre: String,
    pub logline: String,
    /// Who to write to about this work: name, address, email, phone, one per
    /// line. Printed bottom-left on a script's title page.
    #[serde(default)]
    pub contact: String,
    /// The draft's name on a script cover ("Draft", "Shooting script").
    #[serde(default)]
    pub draft_label: String,
    /// The draft's date as the writer wants it printed. Blank means today's
    /// date at the moment of export.
    #[serde(default)]
    pub draft_date: String,
    pub kind: ProjectKind,
    pub created: String,
    pub updated: String,
    pub documents: Vec<Document>,
    #[serde(default)]
    pub items: Vec<DatabaseItem>,
    pub checkpoints: Vec<Checkpoint>,
    pub session_log: Vec<SessionEntry>,
    pub daily_target: u32,
    pub export: ExportSettings,
    #[serde(default)]
    pub styles: ElementStyles,
    /// Project-owned prose and screenplay paragraph style sheets.
    #[serde(default = "default_style_sheets")]
    pub style_sheets: Vec<StyleSheet>,
    #[serde(default)]
    pub formats: Vec<ExportSettings>,
}

impl Project {
    pub fn new(title: impl Into<String>, kind: ProjectKind) -> Self {
        let stamp = now_stamp();
        let styles = ElementStyles::default();
        let export = if kind.default_medium().is_script() {
            screenplay_export_defaults(ExportSettings::default())
        } else {
            ExportSettings::default()
        };
        let mut project = Self {
            schema: 1,
            id: new_id("proj"),
            title: title.into(),
            subtitle: String::new(),
            author: "Your name".into(),
            genre: String::new(),
            logline: String::new(),
            contact: String::new(),
            draft_label: "Draft".into(),
            draft_date: String::new(),
            kind,
            created: stamp.clone(),
            updated: stamp,
            documents: Vec::new(),
            items: Vec::new(),
            checkpoints: Vec::new(),
            session_log: Vec::new(),
            daily_target: 1200,
            export,
            style_sheets: StyleSheet::industry_defaults(&styles),
            styles,
            formats: Vec::new(),
        };
        project.ensure_folders();
        project
    }

    pub fn ensure_folders(&mut self) {
        if self.documents.is_empty() {
            self.add_document(None, "Draft", DocKind::Folder, Medium::Prose);
            self.add_document(None, "Research", DocKind::Folder, Medium::Note);
            self.add_document(None, "Front Matter", DocKind::Folder, Medium::Prose);
            self.add_document(None, "Back Matter", DocKind::Folder, Medium::Prose);
        }
    }

    pub fn sync_element_styles_from_sheets(&mut self) {
        self.styles.update_from_sheets(&self.style_sheets);
    }

    /// Add any built-in stylesheet an older project lacks (TV, stage) and move
    /// TV and stage documents off the film sheet they used to share.
    pub fn ensure_style_sheets(&mut self) {
        StyleSheet::ensure_industry_defaults(&mut self.style_sheets, &self.styles);
        for doc in &mut self.documents {
            if doc.style_sheet_id == "film-screenplay"
                && matches!(doc.medium, Medium::TvScript | Medium::StagePlay)
            {
                doc.style_sheet_id = doc.medium.default_stylesheet_id().into();
            }
        }
    }

    /// The effective format for one paragraph style. `configured` holds an
    /// output format's saved element styles: a value that deliberately differs
    /// from the project base wins, while an untouched value defers to the
    /// project stylesheet so appearance edits reach previews and exporters
    /// without waiting for a compile save.
    pub fn paragraph_format_for(
        &self,
        style: &ParagraphStyle,
        configured: &ElementStyles,
    ) -> ElementStyle {
        let (base, saved) = match style.id.as_str() {
            "body" => (&self.styles.body, &configured.body),
            "chapter-heading" => (&self.styles.chapter_heading, &configured.chapter_heading),
            _ => {
                if let Some(kind) = style.semantic_kind {
                    (
                        self.styles.style_for_kind(kind),
                        configured.style_for_kind(kind),
                    )
                } else if let Some(custom) = self.styles.custom_named(&style.name) {
                    match configured.custom_named(&style.name) {
                        Some(saved) => (&custom.style, &saved.style),
                        None => (&custom.style, &custom.style),
                    }
                } else {
                    (&style.format, &style.format)
                }
            }
        };
        if saved == base {
            style.format.clone()
        } else {
            saved.clone()
        }
    }

    /// The stylesheet that owns a document's paragraphs. A project normally
    /// holds at least the industry defaults, but an older or hand-built project
    /// may describe a document without one. Falling back to the medium's
    /// standard sheet keeps the editor and every export writer agreed.
    pub fn style_sheet_for(&self, doc: &Document) -> StyleSheet {
        self.style_sheets
            .iter()
            .find(|sheet| sheet.id == doc.style_sheet_id)
            .cloned()
            .unwrap_or_else(|| {
                StyleSheet::for_medium(doc.medium, &self.styles)
            })
    }

    /// Per-output-format settings. A format that has never been configured is
    /// seeded from the project's element styles rather than sharing one global blob.
    pub fn settings_for(&self, kind: ExportKind) -> ExportSettings {
        if let Some(settings) = self.formats.iter().find(|s| s.kind == kind) {
            return settings.clone();
        }
        let mut settings = ExportSettings {
            kind,
            styles: self.styles.clone(),
            ..ExportSettings::default()
        };
        if self.kind.default_medium().is_script() {
            settings = screenplay_export_defaults(settings);
        }
        let base = if self.kind.default_medium().is_script() {
            &self.styles.action
        } else {
            &self.styles.body
        };
        settings.font = base.font.clone();
        settings.font_size = base.size;
        settings.line_spacing = base.leading;
        settings
    }

    /// Replace the settings for a format, or append them when it is new.
    pub fn set_settings(&mut self, settings: ExportSettings) {
        if let Some(slot) = self.formats.iter_mut().find(|s| s.kind == settings.kind) {
            *slot = settings;
        } else {
            self.formats.push(settings);
        }
    }

    pub fn touch(&mut self) {
        self.updated = now_stamp();
    }

    pub fn document(&self, id: &str) -> Option<&Document> {
        self.documents.iter().find(|d| d.id == id)
    }

    /// Where a database item shows up in the writing: every document (research
    /// notes included) that names it or links to it, in binder order, with the
    /// first such place in the document's own text. A character is also found by
    /// its first name (a screenplay cue says MARA for Mara Venn). Matching is
    /// whole-word and ignores case.
    pub fn appearances_of(&self, item: &DatabaseItem) -> Vec<Appearance> {
        let names = item_names(item);
        let link = format!("item:{}", item.id);
        let mut found = Vec::new();
        for doc in self.ordered_documents().into_iter().filter(|d| !d.kind.is_folder()) {
            let mut best: Option<(usize, usize, usize, String, bool)> = None; // (doc-wide start, paragraph, offset, excerpt, by link)
            let mut consider = |start: usize, paragraph: usize, offset: usize, excerpt: String, by_link: bool| {
                if best.as_ref().is_none_or(|b| start < b.0) {
                    best = Some((start, paragraph, offset, excerpt, by_link));
                }
            };
            let mut base = 0usize;
            for (index, paragraph) in doc.content.paragraphs.iter().enumerate() {
                let text = paragraph.editor_text();
                let chars: Vec<char> = text.chars().collect();
                if let Some(at) = find_name(&chars, &names) {
                    consider(base + at, index, at, excerpt_around(&chars, at), false);
                }
                base += chars.len() + 1;
            }
            for (range, target) in doc.content.links() {
                if target == link {
                    let position = doc.content.position_at(range.start);
                    let text: Vec<char> = doc.content.paragraphs[position.paragraph].editor_text().chars().collect();
                    consider(range.start, position.paragraph, position.offset, excerpt_around(&text, position.offset), true);
                }
            }
            match best {
                Some((_, paragraph, offset, excerpt, by_link)) => found.push(Appearance {
                    doc_id: doc.id.clone(),
                    title: doc.title.clone(),
                    place: Some((paragraph, offset)),
                    excerpt,
                    by_link,
                    in_research: self.in_research(doc),
                }),
                None => {
                    // Not in the text itself: named in its synopsis or notes.
                    let aside = format!("{}\n{}", doc.synopsis.plain_text(), doc.notes.plain_text());
                    let chars: Vec<char> = aside.chars().collect();
                    if let Some(at) = find_name(&chars, &names) {
                        found.push(Appearance {
                            doc_id: doc.id.clone(),
                            title: doc.title.clone(),
                            place: None,
                            excerpt: excerpt_around(&chars, at),
                            by_link: false,
                            in_research: self.in_research(doc),
                        });
                    }
                }
            }
        }
        found
    }

    /// The other database items whose notes link to this one.
    pub fn items_linking_to(&self, item: &DatabaseItem) -> Vec<&DatabaseItem> {
        let link = format!("item:{}", item.id);
        self.items
            .iter()
            .filter(|other| other.id != item.id && other.notes.links().iter().any(|(_, target)| *target == link))
            .collect()
    }

    /// Every research note, in binder order: a document inside the Research
    /// folder that is not itself a folder.
    pub fn research_docs(&self) -> Vec<&Document> {
        self.ordered_documents()
            .into_iter()
            .filter(|doc| !doc.kind.is_folder() && self.in_research(doc))
            .collect()
    }

    pub fn document_mut(&mut self, id: &str) -> Option<&mut Document> {
        self.documents.iter_mut().find(|d| d.id == id)
    }

    pub fn folder_id(&self, title: &str) -> Option<String> {
        self.documents
            .iter()
            .find(|d| d.title == title && d.kind.is_folder())
            .map(|d| d.id.clone())
    }

    pub fn children(&self, parent: Option<&str>) -> Vec<&Document> {
        let mut list: Vec<&Document> = self
            .documents
            .iter()
            .filter(|d| d.parent.as_deref() == parent)
            .collect();
        list.sort_by_key(|d| d.order);
        list
    }

    /// Where a new binder document goes: the folder the selected document is
    /// in (or the selected folder itself), else the top of the Draft tree.
    /// Research has its own page, so it is never the answer.
    pub fn new_document_home(&self, selected: Option<&str>) -> Option<String> {
        let near = selected.and_then(|id| self.document(id)).and_then(|d| {
            let folder = if d.kind.is_folder() { Some(d.id.clone()) } else { d.parent.clone() };
            let folder_doc = folder.as_deref().and_then(|id| self.document(id))?;
            let in_research = self.in_research(folder_doc) || (folder_doc.parent.is_none() && folder_doc.title == "Research");
            (!in_research).then(|| folder_doc.id.clone())
        });
        near.or_else(|| self.folder_id("Draft"))
    }

    pub fn next_order(&self, parent: Option<&str>) -> i32 {
        self.documents
            .iter()
            .filter(|d| d.parent.as_deref() == parent)
            .map(|d| d.order)
            .max()
            .unwrap_or(-1)
            + 1
    }

    pub fn add_document(
        &mut self,
        parent: Option<String>,
        title: impl Into<String>,
        kind: DocKind,
        medium: Medium,
    ) -> String {
        let order = self.next_order(parent.as_deref());
        let doc = Document::new(parent, order, title, kind, medium);
        let id = doc.id.clone();
        self.documents.push(doc);
        self.touch();
        id
    }

    pub fn insert(&mut self, doc: Document) -> String {
        let id = doc.id.clone();
        self.documents.push(doc);
        self.touch();
        id
    }

    pub fn delete_document(&mut self, id: &str) {
        let mut doomed = vec![id.to_string()];
        let mut i = 0;
        while i < doomed.len() {
            let current = doomed[i].clone();
            for child in self.documents.iter().filter(|d| d.parent.as_deref() == Some(&current)) {
                doomed.push(child.id.clone());
            }
            i += 1;
        }
        self.documents.retain(|d| !doomed.contains(&d.id));
        self.touch();
    }

    /// Move a document one place up or down among its siblings.
    /// Returns false at the edge of the list.
    pub fn move_document(&mut self, id: &str, direction: i32) -> bool {
        let Some(doc) = self.document(id) else {
            return false;
        };
        let parent = doc.parent.clone();
        let siblings: Vec<String> = self
            .children(parent.as_deref())
            .iter()
            .map(|d| d.id.clone())
            .collect();
        let Some(pos) = siblings.iter().position(|s| s == id) else {
            return false;
        };
        let new_pos = pos as i32 + direction;
        if new_pos < 0 || new_pos as usize >= siblings.len() {
            return false;
        }
        let other = siblings[new_pos as usize].clone();
        let this_order = self.document(id).map(|d| d.order).unwrap_or(pos as i32);
        let other_order = self
            .document(&other)
            .map(|d| d.order)
            .unwrap_or(new_pos as i32);
        if let Some(d) = self.document_mut(id) {
            d.order = other_order;
            d.touch();
        }
        if let Some(d) = self.document_mut(&other) {
            d.order = this_order;
            d.touch();
        }
        self.touch();
        true
    }

    /// Move a document into another folder, or to the project root.
    /// A folder cannot be moved inside itself.
    pub fn reparent_document(&mut self, id: &str, parent: Option<String>) -> bool {
        if parent.as_deref() == Some(id) {
            return false;
        }
        if let Some(target) = parent.as_deref() {
            if self.descendant_ids(id).iter().any(|d| d == target) {
                return false;
            }
            if self.document(target).is_none() {
                return false;
            }
        }
        let order = self.next_order(parent.as_deref());
        let Some(doc) = self.document_mut(id) else {
            return false;
        };
        doc.parent = parent;
        doc.order = order;
        doc.touch();
        self.touch();
        true
    }

    /// Place `id` immediately AFTER `target` among the target's siblings,
    /// adopting the target's parent. Used by the corkboard's drag-and-drop.
    ///
    /// Refuses when the two ids are the same, either document is missing, or
    /// `id` is an ancestor of `target` (a folder cannot be dropped inside its
    /// own subtree).
    pub fn move_after(&mut self, id: &str, target: &str) -> bool {
        if id == target {
            return false;
        }
        let Some(new_parent) = self.document(target).map(|d| d.parent.clone()) else {
            return false;
        };
        if self.document(id).is_none() {
            return false;
        }
        if self.descendant_ids(id).iter().any(|d| d == target) {
            return false;
        }

        // Build the destination sibling list without the moved document, so a
        // same-parent move is a single reorder rather than a duplicate.
        let mut siblings: Vec<String> = self
            .children(new_parent.as_deref())
            .iter()
            .map(|d| d.id.clone())
            .filter(|s| s != id)
            .collect();
        let Some(pos) = siblings.iter().position(|s| s == target) else {
            return false;
        };
        siblings.insert(pos + 1, id.to_string());

        if let Some(doc) = self.document_mut(id) {
            doc.parent = new_parent;
        }
        for (index, sibling) in siblings.iter().enumerate() {
            if let Some(doc) = self.document_mut(sibling) {
                doc.order = index as i32;
            }
        }
        if let Some(doc) = self.document_mut(id) {
            doc.touch();
        }
        self.touch();
        true
    }

    pub fn descendant_ids(&self, id: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![id.to_string()];
        while let Some(current) = stack.pop() {
            for child in self.documents.iter().filter(|d| d.parent.as_deref() == Some(&current)) {
                out.push(child.id.clone());
                stack.push(child.id.clone());
            }
        }
        out
    }

    pub fn ordered_documents(&self) -> Vec<&Document> {
        fn walk<'a>(p: &'a Project, parent: Option<&str>, out: &mut Vec<&'a Document>) {
            for doc in p.children(parent) {
                out.push(doc);
                walk(p, Some(&doc.id), out);
            }
        }
        let mut out = Vec::new();
        walk(self, None, &mut out);
        out
    }

    pub fn manuscript_documents(&self) -> Vec<&Document> {
        self.ordered_documents()
            .into_iter()
            .filter(|d| d.include_in_export && !d.kind.is_folder())
            .filter(|d| {
                let mut p = d.parent.clone();
                while let Some(pid) = p {
                    if let Some(parent) = self.document(&pid) {
                        if parent.title == "Research" {
                            return false;
                        }
                        p = parent.parent.clone();
                    } else {
                        break;
                    }
                }
                true
            })
            .collect()
    }

    pub fn words(&self) -> u32 {
        self.documents
            .iter()
            .filter(|d| d.include_in_export && !d.kind.is_folder())
            .map(|d| d.words())
            .sum()
    }

    pub fn today_words(&self) -> u32 {
        self.session_log
            .iter()
            .find(|s| s.date == today())
            .map(|s| s.words)
            .unwrap_or(0)
    }

    pub fn log_words(&mut self, words: u32) {
        let day = today();
        if let Some(entry) = self.session_log.iter_mut().find(|s| s.date == day) {
            entry.words = entry.words.saturating_add(words);
        } else {
            self.session_log.push(SessionEntry { date: day, words });
        }
    }

    pub fn locations(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for doc in &self.documents {
            let sheet = self
                .style_sheets
                .iter()
                .find(|sheet| sheet.id == doc.style_sheet_id);
            for paragraph in &doc.content.paragraphs {
                let kind = sheet
                    .and_then(|sheet| sheet.style(&paragraph.style_id))
                    .and_then(|style| style.semantic_kind);
                if kind == Some(ElementKind::SceneHeading) {
                    let text = paragraph.text_only();
                    let loc = text
                        .split(|c| c == '-' || c == '—')
                        .next()
                        .unwrap_or("")
                        .trim();
                    if !loc.is_empty() && !seen.iter().any(|item| item == loc) {
                        seen.push(loc.to_owned());
                    }
                }
            }
        }
        seen
    }

    pub fn checkpoint(&mut self, label: impl Into<String>, kind: impl Into<String>) -> String {
        let payload = serde_json::to_string(self).unwrap_or_default();
        let cp = Checkpoint {
            id: new_id("cp"),
            label: label.into(),
            created: now_stamp(),
            kind: kind.into(),
            words: self.words(),
            payload,
        };
        let id = cp.id.clone();
        self.checkpoints.insert(0, cp);
        if self.checkpoints.len() > 30 {
            self.checkpoints.truncate(30);
        }
        self.touch();
        id
    }

    pub fn restore_checkpoint(&mut self, checkpoint_id: &str) -> bool {
        let payload = self
            .checkpoints
            .iter()
            .find(|c| c.id == checkpoint_id)
            .map(|c| c.payload.clone());
        if let Some(payload) = payload {
            if let Ok(mut restored) = serde_json::from_str::<Project>(&payload) {
                let checkpoints = std::mem::take(&mut self.checkpoints);
                restored.checkpoints = checkpoints;
                *self = restored;
                self.touch();
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_has_scaffold() {
        let p = Project::new("Test", ProjectKind::Novel);
        assert_eq!(p.documents.len(), 4);
        assert!(p.folder_id("Draft").is_some());
        assert!(p.style_sheets.iter().any(|sheet| sheet.id == "prose"));
        assert!(
            p.style_sheets
                .iter()
                .any(|sheet| sheet.id == "film-screenplay")
        );
    }

    #[test]
    fn new_documents_select_a_stylesheet_by_medium() {
        let prose = Document::new(None, 0, "Chapter", DocKind::Chapter, Medium::Prose);
        let screenplay = Document::new(
            None,
            0,
            "Scene",
            DocKind::Scene,
            Medium::Screenplay,
        );
        assert_eq!(prose.style_sheet_id, "prose");
        assert_eq!(screenplay.style_sheet_id, "film-screenplay");
    }

    #[test]
    fn global_element_defaults_seed_new_project_style_sheets() {
        let mut settings = AppSettings::default();
        settings.style_sheets[0].styles[0].format.size = 13.0;
        let sheets = settings.style_sheets_for_new_project();
        let body = sheets
            .iter()
            .find(|sheet| sheet.id == "prose")
            .and_then(|sheet| sheet.style("body"))
            .expect("the prose sheet should include a body style");
        assert_eq!(body.format.size, 13.0);

        let mut project_copy = sheets;
        project_copy[0].styles[0].format.size = 17.0;
        assert_eq!(settings.style_sheets[0].styles[0].format.size, 13.0);
    }

    #[test]
    fn element_styles_and_stylesheets_stay_in_sync_both_ways() {
        let mut settings = AppSettings::default();
        settings.styles.body.size = 15.0;
        settings.styles.character.indent = 44.0;
        settings.sync_sheet_formats_from_element_styles();
        let prose = settings
            .style_sheets
            .iter()
            .find(|sheet| sheet.id == "prose")
            .and_then(|sheet| sheet.style("body"))
            .expect("prose body style");
        assert_eq!(prose.format.size, 15.0);
        let cue = settings
            .style_sheets
            .iter()
            .find(|sheet| sheet.id == "film-screenplay")
            .and_then(|sheet| sheet.style("character"))
            .expect("character style");
        assert_eq!(cue.format.indent, 44.0);
        assert_eq!(cue.next_style.as_deref(), Some("dialogue"));

        settings
            .style_sheets
            .iter_mut()
            .find(|sheet| sheet.id == "prose")
            .unwrap()
            .styles[0]
            .format
            .size = 18.0;
        settings.sync_element_styles_from_sheets();
        assert_eq!(settings.styles.body.size, 18.0);
    }

    #[test]
    fn app_settings_without_layout_fields_default_to_visible_panels() {
        let settings: AppSettings = serde_json::from_str("{}").expect("empty settings parse");
        assert!(settings.show_binder, "the binder should start visible");
        assert!(settings.show_inspector, "the inspector should start visible");
    }

    #[test]
    fn new_formats_default_to_us_letter() {
        // The product is used on a North American desk. A writer can still
        // choose A4, A5 or the trade paperback trim for any format.
        for kind in ProjectKind::all() {
            let project = Project::new("Test", kind);
            for export in [
                ExportKind::Pdf,
                ExportKind::Docx,
                ExportKind::Rtf,
                ExportKind::Epub,
            ] {
                assert_eq!(
                    project.settings_for(export).paper,
                    PaperSize::Letter,
                    "{kind:?} {export:?}"
                );
            }
        }
    }

    #[test]
    fn film_project_export_defaults_use_screenplay_page_numbering() {
        let project = Project::new("Feature", ProjectKind::Film);
        let settings = project.settings_for(ExportKind::Pdf);
        assert_eq!(settings.paper, PaperSize::Letter);
        assert!(settings.header_footer.page_numbers);
        assert!(!settings.header_footer.number_on_first_page);
        assert_eq!(settings.header_footer.number_location, HfLocation::Header);
        assert_eq!(settings.header_footer.number_align, Alignment::Right);
        assert_eq!(settings.header_footer.number_spacing, 36.0);
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let mut p = Project::new("Test", ProjectKind::Novel);
        let parent = p.folder_id("Draft");
        let id = p.add_document(parent, "Chapter", DocKind::Chapter, Medium::Prose);
        p.document_mut(&id).unwrap().set_plain_text("one two three");
        p.document_mut(&id).unwrap().snapshot("First pass", "rough");
        p.document_mut(&id).unwrap().set_plain_text("changed");
        let snap = p.document(&id).unwrap().snapshots[0].id.clone();
        assert!(p.document_mut(&id).unwrap().restore_snapshot(&snap));
        assert_eq!(p.document(&id).unwrap().body_text(), "one two three");
    }

    #[test]
    fn checkpoint_restores_project() {
        let mut p = Project::new("Test", ProjectKind::Novel);
        p.title = "Original".into();
        let cp = p.checkpoint("Before edit", "manual");
        p.title = "Changed".into();
        assert!(p.restore_checkpoint(&cp));
        assert_eq!(p.title, "Original");
    }

    #[test]
    fn documents_move_within_their_parent() {
        let mut p = Project::new("Test", ProjectKind::Novel);
        let parent = p.folder_id("Draft");
        let first = p.add_document(parent.clone(), "First", DocKind::Chapter, Medium::Prose);
        let second = p.add_document(parent, "Second", DocKind::Chapter, Medium::Prose);
        assert!(p.move_document(&second, -1));
        let order: Vec<String> = p
            .children(p.folder_id("Draft").as_deref())
            .iter()
            .map(|d| d.title.clone())
            .collect();
        assert_eq!(order, vec!["Second", "First"]);
        assert!(!p.move_document(&second, -1));
        assert!(p.move_document(&first, -1));
    }

    #[test]
    fn move_after_places_a_document_directly_after_its_target() {
        let mut p = Project::new("Test", ProjectKind::Novel);
        let draft = p.folder_id("Draft");
        let a = p.add_document(draft.clone(), "A", DocKind::Chapter, Medium::Prose);
        let b = p.add_document(draft.clone(), "B", DocKind::Chapter, Medium::Prose);
        let c = p.add_document(draft.clone(), "C", DocKind::Chapter, Medium::Prose);

        // Same parent: C after A leaves A, C, B.
        assert!(p.move_after(&c, &a));
        let order: Vec<String> = p
            .children(draft.as_deref())
            .iter()
            .map(|d| d.title.clone())
            .collect();
        assert_eq!(order, vec!["A", "C", "B"]);

        // Across parents: a new folder adopts the moved document.
        let other = p.add_document(draft.clone(), "Other", DocKind::Folder, Medium::Prose);
        let d = p.add_document(Some(other.clone()), "D", DocKind::Text, Medium::Prose);
        let e = p.add_document(Some(other.clone()), "E", DocKind::Text, Medium::Prose);
        assert!(p.move_after(&b, &d));
        let moved: Vec<String> = p
            .children(Some(&other))
            .iter()
            .map(|d| d.title.clone())
            .collect();
        assert_eq!(moved, vec!["D", "B", "E"]);
        assert_eq!(p.document(&b).unwrap().parent.as_deref(), Some(other.as_str()));

        // The moved order is contiguous, and invalid targets are refused.
        assert!(p.move_after(&e, &a));
        assert_eq!(
            p.document(&e).unwrap().parent.as_deref(),
            draft.as_deref()
        );
        assert!(!p.move_after(&a, &a));
        assert!(!p.move_after(&a, "doc-missing"));
        assert!(!p.move_after("doc-missing", &a));
        assert!(!p.move_after(&other, &d));
    }

    #[test]
    fn reparent_refuses_cycles() {
        let mut p = Project::new("Test", ProjectKind::Novel);
        let parent = p.folder_id("Draft");
        let folder = p.add_document(parent, "Act One", DocKind::Folder, Medium::Prose);
        let scene = p.add_document(Some(folder.clone()), "Scene", DocKind::Text, Medium::Prose);
        assert!(p.reparent_document(&scene, Some(folder.clone())));
        assert!(!p.reparent_document(&folder, Some(scene)));
    }

    #[test]
    fn snapshot_body_text_uses_blocks_for_scripts() {
        let mut p = Project::new("Test", ProjectKind::Film);
        let parent = p.folder_id("Draft");
        let id = p.add_document(parent, "Scene", DocKind::Scene, Medium::Screenplay);
        let doc = p.document_mut(&id).unwrap();
        doc.set_script_blocks(vec![Block::new(
            ElementKind::Action,
            "The tide went out.",
        )]);
        doc.snapshot("First", "");
        assert_eq!(p.document(&id).unwrap().snapshots[0].body_text(), "The tide went out.");
    }

    #[test]
    fn element_styles_defaults_match_the_base_type() {
        let styles = ElementStyles::default();
        assert_eq!(styles.body.font, "Liberation Serif");
        assert_eq!(styles.body.size, 11.0);
        assert_eq!(styles.body.leading, 1.4);
        assert_eq!(styles.body.first_line_indent, 14.0);
        assert_eq!(styles.body.align, Alignment::Left);
        assert_eq!(styles.chapter_heading.size, 16.0);
        assert_eq!(styles.chapter_heading.before, 12.0);
        assert_eq!(styles.chapter_heading.after, 8.0);
        assert_eq!(styles.character.font, "Courier New");
        assert_eq!(styles.character.size, 12.0);
        assert_eq!(styles.character.leading, 1.0);
        assert_eq!(styles.character.indent, 158.4);
        assert_eq!(styles.parenthetical.indent, 115.2);
        assert_eq!(styles.parenthetical.right_indent, 136.8);
        assert_eq!(styles.dialogue.indent, 72.0);
        assert_eq!(styles.dialogue.right_indent, 108.0);
        assert_eq!(styles.transition.align, Alignment::Right);
        assert!(styles.extension.uppercase);
        assert!(styles.intercut.uppercase);
        assert!(styles.subheading.uppercase);
    }

    #[test]
    fn element_style_alignment_and_indents_round_trip() {
        let mut style = ElementStyle::default();
        style.align = Alignment::Center;
        style.first_line_indent = 18.0;
        style.right_indent = 6.0;

        let json = serde_json::to_string(&style).unwrap();
        let restored: ElementStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, style);
        assert_eq!(restored.align, Alignment::Center);
        assert_eq!(restored.first_line_indent, 18.0);
        assert_eq!(restored.right_indent, 6.0);

        // A style saved before these fields existed must still load with the
        // documented defaults.
        let mut value = serde_json::to_value(&style).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("align");
        object.remove("first_line_indent");
        object.remove("right_indent");
        let old: ElementStyle = serde_json::from_value(value).unwrap();
        assert_eq!(old.align, Alignment::Left);
        assert_eq!(old.first_line_indent, 0.0);
        assert_eq!(old.right_indent, 0.0);
    }

    #[test]
    fn settings_for_seeds_a_missing_format_from_style() {
        let mut p = Project::new("Test", ProjectKind::Novel);
        p.styles.body.font = "Georgia".into();
        p.styles.body.size = 13.0;
        assert!(p.formats.is_empty());
        let seeded = p.settings_for(ExportKind::Pdf);
        assert_eq!(seeded.kind, ExportKind::Pdf);
        assert_eq!(seeded.font, "Georgia");
        assert_eq!(seeded.font_size, 13.0);
        assert!(p.formats.is_empty(), "seeding must not write through");

        p.set_settings(seeded.clone());
        let stored = p.settings_for(ExportKind::Pdf);
        assert_eq!(stored, seeded);
        assert_eq!(p.formats.len(), 1);
    }

    #[test]
    fn set_settings_upserts_per_format() {
        let mut p = Project::new("Test", ProjectKind::Novel);
        let mut pdf = p.settings_for(ExportKind::Pdf);
        p.set_settings(pdf.clone());
        let mut epub = p.settings_for(ExportKind::Epub);
        p.set_settings(epub.clone());
        assert_eq!(p.formats.len(), 2);

        pdf.font_size = 18.0;
        p.set_settings(pdf.clone());
        assert_eq!(p.formats.len(), 2, "same kind replaces, does not duplicate");
        assert_eq!(p.settings_for(ExportKind::Pdf), pdf);
        assert_eq!(p.settings_for(ExportKind::Epub), epub);

        epub.font_size = 14.0;
        p.set_settings(epub.clone());
        assert_eq!(p.settings_for(ExportKind::Pdf), pdf);
        assert_eq!(p.settings_for(ExportKind::Epub), epub);
    }

    #[test]
    fn old_project_json_without_round_two_fields_still_loads() {
        let p = Project::new("Legacy", ProjectKind::Novel);
        let mut value = serde_json::to_value(&p).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("styles");
        object.remove("formats");
        let export = object.get_mut("export").unwrap().as_object_mut().unwrap();
        export.remove("header_footer");
        export.remove("cover_image");

        let restored: Project = serde_json::from_value(value).unwrap();
        assert_eq!(restored.styles, ElementStyles::default());
        assert!(restored.formats.is_empty());
        assert!(restored.export.header_footer.page_numbers);
        assert!(!restored.export.header_footer.running_header);
        assert_eq!(restored.export.cover_image, "");
    }

    #[test]
    fn custom_styles_round_trip_and_lookup() {
        let mut styles = ElementStyles::default();
        assert!(styles.custom.is_empty());
        assert!(styles.custom_named("Epigraph").is_none());

        let mut style = ElementStyle::default();
        style.font = "Liberation Serif".into();
        style.size = 10.0;
        style.align = Alignment::Center;
        styles.custom.push(CustomStyle {
            name: "Epigraph".into(),
            style,
        });

        let json = serde_json::to_string(&styles).unwrap();
        let restored: ElementStyles = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, styles);
        let epigraph = restored.custom_named("Epigraph").unwrap();
        assert_eq!(epigraph.style.size, 10.0);
        assert_eq!(epigraph.style.align, Alignment::Center);

        // Projects saved before custom styles existed still load.
        let mut value = serde_json::to_value(&styles).unwrap();
        value.as_object_mut().unwrap().remove("custom");
        let old: ElementStyles = serde_json::from_value(value).unwrap();
        assert!(old.custom.is_empty());
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;

    fn project_with(text: &str) -> (Project, DatabaseItem, String) {
        let mut p = Project::new("Test", ProjectKind::Novel);
        let item = DatabaseItem::new(ItemKind::Character, "Mara Venn");
        let id = p.add_document(p.folder_id("Draft"), "Chapter", DocKind::Chapter, Medium::Prose);
        p.document_mut(&id).unwrap().content = RichDocument::from_plain_text(text, "body");
        p.items.push(item.clone());
        (p, item, id)
    }

    #[test]
    fn an_item_is_found_by_its_name_or_a_characters_first_name_at_the_first_place() {
        let (p, item, id) = project_with("The tide came in.\nThen MARA stood up. Mara Venn smiled.");
        let found = p.appearances_of(&item);
        assert_eq!(found.len(), 1);
        let a = &found[0];
        assert_eq!(a.doc_id, id);
        // Second paragraph, after "Then ".
        assert_eq!(a.place, Some((1, 5)));
        assert!(a.excerpt.contains("MARA stood up"), "{}", a.excerpt);
        assert!(!a.by_link && !a.in_research);
    }

    #[test]
    fn matching_is_whole_word_and_a_place_is_not_found_by_a_first_name() {
        let (p, item, _) = project_with("The Maraud was loud. Maras everywhere.");
        assert!(p.appearances_of(&item).is_empty(), "Maraud and Maras are other words");
        let mut place = DatabaseItem::new(ItemKind::Place, "Cold Harbor");
        place.id = "place-1".into();
        let mut p = p;
        p.document_mut(&p.documents.iter().find(|d| d.title == "Chapter").unwrap().id.clone()).unwrap().content =
            RichDocument::from_plain_text("Cold weather at the harbor.", "body");
        assert!(p.appearances_of(&place).is_empty(), "only the whole title (or a character's first name) counts");
    }

    #[test]
    fn a_link_counts_even_where_the_words_differ_and_notes_only_hits_have_no_place() {
        let (mut p, item, id) = project_with("She waved to the surveyor.");
        {
            let doc = p.document_mut(&id).unwrap();
            doc.content.set_link(8..14, Some(&format!("item:{}", item.id)));
        }
        let found = p.appearances_of(&item);
        assert_eq!(found.len(), 1);
        assert!(found[0].by_link);
        assert_eq!(found[0].place, Some((0, 8)));

        let other = p.add_document(p.folder_id("Draft"), "Scene", DocKind::Scene, Medium::Prose);
        p.document_mut(&other).unwrap().set_notes_text("Remember to check Mara Venn's timeline.");
        let found = p.appearances_of(&item);
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].title, "Scene");
        assert_eq!(found[1].place, None, "named in the notes, not the text");
    }

    #[test]
    fn items_that_link_to_this_one_are_listed() {
        let (mut p, item, _) = project_with("x");
        let mut other = DatabaseItem::new(ItemKind::Place, "Cold Harbor");
        other.notes = RichDocument::from_plain_text("Home of Mara.", "body");
        other.notes.set_link(8..12, Some(&format!("item:{}", item.id)));
        p.items.push(other.clone());
        let linking = p.items_linking_to(&item);
        assert_eq!(linking.len(), 1);
        assert_eq!(linking[0].title, "Cold Harbor");
        assert!(p.items_linking_to(&other).is_empty());
    }
}

#[cfg(test)]
mod new_document_home_tests {
    use super::*;

    #[test]
    fn new_documents_go_beside_the_selection_never_into_research() {
        let mut p = Project::new("T", ProjectKind::Novel);
        let draft = p.folder_id("Draft").unwrap();
        let research = p.folder_id("Research").unwrap();
        let part = p.add_document(Some(draft.clone()), "Part", DocKind::Folder, Medium::Prose);
        let chapter = p.add_document(Some(part.clone()), "Ch", DocKind::Chapter, Medium::Prose);
        let note = p.add_document(Some(research.clone()), "Note", DocKind::Research, Medium::Note);
        assert_eq!(p.new_document_home(Some(&chapter)), Some(part.clone()));
        assert_eq!(p.new_document_home(Some(&part)), Some(part));
        assert_eq!(p.new_document_home(None), Some(draft.clone()));
        assert_eq!(p.new_document_home(Some(&note)), Some(draft.clone()));
        assert_eq!(p.new_document_home(Some(&research)), Some(draft));
    }
}
