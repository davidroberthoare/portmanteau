//! Page paging and print preview shared by the editor's read-only Paged view
//! and the Compile screen's live manuscript preview.
//!
//! Everything here is a best-effort estimate of the export engine's layout:
//! paragraphs are wrapped with a character-width approximation and lines are
//! packed against the page's text height. It is deliberately driven by the real
//! `ExportSettings`, so changing paper, element styles or margins updates the
//! preview.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Vec2,
};
use portmanteau_core::{
    DocKind, Document, ElementKind, ElementStyle, ElementStyles, ExportSettings, Project,
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
    pub indent: f32,
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
pub fn prose_plan(doc: &Document, settings: &ExportSettings) -> PagePlan {
    let (page_w, page_h) = page_geometry(settings);
    let content_w = page_w - 108.0;
    let content_h = page_h - 78.0;
    let body = &settings.styles.body;
    let heading = &settings.styles.chapter_heading;
    let body_size = body.size.max(6.0);
    let line_h = (body_size * body.leading.max(0.8)).max(8.0);
    let chars_per_line = (content_w / (body_size * 0.5)).floor().max(10.0) as usize;

    let mut pages: Vec<Vec<PageLine>> = vec![Vec::new()];
    let mut used = 0.0f32;

    // The chapter title sits at the top of the first page, set with the
    // chapter-heading style.
    let title = doc.title.trim();
    if !title.is_empty() {
        let title_size = heading.size.max(body_size);
        let title_lead = (title_size * heading.leading.max(0.9)).max(10.0);
        pages[0].push(PageLine {
            text: title.to_string(),
            size: title_size,
            lead: title_lead,
            gap_before: 0.0,
            mono: false,
            indent: 0.0,
            right: false,
            center: false,
        });
        used += title_lead;
        let gap = line_h * 0.9;
        pages[0].push(PageLine {
            text: " ".into(),
            size: body_size,
            lead: line_h,
            gap_before: gap,
            mono: false,
            indent: 0.0,
            right: false,
            center: false,
        });
        used += gap + line_h;
    }

    for paragraph in doc.body_text().split("\n\n") {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        for (i, text) in wrap_words(paragraph, chars_per_line).into_iter().enumerate() {
            let mut gap = if i == 0 && !pages.last().unwrap().is_empty() {
                line_h * 0.45
            } else {
                0.0
            };
            if used + gap + line_h > content_h && !pages.last().unwrap().is_empty() {
                pages.push(Vec::new());
                used = 0.0;
                gap = 0.0;
            }
            if pages.last().unwrap().is_empty() {
                gap = 0.0;
            }
            pages.last_mut().unwrap().push(PageLine {
                text,
                size: body_size,
                lead: line_h,
                gap_before: gap,
                mono: false,
                indent: 0.0,
                right: false,
                center: false,
            });
            used += gap + line_h;
        }
    }

    PagePlan {
        page_w,
        content_w,
        content_h,
        pages,
    }
}

/// Script paging: roughly 55 typed lines per page, with each block's wrapped
/// height counted in those lines. The page keeps the paper width and frame but
/// grows tall enough for all 55 lines.
pub fn script_plan(doc: &Document, settings: &ExportSettings) -> PagePlan {
    let (page_w, _) = page_geometry(settings);
    let content_w = page_w - 108.0;
    let base = &settings.styles.action;
    let base_size = base.size.max(7.0);
    let base_line = (base_size * base.leading.max(0.85)).max(9.0);
    let lines_per_page = 55.0f32;
    let content_h = base_line * lines_per_page;

    let mut pages: Vec<Vec<PageLine>> = vec![Vec::new()];
    let mut used = 0.0f32;

    for (index, block) in doc.blocks.iter().enumerate() {
        let style = script_element_style(&settings.styles, block.kind);
        let size = style.size.max(6.0);
        let lead = (size * style.leading.max(0.85)).max(8.0);
        let indent = style.indent.max(0.0).min(content_w * 0.42);
        let chars_per_line = ((content_w - indent) / (size * 0.6))
            .floor()
            .max(8.0) as usize;
        let wrapped = wrap_words(&block.text, chars_per_line);
        let block_lines = (wrapped.len() as f32 * lead / base_line).max(1.0);
        let mut gap_lines = if index == 0 || block.kind.is_dialogue() {
            0.0
        } else {
            0.4
        };
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
            pages.last_mut().unwrap().push(PageLine {
                text,
                size,
                lead,
                gap_before: if i == 0 { gap_lines * base_line } else { 0.0 },
                mono: true,
                indent,
                right: block.kind == ElementKind::Transition,
                center: false,
            });
        }
        used += gap_lines + block_lines;
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
/// body sections (prose + script), back matter.
pub fn manuscript_page_plan(project: &Project, settings: &ExportSettings) -> Vec<PagePlan> {
    let mut plans: Vec<PagePlan> = Vec::new();

    if settings.include_title_page {
        plans.push(title_page_plan(project, settings));
    }

    // Front matter first, in binder order, matching the export assembly.
    for doc in project
        .ordered_documents()
        .into_iter()
        .filter(|d| d.kind == DocKind::FrontMatter && d.include_in_export)
    {
        plans.push(prose_plan(doc, settings));
    }

    // The draft: prose and script sections share the binder order.
    for doc in project.manuscript_documents() {
        if matches!(doc.kind, DocKind::FrontMatter | DocKind::BackMatter) {
            continue;
        }
        plans.push(if doc.medium.is_script() {
            script_plan(doc, settings)
        } else {
            prose_plan(doc, settings)
        });
    }

    // Back matter last.
    for doc in project
        .ordered_documents()
        .into_iter()
        .filter(|d| d.kind == DocKind::BackMatter && d.include_in_export)
    {
        plans.push(prose_plan(doc, settings));
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
    }

    plans
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
                indent: 0.0,
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
                indent: 0.0,
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
            indent: 0.0,
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
                        .font(sans(9.5))
                        .color(FAINT),
                );
                ui.add_space(10.0);
                for (index, page) in plan.pages.iter().enumerate() {
                    page_rects.push(page_frame(ui, plan, page, index + 1));
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

pub fn page_frame(ui: &mut egui::Ui, plan: &PagePlan, lines: &[PageLine], number: usize) -> egui::Rect {
    Frame::new()
        .fill(PAPER)
        .stroke(Stroke::new(1.0, PAPER_EDGE))
        .corner_radius(CornerRadius::same(3))
        .shadow(egui::epaint::Shadow {
            offset: [0, 10],
            blur: 34,
            spread: 0,
            color: Color32::from_black_alpha(90),
        })
        .inner_margin(Margin {
            left: 54,
            right: 54,
            top: 44,
            bottom: 34,
        })
        .show(ui, |ui| {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.set_width(plan.content_w);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let mut used = 0.0f32;
                for line in lines {
                    if line.gap_before > 0.0 {
                        ui.add_space(line.gap_before);
                        used += line.gap_before;
                    }
                    let rich = RichText::new(if line.text.is_empty() {
                        " "
                    } else {
                        line.text.as_str()
                    })
                    .font(if line.mono {
                        mono(line.size)
                    } else {
                        serif(line.size)
                    })
                    .color(PAPER_TEXT);
                    if line.center {
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), line.lead.max(8.0)),
                            Layout::top_down(Align::Center),
                            |ui| {
                                ui.label(rich);
                            },
                        );
                    } else if line.right {
                        // Constrain the height: an unconstrained right-to-left
                        // layout would swallow the rest of the page.
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), line.lead.max(8.0)),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                ui.add(egui::Label::new(rich).wrap());
                            },
                        );
                    } else if line.indent > 0.0 {
                        ui.horizontal(|ui| {
                            ui.add_space(line.indent);
                            ui.add(egui::Label::new(rich).wrap());
                        });
                    } else {
                        ui.add(egui::Label::new(rich).wrap());
                    }
                    used += line.lead;
                }
                let remaining = (plan.content_h - used).max(6.0);
                ui.add_space(remaining);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("{number}"))
                            .font(serif(9.0))
                            .color(PAPER_MUTED),
                    );
                });
            });
        })
        .response
        .rect
}
