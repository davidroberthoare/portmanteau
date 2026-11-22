//! Importers: Fountain, Final Draft (FDX), Word (DOCX) and plain text.
//!
//! Every importer turns bytes into the same `Imported` value: one or more
//! documents of styled rich text, the title and author when the file says so,
//! and a list of warnings for anything that could not be carried across. The
//! rule is the mirror of the exporters': never silently lose the writer's
//! work. A file with an unknown paragraph type still imports, and the
//! warnings say what was set aside.
//!
//! This crate knows nothing about projects or the filesystem; the command
//! layer decides where the documents go.

mod docx;
mod encoding;
mod fdx;
mod fountain;
mod mapping;
mod marks;
mod plain_text;

use std::collections::BTreeMap;
use std::path::Path;

pub use mapping::{SourceStyle, Target};

use portmanteau_core::{DocKind, Medium, RichDocument};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImportFormat {
    Fountain,
    Fdx,
    Docx,
    PlainText,
}

impl ImportFormat {
    pub fn label(self) -> &'static str {
        match self {
            ImportFormat::Fountain => "Fountain",
            ImportFormat::Fdx => "Final Draft (FDX)",
            ImportFormat::Docx => "Word (DOCX)",
            ImportFormat::PlainText => "plain text",
        }
    }

    /// Accepts a file extension or a name a client might send.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().trim_start_matches('.').to_ascii_lowercase().as_str() {
            "fountain" | "spmd" => Some(Self::Fountain),
            "fdx" => Some(Self::Fdx),
            "docx" => Some(Self::Docx),
            "txt" | "text" | "plain" | "md" | "markdown" => Some(Self::PlainText),
            _ => None,
        }
    }

    /// Script formats default to a screenplay; the others to prose.
    pub fn default_medium(self) -> Medium {
        match self {
            ImportFormat::Fountain | ImportFormat::Fdx => Medium::Screenplay,
            ImportFormat::Docx | ImportFormat::PlainText => Medium::Prose,
        }
    }
}

/// Work out the format from the file name, falling back to the file's own
/// first bytes when the extension is missing or unhelpful.
pub fn detect(path: Option<&Path>, bytes: &[u8]) -> Option<ImportFormat> {
    if let Some(format) = path
        .and_then(Path::extension)
        .and_then(|extension| extension.to_str())
        .and_then(ImportFormat::from_name)
    {
        return Some(format);
    }
    if bytes.starts_with(b"PK") {
        return Some(ImportFormat::Docx);
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
    if head.contains("<FinalDraft") {
        return Some(ImportFormat::Fdx);
    }
    None
}

/// Whether to cut a long file into several documents.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Split {
    /// Split at the file's own structure: Word headings, Fountain sections,
    /// plain-text chapter lines.
    #[default]
    Auto,
    /// Always one document.
    Never,
}

#[derive(Clone, Debug, Default)]
pub struct ImportOptions {
    /// Force the writing medium; `None` follows the format.
    pub medium: Option<Medium>,
    pub split: Split,
    /// Overrides for what a source paragraph style becomes: source style name
    /// to a `Target` token (`heading:2`, `style:dialogue`, `skip`, …). Styles
    /// not listed follow the importer's defaults.
    pub style_map: BTreeMap<String, String>,
}

/// One document's worth of imported text.
#[derive(Clone, Debug)]
pub struct ImportedDocument {
    /// Empty when the file named nothing; the caller supplies a fallback.
    pub title: String,
    pub kind: DocKind,
    pub medium: Medium,
    pub content: RichDocument,
    /// Plain-text synopsis, when the file carried one (Fountain `=` lines).
    pub synopsis: String,
}

#[derive(Clone, Debug)]
pub struct Imported {
    pub format: ImportFormat,
    pub medium: Medium,
    /// The work's title and author, when the file states them.
    pub title: Option<String>,
    pub author: Option<String>,
    /// Contact lines, one per line, when the title page has them.
    pub contact: Option<String>,
    /// The draft date as the file writes it.
    pub draft_date: Option<String>,
    pub documents: Vec<ImportedDocument>,
    /// The paragraph styles the file uses and what each became (Word and Final
    /// Draft only; the other formats have no named styles).
    pub styles: Vec<SourceStyle>,
    /// Human-readable notes on anything not carried across.
    pub warnings: Vec<String>,
}

impl Imported {
    pub fn paragraph_count(&self) -> usize {
        self.documents.iter().map(|d| d.content.paragraphs.len()).sum()
    }
}

/// Import `bytes` as `format`.
pub fn import(
    format: ImportFormat,
    bytes: &[u8],
    options: &ImportOptions,
) -> Result<Imported, String> {
    let medium = options.medium.unwrap_or_else(|| format.default_medium());
    let mut notes = Notes::default();
    let parsed = match format {
        ImportFormat::Docx => docx::import(bytes, medium, options, &mut notes)?,
        ImportFormat::Fdx => {
            let (text, how) = encoding::decode(bytes);
            notes.encoding(how);
            fdx::import(&text, medium, options, &mut notes)?
        }
        ImportFormat::Fountain => {
            let (text, how) = encoding::decode(bytes);
            notes.encoding(how);
            fountain::import(&text, medium, options.split, &mut notes)
        }
        ImportFormat::PlainText => {
            let (text, how) = encoding::decode(bytes);
            notes.encoding(how);
            if medium.is_script() {
                // A screenplay typed as plain text is Fountain without the
                // title page conventions; the same parser reads it.
                fountain::import(&text, medium, options.split, &mut notes)
            } else {
                plain_text::import(&text, options.split)
            }
        }
    };
    if parsed.documents.iter().all(|d| d.content.paragraphs.is_empty()) {
        return Err("The file contains no text to import".into());
    }
    Ok(Imported {
        format,
        medium,
        title: parsed.title,
        author: parsed.author,
        contact: parsed.contact,
        draft_date: parsed.draft_date,
        documents: parsed.documents,
        styles: parsed.styles,
        warnings: notes.finish(),
    })
}

/// What an individual format importer hands back before the shared wrap-up.
#[derive(Default)]
pub(crate) struct Parsed {
    pub title: Option<String>,
    pub author: Option<String>,
    pub contact: Option<String>,
    pub draft_date: Option<String>,
    pub documents: Vec<ImportedDocument>,
    pub styles: Vec<SourceStyle>,
}

/// Collects warnings, counting repeats so a 40-footnote file yields one line.
#[derive(Default)]
pub(crate) struct Notes {
    counted: Vec<(&'static str, usize)>,
    said: Vec<String>,
}

impl Notes {
    /// Something of this kind was set aside `count` times ("footnote").
    pub(crate) fn dropped(&mut self, noun: &'static str, count: usize) {
        if count == 0 {
            return;
        }
        match self.counted.iter_mut().find(|(n, _)| *n == noun) {
            Some((_, total)) => *total += count,
            None => self.counted.push((noun, count)),
        }
    }

    /// A one-off remark.
    pub(crate) fn say(&mut self, message: impl Into<String>) {
        let message = message.into();
        if !self.said.contains(&message) {
            self.said.push(message);
        }
    }

    fn encoding(&mut self, how: encoding::Decoded) {
        if how == encoding::Decoded::Windows1252 {
            self.say("The file is not valid UTF-8, so it was read as Windows-1252; check accented letters and quotes");
        }
    }

    fn finish(self) -> Vec<String> {
        let mut out: Vec<String> = self
            .counted
            .into_iter()
            .map(|(noun, count)| {
                let plural = if count == 1 { "" } else { "s" };
                format!("{count} {noun}{plural} not imported")
            })
            .collect();
        out.extend(self.said);
        out
    }
}

#[cfg(test)]
mod tests;
