//! Compilation and export.
//!
//! One manuscript model, many finish lines: print-ready PDF, reflowable EPUB,
//! editor-friendly DOCX, Final Draft FDX, Fountain, HTML, Markdown, RTF and
//! plain text — plus a project bundle for local archiving and backup.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use portmanteau_core::script;
use portmanteau_core::*;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

mod pdf;
pub use pdf::{
    pdf_layout, pdf_manuscript, pdf_page_count, text_width, Face, Family, PdfLayout,
    PdfLayoutSection, PdfPageLayout, PdfSegment, PdfText,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportInfo {
    pub path: String,
    pub bytes: usize,
    pub words: u32,
    pub format: String,
    pub message: String,
    /// Every file written; more than one when compiling scripts separately.
    #[serde(default)]
    pub files: Vec<String>,
}

/// One exportable section of the manuscript.
struct Section<'a> {
    doc: &'a Document,
    heading: String,
}

pub fn default_filename(project: &Project, kind: ExportKind) -> String {
    let base: String = project
        .title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() { "manuscript" } else { &base };
    format!("{}.{}", base, kind.extension())
}

/// Compile the manuscript. One export is one kind of manuscript (see
/// `CompilePlan`); with `each_separately` a script compile writes one file per
/// script.
pub fn export(
    project: &Project,
    settings: &ExportSettings,
    output: Option<&Path>,
) -> Result<ExportInfo, String> {
    let plan = project.compile_plan(settings);
    let mut settings = settings.clone();
    settings.compile_type = Some(plan.kind);
    if plan.kind.is_script() {
        settings.include_toc = false;
        settings.include_chapter_numbers = false;
    }
    let settings = &settings;
    if settings.each_separately && plan.kind.is_script() {
        let groups = project.compile_groups(&plan);
        if !groups.is_empty() {
            let first = output_path(project, settings, output);
            let dir = first.parent().map(Path::to_path_buf).unwrap_or_default();
            let stem = first
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "manuscript".into());
            let (mut files, mut bytes, mut words) = (Vec::new(), 0usize, 0u32);
            for group in &groups {
                let mut each = settings.clone();
                each.scope = portmanteau_core::CompileScope::Documents(group.ids.clone());
                each.each_separately = false;
                let name = format!("{stem}-{}.{}", slugify(&group.title), settings.kind.extension());
                let info = export_single(project, &each, Some(&dir.join(name)))?;
                bytes += info.bytes;
                words += info.words;
                files.extend(info.files);
            }
            return Ok(ExportInfo {
                path: dir.display().to_string(),
                bytes,
                words,
                format: settings.kind.label().to_string(),
                message: format!(
                    "{} exported · {} separate {} files",
                    settings.kind.label(),
                    files.len(),
                    plan.kind.label().to_lowercase()
                ),
                files,
            });
        }
    }
    export_single(project, settings, output)
}

fn output_path(project: &Project, settings: &ExportSettings, output: Option<&Path>) -> PathBuf {
    match output {
        Some(p) => p.to_path_buf(),
        None => {
            let name = default_filename(project, settings.kind);
            if settings.output_path.trim().is_empty() {
                PathBuf::from("exports").join(name)
            } else {
                PathBuf::from(settings.output_path.trim()).join(name)
            }
        }
    }
}

fn export_single(
    project: &Project,
    settings: &ExportSettings,
    output: Option<&Path>,
) -> Result<ExportInfo, String> {
    let path = output_path(project, settings, output);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not make folder: {e}"))?;
        }
    }

    let bytes: Vec<u8> = match settings.kind {
        ExportKind::Pdf => pdf_manuscript(project, settings),
        ExportKind::Epub => epub_manuscript(project, settings),
        ExportKind::Docx => docx_manuscript(project, settings),
        ExportKind::Fdx => fdx_manuscript(project, settings).into_bytes(),
        ExportKind::Fountain => fountain_manuscript(project, settings).into_bytes(),
        ExportKind::Html => html_manuscript(project, settings).into_bytes(),
        ExportKind::Markdown => markdown_manuscript(project, settings).into_bytes(),
        ExportKind::Rtf => rtf_manuscript(project, settings).into_bytes(),
        ExportKind::Txt => txt_manuscript(project, settings).into_bytes(),
        ExportKind::Bundle => bundle_manuscript(project)?,
    };

    fs::write(&path, &bytes).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    let plan = project.compile_plan(settings);
    let has_images = plan.included.iter().any(|doc| {
        rich_document_has_images(&doc.content)
            || (settings.include_synopsis && rich_document_has_images(&doc.synopsis))
            || (settings.include_notes && rich_document_has_images(&doc.notes))
    });
    // The PDF's own layout is the authority for its page count; the other
    // writers still use the quick per-document estimate.
    let pages: u32 = if settings.kind == ExportKind::Pdf {
        pdf_page_count(project, settings) as u32
    } else {
        plan.included.iter().map(|d| d.pages()).sum()
    };
    let page_word = if pages == 1 { "page" } else { "pages" };
    let message = if has_images {
        format!(
            "{} exported · {pages} {page_word} estimated · Images represented by alt-text placeholders; source assets are not embedded",
            settings.kind.label(),
        )
    } else {
        format!(
            "{} exported · {pages} {page_word} estimated",
            settings.kind.label()
        )
    };
    Ok(ExportInfo {
        path: path.display().to_string(),
        bytes: bytes.len(),
        words: plan.included.iter().map(|d| d.words()).sum(),
        format: settings.kind.label().to_string(),
        message,
        files: vec![path.display().to_string()],
    })
}

fn rich_document_has_images(document: &RichDocument) -> bool {
    document.paragraphs.iter().any(|paragraph| paragraph.has_image())
}

// ---------------------------------------------------------------------------
// Shared manuscript assembly
// ---------------------------------------------------------------------------

fn manuscript_sections<'a>(project: &'a Project, settings: &ExportSettings) -> Vec<Section<'a>> {
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

fn screenplay_blocks(project: &Project, doc: &Document) -> Vec<Block> {
    let sheet = project.style_sheet_for(doc);
    doc.content.to_script_blocks(&sheet)
}

/// One prose paragraph after its stylesheet has been resolved for output.
/// Paragraphs stay separate here rather than being flattened into one string:
/// the rich model's styles, inline marks and hard breaks all need to survive.
struct ProseParagraph {
    format: ElementStyle,
    content: Vec<InlineContent>,
    heading: bool,
    uppercase: bool,
}

impl ProseParagraph {
    /// Text for plain formats: a hard break remains a newline.
    fn plain_text(&self) -> String {
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

    fn transform(&self, text: &str) -> String {
        if self.uppercase {
            text.to_uppercase()
        } else {
            text.to_owned()
        }
    }

    fn is_empty(&self) -> bool {
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
fn prose_style(
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
fn prose_paragraphs(
    project: &Project,
    doc: &Document,
    settings: &ExportSettings,
) -> Vec<ProseParagraph> {
    let sheet = project.style_sheet_for(doc);
    doc.content
        .paragraphs
        .iter()
        .filter_map(|paragraph| {
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
            };
            (!resolved.is_empty()).then_some(resolved)
        })
        .collect()
}

fn front_matter<'a>(project: &'a Project, settings: &ExportSettings) -> Vec<&'a Document> {
    project
        .compile_plan(settings)
        .included
        .into_iter()
        .filter(|d| d.kind == DocKind::FrontMatter)
        .collect()
}

fn back_matter<'a>(project: &'a Project, settings: &ExportSettings) -> Vec<&'a Document> {
    project
        .compile_plan(settings)
        .included
        .into_iter()
        .filter(|d| d.kind == DocKind::BackMatter)
        .collect()
}

fn section_title(_project: &Project, doc: &Document, index: usize, settings: &ExportSettings) -> String {
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

fn paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.replace('\n', " "))
        .collect()
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// CSS declarations that carry one paragraph's resolved appearance. Inline
/// declarations keep EPUB/XHTML output correct even when two project sheets
/// define a style with the same id.
fn prose_inline_style(format: &ElementStyle) -> String {
    // The declarations live inside a double-quoted attribute, so family names
    // use CSS single quotes rather than the double quotes from the shared
    // stylesheet helper.
    let family = html_escape(&element_css_font(format, "Liberation Serif").replace('"', "'"));
    format!(
        "font-family: {family}; font-size: {:.1}pt; line-height: {:.2}; text-align: {}; margin-left: {:.1}pt; margin-right: {:.1}pt; text-indent: {:.1}pt; margin-top: {:.1}pt; margin-bottom: {:.1}pt;",
        format.size.max(8.0),
        format.leading.max(1.0),
        css_align(format.align),
        format.indent.max(0.0),
        format.right_indent.max(0.0),
        format.first_line_indent,
        format.before.max(0.0),
        format.after.max(0.0),
    )
}

fn html_runs(paragraph: &ProseParagraph) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                let mut text = html_escape(&paragraph.transform(&run.text));
                if run.marks.contains(&InlineMark::Italic) {
                    text = format!("<em>{text}</em>");
                }
                if run.marks.contains(&InlineMark::Bold) {
                    text = format!("<strong>{text}</strong>");
                }
                if run.marks.contains(&InlineMark::Underline) {
                    text = format!("<u>{text}</u>");
                }
                if run.marks.contains(&InlineMark::Strikethrough) {
                    text = format!("<s>{text}</s>");
                }
                out.push_str(&text);
            }
            InlineContent::HardBreak => out.push_str("<br/>"),
            InlineContent::Image(image) if image.alt_text.is_empty() => out.push_str("[Image]"),
            InlineContent::Image(image) => out.push_str(&html_escape(&format!(
                "[Image: {}]",
                paragraph.transform(&image.alt_text)
            ))),
        }
    }
    out
}

fn html_prose_paragraph(paragraph: &ProseParagraph, first: bool) -> String {
    let mut style = prose_inline_style(&paragraph.format);
    if first {
        style.push_str("text-indent: 0;");
    }
    let runs = html_runs(paragraph);
    if paragraph.heading {
        format!("<h2 style=\"{style}\">{runs}</h2>\n")
    } else {
        format!("<p style=\"{style}\">{runs}</p>\n")
    }
}

// ---------------------------------------------------------------------------
// PDF
// ---------------------------------------------------------------------------

/// A CSS font stack built from the settings family with a safe serif fallback.
fn css_font_family(family: &str) -> String {
    let safe = family.replace('"', "").replace('\\', "");
    format!("\"{}\", serif", safe)
}

/// CSS family for one element, falling back when its family is unset.
fn element_css_font(style: &ElementStyle, fallback: &str) -> String {
    let family = if style.font.trim().is_empty() {
        fallback
    } else {
        style.font.as_str()
    };
    css_font_family(family)
}

/// CSS left margin: a point indent converted to characters, or the old percentage.
fn element_css_indent(style: &ElementStyle, fallback: &str) -> String {
    if style.indent > 0.0 {
        format!("{:.2}ch", style.indent / 7.2)
    } else {
        fallback.to_string()
    }
}

/// CSS keyword for an element's paragraph alignment.
fn css_align(align: Alignment) -> &'static str {
    match align {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "justify",
    }
}

/// CSS declarations for left/right indents, empty when both are zero.
fn css_indent_margins(style: &ElementStyle) -> String {
    let mut out = String::new();
    if style.indent > 0.0 {
        out.push_str(&format!("margin-left: {:.2}pt; ", style.indent));
    }
    if style.right_indent > 0.0 {
        out.push_str(&format!("margin-right: {:.2}pt; ", style.right_indent));
    }
    out
}

fn font_is_mono(family: &str) -> bool {
    let family = family.to_ascii_lowercase();
    family.contains("courier") || family.contains("mono")
}

// ---------------------------------------------------------------------------
// EPUB
// ---------------------------------------------------------------------------

/// Resolve an optional EPUB cover. A missing or unreadable file is not an
/// export error: the book is still valid without its cover.
fn cover_image(settings: &ExportSettings) -> Option<(String, &'static str, Vec<u8>)> {
    let raw = settings.cover_image.trim();
    if raw.is_empty() {
        return None;
    }
    let path = Path::new(raw);
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let media = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        _ => return None,
    };
    let bytes = fs::read(path).ok()?;
    Some((ext, media, bytes))
}

fn epub_manuscript(project: &Project, settings: &ExportSettings) -> Vec<u8> {
    let mut buf = Vec::new();
    let cover = cover_image(settings);
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        if let Some((ext, _, bytes)) = &cover {
            zip.start_file(format!("OEBPS/cover-image/cover.{}", ext), deflated)
                .unwrap();
            zip.write_all(bytes).unwrap();
        }

        zip.start_file("META-INF/container.xml", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/style.css", deflated).unwrap();
        let st = &settings.styles;
        let body_font = element_css_font(&st.body, "Liberation Serif");
        let scene_font = element_css_font(&st.scene_heading, "Courier New");
        let action_font = element_css_font(&st.action, "Courier New");
        let character_font = element_css_font(&st.character, "Courier New");
        let parenthetical_font = element_css_font(&st.parenthetical, "Courier New");
        let dialogue_font = element_css_font(&st.dialogue, "Courier New");
        let transition_font = element_css_font(&st.transition, "Courier New");
        let character_indent = element_css_indent(&st.character, "32%");
        let parenthetical_indent = element_css_indent(&st.parenthetical, "26%");
        let dialogue_indent = element_css_indent(&st.dialogue, "18%");
        let body_align = css_align(st.body.align);
        let heading_align = css_align(st.chapter_heading.align);
        let body_margins = css_indent_margins(&st.body);
        let heading_margins = css_indent_margins(&st.chapter_heading);
        let title_align = css_align(st.title_page_title.align);
        let subtitle_align = css_align(st.title_page_subtitle.align);
        let author_align = css_align(st.title_page_author.align);
        let body_first_indent = st.body.first_line_indent.max(0.0);
        let css = format!(
            r#"body {{ font-family: {body_font}; line-height: {body_leading:.2}; margin: 5%; {body_margins}text-align: {body_align}; color: #1b1b1b; }}
h1 {{ font-size: {chapter_size:.1}pt; margin: 2em 0 1em; page-break-before: always; text-align: {heading_align}; {heading_margins}}}
h2 {{ font-size: 1.25em; margin: 1.5em 0 .8em; text-align: {heading_align}; }}
p {{ margin: 0 0 0 .2em; text-indent: {body_first_indent:.2}pt; text-align: {body_align}; }}
p.first {{ text-indent: 0; }}
.center {{ text-align: center; text-indent: 0; }}
.scene {{ font-family: {scene_font}; font-size: {scene_size:.1}pt; font-weight: bold; margin-top: 1.5em; text-indent: 0; }}
.action {{ font-family: {action_font}; font-size: {action_size:.1}pt; margin: .5em 0 .8em; text-indent: 0; }}
.character {{ font-family: {character_font}; font-size: {character_size:.1}pt; margin: 1em 0 0 {character_indent}; text-indent: 0; }}
.parenthetical {{ font-family: {parenthetical_font}; font-size: {parenthetical_size:.1}pt; margin: 0 0 0 {parenthetical_indent}; text-indent: 0; }}
.dialogue {{ font-family: {dialogue_font}; font-size: {dialogue_size:.1}pt; margin: 0 10% .7em {dialogue_indent}; text-indent: 0; }}
.transition {{ font-family: {transition_font}; font-size: {transition_size:.1}pt; text-align: right; text-indent: 0; }}
.title-page {{ text-align: center; margin-top: 26%; }}
.title-page h1 {{ page-break-before: avoid; font-size: {title_size:.1}pt; text-align: {title_align}; }}
.title-page .subtitle {{ font-size: {subtitle_size:.1}pt; text-align: {subtitle_align}; }}
.byline {{ font-size: {author_size:.1}pt; margin-top: 1.5em; text-align: {author_align}; }}
.imprint {{ margin-top: 4em; font-size: .85em; color: #555; }}
.cover {{ text-align: center; margin: 0; padding: 0; }}
.cover img {{ max-width: 100%; }}
"#,
            body_font = body_font,
            body_leading = st.body.leading.max(1.0),
            body_align = body_align,
            body_margins = body_margins,
            body_first_indent = body_first_indent,
            heading_align = heading_align,
            heading_margins = heading_margins,
            title_align = title_align,
            subtitle_align = subtitle_align,
            author_align = author_align,
            chapter_size = st.chapter_heading.size.max(8.0),
            scene_font = scene_font,
            scene_size = st.scene_heading.size.max(8.0),
            action_font = action_font,
            action_size = st.action.size.max(8.0),
            character_font = character_font,
            character_size = st.character.size.max(8.0),
            character_indent = character_indent,
            parenthetical_font = parenthetical_font,
            parenthetical_size = st.parenthetical.size.max(8.0),
            parenthetical_indent = parenthetical_indent,
            dialogue_font = dialogue_font,
            dialogue_size = st.dialogue.size.max(8.0),
            dialogue_indent = dialogue_indent,
            transition_font = transition_font,
            transition_size = st.transition.size.max(8.0),
            title_size = st.title_page_title.size.max(8.0),
            subtitle_size = st.title_page_subtitle.size.max(8.0),
            author_size = st.title_page_author.size.max(8.0),
        );
        zip.write_all(css.as_bytes()).unwrap();

        let mut manifest = String::new();
        let mut spine = String::new();
        let mut toc_entries = String::new();
        let mut metadata_extra = String::new();
        let mut files: Vec<(String, String, String)> = Vec::new();

        if let Some((ext, media, _)) = &cover {
            let body = format!(
                "<div class=\"cover\"><img src=\"cover-image/cover.{}\" alt=\"cover\"/></div>",
                ext
            );
            let xhtml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"en\">\n<head><title>Cover</title><link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/></head>\n<body class=\"front\">\n{}\n</body>\n</html>\n",
                body
            );
            zip.start_file("OEBPS/cover.xhtml", deflated).unwrap();
            zip.write_all(xhtml.as_bytes()).unwrap();
            manifest.push_str(
                "<item id=\"cover\" href=\"cover.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
            );
            manifest.push_str(&format!(
                "<item id=\"cover-img\" href=\"cover-image/cover.{}\" media-type=\"{}\" properties=\"cover-image\"/>\n",
                ext, media
            ));
            spine.push_str("<itemref idref=\"cover\"/>\n");
            metadata_extra.push_str("<meta name=\"cover\" content=\"cover-img\"/>\n");
        }

        if settings.include_title_page {
            let body = format!(
                "<div class=\"title-page\"><h1>{}</h1><p class=\"center subtitle\">{}</p><p class=\"byline\">by {}</p><p class=\"imprint\">{}</p></div>",
                xml_escape(&project.title),
                xml_escape(&project.subtitle),
                xml_escape(&project.author),
                xml_escape(&project.genre)
            );
            files.push(("titlepage.xhtml".into(), "Title page".into(), body));
        }

        let mut chapter_no = 0usize;
        for doc in front_matter(project, settings) {
            files.push((
                format!("front-{}.xhtml", slugify(&doc.title)),
                doc.title.clone(),
                xhtml_body(project, doc, None, settings),
            ));
        }
        for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
            chapter_no += 1;
            let title = section_title(project, section.doc, i, settings);
            files.push((
                format!("chapter-{:03}.xhtml", chapter_no),
                title.clone(),
                xhtml_body(project, section.doc, Some(&title), settings),
            ));
        }
        for doc in back_matter(project, settings) {
            files.push((
                format!("back-{}.xhtml", slugify(&doc.title)),
                doc.title.clone(),
                xhtml_body(project, doc, None, settings),
            ));
        }

        for (i, (file, title, body)) in files.iter().enumerate() {
            zip.start_file(format!("OEBPS/{}", file), deflated).unwrap();
            let xhtml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"en\">\n<head><title>{}</title><link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/></head>\n<body class=\"front\">\n{}\n</body>\n</html>\n",
                xml_escape(title),
                body
            );
            zip.write_all(xhtml.as_bytes()).unwrap();
            manifest.push_str(&format!(
                "<item id=\"f{}\" href=\"{}\" media-type=\"application/xhtml+xml\"/>\n",
                i, file
            ));
            spine.push_str(&format!("<itemref idref=\"f{}\"/>\n", i));
            if i > 0 {
                toc_entries.push_str(&format!(
                    "<li><a href=\"{}\">{}</a></li>\n",
                    file,
                    xml_escape(title)
                ));
            }
        }

        // Navigation document.
        let nav = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n<head><title>Contents</title></head><body>\n<nav epub:type=\"toc\" id=\"toc\"><h1>Contents</h1><ol>{}</ol></nav>\n</body></html>",
            toc_entries
        );
        zip.start_file("OEBPS/nav.xhtml", deflated).unwrap();
        zip.write_all(nav.as_bytes()).unwrap();
        manifest.push_str("<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n");
        manifest.push_str("<item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\n");

        let modified = chrono::Local::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let opf = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\">\n<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n<dc:identifier id=\"bookid\">{}</dc:identifier>\n<dc:title>{}</dc:title>\n<dc:creator>{}</dc:creator>\n<dc:language>en</dc:language>\n<dc:description>{}</dc:description>\n<meta property=\"dcterms:modified\">{}</meta>\n{}</metadata>\n<manifest>\n{}</manifest>\n<spine>\n{}</spine>\n</package>\n",
            xml_escape(&project.id),
            xml_escape(&project.title),
            xml_escape(&project.author),
            xml_escape(&project.logline),
            modified,
            metadata_extra,
            manifest,
            spine
        );
        zip.start_file("OEBPS/content.opf", deflated).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    buf
}

fn xhtml_body(
    project: &Project,
    doc: &Document,
    heading: Option<&str>,
    settings: &ExportSettings,
) -> String {
    let mut out = String::new();
    if let Some(h) = heading {
        out.push_str(&format!("<h1>{}</h1>\n", xml_escape(h)));
    } else {
        out.push_str(&format!("<h2>{}</h2>\n", xml_escape(&doc.title)));
    }
    if doc.medium.is_script() {
        let blocks = screenplay_blocks(project, doc);
        for block in &blocks {
            let class = match block.kind {
                ElementKind::SceneHeading => "scene",
                ElementKind::Action | ElementKind::Shot => "action",
                ElementKind::Character | ElementKind::Extension => "character",
                ElementKind::Parenthetical => "parenthetical",
                ElementKind::Dialogue => "dialogue",
                ElementKind::Transition => "transition",
                ElementKind::Act => "scene",
                ElementKind::Scene => "scene",
                ElementKind::Intercut => "trans",
                ElementKind::Subheading => "scene",
                ElementKind::StageDirection => "action",
                ElementKind::Lyric => "dialogue",
                ElementKind::General => "action",
            };
            let text = if settings.styles.style_for_kind(block.kind).uppercase {
                block.text.to_uppercase()
            } else {
                block.text.clone()
            };
            out.push_str(&format!(
                "<p class=\"{}\">{}</p>\n",
                class,
                html_escape(&text)
            ));
        }
    } else {
        for (i, para) in prose_paragraphs(project, doc, settings).iter().enumerate() {
            out.push_str(&html_prose_paragraph(para, i == 0));
        }
    }
    out
}

/// Page width and height in twentieths of a point, the unit DOCX and RTF use.
fn paper_twips(settings: &ExportSettings) -> (i64, i64) {
    let (w_mm, h_mm) = settings.paper.mm();
    (
        (w_mm as f64 * 1440.0 / 25.4).round() as i64,
        (h_mm as f64 * 1440.0 / 25.4).round() as i64,
    )
}

// ---------------------------------------------------------------------------
// DOCX
// ---------------------------------------------------------------------------

fn docx_align(align: Alignment) -> Option<&'static str> {
    match align {
        Alignment::Left => None,
        Alignment::Center => Some("center"),
        Alignment::Right => Some("right"),
        Alignment::Justify => Some("both"),
    }
}

fn docx_text_run(text: &str, properties: &str) -> String {
    format!(
        "<w:r><w:rPr>{properties}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r>",
        xml_escape(text)
    )
}

fn docx_run_properties(
    paragraph: &ProseParagraph,
    marks: &[InlineMark],
    fallback_font: &str,
) -> String {
    let mut properties = String::new();
    let font = if paragraph.format.font.trim().is_empty() {
        fallback_font
    } else {
        &paragraph.format.font
    };
    if !font.trim().is_empty() {
        let font = xml_escape(font);
        properties.push_str(&format!(
            "<w:rFonts w:ascii=\"{font}\" w:hAnsi=\"{font}\"/>"
        ));
    }
    let half_points = (paragraph.format.size.max(8.0) * 2.0).round() as i32;
    properties.push_str(&format!("<w:sz w:val=\"{half_points}\"/>"));
    if paragraph.heading || marks.contains(&InlineMark::Bold) {
        properties.push_str("<w:b/>");
    }
    if marks.contains(&InlineMark::Italic) {
        properties.push_str("<w:i/>");
    }
    if marks.contains(&InlineMark::Underline) {
        properties.push_str("<w:u w:val=\"single\"/>");
    }
    if marks.contains(&InlineMark::Strikethrough) {
        properties.push_str("<w:strike/>");
    }
    properties
}

fn docx_prose_runs(paragraph: &ProseParagraph, fallback_font: &str) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                let properties = docx_run_properties(paragraph, &run.marks, fallback_font);
                out.push_str(&docx_text_run(&paragraph.transform(&run.text), &properties));
            }
            InlineContent::HardBreak => out.push_str("<w:r><w:br/></w:r>"),
            InlineContent::Image(image) => {
                let label = if image.alt_text.is_empty() {
                    "[Image]".to_owned()
                } else {
                    format!("[Image: {}]", paragraph.transform(&image.alt_text))
                };
                let properties = docx_run_properties(paragraph, &[], fallback_font);
                out.push_str(&docx_text_run(&label, &properties));
            }
        }
    }
    out
}

fn docx_prose_paragraph(paragraph: &ProseParagraph, fallback_font: &str) -> String {
    let style = &paragraph.format;
    let mut properties = String::new();
    properties.push_str(if paragraph.heading {
        "<w:pStyle w:val=\"Heading2\"/>"
    } else {
        "<w:pStyle w:val=\"Normal\"/>"
    });
    let before = (style.before.max(0.0) * 20.0).round() as i32;
    let after = (style.after.max(0.0) * 20.0).round() as i32;
    let line = (style.leading.max(1.0) * 240.0).round() as i32;
    properties.push_str(&format!(
        "<w:spacing w:before=\"{before}\" w:after=\"{after}\" w:line=\"{line}\" w:lineRule=\"auto\"/>"
    ));
    let left = (style.indent.max(0.0) * 20.0).round() as i32;
    let right = (style.right_indent.max(0.0) * 20.0).round() as i32;
    let first = (style.first_line_indent * 20.0).round() as i32;
    if first >= 0 {
        properties.push_str(&format!(
            "<w:ind w:left=\"{left}\" w:right=\"{right}\" w:firstLine=\"{first}\"/>"
        ));
    } else {
        properties.push_str(&format!(
            "<w:ind w:left=\"{left}\" w:right=\"{right}\" w:hanging=\"{}\"/>",
            -first
        ));
    }
    if let Some(align) = docx_align(style.align) {
        properties.push_str(&format!("<w:jc w:val=\"{align}\"/>"));
    }
    format!(
        "<w:p><w:pPr>{properties}</w:pPr>{}</w:p>",
        docx_prose_runs(paragraph, fallback_font)
    )
}

fn docx_basic_paragraph(
    body: &mut String,
    text: &str,
    style: &str,
    bold: bool,
    center: bool,
    mono: bool,
    indent: i32,
    prose_font: &str,
) {
    let jc = if center { "<w:jc w:val=\"center\"/>" } else { "" };
    let fonts = if mono {
        "<w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/>".to_string()
    } else {
        format!(
            "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\"/>",
            prose_font, prose_font
        )
    };
    // Element order inside w:pPr matters to Word: pStyle, ind, jc.
    body.push_str(&format!(
        "<w:p><w:pPr><w:pStyle w:val=\"{}\"/><w:ind w:left=\"{}\"/>{}</w:pPr><w:r><w:rPr>{}{}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        style,
        indent,
        jc,
        fonts,
        if bold { "<w:b/>" } else { "" },
        xml_escape(text)
    ));
}

fn docx_manuscript(project: &Project, settings: &ExportSettings) -> Vec<u8> {
    let mut body = String::new();
    let prose_font = xml_escape(&settings.font);

    if settings.include_title_page {
        docx_basic_paragraph(&mut body, &project.title, "Title", true, true, false, 0, &prose_font);
        if !project.subtitle.is_empty() {
            docx_basic_paragraph(
                &mut body,
                &project.subtitle,
                "Subtitle",
                false,
                true,
                false,
                0,
                &prose_font,
            );
        }
        docx_basic_paragraph(
            &mut body,
            &format!("by {}", project.author),
            "Normal",
            false,
            true,
            false,
            0,
            &prose_font,
        );
        docx_basic_paragraph(&mut body, "", "Normal", false, false, false, 0, &prose_font);
    }
    for doc in front_matter(project, settings) {
        docx_basic_paragraph(&mut body, &doc.title, "Heading1", true, false, false, 0, &prose_font);
        for para in prose_paragraphs(project, doc, settings) {
            body.push_str(&docx_prose_paragraph(&para, &settings.font));
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        docx_basic_paragraph(&mut body, &title, "Heading1", true, false, false, 0, &prose_font);
        if section.doc.medium.is_script() {
            let blocks = screenplay_blocks(project, section.doc);
            for block in &blocks {
                let (indent, mono, bold) = match block.kind {
                    ElementKind::Character => (4320, true, false),
                    ElementKind::Parenthetical => (3240, true, false),
                    ElementKind::Dialogue => (2160, true, false),
                    ElementKind::SceneHeading => (0, true, false),
                    _ => (0, true, false),
                };
                let text = if settings.styles.style_for_kind(block.kind).uppercase {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                docx_basic_paragraph(
                    &mut body,
                    &text,
                    if block.kind == ElementKind::SceneHeading { "Heading2" } else { "Normal" },
                    bold,
                    block.kind == ElementKind::Transition,
                    mono,
                    indent,
                    &prose_font,
                );
            }
        } else {
            for para in prose_paragraphs(project, section.doc, settings) {
                body.push_str(&docx_prose_paragraph(&para, &settings.font));
            }
        }
    }
    for doc in back_matter(project, settings) {
        docx_basic_paragraph(&mut body, &doc.title, "Heading1", true, false, false, 0, &prose_font);
        for para in prose_paragraphs(project, doc, settings) {
            body.push_str(&docx_prose_paragraph(&para, &settings.font));
        }
    }

    let (page_w, page_h) = paper_twips(settings);
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{}\n<w:sectPr><w:pgSz w:w=\"{page_w}\" w:h=\"{page_h}\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr></w:body></w:document>",
        body
    );
    let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
 <w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman"/><w:sz w:val="24"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120" w:line="360" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
 <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
 <w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="2400" w:after="240"/></w:pPr><w:rPr><w:sz w:val="52"/></w:rPr></w:style>
 <w:style w:type="paragraph" w:styleId="Subtitle"><w:name w:val="Subtitle"/><w:basedOn w:val="Normal"/><w:pPr><w:jc w:val="center"/></w:pPr><w:rPr><w:sz w:val="26"/><w:i/></w:rPr></w:style>
 <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:pPr><w:pageBreakBefore/><w:spacing w:before="720" w:after="360"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="36"/></w:rPr></w:style>
 <w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="240" w:after="120"/></w:pPr><w:rPr><w:b/></w:rPr></w:style>
</w:styles>"#.replace("Times New Roman", &prose_font);
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#;
    let doc_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;
    let core = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{}</dc:title><dc:creator>{}</dc:creator></cp:coreProperties>",
        xml_escape(&project.title),
        xml_escape(&project.author)
    );

    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, data) in [
            ("[Content_Types].xml", content_types),
            ("_rels/.rels", rels),
            ("word/document.xml", &document),
            ("word/styles.xml", styles.as_str()),
            ("word/_rels/document.xml.rels", doc_rels),
            ("docProps/core.xml", &core),
        ] {
            zip.start_file(name, opts).unwrap();
            zip.write_all(data.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    buf
}

// ---------------------------------------------------------------------------
// Final Draft, Fountain and web/text formats
// ---------------------------------------------------------------------------

fn fdx_type(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::SceneHeading => "Scene Heading",
        ElementKind::Action => "Action",
        ElementKind::Character => "Character",
        ElementKind::Parenthetical => "Parenthetical",
        ElementKind::Dialogue => "Dialogue",
        ElementKind::Transition => "Transition",
        ElementKind::Shot => "Shot",
        ElementKind::Act => "General",
        ElementKind::Scene => "General",
        ElementKind::StageDirection => "Action",
        ElementKind::Lyric => "Lyric",
        ElementKind::General => "General",
        ElementKind::Extension => "Extension",
        ElementKind::Intercut => "Transition",
        ElementKind::Subheading => "Scene Heading",
    }
}

fn fdx_style_attr(marks: &[InlineMark]) -> String {
    let mut parts = Vec::new();
    for (mark, name) in [
        (InlineMark::Bold, "Bold"),
        (InlineMark::Italic, "Italic"),
        (InlineMark::Underline, "Underline"),
        (InlineMark::Strikethrough, "Strikethrough"),
    ] {
        if marks.contains(&mark) {
            parts.push(name);
        }
    }
    parts.join("+")
}

/// Write one paragraph's runs, preserving inline emphasis in Final Draft's
/// `<Text Style="...">` attribute. Rich images contribute an explicit
/// placeholder because FDX cannot embed their pixels here.
fn fdx_runs(paragraph: &RichParagraph, uppercase: bool) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                if run.text.is_empty() {
                    continue;
                }
                let text = if uppercase {
                    run.text.to_uppercase()
                } else {
                    run.text.clone()
                };
                let style = fdx_style_attr(&run.marks);
                if style.is_empty() {
                    out.push_str(&format!("<Text>{}</Text>", xml_escape(&text)));
                } else {
                    out.push_str(&format!(
                        "<Text Style=\"{style}\">{}</Text>",
                        xml_escape(&text)
                    ));
                }
            }
            InlineContent::HardBreak => out.push_str("<Text> </Text>"),
            InlineContent::Image(image) => {
                let label = if image.alt_text.is_empty() {
                    "[Image]".to_owned()
                } else {
                    format!("[Image: {}]", image.alt_text)
                };
                out.push_str(&format!("<Text>{}</Text>", xml_escape(&label)));
            }
        }
    }
    out
}

fn fdx_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n");
    out.push_str("<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"5\">\n");
    if settings.include_title_page {
        out.push_str("  <TitlePage><Content>\n");
        out.push_str(&format!(
            "    <Paragraph Type=\"Title\"><Text>{}</Text></Paragraph>\n",
            xml_escape(&project.title)
        ));
        if !project.subtitle.is_empty() {
            out.push_str(&format!(
                "    <Paragraph Type=\"Subtitle\"><Text>{}</Text></Paragraph>\n",
                xml_escape(&project.subtitle)
            ));
        }
        out.push_str(&format!(
            "    <Paragraph Type=\"Author\"><Text>{}</Text></Paragraph>\n",
            xml_escape(&project.author)
        ));
        out.push_str("  </Content></TitlePage>\n");
    }
    out.push_str("  <Content>\n");
    for section in manuscript_sections(project, settings) {
        if section.doc.medium.is_script() {
            let fallback = StyleSheet::film_screenplay(&project.styles);
            let sheet = project
                .style_sheets
                .iter()
                .find(|sheet| sheet.id == section.doc.style_sheet_id)
                .unwrap_or(&fallback);
            for paragraph in &section.doc.content.paragraphs {
                let style = sheet.style(&paragraph.style_id);
                let kind = style
                    .and_then(|style| style.semantic_kind)
                    .or_else(|| {
                        portmanteau_core::rich_text::script_kind_for_style_id(
                            &paragraph.style_id,
                        )
                    })
                    .unwrap_or(ElementKind::General);
                let uppercase = style
                    .map(|style| style.text_transform == TextTransform::Uppercase)
                    .unwrap_or_else(|| settings.styles.style_for_kind(kind).uppercase);
                let runs = fdx_runs(paragraph, uppercase);
                if runs.is_empty() {
                    continue;
                }
                out.push_str(&format!(
                    "    <Paragraph Type=\"{}\">{runs}</Paragraph>\n",
                    fdx_type(kind)
                ));
            }
        } else {
            out.push_str(&format!(
                "    <Paragraph Type=\"General\"><Text>{}</Text></Paragraph>\n",
                xml_escape(&section.heading)
            ));
            for para in paragraphs(&section.doc.body_text()) {
                out.push_str(&format!(
                    "    <Paragraph Type=\"Action\"><Text>{}</Text></Paragraph>\n",
                    xml_escape(&para)
                ));
            }
        }
    }
    out.push_str("  </Content>\n</FinalDraft>\n");
    out
}

fn fountain_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    if settings.include_title_page {
        out.push_str(&script::fountain_title_page(project));
    }
    for section in manuscript_sections(project, settings) {
        if section.doc.medium.is_script() {
            out.push_str(&format!("# {}\n\n", section.heading));
            let blocks = screenplay_blocks(project, section.doc);
            out.push_str(&script::to_fountain(&blocks, section.doc.medium));
            out.push('\n');
        } else {
            out.push_str(&format!("# {}\n\n", section.heading));
            for para in paragraphs(&section.doc.body_text()) {
                out.push_str(&para);
                out.push_str("\n\n");
            }
        }
    }
    out
}

fn html_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut body = String::new();
    if settings.include_title_page {
        body.push_str(&format!(
            "<header class=\"titlepage\"><h1>{}</h1><p class=\"sub\">{}</p><p>by {}</p></header>\n",
            html_escape(&project.title),
            html_escape(&project.subtitle),
            html_escape(&project.author)
        ));
    }
    if settings.include_toc {
        body.push_str("<nav class=\"toc\"><h2>Contents</h2><ol>\n");
        for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
            let title = section_title(project, section.doc, i, settings);
            body.push_str(&format!(
                "<li><a href=\"#s{}\">{}</a></li>\n",
                i,
                html_escape(&title)
            ));
        }
        body.push_str("</ol></nav>\n");
    }
    for doc in front_matter(project, settings) {
        body.push_str(&format!(
            "<section class=\"frontmatter\"><h2>{}</h2>\n",
            html_escape(&doc.title)
        ));
        for (i, para) in prose_paragraphs(project, doc, settings).iter().enumerate() {
            body.push_str(&html_prose_paragraph(para, i == 0));
        }
        body.push_str("</section>\n");
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        body.push_str(&format!(
            "<section id=\"s{}\" class=\"chapter\"><h2>{}</h2>\n",
            i,
            html_escape(&title)
        ));
        if section.doc.medium.is_script() {
            body.push_str("<div class=\"script\">\n");
            let blocks = screenplay_blocks(project, section.doc);
            for block in &blocks {
                let class = match block.kind {
                    ElementKind::SceneHeading => "scene",
                    ElementKind::Character => "character",
                    ElementKind::Parenthetical => "paren",
                    ElementKind::Dialogue => "dialogue",
                    ElementKind::Transition => "trans",
                    _ => "action",
                };
                let text = if settings.styles.style_for_kind(block.kind).uppercase {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                body.push_str(&format!(
                    "<p class=\"{}\">{}</p>\n",
                    class,
                    html_escape(&text)
                ));
            }
            body.push_str("</div>\n");
        } else {
            for (j, para) in prose_paragraphs(project, section.doc, settings)
                .iter()
                .enumerate()
            {
                body.push_str(&html_prose_paragraph(para, j == 0));
            }
        }
        body.push_str("</section>\n");
    }
    for doc in back_matter(project, settings) {
        body.push_str(&format!(
            "<section class=\"backmatter\"><h2>{}</h2>\n",
            html_escape(&doc.title)
        ));
        for (i, para) in prose_paragraphs(project, doc, settings).iter().enumerate() {
            body.push_str(&html_prose_paragraph(para, i == 0));
        }
        body.push_str("</section>\n");
    }
    let st = &settings.styles;
    let body_font = element_css_font(&st.body, "Liberation Serif");
    let script_font = element_css_font(&st.action, "Courier New");
    let chapter_size = st.chapter_heading.size.max(8.0);
    let body_align = css_align(st.body.align);
    let heading_align = css_align(st.chapter_heading.align);
    let body_indent_margins = css_indent_margins(&st.body);
    let body_first_indent = st.body.first_line_indent.max(0.0);
    let title_align = css_align(st.title_page_title.align);
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\"/>\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/>\n<title>{title}</title>\n<style>\nbody{{max-width:42rem;margin:4rem auto;padding:0 1.5rem;font-family:{body_font};line-height:{body_leading:.2};text-align:{body_align};color:#1e1c19;background:#fbf8f1}}\nh1{{font-size:{h1_size:.1}pt;line-height:1.1;text-align:{heading_align}}}h2{{font-size:{chapter_size:.1}pt;margin-top:3rem;border-bottom:1px solid #d8d0c0;padding-bottom:.4rem;text-align:{heading_align}}}\np{{margin:0 0 .9rem;{body_indent_margins}text-indent:{body_first_indent:.1}pt;text-align:{body_align}}}p.first{{text-indent:0}}\n.titlepage{{text-align:center;margin:18vh 0 22vh}}.titlepage h1{{font-size:{title_size:.1}pt;margin-bottom:.3em;text-align:{title_align}}}.titlepage p{{text-indent:0}}.sub{{font-style:italic;color:#665f54}}\n.toc ol{{line-height:2}}.toc a{{color:#2f5d50;text-decoration:none}}\n.script{{font-family:{script_font};font-size:{script_size:.1}pt}}.script p{{text-indent:0;margin:.4rem 0;text-align:left}}\n.scene{{font-weight:bold;margin-top:1.6rem!important}}.character{{margin-left:32%!important;margin-top:1rem!important}}.paren{{margin-left:24%!important}}.dialogue{{margin-left:16%!important;margin-right:12%!important}}.trans{{text-align:right!important}}\n.frontmatter,.backmatter{{margin-top:4rem}}\n</style>\n</head>\n<body>\n{body}\n</body>\n</html>\n",
        title = html_escape(&project.title),
        body_font = body_font,
        body_leading = st.body.leading.max(1.0),
        body_align = body_align,
        body_indent_margins = body_indent_margins,
        body_first_indent = body_first_indent,
        heading_align = heading_align,
        title_align = title_align,
        h1_size = chapter_size * 1.5,
        chapter_size = chapter_size,
        title_size = st.title_page_title.size.max(8.0),
        script_font = script_font,
        script_size = st.action.size.max(8.0),
        body = body,
    )
}

fn markdown_runs(paragraph: &ProseParagraph) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                let mut text = paragraph.transform(&run.text);
                if text.is_empty() {
                    continue;
                }
                if run.marks.contains(&InlineMark::Italic) {
                    text = format!("*{text}*");
                }
                if run.marks.contains(&InlineMark::Bold) {
                    text = format!("**{text}**");
                }
                if run.marks.contains(&InlineMark::Underline) {
                    text = format!("<u>{text}</u>");
                }
                if run.marks.contains(&InlineMark::Strikethrough) {
                    text = format!("~~{text}~~");
                }
                out.push_str(&text);
            }
            InlineContent::HardBreak => out.push_str("<br>"),
            InlineContent::Image(image) if image.alt_text.is_empty() => out.push_str("[Image]"),
            InlineContent::Image(image) => out.push_str(&format!(
                "[Image: {}]",
                paragraph.transform(&image.alt_text)
            )),
        }
    }
    out
}

fn markdown_prose_paragraph(paragraph: &ProseParagraph) -> String {
    if paragraph.heading {
        format!("### {}", markdown_runs(paragraph))
    } else {
        markdown_runs(paragraph)
    }
}

fn markdown_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    if settings.include_title_page {
        out.push_str(&format!("# {}\n\n", project.title));
        if !project.subtitle.is_empty() {
            out.push_str(&format!("*{}*\n\n", project.subtitle));
        }
        out.push_str(&format!("by {}\n\n---\n\n", project.author));
    }
    if settings.include_toc {
        out.push_str("## Contents\n\n");
        for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
            let title = section_title(project, section.doc, i, settings);
            out.push_str(&format!("{}. {}\n", i + 1, title));
        }
        out.push('\n');
    }
    for doc in front_matter(project, settings) {
        out.push_str(&format!("## {}\n\n", doc.title));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&markdown_prose_paragraph(&para));
            out.push_str("\n\n");
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        out.push_str(&format!("## {}\n\n", title));
        if section.doc.medium.is_script() {
            out.push_str("```fountain\n");
            let blocks = screenplay_blocks(project, section.doc);
            out.push_str(&script::to_fountain(&blocks, section.doc.medium));
            out.push_str("```\n\n");
        } else {
            for para in prose_paragraphs(project, section.doc, settings) {
                out.push_str(&markdown_prose_paragraph(&para));
                out.push_str("\n\n");
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&format!("## {}\n\n", doc.title));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&markdown_prose_paragraph(&para));
            out.push_str("\n\n");
        }
    }
    out
}

fn rtf_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\n' => out.push_str("\\line "),
            c if c.is_ascii() => out.push(c),
            c => out.push_str(&format!("\\u{}?", c as i32)),
        }
    }
    out
}

fn rtf_prose_runs(paragraph: &ProseParagraph) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                let mut prefix = String::new();
                let mut suffix = String::new();
                if run.marks.contains(&InlineMark::Bold) {
                    prefix.push_str("\\b ");
                    suffix.insert_str(0, "\\b0 ");
                }
                if run.marks.contains(&InlineMark::Italic) {
                    prefix.push_str("\\i ");
                    suffix.insert_str(0, "\\i0 ");
                }
                if run.marks.contains(&InlineMark::Underline) {
                    prefix.push_str("\\ul ");
                    suffix.insert_str(0, "\\ulnone ");
                }
                if run.marks.contains(&InlineMark::Strikethrough) {
                    prefix.push_str("\\strike ");
                    suffix.insert_str(0, "\\strike0 ");
                }
                out.push_str(&prefix);
                out.push_str(&rtf_escape(&paragraph.transform(&run.text)));
                out.push_str(&suffix);
            }
            InlineContent::HardBreak => out.push_str("\\line "),
            InlineContent::Image(image) if image.alt_text.is_empty() => out.push_str("[Image]"),
            InlineContent::Image(image) => out.push_str(&rtf_escape(&format!(
                "[Image: {}]",
                paragraph.transform(&image.alt_text)
            ))),
        }
    }
    out
}

fn rtf_prose_paragraph(paragraph: &ProseParagraph) -> String {
    let style = &paragraph.format;
    let mut out = String::from("\\pard");
    match style.align {
        Alignment::Center => out.push_str("\\qc"),
        Alignment::Right => out.push_str("\\qr"),
        Alignment::Justify => out.push_str("\\qj"),
        Alignment::Left => out.push_str("\\ql"),
    }
    let left = (style.indent.max(0.0) * 20.0).round() as i32;
    let right = (style.right_indent.max(0.0) * 20.0).round() as i32;
    let first = (style.first_line_indent * 20.0).round() as i32;
    if left > 0 {
        out.push_str(&format!("\\li{left}"));
    }
    if right > 0 {
        out.push_str(&format!("\\ri{right}"));
    }
    if first != 0 {
        out.push_str(&format!("\\fi{first}"));
    }
    let before = (style.before.max(0.0) * 20.0).round() as i32;
    let after = (style.after.max(0.0) * 20.0).round() as i32;
    if before > 0 {
        out.push_str(&format!("\\sb{before}"));
    }
    if after > 0 {
        out.push_str(&format!("\\sa{after}"));
    }
    let half_points = (style.size.max(8.0) * 2.0).round() as i32;
    out.push_str(&format!(
        "\\f{}\\fs{} ",
        if font_is_mono(&style.font) { 1 } else { 0 },
        half_points
    ));
    if paragraph.heading {
        out.push_str("\\b ");
        out.push_str(&rtf_prose_runs(paragraph));
        out.push_str("\\b0 ");
    } else {
        out.push_str(&rtf_prose_runs(paragraph));
    }
    out.push_str("\\par\n");
    out
}

fn rtf_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{{\\rtf1\\ansi\\deff0{{\\fonttbl{{\\f0 {};}}{{\\f1 Courier New;}}}}\n",
        rtf_escape(&settings.font)
    ));
    let (page_w, page_h) = paper_twips(settings);
    out.push_str(&format!(
        "\\paperw{page_w}\\paperh{page_h}\\margl1440\\margr1440\\margt1440\\margb1440\n"
    ));
    if settings.include_title_page {
        out.push_str("\\qc\\f0\\fs52 ");
        out.push_str(&rtf_escape(&project.title));
        out.push_str("\\par\\fs24\\par ");
        out.push_str(&rtf_escape(&project.author));
        out.push_str("\\par\\page\n");
    }
    for doc in front_matter(project, settings) {
        out.push_str(&format!(
            "\\f0\\fs32\\b {}\\b0\\fs24\\par\n",
            rtf_escape(&doc.title)
        ));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&rtf_prose_paragraph(&para));
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        out.push_str(&format!(
            "\\page\\f0\\fs36\\b {}\\b0\\fs24\\par\n",
            rtf_escape(&title)
        ));
        if section.doc.medium.is_script() {
            let blocks = screenplay_blocks(project, section.doc);
            for block in &blocks {
                let indent = match block.kind {
                    ElementKind::Character => 4400,
                    ElementKind::Parenthetical => 3200,
                    ElementKind::Dialogue => 2000,
                    _ => 0,
                };
                let text = if settings.styles.style_for_kind(block.kind).uppercase {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                out.push_str(&format!(
                    "\\f1\\fs24\\li{}\\ri0 {}\\par\n",
                    indent,
                    rtf_escape(&text)
                ));
            }
        } else {
            for para in prose_paragraphs(project, section.doc, settings) {
                out.push_str(&rtf_prose_paragraph(&para));
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&format!(
            "\\page\\f0\\fs32\\b {}\\b0\\fs24\\par\n",
            rtf_escape(&doc.title)
        ));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&rtf_prose_paragraph(&para));
        }
    }
    out.push_str("}\n");
    out
}

fn txt_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    if settings.include_title_page {
        out.push_str(&format!("{}\n", project.title.to_uppercase()));
        if !project.subtitle.is_empty() {
            out.push_str(&format!("{}\n", project.subtitle));
        }
        out.push_str(&format!("\nby {}\n\n", project.author));
        out.push_str(&"-".repeat(40));
        out.push_str("\n\n");
    }
    for doc in front_matter(project, settings) {
        out.push_str(&format!("{}\n\n", doc.title.to_uppercase()));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&para.plain_text());
            out.push_str("\n\n");
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        out.push_str(&format!("\n{}\n{}\n\n", title.to_uppercase(), "-".repeat(title.len())));
        if section.doc.medium.is_script() {
            let blocks = screenplay_blocks(project, section.doc);
            for block in &blocks {
                out.push_str(&script::block_to_plain(block));
                out.push_str("\n\n");
            }
        } else {
            for para in prose_paragraphs(project, section.doc, settings) {
                out.push_str(&para.plain_text());
                out.push_str("\n\n");
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&format!("\n{}\n\n", doc.title.to_uppercase()));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&para.plain_text());
            out.push_str("\n\n");
        }
    }
    out
}

fn bundle_manuscript(project: &Project) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let json = serde_json::to_string_pretty(project).map_err(|e| e.to_string())?;
        zip.start_file("project.json", opts).unwrap();
        zip.write_all(json.as_bytes()).unwrap();
        zip.start_file("manuscript.md", opts).unwrap();
        zip.write_all(markdown_manuscript(project, &project.export).as_bytes())
            .unwrap();
        zip.start_file("screenplay.fountain", opts).unwrap();
        zip.write_all(fountain_manuscript(project, &project.export).as_bytes())
            .unwrap();
        let mut notes = String::new();
        for doc in &project.documents {
            if !doc.snapshots.is_empty() {
                notes.push_str(&format!(
                    "# {}\n\n{} snapshots\n\n",
                    doc.title,
                    doc.snapshots.len()
                ));
                for snap in &doc.snapshots {
                    notes.push_str(&format!("- {} · {} words · {}\n", snap.name, snap.words, snap.created));
                }
            }
        }
        zip.start_file("version-notes.md", opts).unwrap();
        zip.write_all(notes.as_bytes()).unwrap();
        zip.start_file("README.txt", opts).unwrap();
        zip.write_all(
            format!(
                "{} by {}\n\nThis bundle contains the project file, a Markdown manuscript, a Fountain screenplay, and version notes.\nOpen project.json with Portmanteau to continue writing.\n",
                project.title, project.author
            )
            .as_bytes(),
        )
        .unwrap();
        zip.finish().unwrap();
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> Project {
        portmanteau_core::sample::demo()
    }

    #[test]
    fn epub_css_uses_the_body_style_font() {
        let p = demo();
        let mut s = p.settings_for(ExportKind::Epub);
        s.styles.body.font = "SomeTestFont".into();
        let bytes = epub_manuscript(&p, &s);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut css = String::new();
        std::io::Read::read_to_string(&mut archive.by_name("OEBPS/style.css").unwrap(), &mut css)
            .unwrap();
        assert!(css.contains("SomeTestFont"), "EPUB CSS: {css}");
    }

    #[test]
    fn html_uses_the_body_style_font() {
        let p = demo();
        let mut s = p.settings_for(ExportKind::Html);
        s.styles.body.font = "SomeTestFont".into();
        let html = html_manuscript(&p, &s);
        assert!(html.contains("SomeTestFont"), "HTML should carry the font family");
    }

    #[test]
    fn epub_css_honours_body_alignment_and_first_line_indent() {
        let p = demo();
        let mut s = p.settings_for(ExportKind::Epub);
        s.styles.body.align = Alignment::Center;
        s.styles.body.first_line_indent = 21.0;
        s.styles.chapter_heading.align = Alignment::Right;
        let bytes = epub_manuscript(&p, &s);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut css = String::new();
        std::io::Read::read_to_string(&mut archive.by_name("OEBPS/style.css").unwrap(), &mut css)
            .unwrap();
        assert!(css.contains("text-indent: 21.00pt"), "EPUB CSS: {css}");
        assert!(css.contains("text-align: center"), "EPUB CSS: {css}");
        assert!(css.contains("text-align: right"), "EPUB CSS: {css}");
    }

    #[test]
    fn html_css_honours_body_alignment_and_first_line_indent() {
        let p = demo();
        let mut s = p.settings_for(ExportKind::Html);
        s.styles.body.align = Alignment::Justify;
        s.styles.body.first_line_indent = 17.0;
        let html = html_manuscript(&p, &s);
        assert!(html.contains("text-indent:17.0pt"), "HTML CSS: {html}");
        assert!(html.contains("text-align:justify"), "HTML CSS: {html}");
    }

    #[test]
    fn epub_has_mimetype_first() {
        let p = demo();
        let bytes = epub_manuscript(&p, &p.export);
        assert!(bytes.starts_with(b"PK"));
        assert!(bytes.windows(8).any(|w| w == b"mimetype"));
    }

    #[test]
    fn docx_is_a_zip() {
        let p = demo();
        let bytes = docx_manuscript(&p, &p.export);
        assert!(bytes.starts_with(b"PK"));
    }

    #[test]
    fn docx_and_rtf_page_size_follow_the_paper_setting() {
        let p = demo();
        // Width and height in twentieths of a point for every paper the
        // Compile screen offers.
        let expected = [
            (PaperSize::Letter, 12240, 15840),
            (PaperSize::A4, 11906, 16838),
            (PaperSize::A5, 8391, 11906),
            (PaperSize::UsTrade, 8640, 12960),
        ];
        for (paper, width, height) in expected {
            let mut s = p.settings_for(ExportKind::Docx);
            s.paper = paper;
            let xml = docx_document_xml(docx_manuscript(&p, &s));
            assert!(
                xml.contains(&format!("<w:pgSz w:w=\"{width}\" w:h=\"{height}\"/>")),
                "DOCX should carry {} as {width}x{height} twips",
                paper.label()
            );

            let mut s = p.settings_for(ExportKind::Rtf);
            s.paper = paper;
            let rtf = rtf_manuscript(&p, &s);
            assert!(
                rtf.contains(&format!("\\paperw{width}\\paperh{height}")),
                "RTF should carry {} as {width}x{height} twips",
                paper.label()
            );
        }
    }

    fn docx_document_xml(bytes: Vec<u8>) -> String {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("word/document.xml").unwrap(),
            &mut xml,
        )
        .unwrap();
        xml
    }

    /// A novel with one chapter whose second paragraph uses a writer-defined
    /// paragraph style stored in the project stylesheet.
    fn styled_prose_project() -> Project {
        let mut project = Project::new("Style Test", ProjectKind::Novel);
        let parent = project.folder_id("Draft");
        let doc_id = project.add_document(parent, "Chapter One", DocKind::Chapter, Medium::Prose);
        let sheet_id = project.document(&doc_id).unwrap().style_sheet_id.clone();
        let sheet = project
            .style_sheets
            .iter_mut()
            .find(|sheet| sheet.id == sheet_id)
            .expect("prose stylesheet");
        sheet.styles.push(ParagraphStyle {
            id: "epigraph".into(),
            name: "Epigraph".into(),
            format: ElementStyle {
                align: Alignment::Center,
                size: 9.0,
                first_line_indent: 0.0,
                before: 6.0,
                after: 6.0,
                ..ElementStyle::default()
            },
            semantic_kind: None,
            next_style: Some("body".into()),
            text_transform: TextTransform::None,
        });
        let doc = project.document_mut(&doc_id).unwrap();
        doc.content = RichDocument {
            paragraphs: vec![
                RichParagraph {
                    style_id: "body".into(),
                    content: vec![InlineContent::Text(TextRun::with_marks(
                        "First paragraph.",
                        vec![],
                    ))],
                },
                RichParagraph {
                    style_id: "epigraph".into(),
                    content: vec![InlineContent::Text(TextRun::with_marks(
                        "A centred epigraph.",
                        vec![],
                    ))],
                },
                RichParagraph {
                    style_id: "chapter-heading".into(),
                    content: vec![InlineContent::Text(TextRun::with_marks(
                        "A heading inside the body",
                        vec![],
                    ))],
                },
            ],
        };
        project
    }

    #[test]
    fn prose_export_honours_project_paragraph_styles() {
        let project = styled_prose_project();
        let html = html_manuscript(&project, &project.settings_for(ExportKind::Html));
        assert!(
            html.lines().any(|line| line.contains("A centred epigraph.")
                && line.contains("text-align: center")
                && line.contains("font-size: 9.0pt")),
            "custom paragraph style should reach HTML: {html}"
        );
        assert!(
            !html.contains("style=\"font-family: \""),
            "font names inside a style attribute must not close the attribute"
        );
        assert!(
            html.lines()
                .any(|line| line.contains("<h2") && line.contains("A heading inside the body")),
            "an in-body chapter-heading paragraph should become a subheading"
        );

        let xml = docx_document_xml(docx_manuscript(
            &project,
            &project.settings_for(ExportKind::Docx),
        ));
        assert!(xml.contains("A centred epigraph."));
        assert!(
            xml.contains("<w:jc w:val=\"center\"/>") && xml.contains("<w:sz w:val=\"18\"/>"),
            "DOCX should carry the custom style's alignment and size"
        );
        assert!(
            xml.contains("<w:pStyle w:val=\"Heading2\"/>"),
            "in-body heading should use the DOCX heading style"
        );

        let text = txt_manuscript(&project, &project.settings_for(ExportKind::Txt));
        assert!(
            text.contains("First paragraph.\n\nA centred epigraph.\n\nA heading inside the body"),
            "paragraph boundaries must survive text export: {text}"
        );
    }

    #[test]
    fn stylesheet_appearance_edits_reach_export_without_a_compile_save() {
        let mut project = Project::new("Live Style", ProjectKind::Novel);
        let parent = project.folder_id("Draft");
        let doc_id = project.add_document(parent, "Chapter", DocKind::Chapter, Medium::Prose);
        let sheet_id = project.document(&doc_id).unwrap().style_sheet_id.clone();
        let sheet = project
            .style_sheets
            .iter_mut()
            .find(|sheet| sheet.id == sheet_id)
            .unwrap();
        let body = sheet
            .styles
            .iter_mut()
            .find(|style| style.id == "body")
            .unwrap();
        body.format.size = 13.0;
        body.format.align = Alignment::Right;
        body.format.first_line_indent = 0.0;
        let doc = project.document_mut(&doc_id).unwrap();
        doc.set_plain_text("The stylesheet is the source of truth.");

        // The format settings are still seeded from the old base style, as if
        // the writer had never opened Compile after editing the sheet.
        let settings = project.settings_for(ExportKind::Html);
        let html = html_manuscript(&project, &settings);
        assert!(
            html.lines().any(|line| line.contains("The stylesheet is the source of truth.")
                && line.contains("font-size: 13.0pt")
                && line.contains("text-align: right")),
            "project sheet edit should win over a stale seeded format: {html}"
        );
    }

    #[test]
    fn prose_export_keeps_inline_marks_and_paragraph_breaks() {
        let mut project = Project::new("Marks Test", ProjectKind::Novel);
        let parent = project.folder_id("Draft");
        let doc_id = project.add_document(parent, "Chapter", DocKind::Chapter, Medium::Prose);
        let doc = project.document_mut(&doc_id).unwrap();
        doc.content = RichDocument {
            paragraphs: vec![
                RichParagraph {
                    style_id: "body".into(),
                    content: vec![
                        InlineContent::Text(TextRun::with_marks("Bold", vec![InlineMark::Bold])),
                        InlineContent::Text(TextRun::plain(" and ")),
                        InlineContent::Text(TextRun::with_marks(
                            "italic.",
                            vec![InlineMark::Italic],
                        )),
                    ],
                },
                RichParagraph {
                    style_id: "body".into(),
                    content: vec![InlineContent::Text(TextRun::plain("Second paragraph."))],
                },
            ],
        };

        let html = html_manuscript(&project, &project.settings_for(ExportKind::Html));
        assert!(html.contains("<strong>Bold</strong>"));
        assert!(html.contains("<em>italic.</em>"));
        assert!(html.contains("Second paragraph."));

        let markdown = markdown_manuscript(&project, &project.settings_for(ExportKind::Markdown));
        assert!(markdown.contains("**Bold**"));
        assert!(markdown.contains("*italic.*"));
        assert!(markdown.contains("Second paragraph.\n"));

        let rtf = rtf_manuscript(&project, &project.settings_for(ExportKind::Rtf));
        assert!(rtf.contains("\\b Bold\\b0"));
        assert!(rtf.contains("\\i italic.\\i0"));

        let xml = docx_document_xml(docx_manuscript(
            &project,
            &project.settings_for(ExportKind::Docx),
        ));
        assert!(xml.contains("<w:b/>"));
        assert!(xml.contains("<w:i/>"));
        assert!(xml.contains("Second paragraph."));

        let text = txt_manuscript(&project, &project.settings_for(ExportKind::Txt));
        assert!(text.contains("Bold and italic.\n\nSecond paragraph."));
    }

    #[test]
    fn fdx_is_wellformed_ish() {
        let p = demo();
        let mut settings = p.export.clone();
        settings.compile_type = Some(CompileType::TvScript);
        let fdx = fdx_manuscript(&p, &settings);
        assert!(fdx.contains("<FinalDraft"));
        assert!(fdx.contains("Scene Heading"));
    }

    #[test]
    fn fdx_export_keeps_inline_marks_and_image_alt_text() {
        let mut p = demo();
        let doc_id = p
            .documents
            .iter()
            .find(|doc| doc.medium.is_script() && doc.include_in_export)
            .map(|doc| doc.id.clone())
            .expect("demo has a screenplay document");
        let sheet = p
            .style_sheets
            .iter()
            .find(|sheet| sheet.medium == Medium::Screenplay)
            .cloned()
            .expect("screenplay sheet");
        let doc = p.document_mut(&doc_id).unwrap();
        let target = doc
            .content
            .paragraphs
            .iter_mut()
            .find(|paragraph| paragraph.style_id == "action")
            .expect("screenplay action paragraph");
        target.content = vec![
            InlineContent::Text(TextRun::with_marks(
                "Bold action",
                vec![InlineMark::Bold],
            )),
            InlineContent::Text(TextRun::with_marks(
                " and italic",
                vec![InlineMark::Italic],
            )),
        ];
        doc.content.paragraphs.push(RichParagraph {
            style_id: "action".into(),
            content: vec![InlineContent::Image(ImageObject {
                asset_id: "images/chart.png".into(),
                alt_text: "Harbor chart".into(),
                caption: String::new(),
                width: None,
                height: None,
            })],
        });

        let mut settings = p.export.clone();
        settings.compile_type = Some(CompileType::TvScript);
        let fdx = fdx_manuscript(&p, &settings);
        assert!(fdx.contains("Style=\"Bold\""), "bold run missing from FDX");
        assert!(fdx.contains("Style=\"Italic\""), "italic run missing from FDX");
        assert!(
            fdx.contains("[Image: Harbor chart]"),
            "lossy FDX export should include the image's alt text"
        );
        assert_eq!(sheet.medium, Medium::Screenplay);
    }

    const TINY_PNG: [u8; 67] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
        0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
        0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn epub_embeds_a_cover_only_when_the_file_exists() {
        let p = demo();
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("epub-cover"));
        fs::create_dir_all(&dir).unwrap();
        let png = dir.join("cover.png");
        fs::write(&png, TINY_PNG).unwrap();

        let mut with_cover = p.settings_for(ExportKind::Epub);
        with_cover.cover_image = png.display().to_string();
        let bytes = epub_manuscript(&p, &with_cover);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "OEBPS/cover-image/cover.png"));
        assert!(names.iter().any(|n| n == "OEBPS/cover.xhtml"));
        let mut opf = String::new();
        std::io::Read::read_to_string(&mut archive.by_name("OEBPS/content.opf").unwrap(), &mut opf)
            .unwrap();
        assert!(opf.contains("properties=\"cover-image\""));
        assert!(opf.contains("<meta name=\"cover\" content=\"cover-img\"/>"));

        let bytes = epub_manuscript(&p, &p.settings_for(ExportKind::Epub));
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(!names.iter().any(|n| n.contains("cover")));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn every_format_exports_to_disk() {
        let p = demo();
        let dir = std::env::temp_dir().join(format!("portmanteau-test-{}", portmanteau_core::new_id("t")));
        for kind in ExportKind::all() {
            let mut s = p.export.clone();
            s.kind = kind;
            let path = dir.join(default_filename(&p, kind));
            let info = export(&p, &s, Some(&path)).unwrap();
            assert!(info.bytes > 100, "{:?} too small", kind);
            assert!(path.exists());
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lossy_text_export_warns_and_uses_an_image_alt_placeholder() {
        let mut project = Project::new("Image export", ProjectKind::Novel);
        let doc_id = project.add_document(
            project.folder_id("Draft"),
            "Image scene",
            DocKind::Chapter,
            Medium::Prose,
        );
        let sheet = StyleSheet::novel_prose(&project.styles);
        let doc = project.document_mut(&doc_id).unwrap();
        doc.content = RichDocument::from_plain_text("before after", "body");
        doc.content.insert_image(
            7..7,
            ImageObject {
                asset_id: "images/harbor.png".into(),
                alt_text: "Harbor map".into(),
                caption: String::new(),
                width: None,
                height: None,
            },
            false,
            &sheet,
        );
        let settings = project.settings_for(ExportKind::Txt);
        let path = std::env::temp_dir().join(format!(
            "portmanteau-image-export-{}.txt",
            portmanteau_core::new_id("t")
        ));

        let info = export(&project, &settings, Some(&path)).unwrap();
        let output = fs::read_to_string(&path).unwrap();
        assert!(
            info.message.contains("alt-text placeholders"),
            "lossy image export should be reported"
        );
        assert!(output.contains("[Image: Harbor map]"));
        let _ = fs::remove_file(path);
    }
}
