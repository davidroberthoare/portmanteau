//! Settings: the global house style, edited beside a live manuscript preview.
//! Defaults live in a config file outside any project; a new project starts
//! from them, and the current project can be updated to match at any time.

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Vec2,
};
use portmanteau_core::{CompileType, ElementStyles, ExportKind, StyleSheet};

use crate::app::{PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::i18n::tr;
use crate::ui::views::{
    content_frame, manuscript_preview_navigator, style_editor,
    meta_line, page_head, primary_button, quiet_button, section_label, style_sheets_editor,
    BUILTIN_ELEMENTS,
};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        tr("Defaults"),
        tr("Settings"),
        tr("The house style new projects start from, with a live manuscript preview."),
        |ui| {
            meta_line(
                ui,
                &[&crate::i18n::trf(
                    "{built_in} built-in + {custom} custom element styles",
                    &[("built_in", BUILTIN_ELEMENTS.len().to_string()), ("custom", app.settings.styles.custom.len().to_string())],
                )],
            );
        },
    );

    content_frame().show(ui, |ui| {
        let height = (ui.available_height() - 4.0).max(420.0);
        ui.set_width(ui.available_width());
        tab_row(app, ui);
        ui.add_space(12.0);
        match app.settings_tab {
            1 => display_tab(app, ui),
            2 => super::appearance::show(app, ui),
            _ => {
                ScrollArea::vertical()
                    .id_salt("settings_content")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        preferences_tab(app, ui);
                    });
            }
        }
    });
}

/// The Settings tabs: Preferences (future options) and Display (default styles
/// and the manuscript preview).
fn tab_row(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if settings_tab_button(ui, tr("Preferences"), app.settings_tab == 0) {
            app.settings_tab = 0;
        }
        ui.add_space(4.0);
        if settings_tab_button(ui, tr("Display"), app.settings_tab == 1) {
            app.settings_tab = 1;
        }
        ui.add_space(4.0);
        if settings_tab_button(ui, tr("Appearance"), app.settings_tab == 2) {
            app.settings_tab = 2;
        }
    });
}

fn settings_tab_button(ui: &mut egui::Ui, text: &str, selected: bool) -> bool {
    let button = egui::Button::new(
        RichText::new(text)
            .font(ui_body())
            .color(if selected { TEXT() } else { MUTED() }),
    )
    .fill(if selected { ACCENT_SOFT() } else { RAISED() })
    .stroke(Stroke::new(
        1.0,
        if selected {
            ACCENT().gamma_multiply(0.75)
        } else {
            BORDER()
        },
    ))
    .corner_radius(CornerRadius::same(6))
    .min_size(Vec2::new(112.0, 30.0));
    ui.add(button).clicked()
}

fn preferences_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut changed = false;
    let width = ui.available_width().min(704.0);
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(width);
            section_label(ui, tr("Writing"));
            ui.add_space(4.0);
            changed |= ui
                .checkbox(
                    &mut app.settings.spellcheck,
                    RichText::new(tr("Spell-checking in editors"))
                        .font(ui_body())
                        .color(MUTED()),
                )
                .changed();
            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    tr("The Write toolbar shows a Spelling panel with suggestions. Changes here are saved automatically."),
                )
                .font(ui_small())
                .color(MUTED()),
            );
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            section_label(ui, tr("Software updates"));
            ui.add_space(4.0);
            changed |= ui
                .checkbox(
                    &mut app.settings.updates.auto_check,
                    RichText::new(tr("Check for updates automatically"))
                        .font(ui_body())
                        .color(MUTED()),
                )
                .changed();
            ui.add_space(2.0);
            let last = app
                .settings
                .updates
                .last_checked
                .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
                .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "never".into());
            ui.label(
                RichText::new(crate::i18n::trf(
                    "At most once a day, at startup. It only asks GitHub for the newest release number; nothing about you or your projects is sent. Last checked: {last}.",
                    &[("last", last.to_string())],
                ))
                .font(ui_small())
                .color(MUTED()),
            );
            ui.add_space(4.0);
            if ui.button(tr("Check now")).clicked() {
                app.updater.panel_open = true;
                app.updater.start(ui.ctx(), true);
            }
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            section_label(ui, tr("Interface text size"));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (label, value) in crate::theme::UI_SCALE_PRESETS {
                    let active = (app.settings.ui_scale - value).abs() < 0.04;
                    if ui.selectable_label(active, crate::i18n::scale_name(label)).clicked() && !active {
                        app.settings.ui_scale = value;
                        crate::theme::set_ui_scale(value);
                        crate::theme::install(ui.ctx());
                        changed = true;
                    }
                }
            });
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            section_label(ui, tr("Interface language"));
            ui.add_space(4.0);
            interface_language(app, ui);
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            section_label(ui, tr("Spelling language"));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let current = app.settings.language.clone();
                let selected_label = crate::spellcheck::LANGUAGES
                    .iter()
                    .find(|(code, _)| *code == current)
                    .map(|(_, label)| *label)
                    .unwrap_or("English (US)");
                let mut change: Option<&'static str> = None;
                egui::ComboBox::from_id_salt("spelling_language")
                    .selected_text(RichText::new(selected_label).font(ui_body()).color(TEXT()))
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for (code, label) in crate::spellcheck::LANGUAGES {
                            if ui.selectable_label(current == *code, *label).clicked() {
                                change = Some(*code);
                            }
                        }
                    });
                if let Some(code) = change {
                    app.set_spell_language(code);
                }
                ui.label(
                    RichText::new(tr("Applies to every editor and new projects."))
                        .font(ui_small())
                        .color(MUTED()),
                );
            });
            ui.add_space(10.0);
            hairline(ui);
            ui.add_space(8.0);
            section_label(ui, tr("Workspace"));
            ui.add_space(4.0);
            changed |= ui
                .checkbox(
                    &mut app.settings.show_binder,
                    RichText::new(tr("Show the binder in Write"))
                        .font(ui_body())
                        .color(MUTED()),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.settings.show_inspector,
                    RichText::new(tr("Show the inspector in Write"))
                        .font(ui_body())
                        .color(MUTED()),
                )
                .changed();
            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    tr("The sidebar layout is remembered between launches. Focus mode (F11) hides both panels without changing this preference."),
                )
                .font(ui_small())
                .color(MUTED()),
            );
        });
    if changed {
        if crate::settings::save_settings(&app.settings).is_ok() {
            app.toast(
                tr("Preferences saved"),
                crate::settings::config_path().display().to_string(),
                ToastKind::Info,
            );
        }
    }
}

/// Two columns, mirroring Compile: the default element styles on the left and
/// the manuscript preview on the right. The preview uses the defaults as they
/// are edited, so it is the only proof on this tab.
fn display_tab(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let mut save = false;
    let mut apply = false;
    let mut reset = false;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(tr("Display")).font(serif(21.0)).color(TEXT()));
            ui.label(
                RichText::new(
                    tr("The house style new projects start from — and the same type the editors use."),
                )
                .font(ui_small())
                .color(MUTED()),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            pill(
                ui,
                &crate::i18n::trf(
                    "{built_in} built-in + {custom} custom",
                    &[("built_in", BUILTIN_ELEMENTS.len().to_string()), ("custom", app.settings.styles.custom.len().to_string())],
                ),
                MUTED(),
                RAISED(),
            );
        });
    });
    ui.add_space(8.0);

    let height = ui.available_height().max(420.0);
    ui.horizontal_top(|ui| {
        // ---- Left column: the default element styles -------------------
        ui.allocate_ui_with_layout(
            Vec2::new(440.0, height),
            Layout::top_down(Align::Min),
            |ui| {
                ScrollArea::vertical()
                    .id_salt("settings_default_styles")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        Frame::new()
                            .fill(PANEL())
                            .stroke(Stroke::new(1.0, BORDER()))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::symmetric(14, 11))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                section_label(ui, tr("Display and default styles"));
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(
                                        tr("Type defaults for every manuscript element. The preview uses them; apply them to a project, then each output tab refines per format."),
                                    )
                                    .font(ui_small())
                                    .color(MUTED()),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "Config · {}",
                                        crate::settings::config_path().display()
                                    ))
                                    .font(mono(8.5))
                                    .color(MUTED()),
                                );
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if primary_button(ui, tr("Save defaults")).clicked() {
                                        save = true;
                                    }
                                    if quiet_button(ui, tr("Apply to current project")).clicked() {
                                        apply = true;
                                    }
                                    if quiet_button(ui, tr("Reset defaults")).clicked() {
                                        reset = true;
                                    }
                                });
                                ui.add_space(8.0);
                                hairline(ui);
                                ui.add_space(8.0);

                                let base_styles_changed = style_editor(
                                    ui,
                                    &mut app.settings.styles,
                                    "settings",
                                    CompileType::Prose,
                                );
                                if base_styles_changed {
                                    app.settings.sync_sheet_formats_from_element_styles();
                                }
                                ui.add_space(8.0);
                                section_label(ui, tr("Writing behavior · per-format styles (film, TV, stage)"));
                                if style_sheets_editor(ui, &mut app.settings.style_sheets) {
                                    app.settings.sync_element_styles_from_sheets();
                                }
                            });
                    });
            },
        );
        ui.add_space(16.0);

        // ---- Right column: preview the manuscript with these defaults ---
        let preview_width = ui.available_width();
        ui.allocate_ui_with_layout(
            Vec2::new(preview_width, height),
            Layout::top_down(Align::Min),
            |ui| {
                ScrollArea::vertical()
                    .id_salt("settings_preview")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let mut settings = app.ws.project.settings_for(ExportKind::Pdf);
                        settings.styles = app.settings.styles.clone();
                        manuscript_preview_navigator(
                            &app.ws.project,
                            &settings,
                            &mut app.settings_preview_page,
                            height,
                            "settings",
                            ui,
                        );
                    });
            },
        );
    });

    if reset {
        app.settings.styles = ElementStyles::default();
        app.settings.style_sheets = StyleSheet::industry_defaults(&app.settings.styles);
        app.toast(
            tr("Defaults reset"),
            "Built-in element styles restored — save to keep them",
            ToastKind::Info,
        );
    }
    if save {
        match crate::settings::save_settings(&app.settings) {
            Ok(()) => app.toast(
                tr("Defaults saved"),
                crate::settings::config_path().display().to_string(),
                ToastKind::Ok,
            ),
            Err(e) => app.toast(tr("Could not save defaults"), e, ToastKind::Error),
        }
    }
    if apply {
        app.ws.project.styles = app.settings.element_styles_for_new_project();
        app.ws.project.style_sheets = app.settings.style_sheets_for_new_project();
        app.ws.dirty = true;
        app.toast(
            tr("Defaults applied"),
            tr("The current project now uses the global element styles"),
            ToastKind::Ok,
        );
    }
}

/// Pick the language the menus and labels are shown in, add one from a file,
/// or export the template to start a translation.
fn interface_language(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let active = app.settings.interface_language.clone();
    let shown = app
        .languages
        .iter()
        .find(|l| l.code == active)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| "English".into());
    let mut pick = None;
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("interface_language").width(200.0).selected_text(shown).show_ui(ui, |ui| {
            for l in &app.languages {
                if ui.selectable_label(l.code == active, &l.name).clicked() && l.code != active {
                    pick = Some(l.code.clone());
                }
            }
        });
        if ui.button(tr("Import…")).on_hover_text(tr("Add a language file someone shared with you")).clicked() {
            import_language(app);
        }
        if ui.button(tr("Export template…")).on_hover_text(tr("Save an empty language file to translate")).clicked() {
            export_language_template(app, None);
        }
        if active != "en"
            && ui.button(tr("Export this language…")).on_hover_text(tr("Save this language with its translations filled in, to correct or finish it")).clicked()
        {
            export_language_template(app, Some(active.clone()));
        }
    });
    if let Some(code) = pick {
        app.choose_language(&code);
    }
    ui.add_space(2.0);
    ui.label(
        RichText::new(tr("Menus and labels use this language. Anything not yet translated stays in English. Your own text, and your exports, are never translated."))
            .font(ui_small())
            .color(MUTED()),
    );
    #[cfg(not(target_arch = "wasm32"))]
    ui.label(
        RichText::new(format!("{} {}", tr("Language files go in"), crate::i18n::files::languages_dir().display()))
            .font(ui_small())
            .color(MUTED()),
    );
    for problem in &app.language_problems {
        ui.label(RichText::new(format!("{} {problem}", tr("Skipped"))).font(ui_small()).color(DANGER()));
    }
}

fn import_language(app: &mut PortmanteauApp) {
    let Some(path) = crate::platform::FileDialog::new()
        .set_title(tr("Import a language"))
        .add_filter("Language", &["json"])
        .pick_file()
    else {
        return;
    };
    #[cfg(not(target_arch = "wasm32"))]
    match crate::i18n::files::import(&crate::i18n::files::languages_dir(), &path) {
        Ok(language) => {
            let (name, code) = (language.name.clone(), language.code.clone());
            match app.languages.iter().position(|l| l.code == code) {
                Some(index) => app.languages[index] = language,
                None => app.languages.push(language),
            }
            app.choose_language(&code);
            app.toast(tr("Language added"), name, ToastKind::Info);
        }
        Err(why) => app.toast(tr("That file is not a usable language"), why, ToastKind::Error),
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, path);
}

/// Save the empty template, or (given a language code) that language with its
/// translations filled in.
fn export_language_template(app: &mut PortmanteauApp, code: Option<String>) {
    let language = code.and_then(|c| app.languages.iter().find(|l| l.code == c).cloned());
    let file_name = language.as_ref().map_or("my-language.json".to_string(), |l| format!("{}.json", l.code));
    let Some(path) = crate::platform::FileDialog::new()
        .set_title(tr("Save the language template"))
        .set_file_name(&file_name)
        .save_file()
    else {
        return;
    };
    let contents = language.map_or_else(|| crate::i18n::TEMPLATE.to_string(), |l| l.to_filled_template());
    match std::fs::write(&path, contents) {
        Ok(()) => app.toast(tr("Template saved"), path.display().to_string(), ToastKind::Info),
        Err(e) => app.toast(tr("Template not saved"), e.to_string(), ToastKind::Error),
    }
}
