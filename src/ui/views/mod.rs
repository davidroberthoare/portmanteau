//! The non-writing workspaces: corkboard, outline, research, database,
//! version history, compilation and the command desk.

pub mod agent;
pub mod database;
pub mod compile;
pub mod corkboard;
pub mod history;
pub mod outline;
pub mod research;
pub mod settings;

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea,
    Stroke, TextEdit, Vec2,
};
use portmanteau_core::{
    Alignment, CustomStyle, ElementStyle, ElementStyles, ExportSettings, ParagraphStyle, Project,
    StyleSheet, TextTransform,
};

use crate::app::{PortmanteauApp, View};
use crate::theme::*;
use crate::ui::page::{manuscript_preview, page_frame, PageLine, PagePlan};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    match app.view {
        View::Corkboard => corkboard::show(app, ui),
        View::Outline => outline::show(app, ui),
        View::Research => research::show(app, ui),
        View::Database => database::show(app, ui),
        View::History => history::show(app, ui),
        View::Compile => compile::show(app, ui),
        View::Agent => agent::show(app, ui),
        View::Settings => settings::show(app, ui),
        View::Write => {}
    }
}

/// Shared page header with a kicker, a large title, supporting copy and an
/// optional right-hand action area.
pub fn page_head<R>(
    ui: &mut egui::Ui,
    kicker: &str,
    title: &str,
    subtitle: &str,
    right: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let mut result = None;
    Frame::new()
        .fill(BG)
        .inner_margin(Margin {
            left: 26,
            right: 26,
            top: 20,
            bottom: 15,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(kicker.to_uppercase())
                            .font(sans(9.5))
                            .color(ACCENT)
                            .extra_letter_spacing(1.5),
                    );
                    ui.add_space(2.0);
                    ui.label(RichText::new(title).font(serif(26.0)).color(TEXT));
                    ui.add_space(2.0);
                    ui.label(RichText::new(subtitle).font(sans(11.0)).color(MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    result = Some(right(ui));
                });
            });
        });
    hairline(ui);
    result.unwrap()
}

pub fn content_frame() -> Frame {
    Frame::new().inner_margin(Margin {
        left: 26,
        right: 26,
        top: 18,
        bottom: 12,
    })
}

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(text)
                .font(sans(11.5))
                .color(Color32::from_rgb(29, 24, 17)),
        )
        .fill(ACCENT)
        .corner_radius(CornerRadius::same(6))
        .min_size(Vec2::new(0.0, 29.0)),
    )
}

pub fn quiet_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).font(sans(10.5)).color(MUTED))
            .fill(PANEL_2)
            .stroke(Stroke::new(1.0, BORDER_SOFT))
            .corner_radius(CornerRadius::same(5))
            .min_size(Vec2::new(0.0, 26.0)),
    )
}

/// A small metadata line used at the foot of cards and rows.
pub fn meta_line(ui: &mut egui::Ui, parts: &[&str]) {
    let text = parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("  ·  ");
    ui.label(RichText::new(text).font(sans(9.5)).color(FAINT));
}

const TITLE_PAGE_ELEMENTS: [(&str, &str); 3] = [
    ("Title page title", "title_page_title"),
    ("Title page subtitle", "title_page_subtitle"),
    ("Title page author", "title_page_author"),
];

const PROSE_ELEMENTS: [(&str, &str); 3] = [
    ("Chapter heading", "chapter_heading"),
    ("Body", "body"),
    ("General", "general"),
];

const SCRIPT_ELEMENTS: [(&str, &str); 13] = [
    ("Scene heading", "scene_heading"),
    ("Action", "action"),
    ("Character", "character"),
    ("Parenthetical", "parenthetical"),
    ("Dialogue", "dialogue"),
    ("Transition", "transition"),
    ("Act", "act"),
    ("Scene", "scene"),
    ("Stage direction", "stage_direction"),
    ("Shot", "action"),
    ("Extension", "extension"),
    ("Intercut", "intercut"),
    ("Subheading", "subheading"),
];

/// Every built-in element in editor order, paired with its display label.
/// The Display preview picker and the count in its header read this list.
pub const BUILTIN_ELEMENTS: [(&str, &str); 18] = [
    ("title_page_title", "Title page title"),
    ("title_page_subtitle", "Title page subtitle"),
    ("title_page_author", "Title page author"),
    ("chapter_heading", "Chapter heading"),
    ("body", "Body"),
    ("scene_heading", "Scene heading"),
    ("action", "Action"),
    ("character", "Character"),
    ("parenthetical", "Parenthetical"),
    ("dialogue", "Dialogue"),
    ("transition", "Transition"),
    ("act", "Act"),
    ("scene", "Scene"),
    ("stage_direction", "Stage direction"),
    ("general", "General"),
    ("extension", "Extension"),
    ("intercut", "Intercut"),
    ("subheading", "Subheading"),
];

/// Families offered by the style editor. A project may still hold a custom
/// family; the dropdown shows it instead of silently resetting it.
pub const FONT_FAMILIES: [&str; 8] = [
    "Liberation Serif",
    "Liberation Sans",
    "Liberation Mono",
    "Courier New",
    "Times New Roman",
    "Helvetica",
    "Georgia",
    "DejaVu Sans",
];

fn style_summary(label: &str, style: &ElementStyle) -> String {
    let font = if style.font.trim().is_empty() {
        "inherit"
    } else {
        style.font.as_str()
    };
    format!(
        "{label} — {font} · {:.0} pt · {:.1}×",
        style.size, style.leading
    )
}

/// The shared per-element style editor, modelled after a word processor's
/// Paragraph dialog. Elements are grouped into three collapsible sections so
/// the Compile screen and its Display tab stay compact. Each row edits an owned
/// clone, applied after the draw pass.
///
/// `grey_prose` / `grey_script` disable the group that cannot apply to the open
/// project's medium (a novel has no script elements, a screenplay no prose
/// elements). Settings passes both `false`: its defaults apply to everything.
pub fn element_styles_editor(
    ui: &mut egui::Ui,
    styles: &mut ElementStyles,
    grey_prose: bool,
    grey_script: bool,
) -> bool {
    let mut changed = false;
    let sections: [(&str, &str, &str, &[(&str, &str)]); 3] = [
        (
            "Title page",
            "Title page title",
            "title_page_title",
            &TITLE_PAGE_ELEMENTS,
        ),
        ("Prose", "Body", "body", &PROSE_ELEMENTS),
        ("Script", "Action", "action", &SCRIPT_ELEMENTS),
    ];

    for (section, summary_label, summary_key, elements) in sections {
        let greyed = match section {
            "Prose" => grey_prose,
            "Script" => grey_script,
            _ => false,
        };
        let summary = styles
            .element_mut(summary_key)
            .map(|style| style_summary(summary_label, style))
            .unwrap_or_default();
        egui::CollapsingHeader::new(
            RichText::new(format!("{section}    {summary}"))
                .font(sans(10.5))
                .color(if greyed { FAINT } else { TEXT }),
        )
        .id_salt(("style_section", section))
        .default_open(section == "Prose")
        .show(ui, |ui| {
            ui.add_enabled_ui(!greyed, |ui| {
                ui.add_space(4.0);
                section_label(ui, "Type & paragraph");
                ui.add_space(2.0);
                let mut first = true;
                for (label, key) in elements {
                    if !first {
                        ui.add_space(3.0);
                    }
                    first = false;
                    let Some(mut draft) = styles.element_mut(key).cloned() else {
                        continue;
                    };
                    if element_row(ui, key, label, &mut draft) {
                        if let Some(target) = styles.element_mut(key) {
                            *target = draft;
                            changed = true;
                        }
                    }
                }
                ui.add_space(2.0);
                hairline(ui);
            });
        });
        ui.add_space(2.0);
    }
    changed
}

fn font_combo(ui: &mut egui::Ui, key: &str, element: &mut ElementStyle) -> bool {
    let mut changed = false;
    let mut font = element.font.clone();
    let selected = if font.trim().is_empty() {
        "(inherit)".to_string()
    } else {
        font.clone()
    };
    egui::ComboBox::from_id_salt(("style_font", key))
        .selected_text(RichText::new(selected).font(sans(9.0)).color(TEXT))
        .width(150.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(&mut font, String::new(), "(inherit)")
                .changed()
            {
                changed = true;
            }
            if !font.trim().is_empty() && !FONT_FAMILIES.contains(&font.as_str()) {
                let custom = font.clone();
                if ui
                    .selectable_value(&mut font, custom.clone(), format!("{custom} (custom)"))
                    .changed()
                {
                    changed = true;
                }
            }
            for family in FONT_FAMILIES {
                if ui
                    .selectable_value(&mut font, family.to_string(), family)
                    .changed()
                {
                    changed = true;
                }
            }
        });
    if font != element.font {
        element.font = font;
        changed = true;
    }
    changed
}

fn align_combo(ui: &mut egui::Ui, key: &str, element: &mut ElementStyle) -> bool {
    let mut changed = false;
    let mut align = element.align;
    egui::ComboBox::from_id_salt(("style_align", key))
        .selected_text(RichText::new(align.label()).font(sans(9.0)).color(TEXT))
        .width(74.0)
        .show_ui(ui, |ui| {
            for option in Alignment::all() {
                if ui
                    .selectable_value(&mut align, option, option.label())
                    .changed()
                {
                    changed = true;
                }
            }
        });
    if align != element.align {
        element.align = align;
        changed = true;
    }
    changed
}

fn element_row(
    ui: &mut egui::Ui,
    key: &str,
    label: &str,
    element: &mut ElementStyle,
) -> bool {
    let mut changed = false;
    Frame::new()
        .fill(PANEL_2)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).font(sans(9.5)).color(TEXT));
            ui.add_space(4.0);
            egui::Grid::new(("style_grid", key))
                .num_columns(4)
                .spacing([8.0, 5.0])
                .show(ui, |ui| {
                    grid_label(ui, "Font");
                    changed |= font_combo(ui, key, element);
                    grid_label(ui, "Size");
                    changed |= grid_drag(ui, &mut element.size, 0.1, 4.0..=72.0, " pt");
                    ui.end_row();

                    grid_label(ui, "Leading");
                    changed |= grid_drag(ui, &mut element.leading, 0.02, 0.5..=3.0, "×");
                    grid_label(ui, "Align");
                    changed |= align_combo(ui, key, element);
                    ui.end_row();

                    grid_label(ui, "Indent");
                    changed |= grid_drag(ui, &mut element.indent, 0.5, 0.0..=800.0, " pt");
                    grid_label(ui, "First line");
                    changed |=
                        grid_drag(ui, &mut element.first_line_indent, 0.5, 0.0..=800.0, " pt");
                    ui.end_row();

                    grid_label(ui, "Right");
                    changed |=
                        grid_drag(ui, &mut element.right_indent, 0.5, 0.0..=800.0, " pt");
                    grid_label(ui, "Before");
                    changed |= grid_drag(ui, &mut element.before, 0.5, 0.0..=800.0, " pt");
                    ui.end_row();

                    grid_label(ui, "After");
                    changed |= grid_drag(ui, &mut element.after, 0.5, 0.0..=800.0, " pt");
                    grid_label(ui, "All caps");
                    changed |= ui.checkbox(&mut element.uppercase, "").changed();
                    ui.end_row();
                });
        });
    changed
}

fn grid_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).font(sans(8.5)).color(FAINT));
}

fn grid_drag(
    ui: &mut egui::Ui,
    value: &mut f32,
    speed: f64,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) -> bool {
    ui.add(
        egui::DragValue::new(value)
            .speed(speed)
            .range(range)
            .suffix(suffix),
    )
    .changed()
}

/// The writer-defined styles, shown below the built-in sections. Each row edits
/// an owned clone and writes it back after drawing; add and remove happen once
/// the list has finished drawing so the vec is not mutated mid-iteration.
/// Returns true when the list changed.
pub fn custom_styles_editor(ui: &mut egui::Ui, styles: &mut ElementStyles) -> bool {
    let count = styles.custom.len();
    let mut changed = false;
    egui::CollapsingHeader::new(
        RichText::new(format!(
            "Custom styles    {count} {}",
            if count == 1 { "style" } else { "styles" }
        ))
        .font(sans(10.5))
        .color(TEXT),
    )
    .id_salt("custom_style_section")
    .default_open(true)
    .show(ui, |ui| {
        ui.add_space(4.0);
        section_label(ui, "Writer-defined elements");
        ui.add_space(2.0);
        if styles.custom.is_empty() {
            ui.label(
                RichText::new(
                    "None yet. Add “Epigraph”, “Pull quote” or any element your book needs.",
                )
                .font(sans(9.0))
                .color(FAINT),
            );
            ui.add_space(4.0);
        }

        let mut remove: Option<usize> = None;
        for (index, custom) in styles.custom.iter_mut().enumerate() {
            let mut draft = custom.clone();
            let mut row_changed = false;
            Frame::new()
                .fill(PANEL_2)
                .stroke(Stroke::new(1.0, BORDER_SOFT))
                .corner_radius(CornerRadius::same(5))
                .inner_margin(Margin::symmetric(10, 7))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Name").font(sans(8.5)).color(FAINT));
                        row_changed |= ui
                            .add(
                                TextEdit::singleline(&mut draft.name)
                                    .font(sans(10.0))
                                    .desired_width(180.0),
                            )
                            .changed();
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Remove").font(sans(9.0)).color(RED),
                                    )
                                    .fill(PANEL_3)
                                    .stroke(Stroke::new(1.0, BORDER_SOFT))
                                    .corner_radius(CornerRadius::same(5)),
                                )
                                .clicked()
                            {
                                remove = Some(index);
                            }
                        });
                    });
                    ui.add_space(4.0);
                    row_changed |= element_row(
                        ui,
                        &format!("custom_style_{index}"),
                        "Paragraph",
                        &mut draft.style,
                    );
                });
            if row_changed {
                *custom = draft;
                changed = true;
            }
        }

        if let Some(index) = remove {
            styles.custom.remove(index);
            changed = true;
        }

        ui.add_space(4.0);
        if quiet_button(ui, "Add custom style").clicked() {
            let mut name = "New style".to_string();
            let mut suffix = 2;
            while styles.custom.iter().any(|style| style.name == name)
                || styles.element(&name).is_some()
            {
                name = format!("New style {suffix}");
                suffix += 1;
            }
            styles.custom.push(CustomStyle {
                name,
                style: ElementStyle::default(),
            });
            changed = true;
        }
        ui.add_space(2.0);
        hairline(ui);
    });
    changed
}

/// Edit stylesheet names, display transforms and next-style rules. Formatting
/// fields stay in `ElementStyle` and are edited by the shared element-style
/// editor; this panel controls the paragraph semantics used while writing.
pub fn style_sheets_editor(ui: &mut egui::Ui, sheets: &mut Vec<StyleSheet>) -> bool {
    let mut changed = false;
    for sheet in sheets.iter_mut() {
        let options: Vec<(String, String)> = sheet
            .styles
            .iter()
            .map(|style| (style.id.clone(), style.name.clone()))
            .collect();
        egui::CollapsingHeader::new(
            RichText::new(format!("{} stylesheet", sheet.name))
                .font(sans(10.5))
                .color(TEXT),
        )
        .id_salt(("style_sheet", sheet.id.as_str()))
        .default_open(false)
        .show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Default paragraph").font(sans(9.0)).color(FAINT));
                egui::ComboBox::from_id_salt(("default_style", sheet.id.as_str()))
                    .selected_text(
                        sheet
                            .style(&sheet.default_style)
                            .map(|style| style.name.as_str())
                            .unwrap_or("Choose style"),
                    )
                    .show_ui(ui, |ui| {
                        for (id, name) in &options {
                            changed |= ui
                                .selectable_value(&mut sheet.default_style, id.clone(), name)
                                .changed();
                        }
                    });
            });

            let mut additions = Vec::new();
            for index in 0..sheet.styles.len() {
                let mut draft = sheet.styles[index].clone();
                ui.add_space(4.0);
                Frame::new()
                    .fill(PANEL_2)
                    .stroke(Stroke::new(1.0, BORDER_SOFT))
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(Margin::symmetric(9, 6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Style").font(sans(8.5)).color(FAINT));
                            changed |= ui
                                .add(
                                    TextEdit::singleline(&mut draft.name)
                                        .font(sans(9.5))
                                        .desired_width(110.0),
                                )
                                .changed();
                            ui.label(RichText::new("Enter →").font(sans(8.5)).color(FAINT));
                            egui::ComboBox::from_id_salt((
                                "next_style",
                                sheet.id.as_str(),
                                draft.id.as_str(),
                            ))
                            .selected_text(
                                draft
                                    .next_style
                                    .as_deref()
                                    .and_then(|id| options.iter().find(|(key, _)| key == id))
                                    .map(|(_, name)| name.as_str())
                                    .unwrap_or("Repeat current"),
                            )
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(
                                        &mut draft.next_style,
                                        None,
                                        "Repeat current",
                                    )
                                    .changed();
                                for (id, name) in &options {
                                    changed |= ui
                                        .selectable_value(
                                            &mut draft.next_style,
                                            Some(id.clone()),
                                            name,
                                        )
                                        .changed();
                                }
                            });
                        });
                        let mut uppercase = draft.text_transform == TextTransform::Uppercase;
                        if ui.checkbox(&mut uppercase, "Display uppercase").changed() {
                            draft.text_transform = if uppercase {
                                TextTransform::Uppercase
                            } else {
                                TextTransform::None
                            };
                            changed = true;
                        }
                        egui::CollapsingHeader::new(
                            RichText::new("Appearance")
                                .font(sans(8.5))
                                .color(MUTED),
                        )
                        .id_salt(("style_appearance", sheet.id.as_str(), draft.id.as_str()))
                        .show(ui, |ui| {
                            changed |= element_row(
                                ui,
                                &format!("sheet_{}_{}", sheet.id, draft.id),
                                "Paragraph",
                                &mut draft.format,
                            );
                        });
                    });
                if draft != sheet.styles[index] {
                    sheet.styles[index] = draft;
                    changed = true;
                }
            }

            if quiet_button(ui, "Add custom paragraph style").clicked() {
                let mut suffix = sheet.styles.len() + 1;
                let mut id = format!("custom-{suffix}");
                while sheet.styles.iter().any(|style| style.id == id) {
                    suffix += 1;
                    id = format!("custom-{suffix}");
                }
                additions.push(ParagraphStyle {
                    id,
                    name: "Custom style".into(),
                    format: ElementStyle::default(),
                    semantic_kind: None,
                    next_style: Some(sheet.default_style.clone()),
                    text_transform: TextTransform::None,
                });
            }
            if !additions.is_empty() {
                sheet.styles.extend(additions);
                changed = true;
            }
        });
    }
    changed
}

/// A live single-page "print preview" of the compiled manuscript, with
/// `‹ Prev` / page count / `Next ›` and a section picker. Shared by the Compile
/// screen and Settings → Display so both read the same paging estimate.
///
/// `page` is the caller's current 0-based page cursor; it is clamped when the
/// preview's page count shrinks and moved when the section picker changes.
pub fn manuscript_preview_navigator(
    project: &Project,
    settings: &ExportSettings,
    page: &mut usize,
    ui: &mut egui::Ui,
) {
    let preview = manuscript_preview(project, settings);
    let total = preview.page_count().max(1);
    if *page >= total {
        *page = total - 1;
    }

    // Flatten the section plans so one index selects a page across the whole
    // manuscript.
    let mut pages: Vec<(&PagePlan, &[PageLine])> = Vec::with_capacity(total);
    for plan in &preview.plans {
        for lines in &plan.pages {
            pages.push((plan, lines.as_slice()));
        }
    }
    let starts = preview.section_starts();

    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, "Live preview");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    meta_line(ui, &[&format!("{total} pages · approx.")]);
                });
            });
            ui.label(
                RichText::new("Approximate — rendered from your compile settings")
                    .font(sans(9.0))
                    .color(FAINT),
            );
            ui.add_space(8.0);

            // ---- Page and section navigation ----------------------------
            ui.horizontal(|ui| {
                let can_prev = *page > 0;
                let prev = ui.add_enabled(
                    can_prev,
                    egui::Button::new(
                        RichText::new("‹ Prev")
                            .font(sans(10.5))
                            .color(if can_prev { TEXT } else { FAINT }),
                    )
                    .corner_radius(CornerRadius::same(5)),
                );
                if prev.clicked() {
                    *page -= 1;
                }

                ui.label(
                    RichText::new(format!("Page {} of {}", *page + 1, total))
                        .font(sans(10.0))
                        .color(MUTED),
                );

                let can_next = *page + 1 < total;
                let next = ui.add_enabled(
                    can_next,
                    egui::Button::new(
                        RichText::new("Next ›")
                            .font(sans(10.5))
                            .color(if can_next { TEXT } else { FAINT }),
                    )
                    .corner_radius(CornerRadius::same(5)),
                );
                if next.clicked() {
                    *page += 1;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let current = section_for_page(&starts, *page);
                    let mut chosen = current;
                    egui::ComboBox::from_id_salt("preview_section")
                        .selected_text(
                            RichText::new(preview.sections[current].label.as_str())
                                .font(sans(10.0))
                                .color(TEXT),
                        )
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for (index, section) in preview.sections.iter().enumerate() {
                                let label =
                                    format!("{}  ·  page {}", section.label, starts[index] + 1);
                                ui.selectable_value(&mut chosen, index, label);
                            }
                        });
                    if chosen != current {
                        *page = starts.get(chosen).copied().unwrap_or(0).min(total - 1);
                    }
                    ui.label(RichText::new("Section").font(sans(9.5)).color(MUTED));
                });
            });
            ui.add_space(10.0);

            ScrollArea::horizontal()
                .id_salt("preview_page")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        if let Some((plan, lines)) = pages.get(*page) {
                            page_frame(ui, plan, lines, *page + 1);
                        }
                    });
                });

            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Each section starts on its own page, matching the exported manuscript.",
                )
                .font(sans(9.0))
                .color(FAINT),
            );
        });
}

/// Which section contains `page`, from the section start pages. The last
/// section that starts at or before the page wins.
fn section_for_page(starts: &[usize], page: usize) -> usize {
    starts
        .iter()
        .enumerate()
        .rev()
        .find(|(_, start)| **start <= page)
        .map(|(index, _)| index)
        .unwrap_or(0)
}
