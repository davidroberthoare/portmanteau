//! Shared rich-text content primitives for every long-form writing surface.
//!
//! Paragraphs retain semantic style ids, but they are data boundaries rather
//! than independent editing widgets. The editor presents the complete
//! `RichDocument` as one flow so selection and cursor movement can cross them.

use serde::{Deserialize, Serialize};
use std::ops::Range;

use crate::{ElementKind, ElementStyle, ElementStyles, Medium};

/// How a paragraph's stored text is presented. This never mutates the
/// canonical text (for example, a character name remains "Mara Venn" while the
/// screenplay Character style displays it as "MARA VENN").
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextTransform {
    #[default]
    None,
    Uppercase,
}

/// A named paragraph style. `semantic_kind` is an interchange hint for
/// screenplay formats; prose styles leave it unset.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ParagraphStyle {
    pub id: String,
    pub name: String,
    pub format: ElementStyle,
    #[serde(default)]
    pub semantic_kind: Option<ElementKind>,
    #[serde(default)]
    pub next_style: Option<String>,
    #[serde(default)]
    pub text_transform: TextTransform,
}

/// Page margins in points (72 per inch) for a script format.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct PageMargins {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Default for PageMargins {
    /// Film and TV: 1.5" binding edge, 1" elsewhere.
    fn default() -> Self {
        Self {
            left: 108.0,
            right: 72.0,
            top: 72.0,
            bottom: 72.0,
        }
    }
}

/// A project-owned set of paragraph styles for one writing mode.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StyleSheet {
    #[serde(default)]
    pub margins: PageMargins,
    pub id: String,
    pub name: String,
    pub medium: Medium,
    pub default_style: String,
    pub styles: Vec<ParagraphStyle>,
}

impl StyleSheet {
    pub fn style(&self, id: &str) -> Option<&ParagraphStyle> {
        self.styles.iter().find(|style| style.id == id)
    }

    /// Novel manuscript defaults. Additional styles can be added and edited
    /// in the project without changing the program-wide template.
    pub fn novel_prose(defaults: &ElementStyles) -> Self {
        Self {
            margins: PageMargins::default(),
            id: "prose".into(),
            name: "Novel / Prose".into(),
            medium: Medium::Prose,
            default_style: "body".into(),
            styles: vec![
                ParagraphStyle {
                    id: "body".into(),
                    name: "Body".into(),
                    format: defaults.body.clone(),
                    semantic_kind: None,
                    next_style: Some("body".into()),
                    text_transform: TextTransform::None,
                },
                ParagraphStyle {
                    id: "chapter-heading".into(),
                    name: "Chapter Heading".into(),
                    format: defaults.chapter_heading.clone(),
                    semantic_kind: None,
                    next_style: Some("body".into()),
                    text_transform: TextTransform::None,
                },
                ParagraphStyle {
                    id: "image".into(),
                    name: "Image".into(),
                    format: ElementStyle {
                        align: crate::Alignment::Center,
                        before: 6.0,
                        after: 6.0,
                        ..defaults.body.clone()
                    },
                    semantic_kind: None,
                    next_style: Some("image-caption".into()),
                    text_transform: TextTransform::None,
                },
                ParagraphStyle {
                    id: "image-caption".into(),
                    name: "Image Caption".into(),
                    format: ElementStyle {
                        align: crate::Alignment::Center,
                        size: 9.0,
                        ..defaults.body.clone()
                    },
                    semantic_kind: None,
                    next_style: Some("body".into()),
                    text_transform: TextTransform::None,
                },
            ],
        }
    }

    /// Feature-film screenplay defaults with editable next-style rules.
    pub fn film_screenplay(defaults: &ElementStyles) -> Self {
        Self::script_sheet(
            "film-screenplay",
            "Film Screenplay",
            Medium::Screenplay,
            defaults,
            &|kind| defaults.style_for_kind(kind).clone(),
        )
    }

    /// One-hour drama: film layout plus centred act headings (TEASER, ACT ONE).
    pub fn tv_script(defaults: &ElementStyles) -> Self {
        Self::script_sheet(
            "tv-script",
            "TV One-Hour Drama",
            Medium::TvScript,
            defaults,
            &|kind| match kind {
                ElementKind::Act => ElementStyle {
                    before: 24.0,
                    after: 12.0,
                    align: crate::Alignment::Center,
                    uppercase: true,
                    ..defaults.action.clone()
                },
                _ => defaults.style_for_kind(kind).clone(),
            },
        )
    }

    /// Samuel French stage manuscript: centred cues, full-width dialogue,
    /// italic stage directions set in from the left. Indents are measured from
    /// the left text margin.
    pub fn stage_play(defaults: &ElementStyles) -> Self {
        let base = defaults.action.clone();
        Self::script_sheet(
            "stage-play",
            "Stage Play (Samuel French)",
            Medium::StagePlay,
            defaults,
            &|kind| match kind {
                ElementKind::Act => ElementStyle {
                    before: 24.0,
                    after: 12.0,
                    align: crate::Alignment::Center,
                    uppercase: true,
                    ..base.clone()
                },
                ElementKind::Scene => ElementStyle {
                    before: 12.0,
                    after: 12.0,
                    align: crate::Alignment::Center,
                    uppercase: true,
                    ..base.clone()
                },
                ElementKind::StageDirection => ElementStyle {
                    indent: 108.0,
                    before: 12.0,
                    italic: true,
                    ..base.clone()
                },
                ElementKind::Character => ElementStyle {
                    before: 12.0,
                    align: crate::Alignment::Center,
                    uppercase: true,
                    ..base.clone()
                },
                ElementKind::Parenthetical => ElementStyle {
                    indent: 108.0,
                    right_indent: 108.0,
                    italic: true,
                    ..base.clone()
                },
                ElementKind::Dialogue => base.clone(),
                _ => base.clone(),
            },
        )
    }

    fn script_sheet(
        id: &str,
        name: &str,
        medium: Medium,
        defaults: &ElementStyles,
        format_for: &dyn Fn(ElementKind) -> ElementStyle,
    ) -> Self {
        let kinds = ElementKind::all(medium);
        let default_kind = medium.default_element();
        let known: Vec<&'static str> = kinds.iter().map(|k| script_style_id(*k)).collect();
        let default_id = script_style_id(default_kind);
        let mut styles: Vec<ParagraphStyle> = kinds
            .into_iter()
            .map(|kind| {
                let next = default_next_style(kind)
                    .filter(|next| known.contains(next))
                    .unwrap_or(default_id);
                ParagraphStyle {
                    id: script_style_id(kind).to_owned(),
                    name: kind.label().to_owned(),
                    format: format_for(kind),
                    semantic_kind: Some(kind),
                    next_style: Some(next.to_owned()),
                    text_transform: if kind.is_uppercase() {
                        TextTransform::Uppercase
                    } else {
                        TextTransform::None
                    },
                }
            })
            .collect();
        let body = format_for(default_kind);
        styles.push(ParagraphStyle {
            id: "image".into(),
            name: "Image".into(),
            format: ElementStyle {
                align: crate::Alignment::Center,
                before: 6.0,
                after: 6.0,
                ..defaults.action.clone()
            },
            semantic_kind: None,
            next_style: Some("image-caption".into()),
            text_transform: TextTransform::None,
        });
        styles.push(ParagraphStyle {
            id: "image-caption".into(),
            name: "Image Caption".into(),
            format: ElementStyle {
                align: crate::Alignment::Center,
                size: 9.0,
                ..body
            },
            semantic_kind: None,
            next_style: Some(default_id.to_owned()),
            text_transform: TextTransform::None,
        });
        Self {
            margins: if medium == Medium::StagePlay {
                PageMargins {
                    left: 72.0,
                    ..PageMargins::default()
                }
            } else {
                PageMargins::default()
            },
            id: id.into(),
            name: name.into(),
            medium,
            default_style: default_id.into(),
            styles,
        }
    }

    /// The built-in sheet for a document medium.
    pub fn for_medium(medium: Medium, defaults: &ElementStyles) -> Self {
        match medium {
            Medium::Screenplay => Self::film_screenplay(defaults),
            Medium::TvScript => Self::tv_script(&ElementStyles::default()),
            Medium::StagePlay => Self::stage_play(&ElementStyles::default()),
            Medium::Prose | Medium::Note => Self::novel_prose(defaults),
        }
    }

    /// TV and stage sheets own their formatting; the shared element styles
    /// only mirror prose and film.
    pub fn is_independent_script_sheet(id: &str) -> bool {
        matches!(id, "tv-script" | "stage-play")
    }

    pub fn industry_defaults(defaults: &ElementStyles) -> Vec<Self> {
        vec![
            Self::novel_prose(defaults),
            Self::film_screenplay(defaults),
            Self::tv_script(&ElementStyles::default()),
            Self::stage_play(&ElementStyles::default()),
        ]
    }

    /// Append built-in sheets missing from `sheets`, leaving existing ones alone.
    pub fn ensure_industry_defaults(sheets: &mut Vec<Self>, defaults: &ElementStyles) {
        for sheet in Self::industry_defaults(defaults) {
            if !sheets.iter().any(|existing| existing.id == sheet.id) {
                sheets.push(sheet);
            }
        }
    }
}

pub fn script_style_id(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::SceneHeading => "scene-heading",
        ElementKind::Action => "action",
        ElementKind::Character => "character",
        ElementKind::Parenthetical => "parenthetical",
        ElementKind::Dialogue => "dialogue",
        ElementKind::Transition => "transition",
        ElementKind::Shot => "shot",
        ElementKind::Act => "act",
        ElementKind::Scene => "scene",
        ElementKind::StageDirection => "stage-direction",
        ElementKind::Lyric => "lyric",
        ElementKind::General => "general",
        ElementKind::Extension => "extension",
        ElementKind::Intercut => "intercut",
        ElementKind::Subheading => "subheading",
    }
}

pub fn script_kind_for_style_id(style_id: &str) -> Option<ElementKind> {
    Some(match style_id {
        "scene-heading" => ElementKind::SceneHeading,
        "action" => ElementKind::Action,
        "character" => ElementKind::Character,
        "parenthetical" => ElementKind::Parenthetical,
        "dialogue" => ElementKind::Dialogue,
        "transition" => ElementKind::Transition,
        "shot" => ElementKind::Shot,
        "act" => ElementKind::Act,
        "scene" => ElementKind::Scene,
        "stage-direction" => ElementKind::StageDirection,
        "lyric" => ElementKind::Lyric,
        "general" => ElementKind::General,
        "extension" => ElementKind::Extension,
        "intercut" => ElementKind::Intercut,
        "subheading" => ElementKind::Subheading,
        _ => return None,
    })
}


fn default_next_style(kind: ElementKind) -> Option<&'static str> {
    match kind {
        ElementKind::SceneHeading => Some("action"),
        ElementKind::Action => Some("action"),
        ElementKind::Character => Some("dialogue"),
        ElementKind::Parenthetical => Some("dialogue"),
        ElementKind::Dialogue => Some("character"),
        ElementKind::Transition => Some("scene-heading"),
        ElementKind::Shot => Some("action"),
        ElementKind::Act => Some("scene"),
        ElementKind::Scene => Some("stage-direction"),
        ElementKind::StageDirection => Some("character"),
        ElementKind::Lyric => Some("character"),
        ElementKind::General => Some("action"),
        ElementKind::Extension => Some("dialogue"),
        ElementKind::Intercut => Some("action"),
        ElementKind::Subheading => Some("action"),
    }
}

/// One ordered writing flow. Paragraph boundaries carry style, not separate
/// document identity or editor focus.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct RichDocument {
    pub paragraphs: Vec<RichParagraph>,
}

/// A paragraph styled by a project stylesheet.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RichParagraph {
    pub style_id: String,
    pub content: Vec<InlineContent>,
}

impl RichParagraph {
    pub fn empty(style_id: impl Into<String>) -> Self {
        Self {
            style_id: style_id.into(),
            content: Vec::new(),
        }
    }

    pub fn plain_text(&self) -> String {
        self.text_only()
    }

    /// Replace the visible text in this paragraph, preserving its paragraph
    /// style. This is used for simple value edits such as accepting a character
    /// name; rich range edits use `RichDocument::replace_range`.
    pub fn set_plain_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.content = if text.is_empty() {
            Vec::new()
        } else {
            vec![InlineContent::Text(TextRun::plain(text))]
        };
    }

    /// Text represented by this paragraph, using object-replacement markers
    /// for images so editor positions remain one logical character per object.
    pub fn editor_text(&self) -> String {
        let mut text = String::new();
        for item in &self.content {
            match item {
                InlineContent::Text(run) => text.push_str(&run.text),
                InlineContent::HardBreak => text.push('\n'),
                InlineContent::Image(_) => text.push('\u{FFFC}'),
            }
        }
        text
    }

    /// Plain text for search and word counts. Image alt text is deliberately
    /// separate from visible manuscript text; an export can decide whether to
    /// use it as a placeholder and warn about the conversion.
    pub fn text_only(&self) -> String {
        let mut text = String::new();
        for item in &self.content {
            match item {
                InlineContent::Text(run) => text.push_str(&run.text),
                InlineContent::HardBreak => text.push('\n'),
                InlineContent::Image(_) => {}
            }
        }
        text
    }

    pub fn has_image(&self) -> bool {
        self.content
            .iter()
            .any(|item| matches!(item, InlineContent::Image(_)))
    }
}

/// Inline content is shared by prose, scripts, notes, synopsis and research
/// text. A standalone image is represented by a paragraph containing an image
/// object; an inline image can sit among text runs.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InlineContent {
    Text(TextRun),
    HardBreak,
    Image(ImageObject),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct TextRun {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<InlineMark>,
}

impl TextRun {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            marks: Vec::new(),
        }
    }

    pub fn with_marks(text: impl Into<String>, marks: Vec<InlineMark>) -> Self {
        Self {
            text: text.into(),
            marks,
        }
    }
}

/// Inline character formatting. Paragraph-level appearance belongs to the
/// paragraph's stylesheet entry.
#[derive(
    Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug,
)]
#[serde(rename_all = "snake_case")]
pub enum InlineMark {
    Bold,
    Italic,
    Underline,
    Strikethrough,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ImageObject {
    /// Project-relative asset identifier, not an arbitrary filesystem path.
    pub asset_id: String,
    #[serde(default)]
    pub alt_text: String,
    #[serde(default)]
    pub caption: String,
    /// Optional display dimensions in points. Missing values use the editor's
    /// fitted default while the original asset remains unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
}

impl RichDocument {
    pub fn empty(style_id: impl Into<String>) -> Self {
        Self {
            paragraphs: vec![RichParagraph::empty(style_id)],
        }
    }

    /// Build styled paragraphs from plain text. Newlines are paragraph
    /// boundaries, including consecutive and trailing empty paragraphs.
    pub fn from_plain_text(text: &str, style_id: &str) -> Self {
        let paragraphs = text
            .split('\n')
            .map(|line| {
                let mut paragraph = RichParagraph::empty(style_id);
                if !line.is_empty() {
                    paragraph
                        .content
                        .push(InlineContent::Text(TextRun::plain(line)));
                }
                paragraph
            })
            .collect();
        Self { paragraphs }
    }

    pub fn set_plain_text(&mut self, text: impl Into<String>, style_id: &str) {
        let text = text.into();
        *self = Self::from_plain_text(&text, style_id);
    }

    pub fn from_script_blocks(blocks: &[crate::Block]) -> Self {
        let paragraphs = blocks
            .iter()
            .map(|block| {
                let mut paragraph = RichParagraph::empty(script_style_id(block.kind));
                if !block.text.is_empty() {
                    paragraph
                        .content
                        .push(InlineContent::Text(TextRun::plain(&block.text)));
                }
                paragraph
            })
            .collect();
        Self { paragraphs }
    }

    pub fn from_fountain(text: &str, medium: Medium) -> Self {
        let blocks = crate::script::parse_fountain(text, medium);
        Self::from_script_blocks(&blocks)
    }

    /// Convert screenplay paragraphs back to the semantic element list used
    /// by Fountain and FDX writers. Unknown custom styles fall back to General
    /// until an import/export mapping profile supplies a semantic mapping.
    pub fn to_script_blocks(&self, sheet: &StyleSheet) -> Vec<crate::Block> {
        self.paragraphs
            .iter()
            .map(|paragraph| {
                let kind = sheet
                    .style(&paragraph.style_id)
                    .and_then(|style| style.semantic_kind)
                    .or_else(|| script_kind_for_style_id(&paragraph.style_id))
                    .unwrap_or(ElementKind::General);
                crate::Block::new(kind, paragraph_plain_export_text(paragraph))
            })
            .collect()
    }

    pub fn to_fountain(&self, sheet: &StyleSheet, medium: Medium) -> String {
        let blocks = self.to_script_blocks(sheet);
        crate::script::to_fountain(&blocks, medium)
    }

    /// Flatten the flow to text for search, word counts and plain-text
    /// interchange. Images contribute their alt text, or a generic marker.
    pub fn plain_text(&self) -> String {
        self.paragraphs
            .iter()
            .map(|paragraph| {
                paragraph
                    .content
                    .iter()
                    .map(|item| match item {
                        InlineContent::Text(run) => run.text.clone(),
                        InlineContent::HardBreak => "\n".to_owned(),
                        InlineContent::Image(image) if image.alt_text.is_empty() => {
                            "[Image]".to_owned()
                        }
                        InlineContent::Image(image) => {
                            format!("[Image: {}]", image.alt_text)
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Flatten only actual text, retaining paragraph and hard line breaks but
    /// excluding image objects and their metadata from word counts.
    pub fn text_only(&self) -> String {
        self.paragraphs
            .iter()
            .map(RichParagraph::text_only)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A single position space for the future editor: paragraph boundaries
    /// become newlines and image objects occupy one object-replacement
    /// character, matching a rich-text editor's logical cursor model.
    pub fn editor_text(&self) -> String {
        self.paragraphs
            .iter()
            .map(RichParagraph::editor_text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Number of caret positions in the complete flow, counting paragraph
    /// boundaries and inline objects as one character each.
    pub fn char_count(&self) -> usize {
        self.editor_text().chars().count()
    }

    /// Convert a document-wide character offset into a paragraph-local
    /// position. Offsets are Unicode character indices, not UTF-8 bytes.
    pub fn position_at(&self, char_index: usize) -> TextPosition {
        let mut remaining = char_index.min(self.char_count());
        for (index, paragraph) in self.paragraphs.iter().enumerate() {
            let len = paragraph.editor_text().chars().count();
            if remaining <= len {
                return TextPosition {
                    paragraph: index,
                    offset: remaining,
                };
            }
            remaining -= len;
            if index + 1 < self.paragraphs.len() {
                // The paragraph separator is a real caret step. The position
                // immediately after it is the start of the next paragraph.
                if remaining == 1 {
                    return TextPosition {
                        paragraph: index + 1,
                        offset: 0,
                    };
                }
                remaining -= 1;
            }
        }
        let last = self.paragraphs.len().saturating_sub(1);
        TextPosition {
            paragraph: last,
            offset: self
                .paragraphs
                .get(last)
                .map(|paragraph| paragraph.editor_text().chars().count())
                .unwrap_or(0),
        }
    }

    /// Replace a selection expressed in the one document-wide character
    /// space. Paragraphs inside the selection are joined/deleted, while text
    /// and image objects outside it retain their inline formatting.
    ///
    /// Newlines in inserted text create paragraphs using each style's
    /// `next_style` rule; an unset or invalid rule repeats the current style.
    pub fn replace_range(
        &mut self,
        range: Range<usize>,
        inserted: &str,
        sheet: &StyleSheet,
    ) -> usize {
        self.replace_range_with_marks(range, inserted, &[], sheet)
    }

    pub fn replace_range_with_marks(
        &mut self,
        range: Range<usize>,
        inserted: &str,
        marks: &[InlineMark],
        sheet: &StyleSheet,
    ) -> usize {
        if self.paragraphs.is_empty() {
            self.paragraphs = Self::empty(sheet.default_style.clone()).paragraphs;
        }
        let max = self.char_count();
        let start = range.start.min(max);
        let end = range.end.max(start).min(max);
        let start_pos = self.position_at(start);
        let end_pos = self.position_at(end);
        let original = std::mem::take(&mut self.paragraphs);
        let start_paragraph = &original[start_pos.paragraph];
        let end_paragraph = &original[end_pos.paragraph];
        let (left, _) = split_content(&start_paragraph.content, start_pos.offset);
        let (_, right) = split_content(&end_paragraph.content, end_pos.offset);
        let first_style = start_paragraph.style_id.clone();

        let parts: Vec<&str> = inserted.split('\n').collect();
        let mut output = Vec::with_capacity(
            start_pos.paragraph + parts.len() + original.len() - end_pos.paragraph,
        );
        output.extend(original[..start_pos.paragraph].iter().cloned());

        let mut current_style = first_style.clone();
        let mut first_content = left;
        append_text(&mut first_content, parts[0], marks);
        if parts.len() == 1 {
            append_content(&mut first_content, right);
            output.push(RichParagraph {
                style_id: first_style,
                content: first_content,
            });
        } else {
            output.push(RichParagraph {
                style_id: current_style.clone(),
                content: first_content,
            });
            for part in parts.iter().skip(1).take(parts.len() - 2) {
                current_style = next_style_id(sheet, &current_style);
                let mut content = Vec::new();
                append_text(&mut content, part, marks);
                output.push(RichParagraph {
                    style_id: current_style.clone(),
                    content,
                });
            }
            current_style = next_style_id(sheet, &current_style);
            let mut final_content = Vec::new();
            append_text(&mut final_content, parts[parts.len() - 1], marks);
            append_content(&mut final_content, right);
            output.push(RichParagraph {
                style_id: current_style,
                content: final_content,
            });
        }

        output.extend(
            original
                .iter()
                .skip(end_pos.paragraph + 1)
                .cloned(),
        );
        if output.is_empty() {
            output.push(RichParagraph::empty(sheet.default_style.clone()));
        }
        self.paragraphs = output;
        start + inserted.chars().count()
    }

    /// Toggle an inline formatting mark over a document-wide character range.
    /// Paragraph styles and image objects are left intact.
    pub fn toggle_mark(&mut self, range: Range<usize>, mark: InlineMark) -> bool {
        let start = range.start.min(range.end).min(self.char_count());
        let end = range.start.max(range.end).min(self.char_count());
        if start == end {
            return false;
        }
        let mut changed = false;
        let mut document_offset = 0usize;
        for paragraph in &mut self.paragraphs {
            let paragraph_len = content_char_count(&paragraph.content);
            let local_start = start.saturating_sub(document_offset).min(paragraph_len);
            let local_end = end.saturating_sub(document_offset).min(paragraph_len);
            if local_start < local_end {
                let (left_and_middle, right) = split_content(&paragraph.content, local_end);
                let (left, middle) = split_content(&left_and_middle, local_start);
                let (middle, did_change) = toggle_content_mark(middle, mark);
                if did_change {
                    let mut content = left;
                    append_content(&mut content, middle);
                    append_content(&mut content, right);
                    paragraph.content = content;
                    changed = true;
                }
            }
            document_offset += paragraph_len + 1;
        }
        changed
    }

    /// Insert an image object, optionally as its own paragraph. The object
    /// stores only a project asset id and accessibility/print metadata.
    pub fn insert_image(
        &mut self,
        range: Range<usize>,
        image: ImageObject,
        standalone: bool,
        sheet: &StyleSheet,
    ) -> usize {
        let insertion = range.start.min(range.end).min(self.char_count());
        if range.start != range.end {
            self.replace_range(range, "", sheet);
        }
        let position = self.position_at(insertion);
        if !standalone {
            let paragraph = &mut self.paragraphs[position.paragraph];
            let (left, right) = split_content(&paragraph.content, position.offset);
            let mut content = left;
            content.push(InlineContent::Image(image));
            append_content(&mut content, right);
            paragraph.content = content;
            return insertion + 1;
        }

        let original = std::mem::take(&mut self.paragraphs);
        let paragraph = &original[position.paragraph];
        let (left, right) = split_content(&paragraph.content, position.offset);
        let mut output = original[..position.paragraph].to_vec();
        if !left.is_empty() {
            output.push(RichParagraph {
                style_id: paragraph.style_id.clone(),
                content: left,
            });
        }
        output.push(RichParagraph {
            style_id: if sheet.style("image").is_some() {
                "image".into()
            } else {
                paragraph.style_id.clone()
            },
            content: vec![InlineContent::Image(image.clone())],
        });
        if !image.caption.is_empty() {
            output.push(RichParagraph {
                style_id: if sheet.style("image-caption").is_some() {
                    "image-caption".into()
                } else {
                    paragraph.style_id.clone()
                },
                content: vec![InlineContent::Text(TextRun::plain(&image.caption))],
            });
        }
        output.push(RichParagraph {
            style_id: paragraph.style_id.clone(),
            content: right,
        });
        output.extend(original.iter().skip(position.paragraph + 1).cloned());
        self.paragraphs = output;
        insertion + 1
    }

    /// Reconcile the value edited by a single text widget with this rich
    /// document. A common-prefix/suffix diff limits the replacement to the
    /// changed span, preserving paragraph styles, marks and images elsewhere.
    pub fn sync_editor_text(&mut self, new_text: &str, sheet: &StyleSheet) {
        self.sync_editor_text_with_caret(new_text, sheet, None);
    }

    /// Like `sync_editor_text`, but resolves an ambiguous diff with the caret
    /// the editor reports *after* the edit. Inserting "\n" before an existing
    /// "\n" (or deleting one of two equal characters) produces identical text
    /// whichever character was meant, and a plain prefix/suffix diff picks the
    /// wrong one: the new paragraph then splits the *following* paragraph and
    /// takes its style. The caret says where the edit really happened.
    pub fn sync_editor_text_with_caret(
        &mut self,
        new_text: &str,
        sheet: &StyleSheet,
        caret_after: Option<usize>,
    ) {
        let old: Vec<char> = self.editor_text().chars().collect();
        let new: Vec<char> = new_text.chars().collect();
        if let Some(caret) = caret_after {
            if new.len() > old.len() {
                let added = new.len() - old.len();
                if caret >= added && caret <= new.len() {
                    let start = caret - added;
                    if old[..start] == new[..start] && old[start..] == new[caret..] {
                        let inserted: String = new[start..caret].iter().collect();
                        self.replace_range(start..start, &inserted, sheet);
                        return;
                    }
                }
            } else if new.len() < old.len() {
                let removed = old.len() - new.len();
                if caret <= new.len()
                    && old[..caret] == new[..caret]
                    && old[caret + removed..] == new[caret..]
                {
                    self.replace_range(caret..caret + removed, "", sheet);
                    return;
                }
            }
        }
        let mut prefix = 0;
        while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
            prefix += 1;
        }
        let mut suffix = 0;
        while suffix < old.len().saturating_sub(prefix)
            && suffix < new.len().saturating_sub(prefix)
            && old[old.len() - suffix - 1] == new[new.len() - suffix - 1]
        {
            suffix += 1;
        }
        let inserted: String = new[prefix..new.len() - suffix].iter().collect();
        self.replace_range(prefix..old.len() - suffix, &inserted, sheet);
    }

    pub fn is_empty(&self) -> bool {
        self.paragraphs.iter().all(|paragraph| {
            paragraph
                .content
                .iter()
                .all(|item| matches!(item, InlineContent::Text(run) if run.text.is_empty()))
        })
    }
}

fn paragraph_plain_export_text(paragraph: &RichParagraph) -> String {
    paragraph
        .content
        .iter()
        .map(|item| match item {
            InlineContent::Text(run) => run.text.clone(),
            InlineContent::HardBreak => "\n".to_owned(),
            InlineContent::Image(image) if image.alt_text.is_empty() => {
                "[Image]".to_owned()
            }
            InlineContent::Image(image) => format!("[Image: {}]", image.alt_text),
        })
        .collect::<String>()
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TextPosition {
    pub paragraph: usize,
    pub offset: usize,
}

fn next_style_id(sheet: &StyleSheet, current: &str) -> String {
    sheet
        .style(current)
        .and_then(|style| style.next_style.as_deref())
        .filter(|next| sheet.style(next).is_some())
        .unwrap_or(current)
        .to_owned()
}

fn content_char_count(content: &[InlineContent]) -> usize {
    content
        .iter()
        .map(|item| match item {
            InlineContent::Text(run) => run.text.chars().count(),
            InlineContent::HardBreak | InlineContent::Image(_) => 1,
        })
        .sum()
}

/// Split inline nodes at a Unicode character offset without losing marks or
/// image metadata.
fn split_content(
    content: &[InlineContent],
    char_offset: usize,
) -> (Vec<InlineContent>, Vec<InlineContent>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut remaining = char_offset.min(content_char_count(content));
    let mut split_done = false;

    for item in content {
        if split_done {
            right.push(item.clone());
            continue;
        }
        let len = match item {
            InlineContent::Text(run) => run.text.chars().count(),
            InlineContent::HardBreak | InlineContent::Image(_) => 1,
        };
        if remaining >= len {
            left.push(item.clone());
            remaining -= len;
            if remaining == 0 {
                split_done = true;
            }
            continue;
        }

        match item {
            InlineContent::Text(run) => {
                let split_byte = run
                    .text
                    .char_indices()
                    .nth(remaining)
                    .map(|(byte, _)| byte)
                    .unwrap_or(run.text.len());
                if split_byte > 0 {
                    left.push(InlineContent::Text(TextRun {
                        text: run.text[..split_byte].to_owned(),
                        marks: run.marks.clone(),
                    }));
                }
                if split_byte < run.text.len() {
                    right.push(InlineContent::Text(TextRun {
                        text: run.text[split_byte..].to_owned(),
                        marks: run.marks.clone(),
                    }));
                }
            }
            InlineContent::HardBreak | InlineContent::Image(_) => {
                right.push(item.clone());
            }
        }
        split_done = true;
    }

    (left, right)
}

fn append_text(content: &mut Vec<InlineContent>, text: &str, marks: &[InlineMark]) {
    if text.is_empty() {
        return;
    }
    if let Some(InlineContent::Text(previous)) = content.last_mut() {
        if previous.marks == marks {
            previous.text.push_str(text);
            return;
        }
    }
    content.push(InlineContent::Text(TextRun::with_marks(
        text,
        marks.to_vec(),
    )));
}

fn append_content(target: &mut Vec<InlineContent>, mut additional: Vec<InlineContent>) {
    for item in additional.drain(..) {
        match item {
            InlineContent::Text(run) => append_text(target, &run.text, &run.marks),
            other => target.push(other),
        }
    }
}

fn toggle_content_mark(
    mut content: Vec<InlineContent>,
    mark: InlineMark,
) -> (Vec<InlineContent>, bool) {
    let all_marked = content
        .iter()
        .filter_map(|item| match item {
            InlineContent::Text(run) if !run.text.is_empty() => Some(run.marks.contains(&mark)),
            _ => None,
        })
        .all(|marked| marked);
    let mut changed = false;
    for item in &mut content {
        if let InlineContent::Text(run) = item {
            if all_marked {
                let before = run.marks.len();
                run.marks.retain(|existing| *existing != mark);
                changed |= before != run.marks.len();
            } else if !run.marks.contains(&mark) && !run.text.is_empty() {
                run.marks.push(mark);
                run.marks.sort();
                changed = true;
            }
        }
    }
    (content, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_creates_styled_paragraphs_and_preserves_empty_lines() {
        let document = RichDocument::from_plain_text("one\n\nthree\n", "prose.body");
        assert_eq!(
            document
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.style_id.as_str())
                .collect::<Vec<_>>(),
            vec!["prose.body"; 4]
        );
        assert_eq!(document.plain_text(), "one\n\nthree\n");
        assert_eq!(document.editor_text(), "one\n\nthree\n");
    }

    #[test]
    fn inline_marks_are_attached_to_runs_without_changing_text() {
        let document = RichDocument {
            paragraphs: vec![RichParagraph {
                style_id: "prose.body".into(),
                content: vec![
                    InlineContent::Text(TextRun::plain("plain ")),
                    InlineContent::Text(TextRun::with_marks(
                        "emphasis",
                        vec![InlineMark::Bold, InlineMark::Italic],
                    )),
                ],
            }],
        };
        assert_eq!(document.plain_text(), "plain emphasis");
        assert_eq!(document.paragraphs[0].content.len(), 2);
        assert_eq!(
            match &document.paragraphs[0].content[1] {
                InlineContent::Text(run) => run.marks.clone(),
                _ => unreachable!(),
            },
            vec![InlineMark::Bold, InlineMark::Italic]
        );
    }

    #[test]
    fn images_have_an_editor_position_and_a_plain_text_placeholder() {
        let document = RichDocument {
            paragraphs: vec![RichParagraph {
                style_id: "prose.body".into(),
                content: vec![
                    InlineContent::Text(TextRun::plain("before")),
                    InlineContent::Image(ImageObject {
                        asset_id: "images/map-1.png".into(),
                        alt_text: "Harbor map".into(),
                        caption: "The old harbor".into(),
                        width: Some(240.0),
                        height: None,
                    }),
                    InlineContent::Text(TextRun::plain("after")),
                ],
            }],
        };
        assert_eq!(document.editor_text(), "before\u{FFFC}after");
        assert_eq!(
            document.plain_text(),
            "before[Image: Harbor map]after"
        );
        assert_eq!(document.text_only(), "beforeafter");
    }

    #[test]
    fn image_in_a_paragraph_without_text_can_represent_a_standalone_image() {
        let paragraph = RichParagraph {
            style_id: "image".into(),
            content: vec![InlineContent::Image(ImageObject {
                asset_id: "images/scene.png".into(),
                alt_text: String::new(),
                caption: "Scene reference".into(),
                width: None,
                height: None,
            })],
        };
        assert!(paragraph.has_image());
        assert_eq!(paragraph.editor_text(), "\u{FFFC}");
    }

    #[test]
    fn standalone_image_insertion_adds_an_image_paragraph_and_caption() {
        let mut document = RichDocument::from_plain_text("before after", "body");
        let sheet = StyleSheet::novel_prose(&ElementStyles::default());
        document.insert_image(
            7..7,
            ImageObject {
                asset_id: "images/harbor.png".into(),
                alt_text: "Harbor at low tide".into(),
                caption: "The harbor, 1987".into(),
                width: None,
                height: None,
            },
            true,
            &sheet,
        );
        assert_eq!(
            document
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.style_id.as_str())
                .collect::<Vec<_>>(),
            vec!["body", "image", "image-caption", "body"]
        );
        assert_eq!(
            document.text_only(),
            "before \n\nThe harbor, 1987\nafter"
        );
        assert_eq!(document.paragraphs[1].editor_text(), "\u{FFFC}");
        assert_eq!(document.paragraphs[2].plain_text(), "The harbor, 1987");
    }

    #[test]
    fn fountain_roundtrip_preserves_screenplay_structure() {
        let blocks = vec![
            crate::Block::new(ElementKind::SceneHeading, "INT. HARBOR - DAWN"),
            crate::Block::new(ElementKind::Action, "The lamp swings over the chart."),
            crate::Block::new(ElementKind::Character, "MARA"),
            crate::Block::new(ElementKind::Dialogue, "We missed the tide."),
            crate::Block::new(ElementKind::Transition, "CUT TO:"),
        ];
        let sheet = StyleSheet::film_screenplay(&ElementStyles::default());
        let document = RichDocument::from_script_blocks(&blocks);
        let fountain = document.to_fountain(&sheet, Medium::Screenplay);
        let reparsed = RichDocument::from_fountain(&fountain, Medium::Screenplay);
        let kinds: Vec<ElementKind> = reparsed
            .to_script_blocks(&sheet)
            .iter()
            .map(|block| block.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                ElementKind::SceneHeading,
                ElementKind::Action,
                ElementKind::Character,
                ElementKind::Dialogue,
                ElementKind::Transition,
            ]
        );
    }

    #[test]
    fn tv_and_stage_sheets_are_independent_formats() {
        let sheets = StyleSheet::industry_defaults(&ElementStyles::default());
        let tv = sheets.iter().find(|s| s.id == "tv-script").unwrap();
        let stage = sheets.iter().find(|s| s.id == "stage-play").unwrap();
        assert_eq!(tv.style("act").unwrap().format.align, crate::Alignment::Center);
        assert_eq!(tv.style("dialogue").unwrap().format.indent, 72.0);
        assert_eq!(stage.style("dialogue").unwrap().format.indent, 0.0);
        assert_eq!(stage.style("character").unwrap().format.align, crate::Alignment::Center);
        assert!(stage.style("stage-direction").unwrap().format.italic);
        assert_eq!(stage.default_style, "stage-direction");
        assert_eq!(stage.style("act").unwrap().next_style.as_deref(), Some("scene"));

        // Editing the shared film styles must not touch TV or stage.
        let mut styles = ElementStyles::default();
        styles.update_from_sheets(&sheets);
        assert_eq!(styles.dialogue.indent, 72.0);
        assert_eq!(styles.character.align, crate::Alignment::Left);

        let mut old = vec![StyleSheet::film_screenplay(&styles)];
        StyleSheet::ensure_industry_defaults(&mut old, &styles);
        assert_eq!(old.len(), 4);
    }

    #[test]
    fn industry_stylesheets_have_next_style_rules_and_canonical_character_case() {
        let defaults = ElementStyles::default();
        let sheets = StyleSheet::industry_defaults(&defaults);
        assert_eq!(sheets.len(), 4);
        assert_eq!(sheets[0].id, "prose");
        assert_eq!(sheets[0].default_style, "body");

        let screenplay = &sheets[1];
        assert_eq!(screenplay.id, "film-screenplay");
        assert_eq!(
            screenplay
                .style("character")
                .and_then(|style| style.next_style.as_deref()),
            Some("dialogue")
        );
        let character = screenplay.style("character").unwrap();
        assert_eq!(character.text_transform, TextTransform::Uppercase);
        assert_eq!(character.semantic_kind, Some(ElementKind::Character));
        assert_eq!(character.format.indent, 158.4);
        assert_eq!(screenplay.style("dialogue").unwrap().format.indent, 72.0);
        assert_eq!(screenplay.style("dialogue").unwrap().format.right_indent, 108.0);
        assert_eq!(
            screenplay.style("parenthetical").unwrap().format.indent,
            115.2
        );
        assert_eq!(
            screenplay
                .style("parenthetical")
                .unwrap()
                .format
                .right_indent,
            136.8
        );
        assert_eq!(
            screenplay.style("transition").unwrap().format.align,
            crate::Alignment::Right
        );
        for id in ["extension", "intercut", "subheading", "shot"] {
            assert!(
                screenplay.style(id).is_some(),
                "screenplay defaults should contain {id}"
            );
        }
    }

    #[test]
    fn document_positions_cross_paragraph_boundaries() {
        let document = RichDocument::from_plain_text("abc\ndef", "body");
        assert_eq!(document.position_at(0), TextPosition { paragraph: 0, offset: 0 });
        assert_eq!(document.position_at(3), TextPosition { paragraph: 0, offset: 3 });
        assert_eq!(document.position_at(4), TextPosition { paragraph: 1, offset: 0 });
        assert_eq!(document.position_at(7), TextPosition { paragraph: 1, offset: 3 });
    }

    #[test]
    fn replacement_can_select_across_styled_paragraphs_and_preserve_unselected_marks() {
        let mut document = RichDocument {
            paragraphs: vec![
                RichParagraph {
                    style_id: "character".into(),
                    content: vec![InlineContent::Text(TextRun::with_marks(
                        "Mara",
                        vec![InlineMark::Italic],
                    ))],
                },
                RichParagraph {
                    style_id: "dialogue".into(),
                    content: vec![InlineContent::Text(TextRun::plain("says hello"))],
                },
            ],
        };
        let sheet = StyleSheet::film_screenplay(&ElementStyles::default());

        // Select "ra\nsa" across the Character/Dialogue boundary.
        let caret = document.replace_range(2..7, "X", &sheet);

        assert_eq!(caret, 3);
        assert_eq!(document.paragraphs.len(), 1);
        assert_eq!(document.paragraphs[0].style_id, "character");
        assert_eq!(document.editor_text(), "MaXys hello");
        match &document.paragraphs[0].content[0] {
            InlineContent::Text(run) => {
                assert_eq!(run.text, "Ma");
                assert_eq!(run.marks, vec![InlineMark::Italic]);
            }
            _ => panic!("the unselected marked text should remain first"),
        }
    }

    #[test]
    fn inserted_new_paragraphs_follow_the_stylesheet_next_style_rule() {
        let mut document = RichDocument {
            paragraphs: vec![RichParagraph {
                style_id: "character".into(),
                content: vec![InlineContent::Text(TextRun::plain("Mara Venn"))],
            }],
        };
        let sheet = StyleSheet::film_screenplay(&ElementStyles::default());
        document.replace_range(9..9, "\nHello.", &sheet);

        assert_eq!(
            document
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.style_id.as_str())
                .collect::<Vec<_>>(),
            vec!["character", "dialogue"]
        );
        assert_eq!(document.editor_text(), "Mara Venn\nHello.");
    }

    #[test]
    fn enter_before_an_empty_paragraph_splits_the_caret_paragraph() {
        let sheet = StyleSheet::film_screenplay(&ElementStyles::default());
        let mut document = RichDocument {
            paragraphs: vec![
                RichParagraph {
                    style_id: "character".into(),
                    content: vec![InlineContent::Text(TextRun::plain("Elias"))],
                },
                RichParagraph::empty("scene-heading"),
            ],
        };
        // Enter at the end of "Elias": text goes "Elias\n" -> "Elias\n\n".
        document.sync_editor_text_with_caret("Elias\n\n", &sheet, Some(6));
        let styles: Vec<&str> = document
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.style_id.as_str())
            .collect();
        assert_eq!(styles, vec!["character", "dialogue", "scene-heading"]);
    }

    #[test]
    fn deleting_an_image_object_does_not_consume_adjacent_text() {
        let mut document = RichDocument {
            paragraphs: vec![RichParagraph {
                style_id: "body".into(),
                content: vec![
                    InlineContent::Text(TextRun::plain("A")),
                    InlineContent::Image(ImageObject {
                        asset_id: "images/a.png".into(),
                        alt_text: "A".into(),
                        caption: String::new(),
                        width: None,
                        height: None,
                    }),
                    InlineContent::Text(TextRun::plain("B")),
                ],
            }],
        };
        let sheet = StyleSheet::novel_prose(&ElementStyles::default());
        document.replace_range(1..2, "", &sheet);
        assert_eq!(document.editor_text(), "AB");
    }

    #[test]
    fn inline_marks_toggle_across_paragraph_boundaries() {
        let mut document = RichDocument::from_plain_text("one\ntwo", "body");
        assert!(document.toggle_mark(1..5, InlineMark::Bold));
        let marked: Vec<bool> = document
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.content.iter())
            .filter_map(|item| match item {
                InlineContent::Text(run) => Some(run.marks.contains(&InlineMark::Bold)),
                _ => None,
            })
            .collect();
        assert_eq!(marked, vec![false, true, true, false]);
        assert!(document.toggle_mark(1..5, InlineMark::Bold));
        assert!(document
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.content.iter())
            .all(|item| match item {
                InlineContent::Text(run) => !run.marks.contains(&InlineMark::Bold),
                _ => true,
            }));
    }
}
