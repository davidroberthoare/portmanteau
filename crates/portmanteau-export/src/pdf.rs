//! PDF layout and drawing.
//!
//! Layout is deliberately separate from drawing. The layout step turns the
//! compiled manuscript into positioned lines and rules on fixed-size pages; it
//! has no PDF code and is shared by the PDF writer, the compile screen's page
//! estimate and the automated measurements in `tests`. Drawing then hands
//! finished lines to `krilla`, which embeds and subsets the bundled fonts and
//! writes the file.
//!
//! Scripts are laid out from the document's own stylesheet (film, TV or stage),
//! not from a single generic "screenplay" look. The compiler never invents
//! paragraphs: transitions such as `FADE IN:` are printed only when they are
//! real blocks in the document.

use portmanteau_core::{
    Alignment, Block, DocKind, Document, ElementKind, ElementStyle, ExportSettings, InlineContent,
    InlineMark, Project, TextTransform,
};
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::{FontRef, MetadataProvider};

use crate::{
    back_matter, front_matter, manuscript_sections, page_margins, paper_pt, prose_paragraphs,
    script_scale, script_sheet, section_title, ProseParagraph, SceneNumbers, script_cover_lines, ascent_pt, TitleRole, starts_new_page, TITLE_PAGE_DROP,
};

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

const SERIF_REGULAR: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/LiberationSerif-Regular.ttf"));
const SERIF_BOLD: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/LiberationSerif-Bold.ttf"));
const SERIF_ITALIC: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/LiberationSerif-Italic.ttf"));
const SANS_REGULAR: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/LiberationSans-Regular.ttf"));
const SANS_BOLD: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/LiberationSans-Bold.ttf"));
const SANS_ITALIC: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/LiberationSans-Italic.ttf"));
const MONO_REGULAR: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/LiberationMono-Regular.ttf"));
const MONO_BOLD: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/LiberationMono-Bold.ttf"));
const MONO_ITALIC: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/LiberationMono-Italic.ttf"));
const FALLBACK: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/DejaVuSans.ttf"));

/// The font family a style asks for, reduced to the three families we bundle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    Serif,
    Sans,
    Mono,
}

/// One concrete face. `fallback` is only set for characters the chosen face
/// does not carry; it keeps a symbol from disappearing from the page.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Face {
    pub family: Family,
    pub bold: bool,
    pub italic: bool,
    pub fallback: bool,
}

impl Face {
    pub fn new(family: Family, bold: bool, italic: bool) -> Self {
        Self {
            family,
            bold,
            italic,
            fallback: false,
        }
    }

    fn fallback() -> Self {
        Self {
            family: Family::Sans,
            bold: false,
            italic: false,
            fallback: true,
        }
    }

    /// Resolve a style's family name. Empty or unknown names fall back to the
    /// document's body family, as the other writers do.
    pub fn from_style(style: &ElementStyle, fallback_font: &str) -> Self {
        let name = if style.font.trim().is_empty() {
            fallback_font
        } else {
            style.font.as_str()
        };
        Self::new(family_for_name(name), false, style.italic)
    }

    fn with_marks(self, bold: bool, italic: bool) -> Self {
        Self {
            bold: self.bold || bold,
            italic: self.italic || italic,
            ..self
        }
    }

    fn index(self) -> usize {
        if self.fallback {
            return 9;
        }
        match (self.family, self.bold, self.italic) {
            (Family::Serif, true, _) => 1,
            (Family::Serif, false, true) => 2,
            (Family::Serif, false, false) => 0,
            (Family::Sans, true, _) => 4,
            (Family::Sans, false, true) => 5,
            (Family::Sans, false, false) => 3,
            (Family::Mono, true, _) => 7,
            (Family::Mono, false, true) => 8,
            (Family::Mono, false, false) => 6,
        }
    }

}

fn family_for_name(name: &str) -> Family {
    let name = name.to_ascii_lowercase();
    if name.contains("mono") || name.contains("courier") {
        Family::Mono
    } else if name.contains("sans") || name.contains("helvetica") || name.contains("arial") {
        Family::Sans
    } else {
        Family::Serif
    }
}

struct MeasuredFace {
    font: FontRef<'static>,
    metrics: GlyphMetrics<'static>,
    upem: f32,
}

impl MeasuredFace {
    fn new(data: &'static [u8]) -> Self {
        let font = FontRef::from_index(data, 0)
            .unwrap_or_else(|_| panic!("bundled font could not be parsed"));
        let metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());
        let upem = font
            .metrics(Size::unscaled(), LocationRef::default())
            .units_per_em
            .max(1) as f32;
        Self {
            font,
            metrics,
            upem,
        }
    }

    fn has_char(&self, c: char) -> bool {
        self.font.charmap().map(c).is_some()
    }

    fn advance(&self, c: char, size: f32) -> f32 {
        let Some(gid) = self.font.charmap().map(c) else {
            return 0.0;
        };
        self.metrics.advance_width(gid).unwrap_or(0.0) * size / self.upem
    }
}

fn metrics() -> &'static [MeasuredFace; 10] {
    use std::sync::OnceLock;
    static METRICS: OnceLock<[MeasuredFace; 10]> = OnceLock::new();
    METRICS.get_or_init(|| {
        [
            MeasuredFace::new(SERIF_REGULAR),
            MeasuredFace::new(SERIF_BOLD),
            MeasuredFace::new(SERIF_ITALIC),
            MeasuredFace::new(SANS_REGULAR),
            MeasuredFace::new(SANS_BOLD),
            MeasuredFace::new(SANS_ITALIC),
            MeasuredFace::new(MONO_REGULAR),
            MeasuredFace::new(MONO_BOLD),
            MeasuredFace::new(MONO_ITALIC),
            MeasuredFace::new(FALLBACK),
        ]
    })
}

fn face_for_char(c: char, base: Face) -> Face {
    if base.fallback || c.is_whitespace() {
        return base;
    }
    let measured = &metrics()[base.index()];
    if measured.has_char(c) {
        base
    } else {
        Face::fallback()
    }
}

fn char_width(c: char, face: Face, size: f32) -> f32 {
    metrics()[face.index()].advance(c, size)
}

/// Width of a single face's text at a size, in points.
pub fn text_width(text: &str, face: Face, size: f32) -> f32 {
    text.chars().map(|c| char_width(c, face, size)).sum()
}

// ---------------------------------------------------------------------------
// Layout model
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct PdfSegment {
    pub text: String,
    pub face: Face,
}

#[derive(Clone, Debug)]
pub struct PdfText {
    /// Left edge of the text, in points from the page's left edge.
    pub x: f32,
    /// Baseline, in points from the page's top edge (krilla draws y-down).
    pub baseline: f32,
    pub size: f32,
    pub segments: Vec<PdfSegment>,
}

#[derive(Clone, Debug)]
pub struct PdfPageLayout {
    pub items: Vec<PdfText>,
}

#[derive(Clone, Debug)]
pub struct PdfLayoutSection {
    pub label: String,
    /// 0-based index of the section's first page.
    pub start_page: usize,
    pub page_count: usize,
}

#[derive(Clone, Debug)]
pub struct PdfLayout {
    pub width: f32,
    pub height: f32,
    pub pages: Vec<PdfPageLayout>,
    pub sections: Vec<PdfLayoutSection>,
}

impl PdfLayout {
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

/// One styled character. Keeping individual characters lets line breaking mix
/// the requested face with the fallback face for symbols it does not carry.
#[derive(Clone, Copy, Debug)]
struct StyledChar {
    c: char,
    face: Face,
}

fn push_text(chars: &mut Vec<StyledChar>, text: &str, face: Face) {
    for c in text.chars() {
        if c == '\r' {
            continue;
        }
        chars.push(StyledChar {
            c,
            face: face_for_char(c, face),
        });
    }
}

fn chars_width(chars: &[StyledChar], size: f32) -> f32 {
    chars
        .iter()
        .map(|styled| char_width(styled.c, styled.face, size))
        .sum()
}

/// Greedy word wrap measured with the real embedded fonts. A word longer than
/// the column is broken at a character boundary rather than running off the
/// page. Explicit newlines force a break.
fn wrap_chars(chars: &[StyledChar], max_width: f32, size: f32) -> Vec<Vec<StyledChar>> {
    let max_width = max_width.max(4.0);
    let mut lines: Vec<Vec<StyledChar>> = Vec::new();
    let mut line: Vec<StyledChar> = Vec::new();
    let mut width = 0.0f32;
    let mut last_space: Option<usize> = None;

    for &styled in chars {
        if styled.c == '\n' {
            trim_trailing_spaces(&mut line);
            lines.push(std::mem::take(&mut line));
            width = 0.0;
            last_space = None;
            continue;
        }
        let advance = char_width(styled.c, styled.face, size);
        if width + advance > max_width && !line.is_empty() {
            if styled.c == ' ' {
                // A trailing space never pushes the last word onto the next
                // line; it is trimmed and the break happens after that word.
                trim_trailing_spaces(&mut line);
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                width = 0.0;
                last_space = None;
                continue;
            }
            if let Some(split) = last_space {
                let tail: Vec<StyledChar> = line.split_off(split);
                trim_trailing_spaces(&mut line);
                lines.push(std::mem::take(&mut line));
                line = tail;
                while line.first().is_some_and(|s| s.c == ' ') {
                    line.remove(0);
                }
                width = chars_width(&line, size);
                last_space = line.iter().rposition(|s| s.c == ' ');
            } else {
                lines.push(std::mem::take(&mut line));
                width = 0.0;
                last_space = None;
            }
        }
        if styled.c == ' ' {
            last_space = Some(line.len());
        }
        width += advance;
        line.push(styled);
    }
    trim_trailing_spaces(&mut line);
    lines.push(line);
    if lines.len() > 1 && lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}

fn trim_trailing_spaces(line: &mut Vec<StyledChar>) {
    while line.last().is_some_and(|s| s.c == ' ') {
        line.pop();
    }
}

fn segments(chars: &[StyledChar]) -> Vec<PdfSegment> {
    let mut out: Vec<PdfSegment> = Vec::new();
    for styled in chars {
        if styled.c == '\n' || styled.c == '\r' {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.face == styled.face => last.text.push(styled.c),
            _ => out.push(PdfSegment {
                text: styled.c.to_string(),
                face: styled.face,
            }),
        }
    }
    out
}

#[derive(Clone, Copy)]
struct Frame {
    width: f32,
    height: f32,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl Frame {
    fn content_width(&self) -> f32 {
        (self.width - self.left - self.right).max(24.0)
    }

}

struct Flow {
    frame: Frame,
    pages: Vec<PdfPageLayout>,
    current: PdfPageLayout,
    cursor: f32,
}

impl Flow {
    fn with_pages(frame: Frame, pages: Vec<PdfPageLayout>) -> Self {
        Self {
            pages,
            current: PdfPageLayout { items: Vec::new() },
            cursor: frame.top,
            frame,
        }
    }

    /// The page currently being filled counts, so a one-page section reports
    /// one page even before the flow finishes it.
    fn page_index(&self) -> usize {
        self.pages.len() + usize::from(!self.current.items.is_empty())
    }

    fn finish_page(&mut self) {
        if !self.current.items.is_empty() {
            self.pages.push(std::mem::replace(
                &mut self.current,
                PdfPageLayout { items: Vec::new() },
            ));
        }
        self.cursor = self.frame.top;
    }

    fn page_break(&mut self) {
        self.finish_page();
        self.cursor = self.frame.top;
    }

    fn ensure(&mut self, height: f32) {
        if self.cursor + height > self.frame.height - self.frame.bottom + 0.01
            && !self.current.items.is_empty()
        {
            self.finish_page();
        }
    }

    fn space(&mut self, height: f32) {
        if self.current.items.is_empty() || height <= 0.0 {
            return;
        }
        self.cursor += height;
    }

    fn text_line(
        &mut self,
        chars: &[StyledChar],
        x: f32,
        size: f32,
        leading: f32,
        gap_before: f32,
    ) {
        self.place(&[(x, chars)], size, leading, gap_before);
    }

    /// A line stretched to `available` points by widening the gaps between
    /// words. A line with a single word, or one already wider than the measure,
    /// is set flush left instead.
    fn text_line_justified(
        &mut self,
        chars: &[StyledChar],
        x: f32,
        available: f32,
        size: f32,
        leading: f32,
        gap_before: f32,
    ) {
        let words: Vec<&[StyledChar]> = chars
            .split(|styled| styled.c == ' ')
            .filter(|word| !word.is_empty())
            .collect();
        let natural: f32 = words.iter().map(|word| chars_width(word, size)).sum();
        if words.len() < 2 || natural >= available {
            return self.text_line(chars, x, size, leading, gap_before);
        }
        let gap = (available - natural) / (words.len() - 1) as f32;
        let mut cursor = x;
        let mut pieces = Vec::with_capacity(words.len());
        for word in words {
            pieces.push((cursor, word));
            cursor += chars_width(word, size) + gap;
        }
        self.place(&pieces, size, leading, gap_before);
    }

    /// One baseline carrying one or more separately positioned runs.
    fn place(
        &mut self,
        pieces: &[(f32, &[StyledChar])],
        size: f32,
        leading: f32,
        gap_before: f32,
    ) {
        let gap = if self.current.items.is_empty() {
            0.0
        } else {
            gap_before.max(0.0)
        };
        self.ensure(gap + leading);
        self.cursor += gap;
        let baseline = self.cursor + size * 0.80;
        for (x, chars) in pieces {
            let segments = segments(chars);
            if !segments.is_empty() {
                self.current.items.push(PdfText {
                    x: *x,
                    baseline,
                    size,
                    segments,
                });
            }
        }
        self.cursor += leading;
    }

    fn finish(mut self) -> Vec<PdfPageLayout> {
        if !self.current.items.is_empty() {
            self.pages.push(self.current);
        }
        if self.pages.is_empty() {
            self.pages.push(PdfPageLayout { items: Vec::new() });
        }
        self.pages
    }
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Lay out the compiled manuscript. This is the single source of truth for
/// pagination, including the page estimate shown on the Compile screen.
pub fn pdf_layout(project: &Project, settings: &ExportSettings) -> PdfLayout {
    let plan = project.compile_plan(settings);
    if plan.kind.is_script() {
        script_layout(project, settings)
    } else {
        prose_layout(project, settings)
    }
}

/// The number of pages the PDF writer will produce.
pub fn pdf_page_count(project: &Project, settings: &ExportSettings) -> usize {
    pdf_layout(project, settings).page_count()
}

/// Render the compiled manuscript as PDF bytes.
pub fn pdf_manuscript(project: &Project, settings: &ExportSettings) -> Vec<u8> {
    let layout = pdf_layout(project, settings);
    draw(project, &layout)
}

// ---------------------------------------------------------------------------
// Scripts
// ---------------------------------------------------------------------------

fn script_layout(project: &Project, settings: &ExportSettings) -> PdfLayout {
    let plan = project.compile_plan(settings);
    let included: Vec<&Document> = plan
        .included
        .iter()
        .copied()
        .filter(|doc| !matches!(doc.kind, DocKind::FrontMatter | DocKind::BackMatter))
        .collect();
    let sheet = script_sheet(project, settings);
    let (width, height) = paper_pt(settings);
    // A script sheet is designed for US Letter: 12 pt Courier, 1.5" left and
    // 1" elsewhere, 3.5" of dialogue. On a narrower page, scaling only the
    // indents leaves the type and margins huge relative to the page. Scale the
    // whole sheet instead, so a smaller page is the Letter page reduced — the
    // same look as the editor's paper. Letter itself stays exactly 1:1.
    let scale = script_scale(settings);
    let margins = page_margins(project, settings);
    let frame = Frame {
        width,
        height,
        left: margins.left,
        right: margins.right,
        top: margins.top,
        bottom: margins.bottom,
    };

    let fallback_font = sheet
        .style("action")
        .map(|style| style.format.font.clone())
        .filter(|font| !font.trim().is_empty())
        .unwrap_or_else(|| "Courier New".to_string());
    let title_face = Face::from_style(
        &sheet
            .style("action")
            .map(|style| style.format.clone())
            .unwrap_or_default(),
        &fallback_font,
    );

    let mut pages: Vec<PdfPageLayout> = Vec::new();
    let mut sections: Vec<PdfLayoutSection> = Vec::new();

    if settings.include_title_page {
        pages.push(script_title_page(project, settings, &frame, title_face));
        sections.push(PdfLayoutSection {
            label: "Title page".into(),
            start_page: 0,
            page_count: 1,
        });
    }

    let mut flow = Flow::with_pages(frame, pages);
    let mut scenes = SceneNumbers::default();

    for (index, doc) in included.iter().enumerate() {
        if index > 0 && starts_new_page(doc) {
            flow.page_break();
        }
        let start = flow.page_index();
        let doc_sheet = project.style_sheet_for(doc);
        let mut blocks: Vec<Block> = crate::screenplay_blocks(project, doc);
        // Blank blocks are not content; the writer never inserts separators or
        // transitions of its own.
        blocks.retain(|block| !block.text.trim().is_empty());

        for block in &blocks {
            let style_ref = doc_sheet
                .style(style_id_for(block.kind))
                .map(|style| (&style.format, style.text_transform))
                .unwrap_or_else(|| {
                    let style = settings.styles.style_for_kind(block.kind);
                    (style, TextTransform::None)
                });
            let (format, transform) = style_ref;
            let uppercase =
                format.uppercase || matches!(transform, TextTransform::Uppercase);
            let text = if uppercase {
                block.text.to_uppercase()
            } else {
                block.text.clone()
            };
            let text = scenes.apply(block.kind, text, settings.include_scene_numbers);
            let size = format.size.max(7.0) * scale;
            let leading = (size * format.leading.max(0.9)).max(size * 0.9);
            let indent = format.indent.max(0.0) * scale;
            let right_indent = format.right_indent.max(0.0) * scale;
            let width = (frame.content_width() - indent - right_indent).max(18.0);
            let fallback = doc_sheet
                .style("action")
                .map(|style| style.format.font.clone())
                .filter(|font| !font.trim().is_empty())
                .unwrap_or_else(|| fallback_font.clone());
            let face = Face::from_style(format, &fallback);
            let mut chars = Vec::new();
            push_text(&mut chars, &text, face);
            let lines = wrap_chars(&chars, width, size);
            if lines.iter().all(|line| line.is_empty()) {
                continue;
            }

            let before = format.before.max(0.0) * scale;
            let after = format.after.max(0.0) * scale;
            let needed = before + lines.len() as f32 * leading;
            let widowed_cue = block.kind == ElementKind::Character
                && flow.cursor + needed + leading * 2.0
                    > frame.height - frame.bottom
                && !flow.current.items.is_empty();
            if widowed_cue || (flow.cursor + needed > frame.height - frame.bottom
                && !flow.current.items.is_empty())
            {
                flow.page_break();
            }

            for (line_index, line) in lines.iter().enumerate() {
                let line_width = chars_width(line, size);
                let first = line_index == 0;
                let first_indent = if first {
                    format.first_line_indent.max(0.0) * scale
                } else {
                    0.0
                };
                let left = frame.left + indent + first_indent;
                let available = (width - first_indent).max(8.0);
                // `FADE IN:` opens a script flush left; every other transition
                // is a right-hand cue. The text is the writer's own block; only
                // its alignment is chosen here.
                let fade_in = block.kind == ElementKind::Transition
                    && text.trim_start().to_uppercase().starts_with("FADE IN");
                let x = match format.align {
                    Alignment::Right if !fade_in => left + (available - line_width).max(0.0),
                    Alignment::Center if !fade_in => {
                        left + (available - line_width).max(0.0) * 0.5
                    }
                    _ => left,
                };
                flow.text_line(
                    line,
                    x,
                    size,
                    leading,
                    if first { before } else { 0.0 },
                );
            }
            flow.space(after);
        }

        sections.push(PdfLayoutSection {
            label: doc.title.clone(),
            start_page: start,
            page_count: flow.page_index() - start,
        });
    }

    let mut pages = flow.finish();

    let first_number = settings.header_footer.start_number;
    if settings.header_footer.page_numbers {
        let face = title_face;
        // The title page is unnumbered, and the first page of the script is
        // page 1, not page 2.
        let title_pages = usize::from(settings.include_title_page);
        let number_size = 12.0 * scale;
        let baseline = (frame.top - 24.0 * scale).max(10.0);
        for (index, page) in pages.iter_mut().skip(title_pages).enumerate() {
            let number = settings
                .header_footer
                .number_format
                .format(first_number + index as u32);
            let text = format!("{}.", number);
            let width = text_width(&text, face, number_size);
            page.items.push(PdfText {
                x: frame.width - frame.right - width,
                baseline,
                size: number_size,
                segments: vec![PdfSegment { text, face }],
            });
        }
    }

    PdfLayout {
        width,
        height,
        pages,
        sections,
    }
}

fn style_id_for(kind: ElementKind) -> &'static str {
    portmanteau_core::rich_text::script_style_id(kind)
}

fn script_title_page(
    project: &Project,
    settings: &ExportSettings,
    frame: &Frame,
    face: Face,
) -> PdfPageLayout {
    let mut items = Vec::new();
    // The same lines every writer reads (`script_title`); the PDF places them
    // by baseline. Flowing text starts at the top margin; the draft line is
    // anchored to the foot of the page instead of floated by spacing.
    let mut top = frame.top;
    for line in script_cover_lines(project, settings) {
        let size = line.style.size;
        let width = text_width(&line.text, face, size);
        let available = frame.content_width();
        let x = match line.style.align {
            Alignment::Center => frame.left + (available - width).max(0.0) * 0.5,
            Alignment::Right => frame.left + (available - width).max(0.0),
            Alignment::Left | Alignment::Justify => frame.left,
        };
        let baseline = if line.role == TitleRole::Footer {
            frame.height - frame.bottom - 6.0 * (size / 12.0)
        } else {
            top += line.style.before;
            let baseline = top + ascent_pt(size);
            top += size * line.style.leading;
            baseline
        };
        items.push(PdfText {
            x,
            baseline,
            size,
            segments: vec![PdfSegment {
                text: line.text,
                face,
            }],
        });
    }
    PdfPageLayout { items }
}

pub(crate) fn wrap_plain(text: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
        } else if line.chars().count() + 1 + word.chars().count() <= max_chars {
            line.push(' ');
            line.push_str(word);
        } else {
            out.push(std::mem::take(&mut line));
            line.push_str(word);
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

// ---------------------------------------------------------------------------
// Prose
// ---------------------------------------------------------------------------

fn prose_frame(project: &Project, settings: &ExportSettings) -> Frame {
    let (width, height) = paper_pt(settings);
    let margins = page_margins(project, settings);
    Frame {
        width,
        height,
        left: margins.left,
        right: margins.right,
        top: margins.top,
        bottom: margins.bottom,
    }
}

fn prose_layout(project: &Project, settings: &ExportSettings) -> PdfLayout {
    let frame = prose_frame(project, settings);
    let body = settings.styles.body.clone();
    let body_size = body.size.max(8.0);
    let body_face = Face::from_style(&body, &settings.font);
    let mut pages: Vec<PdfPageLayout> = Vec::new();
    let mut sections: Vec<PdfLayoutSection> = Vec::new();

    if settings.include_title_page {
        pages.push(prose_title_page(project, settings, &frame));
        sections.push(PdfLayoutSection {
            label: "Title page".into(),
            start_page: 0,
            page_count: 1,
        });
    }

    let mut flow = Flow::with_pages(frame, pages);
    let mut headings: Vec<(String, usize)> = Vec::new();

    for doc in front_matter(project, settings) {
        flow.page_break();
        let start = flow.page_index();
        flow_chapter_heading(
            &mut flow,
            &doc.title,
            &settings.styles.chapter_heading,
            &body,
            &settings.font,
        );
        for paragraph in prose_paragraphs(project, doc, settings) {
            flow_prose_paragraph(&mut flow, &paragraph, &body, &settings.font);
        }
        sections.push(PdfLayoutSection {
            label: doc.title.clone(),
            start_page: start,
            page_count: flow.page_index() - start,
        });
    }

    let compiled = manuscript_sections(project, settings);
    for (index, section) in compiled.iter().enumerate() {
        flow.page_break();
        let start = flow.page_index();
        let title = section_title(project, section.doc, index, settings);
        headings.push((title.clone(), start));
        flow_chapter_heading(
            &mut flow,
            &title,
            &settings.styles.chapter_heading,
            &body,
            &settings.font,
        );
        if settings.include_synopsis && !section.doc.synopsis.is_empty() {
            let mut format = body.clone();
            format.size = (body_size - 1.0).max(7.0);
            format.first_line_indent = 0.0;
            format.italic = true;
            flow_wrapped_text(
                &mut flow,
                &format!("Synopsis · {}", section.doc.synopsis.plain_text()),
                &format,
                &settings.font,
            );
            flow.space(format.after.max(6.0));
        }
        for paragraph in prose_paragraphs(project, section.doc, settings) {
            flow_prose_paragraph(&mut flow, &paragraph, &body, &settings.font);
        }
        if settings.include_notes && !section.doc.notes.is_empty() {
            let mut format = body.clone();
            format.size = (body_size - 1.0).max(7.0);
            format.first_line_indent = 0.0;
            format.italic = true;
            flow_wrapped_text(
                &mut flow,
                &format!("Author's note · {}", section.doc.notes.plain_text()),
                &format,
                &settings.font,
            );
        }
        sections.push(PdfLayoutSection {
            label: section.heading.clone(),
            start_page: start,
            page_count: flow.page_index() - start,
        });
    }

    for doc in back_matter(project, settings) {
        flow.page_break();
        let start = flow.page_index();
        flow_chapter_heading(
            &mut flow,
            &doc.title,
            &settings.styles.chapter_heading,
            &body,
            &settings.font,
        );
        for paragraph in prose_paragraphs(project, doc, settings) {
            flow_prose_paragraph(&mut flow, &paragraph, &body, &settings.font);
        }
        sections.push(PdfLayoutSection {
            label: doc.title.clone(),
            start_page: start,
            page_count: flow.page_index() - start,
        });
    }

    let mut pages = flow.finish();

    // Contents is assembled last because it needs the real page numbers, then
    // inserted after the title page. Section starts after the insertion move by
    // one page.
    if settings.include_toc && !headings.is_empty() {
        let insert_at = usize::from(settings.include_title_page && !pages.is_empty());
        let toc = prose_contents_page(settings, &frame, &headings);
        if let Some(toc) = toc {
            pages.insert(insert_at.min(pages.len()), toc);
            for section in &mut sections {
                if section.start_page >= insert_at {
                    section.start_page += 1;
                }
            }
            sections.insert(
                insert_at.min(sections.len()),
                PdfLayoutSection {
                    label: "Contents".into(),
                    start_page: insert_at,
                    page_count: 1,
                },
            );
        }
    }

    apply_prose_furniture(project, settings, &frame, &mut pages, body_size, body_face);

    PdfLayout {
        width: frame.width,
        height: frame.height,
        pages,
        sections,
    }
}

fn prose_title_page(
    project: &Project,
    settings: &ExportSettings,
    frame: &Frame,
) -> PdfPageLayout {
    let styles = &settings.styles;
    let mut items = Vec::new();
    let mut y = frame.height * TITLE_PAGE_DROP;

    let title_style = &styles.title_page_title;
    let title_size = title_style.size.max(8.0);
    let title_face = Face::from_style(title_style, &settings.font).with_marks(true, false);
    for line in wrap_measured(project.title.trim(), title_face, title_size, frame.content_width()) {
        push_aligned(&mut items, frame, &line, title_size, y, title_face, title_style.align);
        y += (title_size * title_style.leading.max(1.0)).max(12.0);
    }
    y += title_style.after.max(0.0);

    if !project.subtitle.trim().is_empty() {
        let style = &styles.title_page_subtitle;
        let size = style.size.max(8.0);
        let face = Face::from_style(style, &settings.font);
        for line in wrap_measured(project.subtitle.trim(), face, size, frame.content_width()) {
            push_aligned(&mut items, frame, &line, size, y, face, style.align);
            y += (size * style.leading.max(1.1)).max(10.0);
        }
        y += style.after.max(0.0);
    }

    let style = &styles.title_page_author;
    let size = style.size.max(8.0);
    let face = Face::from_style(style, &settings.font);
    if !project.author.trim().is_empty() {
        let byline = format!("by {}", project.author.trim());
        push_aligned(&mut items, frame, &byline, size, y.max(frame.height * 0.58), face, style.align);
    }

    let footer = if project.genre.trim().is_empty() {
        chrono::Local::now().format("%Y").to_string()
    } else {
        format!("{} · {}", project.genre.trim(), chrono::Local::now().format("%Y"))
    };
    push_aligned(
        &mut items,
        frame,
        &footer,
        9.0,
        frame.height - 42.0,
        Face::new(Family::Serif, false, false),
        Alignment::Center,
    );

    PdfPageLayout { items }
}

/// Greedy line breaking by measured width, so a larger title size wraps
/// earlier instead of running off the page.
fn wrap_measured(text: &str, face: Face, size: f32, width: f32) -> Vec<String> {
    let mut chars = Vec::new();
    push_text(&mut chars, text, face);
    wrap_chars(&chars, width, size)
        .into_iter()
        .map(|line| line.iter().map(|styled| styled.c).collect::<String>())
        .collect()
}

fn push_aligned(
    items: &mut Vec<PdfText>,
    frame: &Frame,
    text: &str,
    size: f32,
    baseline: f32,
    face: Face,
    align: Alignment,
) {
    if text.trim().is_empty() {
        return;
    }
    let spare = (frame.content_width() - text_width(text, face, size)).max(0.0);
    let offset = match align {
        Alignment::Center => spare * 0.5,
        Alignment::Right => spare,
        Alignment::Left | Alignment::Justify => 0.0,
    };
    items.push(PdfText {
        x: frame.left + offset,
        baseline,
        size,
        segments: vec![PdfSegment {
            text: text.to_string(),
            face,
        }],
    });
}

fn flow_chapter_heading(
    flow: &mut Flow,
    title: &str,
    style: &ElementStyle,
    body: &ElementStyle,
    fallback_font: &str,
) {
    let size = style.size.max(body.size).max(8.0);
    let face = Face::from_style(style, fallback_font).with_marks(true, style.italic);
    let leading = (size * style.leading.max(1.0)).max(size + 2.0);
    let indent = style.indent.max(0.0);
    let right_indent = style.right_indent.max(0.0);
    let width = (flow.frame.content_width() - indent - right_indent).max(24.0);
    let mut chars = Vec::new();
    push_text(&mut chars, title, face);
    let lines = wrap_chars(&chars, width, size);
    let before = style.before.max(0.0);
    let after = style.after.max(0.0);
    let needed = before + lines.len() as f32 * leading;
    flow.ensure(needed);
    for (index, line) in lines.iter().enumerate() {
        let line_width = chars_width(line, size);
        let left = flow.frame.left + indent;
        let x = match style.align {
            Alignment::Right => left + (width - line_width).max(0.0),
            Alignment::Center => left + (width - line_width).max(0.0) * 0.5,
            Alignment::Left | Alignment::Justify => left,
        };
        flow.text_line(
            line,
            x,
            size,
            leading,
            if index == 0 { before } else { 0.0 },
        );
    }
    flow.space(after);
}

fn flow_prose_paragraph(
    flow: &mut Flow,
    paragraph: &ProseParagraph,
    body: &ElementStyle,
    fallback_font: &str,
) {
    let format = &paragraph.format;
    let size = format.size.max(7.0);
    let fallback = if format.font.trim().is_empty() {
        if body.font.trim().is_empty() {
            fallback_font
        } else {
            body.font.as_str()
        }
    } else {
        format.font.as_str()
    };
    let base = Face::from_style(format, fallback);
    let mut chars = Vec::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                let bold = paragraph.heading || run.marks.contains(&InlineMark::Bold);
                let italic = format.italic || run.marks.contains(&InlineMark::Italic);
                let face = base.with_marks(bold, italic);
                push_text(&mut chars, &paragraph.transform(&run.text), face);
            }
            InlineContent::HardBreak => push_text(&mut chars, " ", base),
            InlineContent::Image(image) if image.alt_text.is_empty() => {
                push_text(&mut chars, "[Image]", base)
            }
            InlineContent::Image(image) => {
                push_text(
                    &mut chars,
                    &format!("[Image: {}]", paragraph.transform(&image.alt_text)),
                    base,
                );
            }
        }
    }
    if chars.iter().all(|styled| styled.c.is_whitespace()) {
        return;
    }

    let leading = (size * format.leading.max(0.9)).max(size + 2.0);
    let indent = format.indent.max(0.0);
    let right_indent = format.right_indent.max(0.0);
    let width = (flow.frame.content_width() - indent - right_indent).max(24.0);
    let first_indent = format.first_line_indent.max(0.0);
    let lines = wrap_chars(&chars, width - first_indent.min(width * 0.5), size);
    let before = format.before.max(0.0);
    let after = format.after.max(0.0);

    for (index, line) in lines.iter().enumerate() {
        let first = index == 0;
        let line_width = chars_width(line, size);
        let extra = if first { first_indent } else { 0.0 };
        let left = flow.frame.left + indent + extra;
        let available = (width - extra).max(8.0);
        let x = match format.align {
            Alignment::Right => left + (available - line_width).max(0.0),
            Alignment::Center => left + (available - line_width).max(0.0) * 0.5,
            Alignment::Left | Alignment::Justify => left,
        };
        let gap_before = if first { before } else { 0.0 };
        // The last line of a justified paragraph stays ragged, as in print.
        if format.align == Alignment::Justify && index + 1 < lines.len() {
            flow.text_line_justified(line, left, available, size, leading, gap_before);
        } else {
            flow.text_line(line, x, size, leading, gap_before);
        }
    }
    flow.space(after);
}

fn flow_wrapped_text(
    flow: &mut Flow,
    text: &str,
    format: &ElementStyle,
    fallback_font: &str,
) {
    let paragraph = ProseParagraph {
        format: format.clone(),
        content: vec![InlineContent::Text(portmanteau_core::TextRun::plain(text))],
        heading: false,
        uppercase: false,
    };
    flow_prose_paragraph(flow, &paragraph, format, fallback_font);
}

fn prose_contents_page(
    settings: &ExportSettings,
    frame: &Frame,
    headings: &[(String, usize)],
) -> Option<PdfPageLayout> {
    let body = &settings.styles.body;
    let size = body.size.max(8.0);
    let face = Face::from_style(body, &settings.font);
    let heading_face = Face::new(face.family, true, face.italic);
    let mut items = Vec::new();
    let heading_size = size + 6.0;
    items.push(PdfText {
        x: frame.left,
        baseline: frame.top + 20.0,
        size: heading_size,
        segments: vec![PdfSegment {
            text: "Contents".into(),
            face: heading_face,
        }],
    });

    let step = (size * 1.8).max(14.0);
    let mut y = frame.top + 56.0;
    let limit = frame.height - frame.bottom;
    for (title, start) in headings {
        if y > limit {
            break;
        }
        // The contents page is inserted before the first heading, so every
        // heading moves one page later. Physical pages count from the start
        // number, exactly as the footer does.
        let printed = *start as u32 + settings.header_footer.start_number + 1;
        let number = settings.header_footer.number_format.format(printed);
        let number_width = text_width(&number, face, size);
        let max_title = (frame.content_width() - number_width - 18.0).max(40.0);
        let mut chars = Vec::new();
        push_text(&mut chars, title, face);
        let mut lines = wrap_chars(&chars, max_title, size);
        lines.truncate(1);
        if let Some(line) = lines.first() {
            items.push(PdfText {
                x: frame.left,
                baseline: y,
                size,
                segments: segments(line),
            });
        }
        items.push(PdfText {
            x: frame.width - frame.right - number_width,
            baseline: y,
            size,
            segments: vec![PdfSegment {
                text: number,
                face,
            }],
        });
        y += step;
    }
    Some(PdfPageLayout { items })
}

fn apply_prose_furniture(
    project: &Project,
    settings: &ExportSettings,
    frame: &Frame,
    pages: &mut [PdfPageLayout],
    size: f32,
    face: Face,
) {
    let hf = &settings.header_footer;
    let running = if hf.header_text.trim().is_empty() {
        project.title.clone()
    } else {
        hf.header_text.clone()
    };
    for (index, page) in pages.iter_mut().enumerate() {
        if hf.running_header && index > 0 {
            let text = &running;
            let width = text_width(text, face, 9.0);
            let x = aligned_x(hf.header_align, width, frame.left, frame.width - frame.right);
            page.items.push(PdfText {
                x,
                baseline: hf.header_spacing.max(12.0),
                size: 9.0,
                segments: vec![PdfSegment {
                    text: text.clone(),
                    face,
                }],
            });
        }
        if hf.page_numbers && (index > 0 || hf.number_on_first_page) {
            let number = hf.number_format.format(hf.start_number + index as u32);
            let width = text_width(&number, face, 9.0);
            let x = aligned_x(hf.number_align, width, frame.left, frame.width - frame.right);
            let baseline = match hf.number_location {
                portmanteau_core::HfLocation::Header => hf.number_spacing.max(12.0),
                portmanteau_core::HfLocation::Footer => {
                    frame.height - hf.number_spacing.max(12.0)
                }
            };
            page.items.push(PdfText {
                x,
                baseline,
                size: 9.0,
                segments: vec![PdfSegment {
                    text: number,
                    face,
                }],
            });
        }
    }
    let _ = size;
}

fn aligned_x(align: Alignment, width: f32, left: f32, right: f32) -> f32 {
    let available = (right - left).max(1.0);
    let width = width.min(available);
    match align {
        Alignment::Center => left + (available - width) * 0.5,
        Alignment::Right => right - width,
        Alignment::Left | Alignment::Justify => left,
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn krilla_font(face: Face) -> krilla::text::Font {
    use std::sync::OnceLock;
    static FONTS: OnceLock<[krilla::text::Font; 10]> = OnceLock::new();
    let fonts = FONTS.get_or_init(|| {
        std::array::from_fn(|index| {
            let data = match index {
                0 => SERIF_REGULAR,
                1 => SERIF_BOLD,
                2 => SERIF_ITALIC,
                3 => SANS_REGULAR,
                4 => SANS_BOLD,
                5 => SANS_ITALIC,
                6 => MONO_REGULAR,
                7 => MONO_BOLD,
                8 => MONO_ITALIC,
                _ => FALLBACK,
            };
            krilla::text::Font::new(data.into(), 0)
                .unwrap_or_else(|| panic!("bundled font {index} could not be loaded"))
        })
    });
    fonts[face.index()].clone()
}

fn draw(project: &Project, layout: &PdfLayout) -> Vec<u8> {
    use krilla::geom::Point;
    use krilla::page::PageSettings;
    use krilla::paint::Fill;
    use krilla::text::TextDirection;
    use krilla::Document;

    let mut document = Document::new();
    let mut metadata = krilla::metadata::Metadata::new()
        .title(project.title.clone())
        .creator("Portmanteau".into())
        .producer("Portmanteau".into());
    if !project.author.trim().is_empty() {
        metadata = metadata.authors(vec![project.author.clone()]);
    }
    document.set_metadata(metadata);

    for page_layout in &layout.pages {
        let Some(settings) = PageSettings::from_wh(layout.width, layout.height) else {
            continue;
        };
        let mut page = document.start_page_with(settings);
        let mut surface = page.surface();
        surface.set_fill(Some(Fill::default()));
        for item in &page_layout.items {
            let mut x = item.x;
            for segment in &item.segments {
                if segment.text.is_empty() {
                    continue;
                }
                surface.draw_text(
                    Point::from_xy(x, item.baseline),
                    krilla_font(segment.face),
                    item.size,
                    &segment.text,
                    false,
                    TextDirection::LeftToRight,
                );
                x += text_width(&segment.text, segment.face, item.size);
            }
        }
        surface.finish();
        page.finish();
    }

    document
        .finish()
        .unwrap_or_else(|error| panic!("the PDF could not be written: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm_to_pt;
    use portmanteau_core::{ExportKind, Project};

    fn demo() -> Project {
        portmanteau_core::sample::demo()
    }

    fn plain_prose_settings(project: &Project) -> ExportSettings {
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.paper = portmanteau_core::PaperSize::Letter;
        settings.include_title_page = false;
        settings.include_toc = false;
        settings.header_footer.page_numbers = false;
        settings.header_footer.running_header = false;
        settings
    }

    fn right_edge(item: &PdfText) -> f32 {
        let size = item.size;
        item.x
            + item
                .segments
                .iter()
                .map(|segment| text_width(&segment.text, segment.face, size))
                .sum::<f32>()
    }

    #[test]
    fn custom_margins_move_the_prose_text_block() {
        let project = demo();
        let mut settings = plain_prose_settings(&project);
        settings.margins = Some(portmanteau_core::PageMargins {
            left: 90.0,
            right: 36.0,
            top: 54.0,
            bottom: 108.0,
        });
        let layout = pdf_layout(&project, &settings);
        let items: Vec<&PdfText> = layout.pages.iter().flat_map(|page| &page.items).collect();
        let left = items.iter().map(|item| item.x).fold(f32::MAX, f32::min);
        let right = items.iter().map(|item| right_edge(item)).fold(0.0, f32::max);
        assert!((left - 90.0).abs() < 0.5, "left edge {left}");
        assert!(right <= 612.0 - 36.0 + 0.5, "right edge {right}");
        let bottom = items.iter().map(|item| item.baseline).fold(0.0, f32::max);
        assert!(bottom <= 792.0 - 108.0 + 0.5, "lowest baseline {bottom}");
    }

    #[test]
    fn justified_lines_fill_the_measure_and_the_last_line_stays_ragged() {
        let project = demo();
        let mut settings = plain_prose_settings(&project);
        settings.styles.body.align = Alignment::Justify;
        settings.styles.body.indent = 0.0;
        settings.styles.body.right_indent = 0.0;
        let layout = pdf_layout(&project, &settings);
        let edge = 612.0 - mm_to_pt(20.0);
        // Group words back into lines by baseline, then look at each line's end.
        let mut ends: Vec<(f32, f32)> = Vec::new();
        for page in &layout.pages {
            for item in &page.items {
                match ends.iter_mut().find(|(baseline, _)| (*baseline - item.baseline).abs() < 0.01) {
                    Some((_, end)) => *end = end.max(right_edge(item)),
                    None => ends.push((item.baseline, right_edge(item))),
                }
            }
        }
        let full = ends.iter().filter(|(_, end)| (end - edge).abs() < 0.5).count();
        let beyond = ends.iter().filter(|(_, end)| *end > edge + 0.5).count();
        assert!(full >= 5, "expected justified lines to reach the right edge, got {full}");
        assert_eq!(beyond, 0, "no line may pass the right margin");
        assert!(full < ends.len(), "last lines of paragraphs must stay ragged");
    }

    #[test]
    fn title_page_honours_each_title_alignment() {
        let project = demo();
        let mut settings = plain_prose_settings(&project);
        settings.include_title_page = true;
        let find = |settings: &ExportSettings| {
            let layout = pdf_layout(&project, settings);
            let title = layout.pages[0]
                .items
                .iter()
                .find(|item| item.segments.iter().any(|s| s.text.contains("The Salt Road")))
                .expect("title on the first page")
                .clone();
            (title.x, right_edge(&title))
        };
        settings.styles.title_page_title.align = Alignment::Left;
        let (x_left, _) = find(&settings);
        settings.styles.title_page_title.align = Alignment::Right;
        let (_, end_right) = find(&settings);
        settings.styles.title_page_title.align = Alignment::Center;
        let (x_mid, end_mid) = find(&settings);
        let margin = mm_to_pt(20.0);
        assert!((x_left - margin).abs() < 0.5);
        assert!((end_right - (612.0 - margin)).abs() < 0.5);
        assert!(((x_mid + end_mid) / 2.0 - 306.0).abs() < 0.5);
    }

    fn script_settings(project: &Project, kind: &str) -> ExportSettings {
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.paper = portmanteau_core::PaperSize::Letter;
        settings.compile_type = match kind {
            "tv" => Some(portmanteau_core::CompileType::TvScript),
            "stage" => Some(portmanteau_core::CompileType::StagePlay),
            _ => Some(portmanteau_core::CompileType::Screenplay),
        };
        settings
    }

    #[test]
    fn script_layout_uses_letter_and_the_sheet_margins() {
        let project = demo();
        let mut settings = script_settings(&project, "tv");
        settings.include_title_page = false;
        let layout = pdf_layout(&project, &settings);
        assert!((layout.width - 612.0).abs() < 0.5, "width {}", layout.width);
        assert!((layout.height - 792.0).abs() < 0.5, "height {}", layout.height);
        let first_text = layout
            .pages
            .iter()
            .find_map(|page| page.items.first())
            .expect("a script line");
        let left = first_text.x;
        assert!(
            (left - 108.0).abs() < 1.0,
            "scene action should start at the 1.5in margin, got {left}"
        );
    }

    fn item_text(item: &PdfText) -> String {
        item.segments.iter().map(|segment| segment.text.as_str()).collect()
    }

    /// Find one laid-out line by the text it starts with.
    fn line_starting_with<'a>(layout: &'a PdfLayout, prefix: &str) -> &'a PdfText {
        layout
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item_text(item).starts_with(prefix))
            .unwrap_or_else(|| panic!("no laid-out line starts with {prefix:?}"))
    }

    #[test]
    fn letter_script_columns_use_the_standard_screenplay_geometry() {
        let project = demo();
        let mut settings = script_settings(&project, "tv");
        settings.include_title_page = false;
        let layout = pdf_layout(&project, &settings);

        // 1.5" margin, character cue 2.2" further in, dialogue 1" in with a
        // 1.5" right inset. Dialogue therefore spans 2.5"–6".
        let cue = layout
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item_text(item) == "MARA")
            .expect("a character cue");
        assert!((cue.x - 266.4).abs() < 1.0, "cue at {}", cue.x);
        let dialogue = line_starting_with(&layout, "The chart");
        assert!(
            (dialogue.x - 180.0).abs() < 1.0,
            "dialogue at {}",
            dialogue.x
        );
        // The 3.5" dialogue column fits all but the last word on one line.
        let full_line = line_starting_with(&layout, "The chart says there's a channel");
        assert!(item_text(full_line).starts_with("The chart says there's a channel"));
        let last_line = line_starting_with(&layout, "here.");
        assert!((last_line.x - 180.0).abs() < 1.0, "here. at {}", last_line.x);
    }

    #[test]
    fn narrow_paper_scales_the_whole_script_sheet() {
        let project = demo();
        let mut settings = script_settings(&project, "tv");
        settings.paper = portmanteau_core::PaperSize::A5;
        settings.include_title_page = false;
        let layout = pdf_layout(&project, &settings);
        let scale = layout.width / 612.0;

        // Margins, type and indents reduce together: the A5 page is the Letter
        // sheet at 68.5%, not 12 pt type squeezed inside 1.5" margins. Dialogue
        // therefore starts at 1.71" and holds the same words per line.
        let dialogue = line_starting_with(&layout, "The chart");
        assert!(
            (dialogue.x - (108.0 + 72.0) * scale).abs() < 1.0,
            "dialogue at {}, expected {}",
            dialogue.x,
            (108.0 + 72.0) * scale
        );
        assert!(
            (dialogue.size - 12.0 * scale).abs() < 0.05,
            "dialogue type is {} pt, expected {}",
            dialogue.size,
            12.0 * scale
        );
        assert!(
            item_text(line_starting_with(&layout, "(to herself)")).contains("(to herself)"),
            "the parenthetical should fit on one line"
        );
        assert!(
            item_text(dialogue).chars().count() > 10,
            "the first dialogue line should hold several words, got {:?}",
            item_text(dialogue)
        );
        // The editor fits "…The harbour" on one line. An overflowing trailing
        // space used to move the last word down a line, so the line breaks no
        // longer matched the page on screen.
        assert!(
            item_text(line_starting_with(&layout, "The sea has gone out"))
                .ends_with("harbour"),
            "the action line should break exactly as it does in the editor"
        );
        // Same characters per line as the editor, so the scene heading still
        // fits on one line.
        assert!(
            layout
                .pages
                .iter()
                .flat_map(|page| &page.items)
                .any(|item| item_text(item).contains("BREAKWATER — DAWN")),
            "the scene heading should survive on one line"
        );
    }



    #[test]
    fn script_sections_account_for_every_page() {
        let project = demo();
        for kind in ["tv", "stage"] {
            let settings = script_settings(&project, kind);
            let layout = pdf_layout(&project, &settings);
            let titled = usize::from(settings.include_title_page);
            let body: usize = layout
                .sections
                .iter()
                .filter(|section| section.label != "Title page")
                .map(|section| section.page_count)
                .sum();
            assert_eq!(
                body + titled,
                layout.page_count(),
                "{kind}: {:?}",
                layout.sections
            );
            for section in &layout.sections {
                assert!(section.page_count > 0, "{kind}: {:?}", section);
            }
        }
    }

    #[test]
    fn prose_sections_account_for_every_page() {
        let project = demo();
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.compile_type = Some(portmanteau_core::CompileType::Prose);
        let layout = pdf_layout(&project, &settings);
        let sum: usize = layout.sections.iter().map(|section| section.page_count).sum();
        assert_eq!(sum, layout.page_count(), "{:?}", layout.sections);
    }

    #[test]
    fn paper_choice_moves_script_pages_to_a4() {
        let project = demo();
        let mut settings = script_settings(&project, "tv");
        settings.paper = portmanteau_core::PaperSize::A4;
        let layout = pdf_layout(&project, &settings);
        assert!((layout.width - 595.28).abs() < 0.5, "A4 width {}", layout.width);
        assert!((layout.height - 841.89).abs() < 0.5, "A4 height {}", layout.height);
    }

    /// One scene with a single action line and no transitions of any kind.
    fn silent_script() -> Project {
        let mut project = Project::new("Silent", portmanteau_core::ProjectKind::Film);
        let draft = project.folder_id("Draft").unwrap();
        let id = project.add_document(
            Some(draft),
            "Scene",
            DocKind::Scene,
            portmanteau_core::Medium::Screenplay,
        );
        let doc = project.document_mut(&id).unwrap();
        doc.content = portmanteau_core::RichDocument {
            paragraphs: vec![portmanteau_core::RichParagraph {
                style_id: "action".into(),
                content: vec![InlineContent::Text(portmanteau_core::TextRun::plain(
                    "A room with no exits.",
                ))],
            }],
        };
        project
    }

    #[test]
    fn compiler_does_not_add_transitions() {
        let project = silent_script();
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.include_scene_numbers = false;
        settings.include_title_page = false;
        let layout = pdf_layout(&project, &settings);
        let text: String = layout
            .pages
            .iter()
            .flat_map(|page| page.items.iter())
            .flat_map(|item| item.segments.iter())
            .map(|segment| segment.text.as_str())
            .collect();
        assert!(text.contains("A room with no exits."), "{text}");
        assert!(!text.contains("FADE IN"), "{text}");
    }

    // ---- Real PDFs, checked with the poppler tools when present -------------

    fn tool_available(name: &str) -> bool {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {name}"))
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    fn temp_pdf(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::path::Path::new("/tmp/opencode/pdf-tests");
        std::fs::create_dir_all(dir).expect("temp directory");
        let path = dir.join(format!("{}-{name}.pdf", std::process::id()));
        std::fs::write(&path, bytes).expect("write pdf");
        path
    }

    fn run_tool(tool: &str, args: &[&str], path: &std::path::Path) -> String {
        let out = std::process::Command::new(tool)
            .args(args)
            .arg(path)
            .output()
            .unwrap_or_else(|_| panic!("{tool} should run"));
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// pdftotext writes next to the PDF unless told to use standard output.
    fn text_from_pdf(args: &[&str], path: &std::path::Path) -> String {
        let out = std::process::Command::new("pdftotext")
            .args(args)
            .arg(path)
            .arg("-")
            .output()
            .expect("pdftotext should run");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn page_size_pts(report: &str) -> (f32, f32) {
        let line = report
            .lines()
            .find(|line| line.starts_with("Page size:"))
            .unwrap_or_else(|| panic!("no page size in\n{report}"));
        let numbers: Vec<f32> = line
            .split_whitespace()
            .filter_map(|word| word.parse::<f32>().ok())
            .collect();
        (numbers[0], numbers[1])
    }

    #[test]
    fn pdfinfo_confirms_the_paper_size_and_page_count() {
        if !tool_available("pdfinfo") {
            return;
        }
        let project = demo();
        let mut settings = script_settings(&project, "tv");
        for (paper, expected) in [
            (portmanteau_core::PaperSize::Letter, (612.0, 792.0)),
            (portmanteau_core::PaperSize::A4, (595.28, 841.89)),
        ] {
            settings.paper = paper;
            let layout = pdf_layout(&project, &settings);
            let path = temp_pdf(paper.label(), &draw(&project, &layout));
            let info = run_tool("pdfinfo", &[], &path);
            let (width, height) = page_size_pts(&info);
            assert!(
                (width - expected.0).abs() < 0.5 && (height - expected.1).abs() < 0.5,
                "{} should be {:?} pts, got {width} x {height}",
                paper.label(),
                expected
            );
            let pages = info
                .lines()
                .find(|line| line.starts_with("Pages:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|count| count.parse::<usize>().ok())
                .expect("page count");
            assert_eq!(pages, layout.page_count(), "{}", paper.label());
        }
    }

    #[test]
    fn pdffonts_reports_subsetted_embedded_fonts() {
        if !tool_available("pdffonts") {
            return;
        }
        let project = demo();
        let settings = script_settings(&project, "tv");
        let path = temp_pdf("fonts", &pdf_manuscript(&project, &settings));
        let report = run_tool("pdffonts", &[], &path);
        let mut fonts = 0;
        for line in report.lines().skip(2).filter(|line| !line.trim().is_empty()) {
            fonts += 1;
            assert!(line.contains('+'), "font should be subsetted: {line}");
            assert!(
                line.contains(" yes yes yes "),
                "font should be embedded with a Unicode map: {line}"
            );
        }
        assert!(fonts > 0, "no fonts reported:\n{report}");
    }

    #[test]
    fn pdftotext_round_trips_the_writers_own_text() {
        if !tool_available("pdftotext") {
            return;
        }
        let project = silent_script();
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.include_title_page = false;
        settings.header_footer.page_numbers = false;
        let path = temp_pdf("script-text", &pdf_manuscript(&project, &settings));
        let text = text_from_pdf(&[], &path);
        assert!(text.contains("A room with no exits."), "{text}");
        assert!(
            !text.contains("FADE IN"),
            "the compiler must not invent a transition:\n{text}"
        );

        let mut project = Project::new("Dashes", portmanteau_core::ProjectKind::Novel);
        let draft = project.folder_id("Draft").unwrap();
        let id = project.add_document(
            Some(draft),
            "Chapter",
            DocKind::Chapter,
            portmanteau_core::Medium::Prose,
        );
        project
            .document_mut(&id)
            .unwrap()
            .set_plain_text("She waited — then left.");
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.include_title_page = false;
        settings.include_toc = false;
        let path = temp_pdf("prose-text", &pdf_manuscript(&project, &settings));
        let text = text_from_pdf(&[], &path);
        assert!(text.contains('—'), "the en dash should survive:\n{text}");
    }

    #[test]
    fn script_page_numbers_start_at_one_at_the_top_right() {
        if !tool_available("pdftotext") {
            return;
        }
        let project = demo();
        let mut settings = script_settings(&project, "tv");
        settings.include_title_page = true;
        settings.header_footer.page_numbers = true;
        let layout = pdf_layout(&project, &settings);
        assert!(layout.page_count() >= 2, "the demo needs a script page");
        let path = temp_pdf("numbering", &draw(&project, &layout));
        let xml = text_from_pdf(&["-bbox"], &path);
        let pages: Vec<&str> = xml.split("<page ").skip(1).collect();
        assert!(pages.len() >= 2, "expected a title page and a script page");
        let word = pages[1]
            .split("<word ")
            .skip(1)
            .find(|word| word.contains(">1.</word>"))
            .unwrap_or_else(|| panic!("first script page should be numbered 1:\n{}", pages[1]));
        let attr = |name: &str| -> f32 {
            let rest = word
                .split_once(&format!("{name}=\""))
                .unwrap_or_else(|| panic!("no {name} in {word}"))
                .1;
            rest.split('"').next().unwrap().parse().unwrap()
        };
        let x_max = attr("xMax");
        let y_min = attr("yMin");
        assert!(
            (x_max - 540.0).abs() < 2.0,
            "page number should hug the right margin, xMax {x_max}"
        );
        assert!(
            y_min < 60.0,
            "page number should sit above the text block, yMin {y_min}"
        );

        // The drawing pass must place text where the layout put it: the
        // leftmost word on the script page starts at the stylesheet margin.
        let min_x = pages[1]
            .split("<word ")
            .skip(1)
            .filter_map(|word| {
                let (_, rest) = word.split_once("xMin=\"")?;
                rest.split('"').next()?.parse::<f32>().ok()
            })
            .fold(f32::INFINITY, f32::min);
        let plan = project.compile_plan(&settings);
        let left = project
            .style_sheet_for(plan.included.first().copied().expect("included script"))
            .margins
            .left;
        assert!(
            (min_x - left).abs() < 0.6,
            "script text should start at {left} pt, got {min_x}"
        );
    }

    #[test]
    fn prose_title_page_follows_the_title_style_size() {
        let mut project = Project::new("Needle", portmanteau_core::ProjectKind::Novel);
        let draft = project.folder_id("Draft").unwrap();
        project.add_document(Some(draft), "One", DocKind::Chapter, portmanteau_core::Medium::Prose);
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.include_title_page = true;
        settings.include_toc = false;
        settings.styles.title_page_title.size = 30.0;
        let layout = pdf_layout(&project, &settings);
        let title = layout.pages[0]
            .items
            .iter()
            .find(|item| item.segments.iter().any(|segment| segment.text.contains("Needle")))
            .expect("the title page carries the project title");
        assert_eq!(title.size, 30.0);
    }

    #[test]
    fn prose_centered_chapter_heading_shifts_toward_the_middle() {
        let mut project = Project::new("Heading", portmanteau_core::ProjectKind::Novel);
        let draft = project.folder_id("Draft").unwrap();
        project.add_document(
            Some(draft),
            "Needle Chapter",
            DocKind::Chapter,
            portmanteau_core::Medium::Prose,
        );
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.include_title_page = false;
        settings.include_toc = false;
        settings.header_footer.running_header = false;
        settings.header_footer.page_numbers = false;
        settings.include_chapter_numbers = false;

        let heading_x = |settings: &ExportSettings| {
            let layout = pdf_layout(&project, settings);
            layout
                .pages
                .iter()
                .flat_map(|page| page.items.iter())
                .find(|item| {
                    item.segments
                        .iter()
                        .any(|segment| segment.text.to_lowercase().contains("needle"))
                })
                .map(|item| item.x)
                .expect("the chapter heading")
        };
        settings.styles.chapter_heading.align = Alignment::Left;
        let left = heading_x(&settings);
        settings.styles.chapter_heading.align = Alignment::Center;
        let centered = heading_x(&settings);
        assert!(
            centered > left + 40.0,
            "centred heading should move right: left {left}, centred {centered}"
        );
    }

    #[test]
    fn prose_running_headers_are_optional() {
        let project = demo();
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.include_toc = false;
        settings.header_footer.page_numbers = false;
        settings.header_footer.header_text = "Running head".into();

        let has_header = |settings: &ExportSettings| {
            let layout = pdf_layout(&project, settings);
            layout
                .pages
                .get(1)
                .map(|page| {
                    page.items.iter().any(|item| {
                        item.segments
                            .iter()
                            .any(|segment| segment.text == "Running head")
                    })
                })
                .unwrap_or(false)
        };
        settings.header_footer.running_header = true;
        assert!(has_header(&settings));
        settings.header_footer.running_header = false;
        assert!(!has_header(&settings));
    }

    #[test]
    fn en_dash_is_not_lost() {
        let mut project = Project::new("Dashes", portmanteau_core::ProjectKind::Novel);
        let draft = project.folder_id("Draft").unwrap();
        let id = project.add_document(
            Some(draft),
            "Chapter",
            DocKind::Chapter,
            portmanteau_core::Medium::Prose,
        );
        project
            .document_mut(&id)
            .unwrap()
            .set_plain_text("She waited — then left.");
        let mut settings = project.settings_for(ExportKind::Pdf);
        settings.include_title_page = false;
        settings.include_toc = false;
        let layout = pdf_layout(&project, &settings);
        let text: String = layout
            .pages
            .iter()
            .flat_map(|page| page.items.iter())
            .flat_map(|item| item.segments.iter())
            .map(|segment| segment.text.as_str())
            .collect();
        assert!(text.contains('—'), "{text}");
    }
}
