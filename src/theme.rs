//! Palette, typography and shared visual language.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Visuals,
};

pub const BG: Color32 = Color32::from_rgb(18, 20, 24);
pub const RAIL: Color32 = Color32::from_rgb(14, 16, 19);
pub const PANEL: Color32 = Color32::from_rgb(25, 28, 33);
pub const PANEL_2: Color32 = Color32::from_rgb(31, 35, 41);
pub const PANEL_3: Color32 = Color32::from_rgb(39, 44, 51);
pub const BORDER: Color32 = Color32::from_rgb(48, 53, 62);
pub const BORDER_SOFT: Color32 = Color32::from_rgb(39, 43, 50);
pub const TEXT: Color32 = Color32::from_rgb(233, 228, 218);
pub const MUTED: Color32 = Color32::from_rgb(148, 151, 158);
pub const FAINT: Color32 = Color32::from_rgb(104, 109, 118);
pub const ACCENT: Color32 = Color32::from_rgb(215, 166, 92);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(74, 60, 38);
pub const SAGE: Color32 = Color32::from_rgb(132, 170, 145);
pub const SAGE_SOFT: Color32 = Color32::from_rgb(36, 52, 44);
pub const BLUE: Color32 = Color32::from_rgb(124, 156, 196);
pub const RED: Color32 = Color32::from_rgb(201, 122, 108);
pub const PURPLE: Color32 = Color32::from_rgb(166, 143, 185);
pub const PAPER: Color32 = Color32::from_rgb(246, 241, 231);
pub const PAPER_EDGE: Color32 = Color32::from_rgb(226, 219, 205);
pub const PAPER_TEXT: Color32 = Color32::from_rgb(40, 37, 32);
pub const PAPER_MUTED: Color32 = Color32::from_rgb(128, 119, 103);

pub fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("portmanteau-serif".into()))
}

pub fn sans(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("portmanteau-sans".into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
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
    // binder and toolbars (¶, ▸, ⌕, ＋). Liberation does not carry them.
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
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "portmanteau-mono".into());
    for family in [
        FontFamily::Name("portmanteau-serif".into()),
        FontFamily::Name("portmanteau-sans".into()),
        FontFamily::Monospace,
    ] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("portmanteau-symbols".into());
    }
    // Keep the default emoji/symbol fonts available behind the Latin faces.
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, serif(26.0)),
        (TextStyle::Body, sans(14.0)),
        (TextStyle::Button, sans(13.5)),
        (TextStyle::Small, sans(11.0)),
        (TextStyle::Monospace, mono(12.5)),
    ]
    .into();
    style.animation_time = 0.12;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = 26.0;

    let mut visuals = Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL_2;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = PANEL_2;
    visuals.code_bg_color = Color32::from_rgb(16, 18, 22);
    visuals.override_text_color = Some(TEXT);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = ACCENT;
    visuals.error_fg_color = RED;
    visuals.selection.bg_fill = Color32::from_rgb(91, 73, 42);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_SOFT);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    for w in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(6);
    }
    visuals.widgets.inactive.bg_fill = PANEL_2;
    visuals.widgets.inactive.weak_bg_fill = PANEL_2;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_SOFT);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.hovered.bg_fill = PANEL_3;
    visuals.widgets.hovered.weak_bg_fill = PANEL_3;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.active.bg_fill = ACCENT_SOFT;
    visuals.widgets.active.weak_bg_fill = ACCENT_SOFT;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ACCENT);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 12],
        blur: 40,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    visuals.popup_shadow = visuals.window_shadow;
    style.visuals = visuals;
    ctx.set_style(style);
}

pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .font(sans(10.5))
            .color(FAINT)
            .extra_letter_spacing(1.1),
    );
}

pub fn hairline(ui: &mut egui::Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, BORDER_SOFT),
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

pub fn progress_bar(ui: &mut egui::Ui, fraction: f32, color: Color32, height: f32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same((height / 2.0) as u8), PANEL_3);
    let filled = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
    );
    painter.rect_filled(
        filled,
        CornerRadius::same((height / 2.0) as u8),
        color,
    );
}

pub fn dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}

pub fn label_color(label: &str) -> Color32 {
    match label {
        "Opening" => ACCENT,
        "Development" => BLUE,
        "Reveal" => RED,
        "Script" => PURPLE,
        "Stage" => SAGE,
        "Revised" => SAGE,
        "Draft" => BLUE,
        "To do" => FAINT,
        _ => MUTED,
    }
}

pub fn status_color(status: &str) -> Color32 {
    match status {
        "Revised" | "Final" | "Done" => SAGE,
        "Draft" | "In progress" => ACCENT,
        "To do" | "Outline" => FAINT,
        _ => MUTED,
    }
}
