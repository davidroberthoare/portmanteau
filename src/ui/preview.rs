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

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use portmanteau_core::{CompileScope, CompileType, ExportKind, ExportSettings, Project};
use portmanteau_export::{pdf_layout, text_width, Face, Family, PdfLayout};

use crate::theme::{serif, ui_body, ui_small, FAINT, MUTED, PAPER, PAPER_EDGE, PAPER_TEXT};

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

/// Draw one page of `layout` at `scale` screen points per page point.
/// A page past the end (an empty manuscript) draws as blank paper.
pub fn paint_page(ui: &mut egui::Ui, layout: &PdfLayout, index: usize, scale: f32) -> Rect {
    let size = Vec2::new(layout.width, layout.height) * scale;
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return rect;
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
        PAPER,
        Stroke::new(1.0, PAPER_EDGE),
        egui::StrokeKind::Inside,
    );
    let Some(page) = layout.pages.get(index) else {
        return rect;
    };
    for item in &page.items {
        let mut x = rect.left() + item.x * scale;
        let baseline = rect.top() + item.baseline * scale;
        for segment in &item.segments {
            if !segment.text.trim().is_empty() {
                let galley = painter.layout_no_wrap(
                    segment.text.clone(),
                    FontId::new(item.size * scale, family_for_face(segment.face)),
                    PAPER_TEXT,
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
                painter.galley(Pos2::new(x, baseline - ascent), galley, PAPER_TEXT);
            }
            x += text_width(&segment.text, segment.face, item.size) * scale;
        }
    }
    rect
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

/// The Paged view: the document's pages stacked on a desk, read-only. The
/// page under the top of the viewport is written to `app.paged_page` (1-based
/// within the document) for the toolbar.
pub fn paged_view(
    app: &mut crate::app::PortmanteauApp,
    ui: &mut egui::Ui,
    paged: &Result<PagedDocument, String>,
) {
    let paged = match paged {
        Ok(paged) => paged,
        Err(reason) => {
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                ui.label(RichText::new("Nothing to page").font(serif(26.0)).color(MUTED));
                ui.add_space(8.0);
                ui.label(RichText::new(reason).font(ui_body()).color(FAINT));
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Switch to Continuous to keep writing.")
                        .font(ui_small())
                        .color(FAINT),
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
    egui::ScrollArea::both()
        .id_salt("editor_paged_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            viewport_top = ui.clip_rect().top();
            ui.add_space(22.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Read-only page preview — switch to Continuous to edit")
                        .font(ui_small())
                        .color(FAINT),
                );
                ui.add_space(10.0);
                for page in paged.first..paged.first + paged.count {
                    let rect = paint_page(ui, layout, page, scale);
                    page_tops.push(rect.top());
                    ui.add_space(26.0);
                }
            });
            ui.add_space(36.0);
        });
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
}
