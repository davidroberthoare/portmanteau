//! Story database: characters, places, events, props and custom records,
//! each with notes, tags, related links and a colour.

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea,
    Sense, Stroke, TextEdit, Vec2,
};

use portmanteau_core::{DatabaseItem, ItemKind, StyleSheet};

use crate::app::{ImageInsertState, PortmanteauApp, RichTextTarget};
use crate::theme::*;
use crate::ui::editor::{
    inline_format_toolbar, load_rich_image_textures, paint_rich_images,
    script_flow_layout_job,
};
use crate::ui::views::{content_frame, meta_line, page_head, primary_button, section_label};

const FACE_COLORS: [Color32; 6] = [
    Color32::from_rgb(204, 160, 92),
    Color32::from_rgb(132, 170, 145),
    Color32::from_rgb(166, 143, 185),
    Color32::from_rgb(124, 156, 196),
    Color32::from_rgb(201, 122, 108),
    Color32::from_rgb(176, 166, 132),
];

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut add_kind: Option<ItemKind> = None;
    let counts = counts_line(&app.ws.project.items);

    page_head(
        ui,
        "Story database",
        "Database",
        "Characters, places, events and props, with notes, tags and links.",
        |ui| {
            if primary_button(ui, "+ New item").clicked() {
                add_kind = Some(selected_kind(app).unwrap_or(ItemKind::Character));
            }
            ui.add_space(6.0);
            meta_line(ui, &[&counts]);
        },
    );

    if !app.ws.project.items.is_empty()
        && !app
            .ws
            .project
            .items
            .iter()
            .any(|i| Some(&i.id) == app.selected_item.as_ref())
    {
        app.selected_item = Some(app.ws.project.items[0].id.clone());
    }

    content_frame().show(ui, |ui| {
        if app.ws.project.items.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Nothing catalogued yet").font(serif(20.0)).color(TEXT));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Keep the people, places and things your story needs.")
                        .font(ui_body())
                        .color(MUTED),
                );
                ui.add_space(12.0);
                if primary_button(ui, "Create a character").clicked() {
                    add_kind = Some(ItemKind::Character);
                }
            });
            return;
        }

        let height = (ui.available_height() - 8.0).max(340.0);
        let mut select: Option<String> = None;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(300.0, height),
                Layout::top_down(Align::Min),
                |ui| {
                    Frame::new()
                        .fill(PANEL)
                        .stroke(Stroke::new(1.0, BORDER_SOFT))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(9))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ScrollArea::vertical()
                                .id_salt("database_list")
                                .auto_shrink([false, false])
                                .max_height(height - 20.0)
                                .show(ui, |ui| {
                                    for kind in ItemKind::all() {
                                        let count = app
                                            .ws
                                            .project
                                            .items
                                            .iter()
                                            .filter(|i| i.kind == kind)
                                            .count();
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · {}",
                                                    kind.label().to_uppercase(),
                                                    count
                                                ))
                                                .font(ui_small())
                                                .color(FAINT)
                                                .extra_letter_spacing(1.0),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if ui
                                                        .add(
                                                            egui::Button::new(
                                                                RichText::new("+")
                                                                    .font(ui_strong())
                                                                    .color(ACCENT_INK),
                                                            )
                                                            .frame(false),
                                                        )
                                                        .on_hover_text(format!(
                                                            "New {}",
                                                            kind.label().to_lowercase()
                                                        ))
                                                        .clicked()
                                                    {
                                                        add_kind = Some(kind);
                                                    }
                                                },
                                            );
                                        });
                                        ui.add_space(2.0);
                                        for item in
                                            app.ws.project.items.iter().filter(|i| i.kind == kind)
                                        {
                                            let selected = app.selected_item.as_deref()
                                                == Some(item.id.as_str());
                                            if item_row(ui, item, selected) {
                                                select = Some(item.id.clone());
                                            }
                                        }
                                        ui.add_space(8.0);
                                    }
                                });
                        });
                },
            );
            ui.add_space(14.0);
            ui.vertical(|ui| {
                if let Some(id) = app.selected_item.clone() {
                    detail(app, ui, &id);
                }
            });
        });
        if let Some(id) = select {
            app.selected_item = Some(id);
        }
    });

    if let Some(kind) = add_kind {
        add_item(app, kind);
    }
}

fn selected_kind(app: &PortmanteauApp) -> Option<ItemKind> {
    app.selected_item
        .as_ref()
        .and_then(|id| app.ws.project.items.iter().find(|i| &i.id == id))
        .map(|i| i.kind)
}

fn add_item(app: &mut PortmanteauApp, kind: ItemKind) {
    let mut item = DatabaseItem::new(kind, format!("New {}", kind.label().to_lowercase()));
    item.color = (app.ws.project.items.len() % FACE_COLORS.len()) as u8;
    let id = item.id.clone();
    app.ws.project.items.push(item);
    app.selected_item = Some(id);
    app.ws.dirty = true;
}

fn item_row(ui: &mut egui::Ui, item: &DatabaseItem, selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 34.0),
        Sense::click(),
    );
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), ACCENT_SOFT);
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), PANEL_2);
    }
    let color = FACE_COLORS[item.color as usize % FACE_COLORS.len()];
    ui.painter()
        .circle_filled(egui::pos2(rect.left() + 14.0, rect.center().y), 4.5, color);
    ui.painter().text(
        egui::pos2(rect.left() + 28.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &item.title,
        ui_body(),
        if selected { TEXT } else { MUTED },
    );
    resp.clicked()
}

struct ItemDraft {
    id: String,
    kind: ItemKind,
    title: String,
    notes: String,
    tags_text: String,
    related: Vec<String>,
    color: u8,
}

fn detail(app: &mut PortmanteauApp, ui: &mut egui::Ui, id: &str) {
    let Some(pos) = app.ws.project.items.iter().position(|i| i.id == id) else {
        return;
    };
    let item = app.ws.project.items[pos].clone();
    let mut rich_notes = item.notes.clone();
    let notes_images = load_rich_image_textures(app, ui.ctx(), &rich_notes);
    let styles = app.ws.project.styles.clone();
    let mut note_styles = styles.clone();
    note_styles.body.size = 11.0;
    let sheet = StyleSheet::novel_prose(&note_styles);
    if app.database_tags_id.as_deref() != Some(id) {
        app.database_tags_id = Some(id.to_string());
        app.database_tags_text = item.tags.join(", ");
    }
    let mut draft = ItemDraft {
        id: item.id.clone(),
        kind: item.kind,
        title: item.title.clone(),
        notes: rich_notes.editor_text(),
        tags_text: app.database_tags_text.clone(),
        related: item.related.clone(),
        color: item.color,
    };
    let others: Vec<(String, String)> = app
        .ws
        .project
        .items
        .iter()
        .filter(|i| i.id != id)
        .map(|i| (i.id.clone(), i.title.clone()))
        .collect();
    let appearances: Vec<String> = if item.kind == ItemKind::Character {
        let needle = item.title.trim().to_uppercase();
        if needle.is_empty() {
            Vec::new()
        } else {
            app.ws
                .project
                .manuscript_documents()
                .iter()
                .filter(|d| d.body_text().to_uppercase().contains(&needle))
                .map(|d| d.title.clone())
                .take(8)
                .collect()
        }
    } else {
        Vec::new()
    };

    let mut apply = false;
    let mut delete = false;
    let mut select: Option<String> = None;
    let mut link_add: Option<String> = None;
    let mut link_remove: Option<String> = None;
    let color = FACE_COLORS[item.color as usize % FACE_COLORS.len()];

    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ScrollArea::vertical()
                .id_salt("database_detail")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (avatar, _) =
                            ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                        ui.painter().circle_filled(
                            avatar.center(),
                            24.0,
                            color.gamma_multiply(0.18),
                        );
                        ui.painter().text(
                            avatar.center(),
                            egui::Align2::CENTER_CENTER,
                            initials(&draft.title),
                            serif(16.0),
                            color,
                        );
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            if ui
                                .add(
                                    TextEdit::singleline(&mut draft.title)
                                        .font(serif(23.0))
                                        .hint_text("Title")
                                        .frame(Frame::NONE)
                                        .desired_width(340.0),
                                )
                                .changed()
                            {
                                apply = true;
                            }
                            ComboBox::from_id_salt(("item_kind", draft.id.clone()))
                                .selected_text(draft.kind.label())
                                .width(140.0)
                                .show_ui(ui, |ui| {
                                    for kind in ItemKind::all() {
                                        if ui
                                            .selectable_label(
                                                draft.kind == kind,
                                                kind.label(),
                                            )
                                            .clicked()
                                        {
                                            draft.kind = kind;
                                            apply = true;
                                        }
                                    }
                                });
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Delete").font(ui_small()).color(RED),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                delete = true;
                            }
                        });
                    });
                    ui.add_space(10.0);
                    hairline(ui);
                    ui.add_space(10.0);

                    section_label(ui, "Colour");
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        for (i, swatch) in FACE_COLORS.iter().enumerate() {
                            let (rect, resp) =
                                ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                            if draft.color as usize % FACE_COLORS.len() == i {
                                ui.painter().circle_stroke(
                                    rect.center(),
                                    8.5,
                                    Stroke::new(1.5, TEXT),
                                );
                            }
                            ui.painter().circle_filled(rect.center(), 6.5, *swatch);
                            if resp.clicked() {
                                draft.color = i as u8;
                                apply = true;
                            }
                        }
                    });
                    ui.add_space(10.0);

                    section_label(ui, "Tags");
                    ui.add_space(2.0);
                    if ui
                        .add(
                            TextEdit::singleline(&mut draft.tags_text)
                                .hint_text("Comma-separated, e.g. harbour, family")
                                .font(ui_body())
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        apply = true;
                    }
                    ui.add_space(10.0);

                    section_label(ui, "Notes");
                    ui.add_space(2.0);
                    let notes_id = egui::Id::unique(("database_notes", id));
                    let ctx = ui.ctx().clone();
                    let toolbar = inline_format_toolbar(ui, &ctx, notes_id, &mut rich_notes);
                    if toolbar.changed {
                        apply = true;
                    }
                    if let Some(range) = toolbar.insert_image {
                        app.image_insert = Some(ImageInsertState {
                            target: RichTextTarget::DatabaseNotes(id.to_owned()),
                            range,
                            path: String::new(),
                            alt_text: String::new(),
                            caption: String::new(),
                            standalone: false,
                            error: String::new(),
                        });
                    }
                    let rich_notes_layout = rich_notes.clone();
                    let styles_layout = note_styles.clone();
                    let sheet_layout = sheet.clone();
                    let mut layouter = |ui: &egui::Ui,
                                        text: &dyn egui::TextBuffer,
                                        wrap_width: f32| {
                        let job = script_flow_layout_job(
                            text.as_str(),
                            &rich_notes_layout,
                            &sheet_layout,
                            &styles_layout,
                            1.0,
                            wrap_width,
                            &[],
                        );
                        ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
                    };
                    let notes_output = TextEdit::multiline(&mut draft.notes)
                        .id(notes_id)
                        .layouter(&mut layouter)
                        .hint_text("What matters about this item?")
                        .font(ui_body())
                        .desired_rows(8)
                        .desired_width(f32::INFINITY)
                        .show(ui);
                    if notes_output.response.changed() {
                        rich_notes.sync_editor_text(&draft.notes, &sheet);
                        apply = true;
                    }
                    crate::ui::editor::remember_selection_anchor(ui.ctx(), notes_output.response.id, &notes_output.galley, notes_output.galley_pos);
                    paint_rich_images(
                        ui,
                        &notes_output.galley,
                        notes_output.galley_pos,
                        notes_output.response.rect,
                        &rich_notes,
                        &notes_images,
                        1.0,
                    );
                    ui.add_space(14.0);
                    hairline(ui);
                    ui.add_space(10.0);

                    section_label(ui, "Related");
                    ui.add_space(4.0);
                    if draft.related.is_empty() {
                        ui.label(
                            RichText::new("No links yet.")
                                .font(ui_small())
                                .color(FAINT),
                        );
                    }
                    ui.horizontal_wrapped(|ui| {
                        for rid in draft.related.iter() {
                            let title = others
                                .iter()
                                .find(|(oid, _)| oid == rid)
                                .map(|(_, t)| t.clone())
                                .unwrap_or_else(|| "Missing item".into());
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(&title).font(ui_body()).color(TEXT),
                                    )
                                    .fill(PANEL_2)
                                    .stroke(Stroke::new(1.0, BORDER_SOFT))
                                    .corner_radius(CornerRadius::same(20))
                                    .min_size(Vec2::new(0.0, 24.0)),
                                )
                                .on_hover_text("Open this item")
                                .clicked()
                            {
                                select = Some(rid.clone());
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("×").font(ui_small()).color(FAINT),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text("Remove link")
                                .clicked()
                            {
                                link_remove = Some(rid.clone());
                            }
                        }
                    });
                    ComboBox::from_id_salt(("related_add", draft.id.clone()))
                        .selected_text("Link an item…")
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            let candidates: Vec<(String, String)> = others
                                .iter()
                                .filter(|(oid, _)| !draft.related.contains(oid))
                                .cloned()
                                .collect();
                            if candidates.is_empty() {
                                ui.label(
                                    RichText::new("Everything is already linked.")
                                        .font(ui_small())
                                        .color(FAINT),
                                );
                            }
                            for (oid, title) in candidates {
                                if ui.selectable_label(false, title).clicked() {
                                    link_add = Some(oid);
                                }
                            }
                        });

                    if item.kind == ItemKind::Character {
                        ui.add_space(14.0);
                        hairline(ui);
                        ui.add_space(10.0);
                        section_label(ui, "Appearances");
                        ui.add_space(4.0);
                        if appearances.is_empty() {
                            ui.label(
                                RichText::new("Not yet named in the manuscript.")
                                    .font(ui_small())
                                    .color(FAINT),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "Named in {} document{}",
                                    appearances.len(),
                                    if appearances.len() == 1 { "" } else { "s" }
                                ))
                                .font(ui_small())
                                .color(MUTED),
                            );
                            for title in &appearances {
                                ui.horizontal(|ui| {
                                    dot(ui, color);
                                    ui.label(
                                        RichText::new(title).font(ui_small()).color(MUTED),
                                    );
                                });
                            }
                        }
                    }
                });
        });

    if delete {
        app.ws.project.items.remove(pos);
        app.selected_item = app.ws.project.items.first().map(|i| i.id.clone());
        app.ws.dirty = true;
        return;
    }

    if let Some(rid) = link_add {
        if !draft.related.contains(&rid) {
            draft.related.push(rid);
        }
        apply = true;
    }
    if let Some(rid) = link_remove {
        draft.related.retain(|x| x != &rid);
        apply = true;
    }
    if apply {
        let tags: Vec<String> = draft
            .tags_text
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(String::from)
            .collect();
        app.database_tags_id = Some(id.to_string());
        app.database_tags_text = draft.tags_text.clone();
        let slot = &mut app.ws.project.items[pos];
        slot.kind = draft.kind;
        slot.title = draft.title;
        slot.notes = rich_notes;
        slot.tags = tags;
        slot.related = draft.related;
        slot.color = draft.color;
        app.ws.dirty = true;
    }
    if let Some(new_id) = select {
        app.selected_item = Some(new_id);
    }
}

fn counts_line(items: &[DatabaseItem]) -> String {
    let mut parts = Vec::new();
    for kind in ItemKind::all() {
        let count = items.iter().filter(|i| i.kind == kind).count();
        parts.push(format!("{} {}", count, plural(kind)));
    }
    parts.join(" · ")
}

fn plural(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Character => "characters",
        ItemKind::Place => "places",
        ItemKind::Event => "events",
        ItemKind::Prop => "props",
        ItemKind::Custom => "custom",
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}
