use super::*;

/// One exportable section of the manuscript.
pub(crate) struct Section<'a> {
    pub(crate) doc: &'a Document,
    pub(crate) heading: String,
}

/// Whether a compiled document starts on a fresh page.
///
/// Prose chapters always do. Script documents follow the trade convention:
/// scenes run straight on from each other, and a new page starts only at a
/// chapter-level document (an episode or an act). Every writer asks this one
/// question, so the formats cannot disagree.
pub(crate) fn starts_new_page(doc: &Document) -> bool {
    !doc.medium.is_script() || doc.kind == DocKind::Chapter
}

/// A link as a writer's format may carry it: web and mail addresses only.
/// A `note:` link points inside the project and means nothing outside it, so
/// the text prints plain; anything else (a `javascript:` address in a file
/// from elsewhere, say) is never written out as a link.
pub(crate) fn export_href(link: &str) -> Option<&str> {
    let lower = link.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
        .then_some(link.trim())
}

/// `FADE IN:` opens a script flush left; every other transition is a
/// right-hand cue. The text is the writer's own; only its alignment is
/// chosen, and every writer asks here so none of them right-aligns the opening.
pub(crate) fn opens_script(kind: ElementKind, text: &str) -> bool {
    kind == ElementKind::Transition && text.trim_start().to_uppercase().starts_with("FADE IN")
}

/// The transition style, left-aligned, for an opening `FADE IN:`.
pub(crate) fn opening_transition_style(transition: &ElementStyle) -> ElementStyle {
    ElementStyle {
        align: Alignment::Left,
        ..transition.clone()
    }
}

pub(crate) fn rich_document_has_images(document: &RichDocument) -> bool {
    document.paragraphs.iter().any(|paragraph| paragraph.has_image())
}

pub(crate) fn manuscript_sections<'a>(project: &'a Project, settings: &ExportSettings) -> Vec<Section<'a>> {
    project
        .compile_plan(settings)
        .included
        .into_iter()
        .filter(|d| !matches!(d.kind, DocKind::FrontMatter | DocKind::BackMatter))
        .map(|doc| Section {
            heading: doc.title.clone(),
            doc,
        })
        .collect()
}

pub(crate) fn screenplay_blocks(project: &Project, doc: &Document) -> Vec<Block> {
    let sheet = project.style_sheet_for(doc);
    doc.content.to_script_blocks(&sheet)
}

/// One prose paragraph after its stylesheet has been resolved for output.
/// Paragraphs stay separate here rather than being flattened into one string:
/// the rich model's styles, inline marks and hard breaks all need to survive.
pub(crate) struct ProseParagraph {
    pub(crate) format: ElementStyle,
    pub(crate) content: Vec<InlineContent>,
    pub(crate) heading: bool,
    pub(crate) uppercase: bool,
    /// Which paragraph of the source document this is, so a laid-out line can
    /// point back at the text it came from. `None` for generated text
    /// (synopsis and note blocks).
    pub(crate) index: Option<usize>,
}

impl ProseParagraph {
    /// Text for plain formats: a hard break remains a newline.
    pub(crate) fn plain_text(&self) -> String {
        self.content
            .iter()
            .map(|item| match item {
                InlineContent::Text(run) => self.transform(&run.text),
                InlineContent::HardBreak => "\n".to_owned(),
                InlineContent::Image(image) if image.alt_text.is_empty() => {
                    "[Image]".to_owned()
                }
                InlineContent::Image(image) => {
                    format!("[Image: {}]", self.transform(&image.alt_text))
                }
            })
            .collect()
    }

    pub(crate) fn transform(&self, text: &str) -> String {
        if self.uppercase {
            text.to_uppercase()
        } else {
            text.to_owned()
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        !self
            .content
            .iter()
            .any(|item| match item {
                InlineContent::Text(run) => !run.text.trim().is_empty(),
                InlineContent::Image(_) => true,
                InlineContent::HardBreak => false,
            })
    }
}

/// Resolve a paragraph style for output. A format's saved element style wins
/// only when it deliberately differs from the project style; otherwise the
/// project stylesheet is authoritative, so an appearance edit reaches every
/// writer even before the Compile screen has saved a per-format copy.
pub(crate) fn prose_style(
    project: &Project,
    settings: &ExportSettings,
    style: Option<&ParagraphStyle>,
) -> ElementStyle {
    let Some(style) = style else {
        return settings.styles.body.clone();
    };
    let mut format = project.paragraph_format_for(style, &settings.styles);
    if format.font.trim().is_empty() {
        format.font = settings.styles.body.font.clone();
    }
    if format.font.trim().is_empty() {
        format.font = settings.font.clone();
    }
    format
}

/// Every visible paragraph of a prose document, in order, with its effective
/// output style. This is the single reading path the prose writers share.
pub(crate) fn prose_paragraphs(
    project: &Project,
    doc: &Document,
    settings: &ExportSettings,
) -> Vec<ProseParagraph> {
    let sheet = project.style_sheet_for(doc);
    doc.content
        .paragraphs
        .iter()
        .enumerate()
        .filter_map(|(index, paragraph)| {
            let style = sheet.style(&paragraph.style_id);
            let format = prose_style(project, settings, style);
            let name = style
                .map(|style| style.name.to_ascii_lowercase())
                .unwrap_or_default();
            let heading = paragraph.style_id == "chapter-heading" || name.contains("heading");
            let uppercase = style
                .map(|style| style.text_transform == TextTransform::Uppercase)
                .unwrap_or(false)
                || format.uppercase;
            let resolved = ProseParagraph {
                format,
                content: paragraph.content.clone(),
                heading,
                uppercase,
                index: Some(index),
            };
            (!resolved.is_empty()).then_some(resolved)
        })
        .collect()
}

pub(crate) fn front_matter<'a>(project: &'a Project, settings: &ExportSettings) -> Vec<&'a Document> {
    project
        .compile_plan(settings)
        .included
        .into_iter()
        .filter(|d| d.kind == DocKind::FrontMatter)
        .collect()
}

pub(crate) fn back_matter<'a>(project: &'a Project, settings: &ExportSettings) -> Vec<&'a Document> {
    project
        .compile_plan(settings)
        .included
        .into_iter()
        .filter(|d| d.kind == DocKind::BackMatter)
        .collect()
}

pub(crate) fn section_title(_project: &Project, doc: &Document, index: usize, settings: &ExportSettings) -> String {
    if !settings.include_chapter_numbers {
        return doc.title.clone();
    }
    if doc.kind == DocKind::Chapter {
        format!("Chapter {}", index + 1)
    } else if doc.kind == DocKind::Scene {
        doc.title.clone()
    } else {
        doc.title.clone()
    }
}


/// The entries of a Contents list: one title per compiled section, numbered
/// the way the chapter headings are. Writers that cannot know page numbers
/// (DOCX, RTF, plain text) print this as a simple list.
pub(crate) fn contents_entries(project: &Project, settings: &ExportSettings) -> Vec<String> {
    manuscript_sections(project, settings)
        .iter()
        .enumerate()
        .map(|(index, section)| section_title(project, section.doc, index, settings))
        .collect()
}

/// Running scene numbers across one script compile. One implementation so
/// every writer numbers the same scenes the same way ("1   EXT. HARBOUR").
#[derive(Default)]
pub(crate) struct SceneNumbers {
    seen: u32,
}

impl SceneNumbers {
    /// `text` for a script element, prefixed with the next scene number when
    /// it is a scene heading and numbering is on. The gap is non-breaking so
    /// HTML keeps it.
    pub(crate) fn apply(&mut self, kind: ElementKind, text: String, enabled: bool) -> String {
        if enabled && kind == ElementKind::SceneHeading {
            self.seen += 1;
            format!("{}\u{a0}\u{a0}\u{a0}{}", self.seen, text)
        } else {
            text
        }
    }
}
