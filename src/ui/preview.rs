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

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use portmanteau_core::{ExportSettings, Project};
use portmanteau_export::{pdf_layout, text_width, Face, Family, PdfLayout};

use crate::theme::{PAPER, PAPER_EDGE, PAPER_TEXT};

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
}
