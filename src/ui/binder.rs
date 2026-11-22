//! The binder: a live tree of the project with word counts and quick actions.

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Vec2,
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
    HideBinder,
}

fn collect_rows(
    app: &PortmanteauApp,
    parent: Option<&str>,
    depth: usize,
    out: &mut Vec<Row>,
) {
    for doc in app.ws.project.children(parent) {
        // Research lives on its own page (the Research notes entry under
        // Collections), so its folder is not listed in the binder.
        if parent.is_none() && doc.kind.is_folder() && doc.title == "Research" {
            continue;
        }
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
            let hide = egui::Button::new(
                RichText::new("‹").font(ui_strong()).color(FAINT),
            )
            .frame(false);
            if ui
                .add(hide)
                .on_hover_text("Hide the binder")
                .clicked()
            {
                action = Some(Action::HideBinder);
            }
            ui.add_space(2.0);
            let add = egui::Button::new(RichText::new("+").font(ui_strong()).color(TEXT))
                .fill(PANEL_2)
                .corner_radius(CornerRadius::same(5))
                .min_size(Vec2::new(24.0, 22.0));
            let add = ui.add(add).on_hover_text("Add to the binder");
            egui::Popup::menu(&add).show(|ui| {
                if ui.button("New chapter or scene").clicked() {
                    action = Some(Action::NewUnder(None));
                    ui.close();
                }
                if ui.button("New folder").clicked() {
                    action = Some(Action::NewFolder(None));
                    ui.close();
                }
                ui.separator();
                let has_selection = app.selected.is_some();
                let up = ui.add_enabled(has_selection, egui::Button::new("Move selected up"));
                if up.clicked() {
                    if let Some(id) = app.selected.clone() {
                        action = Some(Action::Move(id, -1));
                    }
                    ui.close();
                }
                let down = ui.add_enabled(has_selection, egui::Button::new("Move selected down"));
                if down.clicked() {
                    if let Some(id) = app.selected.clone() {
                        action = Some(Action::Move(id, 1));
                    }
                    ui.close();
                }
            });
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
                ui.painter().rect_filled(cover, CornerRadius::same(3), PAPER_EDGE);
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
                    ACCENT_INK,
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
                        .font(ui_small())
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
                            .font(ui_body())
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
                        ui_body(),
                        TEXT,
                    );
                    let snippet: String = excerpt.chars().take(52).collect();
                    ui.painter().text(
                        rect.left_top() + Vec2::new(6.0, 22.0),
                        egui::Align2::LEFT_TOP,
                        snippet,
                        ui_small(),
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

                paint_doc_icon(
                    ui.painter(),
                    egui::pos2(x + 6.0, rect.center().y),
                    row.kind,
                    row.medium.is_script(),
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
                    if selected { ui_body() } else { ui_body() },
                    title_color,
                );

                if row.words > 0 && !row.kind.is_folder() {
                    ui.painter().text(
                        egui::pos2(rect.right() - 6.0, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        row.words.to_string(),
                        ui_small(),
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
            let research = app.ws.project.research_docs().len();
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
                    ui_body(),
                    MUTED,
                );
                ui.painter().text(
                    rect.right_center() - Vec2::new(4.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    count.to_string(),
                    ui_small(),
                    FAINT,
                );
                if resp.clicked() {
                    action = Some(Action::View(target_view));
                }
            }
        });

    ui.add_space(6.0);
    // Removed: a "Session target" card (today's words, a streak and a total)
    // that repeated the top bar's progress meter. The streak is the only
    // thing it had that is not shown elsewhere; it belongs on the Versions
    // screen beside the fourteen-day chart.

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
                let parent = parent.or_else(|| app.ws.project.new_document_home(app.selected.as_deref()));
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
                let parent = parent.or_else(|| app.ws.project.new_document_home(app.selected.as_deref()));
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
            Action::HideBinder => app.set_binder_visible(false),
        }
    }
}

/// Binder icons: a folder for containers and a small page for everything
/// else. Pages differ in tint and in the marks on them, so chapters, scripts,
/// research and matter pages read as different kinds at a glance.
fn paint_doc_icon(painter: &egui::Painter, center: egui::Pos2, kind: DocKind, script: bool) {
    use egui::{pos2, Rect, Shape};
    if kind == DocKind::Folder {
        let color = egui::Color32::from_rgb(196, 158, 92);
        let body = Rect::from_center_size(center + Vec2::new(0.0, 1.0), Vec2::new(14.0, 10.0));
        let tab = Rect::from_min_size(
            pos2(body.left(), body.top() - 2.5),
            Vec2::new(6.0, 3.5),
        );
        painter.rect_filled(tab, CornerRadius::same(1), color);
        painter.rect_filled(body, CornerRadius::same(2), color);
        painter.rect_filled(
            Rect::from_min_max(
                pos2(body.left(), body.top() + 2.0),
                pos2(body.right(), body.bottom()),
            ),
            CornerRadius::same(2),
            egui::Color32::from_rgb(214, 180, 118),
        );
        return;
    }
    let tint = match kind {
        DocKind::Chapter => ACCENT_INK,
        DocKind::Scene if script => PURPLE,
        DocKind::Scene => SAGE_INK,
        DocKind::Research => BLUE,
        DocKind::Character => PURPLE,
        DocKind::FrontMatter | DocKind::BackMatter => egui::Color32::from_rgb(150, 140, 126),
        _ => FAINT,
    };
    let page = Rect::from_center_size(center, Vec2::new(11.0, 14.0));
    let fold = 3.5;
    let outline = Stroke::new(1.2, tint);
    let points = vec![
        page.left_top(),
        pos2(page.right() - fold, page.top()),
        pos2(page.right(), page.top() + fold),
        page.right_bottom(),
        page.left_bottom(),
    ];
    painter.add(Shape::convex_polygon(
        points,
        tint.gamma_multiply(0.14),
        outline,
    ));
    painter.line_segment(
        [pos2(page.right() - fold, page.top()), pos2(page.right() - fold, page.top() + fold)],
        Stroke::new(1.0, tint),
    );
    painter.line_segment(
        [pos2(page.right() - fold, page.top() + fold), pos2(page.right(), page.top() + fold)],
        Stroke::new(1.0, tint),
    );
    let line = Stroke::new(1.0, tint);
    let l = page.left() + 2.5;
    let r = page.right() - 2.5;
    match (kind, script) {
        (DocKind::Scene, true) => {
            // Script: short centred cue line above a longer line.
            painter.line_segment([pos2(center.x - 1.5, page.top() + 6.0), pos2(center.x + 1.5, page.top() + 6.0)], line);
            painter.line_segment([pos2(l, page.top() + 9.0), pos2(r, page.top() + 9.0)], line);
        }
        (DocKind::Research, _) => {
            painter.circle_filled(pos2(l + 0.8, page.top() + 7.0), 1.0, tint);
            painter.line_segment([pos2(l + 3.0, page.top() + 7.0), pos2(r, page.top() + 7.0)], line);
            painter.line_segment([pos2(l, page.top() + 10.0), pos2(r, page.top() + 10.0)], line);
        }
        (DocKind::Character, _) => {
            painter.circle_stroke(pos2(center.x, page.top() + 6.5), 1.7, line);
            painter.line_segment([pos2(l + 1.0, page.bottom() - 2.5), pos2(r - 1.0, page.bottom() - 2.5)], line);
        }
        _ => {
            painter.line_segment([pos2(l, page.top() + 7.0), pos2(r, page.top() + 7.0)], line);
            painter.line_segment([pos2(l, page.top() + 10.0), pos2(r, page.top() + 10.0)], line);
        }
    }
}
