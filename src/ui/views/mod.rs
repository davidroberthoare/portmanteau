//! The non-writing workspaces: corkboard, outline, research, database,
//! version history, compilation and the command desk.

pub mod agent;
pub mod appearance;
pub mod appearance_preview;
pub mod database;
pub mod compile;
pub mod corkboard;
pub mod history;
pub mod outline;
pub mod project;
pub mod research;
pub mod search;
pub mod settings;

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea,
    Stroke, TextEdit, Vec2,
};
use portmanteau_core::{
    Alignment, CompileType, CustomStyle, ElementStyle, ElementStyles, ExportSettings, ParagraphStyle, Project,
    StyleSheet, TextTransform,
};

use crate::app::{PortmanteauApp, View};
use crate::theme::*;
use crate::i18n::tr;
use crate::ui::preview::{manuscript_layout, paint_page};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    match app.view {
        View::Corkboard => corkboard::show(app, ui),
        View::Outline => outline::show(app, ui),
        View::Research => research::show(app, ui),
        View::Search => search::show(app, ui),
        View::Database => database::show(app, ui),
        View::History => history::show(app, ui),
        View::Project => project::show(app, ui),
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
        .fill(BG())
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
                            .font(ui_small())
                            .color(ACCENT())
                            .extra_letter_spacing(1.5),
                    );
                    ui.add_space(2.0);
                    ui.label(RichText::new(title).font(serif(26.0)).color(TEXT()));
                    ui.add_space(2.0);
                    ui.label(RichText::new(subtitle).font(ui_body()).color(MUTED()));
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
                .font(ui_body())
                .color(ON_ACCENT()),
        )
        .fill(ACCENT())
        .corner_radius(CornerRadius::same(6))
        .min_size(Vec2::new(0.0, 29.0)),
    )
}

pub fn quiet_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).font(ui_body()).color(MUTED()))
            .fill(RAISED())
            .stroke(Stroke::new(1.0, BORDER()))
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
    ui.label(RichText::new(text).font(ui_small()).color(MUTED()));
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

/// The built-in elements that make sense for one kind of manuscript, in the
/// order a writer thinks of them.
pub fn elements_for(kind: CompileType) -> Vec<(&'static str, &'static str)> {
    const TITLE: [(&str, &str); 3] = TITLE_PAGE_ELEMENTS;
    let mut list: Vec<(&str, &str)> = TITLE.to_vec();
    let rest: &[(&str, &str)] = match kind {
        CompileType::Prose => &PROSE_ELEMENTS,
        CompileType::Screenplay => &[
            ("Scene heading", "scene_heading"),
            ("Action", "action"),
            ("Character", "character"),
            ("Parenthetical", "parenthetical"),
            ("Dialogue", "dialogue"),
            ("Transition", "transition"),
            ("Extension", "extension"),
            ("Intercut", "intercut"),
            ("Subheading", "subheading"),
        ],
        CompileType::TvScript => &[
            ("Act", "act"),
            ("Scene heading", "scene_heading"),
            ("Action", "action"),
            ("Character", "character"),
            ("Parenthetical", "parenthetical"),
            ("Dialogue", "dialogue"),
            ("Transition", "transition"),
            ("Extension", "extension"),
            ("Intercut", "intercut"),
            ("Subheading", "subheading"),
        ],
        CompileType::StagePlay => &[
            ("Act", "act"),
            ("Scene", "scene"),
            ("Character", "character"),
            ("Parenthetical", "parenthetical"),
            ("Dialogue", "dialogue"),
            ("Stage direction", "stage_direction"),
        ],
    };
    list.extend_from_slice(rest);
    list
}

/// The one style editor: pick the kind of manuscript, pick a style (title page
/// parts, headings, body, character cues, then the writer's own styles), and
/// edit it in a single form. Used by Compile (per output format) and by the
/// Settings defaults. `scope` keeps each use's picks apart. Returns true when
/// any style changed.
#[allow(deprecated)]
pub fn style_editor(
    ui: &mut egui::Ui,
    styles: &mut ElementStyles,
    scope: &str,
    default_type: CompileType,
) -> bool {
    let id = egui::Id::new(("style_editor_pick", scope));
    let (mut kind, mut key): (CompileType, String) = ui
        .data_mut(|d| d.get_temp(id))
        .unwrap_or((default_type, "body".to_owned()));
    let mut changed = false;
    let builtin = elements_for(kind);
    let valid = builtin.iter().any(|(_, k)| *k == key)
        || key
            .strip_prefix("custom:")
            .and_then(|i| i.parse::<usize>().ok())
            .is_some_and(|i| i < styles.custom.len());
    if !valid {
        key = builtin
            .iter()
            .find(|(_, k)| *k == "body" || *k == "action")
            .or(builtin.first())
            .map(|(_, k)| (*k).to_owned())
            .unwrap_or_default();
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Type")).font(ui_small()).color(MUTED()));
        egui::ComboBox::from_id_salt(("style_type", scope))
            .selected_text(RichText::new(crate::i18n::tr_dyn(kind.label())).font(ui_small()))
            .width(150.0)
            .show_ui(ui, |ui| {
                for t in CompileType::all() {
                    ui.selectable_value(&mut kind, t, crate::i18n::tr_dyn(t.label()));
                }
            });
    });
    ui.add_space(3.0);
    let current_label = match key.strip_prefix("custom:").and_then(|i| i.parse::<usize>().ok()) {
        Some(i) => styles.custom.get(i).map(|c| c.name.clone()).unwrap_or_default(),
        None => elements_for(kind)
            .iter()
            .find(|(_, k)| *k == key)
            .map(|(l, _)| (*l).to_owned())
            .unwrap_or_default(),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Style")).font(ui_small()).color(MUTED()));
        egui::ComboBox::from_id_salt(("style_pick", scope))
            .selected_text(RichText::new(current_label).font(ui_small()))
            .width(200.0)
            .show_ui(ui, |ui| {
                for (label, k) in elements_for(kind) {
                    ui.selectable_value(&mut key, k.to_owned(), label);
                }
                if !styles.custom.is_empty() {
                    ui.separator();
                }
                for (i, custom) in styles.custom.iter().enumerate() {
                    ui.selectable_value(&mut key, format!("custom:{i}"), &custom.name);
                }
            });
        if quiet_button(ui, tr("New style")).clicked() {
            let mut name = "New style".to_string();
            let mut n = 2;
            while styles.custom.iter().any(|c| c.name == name) || styles.element(&name).is_some() {
                name = format!("New style {n}");
                n += 1;
            }
            styles.custom.push(CustomStyle {
                name,
                style: ElementStyle::default(),
            });
            key = format!("custom:{}", styles.custom.len() - 1);
            changed = true;
        }
    });
    ui.add_space(6.0);

    let custom_index = key.strip_prefix("custom:").and_then(|i| i.parse::<usize>().ok());
    match custom_index {
        Some(index) if index < styles.custom.len() => {
            let mut remove = false;
            let mut draft = styles.custom[index].clone();
            let mut row = false;
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Name")).font(ui_small()).color(MUTED()));
                row |= ui
                    .add(TextEdit::singleline(&mut draft.name).font(ui_small()).desired_width(170.0))
                    .changed();
                if ui
                    .add(egui::Button::new(RichText::new(tr("Remove")).font(ui_small()).color(DANGER())))
                    .clicked()
                {
                    remove = true;
                }
            });
            ui.add_space(4.0);
            row |= element_row(ui, &format!("{scope}_custom_{index}"), "Paragraph", &mut draft.style);
            if remove {
                styles.custom.remove(index);
                key = "body".into();
                changed = true;
            } else if row {
                styles.custom[index] = draft;
                changed = true;
            }
        }
        _ => {
            let label = elements_for(kind)
                .iter()
                .find(|(_, k)| *k == key)
                .map(|(l, _)| *l)
                .unwrap_or("Style");
            if let Some(mut draft) = styles.element_mut(&key).cloned() {
                if element_row(ui, &format!("{scope}_{key}"), label, &mut draft) {
                    if let Some(target) = styles.element_mut(&key) {
                        *target = draft;
                        changed = true;
                    }
                }
            }
        }
    }
    ui.data_mut(|d| d.insert_temp(id, (kind, key)));
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
        .selected_text(RichText::new(selected).font(ui_small()).color(TEXT()))
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
        .selected_text(RichText::new(crate::i18n::tr_dyn(align.label())).font(ui_small()).color(TEXT()))
        .width(74.0)
        .show_ui(ui, |ui| {
            for option in Alignment::all() {
                if ui
                    .selectable_value(&mut align, option, crate::i18n::tr_dyn(option.label()))
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
        .fill(RAISED())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).font(ui_small()).color(TEXT()));
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

                    grid_label(ui, "Italic");
                    changed |= ui.checkbox(&mut element.italic, "").changed();
                    ui.end_row();
                });
        });
    changed
}

fn grid_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).font(ui_small()).color(MUTED()));
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
                .font(ui_body())
                .color(TEXT()),
        )
        .id_salt(("style_sheet", sheet.id.as_str()))
        .default_open(false)
        .show(ui, |ui| {
            ui.add_space(4.0);
            if sheet.medium.is_script() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("Page margins")).font(ui_small()).color(MUTED()));
                    for (label, field) in [
                        ("Left", &mut sheet.margins.left),
                        ("Right", &mut sheet.margins.right),
                        ("Top", &mut sheet.margins.top),
                        ("Bottom", &mut sheet.margins.bottom),
                    ] {
                        ui.label(RichText::new(label).font(ui_small()).color(MUTED()));
                        let mut inches = *field / 72.0;
                        if ui
                            .add(
                                egui::DragValue::new(&mut inches)
                                    .speed(0.05)
                                    .range(0.25..=3.0)
                                    .suffix("\""),
                            )
                            .changed()
                        {
                            *field = inches * 72.0;
                            changed = true;
                        }
                    }
                });
                ui.add_space(4.0);
            }
            if quiet_button(ui, tr("Reset to industry standard")).clicked() {
                *sheet = StyleSheet::for_medium(sheet.medium, &ElementStyles::default());
                changed = true;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Default paragraph")).font(ui_small()).color(MUTED()));
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
                    .fill(RAISED())
                    .stroke(Stroke::new(1.0, BORDER()))
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(Margin::symmetric(9, 6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(tr("Style")).font(ui_small()).color(MUTED()));
                            changed |= ui
                                .add(
                                    TextEdit::singleline(&mut draft.name)
                                        .font(ui_small())
                                        .desired_width(110.0),
                                )
                                .changed();
                            ui.label(RichText::new(tr("Enter →")).font(ui_small()).color(MUTED()));
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
                        if ui.checkbox(&mut uppercase, tr("Display uppercase")).changed() {
                            draft.text_transform = if uppercase {
                                TextTransform::Uppercase
                            } else {
                                TextTransform::None
                            };
                            changed = true;
                        }
                        egui::CollapsingHeader::new(
                            RichText::new(tr("Appearance"))
                                .font(ui_small())
                                .color(MUTED()),
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

            if quiet_button(ui, tr("Add custom paragraph style")).clicked() {
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
/// screen and Settings → Display.
///
/// The pages are the PDF writer's own layout (see `crate::ui::preview`), so the
/// page count and every line break match the exported PDF. `page` is the
/// caller's current 0-based page cursor; it is clamped when the page count
/// shrinks and moved when the section picker changes. `slot` names the cached
/// layout so two previews on screen never share one.
pub fn manuscript_preview_navigator(
    project: &Project,
    settings: &ExportSettings,
    page: &mut usize,
    max_height: f32,
    slot: &'static str,
    ui: &mut egui::Ui,
) {
    let layout = manuscript_layout(ui, project, settings, slot);
    let total = layout.page_count().max(1);
    if *page >= total {
        *page = total - 1;
    }
    let sections: Vec<(String, usize)> = if layout.sections.is_empty() {
        vec![("Manuscript".to_string(), 0)]
    } else {
        layout
            .sections
            .iter()
            .map(|section| (crate::i18n::tr_dyn(&section.label), section.start_page))
            .collect()
    };
    let starts: Vec<usize> = sections.iter().map(|(_, start)| *start).collect();

    Frame::new()
        .fill(BG())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, tr("Live preview"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let plural = if total == 1 { "" } else { "s" };
                    meta_line(ui, &[&format!("{total} page{plural}")]);
                });
            });
            ui.label(
                RichText::new(tr("The exported PDF's own pages, from your compile settings"))
                    .font(ui_small())
                    .color(MUTED()),
            );
            ui.add_space(8.0);

            // ---- Page and section navigation ----------------------------
            ui.horizontal(|ui| {
                let can_prev = *page > 0;
                let prev = ui.add_enabled(
                    can_prev,
                    egui::Button::new(
                        RichText::new(tr("‹ Prev"))
                            .font(ui_body())
                            .color(if can_prev { TEXT() } else { MUTED() }),
                    )
                    .corner_radius(CornerRadius::same(5)),
                );
                if prev.clicked() {
                    *page -= 1;
                }

                ui.label(
                    RichText::new(crate::i18n::trf("Page {page} of {total}", &[("page", (*page + 1).to_string()), ("total", total.to_string())]))
                        .font(ui_small())
                        .color(MUTED()),
                );

                let can_next = *page + 1 < total;
                let next = ui.add_enabled(
                    can_next,
                    egui::Button::new(
                        RichText::new(tr("Next ›"))
                            .font(ui_body())
                            .color(if can_next { TEXT() } else { MUTED() }),
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
                            RichText::new(sections[current].0.as_str())
                                .font(ui_small())
                                .color(TEXT()),
                        )
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for (index, (label, start)) in sections.iter().enumerate() {
                                let label = format!("{label}  ·  page {}", start + 1);
                                ui.selectable_value(&mut chosen, index, label);
                            }
                        });
                    if chosen != current {
                        *page = starts.get(chosen).copied().unwrap_or(0).min(total - 1);
                    }
                    ui.label(RichText::new(tr("Section")).font(ui_small()).color(MUTED()));
                });
            });
            ui.add_space(10.0);

            // Scale the page down so a whole page fits the column, both
            // across and down; the header, navigation and footer take the rest.
            let chrome_height = 190.0;
            let page_height_budget = (max_height - chrome_height).max(240.0);
            let width_budget = (ui.available_width() - 8.0).max(160.0);
            ScrollArea::horizontal()
                .id_salt("preview_page")
                .auto_shrink([false, false])
                .max_height(page_height_budget + 12.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let scale = (width_budget / layout.width.max(1.0))
                            .min(page_height_budget / layout.height.max(1.0))
                            .clamp(0.2, 2.0);
                        paint_page(ui, &layout, *page, scale);
                    });
                });
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
