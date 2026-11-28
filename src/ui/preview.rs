//! The manuscript preview: the real pages of the export.
//!
//! The PDF writer lays the manuscript out (`portmanteau_export::pdf_layout`)
//! before it draws anything. This module draws that same layout on screen, so
//! the Compile and Settings previews show the same line breaks, page breaks,
//! page count and margins as the exported PDF. There is no second pagination
//! engine to drift out of step: change a rule in the exporter and the preview
//! follows.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Pos2, RichText, Sense, Stroke, Vec2};
use portmanteau_core::{CompileScope, CompileType, ExportKind, ExportSettings, Project};
use portmanteau_export::{pdf_layout, text_width, Face, Family, PageCounts, PdfLayout};

use crate::theme::{serif, ui_body, ui_small, BORDER, MUTED};
use crate::i18n::tr;

/// Manuscript pages here show the printed result, so they keep paper white and
/// black ink whatever the interface theme (the writing page in the editor is
/// the themed one).
const PRINT_PAPER: egui::Color32 = egui::Color32::from_rgb(251, 250, 247);
const PRINT_INK: egui::Color32 = egui::Color32::from_rgb(44, 42, 38);

/// The egui family registered by `theme::install` for one exporter face.
pub fn family_for_face(face: Face) -> FontFamily {
    let name = if face.fallback {
        "page-fallback".to_string()
    } else {
        let family = match face.family {
            Family::Serif => "serif",
            Family::Sans => "sans",
            Family::Mono => "mono",
        };
        let style = match (face.bold, face.italic) {
            (true, _) => "-bold",
            (false, true) => "-italic",
            (false, false) => "",
        };
        format!("page-{family}{style}")
    };
    FontFamily::Name(name.into())
}

/// A fingerprint of everything the layout depends on. Laying out a book is
/// far too slow to repeat every frame; the preview recomputes only when this
/// changes (a setting, or any document edited or added).
fn fingerprint(project: &Project, settings: &ExportSettings) -> u64 {
    let mut hasher = DefaultHasher::new();
    serde_json::to_string(settings).unwrap_or_default().hash(&mut hasher);
    project.updated.hash(&mut hasher);
    project.title.hash(&mut hasher);
    project.author.hash(&mut hasher);
    project.documents.len().hash(&mut hasher);
    for doc in &project.documents {
        doc.id.hash(&mut hasher);
        doc.updated.hash(&mut hasher);
        doc.order.hash(&mut hasher);
        doc.parent.hash(&mut hasher);
        doc.content.paragraphs.len().hash(&mut hasher);
    }
    hasher.finish()
}

/// The export's page layout for these settings, computed once per change.
pub fn manuscript_layout(
    ui: &egui::Ui,
    project: &Project,
    settings: &ExportSettings,
    slot: &'static str,
) -> Arc<PdfLayout> {
    let key = fingerprint(project, settings);
    let id = ui.make_persistent_id(("manuscript-layout", slot));
    if let Some((cached, layout)) = ui.data(|d| d.get_temp::<(u64, Arc<PdfLayout>)>(id)) {
        if cached == key {
            return layout;
        }
    }
    let layout = Arc::new(pdf_layout(project, settings));
    ui.data_mut(|d| d.insert_temp(id, (key, layout.clone())));
    layout
}

/// How long a stale page count is kept while the writer is typing. Laying out
/// a whole book on every keystroke would stutter the editor; the count catches
/// up a moment after typing pauses.
const COUNT_REFRESH: std::time::Duration = std::time::Duration::from_millis(900);

/// Real page counts for every document, recomputed when the project changes
/// but no more often than `COUNT_REFRESH`.
pub fn page_counts(ui: &egui::Ui, project: &Project) -> Arc<PageCounts> {
    let key = fingerprint(project, &project.settings_for(ExportKind::Pdf));
    let id = ui.make_persistent_id("page-counts");
    let cached = ui.data(|d| d.get_temp::<(u64, web_time::Instant, Arc<PageCounts>)>(id));
    if let Some((cached_key, at, counts)) = cached {
        if cached_key == key {
            return counts;
        }
        if at.elapsed() < COUNT_REFRESH {
            ui.ctx().request_repaint_after(COUNT_REFRESH);
            return counts;
        }
    }
    let counts = Arc::new(portmanteau_export::page_counts(project));
    ui.data_mut(|d| d.insert_temp(id, (key, web_time::Instant::now(), counts.clone())));
    counts
}

/// The texture for a picture on a previewed page, decoded once and kept.
fn picture_texture(ctx: &egui::Context, image: &portmanteau_export::PdfImage) -> Option<egui::TextureHandle> {
    let id = egui::Id::NULL.with(("preview-picture", &image.asset_id, image.picture.bytes.len()));
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(texture);
    }
    // The preview never needs more than a screenful of pixels: shrink big
    // pictures before they become a texture.
    let decoded = image::load_from_memory(&image.picture.bytes).ok()?.thumbnail(2048, 2048).to_rgba8();
    let size = [decoded.width() as usize, decoded.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, decoded.as_raw());
    let texture = ctx.load_texture(format!("preview-{}", image.asset_id), color, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    Some(texture)
}

/// Draw one page of `layout` at `scale` screen points per page point.
/// A page past the end (an empty manuscript) draws as blank paper.
pub fn paint_page(ui: &mut egui::Ui, layout: &PdfLayout, index: usize, scale: f32) -> egui::Response {
    let size = Vec2::new(layout.width, layout.height) * scale;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter_at(rect.expand(40.0));
    painter.add(
        egui::epaint::Shadow {
            offset: [0, 8],
            blur: 28,
            spread: 0,
            color: Color32::from_black_alpha(24),
        }
        .as_shape(rect, CornerRadius::same(2)),
    );
    painter.rect(
        rect,
        CornerRadius::same(2),
        PRINT_PAPER,
        Stroke::new(1.0, BORDER()),
        egui::StrokeKind::Inside,
    );
    let Some(page) = layout.pages.get(index) else {
        return response;
    };
    for image in &page.images {
        let image_rect = egui::Rect::from_min_size(
            rect.min + Vec2::new(image.x, image.y) * scale,
            Vec2::new(image.width, image.height) * scale,
        );
        match picture_texture(ui.ctx(), image) {
            Some(texture) => {
                painter.image(
                    texture.id(),
                    image_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            None => {
                painter.rect_filled(image_rect, 0.0, BORDER());
            }
        }
    }
    for item in &page.items {
        let mut x = rect.left() + item.x * scale;
        let baseline = rect.top() + item.baseline * scale;
        for segment in &item.segments {
            if !segment.text.trim().is_empty() {
                let galley = painter.layout_no_wrap(
                    segment.text.clone(),
                    FontId::new(item.size * scale, family_for_face(segment.face)),
                    PRINT_INK,
                );
                // Glyph positions are relative to the row, with y on the
                // baseline; shift the galley so that baseline lands where the
                // exporter put it.
                let ascent = galley
                    .rows
                    .first()
                    .and_then(|row| row.glyphs.first())
                    .map(|glyph| glyph.pos.y)
                    .unwrap_or(item.size * scale * 0.8);
                painter.galley(Pos2::new(x, baseline - ascent), galley, PRINT_INK);
            }
            x += text_width(&segment.text, segment.face, item.size) * scale;
        }
    }
    response
}

/// The pages of one document in the compiled manuscript, as the editor's
/// Paged view shows them.
pub struct PagedDocument {
    pub layout: Arc<PdfLayout>,
    /// 0-based index of the document's first page in the manuscript.
    pub first: usize,
    /// How many pages the document touches.
    pub count: usize,
}

/// Lay the manuscript out and find `doc_id`'s pages in it. The document is
/// shown under its own manuscript type with the project's PDF settings, so
/// its pages (and their numbers) are the ones the exported PDF will have.
/// `Err` says why the document has no pages in a compile.
pub fn paged_document(
    ui: &egui::Ui,
    project: &Project,
    doc_id: &str,
) -> Result<PagedDocument, String> {
    let doc = project.document(doc_id).ok_or("No document is open")?;
    let mut settings = project.settings_for(ExportKind::Pdf);
    settings.compile_type = Some(CompileType::from_medium(doc.medium));
    settings.scope = CompileScope::Draft;
    let layout = manuscript_layout(ui, project, &settings, "editor");
    if let Some(section) = layout
        .sections
        .iter()
        .find(|section| section.doc_id.as_deref() == Some(doc_id))
    {
        let (first, count) = (section.start_page, section.page_count.max(1));
        return Ok(PagedDocument { layout, first, count });
    }
    let reason = project
        .compile_plan(&settings)
        .skipped
        .iter()
        .find(|skipped| skipped.doc.id == doc_id)
        .map(|skipped| skipped.reason.clone());
    Err(match reason {
        Some(reason) => format!("{reason}, so it has no pages in a compile."),
        None => "This document has no text to print yet.".to_string(),
    })
}

/// The paragraph and character of `doc_id`'s text nearest a point on a page
/// (page points from its top-left). The nearest line wins, so a click in a
/// margin or between lines still lands somewhere sensible; within a line the
/// character is found by walking the real glyph advances.
pub fn hit_source(layout: &PdfLayout, page: usize, x: f32, y: f32, doc_id: &str) -> Option<(usize, usize)> {
    let page = layout.pages.get(page)?;
    let mut best: Option<(f32, &portmanteau_export::PdfText, &portmanteau_export::PdfSource)> = None;
    for item in &page.items {
        let Some(source) = item.source.as_ref().filter(|s| &*s.doc_id == doc_id) else { continue };
        // The middle of the line's body, between baseline and cap height.
        let centre = item.baseline - item.size * 0.3;
        let width: f32 = item.segments.iter().map(|s| text_width(&s.text, s.face, item.size)).sum();
        let dx = if x < item.x { item.x - x } else if x > item.x + width { x - item.x - width } else { 0.0 };
        // Vertical distance dominates; horizontal only separates the pieces
        // of one justified line.
        let distance = (y - centre).abs() * 4.0 + dx;
        if best.is_none_or(|(d, _, _)| distance < d) {
            best = Some((distance, item, source));
        }
    }
    let (_, item, source) = best?;
    let mut cursor = item.x;
    let mut chars = 0usize;
    'walk: for segment in &item.segments {
        for c in segment.text.chars() {
            let advance = text_width(&c.to_string(), segment.face, item.size);
            if x < cursor + advance * 0.5 {
                break 'walk;
            }
            cursor += advance;
            chars += 1;
        }
    }
    Some((source.paragraph, source.offset + chars))
}

/// The Paged view: the document's pages stacked on a desk, read-only. The
/// page under the top of the viewport is written to `app.paged_page` (1-based
/// within the document) for the toolbar.
pub fn paged_view(
    app: &mut crate::app::PortmanteauApp,
    ui: &mut egui::Ui,
    paged: &Result<PagedDocument, String>,
    doc_id: &str,
) {
    let paged = match paged {
        Ok(paged) => paged,
        Err(reason) => {
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                ui.label(RichText::new(tr("Nothing to page")).font(serif(26.0)).color(MUTED()));
                ui.add_space(8.0);
                ui.label(RichText::new(reason).font(ui_body()).color(MUTED()));
                ui.add_space(6.0);
                ui.label(
                    RichText::new(tr("Switch to Continuous to keep writing."))
                        .font(ui_small())
                        .color(MUTED()),
                );
            });
            return;
        }
    };
    let layout = &paged.layout;
    // Fit the page to the column, then apply the editor's zoom.
    let fit = ((ui.available_width() - 72.0) / layout.width.max(1.0)).clamp(0.3, 1.6);
    let scale = (fit * app.editor_zoom.clamp(0.7, 1.6) / crate::theme::DEFAULT_EDITOR_ZOOM).clamp(0.3, 2.4);
    let mut page_tops = Vec::with_capacity(paged.count);
    let mut viewport_top = f32::NEG_INFINITY;
    let mut clicked: Option<crate::app::CaretRequest> = None;
    egui::ScrollArea::both()
        .id_salt("editor_paged_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            viewport_top = ui.clip_rect().top();
            ui.add_space(22.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(tr("Read-only page preview — click a paragraph to edit it"))
                        .font(ui_small())
                        .color(MUTED()),
                );
                ui.add_space(10.0);
                for page in paged.first..paged.first + paged.count {
                    let response = paint_page(ui, layout, page, scale);
                    page_tops.push(response.rect.top());
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                    if response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let local = (pos - response.rect.min) / scale;
                            if let Some((paragraph, offset)) =
                                hit_source(layout, page, local.x, local.y, doc_id)
                            {
                                clicked = Some(crate::app::CaretRequest {
                                    doc_id: doc_id.to_string(),
                                    paragraph,
                                    offset,
                                    select: 0,
                                    focus: true,
                                });
                            }
                        }
                    }
                    ui.add_space(26.0);
                }
            });
            ui.add_space(36.0);
        });
    if let Some(request) = clicked {
        // Back to the writing surface, caret where the writer clicked.
        app.caret_request = Some(request);
        app.write_mode = crate::app::WriteMode::Continuous;
    }
    app.paged_page = page_tops
        .iter()
        .enumerate()
        .rev()
        .find(|(_, top)| **top <= viewport_top + 1.0)
        .map(|(index, _)| index + 1)
        .unwrap_or(1)
        .min(paged.count.max(1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use portmanteau_core::ExportKind;

    #[test]
    fn every_exporter_face_has_a_registered_family() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx.run_ui(egui::RawInput::default(), |_ui| {}).drop_without_applying_deltas();
        for family in [Family::Serif, Family::Sans, Family::Mono] {
            for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
                let face = Face::new(family, bold, italic);
                let defined = ctx.fonts(|f| f.definitions().families.contains_key(&family_for_face(face)));
                assert!(defined, "{face:?}");
            }
        }
    }

    #[test]
    fn the_layout_is_the_exports_own() {
        let project = portmanteau_core::sample::demo();
        let settings = project.settings_for(ExportKind::Pdf);
        let ctx = egui::Context::default();
        let mut pages = 0;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            pages = manuscript_layout(ui, &project, &settings, "test").pages.len();
        })
        .drop_without_applying_deltas();
        assert_eq!(pages, portmanteau_export::pdf_page_count(&project, &settings));
    }

    fn run<T>(f: impl FnOnce(&egui::Ui) -> T) -> T {
        let ctx = egui::Context::default();
        let mut out = None;
        let mut f = Some(f);
        ctx.run_ui(egui::RawInput::default(), |ui| {
            out = Some((f.take().unwrap())(ui));
        })
        .drop_without_applying_deltas();
        out.unwrap()
    }

    #[test]
    fn a_documents_pages_are_its_pages_in_the_real_layout() {
        let project = portmanteau_core::sample::demo();
        let chapter = project
            .documents
            .iter()
            .find(|d| d.kind == portmanteau_core::DocKind::Chapter)
            .expect("a chapter");
        let paged = run(|ui| paged_document(ui, &project, &chapter.id)).expect("the chapter has pages");
        assert!(paged.count >= 1);
        assert!(paged.first + paged.count <= paged.layout.page_count());
        // The pages shown are the ones the section list assigns to it.
        let section = paged
            .layout
            .sections
            .iter()
            .find(|s| s.doc_id.as_deref() == Some(chapter.id.as_str()))
            .unwrap();
        assert_eq!((paged.first, paged.count), (section.start_page, section.page_count));
    }

    #[test]
    fn an_excluded_document_says_why_it_has_no_pages() {
        let mut project = portmanteau_core::sample::demo();
        let id = project
            .documents
            .iter()
            .find(|d| d.kind == portmanteau_core::DocKind::Chapter)
            .unwrap()
            .id
            .clone();
        project.document_mut(&id).unwrap().include_in_export = false;
        let reason = run(|ui| paged_document(ui, &project, &id)).err().expect("no pages");
        assert!(reason.contains("Excluded from compile"), "{reason}");
    }

    #[test]
    fn a_click_finds_the_paragraph_and_character_under_it() {
        let project = portmanteau_core::sample::demo();
        let chapter = project
            .documents
            .iter()
            .find(|d| d.kind == portmanteau_core::DocKind::Chapter)
            .unwrap();
        let paged = run(|ui| paged_document(ui, &project, &chapter.id)).unwrap();
        let page = &paged.layout.pages[paged.first];
        // Aim at the third character of the first line of document text.
        let item = page.items.iter().find(|item| item.source.is_some()).unwrap();
        let source = item.source.as_ref().unwrap();
        let first = &item.segments[0];
        let x = item.x + text_width(&first.text.chars().take(3).collect::<String>(), first.face, item.size) + 0.5;
        let hit = hit_source(&paged.layout, paged.first, x, item.baseline - item.size * 0.3, &chapter.id).unwrap();
        assert_eq!(hit, (source.paragraph, source.offset + 3));
        // A click far below the text lands on the last line, not nowhere.
        assert!(hit_source(&paged.layout, paged.first, 10.0, paged.layout.height - 5.0, &chapter.id).is_some());
        // Another document's id finds nothing.
        assert!(hit_source(&paged.layout, paged.first, x, item.baseline, "someone-else").is_none());
    }
}
