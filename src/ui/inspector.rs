//! The inspector: metadata, notes and versions for the open document.

use eframe::egui::{
    self, Align, ComboBox, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea,
    Stroke, TextEdit, Vec2,
};
use portmanteau_core::Medium;

use crate::app::{InspectorTab, PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::editor::{inline_format_toolbar, script_flow_layout_job};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        for (tab, label) in [
            (InspectorTab::Document, "Document"),
            (InspectorTab::Notes, "Notes"),
            (InspectorTab::History, "Versions"),
        ] {
            let active = app.inspector_tab == tab;
            let resp = ui.add(
                egui::Button::new(
                    RichText::new(label)
                        .font(sans(10.5))
                        .color(if active { TEXT } else { FAINT }),
                )
                .frame(false)
                .min_size(Vec2::new(55.0, 24.0)),
            );
            if resp.clicked() {
                app.inspector_tab = tab;
            }
            if active {
                let r = resp.rect;
                ui.painter().line_segment(
                    [
                        egui::pos2(r.left() + 8.0, r.bottom() + 1.0),
                        egui::pos2(r.right() - 8.0, r.bottom() + 1.0),
                    ],
                    Stroke::new(1.5, ACCENT),
                );
            }
        }
    });
    hairline(ui);
    ui.add_space(8.0);

    let Some(doc_id) = app.selected.clone() else {
        ui.label(RichText::new("Select a document in the binder.").font(sans(11.0)).color(FAINT));
        return;
    };
    if app.ws.project.document(&doc_id).is_none() {
        ui.label(RichText::new("This document no longer exists.").font(sans(11.0)).color(FAINT));
        return;
    }

    ScrollArea::vertical()
        .id_salt("inspector_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match app.inspector_tab {
                InspectorTab::Document => document_tab(app, ui, &doc_id),
                InspectorTab::Notes => notes_tab(app, ui, &doc_id),
                InspectorTab::History => history_tab(app, ui, &doc_id),
            }
        });
}

fn document_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    let mut restore: Option<String> = None;
    let mut compare: Option<String> = None;
    let mut snapshot = false;
    let styles = app.ws.project.styles.clone();
    let prose_sheet =
        portmanteau_core::StyleSheet::novel_prose(&app.ws.project.styles);
    {
        let Some(doc) = app.ws.project.document_mut(doc_id) else { return };
        section_label(ui, "Document");
        ui.add_space(3.0);
        if ui
            .add(
                TextEdit::singleline(&mut doc.title)
                    .desired_width(f32::INFINITY)
                    .font(serif(16.0))
                    .frame(Frame::NONE),
            )
            .changed()
        {
            doc.touch();
            app.ws.dirty = true;
        }
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(doc.kind.label()).font(sans(9.5)).color(FAINT));
            ui.label(RichText::new("·").font(sans(9.5)).color(FAINT));
            ui.label(RichText::new(doc.medium.label()).font(sans(9.5)).color(FAINT));
        });
        ui.add_space(10.0);

        section_label(ui, "Status");
        ui.add_space(2.0);
        ComboBox::from_id_salt("status_pick")
            .selected_text(RichText::new(&doc.status).font(sans(11.0)).color(status_color(&doc.status)))
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                for status in ["To do", "Outline", "Draft", "In progress", "Revised", "Final"] {
                    if ui.selectable_value(&mut doc.status, status.to_string(), status).clicked() {
                        app.ws.dirty = true;
                    }
                }
            });
        ui.add_space(7.0);
        section_label(ui, "Label");
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let color = label_color(&doc.label);
            let (r, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
            ui.painter().circle_filled(r.center(), 4.0, color);
            ComboBox::from_id_salt("label_pick")
                .selected_text(RichText::new(if doc.label.is_empty() { "None" } else { &doc.label }).font(sans(11.0)))
                .width(ui.available_width() - 20.0)
                .show_ui(ui, |ui| {
                    for label in ["", "Opening", "Development", "Reveal", "Script", "Stage"] {
                        let text = if label.is_empty() { "None" } else { label };
                        if ui.selectable_value(&mut doc.label, label.to_string(), text).clicked() {
                            app.ws.dirty = true;
                        }
                    }
                });
        });
        ui.add_space(7.0);
        section_label(ui, "Format");
        ui.add_space(2.0);
        let mut medium = doc.medium;
        ComboBox::from_id_salt("medium_pick")
            .selected_text(medium.label())
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                for m in [Medium::Prose, Medium::Screenplay, Medium::StagePlay, Medium::TvScript, Medium::Note] {
                    ui.selectable_value(&mut medium, m, m.label());
                }
            });
        if medium != doc.medium {
            doc.convert_medium(medium);
            app.ws.dirty = true;
        }

        ui.add_space(12.0);
        hairline(ui);
        ui.add_space(10.0);
        section_label(ui, "Synopsis");
        ui.add_space(3.0);
        let synopsis_id = egui::Id::unique(("inspector_synopsis", doc_id));
        let ctx = ui.ctx().clone();
        if inline_format_toolbar(ui, &ctx, synopsis_id, &mut doc.synopsis) {
            doc.touch();
            app.ws.dirty = true;
        }
        let mut synopsis = doc.synopsis.editor_text();
        let synopsis_content = doc.synopsis.clone();
        let mut synopsis_layouter =
            |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                let job = script_flow_layout_job(
                    text.as_str(),
                    &synopsis_content,
                    &prose_sheet,
                    &styles,
                    1.0,
                    wrap_width,
                    &[],
                );
                ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
            };
        let synopsis_output = TextEdit::multiline(&mut synopsis)
            .id(synopsis_id)
            .layouter(&mut synopsis_layouter)
            .hint_text("One or two sentences about this piece…")
            .desired_rows(4)
            .desired_width(f32::INFINITY)
            .font(sans(11.0))
            .show(ui);
        if synopsis_output.response.changed() {
            doc.synopsis.sync_editor_text(&synopsis, &prose_sheet);
            doc.touch();
            app.ws.dirty = true;
        }

        ui.add_space(12.0);
        hairline(ui);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let mut include = doc.include_in_export;
            if ui.checkbox(&mut include, "Include in compile").changed() {
                doc.include_in_export = include;
                app.ws.dirty = true;
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Word target").font(sans(10.5)).color(MUTED));
            let mut target = doc.word_target;
            if ui
                .add(egui::DragValue::new(&mut target).speed(50.0).range(0..=500_000))
                .changed()
            {
                doc.word_target = target;
                app.ws.dirty = true;
            }
        });

        ui.add_space(12.0);
        hairline(ui);
        ui.add_space(10.0);
        section_label(ui, "Statistics");
        ui.add_space(5.0);
        let words = doc.words();
        let pages = doc.pages();
        ui.horizontal(|ui| {
            stat_cell(ui, &words.to_string(), "words");
            stat_cell(ui, &pages.to_string(), "pages");
            stat_cell(ui, &format!("{}m", portmanteau_core::reading_minutes(words)), "read");
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("Created {}", doc.created))
                .font(sans(9.5))
                .color(FAINT),
        );
        ui.label(
            RichText::new(format!("Edited {}", doc.updated))
                .font(sans(9.5))
                .color(FAINT),
        );

        ui.add_space(12.0);
        hairline(ui);
        ui.add_space(10.0);
        section_label(ui, "Version snapshot");
        ui.add_space(3.0);
        ui.add(
            TextEdit::singleline(&mut app.snapshot_name)
                .hint_text("Name this version…")
                .desired_width(f32::INFINITY)
                .font(sans(11.0)),
        );
        ui.add_space(4.0);
        let btn = egui::Button::new(RichText::new("Take snapshot").font(sans(11.0)).color(TEXT))
            .fill(PANEL_3)
            .corner_radius(CornerRadius::same(5))
            .min_size(Vec2::new(ui.available_width(), 27.0));
        if ui.add(btn).clicked() {
            snapshot = true;
        }
        ui.add_space(8.0);
        let count = doc.snapshots.len();
        if count > 0 {
            ui.label(
                RichText::new(format!("{count} earlier versions kept")).font(sans(9.5)).color(FAINT),
            );
            let latest = doc.snapshots[0].clone();
            Frame::new()
                .fill(PANEL_2)
                .corner_radius(CornerRadius::same(5))
                .inner_margin(Margin::same(7))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&latest.name).font(sans(10.0)).color(TEXT));
                            ui.label(RichText::new(&latest.created).font(sans(9.0)).color(FAINT));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("Restore").clicked() {
                                restore = Some(latest.id.clone());
                            }
                            if ui.small_button("Compare").clicked() {
                                compare = Some(latest.id.clone());
                            }
                        });
                    });
                });
        }
    }

    if snapshot {
        app.take_snapshot_public();
    }
    if let Some(snap_id) = compare {
        app.open_diff(doc_id, &snap_id);
    }
    if let Some(snap_id) = restore {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            if doc.restore_snapshot(&snap_id) {
                app.ws.dirty = true;
                app.toast("Version restored", "The document was rolled back", ToastKind::Info);
            }
        }
    }
}

fn notes_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    let research: Vec<(String, String)> = app
        .ws
        .project
        .research
        .iter()
        .map(|r| (r.title.clone(), r.kind.clone()))
        .collect();
    let project_notes = app.ws.project.logline.clone();
    let styles = app.ws.project.styles.clone();
    let prose_sheet =
        portmanteau_core::StyleSheet::novel_prose(&app.ws.project.styles);
    {
        let Some(doc) = app.ws.project.document_mut(doc_id) else { return };
        section_label(ui, "Author's notes");
        ui.add_space(3.0);
        let notes_id = egui::Id::unique(("inspector_notes", doc_id));
        let ctx = ui.ctx().clone();
        if inline_format_toolbar(ui, &ctx, notes_id, &mut doc.notes) {
            doc.touch();
            app.ws.dirty = true;
        }
        let mut notes = doc.notes.editor_text();
        let notes_content = doc.notes.clone();
        let mut notes_layouter = |ui: &egui::Ui,
                                  text: &dyn egui::TextBuffer,
                                  wrap_width: f32| {
            let job = script_flow_layout_job(
                text.as_str(),
                &notes_content,
                &prose_sheet,
                &styles,
                1.0,
                wrap_width,
                &[],
            );
            ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
        };
        let notes_output = TextEdit::multiline(&mut notes)
            .id(notes_id)
            .layouter(&mut notes_layouter)
            .hint_text("Intentions, questions, things to fix…")
            .desired_rows(12)
            .desired_width(f32::INFINITY)
            .font(sans(11.0))
            .show(ui);
        if notes_output.response.changed() {
            doc.notes.sync_editor_text(&notes, &prose_sheet);
            doc.touch();
            app.ws.dirty = true;
        }
    }
    ui.add_space(14.0);
    hairline(ui);
    ui.add_space(10.0);
    section_label(ui, "Project compass");
    ui.add_space(4.0);
    ui.label(RichText::new(&project_notes).font(serif(12.0)).color(MUTED));
    ui.add_space(12.0);
    section_label(ui, "Linked research");
    ui.add_space(3.0);
    if research.is_empty() {
        ui.label(RichText::new("No research notes yet.").font(sans(10.5)).color(FAINT));
    }
    for (title, kind) in research.iter().take(5) {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(5.0, 5.0), egui::Sense::hover());
            ui.painter().circle_filled(r.center(), 2.5, BLUE);
            ui.label(RichText::new(title).font(sans(10.5)).color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(kind).font(sans(8.5)).color(FAINT));
            });
        });
    }
    ui.add_space(12.0);
}

fn history_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    let Some(doc) = app.ws.project.document(doc_id) else { return };
    section_label(ui, "Document versions");
    ui.add_space(3.0);
    ui.label(
        RichText::new(format!(
            "{} snapshots · project checkpoint {}",
            doc.snapshots.len(),
            if app.ws.project.checkpoints.is_empty() { "none" } else { "available" }
        ))
        .font(sans(9.5))
        .color(FAINT),
    );
    ui.add_space(8.0);
    if doc.snapshots.is_empty() {
        ui.label(
            RichText::new("No snapshots yet. Use Take snapshot before a big revision.")
                .font(sans(10.5))
                .color(FAINT),
        );
    }
    let snapshots: Vec<_> = doc.snapshots.iter().take(6).cloned().collect();
    let mut restore: Option<String> = None;
    let mut compare: Option<String> = None;
    for (i, snap) in snapshots.iter().enumerate() {
        Frame::new()
            .fill(PANEL_2)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&snap.name).font(sans(10.5)).color(TEXT));
                        ui.label(
                            RichText::new(format!("{} · {} words", snap.created, snap.words))
                                .font(sans(9.0))
                                .color(FAINT),
                        );
                        if !snap.note.is_empty() {
                            ui.label(RichText::new(&snap.note).font(sans(9.0)).color(FAINT));
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Restore").clicked() {
                            restore = Some(snap.id.clone());
                        }
                        if ui.small_button("Compare").clicked() {
                            compare = Some(snap.id.clone());
                        }
                        if i == 0 {
                            pill(ui, "latest", SAGE, SAGE_SOFT);
                        }
                    });
                });
            });
        ui.add_space(4.0);
    }
    ui.add_space(8.0);
    if let Some(snap_id) = compare {
        app.open_diff(doc_id, &snap_id);
    }
    if let Some(snap_id) = restore {
        if let Some(doc) = app.ws.project.document_mut(doc_id) {
            if doc.restore_snapshot(&snap_id) {
                app.ws.dirty = true;
                app.toast("Version restored", "The document was rolled back", ToastKind::Info);
            }
        }
    }
}

fn stat_cell(ui: &mut egui::Ui, value: &str, label: &str) {
    Frame::new()
        .fill(PANEL_2)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(value).font(serif(15.0)).color(TEXT));
                ui.label(RichText::new(label).font(sans(8.5)).color(FAINT));
            });
        });
}
