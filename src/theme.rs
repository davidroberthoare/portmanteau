//! Palette, typography and shared visual language.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Visuals,
};

mod palette;
pub use palette::*;
#[cfg(not(target_arch = "wasm32"))]
pub mod files;

pub fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("portmanteau-serif".into()))
}

static UI_SCALE: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0x3f80_0000); // 1.0

/// Choices offered by Settings → Interface text size.
pub const UI_SCALE_PRESETS: [(&str, f32); 3] = [("Standard", 1.0), ("Large", 1.15), ("Larger", 1.3)];

pub fn set_ui_scale(scale: f32) {
    UI_SCALE.store(
        scale.clamp(0.85, 1.4).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
}

fn ui_scale() -> f32 {
    f32::from_bits(UI_SCALE.load(std::sync::atomic::Ordering::Relaxed))
}

// Interface type scale, in points at 100% text size. Every piece of chrome
// text uses one of these roles, so changing a value here (or the user's
// "Interface text size" preference, which scales all of them) restyles the
// whole app consistently.
/// The zoom control reads 100% at its default. Each medium maps that to the
/// on-screen scale of its stylesheet size that reads comfortably.
pub const DEFAULT_EDITOR_ZOOM: f32 = 1.0;
pub const PROSE_ZOOM_BASE: f32 = 1.4;
pub const SCRIPT_ZOOM_BASE: f32 = 1.2;
/// Height shared by every control in the editor toolbar so they sit on one line.
pub const TOOLBAR_CONTROL_H: f32 = 28.0;
/// Height of the toolbar row itself; combo boxes and the Spelling button fill it.
pub const TOOLBAR_ROW_H: f32 = 32.0;
/// Extra magnification for the small inspector/research text areas so their
/// stylesheet text reads like the rest of the interface.
pub const FIELD_TEXT_ZOOM: f32 = 1.25;

pub fn field_text_zoom() -> f32 {
    FIELD_TEXT_ZOOM * ui_scale()
}
/// Height of buttons nested inside a padded toolbar group.
pub const TOOLBAR_GROUP_H: f32 = TOOLBAR_CONTROL_H - 4.0;

pub const FONT_SMALL_PT: f32 = 11.5;
pub const FONT_BODY_PT: f32 = 13.0;
pub const FONT_STRONG_PT: f32 = 14.5;

fn sans_family() -> FontFamily {
    FontFamily::Name("portmanteau-sans".into())
}

/// Captions, hints, counts and section labels.
pub fn ui_small() -> FontId {
    FontId::new(FONT_SMALL_PT * ui_scale(), sans_family())
}

/// Buttons, menus, list rows and field text.
pub fn ui_body() -> FontId {
    FontId::new(FONT_BODY_PT * ui_scale(), sans_family())
}

/// Emphasised controls and panel titles.
pub fn ui_strong() -> FontId {
    FontId::new(FONT_STRONG_PT * ui_scale(), sans_family())
}

/// Sans text at an explicit size (scaled by the text-size preference). Prefer
/// the role functions above.
pub fn sans(size: f32) -> FontId {
    FontId::new(size * ui_scale(), sans_family())
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Map a style's font-family name to one of the bundled egui families. Used by
/// the Settings preview, which approximates the compiled page on screen.
pub fn family_for_font(font: &str) -> FontFamily {
    let name = font.to_lowercase();
    if name.contains("mono") || name.contains("courier") {
        FontFamily::Monospace
    } else if name.contains("sans")
        || name.contains("helvetica")
        || name.contains("arial")
        || name.contains("dejavu")
    {
        FontFamily::Name("portmanteau-sans".into())
    } else {
        FontFamily::Name("portmanteau-serif".into())
    }
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "portmanteau-serif".into(),
        FontData::from_static(include_bytes!("../assets/LiberationSerif-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "portmanteau-serif-italic".into(),
        FontData::from_static(include_bytes!("../assets/LiberationSerif-Italic.ttf")).into(),
    );
    fonts.font_data.insert(
        "portmanteau-sans".into(),
        FontData::from_static(include_bytes!("../assets/LiberationSans-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "portmanteau-sans-bold".into(),
        FontData::from_static(include_bytes!("../assets/LiberationSans-Bold.ttf")).into(),
    );
    fonts.font_data.insert(
        "portmanteau-mono".into(),
        FontData::from_static(include_bytes!("../assets/LiberationMono-Regular.ttf")).into(),
    );
    // A broad fallback for the small set of typographic marks used in the
    // binder and toolbars (¶, ▸, ✎, ＋). Liberation does not carry them.
    fonts.font_data.insert(
        "portmanteau-symbols".into(),
        FontData::from_static(include_bytes!("../assets/DejaVuSans.ttf")).into(),
    );
    fonts
        .families
        .entry(FontFamily::Name("portmanteau-serif".into()))
        .or_default()
        .insert(0, "portmanteau-serif".into());
    fonts
        .families
        .entry(FontFamily::Name("portmanteau-sans".into()))
        .or_default()
        .insert(0, "portmanteau-sans".into());
    fonts
        .families
        .entry(FontFamily::Name("portmanteau-sans-bold".into()))
        .or_default()
        .insert(0, "portmanteau-sans-bold".into());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "portmanteau-mono".into());
    for family in [
        FontFamily::Name("portmanteau-serif".into()),
        FontFamily::Name("portmanteau-sans".into()),
        FontFamily::Name("portmanteau-sans-bold".into()),
        FontFamily::Monospace,
    ] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("portmanteau-symbols".into());
    }
    // One egui family per face the exporters lay out with, so the manuscript
    // preview draws exactly the glyphs (and so the widths) the PDF writer
    // measured. See `crate::ui::preview`.
    for family in ["serif", "sans", "mono"] {
        for (suffix, style) in [("Regular", ""), ("Bold", "-bold"), ("Italic", "-italic")] {
            let key = format!("page-{family}{style}");
            let bytes: &'static [u8] = match (family, suffix) {
                ("serif", "Regular") => include_bytes!("../assets/LiberationSerif-Regular.ttf"),
                ("serif", "Bold") => include_bytes!("../assets/LiberationSerif-Bold.ttf"),
                ("serif", _) => include_bytes!("../assets/LiberationSerif-Italic.ttf"),
                ("sans", "Regular") => include_bytes!("../assets/LiberationSans-Regular.ttf"),
                ("sans", "Bold") => include_bytes!("../assets/LiberationSans-Bold.ttf"),
                ("sans", _) => include_bytes!("../assets/LiberationSans-Italic.ttf"),
                ("mono", "Regular") => include_bytes!("../assets/LiberationMono-Regular.ttf"),
                ("mono", "Bold") => include_bytes!("../assets/LiberationMono-Bold.ttf"),
                _ => include_bytes!("../assets/LiberationMono-Italic.ttf"),
            };
            fonts.font_data.insert(key.clone(), FontData::from_static(bytes).into());
            fonts
                .families
                .entry(FontFamily::Name(key.clone().into()))
                .or_default()
                .extend([key, "portmanteau-symbols".into()]);
        }
    }
    fonts
        .families
        .entry(FontFamily::Name("page-fallback".into()))
        .or_default()
        .push("portmanteau-symbols".into());
    // Keep the default emoji/symbol fonts available behind the Latin faces.
    ctx.set_fonts(fonts);

    apply_style(ctx);
}

/// Set egui's own style (controls, spacing, text sizes) from the current
/// theme and interface scale. Cheap enough to call when either changes.
pub fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.style_of(egui::Theme::Light)).clone();
    style.text_styles = [
        (TextStyle::Heading, serif(26.0)),
        (TextStyle::Body, ui_body()),
        (TextStyle::Button, ui_body()),
        (TextStyle::Small, ui_small()),
        (TextStyle::Monospace, mono(12.5)),
    ]
    .into();
    style.animation_time = 0.12;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = 28.0 * ui_scale().max(1.0);

    let mut visuals = if current_dark() { Visuals::dark() } else { Visuals::light() };
    visuals.panel_fill = PANEL();
    visuals.window_fill = PANEL();
    visuals.extreme_bg_color = BG();
    visuals.faint_bg_color = RAISED();
    visuals.code_bg_color = RAISED();
    visuals.override_text_color = Some(TEXT());
    visuals.hyperlink_color = ACCENT();
    visuals.warn_fg_color = WARNING();
    visuals.error_fg_color = DANGER();
    visuals.selection.bg_fill = ACCENT_SOFT();
    visuals.selection.stroke = Stroke::new(1.0, ACCENT());
    visuals.widgets.noninteractive.bg_fill = PANEL();
    visuals.widgets.noninteractive.weak_bg_fill = PANEL();
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER());
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT());
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    for w in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(6);
    }
    visuals.widgets.inactive.bg_fill = RAISED();
    visuals.widgets.inactive.weak_bg_fill = RAISED();
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER());
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT());
    visuals.widgets.hovered.bg_fill = HOVER();
    visuals.widgets.hovered.weak_bg_fill = HOVER();
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER());
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT());
    visuals.widgets.active.bg_fill = ACCENT_SOFT();
    visuals.widgets.active.weak_bg_fill = ACCENT_SOFT();
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT());
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ACCENT());
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(28),
    };
    visuals.popup_shadow = visuals.window_shadow;
    style.visuals = visuals;
    // The theme (light or dark) fills the one egui style slot we use.
    ctx.set_theme(egui::Theme::Light);
    ctx.set_style_of(egui::Theme::Light, style);
}

pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .font(sans(10.5))
            .color(MUTED())
            .extra_letter_spacing(1.1),
    );
}

pub fn hairline(ui: &mut egui::Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, BORDER()),
    );
}

pub fn pill(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(20))
        .inner_margin(Margin::symmetric(9, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).font(sans(10.5)).color(fg));
        });
}

pub fn dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}

pub fn label_color(label: &str) -> Color32 {
    match label {
        "Opening" => ACCENT(),
        "Development" => INFO(),
        "Reveal" => DANGER(),
        "Script" => SPECIAL(),
        "Stage" => SUCCESS(),
        "Revised" => SUCCESS(),
        "Draft" => INFO(),
        "To do" => MUTED(),
        _ => MUTED(),
    }
}

pub fn status_color(status: &str) -> Color32 {
    match status {
        "Revised" | "Final" | "Done" => SUCCESS(),
        "Draft" | "In progress" => ACCENT(),
        "To do" | "Outline" => MUTED(),
        _ => MUTED(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_for_font_picks_bundled_families() {
        let sans = FontFamily::Name("portmanteau-sans".into());
        let serif = FontFamily::Name("portmanteau-serif".into());
        assert_eq!(family_for_font("Liberation Mono"), FontFamily::Monospace);
        assert_eq!(family_for_font("Courier New"), FontFamily::Monospace);
        assert_eq!(family_for_font("Liberation Sans"), sans);
        assert_eq!(family_for_font("Helvetica"), sans);
        assert_eq!(family_for_font("Arial"), sans);
        assert_eq!(family_for_font("DejaVu Sans"), sans);
        assert_eq!(family_for_font("Liberation Serif"), serif);
        assert_eq!(family_for_font("Times New Roman"), serif);
        assert_eq!(family_for_font("Georgia"), serif);
        assert_eq!(family_for_font(""), serif);
    }
}
