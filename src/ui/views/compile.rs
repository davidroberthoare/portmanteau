//! Compile: choose an output format, decide what goes in the book, and write a
//! real file — PDF, EPUB, DOCX, Final Draft, Fountain, HTML, Markdown, RTF,
//! plain text or a complete project bundle. Editor display settings live in the
//! separate Settings workspace.

use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};
use portmanteau_core::{
    Alignment, CompileScope, CompileType, DocKind, ElementStyles, ExportKind, ExportSettings,
    HfLocation, NumberFormat, PageMargins, PaperSize, Project, ProjectKind,
};

use crate::app::{CompileTab, PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::views::{
    content_frame, custom_styles_editor, element_styles_editor, manuscript_preview_navigator,
    meta_line, page_head, quiet_button, style_sheets_editor, FONT_FAMILIES,
};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        "Finish",
        "Compile",
        "One manuscript, many finish lines. Pick an output format, then shape what goes in the book.",
        |ui| {
            let included = app
                .ws
                .project
                .compile_plan_for(&app.ws.project.export, app.selected.as_deref())
                .included;
            // Words of what this compile contains, not of the whole project.
            let words: u32 = included.iter().map(|doc| doc.words()).sum();
            meta_line(
                ui,
                &[&format!(
                    "{} document{} in this compile · {words} words",
                    included.len(),
                    if included.len() == 1 { "" } else { "s" },
                )],
            );
        },
    );

    let mut tab = app.compile_tab;
    let mut settings = match tab {
        CompileTab::Format(kind) => app.ws.project.settings_for(kind),
    };
    let CompileTab::Format(kind) = tab;
    let mut compile = false;
    let front_docs: Vec<(String, String, bool)> = app
        .ws
        .project
        .ordered_documents()
        .into_iter()
        .filter(|d| d.kind == DocKind::FrontMatter)
        .map(|d| (d.id.clone(), d.title.clone(), d.include_in_export))
        .collect();
    let back_docs: Vec<(String, String, bool)> = app
        .ws
        .project
        .ordered_documents()
        .into_iter()
        .filter(|d| d.kind == DocKind::BackMatter)
        .map(|d| (d.id.clone(), d.title.clone(), d.include_in_export))
        .collect();
    // Binder items the scope selector can narrow to, with their depth so the
    // labels read like the binder.
    let scope_items: Vec<(String, String)> = app
        .ws
        .project
        .ordered_documents()
        .into_iter()
        .map(|d| (d.id.clone(), d.title.clone()))
        .collect();
    let selected_id = app.selected.clone();
    // Automatic mode resolves to a concrete type for the preview and the
    // export; the saved setting stays "auto".
    let resolved_kind = app
        .ws
        .project
        .compile_plan_for(&settings, selected_id.as_deref())
        .kind;
    let mut effective = settings.clone();
    effective.compile_type = Some(resolved_kind);
    let mut matter_toggles: Vec<(String, bool)> = Vec::new();
    let mut style_sheets_changed = false;

    content_frame().show(ui, |ui| {
        let height = (ui.available_height() - 4.0).max(420.0);
        ui.horizontal_top(|ui| {
            // ---- Vertical tab rail --------------------------------------
            ui.allocate_ui_with_layout(
                Vec2::new(178.0, height),
                Layout::top_down(Align::Min),
                |ui| {
                    section_label(ui, "Output");
                    ui.add_space(5.0);
                    ScrollArea::vertical()
                        .id_salt("compile_tab_rail")
                        .auto_shrink([false, false])
                        .max_height(height - 24.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            for kind in ExportKind::all() {
                                let selected = tab == CompileTab::Format(kind);
                                if tab_button(ui, tab_label(kind), format_color(kind), selected) {
                                    tab = CompileTab::Format(kind);
                                    settings = app.ws.project.settings_for(kind);
                                    app.ws.project.export = settings.clone();
                                    app.preview_page = 0;
                                }
                                ui.add_space(2.0);
                            }
                        });
                },
            );
            ui.add_space(16.0);
            // ---- Middle column: layout parameters -------------------
            ui.allocate_ui_with_layout(
                Vec2::new(400.0, height),
                Layout::top_down(Align::Min),
                |ui| {
                    ScrollArea::vertical()
                        .id_salt("compile_params")
                        .auto_shrink([false, false])
                        .max_height(height)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            output_tab(
                                app,
                                ui,
                                kind,
                                &mut settings,
                                &front_docs,
                                &back_docs,
                                &scope_items,
                                selected_id.as_deref(),
                                &mut matter_toggles,
                                &mut compile,
                                &mut style_sheets_changed,
                            );
                        });
                },
            );
            ui.add_space(16.0);
            // ---- Right column: single-page live preview -------------
            let preview_width = ui.available_width();
            ui.allocate_ui_with_layout(
                Vec2::new(preview_width, height),
                Layout::top_down(Align::Min),
                |ui| {
                    ScrollArea::vertical()
                        .id_salt("compile_preview")
                        .auto_shrink([false, false])
                        .max_height(height)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            manuscript_preview_navigator(
                                &app.ws.project,
                                &effective,
                                &mut app.preview_page,
 height,
                                ui,
                            );
                        });
                },
            );
        });
    });

    app.compile_tab = tab;
    let before = app.ws.project.settings_for(settings.kind);
    app.ws.project.set_settings(settings.clone());
    app.ws.project.export = settings.clone();
    if before != settings {
        app.ws.dirty = true;
    }
    if style_sheets_changed {
        app.ws.project.touch();
        app.ws.dirty = true;
    }
    let mut matter_changed = false;
    for (id, include) in matter_toggles {
        if let Some(doc) = app.ws.project.document_mut(&id) {
            if doc.include_in_export != include {
                doc.include_in_export = include;
                doc.touch();
                matter_changed = true;
            }
        }
    }
    if matter_changed {
        app.ws.project.touch();
        app.ws.dirty = true;
    }
    if compile {
        run_export(app, &effective);
    }
}

/// The rail labels: the compact, familiar names for each export format.
fn tab_label(kind: ExportKind) -> &'static str {
    match kind {
        ExportKind::Pdf => "PDF",
        ExportKind::Epub => "EPUB",
        ExportKind::Docx => "DOCX",
        ExportKind::Fdx => "FDX",
        ExportKind::Fountain => "Fountain",
        ExportKind::Html => "HTML",
        ExportKind::Markdown => "Markdown",
        ExportKind::Rtf => "RTF",
        ExportKind::Txt => "TXT",
        ExportKind::Bundle => "Bundle",
    }
}

fn tab_button(ui: &mut egui::Ui, label: &str, color: Color32, selected: bool) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(7), ACCENT_SOFT);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(7),
            Stroke::new(1.0, ACCENT.gamma_multiply(0.75)),
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(7), PANEL_2);
    }
    let dot = egui::pos2(rect.left() + 14.0, rect.center().y);
    ui.painter().circle_filled(
        dot,
        4.0,
        if selected { color } else { color.gamma_multiply(0.7) },
    );
    ui.painter().text(
        egui::pos2(dot.x + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        ui_body(),
        if selected { TEXT } else { MUTED },
    );
    resp.clicked()
}

fn format_color(kind: ExportKind) -> Color32 {
    match kind {
        ExportKind::Pdf => ACCENT,
        ExportKind::Epub => SAGE,
        ExportKind::Docx => BLUE,
        ExportKind::Fdx => PURPLE,
        ExportKind::Fountain => Color32::from_rgb(122, 176, 186),
        ExportKind::Html => Color32::from_rgb(190, 148, 120),
        ExportKind::Markdown => Color32::from_rgb(158, 166, 150),
        ExportKind::Rtf => Color32::from_rgb(150, 138, 172),
        ExportKind::Txt => MUTED,
        ExportKind::Bundle => Color32::from_rgb(186, 146, 104),
    }
}

fn seeded_settings(
    styles: &ElementStyles,
    project_kind: ProjectKind,
    kind: ExportKind,
) -> ExportSettings {
    let mut settings = ExportSettings {
        kind,
        styles: styles.clone(),
        ..ExportSettings::default()
    };
    let base = if project_kind.default_medium().is_script() {
        &styles.action
    } else {
        &styles.body
    };
    settings.font = base.font.clone();
    settings.font_size = base.size;
    settings.line_spacing = base.leading;
    settings
}

// ---------------------------------------------------------------------------
// Output tab
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn output_tab(
    app: &mut PortmanteauApp,
    ui: &mut egui::Ui,
    kind: ExportKind,
    settings: &mut ExportSettings,
    front_docs: &[(String, String, bool)],
    back_docs: &[(String, String, bool)],
    scope_items: &[(String, String)],
    selected_id: Option<&str>,
    toggles: &mut Vec<(String, bool)>,
    compile: &mut bool,
    style_sheets_changed: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(kind.label()).font(serif(21.0)).color(TEXT));
            ui.label(
                RichText::new(kind.description())
                    .font(ui_small())
                    .color(FAINT),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            pill(
                ui,
                &format!(".{}", kind.extension()),
                format_color(kind),
                PANEL_2,
            );
        });
    });
    ui.add_space(8.0);

    selection_panel(ui, settings, scope_items, selected_id);
    ui.add_space(10.0);

    output_panel(app, ui, settings);
    ui.add_space(10.0);

    action_panel(app, ui, settings, compile);
    ui.add_space(12.0);

    let compile_type = app
        .ws
        .project
        .compile_plan_for(settings, selected_id)
        .kind;
    style_panel(ui, &mut settings.styles, app.ws.project.kind, compile_type);
    ui.add_space(12.0);
    if style_sheets_editor(ui, &mut app.ws.project.style_sheets) {
        *style_sheets_changed = true;
        app.ws.project.sync_element_styles_from_sheets();
        settings.styles = app.ws.project.styles.clone();
    }
    ui.add_space(8.0);

    type_panel(ui, kind, settings, compile_type);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if quiet_button(ui, "Reset this format to base style").clicked() {
            *settings = seeded_settings(&app.ws.project.styles, app.ws.project.kind, settings.kind);
        }
        ui.label(
            RichText::new("Seeds from the project-wide element styles.")
                .font(ui_small())
                .color(FAINT),
        );
    });
    ui.add_space(12.0);

    if compile_type.is_script() {
        script_panel(ui, settings, kind, compile_type);
    } else {
        ui.columns(2, |cols| {
            front_panel(&mut cols[0], settings, kind, front_docs, toggles);
            body_panel(&mut cols[1], settings, kind, back_docs, toggles);
        });
    }
    ui.add_space(12.0);

    preview_panel(app, ui, settings);
}

fn output_panel(app: &PortmanteauApp, ui: &mut egui::Ui, settings: &mut ExportSettings) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Where it goes");
            ui.add_space(4.0);
            let default_name = portmanteau_export::default_filename(&app.ws.project, settings.kind);
            egui::Grid::new("export_location")
                .num_columns(2)
                .spacing([8.0, 5.0])
                .min_col_width(48.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("Folder").font(ui_small()).color(FAINT));
                    output_field(
                        ui,
                        &mut settings.output_path,
                        "Folder for the export (blank = ./exports)",
                    );
                    ui.end_row();

                    ui.label(RichText::new("Filename").font(ui_small()).color(FAINT));
                    output_field(ui, &mut settings.output_filename, &default_name);
                    ui.end_row();
                });
            ui.add_space(5.0);
            ui.label(
                RichText::new(format!(
                    "Writes {}",
                    export_target_label(&app.ws.project, settings)
                ))
                .font(mono(9.0))
                .color(FAINT),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Front and back matter are assembled from the binder folders; excluded chapters are skipped.",
                )
                .font(ui_small())
                .color(FAINT),
            );
        });
}

/// One path field in the "Where it goes" panel, framed like a form input.
fn output_field(ui: &mut egui::Ui, value: &mut String, hint: &str) {
    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.add(
                TextEdit::singleline(value)
                    .hint_text(hint)
                    .font(mono(10.0))
                    .frame(Frame::NONE)
                    .align(egui::Align2::LEFT_CENTER)
                    .min_size(Vec2::new(0.0, 22.0))
                    .desired_width((ui.available_width() - 4.0).max(120.0)),
            );
        });
}

/// Expand a leading `~/` the way a file dialog would; joining a path with a
/// literal `~` used to create a folder named "~" in the working directory.
fn expand_tilde(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            if !home.trim().is_empty() {
                return std::path::Path::new(&home).join(rest);
            }
        }
    }
    std::path::PathBuf::from(path)
}

/// The file the Compile button writes. A blank folder means `./exports`; a
/// blank filename means the format's standard name. The result is absolute so
/// the writer can always find it, however the app was launched.
fn resolved_export_path(project: &Project, settings: &ExportSettings) -> String {
    let folder = if settings.output_path.trim().is_empty() {
        std::path::PathBuf::from("exports")
    } else {
        expand_tilde(settings.output_path.trim())
    };
    let filename = if settings.output_filename.trim().is_empty() {
        std::path::PathBuf::from(portmanteau_export::default_filename(project, settings.kind))
    } else {
        expand_tilde(settings.output_filename.trim())
    };
    let path = if filename.is_absolute() {
        filename
    } else {
        folder.join(filename)
    };
    if path.is_absolute() {
        return path.display().to_string();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(&path))
        .unwrap_or(path)
        .display()
        .to_string()
}

/// The path shown under "Where it goes", with the home folder abbreviated so a
/// long absolute path still fits.
fn export_target_label(project: &Project, settings: &ExportSettings) -> String {
    let path = resolved_export_path(project, settings);
    crate::app::short_path(Path::new(&path))
}

fn action_panel(
    app: &PortmanteauApp,
    ui: &mut egui::Ui,
    settings: &ExportSettings,
    compile: &mut bool,
) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(16, 13))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let label = format!("Compile {}  ▸", settings.kind.label());
                let btn = egui::Button::new(
                    RichText::new(label)
                        .font(ui_body())
                        .color(Color32::WHITE),
                )
                .fill(ACCENT)
                .corner_radius(CornerRadius::same(6))
                .min_size(Vec2::new(190.0, 34.0));
                if ui.add(btn).clicked() {
                    *compile = true;
                }
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new({
                            let plan = app
                                .ws
                                .project
                                .compile_plan_for(settings, app.selected.as_deref());
                            let mut effective = settings.clone();
                            effective.compile_type = Some(plan.kind);
                            let pages = if settings.kind == ExportKind::Pdf {
                                page_estimate(ui.ctx(), &app.ws.project, &effective)
                            } else {
                                plan.included.iter().map(|d| d.pages()).sum()
                            };
                            format!(
                                "{} words · {} pages estimated",
                                plan.included.iter().map(|d| d.words()).sum::<u32>(),
                                pages
                            )
                        })
                        .font(ui_small())
                        .color(MUTED),
                    );
                    ui.label(
                        RichText::new({
                            let count = app
                                .ws
                                .project
                                .compile_plan_for(settings, app.selected.as_deref())
                                .included
                                .iter()
                                .filter(|d| !matches!(
                                    d.kind,
                                    DocKind::FrontMatter | DocKind::BackMatter
                                ))
                                .count();
                            format!(
                                "Includes {count} manuscript section{}",
                                if count == 1 { "" } else { "s" }
                            )
                        })
                        .font(ui_small())
                        .color(FAINT),
                    );
                });
            });
            if let Some(info) = app.ws.last_export.clone() {
                ui.add_space(10.0);
                hairline(ui);
                ui.add_space(9.0);
                ui.horizontal(|ui| {
                    let (dot_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter()
                        .circle_filled(dot_rect.center(), 3.5, SAGE);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Last export · {} · {:.1} KB",
                                info.format,
                                info.bytes as f32 / 1024.0
                            ))
                            .font(ui_small())
                            .color(SAGE),
                        );
                        ui.label(
                            RichText::new(&info.path)
                                .font(mono(9.0))
                                .color(FAINT),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Copy path").clicked() {
                            ui.ctx().copy_text(info.path.clone());
                        }
                    });
                });
            }
        });
}

fn style_panel(
    ui: &mut egui::Ui,
    styles: &mut ElementStyles,
    project_kind: ProjectKind,
    compile_type: CompileType,
) {
    // Grey out the element group that cannot apply to this compile.
    let _ = project_kind;
    let grey_prose = compile_type.is_script();
    let grey_script = compile_type == CompileType::Prose;
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Element styles");
            ui.label(
                RichText::new(
                    "Per-element type for this output format, including writer-defined styles. The global defaults live in Settings → Display.",
                )
                .font(ui_small())
                .color(FAINT),
            );
            ui.add_space(6.0);
            element_styles_editor(ui, styles, grey_prose, grey_script);
            ui.add_space(2.0);
            custom_styles_editor(ui, styles);
        });
}

fn type_panel(
    ui: &mut egui::Ui,
    kind: ExportKind,
    settings: &mut ExportSettings,
    compile_type: CompileType,
) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Page & type");
            ui.add_space(5.0);

            if matches!(
                kind,
                ExportKind::Pdf | ExportKind::Docx | ExportKind::Rtf
            ) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Paper").font(ui_small()).color(MUTED));
                    egui::ComboBox::from_id_salt("paper_pick")
                        .selected_text(
                            RichText::new(settings.paper.label()).font(ui_small()),
                        )
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for paper in PaperSize::all() {
                                ui.selectable_value(&mut settings.paper, paper, paper.label());
                            }
                        });
                });
                ui.add_space(4.0);
            }

            margins_row(ui, kind, settings, compile_type);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Font").font(ui_small()).color(MUTED));
                if compile_type.is_script() {
                    // A script's face comes from its own stylesheet, so a
                    // format-wide font choice would silently do nothing.
                    ui.label(RichText::new("From the script stylesheet").font(ui_small()).color(FAINT));
                } else {
                    font_picker(ui, &mut settings.font);
                }
            });

            if kind == ExportKind::Pdf {
                ui.add_space(7.0);
                header_footer_panel(ui, settings);
            }

            if matches!(kind, ExportKind::Epub | ExportKind::Html) {
                ui.add_space(7.0);
                ui.label(
                    RichText::new("Cover image")
                        .font(ui_small())
                        .color(MUTED),
                );
                ui.add(
                    TextEdit::singleline(&mut settings.cover_image)
                        .hint_text("Path to a .jpg or .png")
                        .font(mono(9.0))
                        .desired_width(f32::INFINITY),
                );
            }
        });
}

/// What "Custom margins" starts from, so ticking the box changes nothing until
/// a value is edited: the automatic margins for this manuscript type, in points.
fn automatic_margins(compile_type: CompileType) -> PageMargins {
    if compile_type.is_script() {
        // Film and TV: a 1.5" binding edge, 1" elsewhere (before any paper scaling).
        PageMargins::default()
    } else {
        // The book default: 20 mm at the sides, 22 mm top and bottom.
        let (side, vertical) = (20.0 * 72.0 / 25.4, 22.0 * 72.0 / 25.4);
        PageMargins {
            left: side,
            right: side,
            top: vertical,
            bottom: vertical,
        }
    }
}

/// Page margins, in inches. Off means automatic. An EPUB reflows, so only its
/// side margins apply; the paged formats use all four.
fn margins_row(
    ui: &mut egui::Ui,
    kind: ExportKind,
    settings: &mut ExportSettings,
    compile_type: CompileType,
) {
    let mut custom = settings.margins.is_some();
    if ui
        .checkbox(&mut custom, "Custom margins")
        .on_hover_text("Off: book or script margins for the chosen paper")
        .changed()
    {
        settings.margins = custom.then(|| automatic_margins(compile_type));
    }
    let Some(margins) = settings.margins.as_mut() else {
        ui.add_space(4.0);
        return;
    };
    let reflows = kind == ExportKind::Epub;
    ui.indent("custom_margins", |ui| {
        ui.horizontal(|ui| {
            margin_field(ui, "Left", &mut margins.left);
            margin_field(ui, "Right", &mut margins.right);
        });
        if !reflows {
            ui.horizontal(|ui| {
                margin_field(ui, "Top", &mut margins.top);
                margin_field(ui, "Bottom", &mut margins.bottom);
            });
        }
    });
    ui.add_space(4.0);
}

/// One margin edge: a label and a field in inches, stored as points.
fn margin_field(ui: &mut egui::Ui, label: &str, points: &mut f32) {
    let mut inches = *points / 72.0;
    ui.label(RichText::new(label).font(ui_small()).color(MUTED));
    let field = egui::DragValue::new(&mut inches)
        .speed(0.02)
        .range(0.0..=3.0)
        .fixed_decimals(2)
        .suffix(" in");
    if ui.add(field).changed() {
        *points = inches * 72.0;
    }
}

fn header_footer_panel(ui: &mut egui::Ui, settings: &mut ExportSettings) {
    // Edit a local clone, then commit once the draw pass is over so the combo
    // boxes can borrow the settings without fighting the rest of the panel.
    let mut hf = settings.header_footer.clone();

    hairline(ui);
    ui.add_space(6.0);
    section_label(ui, "Headers & footers");
    ui.add_space(4.0);

    let mut page_numbers = hf.page_numbers;
    if ui.checkbox(&mut page_numbers, "Page numbers").changed() {
        hf.page_numbers = page_numbers;
    }
    if hf.page_numbers {
        ui.indent("hf_page_numbers", |ui| {
            ui.horizontal(|ui| {
                let mut first = hf.number_on_first_page;
                if ui.checkbox(&mut first, "On first page").changed() {
                    hf.number_on_first_page = first;
                }
                ui.add_space(8.0);
                let mut start = hf.start_number;
                if ui
                    .add(
                        egui::DragValue::new(&mut start)
                            .speed(1.0)
                            .range(1..=9999)
                            .prefix("start at "),
                    )
                    .changed()
                {
                    hf.start_number = start;
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Format").font(ui_small()).color(MUTED));
                egui::ComboBox::from_id_salt("hf_number_format")
                    .selected_text(RichText::new(hf.number_format.label()).font(ui_small()))
                    .width(105.0)
                    .show_ui(ui, |ui| {
                        for format in NumberFormat::all() {
                            ui.selectable_value(&mut hf.number_format, format, format.label());
                        }
                    });
                ui.add_space(6.0);
                ui.label(RichText::new("Where").font(ui_small()).color(MUTED));
                egui::ComboBox::from_id_salt("hf_number_location")
                    .selected_text(RichText::new(hf.number_location.label()).font(ui_small()))
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for location in HfLocation::all() {
                            ui.selectable_value(&mut hf.number_location, location, location.label());
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Align").font(ui_small()).color(MUTED));
                egui::ComboBox::from_id_salt("hf_number_align")
                    .selected_text(RichText::new(hf.number_align.label()).font(ui_small()))
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for align in [Alignment::Left, Alignment::Center, Alignment::Right] {
                            ui.selectable_value(&mut hf.number_align, align, align.label());
                        }
                    });
                ui.add_space(6.0);
                ui.label(RichText::new("Spacing").font(ui_small()).color(MUTED));
                ui.add(
                    egui::DragValue::new(&mut hf.number_spacing)
                        .speed(0.5)
                        .range(0.0..=120.0)
                        .suffix(" pt"),
                );
            });
        });
    }

    ui.add_space(4.0);
    let mut running = hf.running_header;
    if ui.checkbox(&mut running, "Running header").changed() {
        hf.running_header = running;
    }
    if hf.running_header {
        ui.indent("hf_running_header", |ui| {
            ui.add(
                TextEdit::singleline(&mut hf.header_text)
                    .hint_text("blank = book title")
                    .font(ui_small())
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal(|ui| {
                ui.label(RichText::new("Align").font(ui_small()).color(MUTED));
                egui::ComboBox::from_id_salt("hf_header_align")
                    .selected_text(RichText::new(hf.header_align.label()).font(ui_small()))
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for align in [Alignment::Left, Alignment::Center, Alignment::Right] {
                            ui.selectable_value(&mut hf.header_align, align, align.label());
                        }
                    });
                ui.add_space(6.0);
                ui.label(RichText::new("Spacing").font(ui_small()).color(MUTED));
                ui.add(
                    egui::DragValue::new(&mut hf.header_spacing)
                        .speed(0.5)
                        .range(0.0..=120.0)
                        .suffix(" pt"),
                );
            });
        });
    }

    settings.header_footer = hf;
}

fn font_picker(ui: &mut egui::Ui, font: &mut String) {
    let selected = if font.trim().is_empty() {
        "(inherit)".to_string()
    } else {
        font.clone()
    };
    egui::ComboBox::from_id_salt("compile_font")
        .selected_text(RichText::new(selected).font(ui_small()).color(TEXT))
        .width(170.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(font, String::new(), "(inherit)");
            if !font.trim().is_empty() && !FONT_FAMILIES.contains(&font.as_str()) {
                let custom = font.clone();
                ui.selectable_value(font, custom.clone(), format!("{custom} (custom)"));
            }
            for family in FONT_FAMILIES {
                ui.selectable_value(font, family.to_string(), family);
            }
        });
}

/// Script compiles have no book front or back matter: a title page, scene
/// numbers and the "one file per script" choice instead.
fn script_panel(
    ui: &mut egui::Ui,
    settings: &mut ExportSettings,
    kind: ExportKind,
    compile_type: CompileType,
) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Script pages");
            ui.add_space(5.0);
            check(
                ui,
                settings,
                |s| &mut s.include_title_page,
                "Script title page",
            );
            check(
                ui,
                settings,
                |s| &mut s.include_scene_numbers,
                "Scene numbers",
            );
            check(ui, settings, |s| &mut s.include_synopsis, "Include synopses");
            check(ui, settings, |s| &mut s.include_notes, "Include author notes");
            if !matches!(kind, ExportKind::Txt | ExportKind::Markdown | ExportKind::Rtf) {
                check(
                    ui,
                    settings,
                    |s| &mut s.each_separately,
                    "Write each script as its own file",
                );
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "A {} compile takes only {} documents. Front and back matter, contents and chapter numbering belong to the prose manuscript.",
                    compile_type.label().to_lowercase(),
                    compile_type.label().to_lowercase()
                ))
                .font(ui_small())
                .color(FAINT),
            );
        });
}

/// Manuscript type and scope: which kind of document this compile is, and
/// which part of the binder it draws from.
fn selection_panel(
    ui: &mut egui::Ui,
    settings: &mut ExportSettings,
    scope_items: &[(String, String)],
    selected_id: Option<&str>,
) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "What is being compiled");
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Manuscript").font(ui_small()).color(MUTED));
                let selected = settings
                    .compile_type
                    .map(|t| t.label())
                    .unwrap_or("Auto (from the binder)");
                egui::ComboBox::from_id_salt("compile_type")
                    .selected_text(RichText::new(selected).font(ui_small()))
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut settings.compile_type, None, "Auto (from the binder)");
                        for compile_type in CompileType::all() {
                            ui.selectable_value(
                                &mut settings.compile_type,
                                Some(compile_type),
                                compile_type.label(),
                            );
                        }
                    });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Scope").font(ui_small()).color(MUTED));
                let mut draft = settings.scope == CompileScope::Draft;
                let before = settings.scope.clone();
                egui::ComboBox::from_id_salt("compile_scope")
                    .selected_text(
                        RichText::new(match &settings.scope {
                            CompileScope::Draft => "Whole draft".to_owned(),
                            CompileScope::Subtree(id) => scope_items
                                .iter()
                                .find(|(item, _)| item == id)
                                .map(|(_, title)| title.clone())
                                .unwrap_or_else(|| "Whole draft".to_owned()),
                            CompileScope::Documents(_) => "Selected documents".to_owned(),
                        })
                        .font(ui_small()),
                    )
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft, true, "Whole draft");
                        for (id, title) in scope_items.iter().take(40) {
                            ui.selectable_value(
                                &mut settings.scope,
                                CompileScope::Subtree(id.clone()),
                                title,
                            );
                        }
                    });
                if draft {
                    settings.scope = CompileScope::Draft;
                }
                let _ = before;
                if let Some(id) = selected_id {
                    if ui
                        .small_button("Use binder selection")
                        .on_hover_text("Compile the selected binder item and everything beneath it")
                        .clicked()
                    {
                        settings.scope = CompileScope::Subtree(id.to_owned());
                    }
                }
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Each export is one kind of manuscript. Documents of another kind are listed as skipped, not silently dropped.",
                )
                .font(ui_small())
                .color(FAINT),
            );
        });
}

fn front_panel(
    ui: &mut egui::Ui,
    settings: &mut ExportSettings,
    kind: ExportKind,
    front_docs: &[(String, String, bool)],
    toggles: &mut Vec<(String, bool)>,
) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Front matter");
            ui.add_space(5.0);
            let plain_text = matches!(
                kind,
                ExportKind::Txt | ExportKind::Markdown | ExportKind::Rtf
            );
            if !plain_text {
                check(ui, settings, |s| &mut s.include_title_page, "Title page");
            }
            matter_list(ui, front_docs, toggles);
            if !plain_text {
                check(ui, settings, |s| &mut s.include_toc, "Table of contents");
                check(
                    ui,
                    settings,
                    |s| &mut s.include_chapter_numbers,
                    "Chapter numbers",
                );
            }
        });
}

fn body_panel(
    ui: &mut egui::Ui,
    settings: &mut ExportSettings,
    kind: ExportKind,
    back_docs: &[(String, String, bool)],
    toggles: &mut Vec<(String, bool)>,
) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Body & back matter");
            ui.add_space(5.0);
            check(
                ui,
                settings,
                |s| &mut s.include_scene_numbers,
                "Scene numbers",
            );
            check(ui, settings, |s| &mut s.include_synopsis, "Include synopses");
            check(ui, settings, |s| &mut s.include_notes, "Include author notes");
            ui.add_space(7.0);
            hairline(ui);
            ui.add_space(7.0);
            section_label(ui, "Back matter");
            ui.add_space(3.0);
            matter_list(ui, back_docs, toggles);
            ui.add_space(7.0);
            hairline(ui);
            ui.add_space(7.0);
            section_label(ui, "Draft");
            ui.add_space(3.0);
            ui.label(
                RichText::new(format!(
                    "{} is compiled from the binder. Script media keep their element types.",
                    kind.label()
                ))
                .font(ui_small())
                .color(FAINT),
            );
        });
}

fn matter_list(
    ui: &mut egui::Ui,
    docs: &[(String, String, bool)],
    toggles: &mut Vec<(String, bool)>,
) {
    if docs.is_empty() {
        ui.label(RichText::new("(none)").font(ui_small()).color(FAINT));
        return;
    }
    for (id, title, include) in docs {
        let mut value = *include;
        if ui.checkbox(&mut value, title).changed() {
            toggles.push((id.clone(), value));
        }
    }
}

fn check(
    ui: &mut egui::Ui,
    settings: &mut ExportSettings,
    field: impl Fn(&mut ExportSettings) -> &mut bool,
    label: &str,
) {
    let mut value = *field(settings);
    if ui.checkbox(&mut value, label).changed() {
        *field(settings) = value;
    }
}

fn preview_panel(app: &PortmanteauApp, ui: &mut egui::Ui, settings: &ExportSettings) {
    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, "What will be compiled");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    meta_line(ui, &["order follows the binder"]);
                });
            });
            ui.add_space(5.0);
            let plan = app
                .ws
                .project
                .compile_plan_for(settings, app.selected.as_deref());
            ui.horizontal(|ui| {
                pill(ui, plan.kind.label(), format_color(settings.kind), PANEL_2);
                if settings.each_separately && plan.kind.is_script() {
                    let groups = app.ws.project.compile_groups(&plan);
                    ui.label(
                        RichText::new(format!(
                            "{} separate file{}",
                            groups.len(),
                            if groups.len() == 1 { "" } else { "s" }
                        ))
                        .font(ui_small())
                        .color(FAINT),
                    );
                }
            });
            ui.add_space(4.0);
            let docs = &plan.included;
            for d in docs.iter().take(14) {
                ui.horizontal(|ui| {
                    let glyph = if d.medium.is_script() { "▶" } else { "¶" };
                    ui.label(
                        RichText::new(glyph)
                            .font(ui_small())
                            .color(if d.medium.is_script() { PURPLE_INK } else { ACCENT_INK }),
                    );
                    ui.label(RichText::new(&d.title).font(ui_small()).color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} w · {} pp", d.words(), d.pages()))
                                .font(mono(8.5))
                                .color(FAINT),
                        );
                    });
                });
            }
            if docs.is_empty() {
                ui.label(
                    RichText::new("Nothing of this kind is included in the scope.")
                        .font(ui_small())
                        .color(FAINT),
                );
            }
            if !plan.skipped.is_empty() {
                ui.add_space(6.0);
                hairline(ui);
                ui.add_space(5.0);
                ui.label(
                    RichText::new(format!("Skipped ({})", plan.skipped.len()))
                        .font(ui_small())
                        .color(FAINT),
                );
                for skipped in plan.skipped.iter().take(4) {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&skipped.doc.title)
                                .font(ui_small())
                                .color(FAINT),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(&skipped.reason)
                                    .font(mono(8.0))
                                    .color(FAINT),
                            );
                        });
                    });
                }
                if plan.skipped.len() > 4 {
                    ui.label(
                        RichText::new(format!("… and {} more", plan.skipped.len() - 4))
                            .font(ui_small())
                            .color(FAINT),
                    );
                }
            }
            if docs.len() > 14 {
                ui.label(
                    RichText::new(format!("… and {} more sections", docs.len() - 14))
                        .font(ui_small())
                        .color(FAINT),
                );
            }
        });
}

#[derive(Clone, Default)]
struct PageEstimate {
    fingerprint: String,
    pages: usize,
}

/// The PDF page count comes from the real layout, not a rough estimate, but the
/// Compile screen redraws every frame; keep the answer until the project or the
/// settings change.
#[allow(deprecated)]
fn page_estimate(ctx: &egui::Context, project: &Project, settings: &ExportSettings) -> u32 {
    let fingerprint = format!(
        "{}|{}|{}",
        project.words(),
        project.documents.len(),
        serde_json::to_string(settings).unwrap_or_default()
    );
    ctx.data_mut(|data| {
        let cache = data
            .get_temp_mut_or_default::<PageEstimate>(egui::Id::new("compile_page_estimate"));
        if cache.fingerprint != fingerprint {
            cache.pages = portmanteau_export::pdf_page_count(project, settings);
            cache.fingerprint = fingerprint;
        }
        cache.pages as u32
    })
}

fn run_export(app: &mut PortmanteauApp, settings: &ExportSettings) {
    let target = resolved_export_path(&app.ws.project, settings);
    let payload = serde_json::json!({
        "format": settings.kind.label(),
        "path": target,
        "paper": settings.paper.label(),
        "font": settings.font,
        "font_size": settings.font_size,
        "line_spacing": settings.line_spacing,
        "header_footer": serde_json::to_value(&settings.header_footer)
            .unwrap_or(serde_json::Value::Null),
        "cover_image": settings.cover_image,
        "include_title_page": settings.include_title_page,
        "include_toc": settings.include_toc,
        "include_chapter_numbers": settings.include_chapter_numbers,
        "include_scene_numbers": settings.include_scene_numbers,
        "include_synopsis": settings.include_synopsis,
        "include_notes": settings.include_notes,
        "compile_type": settings.compile_type.map(|t| t.key()).unwrap_or("auto"),
        "scope": match &settings.scope {
            CompileScope::Draft => "draft".to_owned(),
            CompileScope::Subtree(id) => id.clone(),
            CompileScope::Documents(ids) => ids.first().cloned().unwrap_or_default(),
        },
        "each_separately": settings.each_separately
    });
    let res = app.ws.execute("export.run", payload);
    // `export.run` records the full file path in the format's output_path, even
    // if the write later fails. Put the folder-only settings back so the Folder
    // field keeps showing the folder and the next compile cannot join the
    // filename twice.
    app.ws.project.set_settings(settings.clone());
    app.ws.project.export = settings.clone();
    if res.ok {
        let path = res
            .data
            .get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&target));
        // The toast names the file and opens it: a compile that writes
        // somewhere the writer cannot find is indistinguishable from one that
        // did nothing.
        app.toast_path(
            "Compile finished",
            res.message.clone(),
            path.clone(),
            ToastKind::Ok,
        );
        app.ws.last_message = format!("{} → {}", res.message, path.display());
    } else {
        app.toast("Compile failed", res.message, ToastKind::Error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Role, Vec2};
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The button in the Compile panel must actually write the file, not just
    /// switch screens: this drives the real view with a pointer click and then
    /// looks on disk.
    #[test]
    fn tilde_paths_expand_instead_of_creating_a_literal_tilde_folder() {
        let app = PortmanteauApp::new(None);
        let mut settings = app.ws.project.settings_for(ExportKind::Pdf);
        settings.output_path = "~/portmanteau-tilde-check".into();
        settings.output_filename = "x.pdf".into();
        let path = resolved_export_path(&app.ws.project, &settings);
        if let Ok(home) = std::env::var("HOME") {
            assert!(path.starts_with(&format!("{home}/")), "{path}");
        }
        assert!(!path.contains("/~/"), "{path}");
    }

    #[test]
    fn compile_button_writes_the_pdf() {
        let mut app = PortmanteauApp::new(None);
        let dir = std::env::temp_dir().join("portmanteau-compile-click");
        let _ = std::fs::remove_dir_all(&dir);
        let mut settings = app.ws.project.settings_for(ExportKind::Pdf);
        settings.output_path = dir.display().to_string();
        settings.output_filename = "click.pdf".into();
        app.ws.project.set_settings(settings);
        app.compile_tab = crate::app::CompileTab::Format(ExportKind::Pdf);

        let app = Rc::new(RefCell::new(app));
        let ui_app = Rc::clone(&app);
        let mut harness = Harness::builder()
            .with_size(Vec2::new(1440.0, 930.0))
            .with_accessibility_check(false)
            .allow_missing_glyphs()
            .build_ui(move |ui| {
                show(&mut ui_app.borrow_mut(), ui);
            });
        harness.run_steps(3);
        harness
            .get_by_role_and_label(Role::Button, "Compile PDF  ▸")
            .click();
        harness.run_steps(2);

        let written = dir.join("click.pdf");

        let app = app.borrow();
        assert!(
            app.ws.last_message.contains("PDF exported"),
            "the click should run the export, last message: {:?}",
            app.ws.last_message
        );
        assert!(
            written.is_file(),
            "the PDF should be on disk at {written:?}"
        );
        let toast = app.toast.as_ref().expect("a finished compile reports itself");
        assert!(matches!(toast.kind, ToastKind::Ok));
        assert_eq!(
            toast.path.as_deref(),
            Some(written.as_path()),
            "the toast should carry the file it wrote so the path can be clicked"
        );
    }

    #[test]
    fn ticking_custom_margins_seeds_the_automatic_values_and_unticking_returns_to_auto() {
        let mut app = PortmanteauApp::new(None);
        app.compile_tab = crate::app::CompileTab::Format(ExportKind::Pdf);
        let before = app.ws.project.settings_for(ExportKind::Pdf);
        assert!(before.margins.is_none(), "margins start automatic");

        let app = Rc::new(RefCell::new(app));
        let ui_app = Rc::clone(&app);
        let mut harness = Harness::builder()
            .with_size(Vec2::new(1440.0, 930.0))
            .with_accessibility_check(false)
            .allow_missing_glyphs()
            .build_ui(move |ui| {
                show(&mut ui_app.borrow_mut(), ui);
            });
        harness.run_steps(3);
        harness
            .get_by_role_and_label(Role::CheckBox, "Custom margins")
            .click_accesskit();
        harness.run_steps(3);
        let seeded = app
            .borrow()
            .ws
            .project
            .settings_for(ExportKind::Pdf)
            .margins
            .expect("ticking the box makes the margins custom");
        assert!((seeded.left - 20.0 * 72.0 / 25.4).abs() < 0.01, "prose seeds the book margin");
        // The four edges are now on screen.
        for label in ["Left", "Right", "Top", "Bottom"] {
            assert!(
                harness.query_all_by_label(label).next().is_some(),
                "the {label} margin field should be on screen"
            );
        }

        harness
            .get_by_role_and_label(Role::CheckBox, "Custom margins")
            .click_accesskit();
        harness.run_steps(3);
        assert!(
            app.borrow().ws.project.settings_for(ExportKind::Pdf).margins.is_none(),
            "unticking returns to automatic"
        );
    }
}
