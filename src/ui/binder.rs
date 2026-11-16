//! The binder: a live tree of the project with word counts and quick actions.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke,
    Vec2,
};
use portmanteau_core::{DocKind, Medium, ProjectKind};

use crate::app::{PortmanteauApp, View};
use crate::theme::*;

struct Row {
    id: String,
    title: String,
    kind: DocKind,
    medium: Medium,
    depth: usize,
    words: u32,
    has_children: bool,
    include: bool,
}

enum Action {
    Select(String),
    Toggle(String),
    NewUnder(Option<String>),
    NewFolder(Option<String>),
    Snapshot(String),
    ToggleInclude(String),
    Move(String, i32),
    Delete(String),
    View(View),
}

fn collect_rows(
    app: &PortmanteauApp,
    parent: Option<&str>,
    depth: usize,
    out: &mut Vec<Row>,
) {
    for doc in app.ws.project.children(parent) {
        let children = !app.ws.project.children(Some(&doc.id)).is_empty();
        out.push(Row {
            id: doc.id.clone(),
            title: doc.title.clone(),
            kind: doc.kind,
            medium: doc.medium,
            depth,
            words: doc.words(),
            has_children: children,
            include: doc.include_in_export,
        });
        if children && !app.collapsed.contains(&doc.id) {
            collect_rows(app, Some(&doc.id), depth + 1, out);
        }
    }
}

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut action: Option<Action> = None;

    ui.horizontal(|ui| {
        section_label(ui, "Binder");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let add = egui::Button::new(RichText::new("+").font(sans(15.0)).color(TEXT))
                .fill(PANEL_2)
                .corner_radius(CornerRadius::same(5))
                .min_size(Vec2::new(24.0, 22.0));
            if ui.add(add).on_hover_text("New chapter or scene").clicked() {
                action = Some(Action::NewUnder(None));
            }
            let add_folder = egui::Button::new(RichText::new("▣ +").font(sans(12.0)).color(TEXT))
                .fill(PANEL_2)
                .corner_radius(CornerRadius::same(5))
                .min_size(Vec2::new(32.0, 22.0));
            if ui.add(add_folder).on_hover_text("New folder").clicked() {
                action = Some(Action::NewFolder(None));
            }
            let has_selection = app.selected.is_some();
            for (glyph, hint, direction) in [
                ("↓", "Move selected down", 1),
                ("↑", "Move selected up", -1),
            ] {
                let button = egui::Button::new(RichText::new(glyph).size(12.0).color(FAINT))
                    .frame(false);
                if ui
                    .add_enabled(has_selection, button)
                    .on_hover_text(hint)
                    .on_disabled_hover_text("Select a document first")
                    .clicked()
                {
                    if let Some(id) = app.selected.clone() {
                        action = Some(Action::Move(id, direction));
                    }
                }
            }
        });
    });
    ui.add_space(4.0);

    Frame::new()
        .fill(PANEL_2)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (cover, _) = ui.allocate_exact_size(Vec2::new(30.0, 38.0), Sense::hover());
                ui.painter().rect_filled(cover, CornerRadius::same(3), Color32::from_rgb(71, 59, 43));
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(cover.min + Vec2::new(4.0, 4.0), Vec2::new(3.0, 30.0)),
                    CornerRadius::same(1),
                    ACCENT.gamma_multiply(0.6),
                );
                ui.painter().text(
                    cover.center() + Vec2::new(5.0, 0.0),
                    egui::Align2::CENTER_CENTER,
                    "S",
                    serif(13.0),
                    ACCENT,
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&app.ws.project.title)
                            .font(serif(15.0))
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} · {} words",
                            app.ws.project.kind.label(),
                            app.ws.project.words()
                        ))
                        .font(sans(9.5))
                        .color(FAINT),
                    );
                });
            });
        });
    ui.add_space(10.0);

    let target_h = 138.0;
    let scroll_h = (ui.available_height() - target_h).max(100.0);
    ScrollArea::vertical()
        .id_salt("binder_scroll")
        .auto_shrink([false, false])
        .max_height(scroll_h)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !app.search.trim().is_empty() {
                let hits: Vec<(String, String, String)> = app
                    .ws
                    .project
                    .search(&app.search)
                    .into_iter()
                    .map(|(d, excerpt)| (d.id.clone(), d.title.clone(), excerpt))
                    .collect();
                section_label(ui, &format!("Search · {} results", hits.len()));
                ui.add_space(4.0);
                if hits.is_empty() {
                    ui.label(
                        RichText::new("No matches in this project.")
                            .font(sans(11.0))
                            .color(FAINT),
                    );
                }
                for (id, title, excerpt) in hits {
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, CornerRadius::same(5), PANEL_2);
                    }
                    ui.painter().text(
                        rect.left_top() + Vec2::new(6.0, 6.0),
                        egui::Align2::LEFT_TOP,
                        &title,
                        sans(11.0),
                        TEXT,
                    );
                    let snippet: String = excerpt.chars().take(52).collect();
                    ui.painter().text(
                        rect.left_top() + Vec2::new(6.0, 22.0),
                        egui::Align2::LEFT_TOP,
                        snippet,
                        sans(9.5),
                        FAINT,
                    );
                    if resp.clicked() {
                        action = Some(Action::Select(id));
                    }
                }
                ui.add_space(8.0);
                return;
            }

            let mut rows = Vec::new();
            collect_rows(app, None, 0, &mut rows);
            for row in rows {
                let selected = app.selected.as_deref() == Some(row.id.as_str());
                let indent = 4.0 + row.depth as f32 * 13.0;
                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 25.0),
                    Sense::click(),
                );
                if selected {
                    ui.painter().rect_filled(rect, CornerRadius::same(5), ACCENT_SOFT);
                    let bar = egui::Rect::from_min_size(
                        egui::pos2(rect.left(), rect.top() + 4.0),
                        Vec2::new(2.0, 17.0),
                    );
                    ui.painter().rect_filled(bar, CornerRadius::same(1), ACCENT);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rect, CornerRadius::same(5), PANEL_2);
                }

                let chevron = row.has_children;
                let mut x = rect.left() + indent;
                if chevron {
                    let open = !app.collapsed.contains(&row.id);
                    let c = egui::pos2(x + 5.0, rect.center().y);
                    let pts = if open {
                        vec![
                            egui::pos2(c.x - 4.0, c.y - 2.0),
                            egui::pos2(c.x + 4.0, c.y - 2.0),
                            egui::pos2(c.x, c.y + 3.0),
                        ]
                    } else {
                        vec![
                            egui::pos2(c.x - 2.0, c.y - 4.0),
                            egui::pos2(c.x + 3.0, c.y),
                            egui::pos2(c.x - 2.0, c.y + 4.0),
                        ]
                    };
                    let chev_rect = egui::Rect::from_center_size(c, Vec2::splat(14.0));
                    let chev_resp = ui.interact(chev_rect, egui::Id::unique(("chev", &row.id)), Sense::click());
                    let color = if chev_resp.hovered() { TEXT } else { FAINT };
                    ui.painter().add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
                    if chev_resp.clicked() {
                        action = Some(Action::Toggle(row.id.clone()));
                    }
                    x += 15.0;
                } else {
                    x += 15.0;
                }

                let (glyph, glyph_color) = match row.kind {
                    DocKind::Folder => ("▣", MUTED),
                    DocKind::Chapter => ("¶", ACCENT),
                    DocKind::Scene if row.medium.is_script() => ("▶", PURPLE),
                    DocKind::Scene => ("◆", SAGE),
                    DocKind::Research => ("✎", BLUE),
                    DocKind::Character => ("☺", PURPLE),
                    DocKind::FrontMatter => ("▤", FAINT),
                    DocKind::BackMatter => ("▤", FAINT),
                    DocKind::Text => ("◆", FAINT),
                };
                ui.painter().text(
                    egui::pos2(x + 5.0, rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    sans(10.5),
                    glyph_color,
                );
                x += 17.0;

                let right_space = 54.0;
                let title_rect = egui::Rect::from_min_max(
                    egui::pos2(x, rect.top()),
                    egui::pos2(rect.right() - right_space, rect.bottom()),
                );
                let painter = ui.painter().with_clip_rect(title_rect);
                let title_color = if row.include { TEXT } else { FAINT };
                let mut title = row.title.clone();
                if !row.include {
                    title.push_str("  (excluded)");
                }
                // The painter clips without an ellipsis, which hid the end of a
                // long chapter name. Truncate to the available width so a writer
                // can see that the row continues.
                let max_chars = ((title_rect.width() / 5.9).floor() as usize).max(4);
                if title.chars().count() > max_chars {
                    let mut cut: String = title.chars().take(max_chars.saturating_sub(1)).collect();
                    cut.push('…');
                    title = cut;
                }
                painter.text(
                    title_rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    title,
                    if selected { sans(11.5) } else { sans(11.0) },
                    title_color,
                );

                if row.words > 0 && !row.kind.is_folder() {
                    ui.painter().text(
                        egui::pos2(rect.right() - 6.0, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        row.words.to_string(),
                        sans(9.5),
                        FAINT,
                    );
                }

                if resp.clicked() {
                    action = Some(Action::Select(row.id.clone()));
                }
                resp.context_menu(|ui| {
                    if ui.button("Open").clicked() {
                        action = Some(Action::Select(row.id.clone()));
                        ui.close();
                    }
                    if ui.button("New inside").clicked() {
                        action = Some(Action::NewUnder(Some(row.id.clone())));
                        ui.close();
                    }
                    if ui.button("New folder").clicked() {
                        action = Some(Action::NewFolder(Some(row.id.clone())));
                        ui.close();
                    }
                    if ui.button("Move up").clicked() {
                        action = Some(Action::Move(row.id.clone(), -1));
                        ui.close();
                    }
                    if ui.button("Move down").clicked() {
                        action = Some(Action::Move(row.id.clone(), 1));
                        ui.close();
                    }
                    if !row.kind.is_folder() && ui.button("Take snapshot").clicked() {
                        action = Some(Action::Snapshot(row.id.clone()));
                        ui.close();
                    }
                    if ui
                        .button(if row.include {
                            "Exclude from compile"
                        } else {
                            "Include in compile"
                        })
                        .clicked()
                    {
                        action = Some(Action::ToggleInclude(row.id.clone()));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() {
                        action = Some(Action::Delete(row.id.clone()));
                        ui.close();
                    }
                });
            }

            ui.add_space(10.0);
            section_label(ui, "Collections");
            ui.add_space(3.0);
            let scenes = app
                .ws
                .project
                .documents
                .iter()
                .filter(|d| d.kind == DocKind::Scene)
                .count();
            let research = app
                .ws
                .project
                .documents
                .iter()
                .filter(|d| d.kind == DocKind::Research)
                .count();
            for (label, count, target_view) in [
                ("Corkboard", scenes, View::Corkboard),
                ("Research notes", research, View::Research),
            ] {
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rect, CornerRadius::same(5), PANEL_2);
                }
                ui.painter().text(
                    rect.left_center() + Vec2::new(2.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    label,
                    sans(10.5),
                    MUTED,
                );
                ui.painter().text(
                    rect.right_center() - Vec2::new(4.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    count.to_string(),
                    sans(9.5),
                    FAINT,
                );
                if resp.clicked() {
                    action = Some(Action::View(target_view));
                }
            }
        });

    ui.add_space(6.0);
    target_card(app, ui);

    if let Some(action) = action {
        match action {
            Action::Select(id) => {
                app.selected = Some(id);
                app.selected_block = 0;
            }
            Action::Toggle(id) => {
                if !app.collapsed.remove(&id) {
                    app.collapsed.insert(id);
                }
            }
            Action::NewUnder(parent) => {
                let parent = parent.or_else(|| {
                    app.selected
                        .as_ref()
                        .and_then(|id| app.ws.project.document(id))
                        .and_then(|d| {
                            if d.kind == DocKind::Folder {
                                Some(d.id.clone())
                            } else {
                                d.parent.clone()
                            }
                        })
                });
                let is_script = matches!(
                    app.ws.project.kind,
                    ProjectKind::Film | ProjectKind::Television | ProjectKind::Theatre
                );
                let id = if is_script {
                    app.ws.project.add_document(
                        parent,
                        "Untitled scene",
                        DocKind::Scene,
                        Medium::Screenplay,
                    )
                } else {
                    app.ws
                        .project
                        .add_document(parent, "Untitled chapter", DocKind::Chapter, Medium::Prose)
                };
                app.ws.dirty = true;
                app.selected = Some(id);
                app.selected_block = 0;
                app.block_focus = Some(0);
            }
            Action::NewFolder(parent) => {
                let parent = parent.or_else(|| {
                    app.selected
                        .as_ref()
                        .and_then(|id| app.ws.project.document(id))
                        .and_then(|d| {
                            if d.kind == DocKind::Folder {
                                Some(d.id.clone())
                            } else {
                                d.parent.clone()
                            }
                        })
                });
                let id = app
                    .ws
                    .project
                    .add_document(parent, "New folder", DocKind::Folder, Medium::Prose);
                app.ws.dirty = true;
                app.selected = Some(id);
                app.selected_block = 0;
            }
            Action::Snapshot(id) => {
                let is_folder = app
                    .ws
                    .project
                    .document(&id)
                    .map(|d| d.kind.is_folder())
                    .unwrap_or(false);
                if is_folder {
                    return;
                }
                if let Some(doc) = app.ws.project.document_mut(&id) {
                    let label = format!("Snapshot · {}", chrono::Local::now().format("%d %b %H:%M"));
                    doc.snapshot(label.clone(), "Snapshot from the binder");
                    app.ws.dirty = true;
                    app.toast("Version saved", label, crate::app::ToastKind::Ok);
                }
            }
            Action::ToggleInclude(id) => {
                if let Some(doc) = app.ws.project.document_mut(&id) {
                    doc.include_in_export = !doc.include_in_export;
                    app.ws.dirty = true;
                }
            }
            Action::Move(id, direction) => {
                if app.ws.project.move_document(&id, direction) {
                    app.ws.dirty = true;
                }
            }
            Action::Delete(id) => {
                app.ws.project.delete_document(&id);
                app.ws.dirty = true;
                if app.selected.as_deref() == Some(id.as_str()) {
                    app.selected = app
                        .ws
                        .project
                        .documents
                        .iter()
                        .find(|d| d.kind == DocKind::Chapter)
                        .map(|d| d.id.clone());
                }
            }
            Action::View(v) => app.view = v,
        }
    }
}

fn target_card(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    Frame::new()
        .fill(PANEL_2)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, "Session target");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("✓ local").font(sans(9.5)).color(SAGE),
                    );
                });
            });
            ui.add_space(5.0);
            let today = app.ws.project.today_words();
            let target = app.ws.project.daily_target.max(1);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(today.to_string())
                        .font(serif(20.0))
                        .color(ACCENT),
                );
                ui.label(
                    RichText::new(format!("of {target} words"))
                        .font(sans(10.0))
                        .color(MUTED),
                );
            });
            progress_bar(ui, today as f32 / target as f32, SAGE, 5.0);
            ui.add_space(7.0);
            ui.horizontal(|ui| {
                let streak = streak_days(app);
                ui.label(
                    RichText::new(format!("{streak}-day streak")).font(sans(9.5)).color(MUTED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} total", app.ws.project.words()))
                            .font(sans(9.5))
                            .color(FAINT),
                    );
                });
            });
        });
}

fn streak_days(app: &PortmanteauApp) -> u32 {
    let log = &app.ws.project.session_log;
    let mut streak = 0;
    for entry in log.iter().rev() {
        if entry.words > 0 {
            streak += 1;
        } else if streak > 0 {
            break;
        }
    }
    streak
}
