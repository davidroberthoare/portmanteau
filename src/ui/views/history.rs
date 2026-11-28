//! Version history: project checkpoints, document snapshots and the writing
//! calendar that shows how the draft was actually made.

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};

use crate::app::{PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::i18n::tr;
use crate::ui::views::{content_frame, meta_line, page_head, primary_button};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        tr("Time"),
        tr("Versions"),
        tr("Nothing is lost. Checkpoints capture the whole project; snapshots capture one document."),
        |ui| {
            meta_line(
                ui,
                &[&format!(
                    "{} checkpoint{} · {} snapshot{}",
                    app.ws.project.checkpoints.len(),
                    if app.ws.project.checkpoints.len() == 1 { "" } else { "s" },
                    snapshot_count(app),
                    if snapshot_count(app) == 1 { "" } else { "s" }
                )],
            );
        },
    );

    content_frame().show(ui, |ui| {
        ScrollArea::vertical()
            .id_salt("history_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Checkpoint composer.
                Frame::new()
                    .fill(PANEL())
                    .stroke(Stroke::new(1.0, BORDER()))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(14, 11))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                section_label(ui, tr("Save a checkpoint"));
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(
                                        "A full copy of the project at this moment, including every document and note.",
                                    )
                                    .font(ui_small())
                                    .color(MUTED()),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if primary_button(ui, tr("Save checkpoint")).clicked() {
                                    let label = if app.checkpoint_label.trim().is_empty() {
                                        format!(
                                            "Checkpoint · {}",
                                            chrono::Local::now().format("%d %b %H:%M")
                                        )
                                    } else {
                                        app.checkpoint_label.clone()
                                    };
                                    app.ws.project.checkpoint(label.clone(), "manual");
                                    app.checkpoint_label.clear();
                                    app.ws.dirty = true;
                                    app.toast(tr("Checkpoint saved"), label, ToastKind::Ok);
                                }
                                ui.add(
                                    TextEdit::singleline(&mut app.checkpoint_label)
                                        .hint_text(tr("Label this moment"))
                                        .font(ui_body())
                                        .desired_width(220.0),
                                );
                            });
                        });
                    });
                ui.add_space(14.0);

                // Writing calendar.
                session_chart(app, ui);
                ui.add_space(16.0);

                let height = (ui.available_height() - 8.0).max(320.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new((ui.available_width() * 0.46).max(380.0), height),
                        Layout::top_down(Align::Min),
                        |ui| checkpoints_list(app, ui, height),
                    );
                    ui.add_space(14.0);
                    ui.vertical(|ui| snapshots_list(app, ui, height));
                });
            });
    });
}

fn snapshot_count(app: &PortmanteauApp) -> usize {
    app.ws
        .project
        .documents
        .iter()
        .map(|d| d.snapshots.len())
        .sum()
}

fn session_chart(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let days = portmanteau_core::activity::per_day(&app.ws.project.activity);
    let entries: Vec<(String, u32)> = (0..14)
        .rev()
        .map(|back| {
            let key = (chrono::Local::now().date_naive() - chrono::Duration::days(back)).format("%Y-%m-%d").to_string();
            let words = days.get(&key).map_or(0, |t| t.added);
            (key, words)
        })
        .collect();
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, tr("The last fourteen days"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let total: u32 = entries.iter().map(|(_, w)| *w).sum();
                    meta_line(ui, &[&crate::i18n::trf("{total} words logged", &[("total", total.to_string())])]);
                });
            });
            ui.add_space(8.0);
            let height = 104.0;
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), height),
                Sense::hover(),
            );
            let max = entries.iter().map(|(_, w)| *w).max().unwrap_or(1).max(1) as f32;
            let count = entries.len().max(1);
            let slot = rect.width() / count as f32;
            for (i, (date, words)) in entries.iter().enumerate() {
                let bar_h = (height - 28.0) * (*words as f32 / max);
                let x = rect.left() + slot * i as f32 + slot * 0.22;
                let bar = egui::Rect::from_min_max(
                    egui::pos2(x, rect.bottom() - 18.0 - bar_h),
                    egui::pos2(x + slot * 0.56, rect.bottom() - 18.0),
                );
                ui.painter().rect_filled(
                    bar,
                    CornerRadius::same(3),
                    if *words == 0 {
                        HOVER()
                    } else if i + 1 == count {
                        ACCENT()
                    } else {
                        ACCENT().gamma_multiply(0.55)
                    },
                );
                let day: String = date.chars().skip(8).collect();
                ui.painter().text(
                    egui::pos2(x + slot * 0.28, rect.bottom() - 4.0),
                    egui::Align2::CENTER_BOTTOM,
                    day,
                    ui_small(),
                    MUTED(),
                );
                if *words > 0 {
                    ui.painter().text(
                        egui::pos2(x + slot * 0.28, bar.top() - 2.0),
                        egui::Align2::CENTER_BOTTOM,
                        words.to_string(),
                        ui_small(),
                        MUTED(),
                    );
                }
            }
        });
}

fn checkpoints_list(app: &mut PortmanteauApp, ui: &mut egui::Ui, height: f32) {
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, tr("Project checkpoints"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    meta_line(ui, &[&format!("{}", app.ws.project.checkpoints.len())]);
                });
            });
            ui.add_space(6.0);
            ScrollArea::vertical()
                .id_salt("checkpoint_list")
                .auto_shrink([false, false])
                .max_height(height - 52.0)
                .show(ui, |ui| {
                    let checkpoints = app.ws.project.checkpoints.clone();
                    if checkpoints.is_empty() {
                        ui.label(
                            RichText::new(tr("No checkpoints yet."))
                                .font(ui_small())
                                .color(MUTED()),
                        );
                    }
                    let mut restore: Option<String> = None;
                    for cp in &checkpoints {
                        Frame::new()
                            .fill(RAISED())
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::same(9))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let (dot_rect, _) =
                                        ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                    let color = match cp.kind.as_str() {
                                        "milestone" => ACCENT(),
                                        "export" => INFO(),
                                        _ => SUCCESS(),
                                    };
                                    ui.painter().circle_filled(dot_rect.center(), 3.5, color);
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&cp.label).font(ui_body()).color(TEXT()),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {} words · {}",
                                                cp.created, cp.words, cp.kind
                                            ))
                                            .font(ui_small())
                                            .color(MUTED()),
                                        );
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.small_button(tr("Restore")).clicked() {
                                            restore = Some(cp.id.clone());
                                        }
                                    });
                                });
                            });
                        ui.add_space(5.0);
                    }
                    if let Some(id) = restore {
                        if app.ws.project.restore_checkpoint(&id) {
                            app.ws.dirty = true;
                            if !app
                                .selected
                                .as_ref()
                                .map(|s| app.ws.project.document(s).is_some())
                                .unwrap_or(false)
                            {
                                app.selected = app
                                    .ws
                                    .project
                                    .documents
                                    .iter()
                                    .find(|d| d.kind == portmanteau_core::DocKind::Chapter)
                                    .map(|d| d.id.clone());
                            }
                            app.toast(
                                tr("Project restored"),
                                tr("The whole project was rolled back to this checkpoint"),
                                ToastKind::Info,
                            );
                        }
                    }
                });
        });
}

fn snapshots_list(app: &mut PortmanteauApp, ui: &mut egui::Ui, height: f32) {
    Frame::new()
        .fill(BG())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, tr("Document snapshots"));
            ui.add_space(6.0);
            ScrollArea::vertical()
                .id_salt("snapshot_list")
                .auto_shrink([false, false])
                .max_height(height - 46.0)
                .show(ui, |ui| {
                    let docs: Vec<(String, String, Vec<portmanteau_core::Snapshot>)> = app
                        .ws
                        .project
                        .documents
                        .iter()
                        .filter(|d| !d.snapshots.is_empty())
                        .map(|d| (d.id.clone(), d.title.clone(), d.snapshots.clone()))
                        .collect();
                    if docs.is_empty() {
                        ui.label(
                            RichText::new(tr("No document snapshots yet. Use Snapshot in the editor toolbar before a big cut."))
                                .font(ui_small())
                                .color(MUTED()),
                        );
                    }
                    let mut restore: Option<(String, String)> = None;
                    let mut compare: Option<(String, String)> = None;
                    for (doc_id, title, snaps) in &docs {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(title).font(ui_body()).color(TEXT()));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                meta_line(
                                    ui,
                                    &[&format!(
                                        "{} version{}",
                                        snaps.len(),
                                        if snaps.len() == 1 { "" } else { "s" }
                                    )],
                                );
                            });
                        });
                        for (i, snap) in snaps.iter().take(5).enumerate() {
                            Frame::new()
                                .fill(RAISED())
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(&snap.name)
                                                    .font(ui_small())
                                                    .color(if i == 0 { TEXT() } else { MUTED() }),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · {} words",
                                                    snap.created, snap.words
                                                ))
                                                .font(ui_small())
                                                .color(MUTED()),
                                            );
                                        });
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if ui.small_button(tr("Restore")).clicked() {
                                                    restore = Some((
                                                        doc_id.clone(),
                                                        snap.id.clone(),
                                                    ));
                                                }
                                                if ui.small_button(tr("Compare")).clicked() {
                                                    compare = Some((
                                                        doc_id.clone(),
                                                        snap.id.clone(),
                                                    ));
                                                }
                                                if i == 0 {
                                                    pill(ui, "latest", SUCCESS(), ACCENT_SOFT());
                                                }
                                            },
                                        );
                                    });
                                });
                            ui.add_space(3.0);
                        }
                        ui.add_space(6.0);
                    }
                    if let Some((doc_id, snap_id)) = compare {
                        app.open_diff(&doc_id, &snap_id);
                    }
                    if let Some((doc_id, snap_id)) = restore {
                        if let Some(doc) = app.ws.project.document_mut(&doc_id) {
                            if doc.restore_snapshot(&snap_id) {
                                app.selected = Some(doc_id);
                                app.ws.dirty = true;
                                app.toast(
                                    tr("Snapshot restored"),
                                    tr("The document was rolled back"),
                                    ToastKind::Info,
                                );
                            }
                        }
                    }
                });
        });
}
