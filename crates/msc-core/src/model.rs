//! Scriptorium domain model: projects, binder, documents, script elements,
//! research, story development, versions and export settings.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::script::ElementKind;
pub use crate::text::{count_words, reading_minutes, truncate_words};

pub fn new_id(prefix: &str) -> String {
    format!("{}-{}", prefix, &Uuid::new_v4().simple().to_string()[..8])
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
    pub text: String,
    pub blocks: Vec<Block>,
    pub synopsis: String,
    pub notes: String,
    pub label: String,
    pub status: String,
    pub include_in_export: bool,
    pub word_target: u32,
    pub created: String,
    pub updated: String,
    pub snapshots: Vec<Snapshot>,
    pub pinned: bool,
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
            text: String::new(),
            blocks: Vec::new(),
            synopsis: String::new(),
            notes: String::new(),
            label: String::new(),
            status: "To do".into(),
            include_in_export: true,
            word_target: 0,
            created: stamp.clone(),
            updated: stamp,
            snapshots: Vec::new(),
            pinned: false,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn with_script(mut self, blocks: Vec<Block>) -> Self {
        self.blocks = blocks;
        self
    }

    pub fn with_synopsis(mut self, synopsis: impl Into<String>) -> Self {
        self.synopsis = synopsis.into();
        self
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
        if self.medium.is_script() {
            self.blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        } else {
            self.text.clone()
        }
    }

    pub fn words(&self) -> u32 {
        count_words(&self.body_text())
    }

    pub fn pages(&self) -> u32 {
        if self.medium.is_script() {
            // One screenplay page is roughly 55 lines.
            let lines: usize = self
                .blocks
                .iter()
                .map(|b| 1 + (b.text.len() / 60))
                .sum();
            ((lines.max(1) as f32) / 55.0).ceil() as u32
        } else {
            ((self.words() as f32) / 280.0).ceil().max(1.0) as u32
        }
    }

    pub fn touch(&mut self) {
        self.updated = now_stamp();
    }

    pub fn snapshot(&mut self, name: impl Into<String>, note: impl Into<String>) {
        let snap = Snapshot {
            id: new_id("snap"),
            name: name.into(),
            created: now_stamp(),
            note: note.into(),
            text: self.text.clone(),
            blocks: self.blocks.clone(),
            words: self.words(),
        };
        self.snapshots.insert(0, snap);
        if self.snapshots.len() > 60 {
            self.snapshots.truncate(60);
        }
    }

    pub fn restore_snapshot(&mut self, snapshot_id: &str) -> bool {
        if let Some(snap) = self.snapshots.iter().find(|s| s.id == snapshot_id).cloned() {
            self.text = snap.text;
            self.blocks = snap.blocks;
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
        if medium.is_script() {
            self.blocks = if self.blocks.is_empty() {
                crate::script::parse_loose(&body)
            } else {
                std::mem::take(&mut self.blocks)
            };
        } else {
            self.text = body;
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
    pub text: String,
    pub blocks: Vec<Block>,
    pub words: u32,
}

impl Snapshot {
    /// Reconstruct the saved body for comparison with the current document.
    /// A script snapshot stores typed blocks; a prose snapshot stores text.
    pub fn body_text(&self) -> String {
        if self.blocks.is_empty() {
            self.text.clone()
        } else {
            self.blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub want: String,
    pub need: String,
    pub flaw: String,
    pub arc: String,
    pub color: u8,
}

impl Character {
    pub fn new(name: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            id: new_id("char"),
            name: name.into(),
            role: role.into(),
            description: String::new(),
            want: String::new(),
            need: String::new(),
            flaw: String::new(),
            arc: String::new(),
            color: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ResearchItem {
    pub id: String,
    pub title: String,
    pub body: String,
    pub kind: String,
    pub source: String,
    pub created: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct IdeaCard {
    pub id: String,
    pub title: String,
    pub body: String,
    pub kind: String,
    pub color: u8,
    pub pinned: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Beat {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub act: u8,
    pub done: bool,
    pub template: String,
}

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

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ExportSettings {
    pub kind: ExportKind,
    pub paper: PaperSize,
    pub font: String,
    pub font_size: f32,
    pub line_spacing: f32,
    pub include_title_page: bool,
    pub include_copyright: bool,
    pub include_dedication: bool,
    pub include_acknowledgements: bool,
    pub include_preface: bool,
    pub include_toc: bool,
    pub include_chapter_numbers: bool,
    pub include_scene_numbers: bool,
    pub include_synopsis: bool,
    pub include_notes: bool,
    pub include_author_bio: bool,
    pub output_path: String,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            kind: ExportKind::Pdf,
            paper: PaperSize::A5,
            font: "Liberation Serif".into(),
            font_size: 11.0,
            line_spacing: 1.4,
            include_title_page: true,
            include_copyright: true,
            include_dedication: true,
            include_acknowledgements: true,
            include_preface: true,
            include_toc: true,
            include_chapter_numbers: true,
            include_scene_numbers: true,
            include_synopsis: false,
            include_notes: false,
            include_author_bio: true,
            output_path: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ExportPreset {
    pub id: String,
    pub name: String,
    pub settings: ExportSettings,
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
    pub kind: ProjectKind,
    pub created: String,
    pub updated: String,
    pub documents: Vec<Document>,
    pub characters: Vec<Character>,
    pub research: Vec<ResearchItem>,
    pub ideas: Vec<IdeaCard>,
    pub beats: Vec<Beat>,
    pub checkpoints: Vec<Checkpoint>,
    pub session_log: Vec<SessionEntry>,
    pub daily_target: u32,
    pub export: ExportSettings,
    pub presets: Vec<ExportPreset>,
}

impl Project {
    pub fn new(title: impl Into<String>, kind: ProjectKind) -> Self {
        let stamp = now_stamp();
        let mut project = Self {
            schema: 1,
            id: new_id("proj"),
            title: title.into(),
            subtitle: String::new(),
            author: "Your name".into(),
            genre: String::new(),
            logline: String::new(),
            kind,
            created: stamp.clone(),
            updated: stamp,
            documents: Vec::new(),
            characters: Vec::new(),
            research: Vec::new(),
            ideas: Vec::new(),
            beats: Vec::new(),
            checkpoints: Vec::new(),
            session_log: Vec::new(),
            daily_target: 1200,
            export: ExportSettings::default(),
            presets: Vec::new(),
        };
        project.ensure_folders();
        project
    }

    pub fn ensure_folders(&mut self) {
        if self.documents.is_empty() {
            self.add_document(None, "Manuscript", DocKind::Folder, Medium::Prose);
            self.add_document(None, "Research", DocKind::Folder, Medium::Note);
            self.add_document(None, "Front Matter", DocKind::Folder, Medium::Prose);
            self.add_document(None, "Back Matter", DocKind::Folder, Medium::Prose);
        }
    }

    pub fn touch(&mut self) {
        self.updated = now_stamp();
    }

    pub fn document(&self, id: &str) -> Option<&Document> {
        self.documents.iter().find(|d| d.id == id)
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

    pub fn locations(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for doc in &self.documents {
            for block in &doc.blocks {
                if block.kind == ElementKind::SceneHeading {
                    let loc = block
                        .text
                        .split(|c| c == '-' || c == '—')
                        .next()
                        .unwrap_or("")
                        .trim();
                    if !loc.is_empty() && !seen.contains(&loc) {
                        seen.push(loc);
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

    pub fn search(&self, query: &str) -> Vec<(&Document, String)> {
        let q = query.to_lowercase();
        let mut hits = Vec::new();
        if q.trim().is_empty() {
            return hits;
        }
        for doc in &self.documents {
            let body = doc.body_text();
            if doc.title.to_lowercase().contains(&q) {
                hits.push((doc, format!("Title — {}", doc.title)));
            } else if let Some(pos) = body.to_lowercase().find(&q) {
                let start = body[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let end = body[pos..]
                    .find('\n')
                    .map(|i| pos + i)
                    .unwrap_or(body.len());
                hits.push((doc, body[start..end].trim().to_string()));
            }
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_has_scaffold() {
        let p = Project::new("Test", ProjectKind::Novel);
        assert_eq!(p.documents.len(), 4);
        assert!(p.folder_id("Manuscript").is_some());
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let mut p = Project::new("Test", ProjectKind::Novel);
        let parent = p.folder_id("Manuscript");
        let id = p.add_document(parent, "Chapter", DocKind::Chapter, Medium::Prose);
        p.document_mut(&id).unwrap().text = "one two three".into();
        p.document_mut(&id).unwrap().snapshot("First pass", "rough");
        p.document_mut(&id).unwrap().text = "changed".into();
        let snap = p.document(&id).unwrap().snapshots[0].id.clone();
        assert!(p.document_mut(&id).unwrap().restore_snapshot(&snap));
        assert_eq!(p.document(&id).unwrap().text, "one two three");
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
        let parent = p.folder_id("Manuscript");
        let first = p.add_document(parent.clone(), "First", DocKind::Chapter, Medium::Prose);
        let second = p.add_document(parent, "Second", DocKind::Chapter, Medium::Prose);
        assert!(p.move_document(&second, -1));
        let order: Vec<String> = p
            .children(p.folder_id("Manuscript").as_deref())
            .iter()
            .map(|d| d.title.clone())
            .collect();
        assert_eq!(order, vec!["Second", "First"]);
        assert!(!p.move_document(&second, -1));
        assert!(p.move_document(&first, -1));
    }

    #[test]
    fn reparent_refuses_cycles() {
        let mut p = Project::new("Test", ProjectKind::Novel);
        let parent = p.folder_id("Manuscript");
        let folder = p.add_document(parent, "Act One", DocKind::Folder, Medium::Prose);
        let scene = p.add_document(Some(folder.clone()), "Scene", DocKind::Text, Medium::Prose);
        assert!(p.reparent_document(&scene, Some(folder.clone())));
        assert!(!p.reparent_document(&folder, Some(scene)));
    }

    #[test]
    fn snapshot_body_text_uses_blocks_for_scripts() {
        let mut p = Project::new("Test", ProjectKind::Film);
        let parent = p.folder_id("Manuscript");
        let id = p.add_document(parent, "Scene", DocKind::Scene, Medium::Screenplay);
        let doc = p.document_mut(&id).unwrap();
        doc.blocks = vec![Block::new(ElementKind::Action, "The tide went out.")];
        doc.snapshot("First", "");
        assert_eq!(p.document(&id).unwrap().snapshots[0].body_text(), "The tide went out.");
    }
}
