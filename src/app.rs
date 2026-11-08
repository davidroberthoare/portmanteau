//! Scriptorium desktop shell.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Id, Key, Layout, Margin, RichText,
    Stroke, TextEdit, Vec2,
};

use msc_api::Workspace;
use msc_core::{DocKind, ElementKind, Medium, ProjectKind};

use crate::icons::{self, Icon};
use crate::theme::*;
use crate::ui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Write,
    Corkboard,
    Outline,
    Research,
    Develop,
    Characters,
    History,
    Compile,
    Agent,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Write => "Write",
            View::Corkboard => "Corkboard",
            View::Outline => "Outline",
            View::Research => "Research",
            View::Develop => "Develop",
            View::Characters => "Characters",
            View::History => "Versions",
            View::Compile => "Compile",
            View::Agent => "Command desk",
        }
    }
    pub fn icon(self) -> Icon {
        match self {
            View::Write => Icon::Write,
            View::Corkboard => Icon::Corkboard,
            View::Outline => Icon::Outline,
            View::Research => Icon::Research,
            View::Develop => Icon::Develop,
            View::Characters => Icon::Characters,
            View::History => Icon::History,
            View::Compile => Icon::Compile,
            View::Agent => Icon::Agent,
        }
    }
    pub fn all() -> [View; 9] {
        [
            View::Write,
            View::Corkboard,
            View::Outline,
            View::Research,
            View::Develop,
            View::Characters,
            View::History,
            View::Compile,
            View::Agent,
        ]
    }
    /// Development aid: `SCRIPTORIUM_VIEW=corkboard` opens a workspace
    /// directly, which makes automated screenshots of each screen possible.
    pub fn from_key(key: &str) -> Option<View> {
        View::all()
            .into_iter()
            .find(|v| format!("{v:?}").eq_ignore_ascii_case(key) || v.label().eq_ignore_ascii_case(key))
    }
}

fn initial_view() -> View {
    std::env::var("SCRIPTORIUM_VIEW")
        .ok()
        .and_then(|key| View::from_key(key.trim()))
        .unwrap_or(View::Write)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Document,
    Notes,
    History,
    Comments,
}

pub struct Toast {
    pub message: String,
    pub detail: String,
    pub at: Instant,
    pub kind: ToastKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PathDialog {
    Open,
    SaveAs,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Ok,
    Error,
    Info,
}

pub struct ScriptoriumApp {
    pub ws: Workspace,
    pub view: View,
    pub selected: Option<String>,
    pub selected_block: usize,
    pub block_focus: Option<usize>,
    pub inspector_tab: InspectorTab,
    pub collapsed: HashSet<String>,
    pub search: String,
    pub focus_search: bool,
    pub focus_mode: bool,
    pub snapshot_name: String,
    pub checkpoint_label: String,
    pub comment_draft: String,
    pub selected_research: Option<String>,
    pub selected_idea: Option<String>,
    pub selected_character: Option<String>,
    pub toast: Option<Toast>,
    pub show_about: bool,
    pub show_new_project: bool,
    pub new_project_title: String,
    pub new_project_kind: ProjectKind,
    pub cmd_name: String,
    pub cmd_payload: String,
    pub cmd_output: String,
    pub cmd_selected: usize,
    pub path_dialog: Option<PathDialog>,
    pub path_draft: String,
    pub outline_filter: usize,
    pub screenshot: Option<PathBuf>,
    pub frame_no: u32,
    pub screenshot_requested: bool,
    pub screenshot_saved: bool,
    pub editor_font_size: f32,
}

impl ScriptoriumApp {
    pub fn new(screenshot: Option<PathBuf>) -> Self {
        let ws = Workspace::demo();
        let selected = ws
            .project
            .documents
            .iter()
            .find(|d| d.title.starts_with("Chapter One"))
            .map(|d| d.id.clone())
            .or_else(|| ws.project.documents.iter().find(|d| d.kind == DocKind::Chapter).map(|d| d.id.clone()));
        let selected = std::env::var("SCRIPTORIUM_DOC")
            .ok()
            .and_then(|needle| {
                let needle = needle.to_lowercase();
                ws.project
                    .documents
                    .iter()
                    .find(|d| d.title.to_lowercase().contains(&needle))
                    .map(|d| d.id.clone())
            })
            .or(selected);
        let selected_character = ws.project.characters.first().map(|c| c.id.clone());
        let selected_research = ws.project.research.first().map(|r| r.id.clone());
        let mut collapsed = HashSet::new();
        if let Some(manuscript) = ws.project.folder_id("Manuscript") {
            collapsed.remove(&manuscript);
        }
        Self {
            ws,
            view: initial_view(),
            selected,
            selected_block: 0,
            block_focus: None,
            inspector_tab: InspectorTab::Document,
            collapsed,
            search: String::new(),
            focus_search: false,
            focus_mode: false,
            snapshot_name: String::new(),
            checkpoint_label: String::new(),
            comment_draft: String::new(),
            selected_research,
            selected_idea: None,
            selected_character,
            toast: None,
            show_about: false,
            show_new_project: false,
            new_project_title: String::new(),
            new_project_kind: ProjectKind::Novel,
            cmd_name: "project.info".into(),
            cmd_payload: "{}".into(),
            cmd_output: String::new(),
            cmd_selected: 0,
            path_dialog: None,
            path_draft: String::new(),
            outline_filter: 0,
            screenshot,
            frame_no: 0,
            screenshot_requested: false,
            screenshot_saved: false,
            editor_font_size: 17.0,
        }
    }

    pub fn current_doc(&self) -> Option<&msc_core::Document> {
        self.selected
            .as_ref()
            .and_then(|id| self.ws.project.document(id))
    }

    pub fn toast(&mut self, message: impl Into<String>, detail: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast {
            message: message.into(),
            detail: detail.into(),
            at: Instant::now(),
            kind,
        });
    }

    pub fn open_doc(&mut self, id: String) {
        self.selected = Some(id);
        self.selected_block = 0;
        self.view = View::Write;
    }

    fn save_project(&mut self) {
        match self.ws.save(None) {
            Ok(msg) => self.toast("Project saved", msg, ToastKind::Ok),
            Err(e) => self.toast("Could not save", e, ToastKind::Error),
        }
    }

    /// Public door for the editor and empty states, which cannot call private
    /// helpers from another module.
    pub fn take_snapshot_public(&mut self) {
        self.take_snapshot();
    }

    pub fn new_chapter_public(&mut self) {
        self.new_chapter();
    }

    fn new_chapter(&mut self) {
        let parent = self
            .selected
            .as_ref()
            .and_then(|id| self.ws.project.document(id))
            .and_then(|d| {
                if d.kind == DocKind::Folder {
                    Some(d.id.clone())
                } else {
                    d.parent.clone()
                }
            })
            .or_else(|| self.ws.project.folder_id("Manuscript"));
        let is_script = self
            .ws
            .project
            .document(
                parent
                    .as_deref()
                    .unwrap_or_default(),
            )
            .map(|d| d.title == "Script Lab")
            .unwrap_or(false)
            || matches!(self.ws.project.kind, ProjectKind::Film | ProjectKind::Television | ProjectKind::Theatre);
        let (kind, medium, title) = if is_script {
            (DocKind::Scene, Medium::Screenplay, "Untitled scene")
        } else {
            (DocKind::Chapter, Medium::Prose, "Untitled chapter")
        };
        let id = self.ws.project.add_document(parent, title, kind, medium);
        if medium.is_script() {
            if let Some(doc) = self.ws.project.document_mut(&id) {
                doc.blocks.push(msc_core::Block::new(ElementKind::SceneHeading, "INT. LOCATION — DAY"));
            }
        }
        self.ws.dirty = true;
        self.selected = Some(id);
        self.selected_block = 0;
        self.block_focus = Some(0);
        self.toast("New document", format!("{} added to the binder", title), ToastKind::Ok);
    }

    fn take_snapshot(&mut self) {
        let Some(id) = self.selected.clone() else { return };
        let name = if self.snapshot_name.trim().is_empty() {
            format!("Snapshot · {}", chrono::Local::now().format("%d %b %H:%M"))
        } else {
            self.snapshot_name.clone()
        };
        if let Some(doc) = self.ws.project.document_mut(&id) {
            doc.snapshot(name.clone(), "Manual snapshot from the desktop");
            self.ws.dirty = true;
            self.snapshot_name.clear();
            self.toast("Version saved", name, ToastKind::Ok);
        }
    }

    fn checkpoint(&mut self, label: Option<String>) {
        let label = label.unwrap_or_else(|| {
            if self.checkpoint_label.trim().is_empty() {
                format!("Checkpoint · {}", chrono::Local::now().format("%d %b %H:%M"))
            } else {
                self.checkpoint_label.clone()
            }
        });
        self.ws.project.checkpoint(label.clone(), "manual");
        self.ws.dirty = true;
        self.checkpoint_label.clear();
        self.toast("Project checkpoint saved", label, ToastKind::Ok);
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(54.0)
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(15, 17, 20))
                    .inner_margin(Margin::symmetric(14, 9)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, CornerRadius::same(7), ACCENT);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "S",
                        serif(18.0),
                        Color32::from_rgb(28, 24, 18),
                    );
                    ui.add_space(3.0);
                    ui.label(RichText::new("Scriptorium").font(serif(19.0)).color(TEXT));
                    ui.add_space(10.0);

                    self.menus(ui);
                    ui.add_space(8.0);
                    self.search_field(ui);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let compile = egui::Button::new(
                            RichText::new("Compile  ▸").font(sans(12.5)).color(Color32::from_rgb(32, 26, 18)),
                        )
                        .fill(ACCENT)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(96.0, 30.0));
                        if ui.add(compile).on_hover_text("Compile and export the manuscript").clicked() {
                            self.view = View::Compile;
                        }
                        ui.add_space(8.0);
                        let today = self.ws.project.today_words();
                        let target = self.ws.project.daily_target.max(1);
                        ui.vertical(|ui| {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(format!("{} / {} today", today, target))
                                    .font(sans(10.5))
                                    .color(MUTED),
                            );
                            ui.add_space(2.0);
                            let (r, _) = ui.allocate_exact_size(Vec2::new(88.0, 4.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, CornerRadius::same(2), PANEL_3);
                            let f = (today as f32 / target as f32).clamp(0.0, 1.0);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(r.min, Vec2::new(r.width() * f, 4.0)),
                                CornerRadius::same(2),
                                SAGE,
                            );
                        });
                        ui.add_space(10.0);
                        let (avatar, _) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), egui::Sense::click());
                        ui.painter().circle_filled(avatar.center(), 14.0, PANEL_3);
                        ui.painter().text(
                            avatar.center(),
                            egui::Align2::CENTER_CENTER,
                            "AM",
                            sans(10.5),
                            TEXT,
                        );
                        ui.add_space(6.0);
                        let (dir, resp) = ui.allocate_exact_size(Vec2::new(50.0, 26.0), egui::Sense::click());
                        ui.painter().rect_filled(dir, CornerRadius::same(6), PANEL_2);
                        ui.painter().text(
                            dir.center(),
                            egui::Align2::CENTER_CENTER,
                            "focus",
                            sans(10.0),
                            if self.focus_mode { ACCENT } else { MUTED },
                        );
                        if resp.on_hover_text("Hide the binder and inspector (F11)").clicked() {
                            self.focus_mode = !self.focus_mode;
                        }
                    });
                });
            });
    }

    fn menus(&mut self, ui: &mut egui::Ui) {
        let menu_text = |s: &str| RichText::new(s).font(sans(12.0)).color(MUTED);
        ui.menu_button(menu_text("File"), |ui| {
            if ui.button("New project…").clicked() {
                self.new_project_title.clear();
                self.show_new_project = true;
                ui.close_menu();
            }
            if ui.button("Save project").clicked() {
                self.save_project();
                ui.close_menu();
            }
            if ui.button("Save project as…").clicked() {
                self.path_dialog = Some(PathDialog::SaveAs);
                self.path_draft = self
                    .ws
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| {
                        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                        format!("{home}/Scriptorium/{}.scriptorium.json", slug(&self.ws.project.title))
                    });
                ui.close_menu();
            }
            if ui.button("Open project…").clicked() {
                self.path_dialog = Some(PathDialog::Open);
                self.path_draft = self
                    .ws
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                ui.close_menu();
            }
            if ui.button("Save checkpoint").clicked() {
                self.checkpoint(None);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Compile & export…").clicked() {
                self.view = View::Compile;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button(menu_text("Edit"), |ui| {
            if ui.button("Search manuscript").clicked() {
                self.focus_search = true;
                ui.close_menu();
            }
            if ui.button("Take snapshot").clicked() {
                self.take_snapshot();
                ui.close_menu();
            }
            if ui.button("Add comment").clicked() {
                self.inspector_tab = InspectorTab::Comments;
                ui.close_menu();
            }
        });
        ui.menu_button(menu_text("View"), |ui| {
            for v in View::all() {
                if ui.selectable_label(self.view == v, v.label()).clicked() {
                    self.view = v;
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui.button("Focus mode").clicked() {
                self.focus_mode = !self.focus_mode;
                ui.close_menu();
            }
        });
        ui.menu_button(menu_text("Insert"), |ui| {
            if ui.button("New chapter").clicked() {
                self.new_chapter();
                ui.close_menu();
            }
            if ui.button("New research note").clicked() {
                self.view = View::Research;
                ui.close_menu();
            }
            if ui.button("New character").clicked() {
                self.view = View::Characters;
                ui.close_menu();
            }
            if ui.button("New idea card").clicked() {
                self.view = View::Develop;
                ui.close_menu();
            }
        });
        ui.add_space(2.0);
        ui.label(RichText::new("Project").font(sans(12.0)).color(FAINT));
    }

    fn search_field(&mut self, ui: &mut egui::Ui) {
        Frame::new()
            .fill(PANEL_2)
            .stroke(Stroke::new(1.0, BORDER_SOFT))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(9, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (icon, _) =
                        ui.allocate_exact_size(Vec2::new(13.0, 13.0), egui::Sense::hover());
                    let c = icon.center();
                    ui.painter()
                        .circle_stroke(c - Vec2::new(1.0, 1.0), 3.6, Stroke::new(1.2, FAINT));
                    ui.painter().line_segment(
                        [c + Vec2::new(1.8, 1.8), c + Vec2::new(5.0, 5.0)],
                        Stroke::new(1.2, FAINT),
                    );
                    let resp = ui.add(
                        TextEdit::singleline(&mut self.search)
                            .hint_text(RichText::new("Search manuscript…").font(sans(11.5)).color(FAINT))
                            .desired_width(190.0)
                            .frame(false)
                            .font(sans(11.5)),
                    );
                    if self.focus_search {
                        resp.request_focus();
                        self.focus_search = false;
                    }
                    if !self.search.is_empty() {
                        if ui.small_button("×").clicked() {
                            self.search.clear();
                        }
                    }
                });
            });
    }

    fn rail(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("rail")
            .exact_width(62.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(RAIL)
                    .inner_margin(Margin::symmetric(8, 12)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(2.0);
                    let (logo, _) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), egui::Sense::hover());
                    ui.painter().rect_stroke(
                        logo,
                        CornerRadius::same(9),
                        Stroke::new(1.2, ACCENT.gamma_multiply(0.7)),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        logo.center(),
                        egui::Align2::CENTER_CENTER,
                        "S",
                        serif(20.0),
                        ACCENT,
                    );
                    ui.add_space(18.0);
                    for view in View::all() {
                        let active = self.view == view;
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(46.0, 44.0), egui::Sense::click());
                        let painter = ui.painter();
                        if active {
                            painter.rect_filled(rect, CornerRadius::same(8), ACCENT_SOFT);
                            let bar = egui::Rect::from_min_size(
                                egui::pos2(rect.left() - 8.0, rect.top() + 11.0),
                                Vec2::new(2.5, 22.0),
                            );
                            painter.rect_filled(bar, CornerRadius::same(2), ACCENT);
                        } else if resp.hovered() {
                            painter.rect_filled(rect, CornerRadius::same(8), PANEL_2);
                        }
                        icons::draw(
                            painter,
                            rect.shrink(12.0),
                            view.icon(),
                            if active { ACCENT } else { MUTED },
                        );
                        if resp.on_hover_text(view.label()).clicked() {
                            self.view = view;
                        }
                        ui.add_space(2.0);
                    }
                });

                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    let (av, _) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), egui::Sense::click());
                    ui.painter().circle_filled(av.center(), 14.0, SAGE_SOFT);
                    ui.painter().text(av.center(), egui::Align2::CENTER_CENTER, "AM", sans(9.5), SAGE);
                    ui.add_space(6.0);
                    let (help, hr) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), egui::Sense::click());
                    ui.painter().text(
                        help.center(),
                        egui::Align2::CENTER_CENTER,
                        "?",
                        serif(17.0),
                        if hr.hovered() { TEXT } else { FAINT },
                    );
                    if hr.clicked() {
                        self.show_about = true;
                    }
                });
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(27.0)
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(15, 17, 20))
                    .inner_margin(Margin::symmetric(14, 4)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let color = if self.ws.dirty { ACCENT } else { SAGE };
                    let (dot_rect, _) =
                        ui.allocate_exact_size(Vec2::new(7.0, 7.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 3.0, color);
                    ui.label(
                        RichText::new(if self.ws.dirty { "Unsaved changes" } else { "All changes saved" })
                            .font(sans(10.5))
                            .color(MUTED),
                    );
                    ui.add_space(8.0);
                    let path = self
                        .ws
                        .path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Demonstration project · in memory".into());
                    ui.label(RichText::new(path).font(mono(10.0)).color(FAINT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} words  ·  {} pages estimated",
                                self.ws.project.words(),
                                self.ws
                                    .project
                                    .manuscript_documents()
                                    .iter()
                                    .map(|d| d.pages())
                                    .sum::<u32>()
                            ))
                            .font(sans(10.5))
                            .color(MUTED),
                        );
                        ui.add_space(12.0);
                        if let Some(doc) = self.current_doc() {
                            ui.label(
                                RichText::new(format!("{} · {}", doc.title, doc.medium.label()))
                                    .font(sans(10.5))
                                    .color(FAINT),
                            );
                        }
                    });
                });
            });
    }

    fn toast_ui(&mut self, ctx: &egui::Context) {
        let Some(toast) = &self.toast else { return };
        if toast.at.elapsed() > Duration::from_secs(5) {
            self.toast = None;
            return;
        }
        let accent = match toast.kind {
            ToastKind::Ok => SAGE,
            ToastKind::Error => RED,
            ToastKind::Info => BLUE,
        };
        egui::Area::new(Id::new("toast"))
            .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-22.0, -42.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(PANEL_2)
                    .stroke(Stroke::new(1.0, accent.gamma_multiply(0.6)))
                    .corner_radius(CornerRadius::same(9))
                    .inner_margin(Margin::same(13))
                    .show(ui, |ui| {
                        ui.set_max_width(360.0);
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
                            ui.painter().circle_filled(r.center(), 4.0, accent);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&toast.message).font(sans(12.0)).color(TEXT));
                                ui.label(RichText::new(&toast.detail).font(sans(10.5)).color(MUTED));
                            });
                        });
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(200));
    }

    fn modals(&mut self, ctx: &egui::Context) {
        if self.show_new_project {
            let mut open = self.show_new_project;
            egui::Window::new("New project")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .min_width(440.0)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    section_label(ui, "Working title");
                    ui.add(
                        TextEdit::singleline(&mut self.new_project_title)
                            .hint_text("e.g. The Salt Road")
                            .desired_width(f32::INFINITY)
                            .font(serif(19.0)),
                    );
                    ui.add_space(12.0);
                    section_label(ui, "What are you writing?");
                    ui.horizontal_wrapped(|ui| {
                        for kind in ProjectKind::all() {
                            let selected = self.new_project_kind == kind;
                            let btn = egui::Button::new(
                                RichText::new(kind.label()).font(sans(11.5))
                                    .color(if selected { Color32::from_rgb(25, 21, 15) } else { TEXT }),
                            )
                            .fill(if selected { ACCENT } else { PANEL_2 })
                            .corner_radius(CornerRadius::same(20));
                            if ui.add(btn).clicked() {
                                self.new_project_kind = kind;
                            }
                        }
                    });
                    ui.add_space(16.0);
                    let ready = !self.new_project_title.trim().is_empty();
                    let btn = egui::Button::new(
                        RichText::new("Create project").font(sans(12.5))
                            .color(Color32::from_rgb(25, 21, 15)),
                    )
                    .fill(if ready { ACCENT } else { PANEL_3 })
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(140.0, 32.0));
                    if ui.add_enabled(ready, btn).clicked() {
                        let title = self.new_project_title.trim().to_string();
                        self.ws = Workspace::new_project(&title, self.new_project_kind);
                        let parent = self.ws.project.folder_id("Manuscript");
                        let id = self.ws.project.add_document(
                            parent,
                            "Chapter One",
                            DocKind::Chapter,
                            self.new_project_kind.default_medium(),
                        );
                        self.selected = Some(id);
                        self.view = View::Write;
                        self.show_new_project = false;
                        self.toast("Project created", title, ToastKind::Ok);
                    }
                });
            if !open {
                self.show_new_project = false;
            }
        }

        if self.show_about {
            let mut open = self.show_about;
            egui::Window::new("About Scriptorium")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .min_width(400.0)
                .show(ctx, |ui| {
                    ui.label(RichText::new("Scriptorium").font(serif(26.0)).color(TEXT));
                    ui.label(
                        RichText::new("A native writing studio for novels, film, television and theatre.")
                            .font(sans(12.0))
                            .color(MUTED),
                    );
                    ui.add_space(10.0);
                    hairline(ui);
                    ui.add_space(10.0);
                    for line in [
                        "Binder, corkboard and outline for long-form structure",
                        "Prose and screenplay editors with typed elements",
                        "Research, idea board, characters and story beats",
                        "Document snapshots, project checkpoints and restore",
                        "PDF, EPUB, DOCX, Final Draft, Fountain, HTML, Markdown, RTF and text",
                        "A JSON command API and an MCP server under the desktop UI",
                    ] {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(5.0, 5.0), egui::Sense::hover());
                            ui.painter().circle_filled(r.center(), 2.5, ACCENT);
                            ui.label(RichText::new(line).font(sans(11.5)).color(TEXT));
                        });
                    }
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Version 0.1 · modular Rust workspace")
                            .font(mono(10.0))
                            .color(FAINT),
                    );
                });
            if !open {
                self.show_about = false;
            }
        }

        self.idea_modal(ctx);
        self.path_modal(ctx);
    }

    fn path_modal(&mut self, ctx: &egui::Context) {
        let Some(mode) = self.path_dialog else { return };
        let mut open = true;
        let mut commit = false;
        let (title, verb, hint) = match mode {
            PathDialog::Open => (
                "Open project",
                "Open",
                "Path to a .scriptorium.json project",
            ),
            PathDialog::SaveAs => (
                "Save project as",
                "Save",
                "Where the project file should live",
            ),
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(520.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                section_label(ui, "Project file");
                ui.add_space(3.0);
                let resp = ui.add(
                    TextEdit::singleline(&mut self.path_draft)
                        .hint_text(hint)
                        .font(mono(10.5))
                        .desired_width(f32::INFINITY),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    commit = true;
                }
                ui.add_space(5.0);
                ui.label(
                    RichText::new(match mode {
                        PathDialog::Open => "Unsaved changes in the current project are left untouched.",
                        PathDialog::SaveAs => "A new copy is written; the open project switches to it.",
                    })
                    .font(sans(9.5))
                    .color(FAINT),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let btn = egui::Button::new(
                        RichText::new(verb)
                            .font(sans(11.5))
                            .color(Color32::from_rgb(29, 24, 17)),
                    )
                    .fill(ACCENT)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(100.0, 30.0));
                    if ui
                        .add_enabled(!self.path_draft.trim().is_empty(), btn)
                        .clicked()
                    {
                        commit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.path_dialog = None;
                    }
                });
            });

        if commit {
            match mode {
                PathDialog::Open => {
                    let path = PathBuf::from(self.path_draft.trim());
                    match Workspace::open(&path) {
                        Ok(ws) => {
                            self.ws = ws;
                            self.selected = self
                                .ws
                                .project
                                .documents
                                .iter()
                                .find(|d| d.kind == DocKind::Chapter)
                                .map(|d| d.id.clone());
                            self.collapsed.clear();
                            self.view = View::Write;
                            self.path_dialog = None;
                            self.toast("Project opened", path.display().to_string(), ToastKind::Ok);
                        }
                        Err(e) => {
                            self.toast("Could not open project", e, ToastKind::Error);
                        }
                    }
                }
                PathDialog::SaveAs => {
                    let mut path = PathBuf::from(self.path_draft.trim());
                    if path.extension().is_none() {
                        path.set_extension("scriptorium.json");
                    }
                    match self.ws.save(Some(path.clone())) {
                        Ok(msg) => {
                            self.path_dialog = None;
                            self.toast("Project saved", msg, ToastKind::Ok);
                        }
                        Err(e) => {
                            self.toast("Could not save project", e, ToastKind::Error);
                        }
                    }
                }
            }
        }
        if !open && self.path_dialog.is_some() {
            self.path_dialog = None;
        }
    }

    fn idea_modal(&mut self, ctx: &egui::Context) {
        let Some(id) = self.selected_idea.clone() else { return };
        let Some(pos) = self.ws.project.ideas.iter().position(|i| i.id == id) else {
            self.selected_idea = None;
            return;
        };
        let mut open = true;
        let mut delete = false;
        let mut close = false;
        egui::Window::new("Idea card")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(420.0)
            .show(ctx, |ui| {
                let card = &mut self.ws.project.ideas[pos];
                section_label(ui, "Title");
                if ui
                    .add(TextEdit::singleline(&mut card.title).desired_width(f32::INFINITY).font(serif(18.0)))
                    .changed()
                {
                    self.ws.dirty = true;
                }
                ui.add_space(8.0);
                section_label(ui, "Note");
                if ui
                    .add(TextEdit::multiline(&mut card.body).desired_rows(7).desired_width(f32::INFINITY))
                    .changed()
                {
                    self.ws.dirty = true;
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut card.pinned, "Pin to the top").changed() {
                        self.ws.dirty = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Delete").clicked() {
                            delete = true;
                        }
                        if ui.button("Done").clicked() {
                            close = true;
                        }
                    });
                });
            });
        if delete {
            self.ws.project.ideas.retain(|i| i.id != id);
            self.ws.dirty = true;
            self.selected_idea = None;
            return;
        }
        if !open || close {
            self.selected_idea = None;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (save, save_as, open, compile, focus) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(Key::S),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::S),
                i.modifiers.command && i.key_pressed(Key::O),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::E),
                i.key_pressed(Key::F11),
            )
        });
        if save && !save_as {
            self.save_project();
        }
        if save_as {
            self.path_dialog = Some(PathDialog::SaveAs);
            self.path_draft = self
                .ws
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                    format!("{home}/Scriptorium/{}.scriptorium.json", slug(&self.ws.project.title))
                });
        }
        if open {
            self.path_dialog = Some(PathDialog::Open);
            self.path_draft = self
                .ws
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
        }
        if compile {
            self.view = View::Compile;
        }
        if focus {
            self.focus_mode = !self.focus_mode;
        }
    }

    fn screenshot_tick(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else { return };
        self.frame_no = self.frame_no.saturating_add(1);
        ctx.request_repaint();
        if self.frame_no >= 120 && !self.screenshot_requested {
            self.screenshot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let mut save: Option<std::sync::Arc<egui::ColorImage>> = None;
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    save = Some(image.clone());
                }
            }
        });
        if let Some(image) = save {
            if self.screenshot_saved {
                return;
            }
            self.screenshot_saved = true;
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let [w, h] = image.size;
            let mut rgba = Vec::with_capacity(w * h * 4);
            for px in &image.pixels {
                let [r, g, b, a] = px.to_array();
                rgba.extend_from_slice(&[r, g, b, a]);
            }
            match image::save_buffer(&path, &rgba, w as u32, h as u32, image::ColorType::Rgba8) {
                Ok(_) => eprintln!("screenshot saved to {}", path.display()),
                Err(e) => eprintln!("could not save screenshot: {e}"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn slug(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if s.is_empty() { "untitled".into() } else { s }
}

impl eframe::App for ScriptoriumApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.top_bar(ctx);
        self.status_bar(ctx);
        self.rail(ctx);
        self.toast_ui(ctx);

        if self.view == View::Write {
            if !self.focus_mode {
                self.binder(ctx);
            }
            if !self.focus_mode {
                self.inspector(ctx);
            }
            egui::CentralPanel::default()
                .frame(Frame::new().fill(BG))
                .show(ctx, |ui| {
                    ui::editor::show(self, ui);
                });
        } else {
            egui::CentralPanel::default()
                .frame(Frame::new().fill(BG))
                .show(ctx, |ui| {
                    ui::views::show(self, ui);
                });
        }

        self.modals(ctx);
        self.screenshot_tick(ctx);
    }
}

impl ScriptoriumApp {
    fn binder(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("binder")
            .exact_width(272.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(12, 10)),
            )
            .show(ctx, |ui| {
                ui::binder::show(self, ui);
            });
    }

    fn inspector(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .exact_width(304.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(14, 10)),
            )
            .show(ctx, |ui| {
                ui::inspector::show(self, ui);
            });
    }
}
