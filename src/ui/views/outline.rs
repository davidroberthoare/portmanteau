//! Outline: a compact, sortable table of the whole manuscript.

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, Vec2,
};
use portmanteau_core::{DocKind, Medium};

use crate::app::{PortmanteauApp, View};
use crate::theme::*;
use crate::ui::views::{content_frame, meta_line, page_head};

struct Row {
    id: String,
    title: String,
    kind: DocKind,
    medium: Medium,
    label: String,
    status: String,
    words: u32,
    target: u32,
    include: bool,
    depth: usize,
}

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        "Structure",
        "Outline",
        "Titles, labels, status and lengths in one place. Click any row to write.",
        |_ui| {},
    );

    content_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            for (value, label) in [(0usize, "Everything"), (1, "Prose"), (2, "Script")] {
                let active = app.outline_filter == value;
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(label)
                                .font(ui_small())
                                .color(if active { ON_ACCENT() } else { MUTED() }),
                        )
                        .fill(if active { ACCENT() } else { RAISED() })
                        .corner_radius(CornerRadius::same(12))
                        .min_size(Vec2::new(0.0, 22.0)),
                    )
                    .clicked()
                {
                    app.outline_filter = value;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let total: u32 = app
                    .ws
                    .project
                    .manuscript_documents()
                    .iter()
                    .map(|d| d.words())
                    .sum();
                meta_line(ui, &[&format!("{total} words in the manuscript")]);
            });
        });
        ui.add_space(10.0);

        let mut rows = collect(app);
        if app.outline_filter == 1 {
            rows.retain(|r| !r.medium.is_script());
        } else if app.outline_filter == 2 {
            rows.retain(|r| r.medium.is_script());
        }

        let mut open: Option<String> = None;
        let mut toggle: Option<String> = None;

        ScrollArea::vertical()
            .id_salt("outline_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                header_row(ui);
                for (i, row) in rows.iter().enumerate() {
                    let height = 34.0;
                    let (rect, resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), height),
                        Sense::click(),
                    );
                    let fill = if i % 2 == 0 { PANEL() } else { BG() };
                    ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, CornerRadius::same(4), RAISED());
                    }
                    let painter = ui.painter();

                    let title_x = rect.left() + 12.0 + row.depth as f32 * 14.0;
                    let glyph = match row.kind {
                        DocKind::Folder => "▣",
                        DocKind::Chapter => "¶",
                        DocKind::Scene => "▶",
                        DocKind::Text => "◆",
                        _ => "·",
                    };
                    painter.text(
                        egui::pos2(title_x, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        glyph,
                        ui_small(),
                        if row.medium.is_script() { SPECIAL() } else { ACCENT() },
                    );
                    let title_rect = egui::Rect::from_min_max(
                        egui::pos2(title_x + 17.0, rect.top()),
                        egui::pos2(rect.left() + rect.width() * 0.46, rect.bottom()),
                    );
                    let clipped = ui.painter().with_clip_rect(title_rect);
                    clipped.text(
                        title_rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        &row.title,
                        if row.kind == DocKind::Folder {
                            ui_body()
                        } else {
                            ui_body()
                        },
                        if row.include { TEXT() } else { MUTED() },
                    );

                    let label_x = rect.left() + rect.width() * 0.47;
                    if !row.label.is_empty() {
                        let color = label_color(&row.label);
                        ui.painter().circle_filled(
                            egui::pos2(label_x + 4.0, rect.center().y),
                            3.0,
                            color,
                        );
                        ui.painter().text(
                            egui::pos2(label_x + 12.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &row.label,
                            ui_small(),
                            color,
                        );
                    }

                    let status_x = rect.left() + rect.width() * 0.61;
                    ui.painter().text(
                        egui::pos2(status_x, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &row.status,
                        ui_small(),
                        status_color(&row.status),
                    );

                    let words_x = rect.right() - 196.0;
                    ui.painter().text(
                        egui::pos2(words_x, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{} w", row.words),
                        mono(9.0),
                        MUTED(),
                    );

                    // Target progress bar.
                    if row.target > 0 {
                        let bar = egui::Rect::from_min_size(
                            egui::pos2(rect.right() - 184.0, rect.center().y - 2.0),
                            Vec2::new(86.0, 4.0),
                        );
                        ui.painter().rect_filled(bar, CornerRadius::same(2), HOVER());
                        let fraction = (row.words as f32 / row.target as f32).clamp(0.0, 1.0);
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(bar.min, Vec2::new(bar.width() * fraction, 4.0)),
                            CornerRadius::same(2),
                            SUCCESS(),
                        );
                    }

                    let check_rect = egui::Rect::from_center_size(
                        egui::pos2(rect.right() - 16.0, rect.center().y),
                        Vec2::splat(16.0),
                    );
                    let mut include = row.include;
                    let check = ui.put(check_rect, egui::Checkbox::without_text(&mut include));
                    let _ = check;
                    if include != row.include {
                        toggle = Some(row.id.clone());
                    }

                    if resp.clicked() && !row.kind.is_folder() {
                        open = Some(row.id.clone());
                    }
                }
                ui.add_space(8.0);
            });

        if let Some(id) = toggle {
            if let Some(doc) = app.ws.project.document_mut(&id) {
                doc.include_in_export = !doc.include_in_export;
                app.ws.dirty = true;
            }
        }
        if let Some(id) = open {
            app.open_doc(id);
            app.view = View::Write;
        }
    });
}

fn header_row(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 22.0),
        Sense::hover(),
    );
    let painter = ui.painter();
    painter.text(
        rect.left_center() + Vec2::new(12.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "SECTION",
        ui_small(),
        MUTED(),
    );
    for (fraction, label) in [
        (0.47, "LABEL"),
        (0.61, "STATUS"),
        (0.805, "LENGTH"),
    ] {
        painter.text(
            egui::pos2(rect.left() + rect.width() * fraction, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            ui_small(),
            MUTED(),
        );
    }
    painter.text(
        egui::pos2(rect.right() - 6.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        "COMPILE",
        ui_small(),
        MUTED(),
    );
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.bottom() - 1.0),
            egui::pos2(rect.right(), rect.bottom() - 1.0),
        ],
        Stroke::new(1.0, BORDER()),
    );
}

fn collect(app: &PortmanteauApp) -> Vec<Row> {
    app.ws
        .project
        .manuscript_documents()
        .into_iter()
        .filter(|d| !matches!(d.kind, DocKind::FrontMatter | DocKind::BackMatter))
        .map(|d| Row {
            id: d.id.clone(),
            title: d.title.clone(),
            kind: d.kind,
            medium: d.medium,
            label: d.label.clone(),
            status: d.status.clone(),
            words: d.words(),
            target: d.word_target,
            include: d.include_in_export,
            depth: depth_of(app, d),
        })
        .collect()
}

fn depth_of(app: &PortmanteauApp, doc: &portmanteau_core::Document) -> usize {
    let mut depth: usize = 0;
    let mut parent = doc.parent.clone();
    while let Some(id) = parent {
        depth += 1;
        parent = app
            .ws
            .project
            .document(&id)
            .and_then(|d| d.parent.clone());
        if depth > 8 {
            break;
        }
    }
    depth.saturating_sub(1)
}

pub fn _unused(ui: &mut egui::Ui) {
    let _ = Frame::new().inner_margin(Margin::same(0)).show(ui, |_| {});
}
