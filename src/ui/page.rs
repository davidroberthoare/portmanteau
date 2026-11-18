//! Page paging and print preview shared by the editor's read-only Paged view
//! and the Compile screen's live manuscript preview.
//!
//! Everything here is a best-effort estimate of the export engine's layout:
//! paragraphs are wrapped with a character-width approximation and lines are
//! packed against the page's text height. It is deliberately driven by the real
//! `ExportSettings`, so changing paper, element styles or margins updates the
//! preview.

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontFamily, FontId, Frame, Layout, Margin, RichText,
    ScrollArea, Stroke, Vec2,
};
use portmanteau_core::{
    Alignment, DocKind, Document, ElementKind, ElementStyle, ElementStyles, ExportSettings,
    InlineContent, Project, RichParagraph, StyleSheet, TextTransform,
};

use crate::app::PortmanteauApp;
use crate::theme::*;

/// One rendered line inside a page. The paging estimator fills pages with these
/// so a page break can fall between any two lines.
pub struct PageLine {
    pub text: String,
    pub size: f32,
    pub lead: f32,
    pub gap_before: f32,
    pub mono: bool,
    /// Font-family name from the element style; empty means inherit.
    pub font: String,
    /// Paragraph alignment. `center`/`right` are kept as overrides for the
    /// title page and screenplay transitions.
    pub align: Alignment,
    pub indent: f32,
    /// Extra indent applied to the first wrapped line of a paragraph only.
    pub first_line_indent: f32,
    /// Width removed from the right edge of the text column.
    pub right_indent: f32,
    /// Space after this line when it ends a paragraph or script block.
    pub after: f32,
    pub right: bool,
    /// Center the line horizontally (used by the title page).
    pub center: bool,
}

/// An estimated pagination of one document. Geometry and the pages themselves
/// are owned so the view can draw without borrowing the project.
pub struct PagePlan {
    pub page_w: f32,
    pub content_w: f32,
    pub content_h: f32,
    pub pages: Vec<Vec<PageLine>>,
}

/// One entry in the compiled manuscript's running order: the generated title
/// page, a front-matter document, a draft section or a back-matter document.
/// `page_count` is how many pages that section contributed, so the section
/// start pages are the cumulative counts.
pub struct PreviewSection {
    pub label: String,
    pub page_count: usize,
}

/// The whole compiled manuscript, paged, plus the section boundaries the
/// Compile screen uses for its section picker.
pub struct ManuscriptPreview {
    pub plans: Vec<PagePlan>,
    pub sections: Vec<PreviewSection>,
}

impl ManuscriptPreview {
    /// Total pages across every section.
    pub fn page_count(&self) -> usize {
        self.plans.iter().map(|plan| plan.pages.len()).sum()
    }

    /// The 0-based page index where `section` starts, from the cumulative page
    /// counts of the sections before it.
    pub fn section_start(&self, section: usize) -> usize {
        self.sections
            .iter()
            .take(section)
            .map(|section| section.page_count)
            .sum()
    }

    /// Every section's start page in order, parallel to `sections`.
    pub fn section_starts(&self) -> Vec<usize> {
        (0..self.sections.len())
            .map(|section| self.section_start(section))
            .collect()
    }
}

pub fn empty_page_plan() -> PagePlan {
    PagePlan {
        page_w: 444.0,
        content_w: 336.0,
        content_h: 552.0,
        pages: vec![Vec::new()],
    }
}

/// Page size from the active export target's paper, mapped into egui points at
/// a readable on-screen scale (1 mm ~ 3 points). Margins match the continuous
/// paper: 54 pt left/right, 44 top, 34 bottom.
pub fn page_geometry(settings: &ExportSettings) -> (f32, f32) {
    let (mm_w, mm_h) = settings.paper.mm();
    ((mm_w * 3.0).clamp(420.0, 760.0), (mm_h * 3.0).max(520.0))
}

/// Best-effort prose paging. Paragraphs are wrapped to the text width with a
/// character-width estimate (`size * 0.55`), then lines are packed until the
/// page's text height is used up.
pub fn prose_plan(project: &Project, doc: &Document, settings: &ExportSettings) -> PagePlan {
    let (page_w, page_h) = page_geometry(settings);
    let content_w = page_w - 108.0;
    let content_h = page_h - 78.0;
    let sheet = project.style_sheet_for(doc);
    let body = sheet
        .style("body")
        .map(|style| project.paragraph_format_for(style, &settings.styles))
        .unwrap_or_else(|| settings.styles.body.clone());
    let heading = sheet
        .style("chapter-heading")
        .map(|style| project.paragraph_format_for(style, &settings.styles))
        .unwrap_or_else(|| settings.styles.chapter_heading.clone());

    let mut pages: Vec<Vec<PageLine>> = vec![Vec::new()];
    let mut used = 0.0f32;

    // The chapter title sits at the top of the first page, set with the
    // chapter-heading style: its font, alignment, indents and spacing.
    let title = doc.title.trim();
    if !title.is_empty() {
        let title_size = heading.size.max(body.size.max(6.0));
        let title_lead = (title_size * heading.leading.max(0.9)).max(10.0);
        let title_before = heading.before.max(0.0);
        let title_after = heading.after.max(0.0);
        pages[0].push(PageLine {
            text: title.to_string(),
            size: title_size,
            lead: title_lead,
            gap_before: title_before,
            mono: false,
            font: heading.font.clone(),
            align: heading.align,
            indent: heading.indent.max(0.0),
            first_line_indent: heading.first_line_indent.max(0.0),
            right_indent: heading.right_indent.max(0.0),
            after: title_after,
            right: false,
            center: false,
        });
        used += title_before + title_lead + title_after;
    }

    // Paragraphs keep their own stylesheet appearance. The old version
    // flattened the rich document into one string, which collapsed typed
    // paragraphs and ignored every style but Body.
    for paragraph in &doc.content.paragraphs {
        let style = sheet.style(&paragraph.style_id);
        let format = style
            .map(|style| project.paragraph_format_for(style, &settings.styles))
            .unwrap_or_else(|| body.clone());
        let uppercase = style
            .map(|style| style.text_transform == TextTransform::Uppercase)
            .unwrap_or(false)
            || format.uppercase;
        let mut text = prose_paragraph_text(paragraph);
        if uppercase {
            text = text.to_uppercase();
        }
        if text.trim().is_empty() {
            continue;
        }
        let size = format.size.max(6.0);
        let lead = (size * format.leading.max(0.8)).max(8.0);
        let indent = format.indent.max(0.0);
        let right_indent = format.right_indent.max(0.0);
        let text_w = (content_w - indent - right_indent).max(24.0);
        let chars_per_line = (text_w / (size * 0.5)).floor().max(10.0) as usize;
        let wrapped = wrap_words(&text, chars_per_line);
        let count = wrapped.len();
        for (i, line) in wrapped.into_iter().enumerate() {
            let first = i == 0;
            let last = i + 1 == count;
            let mut gap = if first { format.before.max(0.0) } else { 0.0 };
            if used + gap + lead > content_h && !pages.last().unwrap().is_empty() {
                pages.push(Vec::new());
                used = 0.0;
                gap = 0.0;
            }
            if pages.last().unwrap().is_empty() {
                gap = 0.0;
            }
            let after = if last { format.after.max(0.0) } else { 0.0 };
            pages.last_mut().unwrap().push(PageLine {
                text: line,
                size,
                lead,
                gap_before: gap,
                mono: false,
                font: format.font.clone(),
                align: format.align,
                indent,
                first_line_indent: if first {
                    format.first_line_indent.max(0.0)
                } else {
                    0.0
                },
                right_indent,
                after,
                right: false,
                center: false,
            });
            used += gap + lead + after;
        }
    }

    PagePlan {
        page_w,
        content_w,
        content_h,
        pages,
    }
}

fn prose_paragraph_text(paragraph: &RichParagraph) -> String {
    paragraph
        .content
        .iter()
        .map(|item| match item {
            InlineContent::Text(run) => run.text.clone(),
            InlineContent::HardBreak => " ".to_owned(),
            InlineContent::Image(image) if image.alt_text.is_empty() => "[Image]".to_owned(),
            InlineContent::Image(image) => format!("[Image: {}]", image.alt_text),
        })
        .collect()
}

/// Script paging: roughly 55 typed lines per page, with each block's wrapped
/// height counted in those lines. The page keeps the paper width and frame but
/// grows tall enough for all 55 lines.
pub fn script_plan(doc: &Document, settings: &ExportSettings) -> PagePlan {
    // Screenplays use US Letter regardless of the prose book's trim size.
    // `page_geometry` maps millimetres to three screen points.
    let page_w: f32 = 8.5 * 25.4 * 3.0;
    let page_h: f32 = 11.0 * 25.4 * 3.0;
    let content_w = page_w - 2.5 * 25.4 * 3.0;
    let base = &settings.styles.action;
    let base_size = base.size.max(7.0);
    let base_line = (base_size * base.leading.max(0.85)).max(9.0);
    let lines_per_page = 55.0f32;
    let content_h = (page_h - 2.0 * 25.4 * 3.0).max(base_line * lines_per_page);

    let mut pages: Vec<Vec<PageLine>> = vec![Vec::new()];
    let mut used = 0.0f32;

    let default_sheet = StyleSheet::film_screenplay(&settings.styles);
    let blocks = doc.content.to_script_blocks(&default_sheet);
    for (index, block) in blocks.iter().enumerate() {
        let style = script_element_style(&settings.styles, block.kind);
        let size = style.size.max(6.0);
        let lead = (size * style.leading.max(0.85)).max(8.0);
        let indent = style.indent.max(0.0).min(content_w * 0.42);
        let right_indent = style.right_indent.max(0.0);
        let text_w = (content_w - indent - right_indent).max(12.0);
        let chars_per_line = (text_w / (size * 0.6)).floor().max(8.0) as usize;
        let text = if style.uppercase {
            block.text.to_uppercase()
        } else {
            block.text.clone()
        };
        let wrapped = wrap_words(&text, chars_per_line);
        let count = wrapped.len();
        let block_lines = (count as f32 * lead / base_line).max(1.0);
        let after = style.after.max(0.0);

        // Blocks are separated by their style's before/after spacing, with the
        // old one-fifth-line gap as the fallback when before is unset.
        let mut gap_points = if index == 0 || block.kind.is_dialogue() {
            0.0
        } else {
            0.4 * base_line
        };
        if index > 0 && style.before > 0.0 {
            gap_points = style.before;
        }
        let mut gap_lines = gap_points / base_line;
        let after_lines = after / base_line;

        if used + gap_lines + block_lines > lines_per_page
            && !pages.last().unwrap().is_empty()
        {
            pages.push(Vec::new());
            used = 0.0;
            gap_lines = 0.0;
        }
        if pages.last().unwrap().is_empty() {
            gap_lines = 0.0;
        }
        for (i, text) in wrapped.into_iter().enumerate() {
            let first = i == 0;
            let last = i + 1 == count;
            let transition = block.kind == ElementKind::Transition;
            pages.last_mut().unwrap().push(PageLine {
                text,
                size,
                lead,
                gap_before: if first { gap_lines * base_line } else { 0.0 },
                mono: true,
                font: style.font.clone(),
                align: if transition {
                    Alignment::Right
                } else {
                    style.align
                },
                indent,
                first_line_indent: if first {
                    style.first_line_indent.max(0.0)
                } else {
                    0.0
                },
                right_indent,
                after: if last { after } else { 0.0 },
                right: transition,
                center: false,
            });
        }
        used += gap_lines + block_lines + after_lines;
    }

    PagePlan {
        page_w,
        content_w,
        content_h,
        pages,
    }
}

pub fn script_element_style<'a>(
    styles: &'a ElementStyles,
    kind: ElementKind,
) -> &'a ElementStyle {
    styles.style_for_kind(kind)
}

/// Break a paragraph into display lines using an estimated character width.
/// Wrapping happens on spaces; a single word longer than the line stays whole.
pub fn wrap_words(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(4);
    let mut out = Vec::new();
    for raw in text.split('\n') {
        let mut line = String::new();
        for word in raw.split_whitespace() {
            let word_len = word.chars().count();
            if line.is_empty() {
                line = word.to_string();
            } else if line.chars().count() + 1 + word_len <= max_chars {
                line.push(' ');
                line.push_str(word);
            } else {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            }
        }
        if !line.is_empty() {
            out.push(line);
        } else if raw.trim().is_empty() && !out.is_empty() {
            out.push(" ".into());
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Page the whole compiled manuscript: optional title page, front matter,
/// body sections (prose + script), back matter — with section boundaries for
/// the Compile preview's page and section navigation.
pub fn manuscript_preview(project: &Project, settings: &ExportSettings) -> ManuscriptPreview {
    let mut plans: Vec<PagePlan> = Vec::new();
    let mut sections: Vec<PreviewSection> = Vec::new();

    if settings.include_title_page {
        let plan = title_page_plan(project, settings);
        sections.push(PreviewSection {
            label: "Title page".to_string(),
            page_count: plan.pages.len(),
        });
        plans.push(plan);
    }

    // Front matter first, in binder order, matching the export assembly.
    for doc in project
        .ordered_documents()
        .into_iter()
        .filter(|d| d.kind == DocKind::FrontMatter && d.include_in_export)
    {
        let plan = prose_plan(project, doc, settings);
        sections.push(section_for(doc, &plan));
        plans.push(plan);
    }

    // The draft: prose and script sections share the binder order.
    for doc in project.manuscript_documents() {
        if matches!(doc.kind, DocKind::FrontMatter | DocKind::BackMatter) {
            continue;
        }
        let plan = if doc.medium.is_script() {
            script_plan(doc, settings)
        } else {
            prose_plan(project, doc, settings)
        };
        sections.push(section_for(doc, &plan));
        plans.push(plan);
    }

    // Back matter last.
    for doc in project
        .ordered_documents()
        .into_iter()
        .filter(|d| d.kind == DocKind::BackMatter && d.include_in_export)
    {
        let plan = prose_plan(project, doc, settings);
        sections.push(section_for(doc, &plan));
        plans.push(plan);
    }

    // Always show at least one page so an empty manuscript still reads as a
    // page of paper rather than a blank panel.
    if plans.is_empty() {
        let (page_w, page_h) = page_geometry(settings);
        plans.push(PagePlan {
            page_w,
            content_w: page_w - 108.0,
            content_h: page_h - 78.0,
            pages: vec![Vec::new()],
        });
        sections.push(PreviewSection {
            label: "Manuscript".to_string(),
            page_count: 1,
        });
    }

    ManuscriptPreview { plans, sections }
}

/// Section label and page count for one paged document. The label is the
/// document title, matching the binder.
fn section_for(doc: &Document, plan: &PagePlan) -> PreviewSection {
    let title = doc.title.trim();
    PreviewSection {
        label: if title.is_empty() {
            "Untitled".to_string()
        } else {
            title.to_string()
        },
        page_count: plan.pages.len(),
    }
}

/// A single centered title page: title, subtitle and byline, set with the
/// title-page styles from the active format.
fn title_page_plan(project: &Project, settings: &ExportSettings) -> PagePlan {
    let (page_w, page_h) = page_geometry(settings);
    let content_w = page_w - 108.0;
    let content_h = page_h - 78.0;
    let styles = &settings.styles;
    let mut lines: Vec<PageLine> = Vec::new();

    let title = project.title.trim();
    let subtitle = project.subtitle.trim();
    let author = project.author.trim();

    let title_style = &styles.title_page_title;
    let title_size = title_style.size.max(8.0);
    let title_lead = (title_size * title_style.leading.max(1.0)).max(12.0);
    let subtitle_style = &styles.title_page_subtitle;
    let subtitle_size = subtitle_style.size.max(8.0);
    let subtitle_lead = (subtitle_size * subtitle_style.leading.max(1.0)).max(10.0);
    let author_style = &styles.title_page_author;
    let author_size = author_style.size.max(8.0);
    let author_lead = (author_size * author_style.leading.max(1.0)).max(10.0);

    // Start well down the page, like a printed half-title.
    let mut gap = (content_h * 0.30).max(24.0);

    if !title.is_empty() {
        for (i, line) in wrap_words(title, 34).into_iter().enumerate() {
            lines.push(PageLine {
                text: line,
                size: title_size,
                lead: title_lead,
                gap_before: if i == 0 { gap } else { 0.0 },
                mono: false,
                font: title_style.font.clone(),
                align: title_style.align,
                indent: 0.0,
                first_line_indent: 0.0,
                right_indent: 0.0,
                after: 0.0,
                right: false,
                center: true,
            });
        }
        gap = 28.0;
    }

    if !subtitle.is_empty() {
        for (i, line) in wrap_words(subtitle, 50).into_iter().enumerate() {
            lines.push(PageLine {
                text: line,
                size: subtitle_size,
                lead: subtitle_lead,
                gap_before: if i == 0 { gap } else { 0.0 },
                mono: false,
                font: subtitle_style.font.clone(),
                align: subtitle_style.align,
                indent: 0.0,
                first_line_indent: 0.0,
                right_indent: 0.0,
                after: 0.0,
                right: false,
                center: true,
            });
        }
        gap = 44.0;
    }

    if !author.is_empty() {
        lines.push(PageLine {
            text: format!("by {author}"),
            size: author_size,
            lead: author_lead,
            gap_before: gap,
            mono: false,
            font: author_style.font.clone(),
            align: author_style.align,
            indent: 0.0,
            first_line_indent: 0.0,
            right_indent: 0.0,
            after: 0.0,
            right: false,
            center: true,
        });
    }

    PagePlan {
        page_w,
        content_w,
        content_h,
        pages: vec![lines],
    }
}

pub fn paged_view(app: &mut PortmanteauApp, ui: &mut egui::Ui, plan: &PagePlan) {
    let mut page_rects = Vec::with_capacity(plan.pages.len());
    let mut viewport_top = f32::NEG_INFINITY;
    ScrollArea::vertical()
        .id_salt("editor_paged_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            viewport_top = ui.clip_rect().top();
            ui.add_space(22.0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(plan.page_w);
                ui.label(
                    RichText::new("Read-only page preview — switch to Continuous to edit")
                        .font(ui_small())
                        .color(FAINT),
                );
                ui.add_space(10.0);
                for (index, page) in plan.pages.iter().enumerate() {
                    page_rects.push(page_frame(ui, plan, page, index + 1, 1.0));
                    if index + 1 < plan.pages.len() {
                        ui.add_space(26.0);
                    }
                }
            });
            ui.add_space(36.0);
        });

    // Which page has the top of the viewport scrolled into it? The toolbar
    // reads this on the next frame.
    app.paged_page = page_rects
        .iter()
        .enumerate()
        .rev()
        .find(|(_, rect)| rect.top() <= viewport_top + 1.0)
        .map(|(index, _)| index + 1)
        .unwrap_or(1)
        .min(plan.pages.len().max(1));
}

pub fn page_frame(
    ui: &mut egui::Ui,
    plan: &PagePlan,
    lines: &[PageLine],
    number: usize,
    scale: f32,
) -> egui::Rect {
    Frame::new()
        .fill(PAPER)
        .stroke(Stroke::new(1.0, PAPER_EDGE))
        .corner_radius(CornerRadius::same(3))
        .shadow(egui::epaint::Shadow {
            offset: [0, 10],
            blur: 34,
            spread: 0,
            color: Color32::from_black_alpha(24),
        })
        .inner_margin(Margin {
            left: (54.0 * scale) as i8,
            right: (54.0 * scale) as i8,
            top: (44.0 * scale) as i8,
            bottom: (34.0 * scale) as i8,
        })
        .show(ui, |ui| {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.set_width(plan.content_w * scale);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let mut used = 0.0f32;
                for line in lines {
                    if line.gap_before > 0.0 {
                        ui.add_space(line.gap_before * scale);
                        used += line.gap_before;
                    }
                    let family = if line.font.trim().is_empty() {
                        if line.mono {
                            FontFamily::Monospace
                        } else {
                            FontFamily::Name("portmanteau-serif".into())
                        }
                    } else {
                        family_for_font(&line.font)
                    };
                    let rich = RichText::new(if line.text.is_empty() {
                        " "
                    } else {
                        line.text.as_str()
                    })
                    .font(FontId::new(line.size * scale, family))
                    .color(PAPER_TEXT);
                    let left = (line.indent.max(0.0) + line.first_line_indent.max(0.0))
                        .min((plan.content_w - 24.0).max(0.0));
                    let right = line.right_indent.max(0.0);
                    let text_w = (plan.content_w - left - right).max(24.0);
                    let row_h = line.lead.max(8.0);
                    let align = if line.center {
                        Align::Center
                    } else if line.right {
                        Align::Max
                    } else {
                        match line.align {
                            Alignment::Center => Align::Center,
                            Alignment::Right => Align::Max,
                            // Justify is approximated as left alignment.
                            _ => Align::Min,
                        }
                    };
                    ui.horizontal(|ui| {
                        if left > 0.0 {
                            ui.add_space(left * scale);
                        }
                        ui.allocate_ui_with_layout(
                            Vec2::new(text_w * scale, row_h * scale),
                            Layout::top_down(align),
                            |ui| {
                                ui.add(egui::Label::new(rich).wrap());
                            },
                        );
                    });
                    used += line.lead;
                    if line.after > 0.0 {
                        ui.add_space(line.after * scale);
                        used += line.after;
                    }
                }
                let remaining = (plan.content_h - used).max(6.0);
                ui.add_space(remaining * scale);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("{number}"))
                            .font(serif(9.0 * scale))
                            .color(PAPER_MUTED),
                    );
                });
            });
        })
        .response
        .rect
}
