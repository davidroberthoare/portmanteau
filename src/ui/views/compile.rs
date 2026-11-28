//! Compile: choose an output format, decide what goes in the book, and write a
//! real file — PDF, EPUB, DOCX, Final Draft, Fountain, HTML, Markdown,
//! plain text or a complete project bundle. Editor display settings live in the
//! separate Settings workspace.

use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};
use portmanteau_core::{
    CompileScope, CompileType, DocKind, ElementStyles, ExportKind, ExportSettings,
    NumberFormat, PageMargins, PaperSize, Project, ProjectKind, RegionMode,
};

use crate::app::{CompileSection, CompileTab, PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::i18n::tr;
use crate::ui::views::{
    content_frame, manuscript_preview_navigator, meta_line, page_head, quiet_button,
    style_editor, FONT_FAMILIES,
};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        tr("Finish"),
        tr("Compile"),
        tr("One manuscript, many finish lines. Pick an output format, then shape what goes in the book."),
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

    content_frame().show(ui, |ui| {
        let height = (ui.available_height() - 4.0).max(420.0);
        ui.horizontal_top(|ui| {
            // ---- Vertical tab rail --------------------------------------
            ui.allocate_ui_with_layout(
                Vec2::new(178.0, height),
                Layout::top_down(Align::Min),
                |ui| {
                    section_label(ui, tr("Output"));
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
                    output_tab(
                        app,
                        ui,
                        kind,
                        &mut settings,
                        selected_id.as_deref(),
                        &mut matter_toggles,
                        &mut compile,
                    );
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
                                "compile",
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
        let _ = run_export(app, &effective);
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
        ExportKind::Txt => "TXT",
        ExportKind::Bundle => "Bundle",
    }
}

fn tab_button(ui: &mut egui::Ui, label: &str, color: Color32, selected: bool) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(7), ACCENT_SOFT());
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(7),
            Stroke::new(1.0, ACCENT().gamma_multiply(0.75)),
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(7), RAISED());
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
        if selected { TEXT() } else { MUTED() },
    );
    resp.clicked()
}

fn format_color(kind: ExportKind) -> Color32 {
    match kind {
        ExportKind::Pdf => ACCENT(),
        ExportKind::Epub => SUCCESS(),
        ExportKind::Docx => INFO(),
        ExportKind::Fdx => SPECIAL(),
        ExportKind::Fountain => SPECIAL(),
        ExportKind::Html => WARNING(),
        ExportKind::Markdown => MUTED(),
        ExportKind::Txt => MUTED(),
        ExportKind::Bundle => MUTED(),
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
    selected_id: Option<&str>,
    toggles: &mut Vec<(String, bool)>,
    compile: &mut bool,
) {
    // Pinned above the tabs: which format this is, where it goes, and the
    // Compile button, so none of them scroll away while settings are edited.
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(kind.label()).font(serif(21.0)).color(TEXT()));
            ui.label(
                RichText::new(kind.description())
                    .font(ui_small())
                    .color(MUTED()),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            pill(
                ui,
                &format!(".{}", kind.extension()),
                format_color(kind),
                RAISED(),
            );
        });
    });
    ui.add_space(8.0);
    output_panel(app, ui, settings);
    ui.add_space(8.0);
    action_panel(app, ui, settings, compile);
    ui.add_space(10.0);
    section_tabs(ui, &mut app.compile_section);
    ui.add_space(8.0);

    let compile_type = app
        .ws
        .project
        .compile_plan_for(settings, selected_id)
        .kind;
    ScrollArea::vertical()
        .id_salt("compile_params")
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match app.compile_section {
                CompileSection::Contents => {
                    contents_panel(app, ui, settings, selected_id, toggles);
                    ui.add_space(10.0);
                    if compile_type.is_script() {
                        script_panel(ui, settings, kind, compile_type);
                    } else {
                        include_panel(ui, settings, kind);
                    }
                }
                CompileSection::Layout => {
                    type_panel(ui, kind, settings, compile_type, &app.ws.project.cover_image, &app.ws);
                    if kind == ExportKind::Pdf {
                        ui.add_space(8.0);
                        header_footer_panel(ui, settings, compile_type);
                    }
                    ui.add_space(8.0);
                    reset_row(app, ui, settings);
                }
                CompileSection::Styles => {
                    style_panel(ui, &mut settings.styles, &mut settings.font, compile_type);
                    ui.add_space(8.0);
                    reset_row(app, ui, settings);
                }
            }
            ui.add_space(12.0);
        });
}

/// The three groups of settings: what goes in, how the pages look, and the
/// type styles.
fn section_tabs(ui: &mut egui::Ui, current: &mut CompileSection) {
    ui.horizontal(|ui| {
        for (section, label) in [
            (CompileSection::Contents, "Contents"),
            (CompileSection::Layout, "Layout"),
            (CompileSection::Styles, "Styles"),
        ] {
            let selected = *current == section;
            let text = RichText::new(label).font(ui_body()).color(if selected {
                ON_ACCENT()
            } else {
                MUTED()
            });
            let button = egui::Button::new(text)
                .fill(if selected { ACCENT() } else { PANEL() })
                .stroke(Stroke::new(1.0, if selected { ACCENT() } else { BORDER() }))
                .corner_radius(CornerRadius::same(6))
                .min_size(Vec2::new(86.0, 28.0));
            if ui.add(button).clicked() {
                *current = section;
            }
        }
    });
}

fn reset_row(app: &PortmanteauApp, ui: &mut egui::Ui, settings: &mut ExportSettings) {
    ui.horizontal(|ui| {
        if quiet_button(ui, tr("Reset this format to base style")).clicked() {
            *settings = seeded_settings(&app.ws.project.styles, app.ws.project.kind, settings.kind);
        }
        ui.label(
            RichText::new(tr("Seeds from the project-wide element styles."))
                .font(ui_small())
                .color(MUTED()),
        );
    });
}

fn output_panel(app: &PortmanteauApp, ui: &mut egui::Ui, settings: &mut ExportSettings) {
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, tr("Where it goes"));
            ui.add_space(4.0);
            let default_name = portmanteau_export::default_filename(&app.ws.project, settings.kind);
            egui::Grid::new("export_location")
                .num_columns(2)
                .spacing([8.0, 5.0])
                .min_col_width(48.0)
                .show(ui, |ui| {
                    // In the browser there is no folder: the file downloads.
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.label(RichText::new(tr("Folder")).font(ui_small()).color(MUTED()));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // The button keeps its own width at the right; the
                            // field takes the rest.
                            if ui
                                .button(RichText::new(tr("Browse…")).font(ui_small()))
                                .on_hover_text(tr("Choose the folder the export is written to"))
                                .clicked()
                            {
                                if let Some(folder) = choose_export_folder(&settings.output_path) {
                                    settings.output_path = folder;
                                }
                            }
                            output_field(
                                ui,
                                &mut settings.output_path,
                                "Folder for the export (blank = ./exports)",
                            );
                        });
                        ui.end_row();

                    }

                    ui.label(RichText::new(tr("Filename")).font(ui_small()).color(MUTED()));
                    output_field(ui, &mut settings.output_filename, &default_name);
                    ui.end_row();
                });
            ui.add_space(5.0);
            #[cfg(target_arch = "wasm32")]
            {
                ui.add_space(5.0);
                ui.label(
                    RichText::new(tr("The compiled file downloads through your browser."))
                        .font(ui_small())
                        .color(MUTED()),
                );
            }
        });
}

/// One path field in the "Where it goes" panel, framed like a form input.
fn output_field(ui: &mut egui::Ui, value: &mut String, hint: &str) {
    Frame::new()
        .fill(BG())
        .stroke(Stroke::new(1.0, BORDER()))
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

/// Ask the desktop for a folder, starting where the field already points.
/// `None` when the writer cancels.
#[cfg(not(target_arch = "wasm32"))]
fn choose_export_folder(current: &str) -> Option<String> {
    let mut dialog = crate::platform::FileDialog::new().set_title("Export folder");
    let start = expand_tilde(current);
    if start.is_dir() {
        dialog = dialog.set_directory(start);
    }
    dialog
        .pick_folder()
        .map(|folder| folder.to_string_lossy().into_owned())
}

fn action_panel(
    app: &PortmanteauApp,
    ui: &mut egui::Ui,
    settings: &ExportSettings,
    compile: &mut bool,
) {
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(16, 13))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let label = format!("Compile {}  ▸", settings.kind.label());
                let btn = egui::Button::new(
                    RichText::new(label)
                        .font(ui_body())
                        .color(ON_ACCENT()),
                )
                .fill(ACCENT())
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
                            let pages = page_estimate(ui.ctx(), &app.ws.project, &effective);
                            format!(
                                "{} words · {} pages estimated",
                                plan.included.iter().map(|d| d.words()).sum::<u32>(),
                                pages
                            )
                        })
                        .font(ui_small())
                        .color(MUTED()),
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
                        .color(MUTED()),
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
                        .circle_filled(dot_rect.center(), 3.5, SUCCESS());
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Last export · {} · {:.1} KB",
                                info.format,
                                info.bytes as f32 / 1024.0
                            ))
                            .font(ui_small())
                            .color(SUCCESS()),
                        );
                        // The one place the output path is shown: a live link
                        // that opens the file.
                        let path = Path::new(&info.path);
                        let link = ui.link(
                            RichText::new(crate::app::short_path(path))
                                .font(mono(9.0))
                                .color(ACCENT()),
                        );
                        if link.on_hover_text(tr("Open the file")).clicked() {
                            crate::app::open_path(path);
                        }
                    });
                });
            }
        });
}

fn style_panel(ui: &mut egui::Ui, styles: &mut ElementStyles, global_font: &mut String, compile_type: CompileType) {
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, tr("Global font"));
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Font")).font(ui_small()).color(MUTED()));
                if compile_type.is_script() {
                    // A script's face comes from its own styles below, so a
                    // format-wide font choice would silently do nothing.
                    ui.label(RichText::new(tr("Scripts set their font in each style below")).font(ui_small()).color(MUTED()));
                } else {
                    font_picker(ui, global_font);
                }
            });
            ui.label(
                RichText::new(tr("Used by every style whose font is left on (inherit)."))
                    .font(ui_small())
                    .color(MUTED()),
            );
            ui.add_space(10.0);
            section_label(ui, tr("Type styles for this format"));
            ui.label(
                RichText::new(
                    "Pick a type of manuscript and a style, then set its font and paragraph. The global defaults live in Settings → Display.",
                )
                .font(ui_small())
                .color(MUTED()),
            );
            ui.add_space(8.0);
            style_editor(ui, styles, "compile", compile_type);
        });
}

fn type_panel(
    ui: &mut egui::Ui,
    kind: ExportKind,
    settings: &mut ExportSettings,
    compile_type: CompileType,
    project_cover: &str,
    ws: &portmanteau_api::Workspace,
) {
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, tr("Page & type"));
            ui.add_space(5.0);

            if matches!(
                kind,
                ExportKind::Pdf | ExportKind::Docx
            ) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("Paper")).font(ui_small()).color(MUTED()));
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
                if settings.paper == PaperSize::Custom {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        for (label, value) in [
                            ("Width", &mut settings.custom_paper_mm.0),
                            ("Height", &mut settings.custom_paper_mm.1),
                        ] {
                            ui.label(RichText::new(label).font(ui_small()).color(MUTED()));
                            ui.add(
                                egui::DragValue::new(value)
                                    .speed(0.5)
                                    .range(50.0..=600.0)
                                    .fixed_decimals(1)
                                    .suffix(" mm"),
                            );
                        }
                    });
                }
                ui.add_space(4.0);
            }

            margins_row(ui, kind, settings, compile_type);

            if kind == ExportKind::Epub || (kind == ExportKind::Pdf && !compile_type.is_script()) {
                ui.add_space(7.0);
                check(ui, settings, |s| &mut s.include_cover, "Include a cover");
                if settings.include_cover {
                    let file_name = |path: &str| {
                        Path::new(path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.to_owned())
                    };
                    let own = settings.cover_image.trim().to_owned();
                    ui.horizontal_wrapped(|ui| {
                        if !own.is_empty() {
                            ui.label(
                                RichText::new(format!("This format uses {}", file_name(&own)))
                                    .font(ui_small())
                                    .color(MUTED()),
                            );
                            if quiet_button(ui, tr("Use the project cover")).clicked() {
                                settings.cover_image.clear();
                            }
                        } else if project_cover.trim().is_empty() {
                            ui.label(
                                RichText::new(tr("No cover yet. Add one on the Project page, or pick one for this format."))
                                    .font(ui_small())
                                    .color(MUTED()),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!("Using the project cover, {}", file_name(project_cover)))
                                    .font(ui_small())
                                    .color(MUTED()),
                            );
                        }
                        if quiet_button(ui, tr("Choose another…")).clicked() {
                            if let Some(path) = crate::platform::FileDialog::new()
                                .set_title("Cover picture")
                                .add_filter("Pictures", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                // A saved project keeps its own copy, so the export
                                // never reads a picture from outside the project.
                                let stem = format!("cover-{}", kind.label().to_ascii_lowercase());
                                settings.cover_image = ws
                                    .store_project_image(&path, &stem)
                                    .unwrap_or_else(|_| path.display().to_string());
                            }
                        }
                    });
                }
            }
        });
}

/// What "Custom margins" starts from, so ticking the box changes nothing until
/// a value is edited: the automatic margins for this manuscript type, in points.
fn automatic_margins(compile_type: CompileType, mirrored: bool) -> PageMargins {
    if compile_type.is_script() {
        // Film and TV: a 1.5" binding edge, 1" elsewhere (before any paper scaling).
        PageMargins::default()
    } else {
        // The book default: 20 mm at the sides, 22 mm top and bottom.
        let mm = |v: f32| v * 72.0 / 25.4;
        let (inside, outside) = if mirrored { (23.0, 17.0) } else { (20.0, 20.0) };
        let vertical = mm(22.0);
        PageMargins {
            left: mm(inside),
            right: mm(outside),
            top: vertical,
            bottom: vertical,
        }
    }
}

/// Page margins, in inches. Off means automatic. An EPUB reflows, so only its
/// side margins apply; the paged formats use all four. A prose book in a paged
/// format can mirror them: the left field becomes the inside (binding) edge and
/// the right one the outside edge, swapped on left-hand pages.
fn margins_row(
    ui: &mut egui::Ui,
    kind: ExportKind,
    settings: &mut ExportSettings,
    compile_type: CompileType,
) {
    let can_mirror = !compile_type.is_script()
        && matches!(kind, ExportKind::Pdf | ExportKind::Docx);
    if can_mirror {
        ui.checkbox(&mut settings.mirror_margins, tr("Book margins (inside and outside)"))
            .on_hover_text(tr("Facing pages: the inside margin sits at the spine of every page"));
    } else {
        settings.mirror_margins = false;
    }
    if kind == ExportKind::Pdf && !compile_type.is_script() {
        ui.checkbox(&mut settings.chapters_start_right, tr("Start chapters on right-hand pages"))
            .on_hover_text(tr("Leaves a blank left-hand page when a chapter would otherwise open on one"));
    } else {
        settings.chapters_start_right = false;
    }
    let mirrored = settings.mirror_margins;
    let mut custom = settings.margins.is_some();
    if ui
        .checkbox(&mut custom, "Custom margins")
        .on_hover_text(tr("Off: book or script margins for the chosen paper"))
        .changed()
    {
        settings.margins = custom.then(|| automatic_margins(compile_type, mirrored));
    }
    let Some(margins) = settings.margins.as_mut() else {
        ui.add_space(4.0);
        return;
    };
    let reflows = kind == ExportKind::Epub;
    let (left_label, right_label) = if mirrored {
        ("Inside", "Outside")
    } else {
        ("Left", "Right")
    };
    ui.indent("custom_margins", |ui| {
        ui.horizontal(|ui| {
            margin_field(ui, left_label, &mut margins.left);
            margin_field(ui, right_label, &mut margins.right);
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
    ui.label(RichText::new(label).font(ui_small()).color(MUTED()));
    let field = egui::DragValue::new(&mut inches)
        .speed(0.02)
        .range(0.0..=3.0)
        .fixed_decimals(2)
        .suffix(" in");
    if ui.add(field).changed() {
        *points = inches * 72.0;
    }
}

/// Which stretch of the book the Headers & footers editor is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HfRegion {
    Body,
    Front,
    Back,
}

/// Which template within the stretch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HfVariant {
    Odd,
    Even,
    First,
}

fn number_format_combo(ui: &mut egui::Ui, id: &str, value: &mut NumberFormat) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(RichText::new(value.label()).font(ui_small()))
        .width(96.0)
        .show_ui(ui, |ui| {
            for format in NumberFormat::all() {
                ui.selectable_value(value, format, format.label());
            }
        });
}

/// Headers, footers and page numbers. A book gets templates of six slots with
/// fields, optional left/right-hand and first-page variants, and its front
/// matter, body and back matter can each have their own. A script only has
/// page numbers (they print at the top right).
#[allow(deprecated)]
fn header_footer_panel(ui: &mut egui::Ui, settings: &mut ExportSettings, compile_type: CompileType) {
    // Edit a local clone, then commit once the draw pass is over.
    let mut hf = settings.header_footer.clone();
    let state_id = egui::Id::new("hf_editor_state");
    let (mut region, mut variant, mut focused): (HfRegion, HfVariant, Option<(usize, usize)>) = ui
        .data_mut(|d| d.get_temp(state_id))
        .unwrap_or((HfRegion::Body, HfVariant::Odd, None));

    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, tr("Headers & footers"));
            ui.add_space(5.0);

            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut hf.page_numbers, tr("Page numbers"));
                if hf.page_numbers {
                    ui.add_space(6.0);
                    number_format_combo(ui, "hf_number_format", &mut hf.number_format);
                    ui.label(RichText::new(tr("first page is")).font(ui_small()).color(MUTED()));
                    ui.add(egui::DragValue::new(&mut hf.start_number).speed(1.0).range(1..=9999));
                }
            });

            if compile_type.is_script() {
                ui.label(
                    RichText::new(tr("A script prints its page number at the top right, from page 2."))
                        .font(ui_small())
                        .color(MUTED()),
                );
                return;
            }

            ui.checkbox(&mut hf.different_odd_even, tr("Different left-hand and right-hand pages"));
            ui.checkbox(&mut hf.different_first_page, tr("Different first page of each part"));
            ui.checkbox(
                &mut hf.no_header_on_chapter_openers,
                "No header on the page where a chapter opens",
            );
            ui.checkbox(&mut hf.restart_body_numbering, tr("Start the body at page 1"));

            ui.add_space(6.0);
            hairline(ui);
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Edit")).font(ui_small()).color(MUTED()));
                for (r, label) in [
                    (HfRegion::Front, "Front matter"),
                    (HfRegion::Body, "Body"),
                    (HfRegion::Back, "Back matter"),
                ] {
                    ui.selectable_value(&mut region, r, label);
                }
            });
            ui.add_space(4.0);

            // Front and back matter choose how they get their templates.
            let mut editable = true;
            if region != HfRegion::Body {
                let r = if region == HfRegion::Front { &mut hf.front } else { &mut hf.back };
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt(("hf_region_mode", region as u8))
                        .selected_text(RichText::new(r.mode.label()).font(ui_small()))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for mode in RegionMode::all() {
                                ui.selectable_value(&mut r.mode, mode, mode.label());
                            }
                        });
                    number_format_combo(ui, &format!("hf_region_numbers_{}", region as u8), &mut r.number_format);
                    ui.checkbox(&mut r.restart_numbering, tr("Restart at 1"));
                });
                editable = r.mode == RegionMode::Custom;
                if !editable {
                    ui.label(
                        RichText::new(match r.mode {
                            RegionMode::None => "No headers or footers here.",
                            _ => "Uses the body's headers and footers.",
                        })
                        .font(ui_small())
                        .color(MUTED()),
                    );
                }
            }

            if editable {
                // Only the variants that are switched on can be edited.
                let mut variants = vec![(
                    HfVariant::Odd,
                    if hf.different_odd_even { "Right-hand pages" } else { "All pages" },
                )];
                if hf.different_odd_even {
                    variants.push((HfVariant::Even, "Left-hand pages"));
                }
                if hf.different_first_page {
                    variants.push((HfVariant::First, "First page"));
                }
                if !variants.iter().any(|(v, _)| *v == variant) {
                    variant = HfVariant::Odd;
                }
                ui.horizontal(|ui| {
                    for (v, label) in &variants {
                        ui.selectable_value(&mut variant, *v, *label);
                    }
                });
                ui.add_space(4.0);
                let furniture = match region {
                    HfRegion::Body => &mut hf.body,
                    HfRegion::Front => &mut hf.front.furniture,
                    HfRegion::Back => &mut hf.back.furniture,
                };
                let slots = match variant {
                    HfVariant::Odd => &mut furniture.odd,
                    HfVariant::Even => &mut furniture.even,
                    HfVariant::First => &mut furniture.first,
                };
                egui::Grid::new(("hf_slots", region as u8, variant as u8))
                    .num_columns(4)
                    .spacing([6.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("");
                        for label in ["Left", "Centre", "Right"] {
                            ui.label(RichText::new(label).font(ui_small()).color(MUTED()));
                        }
                        ui.end_row();
                        for (row, (label, texts)) in
                            [("Header", &mut slots.header), ("Footer", &mut slots.footer)]
                                .into_iter()
                                .enumerate()
                        {
                            ui.label(RichText::new(label).font(ui_small()).color(MUTED()));
                            for (column, text) in texts.iter_mut().enumerate() {
                                let response = ui.add(
                                    TextEdit::singleline(text)
                                        .font(ui_small())
                                        .desired_width(84.0),
                                );
                                if response.has_focus() {
                                    focused = Some((row, column));
                                }
                            }
                            ui.end_row();
                        }
                    });
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("hf_insert_field")
                        .selected_text(RichText::new(tr("Insert field…")).font(ui_small()))
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for (token, what) in portmanteau_core::HEADER_FIELDS {
                                if ui.selectable_label(false, format!("{token}   {what}")).clicked() {
                                    let (row, column) = focused.unwrap_or((1, 1));
                                    let target = if row == 0 {
                                        &mut slots.header[column]
                                    } else {
                                        &mut slots.footer[column]
                                    };
                                    target.push_str(token);
                                }
                            }
                        });
                    ui.label(
                        RichText::new(tr("adds to the box you last clicked"))
                            .font(ui_small())
                            .color(MUTED()),
                    );
                });
            }

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Header from top")).font(ui_small()).color(MUTED()));
                ui.add(egui::DragValue::new(&mut hf.header_spacing).speed(0.5).range(6.0..=120.0).suffix(" pt"));
                ui.label(RichText::new(tr("footer from bottom")).font(ui_small()).color(MUTED()));
                ui.add(egui::DragValue::new(&mut hf.footer_spacing).speed(0.5).range(6.0..=120.0).suffix(" pt"));
                ui.label(RichText::new(tr("size")).font(ui_small()).color(MUTED()));
                ui.add(egui::DragValue::new(&mut hf.size).speed(0.1).range(5.0..=24.0).suffix(" pt"));
            });
        });

    ui.data_mut(|d| d.insert_temp(state_id, (region, variant, focused)));
    settings.header_footer = hf;
}

fn font_picker(ui: &mut egui::Ui, font: &mut String) {
    let selected = if font.trim().is_empty() {
        "(program default)".to_string()
    } else {
        font.clone()
    };
    egui::ComboBox::from_id_salt("compile_font")
        .selected_text(RichText::new(selected).font(ui_small()).color(TEXT()))
        .width(170.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(font, String::new(), "(program default)");
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
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, tr("Script pages"));
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
            if !matches!(kind, ExportKind::Txt | ExportKind::Markdown) {
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
                .color(MUTED()),
            );
        });
}

/// What is in the compile: the kind of manuscript (asked only when the project
/// really mixes kinds), whole book or one part, and one list of every document
/// with a tick for whether it goes in. Front and back matter are ordinary
/// entries of that list.
fn contents_panel(
    app: &PortmanteauApp,
    ui: &mut egui::Ui,
    settings: &mut ExportSettings,
    selected_id: Option<&str>,
    toggles: &mut Vec<(String, bool)>,
) {
    let project = &app.ws.project;
    let title_of = |id: &str| project.document(id).map(|d| d.title.clone());
    let auto_kind = project.compile_plan_for(settings, selected_id).kind;
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, tr("What goes in"));
            ui.add_space(5.0);

            // The kind of manuscript follows the documents. Only a project
            // that holds more than one kind is ever asked.
            let types = project.compile_types_in(&settings.scope);
            if types.len() > 1 {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("Compile as")).font(ui_small()).color(MUTED()));
                    let shown = settings.compile_type.unwrap_or(auto_kind).label();
                    egui::ComboBox::from_id_salt("compile_type")
                        .selected_text(RichText::new(shown).font(ui_small()))
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for t in types {
                                ui.selectable_value(&mut settings.compile_type, Some(t), t.label());
                            }
                        });
                });
                ui.label(
                    RichText::new(tr("This project has more than one kind of document; each export is one kind."))
                        .font(ui_small())
                        .color(MUTED()),
                );
                ui.add_space(4.0);
            } else {
                settings.compile_type = None;
            }

            let whole = settings.scope == CompileScope::Draft;
            if ui.radio(whole, "Whole book").clicked() {
                settings.scope = CompileScope::Draft;
            }
            let part_id = match &settings.scope {
                CompileScope::Subtree(id) => Some(id.clone()),
                _ => selected_id.map(str::to_owned),
            };
            let part_title = part_id.as_deref().and_then(title_of);
            let part_active = matches!(settings.scope, CompileScope::Subtree(_));
            let part_label = match &part_title {
                Some(t) => format!("Only “{t}” and what is inside it"),
                None => "Only part of the book".to_owned(),
            };
            let clicked = ui
                .add_enabled(part_title.is_some(), egui::RadioButton::new(part_active, part_label))
                .on_disabled_hover_text("Select an item in the binder first")
                .clicked();
            if clicked {
                if let Some(id) = part_id {
                    settings.scope = CompileScope::Subtree(id);
                }
            }
            if part_active {
                if let (CompileScope::Subtree(current), Some(sel)) = (&settings.scope, selected_id) {
                    if current != sel {
                        if let Some(title) = title_of(sel) {
                            if ui
                                .small_button(format!("Use “{title}” instead"))
                                .on_hover_text(tr("Compile the item now selected in the binder"))
                                .clicked()
                            {
                                settings.scope = CompileScope::Subtree(sel.to_owned());
                            }
                        }
                    }
                }
            }

            ui.add_space(7.0);
            hairline(ui);
            ui.add_space(7.0);
            document_list(app, ui, settings, toggles);
        });
}

/// Every document the compile could draw from, in binder order, each with a
/// tick. Documents the chosen kind cannot take are listed greyed with why.
fn document_list(
    app: &PortmanteauApp,
    ui: &mut egui::Ui,
    settings: &ExportSettings,
    toggles: &mut Vec<(String, bool)>,
) {
    let project = &app.ws.project;
    let plan = project.compile_plan_for(settings, app.selected.as_deref());
    let reasons: std::collections::HashMap<&str, &str> = plan
        .skipped
        .iter()
        .map(|s| (s.doc.id.as_str(), s.reason.as_str()))
        .collect();
    let included: std::collections::HashSet<&str> =
        plan.included.iter().map(|d| d.id.as_str()).collect();
    let counts = crate::ui::preview::page_counts(ui, project);
    let mut shown = 0;
    ScrollArea::vertical()
        .id_salt("compile_documents")
        .max_height(300.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for doc in project.ordered_documents() {
                let in_plan = included.contains(doc.id.as_str());
                let reason = reasons.get(doc.id.as_str()).copied();
                if !in_plan && reason.is_none() {
                    continue;
                }
                shown += 1;
                let mut depth: usize = 0;
                let mut parent = doc.parent.clone();
                while let Some(id) = parent {
                    depth += 1;
                    parent = project.document(&id).and_then(|p| p.parent.clone());
                }
                ui.horizontal(|ui| {
                    ui.add_space(depth.saturating_sub(1) as f32 * 14.0);
                    let can_toggle = in_plan || reason == Some("Excluded from compile");
                    let mut value = doc.include_in_export;
                    let text = RichText::new(&doc.title)
                        .font(ui_small())
                        .color(if in_plan { TEXT() } else { MUTED() });
                    if ui
                        .add_enabled(can_toggle, egui::Checkbox::new(&mut value, text))
                        .changed()
                    {
                        toggles.push((doc.id.clone(), value));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let note = if in_plan {
                            format!("{} w · {} pp", doc.words(), counts.label(&doc.id))
                        } else {
                            reason.unwrap_or_default().to_owned()
                        };
                        ui.label(RichText::new(note).font(mono(8.5)).color(MUTED()));
                    });
                });
            }
            if shown == 0 {
                ui.label(
                    RichText::new(tr("Nothing of this kind is in the chosen part of the book."))
                        .font(ui_small())
                        .color(MUTED()),
                );
            }
        });
}

/// The switches for a prose compile.
fn include_panel(ui: &mut egui::Ui, settings: &mut ExportSettings, kind: ExportKind) {
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, tr("Also include"));
            ui.add_space(5.0);
            let plain_text = matches!(
                kind,
                ExportKind::Txt | ExportKind::Markdown
            );
            if !plain_text {
                check(ui, settings, |s| &mut s.include_title_page, "Title page");
                check(ui, settings, |s| &mut s.include_toc, "Table of contents");
                if settings.include_toc {
                    ui.horizontal(|ui| {
                        ui.add_space(22.0);
                        ui.label(RichText::new(tr("Heading")).font(ui_small()).color(MUTED()));
                        let mut title = settings.toc_title.clone();
                        if ui
                            .add(
                                TextEdit::singleline(&mut title)
                                    .hint_text(tr("Contents"))
                                    .desired_width(150.0),
                            )
                            .changed()
                        {
                            settings.toc_title = title;
                        }
                    });
                    if kind == ExportKind::Pdf {
                        ui.horizontal(|ui| {
                            ui.add_space(22.0);
                            check(ui, settings, |s| &mut s.toc_page_numbers, "Page numbers");
                        });
                        if settings.toc_page_numbers {
                            ui.horizontal(|ui| {
                                ui.add_space(22.0);
                                check(ui, settings, |s| &mut s.toc_leaders, "Dots between title and number");
                            });
                        }
                    }
                }
                check(
                    ui,
                    settings,
                    |s| &mut s.include_chapter_numbers,
                    "Chapter numbers",
                );
            }
            check(ui, settings, |s| &mut s.include_synopsis, "Synopses");
            check(ui, settings, |s| &mut s.include_notes, "Author notes");
            if kind.writes_hyperlinks() {
                check(ui, settings, |s| &mut s.include_hyperlinks, "Write links as links");
            }
        });
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

/// Compile again with the settings the Compile screen last used for the
/// format that is selected there, without opening the screen. Says where the
/// file went; a failure is reported by `run_export`.
pub(crate) fn compile_with_last_settings(app: &mut PortmanteauApp) {
    let CompileTab::Format(kind) = app.compile_tab;
    let settings = app.ws.project.settings_for(kind);
    if let Some(path) = run_export(app, &settings) {
        // In the browser the file was downloaded; there is nothing to open.
        #[cfg(not(target_arch = "wasm32"))]
        app.toast_with_file(format!("{} compiled", kind.label()), path, ToastKind::Ok);
        #[cfg(target_arch = "wasm32")]
        app.toast(format!("{} compiled", kind.label()), path.display().to_string(), ToastKind::Ok);
    }
}

fn run_export(app: &mut PortmanteauApp, settings: &ExportSettings) -> Option<PathBuf> {
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
        "toc_title": settings.toc_title,
        "toc_page_numbers": settings.toc_page_numbers,
        "toc_leaders": settings.toc_leaders,
        "include_chapter_numbers": settings.include_chapter_numbers,
        "include_scene_numbers": settings.include_scene_numbers,
        "include_synopsis": settings.include_synopsis,
        "include_notes": settings.include_notes,
        "include_hyperlinks": settings.include_hyperlinks,
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
    #[cfg(target_arch = "wasm32")]
    for (name, bytes) in portmanteau_export::take_outbox() {
        let mime = match name.rsplit('.').next().unwrap_or("") {
            "pdf" => "application/pdf",
            "epub" => "application/epub+zip",
            "html" => "text/html",
            "txt" | "fountain" | "md" => "text/plain",
            _ => "application/octet-stream",
        };
        crate::web::download_file(&name, &bytes, mime);
    }
    if res.ok {
        let path = res
            .data
            .get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&target));
        // No toast: "Last export" in the action panel names the file and
        // opens it, so a compile that wrote somewhere unexpected is still
        // findable without a second, vanishing notice.
        app.ws.last_message = format!("{} → {}", res.message, path.display());
        Some(path)
    } else {
        app.toast(tr("Compile failed"), res.message, ToastKind::Error);
        None
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
    fn compile_with_last_settings_writes_the_selected_format_without_the_screen() {
        let dir = std::env::temp_dir().join(format!("pm-quick-compile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = PortmanteauApp::new(None);
        app.compile_tab = CompileTab::Format(ExportKind::Markdown);
        let mut settings = app.ws.project.settings_for(ExportKind::Markdown);
        settings.output_path = dir.display().to_string();
        settings.output_filename = "quick.md".into();
        app.ws.project.set_settings(settings);
        compile_with_last_settings(&mut app);
        let written = std::fs::read_to_string(dir.join("quick.md")).expect("the file is written");
        assert!(written.contains("Salt Road") || written.len() > 200, "{written}");
        let _ = std::fs::remove_dir_all(&dir);
    }

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
        assert!(app.toast.is_none(), "the compile reports in the action panel, not a toast");
        let info = app.ws.last_export.as_ref().expect("the compile records its file");
        assert_eq!(Path::new(&info.path), written.as_path());
        drop(app);
        // The one place the path appears is a live link; no copy button, no
        // second "Writes" line.
        harness.get_by_role_and_label(Role::Link, &crate::app::short_path(&written));
        assert!(harness.query_by_label("Copy path").is_none());
        assert!(harness.query_by_label_contains("Writes ").is_none());
    }

    #[test]
    fn ticking_custom_margins_seeds_the_automatic_values_and_unticking_returns_to_auto() {
        let mut app = PortmanteauApp::new(None);
        app.compile_tab = crate::app::CompileTab::Format(ExportKind::Pdf);
        app.compile_section = crate::app::CompileSection::Layout;
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

    #[test]
    fn the_three_tabs_show_their_own_settings() {
        let mut app = PortmanteauApp::new(None);
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
        // Contents: what goes in, no scope or manuscript dropdowns.
        assert!(harness.query_by_label("Whole book").is_some());
        assert!(harness.query_by_label("Scope").is_none());
        assert!(harness.query_by_label("Manuscript").is_none());

        harness.get_by_label("Layout").click();
        harness.run_steps(3);
        assert!(harness.query_by_label("Book margins (inside and outside)").is_some());
        assert!(harness.query_by_label("Different left-hand and right-hand pages").is_some());

        harness.get_by_label("Styles").click();
        harness.run_steps(3);
        assert!(harness.query_by_label("Type").is_some());
        assert!(harness.query_by_label("Style").is_some());
        assert!(harness.query_by_label("Element styles").is_none());
    }
}
