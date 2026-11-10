//! Compile: choose an output format or the Display settings, decide what goes
//! in the book, and write a real file — PDF, EPUB, DOCX, Final Draft, Fountain,
//! HTML, Markdown, RTF, plain text or a complete project bundle.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};
use portmanteau_core::{
    DocKind, ElementStyles, ExportKind, ExportSettings, PaperSize, ProjectKind,
};

use crate::app::{CompileTab, PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::views::{
    content_frame, custom_styles_editor, element_preview, element_styles_editor, meta_line,
    page_head, primary_button, quiet_button, BUILTIN_ELEMENTS, FONT_FAMILIES,
};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        "Finish",
        "Compile",
        "One manuscript, many finish lines. Pick an output format, or set how the pages look while you write.",
        |ui| {
            meta_line(
                ui,
                &[&format!(
                    "{} sections · {} words",
                    app.ws.project.manuscript_documents().len(),
                    app.ws.project.words()
                )],
            );
        },
    );

    let mut tab = app.compile_tab;
    let mut settings = match tab {
        CompileTab::Format(kind) => app.ws.project.settings_for(kind),
        CompileTab::Display => app.ws.project.settings_for(ExportKind::Pdf),
    };
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
    let mut matter_toggles: Vec<(String, bool)> = Vec::new();

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
                                }
                                ui.add_space(2.0);
                            }
                            ui.add_space(10.0);
                            hairline(ui);
                            ui.add_space(8.0);
                            if tab_button(ui, "Display", MUTED, tab == CompileTab::Display) {
                                tab = CompileTab::Display;
                            }
                        });
                },
            );
            ui.add_space(16.0);
            // ---- Content pane -------------------------------------------
            ui.vertical(|ui| {
                ScrollArea::vertical()
                    .id_salt("compile_content")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        match tab {
                            CompileTab::Display => display_tab(app, ui),
                            CompileTab::Format(kind) => output_tab(
                                app,
                                ui,
                                kind,
                                &mut settings,
                                &front_docs,
                                &back_docs,
                                &mut matter_toggles,
                                &mut compile,
                            ),
                        }
                    });
            });
        });
    });

    app.compile_tab = tab;
    if let CompileTab::Format(_) = tab {
        let before = app.ws.project.settings_for(settings.kind);
        app.ws.project.set_settings(settings.clone());
        app.ws.project.export = settings.clone();
        if before != settings {
            app.ws.dirty = true;
        }
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
        run_export(app, &settings);
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
        sans(11.0),
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
    toggles: &mut Vec<(String, bool)>,
    compile: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(kind.label()).font(serif(21.0)).color(TEXT));
            ui.label(
                RichText::new(kind.description())
                    .font(sans(10.0))
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

    output_panel(app, ui, settings);
    ui.add_space(10.0);

    action_panel(app, ui, settings, compile);
    ui.add_space(12.0);

    style_panel(ui, &mut settings.styles);
    ui.add_space(12.0);

    type_panel(ui, kind, settings);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if quiet_button(ui, "Reset this format to base style").clicked() {
            *settings = seeded_settings(&app.ws.project.styles, app.ws.project.kind, settings.kind);
        }
        ui.label(
            RichText::new("Seeds from the project-wide element styles.")
                .font(sans(9.0))
                .color(FAINT),
        );
    });
    ui.add_space(12.0);

    ui.columns(2, |cols| {
        front_panel(&mut cols[0], settings, kind, front_docs, toggles);
        body_panel(&mut cols[1], settings, kind, back_docs, toggles);
    });
    ui.add_space(12.0);

    preview_panel(app, ui);
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
            ui.horizontal(|ui| {
                Frame::new()
                    .fill(BG)
                    .stroke(Stroke::new(1.0, BORDER_SOFT))
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.add(
                            TextEdit::singleline(&mut settings.output_path)
                                .hint_text("Folder for the export (blank = exports/)")
                                .font(mono(10.0))
                                .frame(false)
                                .desired_width((ui.available_width() - 170.0).max(160.0)),
                        );
                    });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let name = portmanteau_export::default_filename(&app.ws.project, settings.kind);
                    ui.label(
                        RichText::new(format!("/ {name}"))
                            .font(mono(9.0))
                            .color(FAINT),
                    );
                });
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Front and back matter are assembled from the binder folders; excluded chapters are skipped.",
                )
                .font(sans(9.0))
                .color(FAINT),
            );
        });
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
                        .font(sans(12.0))
                        .color(Color32::from_rgb(29, 24, 17)),
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
                        RichText::new(format!(
                            "{} words · {} pages estimated",
                            app.ws.project.words(),
                            app.ws
                                .project
                                .manuscript_documents()
                                .iter()
                                .map(|d| d.pages())
                                .sum::<u32>()
                        ))
                        .font(sans(10.0))
                        .color(MUTED),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Includes {} manuscript sections",
                            app.ws
                                .project
                                .manuscript_documents()
                                .iter()
                                .filter(|d| !matches!(
                                    d.kind,
                                    DocKind::FrontMatter | DocKind::BackMatter
                                ))
                                .count()
                        ))
                        .font(sans(9.0))
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
                            .font(sans(10.0))
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

fn style_panel(ui: &mut egui::Ui, styles: &mut ElementStyles) {
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
                    "Per-element type for this output format, including writer-defined styles. The global defaults live in the Display tab.",
                )
                .font(sans(9.0))
                .color(FAINT),
            );
            ui.add_space(6.0);
            element_styles_editor(ui, styles);
            ui.add_space(2.0);
            custom_styles_editor(ui, styles);
        });
}

fn type_panel(ui: &mut egui::Ui, kind: ExportKind, settings: &mut ExportSettings) {
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
                    ui.label(RichText::new("Paper").font(sans(9.5)).color(MUTED));
                    egui::ComboBox::from_id_salt("paper_pick")
                        .selected_text(
                            RichText::new(settings.paper.label()).font(sans(10.0)),
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

            ui.horizontal(|ui| {
                ui.label(RichText::new("Font").font(sans(9.5)).color(MUTED));
                font_picker(ui, &mut settings.font);
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Type size").font(sans(9.5)).color(MUTED));
                ui.add(
                    egui::DragValue::new(&mut settings.font_size)
                        .speed(0.1)
                        .range(7.0..=24.0)
                        .suffix(" pt"),
                );
                ui.add_space(6.0);
                ui.label(RichText::new("Leading").font(sans(9.5)).color(MUTED));
                ui.add(
                    egui::DragValue::new(&mut settings.line_spacing)
                        .speed(0.02)
                        .range(1.0..=2.4)
                        .suffix("×"),
                );
            });

            if kind == ExportKind::Pdf {
                ui.add_space(7.0);
                check(
                    ui,
                    settings,
                    |s| &mut s.include_page_numbers,
                    "Page numbers",
                );
                check(
                    ui,
                    settings,
                    |s| &mut s.include_running_headers,
                    "Running headers",
                );
            }

            if matches!(kind, ExportKind::Epub | ExportKind::Html) {
                ui.add_space(7.0);
                ui.label(
                    RichText::new("Cover image")
                        .font(sans(9.5))
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

fn font_picker(ui: &mut egui::Ui, font: &mut String) {
    let selected = if font.trim().is_empty() {
        "(inherit)".to_string()
    } else {
        font.clone()
    };
    egui::ComboBox::from_id_salt("compile_font")
        .selected_text(RichText::new(selected).font(sans(10.0)).color(TEXT))
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
                .font(sans(9.0))
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
        ui.label(RichText::new("(none)").font(sans(9.0)).color(FAINT));
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

fn preview_panel(app: &PortmanteauApp, ui: &mut egui::Ui) {
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
            let docs: Vec<_> = app
                .ws
                .project
                .manuscript_documents()
                .into_iter()
                .filter(|d| !matches!(d.kind, DocKind::FrontMatter | DocKind::BackMatter))
                .collect();
            for d in docs.iter().take(14) {
                ui.horizontal(|ui| {
                    let glyph = if d.medium.is_script() { "▶" } else { "¶" };
                    ui.label(
                        RichText::new(glyph)
                            .font(sans(8.5))
                            .color(if d.medium.is_script() { PURPLE } else { ACCENT }),
                    );
                    ui.label(
                        RichText::new(&d.title)
                            .font(sans(10.0))
                            .color(if d.include_in_export { TEXT } else { FAINT }),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} w · {} pp", d.words(), d.pages()))
                                .font(mono(8.5))
                                .color(FAINT),
                        );
                    });
                });
            }
            if docs.len() > 14 {
                ui.label(
                    RichText::new(format!("… and {} more sections", docs.len() - 14))
                        .font(sans(9.0))
                        .color(FAINT),
                );
            }
        });
}

fn run_export(app: &mut PortmanteauApp, settings: &ExportSettings) {
    let payload = serde_json::json!({
        "format": settings.kind.label(),
        "path": settings.output_path,
        "paper": settings.paper.label(),
        "font": settings.font,
        "font_size": settings.font_size,
        "line_spacing": settings.line_spacing,
        "include_page_numbers": settings.include_page_numbers,
        "include_running_headers": settings.include_running_headers,
        "cover_image": settings.cover_image,
        "include_title_page": settings.include_title_page,
        "include_toc": settings.include_toc,
        "include_chapter_numbers": settings.include_chapter_numbers,
        "include_scene_numbers": settings.include_scene_numbers,
        "include_synopsis": settings.include_synopsis,
        "include_notes": settings.include_notes
    });
    let res = app.ws.execute("export.run", payload);
    if res.ok {
        let path = res
            .data
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("the export folder");
        app.toast("Compile finished", res.message.clone(), ToastKind::Ok);
        app.ws.last_message = format!("{} → {}", res.message, path);
    } else {
        app.toast("Compile failed", res.message, ToastKind::Error);
    }
}

// ---------------------------------------------------------------------------
// Display tab
// ---------------------------------------------------------------------------

fn display_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut save = false;
    let mut apply = false;
    let mut reset = false;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Display").font(serif(21.0)).color(TEXT));
            ui.label(
                RichText::new(
                    "How the words look while you write, and the global house style new projects start from.",
                )
                .font(sans(10.0))
                .color(FAINT),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            pill(
                ui,
                &format!(
                    "{} built-in + {} custom",
                    BUILTIN_ELEMENTS.len(),
                    app.settings.styles.custom.len()
                ),
                MUTED,
                PANEL_2,
            );
        });
    });
    ui.add_space(8.0);

    live_preview_panel(ui, &app.settings.styles);
    ui.add_space(10.0);

    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Editor display");
            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    "On-screen type sizes. The compiled page uses each output format's own element styles.",
                )
                .font(sans(9.5))
                .color(FAINT),
            );
            ui.add_space(6.0);
            egui::Grid::new("display_grid")
                .num_columns(4)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Prose size").font(sans(9.5)).color(MUTED));
                    ui.add(
                        egui::DragValue::new(&mut app.settings.display.prose_size)
                            .speed(0.1)
                            .range(9.0..=32.0)
                            .suffix(" pt"),
                    );
                    ui.label(RichText::new("Leading").font(sans(9.5)).color(MUTED));
                    ui.add(
                        egui::DragValue::new(&mut app.settings.display.prose_leading)
                            .speed(0.02)
                            .range(1.0..=2.6)
                            .suffix("×"),
                    );
                    ui.end_row();

                    ui.label(RichText::new("Script size").font(sans(9.5)).color(MUTED));
                    ui.add(
                        egui::DragValue::new(&mut app.settings.display.script_size)
                            .speed(0.1)
                            .range(7.0..=24.0)
                            .suffix(" pt"),
                    );
                    ui.label(RichText::new("Leading").font(sans(9.5)).color(MUTED));
                    ui.add(
                        egui::DragValue::new(&mut app.settings.display.script_leading)
                            .speed(0.02)
                            .range(1.0..=2.2)
                            .suffix("×"),
                    );
                    ui.end_row();
                });
        });

    ui.add_space(10.0);
    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Editing");
            ui.add_space(4.0);
            ui.checkbox(
                &mut app.settings.spellcheck,
                RichText::new("Spell-checking in editors")
                    .font(sans(10.5))
                    .color(MUTED),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    "The Write toolbar shows a Spelling panel with suggestions. Save defaults to keep this setting.",
                )
                .font(sans(9.5))
                .color(FAINT),
            );
        });

    ui.add_space(10.0);
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    section_label(ui, "Default styles");
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(
                            "Type defaults for every manuscript element. Apply them to a project, then each output tab refines per format.",
                        )
                        .font(sans(9.5))
                        .color(FAINT),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Config · {}",
                            crate::settings::config_path().display()
                        ))
                        .font(mono(8.5))
                        .color(FAINT),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(ui, "Save defaults").clicked() {
                        save = true;
                    }
                    if quiet_button(ui, "Apply to current project").clicked() {
                        apply = true;
                    }
                    if quiet_button(ui, "Reset defaults").clicked() {
                        reset = true;
                    }
                });
            });
            ui.add_space(8.0);
            hairline(ui);
            ui.add_space(8.0);

            element_styles_editor(ui, &mut app.settings.styles);
            ui.add_space(2.0);
            custom_styles_editor(ui, &mut app.settings.styles);
        });

    ui.add_space(10.0);
    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("New projects copy these defaults automatically.")
                        .font(sans(9.5))
                        .color(MUTED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("Global settings · local file")
                            .font(sans(9.0))
                            .color(FAINT),
                    );
                });
            });
        });

    if reset {
        app.settings.styles = ElementStyles::default();
        app.toast(
            "Defaults reset",
            "Built-in element styles restored — save to keep them",
            ToastKind::Info,
        );
    }
    if save {
        match crate::settings::save_settings(&app.settings) {
            Ok(()) => app.toast(
                "Defaults saved",
                crate::settings::config_path().display().to_string(),
                ToastKind::Ok,
            ),
            Err(e) => app.toast("Could not save defaults", e, ToastKind::Error),
        }
    }
    if apply {
        app.ws.project.styles = app.settings.styles.clone();
        app.ws.dirty = true;
        app.toast(
            "Defaults applied",
            "The current project now uses the global element styles",
            ToastKind::Ok,
        );
    }
}

/// The live style preview: pick any built-in or custom element and see the
/// sample set in its current type and paragraph style. The picker's choice
/// lives in egui's temporary data so it survives the per-frame redraw.
fn live_preview_panel(ui: &mut egui::Ui, styles: &ElementStyles) {
    let id = egui::Id::new("settings_preview_element");
    let mut selected: String = ui
        .data(|data| data.get_temp(id))
        .unwrap_or_else(|| "body".to_string());

    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, "Live preview");
                ui.add_space(6.0);
                let display = BUILTIN_ELEMENTS
                    .iter()
                    .find(|(key, _)| *key == selected)
                    .map(|(_, label)| (*label).to_string())
                    .unwrap_or_else(|| selected.clone());
                egui::ComboBox::from_id_salt("settings_preview_element_pick")
                    .selected_text(RichText::new(display).font(sans(9.5)).color(TEXT))
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        for (key, label) in BUILTIN_ELEMENTS {
                            ui.selectable_value(&mut selected, key.to_string(), label);
                        }
                        if !styles.custom.is_empty() {
                            ui.separator();
                            for custom in &styles.custom {
                                ui.selectable_value(
                                    &mut selected,
                                    custom.name.clone(),
                                    format!("{} (custom)", custom.name),
                                );
                            }
                        }
                    });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    meta_line(ui, &["live as you edit below"]);
                });
            });
            ui.add_space(6.0);
            let style = styles
                .element(&selected)
                .or_else(|| styles.custom_named(&selected).map(|custom| &custom.style));
            match style {
                Some(style) => element_preview(ui, style),
                None => {
                    selected = "body".to_string();
                    element_preview(ui, &styles.body);
                }
            }
            ui.add_space(5.0);
            ui.label(
                RichText::new(
                    "Approximate on-screen proof. The compiled page uses the same font, size, leading, alignment and indents, but line breaking and page margins may differ.",
                )
                .font(sans(8.5))
                .color(FAINT),
            );
        });
    ui.data_mut(|data| data.insert_temp(id, selected));
}
