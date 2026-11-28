//! Settings → Appearance: choose, import, duplicate and edit interface themes.
//! The colours themselves live in `theme`; this page only edits them.

use eframe::egui::{self, CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke};

use crate::app::{PortmanteauApp, ToastKind};
use crate::theme::{self, files, Palette};
use crate::ui::views::section_label;

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    {
        Frame::new()
            .fill(theme::PANEL())
            .stroke(Stroke::new(1.0, theme::BORDER()))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(14, 11))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                chooser(app, ui);
                ui.add_space(10.0);
                theme::hairline(ui);
                ui.add_space(8.0);
                let height = (ui.available_height() - 24.0).max(320.0);
                ui.columns(2, |cols| {
                    // The colour list scrolls on its own; the preview stays in view.
                    ScrollArea::vertical().id_salt("appearance_colours").max_height(height).auto_shrink([false, false]).show(&mut cols[0], |ui| editor(app, ui));
                    section_label(&mut cols[1], "Preview");
                    cols[1].add_space(4.0);
                    super::appearance_preview::show(&mut cols[1]);
                });
            });
    }
}

fn note(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).font(theme::ui_small()).color(theme::MUTED()));
}

fn chooser(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    section_label(ui, "Theme");
    ui.add_space(4.0);
    let active = app.settings.theme.clone();
    let mut pick = None;
    let shown = app
        .themes
        .iter()
        .find(|t| t.id == active)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Light".into());
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("theme_choice").width(220.0).selected_text(shown).show_ui(ui, |ui| {
            let mut last_builtin = true;
            for t in &app.themes {
                if last_builtin && !t.builtin {
                    ui.separator();
                    ui.label(RichText::new("Your themes").font(theme::ui_small()).color(theme::MUTED()));
                }
                last_builtin = t.builtin;
                if ui.selectable_label(t.id == active, &t.name).clicked() && t.id != active {
                    pick = Some(t.id.clone());
                }
            }
        });
        if ui.button("Duplicate").on_hover_text("Make an editable copy of this theme").clicked() {
            duplicate(app, ui.ctx());
        }
        if ui.button("Import…").on_hover_text("Add a theme file someone shared with you").clicked() {
            import(app, ui.ctx());
        }
        if ui.button("Export…").on_hover_text("Save this theme as a file to share").clicked() {
            export(app);
        }
        let user_theme = app.themes.iter().any(|t| t.id == active && !t.builtin);
        if ui.add_enabled(user_theme, egui::Button::new("Delete")).clicked() {
            delete_active(app, ui.ctx());
        }
    });
    if let Some(id) = pick {
        app.choose_theme(ui.ctx(), &id);
    }
    ui.add_space(4.0);
    note(ui, "Colours change everywhere at once. Manuscripts and exports are not affected: they have their own styles.");
    #[cfg(not(target_arch = "wasm32"))]
    note(ui, &format!("Your themes are the .json files in {}", files::themes_dir().display()));
    for problem in &app.theme_problems {
        ui.label(RichText::new(format!("Skipped {problem}")).font(theme::ui_small()).color(theme::DANGER()));
    }
}

fn duplicate(app: &mut PortmanteauApp, ctx: &egui::Context) {
    let Some(source) = app.themes.iter().find(|t| t.id == app.settings.theme).cloned() else { return };
    let copy = theme::Theme { name: format!("{} copy", source.name), ..source };
    match save_and_add(app, &copy) {
        Ok(id) => app.choose_theme(ctx, &id),
        Err(why) => app.toast("Theme not saved", why, ToastKind::Error),
    }
}

fn save_and_add(app: &mut PortmanteauApp, theme: &theme::Theme) -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let saved = files::save_new(&files::themes_dir(), theme)?;
    #[cfg(target_arch = "wasm32")]
    let saved = theme::Theme { id: format!("custom-{}", app.themes.len()), builtin: false, ..theme.clone() };
    let id = saved.id.clone();
    app.themes.push(saved);
    Ok(id)
}

fn import(app: &mut PortmanteauApp, ctx: &egui::Context) {
    let Some(path) = crate::platform::FileDialog::new()
        .set_title("Import a theme")
        .add_filter("Theme", &["json"])
        .pick_file()
    else {
        return;
    };
    #[cfg(not(target_arch = "wasm32"))]
    match files::import(&files::themes_dir(), &path) {
        Ok(theme) => {
            let (name, id) = (theme.name.clone(), theme.id.clone());
            app.themes.push(theme);
            app.choose_theme(ctx, &id);
            app.toast("Theme added", name, ToastKind::Info);
        }
        Err(why) => app.toast("That file is not a usable theme", why, ToastKind::Error),
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx, path);
}

fn export(app: &mut PortmanteauApp) {
    let Some(t) = app.themes.iter().find(|t| t.id == app.settings.theme).cloned() else { return };
    let Some(path) = crate::platform::FileDialog::new()
        .set_title("Export theme")
        .set_file_name(format!("{}.json", files::id_for_name(&t.name)))
        .save_file()
    else {
        return;
    };
    match std::fs::write(&path, t.to_json()) {
        Ok(()) => app.toast("Theme exported", path.display().to_string(), ToastKind::Info),
        Err(e) => app.toast("Theme not exported", e.to_string(), ToastKind::Error),
    }
}

fn delete_active(app: &mut PortmanteauApp, ctx: &egui::Context) {
    let Some(t) = app.themes.iter().find(|t| t.id == app.settings.theme).cloned() else { return };
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(why) = files::delete(&files::themes_dir(), &t) {
        app.toast("Theme not deleted", why, ToastKind::Error);
        return;
    }
    app.themes.retain(|x| x.id != t.id);
    app.choose_theme(ctx, "light");
}

/// The Global sliders: temporary hue / saturation / lightness shifts applied
/// over a snapshot of the colours taken when the sliders were last at zero.
/// Only the resulting colours are saved, never the slider positions.
#[derive(Clone)]
pub struct ThemeAdjust {
    id: String,
    base: Palette,
    hue: f32,
    saturation: f32,
    lightness: f32,
}

impl ThemeAdjust {
    fn fresh(id: &str, base: Palette) -> Self {
        Self { id: id.to_string(), base, hue: 0.0, saturation: 0.0, lightness: 0.0 }
    }
}

fn editor(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let active = app.settings.theme.clone();
    let Some(index) = app.themes.iter().position(|t| t.id == active) else { return };
    if app.themes[index].builtin {
        section_label(ui, "Colours");
        ui.add_space(4.0);
        note(ui, "Built-in themes stay as they are. Press Duplicate to make a copy you can recolour.");
        ui.add_space(6.0);
        for (group, items) in Palette::GROUPS {
            ui.add_space(8.0);
            section_label(ui, group);
            ui.add_space(3.0);
            for info in *items {
                if let Some(color) = app.themes[index].palette.get(info.key) {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 18.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, CornerRadius::same(4), color);
                        ui.painter().rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, theme::BORDER()), egui::StrokeKind::Inside);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(RichText::new(info.label).font(theme::ui_body()).color(theme::TEXT()));
                            ui.label(RichText::new(info.hint).font(theme::ui_small()).color(theme::MUTED()));
                        });
                    });
                    ui.add_space(3.0);
                }
            }
        }
        return;
    }
    let mut changed = false;
    section_label(ui, "Name");
    ui.add_space(4.0);
    changed |= ui.text_edit_singleline(&mut app.themes[index].name).changed();
    ui.add_space(4.0);
    // Global: shift every colour at once.
    if app.theme_adjust.as_ref().map(|a| a.id.as_str()) != Some(active.as_str()) {
        app.theme_adjust = Some(ThemeAdjust::fresh(&active, app.themes[index].palette));
    }
    ui.add_space(10.0);
    section_label(ui, "Global");
    ui.add_space(4.0);
    note(ui, "Slide every colour at once. The sliders start at zero each time and only the resulting colours are kept.");
    ui.add_space(4.0);
    if let Some(adjust) = app.theme_adjust.as_mut() {
        let mut moved = false;
        for (label, value, range, suffix) in [
            ("Hue", &mut adjust.hue, -180.0..=180.0, "°"),
            ("Saturation", &mut adjust.saturation, -100.0..=100.0, "%"),
            ("Lightness", &mut adjust.lightness, -100.0..=100.0, "%"),
        ] {
            ui.horizontal(|ui| {
                ui.add_sized([80.0, 18.0], egui::Label::new(RichText::new(label).font(theme::ui_body()).color(theme::TEXT())));
                moved |= ui.add(egui::Slider::new(value, range).suffix(suffix).integer()).changed();
            });
        }
        let at_zero = adjust.hue == 0.0 && adjust.saturation == 0.0 && adjust.lightness == 0.0;
        if ui.add_enabled(!at_zero, egui::Button::new("Reset sliders")).on_hover_text("Back to the colours as they were before the sliders moved").clicked() {
            adjust.hue = 0.0;
            adjust.saturation = 0.0;
            adjust.lightness = 0.0;
            moved = true;
        }
        if moved {
            app.themes[index].palette = adjust.base.adjusted(adjust.hue, adjust.saturation / 100.0, adjust.lightness / 100.0);
            changed = true;
        }
    }
    let mut hand_edited = false;
    for (group, items) in Palette::GROUPS {
        ui.add_space(10.0);
        section_label(ui, group);
        ui.add_space(4.0);
        for info in *items {
            let Some(mut color) = app.themes[index].palette.get(info.key) else { continue };
            ui.horizontal(|ui| {
                let alpha = if color.a() < 255 { egui::color_picker::Alpha::OnlyBlend } else { egui::color_picker::Alpha::Opaque };
                if egui::color_picker::color_edit_button_srgba(ui, &mut color, alpha).changed() {
                    app.themes[index].palette.set(info.key, color);
                    changed = true;
                    hand_edited = true;
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new(info.label).font(theme::ui_body()).color(theme::TEXT()));
                    ui.label(RichText::new(info.hint).font(theme::ui_small()).color(theme::MUTED()));
                });
            });
            ui.add_space(3.0);
        }
    }
    if hand_edited {
        // A colour picked by hand becomes part of the starting point, and the
        // sliders go back to zero over it.
        app.theme_adjust = Some(ThemeAdjust::fresh(&active, app.themes[index].palette));
    }
    if changed {
        let edited = app.themes[index].clone();
        theme::activate(&edited);
        theme::apply_style(ui.ctx());
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(why) = files::write(&files::themes_dir(), &edited) {
            app.toast("Theme not saved", why, ToastKind::Error);
        }
    }
}
