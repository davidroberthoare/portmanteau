//! Corkboard: the manuscript as index cards, grouped by part or chapter folder.

use eframe::egui::{
    self, CornerRadius, RichText, ScrollArea, Sense,
    Stroke, Vec2,
};

use crate::app::{PortmanteauApp, View};
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

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut new_card = false;
    page_head(
        ui,
        "Structure",
        "Corkboard",
        "Every chapter and scene as a card. Read the shape of the story before you write it.",
        |ui| {
            if primary_button(ui, "+ New card").clicked() {
                new_card = true;
            }
            ui.add_space(6.0);
            meta_line(ui, &[&format!("{} cards", card_count(app))]);
        },
    );

    let cards = collect(app);
    content_frame().show(ui, |ui| {
        ScrollArea::vertical()
            .id_salt("corkboard_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if cards.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("The board is empty").font(serif(20.0)).color(MUTED));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Create a chapter to see it pinned here.")
                                .font(sans(11.0))
                                .color(FAINT),
                        );
                    });
                    return;
                }

                let mut last_group = String::new();
                let avail = ui.available_width();
                let gap = 14.0;
                let cols = ((avail / 285.0).floor() as usize).clamp(1, 4);
                let card_w = (avail - gap * (cols.saturating_sub(1)) as f32) / cols as f32;

                let mut index = 0;
                while index < cards.len() {
                    let group = &cards[index].group;
                    if *group != last_group {
                        ui.add_space(if last_group.is_empty() { 2.0 } else { 16.0 });
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(group.to_uppercase())
                                    .font(sans(9.0))
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
                        last_group = group.clone();
                    }
                    let end = cards[index..]
                        .iter()
                        .position(|c| &c.group != group)
                        .map(|n| index + n)
                        .unwrap_or(cards.len());
                    let slice = &cards[index..end];

                    for row in slice.chunks(cols) {
                        ui.horizontal_top(|ui| {
                            for card in row {
                                card_ui(app, ui, card, card_w);
                                ui.add_space(gap);
                            }
                        });
                        ui.add_space(gap);
                    }
                    index = end;
                }
            });
    });

    if new_card {
        app.new_chapter_public();
        app.view = View::Write;
    }
}

fn card_count(app: &PortmanteauApp) -> usize {
    app.ws
        .project
        .documents
        .iter()
        .filter(|d| matches!(d.kind, portmanteau_core::DocKind::Chapter | portmanteau_core::DocKind::Scene))
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
                portmanteau_core::DocKind::Chapter | portmanteau_core::DocKind::Scene | portmanteau_core::DocKind::Text
            )
        })
        .map(|d| {
            let group = d
                .parent
                .as_ref()
                .and_then(|p| app.ws.project.document(p))
                .map(|p| p.title.clone())
                .unwrap_or_else(|| "Manuscript".into());
            Card {
                id: d.id.clone(),
                title: d.title.clone(),
                synopsis: if d.synopsis.is_empty() {
                    "No synopsis yet. Describe the turn this piece makes.".into()
                } else {
                    d.synopsis.clone()
                },
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

fn card_ui(app: &mut PortmanteauApp, ui: &mut egui::Ui, card: &Card, width: f32) {
    let height = 176.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let selected = app.selected.as_deref() == Some(card.id.as_str());
    let accent = label_color(&card.label);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(8),
        if resp.hovered() { PANEL_3 } else { PANEL_2 },
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(
            if selected { 1.4 } else { 1.0 },
            if selected {
                ACCENT
            } else if resp.hovered() {
                BORDER
            } else {
                BORDER_SOFT
            },
        ),
        egui::StrokeKind::Inside,
    );
    painter.rect_filled(
        egui::Rect::from_min_size(rect.min + Vec2::new(1.0, 1.0), Vec2::new(width - 2.0, 4.0)),
        CornerRadius {
            nw: 7,
            ne: 7,
            sw: 0,
            se: 0,
        },
        accent,
    );

    ui.painter().text(
        rect.left_top() + Vec2::new(13.0, 15.0),
        egui::Align2::LEFT_TOP,
        card.kind.to_uppercase(),
        sans(8.0),
        accent,
    );
    let title = ui.painter().layout(
        card.title.clone(),
        serif(15.0),
        TEXT,
        width - 26.0,
    );
    ui.painter()
        .galley(rect.left_top() + Vec2::new(13.0, 32.0), title, TEXT);

    let synopsis: String = if card.synopsis.chars().count() > 150 {
        format!("{}…", card.synopsis.chars().take(147).collect::<String>())
    } else {
        card.synopsis.clone()
    };
    let body = ui.painter().layout(
        synopsis,
        sans(10.0),
        MUTED,
        width - 26.0,
    );
    ui.painter()
        .galley(rect.left_top() + Vec2::new(13.0, 70.0), body, MUTED);

    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 13.0, rect.bottom() - 34.0),
            egui::pos2(rect.right() - 13.0, rect.bottom() - 34.0),
        ],
        Stroke::new(1.0, BORDER_SOFT),
    );
    ui.painter().text(
        rect.left_bottom() + Vec2::new(13.0, -19.0),
        egui::Align2::LEFT_CENTER,
        format!("{} words", card.words),
        sans(9.0),
        FAINT,
    );
    let (dot_rect, _) = (
        egui::Rect::from_center_size(
            egui::pos2(rect.right() - 62.0, rect.bottom() - 19.0),
            Vec2::splat(6.0),
        ),
        (),
    );
    ui.painter()
        .circle_filled(dot_rect.center(), 2.6, status_color(&card.status));
    ui.painter().text(
        rect.right_bottom() + Vec2::new(-13.0, -19.0),
        egui::Align2::RIGHT_CENTER,
        &card.status,
        sans(9.0),
        status_color(&card.status),
    );

    if resp.clicked() {
        app.open_doc(card.id.clone());
        app.view = View::Write;
    }
    resp.on_hover_text("Open this card in the editor");
}
