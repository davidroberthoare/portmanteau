//! Small pieces shared by the Search page and the editor's find bar: the
//! three option toggles (match case, whole word, regular expression).

use crate::i18n::tr;
use eframe::egui::{self, CornerRadius, RichText, Stroke, Vec2};
use portmanteau_core::FindOptions;

use crate::theme::*;

/// A little toggle button that shows whether it is on. Returns true when clicked.
pub fn toggle(ui: &mut egui::Ui, text: &str, on: bool, hover: &str) -> bool {
    let button = egui::Button::new(RichText::new(text).font(mono(12.0)).color(if on { ACCENT() } else { MUTED() }))
        .fill(if on { ACCENT_SOFT() } else { egui::Color32::TRANSPARENT })
        .stroke(Stroke::new(1.0, if on { ACCENT() } else { egui::Color32::TRANSPARENT }))
        .corner_radius(CornerRadius::same(4))
        .min_size(Vec2::new(26.0, 22.0));
    ui.add(button).on_hover_text(hover).clicked()
}

/// Match case (Aa), whole word (ab) and regular expression (.*). True if any changed.
pub fn option_toggles(ui: &mut egui::Ui, options: &mut FindOptions) -> bool {
    let mut changed = false;
    if toggle(ui, "Aa", options.case_sensitive, tr("Match case")) {
        options.case_sensitive = !options.case_sensitive;
        changed = true;
    }
    if toggle(ui, "ab", options.whole_word, tr("Match whole word")) {
        options.whole_word = !options.whole_word;
        changed = true;
    }
    if toggle(ui, ".*", options.regex, tr("Use a regular expression (replacement may use $1)")) {
        options.regex = !options.regex;
        changed = true;
    }
    changed
}
