//! The inspector: metadata, notes and versions for the open document.

use eframe::egui::{
    self, Align, ComboBox, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea,
    Stroke, TextEdit, Vec2,
};
use portmanteau_core::Medium;

use crate::app::{ImageInsertState, InspectorTab, PortmanteauApp, RichTextTarget, ToastKind};
use crate::theme::*;
use crate::ui::editor::{
    inline_format_toolbar, load_rich_image_textures, paint_rich_images, script_flow_layout_job,
};

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
                        .font(ui_body())
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
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let hide = egui::Button::new(
                RichText::new("›").font(ui_body()).color(FAINT),
            )
            .frame(false);
            if ui
                .add(hide)
                .on_hover_text("Hide the inspector")
                .clicked()
            {
                app.set_inspector_visible(false);
            }
        });
    });
    hairline(ui);
    ui.add_space(8.0);

    let Some(doc_id) = app.selected.clone() else {
        ui.label(RichText::new("Select a document in the binder.").font(ui_body()).color(FAINT));
        return;
    };
    if app.ws.project.document(&doc_id).is_none() {
        ui.label(RichText::new("This document no longer exists.").font(ui_body()).color(FAINT));
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
    let pages = crate::ui::preview::page_counts(ui, &app.ws.project).label(doc_id);
    let styles = app.ws.project.styles.clone();
    let prose_sheet =
        portmanteau_core::StyleSheet::novel_prose(&app.ws.project.styles);
    let synopsis_snapshot = app
        .ws
        .project
        .document(doc_id)
        .map(|doc| doc.synopsis.clone())
        .unwrap_or_default();
    let synopsis_images = load_rich_image_textures(app, ui.ctx(), &synopsis_snapshot);
    // The checker and the setting are borrowed before the document is, so the
    // layouter and the right-click menu can read them (disjoint app fields).
    let checker = &app.spell;
    let spell_on = app.settings.spellcheck;
    let mut spelling: Vec<crate::ui::editor::FieldSpell> = Vec::new();
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
            ui.label(RichText::new(doc.kind.label()).font(ui_small()).color(FAINT));
            ui.label(RichText::new("·").font(ui_small()).color(FAINT));
            ui.label(RichText::new(doc.medium.label()).font(ui_small()).color(FAINT));
        });
        ui.add_space(10.0);

        section_label(ui, "Status");
        ui.add_space(2.0);
        ComboBox::from_id_salt("status_pick")
            .selected_text(RichText::new(&doc.status).font(ui_body()).color(status_color(&doc.status)))
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
                .selected_text(RichText::new(if doc.label.is_empty() { "None" } else { &doc.label }).font(ui_body()))
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
        let toolbar = inline_format_toolbar(ui, &ctx, synopsis_id, &mut doc.synopsis);
        if toolbar.changed {
            doc.touch();
            app.ws.dirty = true;
        }
        if let Some(range) = toolbar.insert_image {
            app.image_insert = Some(ImageInsertState {
                target: RichTextTarget::DocumentSynopsis(doc_id.to_owned()),
                range,
                path: String::new(),
                alt_text: String::new(),
                caption: String::new(),
                standalone: false,
                error: String::new(),
            });
        }
        let mut synopsis = doc.synopsis.editor_text();
        let synopsis_content = doc.synopsis.clone();
        let synopsis_spans = crate::ui::editor::field_spans(checker, spell_on, &synopsis);
        let mut synopsis_layouter =
            |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                let job = script_flow_layout_job(
                    text.as_str(),
                    &synopsis_content,
                    &prose_sheet,
                    &styles,
                    field_text_zoom(),
                    wrap_width,
                    &synopsis_spans,
                );
                ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
            };
        let synopsis_output = TextEdit::multiline(&mut synopsis)
            .id(synopsis_id)
            .layouter(&mut synopsis_layouter)
            .hint_text("One or two sentences about this piece…")
            .desired_rows(4)
            .desired_width(f32::INFINITY)
            .font(ui_body())
            .show(ui);
        if synopsis_output.response.changed() {
            doc.synopsis.sync_editor_text(&synopsis, &prose_sheet);
            crate::ui::editor::autolink_pasted(ui.ctx(), &mut doc.synopsis, synopsis_output.cursor_range.map(|r| r.primary.index.0));
            doc.touch();
            app.ws.dirty = true;
        }
        crate::ui::editor::remember_selection_anchor(ui.ctx(), synopsis_output.response.id, &synopsis_output.galley, synopsis_output.galley_pos);
        let fixed = crate::ui::editor::field_spell_menu(
            ui,
            checker,
            spell_on,
            egui::Id::unique(("spell_menu_synopsis", doc_id)),
            &synopsis_output,
            &synopsis,
            &mut doc.synopsis,
            &prose_sheet,
        );
        if fixed.replaced {
            doc.touch();
            app.ws.dirty = true;
        }
        spelling.push(fixed);
        paint_rich_images(
            ui,
            &synopsis_output.galley,
            synopsis_output.galley_pos,
            synopsis_output.response.rect,
            &doc.synopsis,
            &synopsis_images,
            field_text_zoom(),
        );

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
            ui.label(RichText::new("Word target").font(ui_body()).color(MUTED));
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
        ui.horizontal(|ui| {
            stat_cell(ui, &words.to_string(), "words");
            stat_cell(ui, &pages, "pages");
            stat_cell(ui, &format!("{}m", portmanteau_core::reading_minutes(words)), "read");
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("Created {}", doc.created))
                .font(ui_small())
                .color(FAINT),
        );
        ui.label(
            RichText::new(format!("Edited {}", doc.updated))
                .font(ui_small())
                .color(FAINT),
        );

        // Taking, comparing and restoring versions lives on the Versions tab
        // (and in Edit > Take snapshot). This tab used to repeat a snapshot
        // box and the latest version here; one home is enough.
    }
    crate::ui::editor::apply_field_spell(app, spelling);
}

fn notes_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    // Research notes this document actually names, not just the first few.
    let research: Vec<(String, String, String)> = app
        .ws
        .project
        .document(doc_id)
        .map(|doc| crate::ui::views::research::notes_mentioned_by(&app.ws.project, doc))
        .unwrap_or_default()
        .into_iter()
        .map(|note| {
            (note.id.clone(), note.title.clone(), crate::ui::views::research::kind_of(note).to_string())
        })
        .collect();
    let mut open_note: Option<String> = None;
    let project_notes = app.ws.project.logline.clone();
    let styles = app.ws.project.styles.clone();
    let prose_sheet =
        portmanteau_core::StyleSheet::novel_prose(&app.ws.project.styles);
    let notes_snapshot = app
        .ws
        .project
        .document(doc_id)
        .map(|doc| doc.notes.clone())
        .unwrap_or_default();
    let notes_images = load_rich_image_textures(app, ui.ctx(), &notes_snapshot);
    let checker = &app.spell;
    let spell_on = app.settings.spellcheck;
    let mut spelling: Vec<crate::ui::editor::FieldSpell> = Vec::new();
    {
        let Some(doc) = app.ws.project.document_mut(doc_id) else { return };
        section_label(ui, "Author's notes");
        ui.add_space(3.0);
        let notes_id = egui::Id::unique(("inspector_notes", doc_id));
        let ctx = ui.ctx().clone();
        let toolbar = inline_format_toolbar(ui, &ctx, notes_id, &mut doc.notes);
        if toolbar.changed {
            doc.touch();
            app.ws.dirty = true;
        }
        if let Some(range) = toolbar.insert_image {
            app.image_insert = Some(ImageInsertState {
                target: RichTextTarget::DocumentNotes(doc_id.to_owned()),
                range,
                path: String::new(),
                alt_text: String::new(),
                caption: String::new(),
                standalone: false,
                error: String::new(),
            });
        }
        let mut notes = doc.notes.editor_text();
        let notes_content = doc.notes.clone();
        let notes_spans = crate::ui::editor::field_spans(checker, spell_on, &notes);
        let mut notes_layouter = |ui: &egui::Ui,
                                  text: &dyn egui::TextBuffer,
                                  wrap_width: f32| {
            let job = script_flow_layout_job(
                text.as_str(),
                &notes_content,
                &prose_sheet,
                &styles,
                field_text_zoom(),
                wrap_width,
                &notes_spans,
            );
            ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
        };
        let notes_output = TextEdit::multiline(&mut notes)
            .id(notes_id)
            .layouter(&mut notes_layouter)
            .hint_text("Intentions, questions, things to fix…")
            .desired_rows(12)
            .desired_width(f32::INFINITY)
            .font(ui_body())
            .show(ui);
        if notes_output.response.changed() {
            doc.notes.sync_editor_text(&notes, &prose_sheet);
            crate::ui::editor::autolink_pasted(ui.ctx(), &mut doc.notes, notes_output.cursor_range.map(|r| r.primary.index.0));
            doc.touch();
            app.ws.dirty = true;
        }
        crate::ui::editor::remember_selection_anchor(ui.ctx(), notes_output.response.id, &notes_output.galley, notes_output.galley_pos);
        let fixed = crate::ui::editor::field_spell_menu(
            ui,
            checker,
            spell_on,
            egui::Id::unique(("spell_menu_notes", doc_id)),
            &notes_output,
            &notes,
            &mut doc.notes,
            &prose_sheet,
        );
        if fixed.replaced {
            doc.touch();
            app.ws.dirty = true;
        }
        spelling.push(fixed);
        paint_rich_images(
            ui,
            &notes_output.galley,
            notes_output.galley_pos,
            notes_output.response.rect,
            &doc.notes,
            &notes_images,
            field_text_zoom(),
        );
    }
    crate::ui::editor::apply_field_spell(app, spelling);
    ui.add_space(14.0);
    hairline(ui);
    ui.add_space(10.0);
    section_label(ui, "Project compass");
    ui.add_space(4.0);
    ui.label(RichText::new(&project_notes).font(serif(12.0)).color(MUTED));
    ui.add_space(12.0);
    section_label(ui, "Research mentioned here");
    ui.add_space(3.0);
    if research.is_empty() {
        ui.label(
            RichText::new("No research note is named in this document.")
                .font(ui_small())
                .color(FAINT),
        );
    }
    for (id, title, kind) in research.iter().take(6) {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(5.0, 5.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(r.center(), 2.5, crate::ui::views::research::kind_color(kind));
            if ui
                .add(egui::Button::new(RichText::new(title).font(ui_body()).color(BLUE)).frame(false))
                .on_hover_text("Open in Research")
                .clicked()
            {
                open_note = Some(id.clone());
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(kind).font(ui_small()).color(FAINT));
            });
        });
    }
    ui.add_space(12.0);
    if let Some(id) = open_note {
        app.selected_research = Some(id);
        app.view = crate::app::View::Research;
    }
}

fn history_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui, doc_id: &str) {
    let mut take_snapshot = false;
    section_label(ui, "Save a version");
    ui.add_space(3.0);
    ui.add(
        TextEdit::singleline(&mut app.snapshot_name)
            .hint_text("Name this version…")
            .desired_width(f32::INFINITY)
            .font(ui_body()),
    );
    ui.add_space(4.0);
    let button = egui::Button::new(RichText::new("Take snapshot").font(ui_body()).color(TEXT))
        .fill(PANEL_3)
        .corner_radius(CornerRadius::same(5))
        .min_size(Vec2::new(ui.available_width(), 27.0));
    if ui.add(button).clicked() {
        take_snapshot = true;
    }
    ui.add_space(10.0);
    hairline(ui);
    ui.add_space(8.0);
    if take_snapshot {
        app.take_snapshot_public();
    }
    let Some(doc) = app.ws.project.document(doc_id) else { return };
    section_label(ui, "Document versions");
    ui.add_space(3.0);
    ui.label(
        RichText::new(format!(
            "{} snapshots · project checkpoint {}",
            doc.snapshots.len(),
            if app.ws.project.checkpoints.is_empty() { "none" } else { "available" }
        ))
        .font(ui_small())
        .color(FAINT),
    );
    ui.add_space(8.0);
    if doc.snapshots.is_empty() {
        ui.label(
            RichText::new("No snapshots yet. Take one before a big revision.")
                .font(ui_body())
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
                        ui.label(RichText::new(&snap.name).font(ui_body()).color(TEXT));
                        ui.label(
                            RichText::new(format!("{} · {} words", snap.created, snap.words))
                                .font(ui_small())
                                .color(FAINT),
                        );
                        if !snap.note.is_empty() {
                            ui.label(RichText::new(&snap.note).font(ui_small()).color(FAINT));
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
                ui.label(RichText::new(label).font(ui_small()).color(FAINT));
            });
        });
}
