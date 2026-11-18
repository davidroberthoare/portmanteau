//! Corkboard: the manuscript as editable index cards, grouped by part.
//!
//! Title and synopsis are edited in place. A single click selects a card, a
//! double-click (or the small Write button) opens it in the editor, and cards
//! can be dragged onto one another to reorder them within or across parts.

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke,
    TextEdit, UiBuilder, Vec2,
};

use crate::app::PortmanteauApp;
use crate::theme::*;
use crate::ui::views::{content_frame, meta_line, page_head, primary_button};

struct Card {
    id: String,
    title: String,
    synopsis: String,
    words: u32,
    status: String,
    label: String,
    kind: String,
    group: String,
}

/// A pending title/synopsis change, applied after the draw pass.
struct CardEdit {
    id: String,
    title: Option<String>,
    synopsis: Option<String>,
}

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut new_card = false;
    page_head(
        ui,
        "Structure",
        "Corkboard",
        "Every chapter and scene as a card. Edit titles and synopses in place, then drag cards to reorder the story.",
        |ui| {
            if primary_button(ui, "+ New card").clicked() {
                new_card = true;
            }
            ui.add_space(10.0);
            zoom_control(app, ui);
            ui.add_space(8.0);
            meta_line(ui, &[&format!("{} cards", card_count(app))]);
        },
    );

    let zoom = app.corkboard_zoom.clamp(0.6, 1.6);
    let mut cards = collect(app);
    let groups = group_ranges(&cards);
    let selected = app.selected.clone();
    let mut edits: Vec<CardEdit> = Vec::new();
    let mut select: Option<String> = None;
    let mut open: Option<String> = None;
    let mut drop_after: Option<(String, String)> = None;

    content_frame().show(ui, |ui| {
        ScrollArea::vertical()
            .id_salt("corkboard_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if cards.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("The board is empty")
                                .font(serif(20.0))
                                .color(MUTED),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Create a chapter to see it pinned here.")
                                .font(ui_body())
                                .color(FAINT),
                        );
                    });
                    return;
                }

                let avail = ui.available_width();
                let gap = 14.0;
                // Zoom scales the target card width, which in turn decides how
                // many columns fit. 230pt at 1.0 keeps cards compact: they only
                // carry a title, a short synopsis and a footer.
                let target_w = (230.0 * zoom).max(120.0);
                let cols = (((avail + gap) / (target_w + gap)).floor() as usize).clamp(1, 8);
                let card_w = (avail - gap * (cols.saturating_sub(1)) as f32) / cols as f32;

                for (index, (group, start, end)) in groups.iter().enumerate() {
                    ui.add_space(if index == 0 { 2.0 } else { 16.0 });
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(group.to_uppercase())
                                .font(ui_small())
                                .color(FAINT)
                                .extra_letter_spacing(1.2),
                        );
                        let (line, _) = ui.allocate_exact_size(
                            Vec2::new((ui.available_width() - 4.0).max(0.0), 1.0),
                            Sense::hover(),
                        );
                        ui.painter().line_segment(
                            [line.left_center(), line.right_center()],
                            Stroke::new(1.0, BORDER_SOFT),
                        );
                    });
                    ui.add_space(7.0);

                    for row_start in (*start..*end).step_by(cols) {
                        let row_end = (row_start + cols).min(*end);
                        ui.horizontal_top(|ui| {
                            for i in row_start..row_end {
                                card_ui(
                                    ui,
                                    &mut cards[i],
                                    card_w,
                                    zoom,
                                    selected.as_deref(),
                                    &mut edits,
                                    &mut select,
                                    &mut open,
                                    &mut drop_after,
                                );
                                ui.add_space(gap);
                            }
                        });
                        ui.add_space(gap);
                    }
                }
            });
    });

    // Apply edits only after every card has finished drawing.
    let synopsis_sheet =
        portmanteau_core::StyleSheet::novel_prose(&app.ws.project.styles);
    for edit in edits {
        let mut changed = false;
        {
            if let Some(doc) = app.ws.project.document_mut(&edit.id) {
                if let Some(title) = edit.title {
                    if doc.title != title {
                        doc.title = title;
                        changed = true;
                    }
                }
                if let Some(synopsis) = edit.synopsis {
                    if doc.synopsis.plain_text() != synopsis {
                        doc.synopsis.sync_editor_text(&synopsis, &synopsis_sheet);
                        changed = true;
                    }
                }
                if changed {
                    doc.touch();
                }
            }
        }
        if changed {
            app.ws.project.touch();
            app.ws.dirty = true;
        }
    }

    // Reorder once the pointer is released, so no card is mutated mid-draw.
    if let Some((dragged, target)) = drop_after {
        if app.ws.project.move_after(&dragged, &target) {
            app.ws.dirty = true;
        }
    }

    if let Some(id) = select {
        app.selected = Some(id);
    }
    if let Some(id) = open {
        app.open_doc(id);
    }

    if new_card {
        app.new_chapter_public();
        // Stay on the corkboard. The new card is drawn next frame; ask that
        // frame to put the caret in its title so the writer can type at once.
        if let Some(id) = app.selected.clone() {
            ui.ctx().data_mut(|d| d.insert_temp(pending_focus_id(), id));
        }
    }
}

fn zoom_control(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let zoom = app.corkboard_zoom.clamp(0.6, 1.6);
    let mut next = zoom;
    ui.horizontal(|ui| {
        if zoom_button(ui, "+").on_hover_text("Zoom in").clicked() {
            next = (zoom + 0.1).min(1.6);
        }
        ui.add_sized(
            Vec2::new(40.0, 26.0),
            egui::Label::new(
                RichText::new(format!("{}%", (zoom * 100.0).round() as i32))
                    .font(ui_body())
                    .color(MUTED),
            )
            .selectable(false),
        )
        .on_hover_text("Card size");
        if zoom_button(ui, "-").on_hover_text("Zoom out").clicked() {
            next = (zoom - 0.1).max(0.6);
        }
    });
    app.corkboard_zoom = next;
}

fn zoom_button(ui: &mut egui::Ui, glyph: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(glyph).font(ui_strong()).color(MUTED))
            .fill(PANEL_2)
            .stroke(Stroke::new(1.0, BORDER_SOFT))
            .corner_radius(CornerRadius::same(5))
            .min_size(Vec2::new(24.0, 26.0)),
    )
}

fn card_count(app: &PortmanteauApp) -> usize {
    app.ws
        .project
        .documents
        .iter()
        .filter(|d| {
            matches!(
                d.kind,
                portmanteau_core::DocKind::Chapter | portmanteau_core::DocKind::Scene
            )
        })
        .count()
}

fn collect(app: &PortmanteauApp) -> Vec<Card> {
    app.ws
        .project
        .manuscript_documents()
        .into_iter()
        .filter(|d| {
            matches!(
                d.kind,
                portmanteau_core::DocKind::Chapter
                    | portmanteau_core::DocKind::Scene
                    | portmanteau_core::DocKind::Text
            )
        })
        .map(|d| {
            let group = d
                .parent
                .as_ref()
                .and_then(|p| app.ws.project.document(p))
                .map(|p| p.title.clone())
                .unwrap_or_else(|| "Draft".into());
            Card {
                id: d.id.clone(),
                title: d.title.clone(),
                synopsis: d.synopsis.plain_text(),
                words: d.words(),
                status: d.status.clone(),
                label: d.label.clone(),
                kind: if d.kind == portmanteau_core::DocKind::Scene {
                    match d.medium {
                        portmanteau_core::Medium::Screenplay => "Scene · screenplay".into(),
                        portmanteau_core::Medium::StagePlay => "Scene · stage".into(),
                        portmanteau_core::Medium::TvScript => "Scene · TV".into(),
                        _ => "Scene".into(),
                    }
                } else {
                    d.kind.label().to_string()
                },
                group,
            }
        })
        .collect()
}

/// Contiguous runs of cards sharing a parent, as (group title, start, end).
fn group_ranges(cards: &[Card]) -> Vec<(String, usize, usize)> {
    let mut groups = Vec::new();
    let mut start = 0;
    while start < cards.len() {
        let group = cards[start].group.clone();
        let mut end = start;
        while end < cards.len() && cards[end].group == group {
            end += 1;
        }
        groups.push((group, start, end));
        start = end;
    }
    groups
}

fn pending_focus_id() -> egui::Id {
    egui::Id::unique("corkboard_pending_focus")
}

fn title_edit_id(card_id: &str) -> egui::Id {
    egui::Id::unique(("corkboard_title", card_id))
}

fn synopsis_edit_id(card_id: &str) -> egui::Id {
    egui::Id::unique(("corkboard_synopsis", card_id))
}

fn record_edit(
    edits: &mut Vec<CardEdit>,
    id: &str,
    title: Option<String>,
    synopsis: Option<String>,
) {
    if let Some(entry) = edits.iter_mut().find(|e| e.id == id) {
        if title.is_some() {
            entry.title = title;
        }
        if synopsis.is_some() {
            entry.synopsis = synopsis;
        }
    } else {
        edits.push(CardEdit {
            id: id.to_string(),
            title,
            synopsis,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn card_ui(
    ui: &mut egui::Ui,
    card: &mut Card,
    width: f32,
    zoom: f32,
    selected: Option<&str>,
    edits: &mut Vec<CardEdit>,
    select: &mut Option<String>,
    open: &mut Option<String>,
    drop_after: &mut Option<(String, String)>,
) {
    let height = (200.0 * zoom).clamp(180.0, 250.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let resp = ui.interact(
        rect,
        egui::Id::unique(("corkboard_card", card.id.as_str())),
        Sense::click_and_drag(),
    );
    let is_selected = selected == Some(card.id.as_str());
    let accent = label_color(&card.label);
    let pointer_over = ui.rect_contains_pointer(rect);
    let is_drop_target = resp
        .dnd_hover_payload::<String>()
        .is_some_and(|payload| payload.as_str() != card.id.as_str());
    // The payload survives the whole drag gesture, so the card that started
    // the drag can be dimmed even while the pointer is over another card.
    let dragged_id = egui::DragAndDrop::payload::<String>(&ui.ctx());
    let is_drag_source = resp.dragged()
        || dragged_id
            .as_ref()
            .is_some_and(|payload| payload.as_str() == card.id.as_str());

    let fill = if is_selected || pointer_over || is_drop_target {
        PANEL_3
    } else {
        PANEL_2
    };
    let (stroke_width, stroke_color) = if is_drop_target {
        (3.0, ACCENT)
    } else if is_selected {
        (1.4, ACCENT)
    } else if pointer_over {
        (1.0, BORDER)
    } else {
        (1.0, BORDER_SOFT)
    };

    // Frame supplies the rounded panel; interactions were registered first so
    // the text fields inside keep their own clicks and drags.
    let mut card_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    if is_drag_source {
        card_ui.multiply_opacity(0.4);
    }
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(stroke_width, stroke_color))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(13))
        .show(&mut card_ui, |ui| {
            ui.set_width((rect.width() - 26.0).max(0.0));
            ui.set_min_height((rect.height() - 26.0).max(0.0));

            ui.horizontal(|ui| {
                // Painted grab handle: three short lines, not a font glyph, so
                // it never depends on a symbol being present in the bundle.
                let (handle_rect, handle_resp) =
                    ui.allocate_exact_size(Vec2::new(14.0, 12.0), Sense::hover());
                let handle_color = if is_selected { accent } else { FAINT };
                for i in 0..3 {
                    let y = handle_rect.top() + 2.0 + i as f32 * 3.5;
                    ui.painter().line_segment(
                        [
                            egui::pos2(handle_rect.left(), y),
                            egui::pos2(handle_rect.right(), y),
                        ],
                        Stroke::new(1.4, handle_color),
                    );
                }
                handle_resp.on_hover_text("Drag to reorder");
                ui.add_space(3.0);
                ui.label(
                    RichText::new(card.kind.to_uppercase())
                        .font(ui_small())
                        .color(accent),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Write").font(ui_small()).color(FAINT))
                                .frame(false),
                        )
                        .on_hover_text("Open this card in the Write editor")
                        .clicked()
                    {
                        *open = Some(card.id.clone());
                    }
                });
            });
            ui.add_space(3.0);

            let title_id = title_edit_id(&card.id);
            let want_focus = ui
                .ctx()
                .data(|d| d.get_temp::<String>(pending_focus_id()))
                .as_deref()
                == Some(card.id.as_str());
            if want_focus {
                ui.memory_mut(|m| m.request_focus(title_id));
                ui.ctx()
                    .data_mut(|d| d.remove_temp::<String>(pending_focus_id()));
            }

            let title_resp = ui.add(
                TextEdit::singleline(&mut card.title)
                    .id(title_id)
                    .font(serif(15.0))
                    .text_color(TEXT)
                    .frame(Frame::NONE)
                    .hint_text("Untitled")
                    .desired_width(f32::INFINITY),
            );
            if title_resp.changed() {
                record_edit(edits, &card.id, Some(card.title.clone()), None);
            }
            if title_resp.clicked() || title_resp.gained_focus() {
                *select = Some(card.id.clone());
            }
            ui.add_space(2.0);

            let synopsis_resp = ui.add_sized(
                Vec2::new(ui.available_width(), 58.0),
                TextEdit::multiline(&mut card.synopsis)
                    .id(synopsis_edit_id(&card.id))
                    .font(ui_small())
                    .text_color(MUTED)
                    .frame(Frame::NONE)
                    .hint_text("Add a synopsis…")
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            if synopsis_resp.changed() {
                record_edit(edits, &card.id, None, Some(card.synopsis.clone()));
            }
            if synopsis_resp.clicked() {
                *select = Some(card.id.clone());
            }
        });

    // Accent strip and footer sit on top of the frame, matching the old cards.
    // A source card being dragged fades with its inner content.
    let mut overlay = ui.painter().clone();
    if is_drag_source {
        overlay.set_opacity(0.35);
    }
    overlay.rect_filled(
        egui::Rect::from_min_size(rect.min + Vec2::new(1.0, 1.0), Vec2::new(width - 2.0, 4.0)),
        CornerRadius {
            nw: 7,
            ne: 7,
            sw: 0,
            se: 0,
        },
        accent,
    );
    overlay.line_segment(
        [
            egui::pos2(rect.left() + 13.0, rect.bottom() - 34.0),
            egui::pos2(rect.right() - 13.0, rect.bottom() - 34.0),
        ],
        Stroke::new(1.0, BORDER_SOFT),
    );
    overlay.text(
        rect.left_bottom() + Vec2::new(13.0, -19.0),
        egui::Align2::LEFT_CENTER,
        format!("{} words", card.words),
        ui_small(),
        FAINT,
    );
    overlay.circle_filled(
        egui::pos2(rect.right() - 62.0, rect.bottom() - 19.0),
        2.6,
        status_color(&card.status),
    );
    overlay.text(
        rect.right_bottom() + Vec2::new(-13.0, -19.0),
        egui::Align2::RIGHT_CENTER,
        &card.status,
        ui_small(),
        status_color(&card.status),
    );

    // Drop cue: the target keeps a thick accent outline and gains a vertical
    // insertion bar at the edge it will land on. `move_after` always inserts
    // after the target, so the bar is always the right edge.
    if is_drop_target {
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 3.5, rect.top() + 14.0),
            Vec2::new(3.5, (rect.height() - 28.0).max(20.0)),
        );
        overlay.rect_filled(bar, CornerRadius::same(2), ACCENT);
    }

    // Drag-and-drop: this card is both a source and a drop target.
    resp.dnd_set_drag_payload(card.id.clone());
    if let Some(payload) = resp.dnd_release_payload::<String>() {
        if payload.as_str() != card.id.as_str() {
            *drop_after = Some((payload.as_str().to_string(), card.id.clone()));
        }
    }

    if resp.double_clicked() {
        *open = Some(card.id.clone());
    } else if resp.clicked() {
        *select = Some(card.id.clone());
    }

    if pointer_over {
        resp.clone().on_hover_text(
            "Click to select · Double-click or Write to open · Drag onto another card to reorder",
        );
    }
}
