//! Page geometry shared by every writer.
//!
//! The one rule: a writer never owns a margin or a paper constant. It asks this
//! module, so a writer who changes the page margins (or the paper) in Compile
//! sees the same page in every format.
//!
//! Units: the model stores points (1/72 inch). DOCX and RTF want twips
//! (1/20 point); CSS takes points directly.

use super::*;

/// Book margins used when the writer has not set any: 20 mm at the sides and
/// 22 mm top and bottom, which is a comfortable trade-paperback text block.
const PROSE_SIDE_MM: f32 = 20.0;
const PROSE_VERTICAL_MM: f32 = 22.0;

/// Width of a US Letter page in points. Script sheets are designed on it, so a
/// narrower page scales the whole sheet by `paper width / LETTER_WIDTH`.
const LETTER_WIDTH_PT: f32 = 612.0;

pub(crate) fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

pub(crate) fn pt_to_twips(pt: f32) -> i64 {
    (pt * 20.0).round() as i64
}

/// Paper size in points.
pub(crate) fn paper_pt(settings: &ExportSettings) -> (f32, f32) {
    let (w_mm, h_mm) = settings.paper.mm();
    (mm_to_pt(w_mm), mm_to_pt(h_mm))
}

/// Paper size in twips, the unit DOCX and RTF use.
pub(crate) fn paper_twips(settings: &ExportSettings) -> (i64, i64) {
    let (w, h) = paper_pt(settings);
    (pt_to_twips(w), pt_to_twips(h))
}

/// How much a script sheet shrinks on a paper narrower than Letter. Letter
/// itself is exactly 1.
pub(crate) fn script_scale(settings: &ExportSettings) -> f32 {
    (paper_pt(settings).0 / LETTER_WIDTH_PT).clamp(0.3, 1.0)
}

/// The style sheet that governs a script compile: the first included
/// document's sheet, else the project's screenplay defaults.
pub(crate) fn script_sheet(project: &Project, settings: &ExportSettings) -> StyleSheet {
    project
        .compile_plan(settings)
        .included
        .iter()
        .find(|doc| !matches!(doc.kind, DocKind::FrontMatter | DocKind::BackMatter))
        .map(|doc| project.style_sheet_for(doc))
        .unwrap_or_else(|| StyleSheet::for_medium(Medium::Screenplay, &project.styles))
}

/// The margins this compile uses, in points, always leaving a usable text block.
pub(crate) fn page_margins(project: &Project, settings: &ExportSettings) -> PageMargins {
    let (width, height) = paper_pt(settings);
    let wanted = settings.margins.unwrap_or_else(|| {
        if project.compile_plan(settings).kind.is_script() {
            let scale = script_scale(settings);
            let sheet = script_sheet(project, settings).margins;
            PageMargins {
                left: sheet.left * scale,
                right: sheet.right * scale,
                top: sheet.top * scale,
                bottom: sheet.bottom * scale,
            }
        } else {
            PageMargins {
                left: mm_to_pt(PROSE_SIDE_MM),
                right: mm_to_pt(PROSE_SIDE_MM),
                top: mm_to_pt(PROSE_VERTICAL_MM),
                bottom: mm_to_pt(PROSE_VERTICAL_MM),
            }
        }
    });
    // Never let margins swallow the page: keep at least a third of each axis.
    let clamp_pair = |a: f32, b: f32, span: f32| {
        let room = span * (2.0 / 3.0);
        let total = a.max(0.0) + b.max(0.0);
        if total <= room || total == 0.0 {
            (a.max(0.0), b.max(0.0))
        } else {
            (a.max(0.0) * room / total, b.max(0.0) * room / total)
        }
    };
    let (left, right) = clamp_pair(wanted.left, wanted.right, width);
    let (top, bottom) = clamp_pair(wanted.top, wanted.bottom, height);
    PageMargins {
        left,
        right,
        top,
        bottom,
    }
}
