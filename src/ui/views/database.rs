//! Story database: characters, places, events, props and custom records,
//! each with notes, tags, related links and a colour.

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea,
    Sense, Stroke, TextEdit, Vec2,
};

use portmanteau_core::{DatabaseItem, ItemKind, StyleSheet};

use crate::app::PortmanteauApp;
use crate::theme::*;
use crate::ui::editor::{load_rich_image_textures, paint_rich_images, script_flow_layout_job};
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
        let filter_id = ui.make_persistent_id("database-list-filter");
        let mut filter: ListFilter = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
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
                            filter_ui(ui, &app.ws.project.items, &mut filter);
                            ui.add_space(6.0);
                            ScrollArea::vertical()
                                .id_salt("database_list")
                                .auto_shrink([false, false])
                                .max_height(height - 20.0 - filter_height(&app.ws.project.items))
                                .show(ui, |ui| {
                                    for kind in ItemKind::all() {
                                        let count = app
                                            .ws
                                            .project
                                            .items
                                            .iter()
                                            .filter(|i| i.kind == kind && filter.allows(i))
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
                                        for item in app
                                            .ws
                                            .project
                                            .items
                                            .iter()
                                            .filter(|i| i.kind == kind && filter.allows(i))
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
        ui.data_mut(|d| d.insert_temp(filter_id, filter));
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
    // A row with tags is a little taller: the tags sit under the name.
    let height = if item.tags.is_empty() { 34.0 } else { 46.0 };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, true, selected, format!("{} · {}", item.title, item.kind.label()))
    });
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), ACCENT_SOFT);
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), PANEL_2);
    }
    let color = FACE_COLORS[item.color as usize % FACE_COLORS.len()];
    let top = if item.tags.is_empty() { rect.center().y } else { rect.top() + 16.0 };
    ui.painter().circle_filled(egui::pos2(rect.left() + 14.0, top), 4.5, color);
    ui.painter().text(
        egui::pos2(rect.left() + 28.0, top),
        egui::Align2::LEFT_CENTER,
        &item.title,
        ui_body(),
        if selected { TEXT } else { MUTED },
    );
    if !item.tags.is_empty() {
        let clip = ui.painter().with_clip_rect(rect.shrink2(Vec2::new(6.0, 0.0)));
        clip.text(
            egui::pos2(rect.left() + 28.0, rect.top() + 33.0),
            egui::Align2::LEFT_CENTER,
            item.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join("  "),
            ui_small(),
            FAINT,
        );
    }
    resp.clicked()
}

/// What the list shows: a search, and optionally one tag.
#[derive(Clone, Default)]
struct ListFilter {
    search: String,
    tag: Option<String>,
}

impl ListFilter {
    fn allows(&self, item: &DatabaseItem) -> bool {
        let needle = self.search.trim().to_lowercase();
        (self.tag.is_none() || item.tags.iter().any(|t| Some(t) == self.tag.as_ref()))
            && (needle.is_empty()
                || item.title.to_lowercase().contains(&needle)
                || item.tags.iter().any(|t| t.to_lowercase().contains(&needle))
                || item.notes.plain_text().to_lowercase().contains(&needle))
    }
}

/// Every tag in use with how many items carry it, most used first.
fn tag_counts(items: &[DatabaseItem]) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for tag in items.iter().flat_map(|i| i.tags.iter()) {
        match counts.iter_mut().find(|(t, _)| t == tag) {
            Some((_, n)) => *n += 1,
            None => counts.push((tag.clone(), 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    counts
}

/// Roughly how much room the search box and tag chips take, so the list scrolls inside what is left.
fn filter_height(items: &[DatabaseItem]) -> f32 {
    let tags = tag_counts(items).len();
    34.0 + if tags == 0 { 0.0 } else { 28.0 * (1.0 + (tags as f32 / 4.0).floor().min(3.0)) }
}

fn filter_ui(ui: &mut egui::Ui, items: &[DatabaseItem], filter: &mut ListFilter) {
    ui.add(
        TextEdit::singleline(&mut filter.search)
            .hint_text("Search names, tags and notes")
            .font(ui_small())
            .desired_width(f32::INFINITY),
    );
    let tags = tag_counts(items);
    if tags.is_empty() {
        return;
    }
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
        for (tag, count) in tags {
            let active = filter.tag.as_deref() == Some(tag.as_str());
            let button = egui::Button::new(RichText::new(format!("#{tag} {count}")).font(ui_small()).color(if active { TEXT } else { MUTED }))
                .fill(if active { ACCENT_SOFT } else { PANEL_2 })
                .stroke(Stroke::new(1.0, if active { ACCENT } else { BORDER_SOFT }))
                .corner_radius(CornerRadius::same(10))
                .min_size(Vec2::new(0.0, 20.0));
            if ui.add(button).on_hover_text("Show only items with this tag").clicked() {
                filter.tag = if active { None } else { Some(tag) };
            }
        }
    });
}

struct ItemDraft {
    id: String,
    kind: ItemKind,
    title: String,
    notes: String,
    tags_text: String,
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
        color: item.color,
    };
    // Where the item is named or linked in the writing (kept a moment while typing).
    let appearances = appearances_cached(ui.ctx(), &app.ws.project, &item);
    let linked_from: Vec<(String, String)> = app
        .ws
        .project
        .items_linking_to(&item)
        .into_iter()
        .map(|other| (other.id.clone(), other.title.clone()))
        .collect();
    let tag_suggestions: Vec<String> = tag_counts(&app.ws.project.items)
        .into_iter()
        .map(|(tag, _)| tag)
        .filter(|tag| !item.tags.contains(tag))
        .take(10)
        .collect();
    let mut open_appearance: Option<portmanteau_core::Appearance> = None;

    let mut apply = false;
    let mut delete = false;
    let mut select: Option<String> = None;
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
                    if !tag_suggestions.is_empty() {
                        ui.add_space(3.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                            ui.label(RichText::new("Used elsewhere:").font(ui_small()).color(FAINT));
                            for tag in &tag_suggestions {
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new(format!("+ #{tag}")).font(ui_small()).color(MUTED))
                                            .fill(PANEL_2)
                                            .stroke(Stroke::new(1.0, BORDER_SOFT))
                                            .corner_radius(CornerRadius::same(10))
                                            .min_size(Vec2::new(0.0, 20.0)),
                                    )
                                    .on_hover_text("Add this tag")
                                    .clicked()
                                {
                                    if !draft.tags_text.trim().is_empty() && !draft.tags_text.trim_end().ends_with(',') {
                                        draft.tags_text.push_str(", ");
                                    }
                                    draft.tags_text.push_str(tag);
                                    apply = true;
                                }
                            }
                        });
                    }
                    ui.add_space(10.0);

                    section_label(ui, "Notes");
                    ui.add_space(2.0);
                    let notes_id = egui::Id::unique(("database_notes", id));
                    // The fixed formatting bar, as on research notes (see ui/rich_toolbar.rs).
                    let bar = crate::ui::rich_toolbar::show(
                        ui,
                        notes_id,
                        &mut rich_notes,
                        &crate::ui::rich_toolbar::Options { styles: Some(&sheet) },
                    );
                    if bar.changed {
                        apply = true;
                    }
                    ui.add_space(6.0);
                    let checker = &app.spell;
                    let spell_on = app.settings.spellcheck;
                    let notes_spans =
                        crate::ui::editor::field_spans(checker, spell_on, &draft.notes);
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
                            field_text_zoom(),
                            wrap_width,
                            &notes_spans,
                        );
                        ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
                    };
                    let mention_pick = crate::ui::mention::pre_edit(ui.ctx(), notes_id);
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
                        let caret = notes_output.cursor_range.map(|r| r.primary.index.0);
                        crate::ui::editor::autolink_pasted(ui.ctx(), &mut rich_notes, caret);
                        crate::ui::rich_toolbar::after_edit(ui.ctx(), notes_id, &mut rich_notes, caret);
                        apply = true;
                    }
                    if crate::ui::mention::post_edit(ui.ctx(), notes_id, &notes_output, &mut rich_notes, mention_pick) {
                        draft.notes = rich_notes.editor_text();
                        apply = true;
                    }
                    crate::ui::editor::remember_selection_anchor(ui.ctx(), notes_output.response.id, &notes_output.galley, notes_output.galley_pos);
                    let fixed = crate::ui::editor::field_spell_menu(
                        ui,
                        checker,
                        spell_on,
                        egui::Id::unique(("spell_menu_database", id)),
                        &notes_output,
                        &draft.notes,
                        &mut rich_notes,
                        &sheet,
                    );
                    if fixed.replaced {
                        draft.notes = rich_notes.editor_text();
                        apply = true;
                    }
                    crate::ui::editor::apply_field_spell(app, vec![fixed]);
                    paint_rich_images(
                        ui,
                        &notes_output.galley,
                        notes_output.galley_pos,
                        notes_output.response.rect,
                        &rich_notes,
                        &notes_images,
                        field_text_zoom(),
                    );
                    ui.add_space(14.0);
                    hairline(ui);
                    ui.add_space(10.0);

                    section_label(ui, "Linked from");
                    ui.add_space(4.0);
                    if linked_from.is_empty() {
                        ui.label(
                            RichText::new("No other item links here. Type @ in another item's notes to link to this one.")
                                .font(ui_small())
                                .color(FAINT),
                        );
                    }
                    ui.horizontal_wrapped(|ui| {
                        for (oid, title) in &linked_from {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(title).font(ui_body()).color(TEXT))
                                        .fill(PANEL_2)
                                        .stroke(Stroke::new(1.0, BORDER_SOFT))
                                        .corner_radius(CornerRadius::same(20))
                                        .min_size(Vec2::new(0.0, 24.0)),
                                )
                                .on_hover_text("Open this item")
                                .clicked()
                            {
                                select = Some(oid.clone());
                            }
                        }
                    });

                    ui.add_space(14.0);
                    hairline(ui);
                    ui.add_space(10.0);
                    section_label(ui, "Appearances");
                    ui.add_space(4.0);
                    if appearances.is_empty() {
                        ui.label(
                            RichText::new("Not yet named or linked in your writing.")
                                .font(ui_small())
                                .color(FAINT),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "In {} document{} · click one to go to the first place",
                                appearances.len(),
                                if appearances.len() == 1 { "" } else { "s" }
                            ))
                            .font(ui_small())
                            .color(MUTED),
                        );
                        ui.add_space(2.0);
                        for appearance in appearances.iter().take(30) {
                            ui.horizontal(|ui| {
                                dot(ui, color);
                                let title = if appearance.by_link { format!("{}  @", appearance.title) } else { appearance.title.clone() };
                                if ui
                                    .add(egui::Button::new(RichText::new(title).font(ui_body()).color(BLUE)).frame(false))
                                    .on_hover_text(if appearance.place.is_some() { "Open at the first appearance" } else { "Open (named in its notes or synopsis)" })
                                    .clicked()
                                {
                                    open_appearance = Some(appearance.clone());
                                }
                                ui.label(RichText::new(&appearance.excerpt).font(ui_small()).color(FAINT));
                            });
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

    if let Some(appearance) = open_appearance {
        app.open_appearance(&appearance);
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
        slot.color = draft.color;
        app.ws.dirty = true;
    }
    if let Some(new_id) = select {
        app.selected_item = Some(new_id);
    }
}

/// Appearances change only when the writing does, and computing them reads every
/// document, so keep the last answer for a moment while the writer types.
fn appearances_cached(ctx: &egui::Context, project: &portmanteau_core::Project, item: &DatabaseItem) -> Vec<portmanteau_core::Appearance> {
    use std::time::{Duration, Instant};
    type Cached = (String, String, Instant, Vec<portmanteau_core::Appearance>);
    let key = egui::Id::unique("database-appearances");
    let fingerprint = format!("{}|{}|{}", project.updated, project.documents.len(), item.title);
    if let Some((id, seen, at, found)) = ctx.data(|d| d.get_temp::<Cached>(key)) {
        if id == item.id && (seen == fingerprint || at.elapsed() < Duration::from_millis(900)) {
            if seen != fingerprint {
                ctx.request_repaint_after(Duration::from_millis(950));
            }
            return found;
        }
    }
    let found = project.appearances_of(item);
    ctx.data_mut(|d| d.insert_temp(key, (item.id.clone(), fingerprint, Instant::now(), found.clone())));
    found
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

#[cfg(test)]
mod tests {
    use super::*;

    fn db_harness(app: PortmanteauApp) -> (egui_kittest::Harness<'static>, std::rc::Rc<std::cell::RefCell<PortmanteauApp>>) {
        let app = std::rc::Rc::new(std::cell::RefCell::new(app));
        let ui_app = std::rc::Rc::clone(&app);
        let mut themed = false;
        let harness = egui_kittest::Harness::builder()
            .with_size(Vec2::new(1440.0, 930.0))
            .with_accessibility_check(false)
            .allow_missing_glyphs()
            .build_ui(move |ui| {
                if !themed {
                    crate::theme::install(ui.ctx());
                    themed = true;
                    return;
                }
                crate::ui::mention::publish_targets(ui.ctx(), &ui_app.borrow().ws.project);
                show(&mut ui_app.borrow_mut(), ui);
            });
        (harness, app)
    }

    #[test]
    fn tags_filter_the_list_and_show_under_each_name() {
        use egui_kittest::kittest::Queryable;
        let (mut harness, _app) = db_harness(PortmanteauApp::new(None));
        harness.run_steps(4);
        // List rows are named "<title> · <kind>"; the open item's own heading is not a row.
        let listed = |harness: &egui_kittest::Harness<'static>, name: &str| harness.query_by_label_contains(&format!("{name} · ")).is_some();
        for name in ["Mara Venn", "Elias Rook", "The Boy", "Ruth Alder"] {
            assert!(listed(&harness, name), "{name} is listed");
        }
        // The chip for a tag used by two items; pressing it narrows the list to them.
        harness.get_by_label_contains("#harbour 2").click();
        harness.run_steps(3);
        assert!(listed(&harness, "Mara Venn"));
        assert!(listed(&harness, "Elias Rook"));
        assert!(!listed(&harness, "The Boy"), "no harbour tag");
        assert!(!listed(&harness, "Ruth Alder"));
        // Pressing it again clears the filter.
        harness.get_by_label_contains("#harbour 2").click();
        harness.run_steps(3);
        assert!(listed(&harness, "The Boy"));
    }

    #[test]
    fn tag_counts_are_most_used_first_and_the_search_reaches_names_tags_and_notes() {
        let project = portmanteau_core::sample::demo();
        let counts = tag_counts(&project.items);
        assert_eq!(counts[0].1, 2, "the most used tags come first: {counts:?}");
        assert!(counts.iter().any(|(t, n)| t == "mystery" && *n == 1));
        let filter = |search: &str, tag: Option<&str>| ListFilter { search: search.into(), tag: tag.map(String::from) };
        let mara = project.items.iter().find(|i| i.title == "Mara Venn").unwrap();
        assert!(filter("mara", None).allows(mara));
        assert!(filter("FAMILY", None).allows(mara), "by tag");
        assert!(filter("surveyor", None).allows(mara), "by notes");
        assert!(!filter("mara", Some("mystery")).allows(mara));
        assert!(!filter("zzz", None).allows(mara));
    }

    /// Clicking where a character appears opens that document with the caret
    /// at the first place the name is used.
    #[test]
    fn an_appearance_opens_the_document_at_the_first_place() {
        use egui_kittest::kittest::Queryable;
        let (mut harness, app) = db_harness(PortmanteauApp::new(None));
        harness.run_steps(4);
        let mara = app.borrow().ws.project.items.iter().find(|i| i.title == "Mara Venn").unwrap().clone();
        let first = app.borrow().ws.project.appearances_of(&mara).into_iter().next().expect("Mara appears");
        let (paragraph, offset) = first.place.expect("in the text");
        // The list is below the fold of this window, so activate it through accessibility rather than a pointer click.
        harness.get_by_label(first.title.as_str()).click_accesskit();
        harness.run_steps(3);
        let app = app.borrow();
        assert_eq!(app.selected.as_deref(), Some(first.doc_id.as_str()));
        assert_eq!(app.view, crate::app::View::Write);
        let request = app.caret_request.as_ref().unwrap_or_else(|| panic!("no caret request; clicked {:?}, appearance {first:?}", first.title));
        assert_eq!((request.doc_id.as_str(), request.paragraph, request.offset), (first.doc_id.as_str(), paragraph, offset));
    }
}
