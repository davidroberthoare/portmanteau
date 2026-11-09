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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportInfo {
    pub path: String,
    pub bytes: usize,
    pub words: u32,
    pub format: String,
    pub message: String,
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

pub fn export(
    project: &Project,
    settings: &ExportSettings,
    output: Option<&Path>,
) -> Result<ExportInfo, String> {
    let path: PathBuf = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let name = default_filename(project, settings.kind);
            if settings.output_path.trim().is_empty() {
                PathBuf::from("exports").join(name)
            } else {
                PathBuf::from(settings.output_path.trim()).join(name)
            }
        }
    };
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
    Ok(ExportInfo {
        path: path.display().to_string(),
        bytes: bytes.len(),
        words: project.words(),
        format: settings.kind.label().to_string(),
        message: format!(
            "{} exported · {} pages estimated",
            settings.kind.label(),
            project
                .manuscript_documents()
                .iter()
                .map(|d| d.pages())
                .sum::<u32>()
        ),
    })
}

// ---------------------------------------------------------------------------
// Shared manuscript assembly
// ---------------------------------------------------------------------------

fn manuscript_sections(project: &Project) -> Vec<Section<'_>> {
    project
        .manuscript_documents()
        .into_iter()
        .filter(|d| !matches!(d.kind, DocKind::FrontMatter | DocKind::BackMatter))
        .map(|doc| Section {
            heading: doc.title.clone(),
            doc,
        })
        .collect()
}

fn front_matter<'a>(project: &'a Project, settings: &ExportSettings) -> Vec<&'a Document> {
    let mut out = Vec::new();
    for doc in project.ordered_documents() {
        if doc.kind != DocKind::FrontMatter {
            continue;
        }
        let t = doc.title.to_lowercase();
        let include = if t.contains("copyright") {
            settings.include_copyright
        } else if t.contains("dedication") {
            settings.include_dedication
        } else if t.contains("note") || t.contains("preface") || t.contains("introduction") {
            settings.include_preface
        } else if t.contains("title") {
            false
        } else {
            true
        };
        if include {
            out.push(doc);
        }
    }
    out
}

fn back_matter<'a>(project: &'a Project, settings: &ExportSettings) -> Vec<&'a Document> {
    let mut out = Vec::new();
    for doc in project.ordered_documents() {
        if doc.kind != DocKind::BackMatter {
            continue;
        }
        let t = doc.title.to_lowercase();
        let include = if t.contains("about") {
            settings.include_author_bio
        } else if t.contains("acknowledg") {
            settings.include_acknowledgements
        } else {
            true
        };
        if include {
            out.push(doc);
        }
    }
    out
}

fn is_screenplay(project: &Project) -> bool {
    matches!(
        project.kind,
        ProjectKind::Film | ProjectKind::Television | ProjectKind::Theatre
    )
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

// ---------------------------------------------------------------------------
// PDF
// ---------------------------------------------------------------------------

struct PdfLine {
    text: String,
    x: f32,
    y: f32,
    size: f32,
    bold: bool,
    /// Script elements are set in Courier; prose and front matter in Times.
    mono: bool,
}

struct PdfPage {
    lines: Vec<PdfLine>,
}

impl PdfPage {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }
}

fn wrap_text(text: &str, width_chars: usize) -> Vec<String> {
    if width_chars == 0 {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
        } else if line.chars().count() + 1 + word.chars().count() <= width_chars {
            line.push(' ');
            line.push_str(word);
        } else {
            out.push(line);
            line = word.to_string();
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn pdf_text(s: &str) -> String {
    // The built-in PDF fonts use WinAnsi; keep the stream ASCII-clean and map
    // the typography a writer is most likely to use.
    let mut out = String::new();
    for c in s.chars() {
        let mapped: String = match c {
            '(' => "\\(".into(),
            ')' => "\\)".into(),
            '\\' => "\\\\".into(),
            '“' | '”' => "\"".into(),
            '‘' | '’' => "'".into(),
            '–' | '—' => "-".into(),
            '…' => "...".into(),
            '·' => "-".into(),
            '©' => "(c)".into(),
            '®' => "(R)".into(),
            'é' => "e".into(),
            'è' => "e".into(),
            'ê' => "e".into(),
            'à' => "a".into(),
            'ç' => "c".into(),
            'ó' => "o".into(),
            'ü' => "u".into(),
            c if c.is_ascii() => c.to_string(),
            _ => "?".into(),
        };
        out.push_str(&mapped);
    }
    out
}

/// Map a family name to a PDF base-14 font (the writer embeds no font files).
fn pdf_font(family: &str, bold: bool) -> &'static str {
    let family = family.to_ascii_lowercase();
    if family.contains("courier") || family.contains("mono") {
        if bold { "Courier-Bold" } else { "Courier" }
    } else if family.contains("sans") || family.contains("helvetica") || family.contains("arial") {
        if bold { "Helvetica-Bold" } else { "Helvetica" }
    } else if bold {
        "Times-Bold"
    } else {
        "Times-Roman"
    }
}

/// A CSS font stack built from the settings family with a safe serif fallback.
fn css_font_family(family: &str) -> String {
    let safe = family.replace('"', "").replace('\\', "");
    format!("\"{}\", serif", safe)
}

/// The style that governs one screenplay element in the export writers.
fn script_style<'a>(styles: &'a ElementStyles, kind: ElementKind) -> &'a ElementStyle {
    match kind {
        ElementKind::SceneHeading => &styles.scene_heading,
        ElementKind::Action | ElementKind::Shot => &styles.action,
        ElementKind::Character => &styles.character,
        ElementKind::Parenthetical => &styles.parenthetical,
        ElementKind::Dialogue | ElementKind::Lyric => &styles.dialogue,
        ElementKind::Transition => &styles.transition,
        ElementKind::Act => &styles.act,
        ElementKind::Scene => &styles.scene,
        ElementKind::StageDirection => &styles.stage_direction,
        ElementKind::General => &styles.general,
    }
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

fn build_pdf(
    title: &str,
    author: &str,
    pages: &[PdfPage],
    width_pt: f32,
    height_pt: f32,
    footer: bool,
    header: bool,
    font: &str,
    font_bold: &str,
) -> Vec<u8> {
    // Stable object plan. A PDF page is a dictionary, and its content stream is
    // a *separate* object; mixing the two up makes readers treat a stream as a
    // page and silently drop text. Keep the numbering explicit:
    //
    //   1 catalog · 2 pages · 3 prose · 4 prose bold
    //   5 Courier · 6 Courier-Bold · 7 page 1 · 8 content 1 · ...
    let n = pages.len();
    let mut objects: Vec<Vec<u8>> = Vec::with_capacity(7 + n * 2);
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(Vec::new()); // Pages, filled once the kids are known.
    objects.push(
        format!("<< /Type /Font /Subtype /Type1 /BaseFont /{} /Encoding /WinAnsiEncoding >>", font)
            .into_bytes(),
    );
    objects.push(
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /{} /Encoding /WinAnsiEncoding >>",
            font_bold
        )
        .into_bytes(),
    );
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>".to_vec());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier-Bold /Encoding /WinAnsiEncoding >>".to_vec());

    let mut kids: Vec<usize> = Vec::with_capacity(n);
    for (i, page) in pages.iter().enumerate() {
        let page_num = 7 + i * 2;
        let content_num = page_num + 1;
        kids.push(page_num);

        let mut content = String::new();
        for line in &page.lines {
            content.push_str("BT\n");
            let font = match (line.mono, line.bold) {
                (true, true) => "F4",
                (true, false) => "F3",
                (false, true) => "F2",
                (false, false) => "F1",
            };
            content.push_str(&format!("/{} {} Tf\n", font, line.size));
            content.push_str(&format!("1 0 0 1 {:.2} {:.2} Tm\n", line.x, line.y));
            content.push_str(&format!("({}) Tj\nET\n", pdf_text(&line.text)));
        }
        if footer && i > 0 {
            content.push_str("BT\n/F1 9 Tf\n");
            content.push_str(&format!(
                "1 0 0 1 {:.2} 30 Tm\n({}) Tj\nET\n",
                width_pt / 2.0 - 3.0,
                i + 1
            ));
        }
        if header && i > 0 {
            // A quiet running head in the top margin. It is drawn in the same
            // content stream as the page, so the object plan stays untouched.
            let head = pdf_text(title);
            let x = ((width_pt - head.chars().count() as f32 * 4.5) / 2.0).max(20.0);
            content.push_str("BT\n/F1 9 Tf\n");
            content.push_str(&format!(
                "1 0 0 1 {:.2} {:.2} Tm\n({}) Tj\nET\n",
                x,
                height_pt - 30.0,
                head
            ));
        }

        let page_obj = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 6 0 R >> >> /Contents {} 0 R >>",
            width_pt, height_pt, content_num
        );
        objects.push(page_obj.into_bytes());
        objects.push(
            format!(
                "<< /Length {} >>\nstream\n{}endstream",
                content.len(),
                content
            )
            .into_bytes(),
        );
    }

    let kids_refs: Vec<String> = kids.iter().map(|n| format!("{n} 0 R")).collect();
    objects[1] = format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids_refs.join(" "),
        n
    )
    .into_bytes();

    // Info object.
    let info_num = objects.len() + 1;
    objects.push(
        format!(
            "<< /Title ({}) /Author ({}) /Producer (Portmanteau) /Creator (Portmanteau) >>",
            pdf_text(title),
            pdf_text(author)
        )
        .into_bytes(),
    );

    let mut pdf: Vec<u8> = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(obj);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info {} 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            info_num,
            xref
        )
        .as_bytes(),
    );
    pdf
}

struct PdfFlow {
    pages: Vec<PdfPage>,
    y: f32,
    top: f32,
    bottom: f32,
    page_height: f32,
    line_gap: f32,
    current: PdfPage,
}

impl PdfFlow {
    fn new(page_height: f32, top_margin: f32, bottom_margin: f32, line_gap: f32) -> Self {
        Self {
            pages: Vec::new(),
            y: page_height - top_margin,
            top: top_margin,
            bottom: bottom_margin,
            page_height,
            line_gap,
            current: PdfPage::new(),
        }
    }

    fn finish(mut self) -> Vec<PdfPage> {
        if !self.current.lines.is_empty() {
            self.pages.push(self.current);
        }
        self.pages
    }

    fn ensure(&mut self, needed: f32) {
        if self.y - needed < self.bottom {
            self.pages.push(std::mem::replace(&mut self.current, PdfPage::new()));
            self.y = self.page_height - self.top;
        }
    }

    fn line(&mut self, text: String, x: f32, size: f32, bold: bool, gap_before: f32) {
        let advance = self.line_gap.max(size + 3.0);
        self.ensure(advance + gap_before);
        self.y -= gap_before;
        self.y -= advance;
        self.current.lines.push(PdfLine {
            text,
            x,
            y: self.y,
            size,
            bold,
            mono: false,
        });
    }

    /// A screenplay line: Courier, a baseline just past its type size, and its
    /// own indent.
    fn script_line(&mut self, text: String, x: f32, size: f32, bold: bool, gap_before: f32) {
        let advance = size + 1.0;
        self.ensure(advance + gap_before);
        self.y -= gap_before;
        self.y -= advance;
        self.current.lines.push(PdfLine {
            text,
            x,
            y: self.y,
            size,
            bold,
            mono: true,
        });
    }

    fn page_break(&mut self) {
        if !self.current.lines.is_empty() {
            self.pages.push(std::mem::replace(&mut self.current, PdfPage::new()));
        }
        self.y = self.page_height - self.top;
    }
}

fn pdf_manuscript(project: &Project, settings: &ExportSettings) -> Vec<u8> {
    let (w_mm, h_mm) = settings.paper.mm();
    let mut w = w_mm * 2.834645;
    let mut h = h_mm * 2.834645;
    let screenplay = is_screenplay(project);
    let mut margin = 20.0 * 2.834645;
    let mut top = 22.0 * 2.834645;
    let mut bottom = 22.0 * 2.834645;
    // A screenplay is a working document, not a trim size: keep the industry
    // standard US Letter and 1.5" left margin even when the book is set to A5.
    if screenplay {
        w = 8.5 * 72.0;
        h = 11.0 * 72.0;
        margin = 1.5 * 72.0;
        top = 1.0 * 72.0;
        bottom = 1.0 * 72.0;
    }
    let size = settings.styles.body.size.max(8.0);
    let gap = size * settings.styles.body.leading.max(1.0);
    let mut flow = PdfFlow::new(h, top, bottom, gap);
    let mut pages: Vec<PdfPage> = Vec::new();
    let content_width = w - margin * 2.0;

    if settings.include_title_page {
        let title_style = &settings.styles.title_page_title;
        let subtitle_style = &settings.styles.title_page_subtitle;
        let author_style = &settings.styles.title_page_author;
        let title_size = title_style.size.max(8.0);
        let subtitle_size = subtitle_style.size.max(8.0);
        let author_size = author_style.size.max(8.0);
        let mut page = PdfPage::new();
        // Title, subtitle and byline get their own vertical bands. The earlier
        // version moved the cursor down and then subtracted the offset again,
        // which drew the subtitle on top of the title.
        let mut y = h * 0.34;
        for line in wrap_text(&project.title, 34) {
            page.lines.push(PdfLine {
                text: line.clone(),
                x: w / 2.0 - line.chars().count() as f32 * title_size * 0.24,
                y,
                size: title_size,
                bold: true,
                mono: false,
            });
            y -= title_size * 1.25;
        }
        y -= 18.0;
        if !project.subtitle.is_empty() {
            for line in wrap_text(&project.subtitle, 50) {
                page.lines.push(PdfLine {
                    text: line.clone(),
                    x: w / 2.0 - line.chars().count() as f32 * subtitle_size * 0.24,
                    y,
                    size: subtitle_size,
                    bold: false,
                    mono: false,
                });
                y -= 18.0;
            }
        }
        y -= 34.0;
        let byline = format!("by {}", project.author);
        page.lines.push(PdfLine {
            text: byline.clone(),
            x: w / 2.0 - byline.chars().count() as f32 * author_size * 0.24,
            y,
            size: author_size,
            bold: false,
            mono: false,
        });
        let footer = if screenplay {
            "Portmanteau · screenplay".to_string()
        } else {
            format!("{} · {}", project.genre, chrono::Local::now().format("%Y"))
        };
        page.lines.push(PdfLine {
            x: w / 2.0 - footer.chars().count() as f32 * 2.3,
            text: footer,
            y: 70.0,
            size: 9.0,
            bold: false,
            mono: false,
        });
        pages.push(page);
    }

    if screenplay {
        // Screenplay layout: Courier-like fixed columns, scene numbers optional.
        let mut page = PdfPage::new();
        let mut line_index = 0usize;
        let mut scene_number = 1usize;
        let max_lines = ((h - top - bottom) / 13.0) as usize;
        let char_w = 7.2; // 12pt Courier at 0.6em
        let full_chars = (content_width / char_w).floor().max(20.0) as usize;
        let width_for = |indent_mm: f32| {
            ((content_width - indent_mm * 2.834645) / char_w)
                .floor()
                .max(14.0) as usize
        };
        fn push_line(
            page: &mut PdfPage,
            line_index: &mut usize,
            h: f32,
            top: f32,
            margin: f32,
            text: String,
            x_mm: f32,
            size: f32,
            bold: bool,
        ) {
            let y = h - top - (*line_index as f32) * 13.0;
            page.lines.push(PdfLine {
                text,
                x: x_mm * 2.834645 + margin,
                y,
                size,
                bold,
                mono: true,
            });
            *line_index += 1;
        }
        for section in manuscript_sections(project) {
            let doc = section.doc;
            if doc.medium.is_script() {
                let heading = format!("{}.", section.heading.to_uppercase());
                for line in wrap_text(&heading, full_chars) {
                    push_line(&mut page, &mut line_index, h, top, margin, line, 0.0, 12.0, true);
                }
                push_line(
                    &mut page,
                    &mut line_index,
                    h,
                    top,
                    margin,
                    String::new(),
                    0.0,
                    12.0,
                    false,
                );
                for block in &doc.blocks {
                    let style = script_style(&settings.styles, block.kind);
                    let block_size = style.size.max(8.0);
                    let indent_mm = style.indent / 2.834645;
                    let width_chars = width_for(indent_mm);
                    let text = if block.kind.is_uppercase() {
                        block.text.to_uppercase()
                    } else {
                        block.text.clone()
                    };
                    if block.kind == ElementKind::SceneHeading && settings.include_scene_numbers {
                        for (i, wrapped) in
                            wrap_text(&text, full_chars.saturating_sub(4)).iter().enumerate()
                        {
                            let line = if i == 0 {
                                format!("{}   {}", scene_number, wrapped)
                            } else {
                                wrapped.clone()
                            };
                            push_line(
                                &mut page,
                                &mut line_index,
                                h,
                                top,
                                margin,
                                line,
                                0.0,
                                block_size,
                                false,
                            );
                        }
                        scene_number += 1;
                    } else {
                        for wrapped in wrap_text(&text, width_chars) {
                            push_line(
                                &mut page,
                                &mut line_index,
                                h,
                                top,
                                margin,
                                wrapped,
                                indent_mm,
                                block_size,
                                false,
                            );
                        }
                    }
                    if !block.kind.is_dialogue() {
                        push_line(
                            &mut page,
                            &mut line_index,
                            h,
                            top,
                            margin,
                            String::new(),
                            0.0,
                            12.0,
                            false,
                        );
                    }
                    if line_index >= max_lines {
                        pages.push(std::mem::replace(&mut page, PdfPage::new()));
                        line_index = 0;
                    }
                }
            } else {
                for line in wrap_text(&section.heading.to_uppercase(), full_chars) {
                    push_line(&mut page, &mut line_index, h, top, margin, line, 0.0, 12.0, true);
                }
                push_line(
                    &mut page,
                    &mut line_index,
                    h,
                    top,
                    margin,
                    String::new(),
                    0.0,
                    12.0,
                    false,
                );
                for para in paragraphs(&doc.body_text()) {
                    for line in wrap_text(&para, full_chars) {
                        push_line(
                            &mut page,
                            &mut line_index,
                            h,
                            top,
                            margin,
                            line,
                            0.0,
                            12.0,
                            false,
                        );
                    }
                    push_line(
                        &mut page,
                        &mut line_index,
                        h,
                        top,
                        margin,
                        String::new(),
                        0.0,
                        12.0,
                        false,
                    );
                }
            }
            if line_index >= max_lines - 3 {
                pages.push(std::mem::replace(&mut page, PdfPage::new()));
                line_index = 0;
            }
        }
        if !page.lines.is_empty() {
            pages.push(page);
        }
        let _ = char_w;
        return build_pdf(
            &project.title,
            &project.author,
            &pages,
            w,
            h,
            settings.include_page_numbers,
            settings.include_running_headers,
            pdf_font(&settings.styles.body.font, false),
            pdf_font(&settings.styles.body.font, true),
        );
    }

    // Prose: flowing layout with chapter headings and optional front matter.
    // Times-Roman averages roughly half an em per character. The old formula
    // mixed millimetres and points and let lines run past the page edge.
    let chars_per_line = (content_width / (size * 0.50)).max(30.0) as usize;
    let mut headings: Vec<(String, usize)> = Vec::new();
    let add_para = |flow: &mut PdfFlow, text: &str, bold: bool, indent: f32| {
        for part in wrap_text(text, chars_per_line) {
            flow.line(part, margin + indent, size, bold, 0.0);
        }
    };

    for doc in front_matter(project, settings) {
        flow.line(doc.title.clone(), margin, size + 3.0, true, 12.0);
        for para in paragraphs(&doc.body_text()) {
            add_para(&mut flow, &para, false, 0.0);
            flow.line(String::new(), margin, size, false, 0.0);
        }
        flow.page_break();
    }

    let sections = manuscript_sections(project);
    for (i, section) in sections.iter().enumerate() {
        flow.page_break();
        let title = section_title(project, section.doc, i, settings);
        headings.push((title.clone(), flow.pages.len() + 1));
        let head_size = settings.styles.chapter_heading.size.max(size);
        let mut first_heading_line = true;
        for part in wrap_text(&title, chars_per_line) {
            let gap_before = if first_heading_line {
                settings.styles.chapter_heading.before
            } else {
                0.0
            };
            flow.line(part, margin, head_size, true, gap_before);
            first_heading_line = false;
        }
        flow.line(
            String::new(),
            margin,
            size,
            false,
            settings.styles.chapter_heading.after,
        );
        if settings.include_synopsis && !section.doc.synopsis.is_empty() {
            for part in wrap_text(&format!("Synopsis · {}", section.doc.synopsis), chars_per_line) {
                flow.line(part, margin, size - 1.0, false, 0.0);
            }
            flow.line(String::new(), margin, size, false, 0.0);
        }
        if section.doc.medium.is_script() {
            // Mixed projects: a script section keeps Courier columns even when
            // the surrounding manuscript is prose. This is what makes a novel
            // with a companion pilot read correctly in one PDF.
            let mut scene_number = 1usize;
            for block in &section.doc.blocks {
                let style = script_style(&settings.styles, block.kind);
                let block_size = style.size.max(8.0);
                let indent_mm = style.indent / 2.834645;
                let width = ((content_width - indent_mm * 2.834645) / 7.2)
                    .floor()
                    .max(14.0) as usize;
                let text = if block.kind.is_uppercase() {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                if block.kind == ElementKind::SceneHeading
                    && settings.include_scene_numbers
                {
                    for (i, part) in wrap_text(&text, width.saturating_sub(4)).iter().enumerate() {
                        let line = if i == 0 {
                            format!("{}   {}", scene_number, part)
                        } else {
                            part.clone()
                        };
                        flow.script_line(line, margin, block_size, false, 0.0);
                    }
                    scene_number += 1;
                } else {
                    for part in wrap_text(&text, width) {
                        flow.script_line(
                            part,
                            margin + indent_mm * 2.834645,
                            block_size,
                            false,
                            0.0,
                        );
                    }
                }
                // A blank line closes each beat. Character and parenthetical
                // lines stay tight to their dialogue; everything else gets air.
                if !matches!(
                    block.kind,
                    ElementKind::Character | ElementKind::Parenthetical
                ) {
                    flow.script_line(String::new(), margin, 12.0, false, 0.0);
                }
            }
        } else {
            for para in paragraphs(&section.doc.body_text()) {
                add_para(&mut flow, &para, false, 0.0);
                flow.line(String::new(), margin, size, false, 0.0);
            }
        }
        if settings.include_notes && !section.doc.notes.is_empty() {
            for part in wrap_text(&format!("Author's note · {}", section.doc.notes), chars_per_line) {
                flow.line(part, margin, size - 1.0, false, 0.0);
            }
        }
    }
    for doc in back_matter(project, settings) {
        flow.page_break();
        flow.line(doc.title.clone(), margin, size + 3.0, true, 12.0);
        for para in paragraphs(&doc.body_text()) {
            add_para(&mut flow, &para, false, 0.0);
            flow.line(String::new(), margin, size, false, 0.0);
        }
    }
    pages.extend(flow.finish());

    if settings.include_toc && !headings.is_empty() {
        // The title page (when present) sits before the flow, and the contents
        // page will be inserted before the flow too. Both shift the printed
        // number of every chapter by one, so convert flow page numbers to the
        // reader's page numbers before drawing the list.
        let title_offset = usize::from(settings.include_title_page && !pages.is_empty());
        let mut toc = PdfPage::new();
        toc.lines.push(PdfLine {
            text: "Contents".into(),
            x: margin,
            y: h - top - 20.0,
            size: size + 6.0,
            bold: true,
            mono: false,
        });
        let mut y = top + 50.0;
        for (title, page) in &headings {
            let printed = page + title_offset + 1;
            toc.lines.push(PdfLine {
                text: title.clone(),
                x: margin,
                y: h - y,
                size,
                bold: false,
                mono: false,
            });
            toc.lines.push(PdfLine {
                text: printed.to_string(),
                x: w - margin - 20.0,
                y: h - y,
                size,
                bold: false,
                mono: false,
            });
            y += size * 1.8;
            if y > h - bottom {
                break;
            }
        }
        let insert_at = if title_offset == 1 { 1 } else { 0 };
        pages.insert(insert_at.min(pages.len()), toc);
    }

    build_pdf(
        &project.title,
        &project.author,
        &pages,
        w,
        h,
        settings.include_page_numbers,
        settings.include_running_headers,
        pdf_font(&settings.styles.body.font, false),
        pdf_font(&settings.styles.body.font, true),
    )
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
        let css = format!(
            r#"body {{ font-family: {body_font}; line-height: {body_leading:.2}; margin: 5%; color: #1b1b1b; }}
h1 {{ font-size: {chapter_size:.1}pt; margin: 2em 0 1em; page-break-before: always; }}
h2 {{ font-size: 1.25em; margin: 1.5em 0 .8em; }}
p {{ margin: 0 0 0 .2em; text-indent: 1.4em; text-align: justify; }}
p.first, .front p {{ text-indent: 0; }}
.center {{ text-align: center; text-indent: 0; }}
.scene {{ font-family: {scene_font}; font-size: {scene_size:.1}pt; text-transform: uppercase; font-weight: bold; margin-top: 1.5em; text-indent: 0; }}
.action {{ font-family: {action_font}; font-size: {action_size:.1}pt; margin: .5em 0 .8em; text-indent: 0; }}
.character {{ font-family: {character_font}; font-size: {character_size:.1}pt; margin: 1em 0 0 {character_indent}; text-indent: 0; text-transform: uppercase; }}
.parenthetical {{ font-family: {parenthetical_font}; font-size: {parenthetical_size:.1}pt; margin: 0 0 0 {parenthetical_indent}; text-indent: 0; }}
.dialogue {{ font-family: {dialogue_font}; font-size: {dialogue_size:.1}pt; margin: 0 10% .7em {dialogue_indent}; text-indent: 0; }}
.transition {{ font-family: {transition_font}; font-size: {transition_size:.1}pt; text-align: right; text-indent: 0; }}
.title-page {{ text-align: center; margin-top: 26%; }}
.title-page h1 {{ page-break-before: avoid; font-size: {title_size:.1}pt; }}
.title-page .subtitle {{ font-size: {subtitle_size:.1}pt; }}
.byline {{ font-size: {author_size:.1}pt; margin-top: 1.5em; }}
.imprint {{ margin-top: 4em; font-size: .85em; color: #555; }}
.cover {{ text-align: center; margin: 0; padding: 0; }}
.cover img {{ max-width: 100%; }}
"#,
            body_font = body_font,
            body_leading = st.body.leading.max(1.0),
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
                xhtml_body(doc, None),
            ));
        }
        for (i, section) in manuscript_sections(project).iter().enumerate() {
            chapter_no += 1;
            let title = section_title(project, section.doc, i, settings);
            files.push((
                format!("chapter-{:03}.xhtml", chapter_no),
                title.clone(),
                xhtml_body(section.doc, Some(&title)),
            ));
        }
        for doc in back_matter(project, settings) {
            files.push((
                format!("back-{}.xhtml", slugify(&doc.title)),
                doc.title.clone(),
                xhtml_body(doc, None),
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

fn xhtml_body(doc: &Document, heading: Option<&str>) -> String {
    let mut out = String::new();
    if let Some(h) = heading {
        out.push_str(&format!("<h1>{}</h1>\n", xml_escape(h)));
    } else {
        out.push_str(&format!("<h2>{}</h2>\n", xml_escape(&doc.title)));
    }
    if doc.medium.is_script() {
        for block in &doc.blocks {
            let class = match block.kind {
                ElementKind::SceneHeading => "scene",
                ElementKind::Action | ElementKind::Shot => "action",
                ElementKind::Character => "character",
                ElementKind::Parenthetical => "parenthetical",
                ElementKind::Dialogue => "dialogue",
                ElementKind::Transition => "transition",
                ElementKind::Act => "scene",
                ElementKind::Scene => "scene",
                ElementKind::StageDirection => "action",
                ElementKind::Lyric => "dialogue",
                ElementKind::General => "action",
            };
            let text = if block.kind.is_uppercase() {
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
        for (i, para) in paragraphs(&doc.body_text()).iter().enumerate() {
            out.push_str(&format!(
                "<p class=\"{}\">{}</p>\n",
                if i == 0 { "first" } else { "" },
                html_escape(para)
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// DOCX
// ---------------------------------------------------------------------------

fn docx_manuscript(project: &Project, settings: &ExportSettings) -> Vec<u8> {
    let mut body = String::new();
    let prose_font = xml_escape(&settings.font);
    let mut add_para = |text: &str, style: &str, bold: bool, center: bool, mono: bool, indent: i32| {
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
    };

    if settings.include_title_page {
        add_para(&project.title, "Title", true, true, false, 0);
        if !project.subtitle.is_empty() {
            add_para(&project.subtitle, "Subtitle", false, true, false, 0);
        }
        add_para(&format!("by {}", project.author), "Normal", false, true, false, 0);
        add_para("", "Normal", false, false, false, 0);
    }
    for doc in front_matter(project, settings) {
        add_para(&doc.title, "Heading1", true, false, false, 0);
        for para in paragraphs(&doc.body_text()) {
            add_para(&para, "Normal", false, false, false, 0);
        }
    }
    for (i, section) in manuscript_sections(project).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        add_para(&title, "Heading1", true, false, false, 0);
        if section.doc.medium.is_script() {
            for block in &section.doc.blocks {
                let (indent, mono, bold) = match block.kind {
                    ElementKind::Character => (4320, true, false),
                    ElementKind::Parenthetical => (3240, true, false),
                    ElementKind::Dialogue => (2160, true, false),
                    ElementKind::SceneHeading => (0, true, false),
                    _ => (0, true, false),
                };
                let text = if block.kind.is_uppercase() {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                add_para(
                    &text,
                    if block.kind == ElementKind::SceneHeading { "Heading2" } else { "Normal" },
                    bold,
                    block.kind == ElementKind::Transition,
                    mono,
                    indent,
                );
            }
        } else {
            for para in paragraphs(&section.doc.body_text()) {
                add_para(&para, "Normal", false, false, false, 0);
            }
        }
    }
    for doc in back_matter(project, settings) {
        add_para(&doc.title, "Heading1", true, false, false, 0);
        for para in paragraphs(&doc.body_text()) {
            add_para(&para, "Normal", false, false, false, 0);
        }
    }

    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{}\n<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr></w:body></w:document>",
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
    }
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
    for section in manuscript_sections(project) {
        if section.doc.medium.is_script() {
            for block in &section.doc.blocks {
                let text = if block.kind.is_uppercase() {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                out.push_str(&format!(
                    "    <Paragraph Type=\"{}\"><Text>{}</Text></Paragraph>\n",
                    fdx_type(block.kind),
                    xml_escape(&text)
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
    for section in manuscript_sections(project) {
        if section.doc.medium.is_script() {
            out.push_str(&format!("# {}\n\n", section.heading));
            out.push_str(&script::to_fountain(&section.doc.blocks, section.doc.medium));
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
        for (i, section) in manuscript_sections(project).iter().enumerate() {
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
        for para in paragraphs(&doc.body_text()) {
            body.push_str(&format!("<p>{}</p>\n", html_escape(&para)));
        }
        body.push_str("</section>\n");
    }
    for (i, section) in manuscript_sections(project).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        body.push_str(&format!(
            "<section id=\"s{}\" class=\"chapter\"><h2>{}</h2>\n",
            i,
            html_escape(&title)
        ));
        if section.doc.medium.is_script() {
            body.push_str("<div class=\"script\">\n");
            for block in &section.doc.blocks {
                let class = match block.kind {
                    ElementKind::SceneHeading => "scene",
                    ElementKind::Character => "character",
                    ElementKind::Parenthetical => "paren",
                    ElementKind::Dialogue => "dialogue",
                    ElementKind::Transition => "trans",
                    _ => "action",
                };
                let text = if block.kind.is_uppercase() {
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
            for (j, para) in paragraphs(&section.doc.body_text()).iter().enumerate() {
                body.push_str(&format!(
                    "<p class=\"{}\">{}</p>\n",
                    if j == 0 { "first" } else { "" },
                    html_escape(para)
                ));
            }
        }
        body.push_str("</section>\n");
    }
    for doc in back_matter(project, settings) {
        body.push_str(&format!(
            "<section class=\"backmatter\"><h2>{}</h2>\n",
            html_escape(&doc.title)
        ));
        for para in paragraphs(&doc.body_text()) {
            body.push_str(&format!("<p>{}</p>\n", html_escape(&para)));
        }
        body.push_str("</section>\n");
    }
    let st = &settings.styles;
    let body_font = element_css_font(&st.body, "Liberation Serif");
    let script_font = element_css_font(&st.action, "Courier New");
    let chapter_size = st.chapter_heading.size.max(8.0);
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\"/>\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/>\n<title>{title}</title>\n<style>\nbody{{max-width:42rem;margin:4rem auto;padding:0 1.5rem;font-family:{body_font};line-height:{body_leading:.2};color:#1e1c19;background:#fbf8f1}}\nh1{{font-size:{h1_size:.1}pt;line-height:1.1}}h2{{font-size:{chapter_size:.1}pt;margin-top:3rem;border-bottom:1px solid #d8d0c0;padding-bottom:.4rem}}\np{{margin:0 0 .9rem;text-indent:1.4rem;text-align:justify}}p.first{{text-indent:0}}\n.titlepage{{text-align:center;margin:18vh 0 22vh}}.titlepage h1{{font-size:{title_size:.1}pt;margin-bottom:.3em}}.titlepage p{{text-indent:0}}.sub{{font-style:italic;color:#665f54}}\n.toc ol{{line-height:2}}.toc a{{color:#2f5d50;text-decoration:none}}\n.script{{font-family:{script_font};font-size:{script_size:.1}pt}}.script p{{text-indent:0;margin:.4rem 0;text-align:left}}\n.scene{{text-transform:uppercase;font-weight:bold;margin-top:1.6rem!important}}.character{{margin-left:32%!important;text-transform:uppercase;margin-top:1rem!important}}.paren{{margin-left:24%!important}}.dialogue{{margin-left:16%!important;margin-right:12%!important}}.trans{{text-align:right!important}}\n.frontmatter,.backmatter{{margin-top:4rem}}\n</style>\n</head>\n<body>\n{body}\n</body>\n</html>\n",
        title = html_escape(&project.title),
        body_font = body_font,
        body_leading = st.body.leading.max(1.0),
        h1_size = chapter_size * 1.5,
        chapter_size = chapter_size,
        title_size = st.title_page_title.size.max(8.0),
        script_font = script_font,
        script_size = st.action.size.max(8.0),
        body = body,
    )
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
        for (i, section) in manuscript_sections(project).iter().enumerate() {
            let title = section_title(project, section.doc, i, settings);
            out.push_str(&format!("{}. {}\n", i + 1, title));
        }
        out.push('\n');
    }
    for doc in front_matter(project, settings) {
        out.push_str(&format!("## {}\n\n", doc.title));
        for para in paragraphs(&doc.body_text()) {
            out.push_str(&para);
            out.push_str("\n\n");
        }
    }
    for (i, section) in manuscript_sections(project).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        out.push_str(&format!("## {}\n\n", title));
        if section.doc.medium.is_script() {
            out.push_str("```fountain\n");
            out.push_str(&script::to_fountain(&section.doc.blocks, section.doc.medium));
            out.push_str("```\n\n");
        } else {
            for para in paragraphs(&section.doc.body_text()) {
                out.push_str(&para);
                out.push_str("\n\n");
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&format!("## {}\n\n", doc.title));
        for para in paragraphs(&doc.body_text()) {
            out.push_str(&para);
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

fn rtf_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{{\\rtf1\\ansi\\deff0{{\\fonttbl{{\\f0 {};}}{{\\f1 Courier New;}}}}\n",
        rtf_escape(&settings.font)
    ));
    out.push_str("\\paperw11906\\paperh16838\\margl1440\\margr1440\\margt1440\\margb1440\n");
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
        for para in paragraphs(&doc.body_text()) {
            out.push_str(&format!("{}\\par\n", rtf_escape(&para)));
        }
    }
    for (i, section) in manuscript_sections(project).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        out.push_str(&format!(
            "\\page\\f0\\fs36\\b {}\\b0\\fs24\\par\n",
            rtf_escape(&title)
        ));
        if section.doc.medium.is_script() {
            for block in &section.doc.blocks {
                let indent = match block.kind {
                    ElementKind::Character => 4400,
                    ElementKind::Parenthetical => 3200,
                    ElementKind::Dialogue => 2000,
                    _ => 0,
                };
                let text = if block.kind.is_uppercase() {
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
            for para in paragraphs(&section.doc.body_text()) {
                out.push_str(&format!("{}\\par\n", rtf_escape(&para)));
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&format!(
            "\\page\\f0\\fs32\\b {}\\b0\\fs24\\par\n",
            rtf_escape(&doc.title)
        ));
        for para in paragraphs(&doc.body_text()) {
            out.push_str(&format!("{}\\par\n", rtf_escape(&para)));
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
        for para in paragraphs(&doc.body_text()) {
            out.push_str(&format!("{}\n\n", para));
        }
    }
    for (i, section) in manuscript_sections(project).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        out.push_str(&format!("\n{}\n{}\n\n", title.to_uppercase(), "-".repeat(title.len())));
        if section.doc.medium.is_script() {
            for block in &section.doc.blocks {
                out.push_str(&script::block_to_plain(block));
                out.push_str("\n\n");
            }
        } else {
            for para in paragraphs(&section.doc.body_text()) {
                out.push_str(&format!("{}\n\n", para));
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&format!("\n{}\n\n", doc.title.to_uppercase()));
        for para in paragraphs(&doc.body_text()) {
            out.push_str(&format!("{}\n\n", para));
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
    fn pdf_has_signature_and_pages() {
        let p = demo();
        let mut s = p.export.clone();
        s.kind = ExportKind::Pdf;
        let bytes = pdf_manuscript(&p, &s);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Type /Page"));
        assert!(text.contains("The Salt Road"));
        assert!(text.contains("startxref"));
        assert!(text.contains("%%EOF"));
    }

    #[test]
    fn pdf_page_objects_point_at_real_page_streams() {
        let p = demo();
        let mut s = p.export.clone();
        s.kind = ExportKind::Pdf;
        let bytes = pdf_manuscript(&p, &s);
        let doc = String::from_utf8_lossy(&bytes);

        // Pull the /Kids list out of the page tree and prove every reference is
        // a page dictionary, not a content stream. This is the exact mistake
        // that made readers report "Kid object is wrong type (stream)" and drop
        // the ends of paragraphs.
        let kids_at = doc.find("/Type /Pages /Kids [").expect("page tree");
        let kids_start = kids_at + "/Type /Pages /Kids [".len();
        let kids_end = doc[kids_start..].find(']').unwrap() + kids_start;
        let kids = &doc[kids_start..kids_end];
        let numbers: Vec<&str> = kids.split_whitespace().filter(|s| *s != "0" && *s != "R").collect();
        assert!(numbers.len() >= 2, "expected several pages, got {:?}", numbers);

        for n in &numbers {
            let marker = format!("\n{} 0 obj\n", n);
            let at = doc.find(&marker).unwrap_or_else(|| panic!("object {} missing", n));
            let body = &doc[at..];
            assert!(
                body.starts_with(&format!("\n{} 0 obj\n<< /Type /Page ", n)),
                "kid {} is not a /Type /Page dictionary",
                n
            );
            let content_at = body.find("/Contents ").expect("contents ref");
            let after = &body[content_at + "/Contents ".len()..];
            let content_num: String = after
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            assert!(!content_num.is_empty(), "page {} has no content object", n);
            let content_marker = format!("\n{} 0 obj\n", content_num);
            let content_pos = doc.find(&content_marker).expect("content object");
            let content_body = &doc[content_pos..];
            assert!(
                content_body.contains("stream"),
                "contents {} is not a stream",
                content_num
            );
        }

        // Each page dictionary names a media box; a missing box is another
        // common way malformed hand-built PDFs break readers.
        assert_eq!(doc.matches("/MediaBox").count(), numbers.len());
    }

    #[test]
    fn pdf_uses_courier_for_scripts_and_times_for_prose() {
        let p = demo();
        let mut s = p.export.clone();
        s.kind = ExportKind::Pdf;
        let text = String::from_utf8_lossy(&pdf_manuscript(&p, &s)).to_string();
        assert!(text.contains("/BaseFont /Times-Roman"));
        assert!(text.contains("/BaseFont /Courier"));
        // The demonstration project mixes a novel with a television pilot and
        // a stage play, so both fonts must actually be selected in content.
        assert!(text.contains("/F1 "));
        assert!(text.contains("/F3 "));
    }

    #[test]
    fn pdf_font_maps_family_names_to_base14_fonts() {
        assert_eq!(pdf_font("Courier New", false), "Courier");
        assert_eq!(pdf_font("Liberation Serif", true), "Times-Bold");
        assert_eq!(pdf_font("Liberation Sans", false), "Helvetica");
    }

    #[test]
    fn pdf_base_font_follows_the_body_style() {
        let p = demo();
        let mut s = p.settings_for(ExportKind::Pdf);
        s.styles.body.font = "Courier New".into();
        let text = String::from_utf8_lossy(&pdf_manuscript(&p, &s)).to_string();
        assert!(text.contains("/BaseFont /Courier "), "prose F1 should use Courier");
        assert!(
            text.contains("/BaseFont /Courier-Bold "),
            "prose F2 should use Courier-Bold"
        );
    }

    #[test]
    fn pdf_title_page_uses_the_title_style_size() {
        let p = demo();
        let mut s = p.settings_for(ExportKind::Pdf);
        s.include_title_page = true;
        s.styles.title_page_title.size = 30.0;
        let text = String::from_utf8_lossy(&pdf_manuscript(&p, &s)).to_string();
        assert!(
            text.contains("F2 30 Tf"),
            "title should be drawn at 30pt in the bold prose face"
        );
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
    fn fdx_is_wellformed_ish() {
        let p = demo();
        let fdx = fdx_manuscript(&p, &p.export);
        assert!(fdx.contains("<FinalDraft"));
        assert!(fdx.contains("Scene Heading"));
    }

    #[test]
    fn pdf_page_numbers_follow_the_setting() {
        let p = demo();
        let mut on = p.settings_for(ExportKind::Pdf);
        on.include_page_numbers = true;
        on.include_running_headers = false;
        let mut off = on.clone();
        off.include_page_numbers = false;

        let with_numbers = String::from_utf8_lossy(&pdf_manuscript(&p, &on)).into_owned();
        let without_numbers = String::from_utf8_lossy(&pdf_manuscript(&p, &off)).into_owned();
        assert!(with_numbers.contains(" 30 Tm"), "footer page number missing");
        assert!(
            !without_numbers.contains(" 30 Tm"),
            "footer page number should be omitted"
        );
        assert_ne!(with_numbers, without_numbers);
    }

    #[test]
    fn pdf_running_headers_are_optional() {
        let mut p = demo();
        p.title = "Header Probe".into();
        let mut off = p.settings_for(ExportKind::Pdf);
        off.include_running_headers = false;
        let mut on = off.clone();
        on.include_running_headers = true;

        let without = String::from_utf8_lossy(&pdf_manuscript(&p, &off)).into_owned();
        let with = String::from_utf8_lossy(&pdf_manuscript(&p, &on)).into_owned();
        assert!(
            with.matches("Header Probe").count() > without.matches("Header Probe").count(),
            "running headers should repeat the title on body pages"
        );
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
}
