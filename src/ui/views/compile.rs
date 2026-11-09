//! Compile: choose a finish line, decide what goes in the book, and write a
//! real file — PDF, EPUB, DOCX, Final Draft, Fountain, HTML, Markdown, RTF,
//! plain text or a complete project bundle.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};
use msc_core::{DocKind, ExportKind, PaperSize};

use crate::app::{ScriptoriumApp, ToastKind};
use crate::theme::*;
use crate::ui::views::{content_frame, meta_line, page_head, quiet_button};

pub fn show(app: &mut ScriptoriumApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        "Finish",
        "Compile",
        "One manuscript, many finish lines. Build the file your reader, editor or producer needs.",
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

    let mut settings = app.ws.project.export.clone();
    let mut compile = false;

    content_frame().show(ui, |ui| {
        // Presets.
        ui.horizontal_wrapped(|ui| {
            section_label(ui, "Presets");
            ui.add_space(4.0);
            let presets = app.ws.project.presets.clone();
            for preset in &presets {
                let active = preset.settings.kind == settings.kind
                    && preset.settings.paper == settings.paper
                    && (preset.settings.font_size - settings.font_size).abs() < 0.01;
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(&preset.name)
                                .font(sans(9.5))
                                .color(if active { Color32::from_rgb(29, 24, 17) } else { MUTED }),
                        )
                        .fill(if active { ACCENT } else { PANEL_2 })
                        .stroke(Stroke::new(1.0, BORDER_SOFT))
                        .corner_radius(CornerRadius::same(12))
                        .min_size(Vec2::new(0.0, 23.0)),
                    )
                    .clicked()
                {
                    settings = preset.settings.clone();
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if quiet_button(ui, "Open outline").clicked() {
                    app.view = crate::app::View::Outline;
                }
            });
        });
        ui.add_space(12.0);

        let height = (ui.available_height() - 4.0).max(420.0);
        ui.horizontal_top(|ui| {
            // ---- Format shelf ----------------------------------------------
            ui.allocate_ui_with_layout(
                Vec2::new(292.0, height),
                Layout::top_down(Align::Min),
                |ui| {
                    section_label(ui, "Output format");
                    ui.add_space(5.0);
                    ScrollArea::vertical()
                        .id_salt("format_list")
                        .auto_shrink([false, false])
                        .max_height(height - 26.0)
                        .show(ui, |ui| {
                            for kind in ExportKind::all() {
                                format_card(ui, kind, &mut settings);
                                ui.add_space(4.0);
                            }
                        });
                },
            );
            ui.add_space(16.0);
            // ---- Settings ---------------------------------------------------
            ui.vertical(|ui| {
                ScrollArea::vertical()
                    .id_salt("compile_settings")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        output_panel(app, ui, &mut settings);
                        ui.add_space(10.0);
                        ui.columns(2, |cols| {
                            page_panel(&mut cols[0], &mut settings);
                            body_panel(&mut cols[1], &mut settings);
                        });
                        ui.add_space(12.0);

                        // The action and its result.
                        Frame::new()
                            .fill(PANEL)
                            .stroke(Stroke::new(1.0, BORDER_SOFT))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::symmetric(16, 13))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let label = format!(
                                        "Compile {}  ▸",
                                        settings.kind.label()
                                    );
                                    let btn = egui::Button::new(
                                        RichText::new(label)
                                            .font(sans(12.0))
                                            .color(Color32::from_rgb(29, 24, 17)),
                                    )
                                    .fill(ACCENT)
                                    .corner_radius(CornerRadius::same(6))
                                    .min_size(Vec2::new(190.0, 34.0));
                                    if ui.add(btn).clicked() {
                                        compile = true;
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
                                        let (dot_rect, _) = ui.allocate_exact_size(
                                            Vec2::splat(8.0),
                                            Sense::hover(),
                                        );
                                        ui.painter().circle_filled(
                                            dot_rect.center(),
                                            3.5,
                                            SAGE,
                                        );
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
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if ui.small_button("Copy path").clicked() {
                                                    ui.ctx()
                                                        .copy_text(info.path.clone());
                                                }
                                            },
                                        );
                                    });
                                }
                            });
                        ui.add_space(14.0);
                        preview_panel(app, ui);
                    });
            });
        });
    });

    let before = app.ws.project.export.clone();
    app.ws.project.export = settings.clone();
    if before != settings {
        app.ws.dirty = true;
    }
    if compile {
        run_export(app, &settings);
    }
}

fn format_card(ui: &mut egui::Ui, kind: ExportKind, settings: &mut msc_core::ExportSettings) {
    let selected = settings.kind == kind;
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
    let color = format_color(kind);
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(6),
        if selected {
            ACCENT_SOFT
        } else if resp.hovered() {
            PANEL_2
        } else {
            PANEL
        },
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(
            if selected { 1.2 } else { 1.0 },
            if selected { ACCENT } else { BORDER_SOFT },
        ),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.left_center() + Vec2::new(11.0, -8.0),
        egui::Align2::LEFT_CENTER,
        kind.label(),
        sans(10.5),
        if selected { TEXT } else { MUTED },
    );
    ui.painter().text(
        rect.left_center() + Vec2::new(11.0, 8.0),
        egui::Align2::LEFT_CENTER,
        kind.description(),
        sans(8.5),
        FAINT,
    );
    ui.painter().text(
        rect.right_center() - Vec2::new(10.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        format!(".{}", kind.extension()),
        mono(8.5),
        color,
    );
    if resp.clicked() {
        settings.kind = kind;
    }
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

fn output_panel(app: &ScriptoriumApp, ui: &mut egui::Ui, settings: &mut msc_core::ExportSettings) {
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
                    let name = msc_export::default_filename(&app.ws.project, settings.kind);
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

fn page_panel(ui: &mut egui::Ui, settings: &mut msc_core::ExportSettings) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Page & type");
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Paper").font(sans(9.5)).color(MUTED));
                egui::ComboBox::from_id_salt("paper_pick")
                    .selected_text(RichText::new(settings.paper.label()).font(sans(10.0)))
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for paper in PaperSize::all() {
                            ui.selectable_value(&mut settings.paper, paper, paper.label());
                        }
                    });
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
            ui.add_space(7.0);
            hairline(ui);
            ui.add_space(7.0);
            section_label(ui, "Front matter");
            ui.add_space(3.0);
            check(ui, settings, |s| &mut s.include_title_page, "Title page");
            check(ui, settings, |s| &mut s.include_copyright, "Copyright");
            check(ui, settings, |s| &mut s.include_dedication, "Dedication");
            check(ui, settings, |s| &mut s.include_preface, "Author's note");
            check(ui, settings, |s| &mut s.include_toc, "Table of contents");
            check(ui, settings, |s| &mut s.include_chapter_numbers, "Chapter numbers");
        });
}

fn body_panel(ui: &mut egui::Ui, settings: &mut msc_core::ExportSettings) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "Body & back matter");
            ui.add_space(5.0);
            check(ui, settings, |s| &mut s.include_scene_numbers, "Scene numbers");
            check(ui, settings, |s| &mut s.include_synopsis, "Include synopses");
            check(ui, settings, |s| &mut s.include_notes, "Include author notes");
            check(ui, settings, |s| &mut s.include_acknowledgements, "Acknowledgements");
            check(ui, settings, |s| &mut s.include_author_bio, "About the author");
            ui.add_space(7.0);
            hairline(ui);
            ui.add_space(7.0);
            section_label(ui, "Manuscript");
            ui.add_space(3.0);
            let docs = &settings.kind;
            ui.label(
                RichText::new(format!(
                    "{} is compiled from the binder. Script media keep their element types.",
                    docs.label()
                ))
                .font(sans(9.0))
                .color(FAINT),
            );
        });
}

fn check(
    ui: &mut egui::Ui,
    settings: &mut msc_core::ExportSettings,
    field: impl Fn(&mut msc_core::ExportSettings) -> &mut bool,
    label: &str,
) {
    let mut value = *field(settings);
    if ui.checkbox(&mut value, label).changed() {
        *field(settings) = value;
    }
}

fn preview_panel(app: &ScriptoriumApp, ui: &mut egui::Ui) {
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

fn run_export(app: &mut ScriptoriumApp, settings: &msc_core::ExportSettings) {
    let payload = serde_json::json!({
        "format": settings.kind.label(),
        "path": settings.output_path,
        "paper": settings.paper.label(),
        "include_title_page": settings.include_title_page,
        "include_copyright": settings.include_copyright,
        "include_dedication": settings.include_dedication,
        "include_acknowledgements": settings.include_acknowledgements,
        "include_preface": settings.include_preface,
        "include_toc": settings.include_toc,
        "include_chapter_numbers": settings.include_chapter_numbers,
        "include_scene_numbers": settings.include_scene_numbers,
        "include_synopsis": settings.include_synopsis,
        "include_notes": settings.include_notes,
        "include_author_bio": settings.include_author_bio
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
