//! Word `.docx`: a zip of XML parts.
//!
//! Read: `word/document.xml` (the text), `word/styles.xml` (what each paragraph
//! style *is*, so a renamed or localised "Heading 1" is still a heading) and
//! `docProps/core.xml` (title and author). Heading paragraphs cut the file
//! into documents; footnotes, images and list formatting are not carried
//! across, and the warnings say how many.

use std::collections::HashMap;
use std::io::Read;

use portmanteau_core::rich_text::script_style_id;
use portmanteau_core::{
    script, DocKind, InlineContent, InlineMark, Medium, RichDocument, RichParagraph, TextRun,
};
use roxmltree::Node;

use crate::fdx::element_kind;
use crate::{ImportedDocument, Notes, Parsed, Split};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// A Word file is a zip from an unknown source, and a few kilobytes can
/// inflate to gigabytes. No part we read may exceed this many bytes once
/// decompressed; a real manuscript's text part is a small fraction of it.
const MAX_PART_BYTES: u64 = 64 * 1024 * 1024;

/// Upper bound on XML elements in one part, against deeply padded documents.
const MAX_XML_NODES: u32 = 4_000_000;

pub(crate) fn import(
    bytes: &[u8],
    medium: Medium,
    split: Split,
    notes: &mut Notes,
) -> Result<Parsed, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|error| format!("Could not open the DOCX file: {error}"))?;
    let body_xml = match read_part(&mut archive, "word/document.xml") {
        Part_::Text(text) => text,
        Part_::Missing => return Err("This DOCX file has no word/document.xml".into()),
        Part_::TooLarge => {
            return Err(format!(
                "The document text is larger than {} MB once unpacked; refusing to read it",
                MAX_PART_BYTES / (1024 * 1024)
            ))
        }
    };
    // Styles and properties are optional: a part that is missing or too large
    // is skipped (headings then fall back on their built-in names).
    let styles = read_part(&mut archive, "word/styles.xml")
        .text()
        .map(|xml| Styles::parse(&xml))
        .unwrap_or_default();
    let (core_title, core_author) = read_part(&mut archive, "docProps/core.xml")
        .text()
        .map(|xml| core_properties(&xml))
        .unwrap_or_default();

    let document = parse_xml(&body_xml)
        .map_err(|error| format!("Could not read the DOCX body: {error}"))?;
    notes.dropped("table", count(&document, "tbl"));

    let mut reader = Reader {
        styles: &styles,
        medium,
        title_paragraph: None,
        footnotes: 0,
        images: 0,
        list_items: 0,
    };
    let mut parts: Vec<Part> = Vec::new();
    for paragraph in document.descendants().filter(|n| is(n, "p")) {
        if let Some(part) = reader.paragraph(&paragraph) {
            parts.push(part);
        }
    }
    let Reader { title_paragraph, footnotes, images, list_items, .. } = reader;
    notes.dropped("footnote or endnote", footnotes);
    notes.dropped("image", images);
    if list_items > 0 {
        notes.say("Bullets and numbering were not imported; list items became plain paragraphs");
    }

    let documents = split_documents(parts, medium, split);
    Ok(Parsed {
        title: title_paragraph.or(core_title),
        author: core_author,
        contact: None,
        draft_date: None,
        documents,
    })
}

fn is(node: &Node, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && node.tag_name().namespace().map_or(true, |ns| ns == W)
}

fn count(document: &roxmltree::Document, name: &str) -> usize {
    document.descendants().filter(|n| is(n, name)).count()
}

/// The outcome of reading one zip entry.
#[allow(non_camel_case_types)]
enum Part_ {
    Text(String),
    Missing,
    TooLarge,
}

impl Part_ {
    fn text(self) -> Option<String> {
        match self {
            Part_::Text(text) => Some(text),
            _ => None,
        }
    }
}

/// Read one entry, never more than `MAX_PART_BYTES` of *decompressed* output.
/// The size the zip header claims is the attacker's to choose, so the cap is
/// applied to the bytes that actually come out.
fn read_part(archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str) -> Part_ {
    let Ok(file) = archive.by_name(name) else { return Part_::Missing };
    let mut bytes = Vec::new();
    if file.take(MAX_PART_BYTES + 1).read_to_end(&mut bytes).is_err() {
        return Part_::Missing;
    }
    if bytes.len() as u64 > MAX_PART_BYTES {
        return Part_::TooLarge;
    }
    match String::from_utf8(bytes) {
        Ok(text) => Part_::Text(text),
        Err(_) => Part_::Missing,
    }
}

fn parse_xml(xml: &str) -> Result<roxmltree::Document<'_>, roxmltree::Error> {
    roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: MAX_XML_NODES,
            ..roxmltree::ParsingOptions::default()
        },
    )
}

fn attr<'a>(node: &Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attributes().find(|a| a.name() == name).map(|a| a.value())
}

fn child<'a, 'b>(node: &Node<'a, 'b>, name: &str) -> Option<Node<'a, 'b>> {
    node.children().find(|c| is(c, name))
}

fn core_properties(xml: &str) -> (Option<String>, Option<String>) {
    let Ok(doc) = parse_xml(xml) else { return (None, None) };
    let find = |name: &str| {
        doc.descendants()
            .find(|n| n.is_element() && n.tag_name().name() == name)
            .and_then(|n| n.text())
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
    };
    (find("title"), find("creator"))
}

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    Title,
    Heading(u8),
    Body,
}

#[derive(Default)]
struct Styles {
    /// id -> (display name, parent id, outline level)
    by_id: HashMap<String, (String, Option<String>, Option<u8>)>,
}

impl Styles {
    fn parse(xml: &str) -> Self {
        let mut styles = Styles::default();
        let Ok(doc) = parse_xml(xml) else { return styles };
        for style in doc.descendants().filter(|n| is(n, "style")) {
            if attr(&style, "type") != Some("paragraph") {
                continue;
            }
            let Some(id) = attr(&style, "styleId") else { continue };
            let name = child(&style, "name").and_then(|n| attr(&n, "val")).unwrap_or(id);
            let parent = child(&style, "basedOn").and_then(|n| attr(&n, "val")).map(str::to_string);
            let outline = child(&style, "pPr")
                .and_then(|p| child(&p, "outlineLvl"))
                .and_then(|o| attr(&o, "val"))
                .and_then(|v| v.parse::<u8>().ok());
            styles.by_id.insert(id.to_string(), (name.to_string(), parent, outline));
        }
        styles
    }

    /// The style's own display name, e.g. "heading 1" (always English in the
    /// file, whatever language Word shows).
    fn name(&self, id: &str) -> String {
        self.by_id.get(id).map_or_else(|| id.to_string(), |(name, _, _)| name.clone())
    }

    /// What a paragraph in this style is, following `basedOn` parents.
    fn role(&self, id: &str) -> Role {
        let mut current = Some(id.to_string());
        for _ in 0..8 {
            let Some(id) = current else { break };
            let (name, parent, outline) = match self.by_id.get(&id) {
                Some(entry) => entry.clone(),
                None => (id.clone(), None, None),
            };
            let lower = name.to_lowercase();
            if lower == "title" {
                return Role::Title;
            }
            if let Some(level) = lower.strip_prefix("heading ").and_then(|n| n.trim().parse::<u8>().ok()) {
                return Role::Heading(level.max(1));
            }
            if let Some(level) = outline.filter(|l| *l < 9) {
                return Role::Heading(level + 1);
            }
            current = parent;
        }
        Role::Body
    }
}

// ---------------------------------------------------------------------------
// Paragraphs
// ---------------------------------------------------------------------------

enum Part {
    Heading { level: u8, text: String },
    Paragraph(RichParagraph),
}

struct Reader<'a> {
    styles: &'a Styles,
    medium: Medium,
    title_paragraph: Option<String>,
    footnotes: usize,
    images: usize,
    list_items: usize,
}

impl Reader<'_> {
    fn paragraph(&mut self, node: &Node) -> Option<Part> {
        let properties = child(node, "pPr");
        let style_id = properties
            .and_then(|p| child(&p, "pStyle"))
            .and_then(|s| attr(&s, "val"))
            .unwrap_or_default();
        if properties.and_then(|p| child(&p, "numPr")).is_some() {
            self.list_items += 1;
        }

        let content = self.content(node);
        let text: String = content
            .iter()
            .map(|item| match item {
                InlineContent::Text(run) => run.text.as_str(),
                _ => " ",
            })
            .collect();
        if text.trim().is_empty() {
            return None;
        }

        match self.styles.role(style_id) {
            Role::Title => {
                if self.title_paragraph.is_none() {
                    self.title_paragraph = Some(text.trim().to_string());
                    return None; // exports add their own title page
                }
                Some(Part::Heading { level: 1, text: text.trim().to_string() })
            }
            Role::Heading(level) => Some(Part::Heading { level, text: text.trim().to_string() }),
            Role::Body => {
                let style = if self.medium.is_script() {
                    let kind = element_kind(&self.styles.name(style_id))
                        .unwrap_or_else(|| script::classify(&text));
                    script_style_id(kind)
                } else {
                    "body"
                };
                Some(Part::Paragraph(RichParagraph { style_id: style.into(), content }))
            }
        }
    }

    /// The paragraph's runs. Text boxes nest whole paragraphs inside runs, so a
    /// run belongs to the *nearest* enclosing paragraph only.
    fn content(&mut self, paragraph: &Node) -> Vec<InlineContent> {
        let mut out = Vec::new();
        for run in paragraph.descendants().filter(|n| is(n, "r")) {
            if run.ancestors().find(|a| is(a, "p")).map(|a| a.id()) != Some(paragraph.id()) {
                continue;
            }
            let marks = run_marks(&run);
            for piece in run.children().filter(Node::is_element) {
                match piece.tag_name().name() {
                    "t" => {
                        let text = piece.text().unwrap_or_default();
                        if !text.is_empty() {
                            out.push(InlineContent::Text(TextRun::with_marks(text, marks.clone())));
                        }
                    }
                    "tab" => out.push(InlineContent::Text(TextRun::with_marks("\t", marks.clone()))),
                    "noBreakHyphen" => out.push(InlineContent::Text(TextRun::with_marks("-", marks.clone()))),
                    // A page break is layout; a line break is the writer's.
                    "br" if attr(&piece, "type") != Some("page") => out.push(InlineContent::HardBreak),
                    "cr" => out.push(InlineContent::HardBreak),
                    "footnoteReference" | "endnoteReference" => self.footnotes += 1,
                    // A text box is a drawing that holds paragraphs, which are
                    // read as paragraphs of their own, so it is not a lost image.
                    "drawing" | "pict" | "object"
                        if !piece.descendants().any(|n| is(&n, "txbxContent")) =>
                    {
                        self.images += 1
                    }
                    _ => {}
                }
            }
        }
        out
    }
}

fn run_marks(run: &Node) -> Vec<InlineMark> {
    let mut marks: Vec<InlineMark> = Vec::new();
    let mut add = |mark: InlineMark| {
        if !marks.contains(&mark) {
            marks.push(mark);
        }
    };
    let Some(properties) = child(run, "rPr") else { return marks };
    for property in properties.children().filter(Node::is_element) {
        let enabled = attr(&property, "val")
            .map(|v| !matches!(v.to_ascii_lowercase().as_str(), "false" | "off" | "none" | "0"))
            .unwrap_or(true);
        if !enabled {
            continue;
        }
        match property.tag_name().name() {
            "b" => add(InlineMark::Bold),
            "i" => add(InlineMark::Italic),
            "u" => add(InlineMark::Underline),
            "strike" | "dstrike" => add(InlineMark::Strikethrough),
            // Character styles Word applies for Strong / Emphasis.
            "rStyle" => match attr(&property, "val").map(str::to_lowercase).as_deref() {
                Some("strong") => add(InlineMark::Bold),
                Some("emphasis") => add(InlineMark::Italic),
                _ => {}
            },
            _ => {}
        }
    }
    marks
}

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

/// Cut at the shallowest heading level present. Text before the first heading
/// becomes its own untitled document.
fn split_documents(parts: Vec<Part>, medium: Medium, split: Split) -> Vec<ImportedDocument> {
    let kind = if medium.is_script() { DocKind::Scene } else { DocKind::Chapter };
    let new_doc = |title: String| ImportedDocument {
        title,
        kind,
        medium,
        content: RichDocument::default(),
        synopsis: String::new(),
    };
    let cut_level = (split == Split::Auto)
        .then(|| {
            parts
                .iter()
                .filter_map(|p| match p {
                    Part::Heading { level, .. } => Some(*level),
                    _ => None,
                })
                .min()
        })
        .flatten();

    let mut documents: Vec<ImportedDocument> = Vec::new();
    let mut current = new_doc(String::new());
    for part in parts {
        match part {
            Part::Heading { level, text } if Some(level) == cut_level => {
                if !current.content.paragraphs.is_empty() || !current.title.is_empty() {
                    documents.push(current);
                }
                current = new_doc(text);
            }
            Part::Heading { text, .. } => current.content.paragraphs.push(RichParagraph {
                style_id: "chapter-heading".into(),
                content: vec![InlineContent::Text(TextRun::plain(text))],
            }),
            Part::Paragraph(paragraph) => current.content.paragraphs.push(paragraph),
        }
    }
    if !current.content.paragraphs.is_empty() || documents.is_empty() {
        documents.push(current);
    }
    documents
}
