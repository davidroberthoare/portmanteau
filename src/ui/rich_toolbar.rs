//! A fixed formatting toolbar for any rich-text field.
//!
//! Put it above a `TextEdit` that edits a `RichDocument` through the editor's
//! usual layouter:
//!
//! ```ignore
//! let bar = rich_toolbar::show(ui, field_id, &mut content, &Options::default());
//! // … show the TextEdit with `.id(field_id)` …
//! // after syncing the edited text back into `content`:
//! rich_toolbar::after_edit(ctx, field_id, &mut content, caret);
//! ```
//!
//! It works on the field's selection, or on the caret: with nothing selected
//! a button means "the text I type next", held as *pending marks* until the
//! text arrives. The toolbar knows nothing about what the field belongs to, so
//! the same bar can go on a note, a synopsis or a manuscript. It sits beside
//! the floating popup (`inline_format_toolbar`) and shares its link dialog.

use std::ops::Range;

use eframe::egui::{self, Color32, CornerRadius, Frame, Id, Key, Margin, Modifiers, RichText, Stroke, Vec2};
use portmanteau_core::{InlineMark, RichDocument, StyleSheet};

use crate::theme::*;
use crate::ui::editor::{begin_link_edit, link_support};

/// What the toolbar offers beyond the character marks and the link.
#[derive(Default)]
pub struct Options<'a> {
    /// When set, a paragraph-style dropdown listing these styles.
    pub styles: Option<&'a StyleSheet>,
}

#[derive(Default, Debug)]
pub struct Outcome {
    /// A button or shortcut changed the document.
    pub changed: bool,
}

/// Marks the next typed text should get (`on`) or lose (`off`), waiting at the
/// caret where the button was pressed.
#[derive(Clone, Debug, Default, PartialEq)]
struct Pending {
    caret: usize,
    on: Vec<InlineMark>,
    off: Vec<InlineMark>,
}

const MARKS: [(&str, InlineMark, &str); 4] = [
    ("B", InlineMark::Bold, "Bold (Ctrl/Cmd+B)"),
    ("I", InlineMark::Italic, "Italic (Ctrl/Cmd+I)"),
    ("U", InlineMark::Underline, "Underline (Ctrl/Cmd+U)"),
    ("S", InlineMark::Strikethrough, "Strikethrough"),
];

fn pending_key(field: Id) -> Id {
    Id::unique(("rich-toolbar-pending", field))
}

/// The field's selection, or an empty range at its caret.
fn selection(ctx: &egui::Context, field: Id) -> Range<usize> {
    egui::text_edit::TextEditState::load(ctx, field)
        .and_then(|state| state.cursor.char_range())
        .map_or(0..0, |range| {
            range.primary.index.0.min(range.secondary.index.0)..range.primary.index.0.max(range.secondary.index.0)
        })
}

/// The marks currently in effect where the writer is working, counting any
/// the writer has switched on or off for the next typing.
fn active_marks(content: &RichDocument, range: &Range<usize>, pending: Option<&Pending>) -> Vec<InlineMark> {
    let mut marks = content.marks_in(range.clone());
    if let Some(pending) = pending.filter(|p| range.is_empty() && p.caret == range.start) {
        marks.retain(|m| !pending.off.contains(m));
        for mark in &pending.on {
            if !marks.contains(mark) {
                marks.push(*mark);
            }
        }
    }
    marks
}

/// Toggle one mark: over the selection, or for the next typing at the caret.
fn toggle(ctx: &egui::Context, field: Id, content: &mut RichDocument, range: &Range<usize>, mark: InlineMark) -> bool {
    let key = pending_key(field);
    if !range.is_empty() {
        return content.toggle_mark(range.clone(), mark);
    }
    let mut pending: Pending = ctx.data(|d| d.get_temp(key)).filter(|p: &Pending| p.caret == range.start).unwrap_or(Pending {
        caret: range.start,
        ..Pending::default()
    });
    let now_on = active_marks(content, range, Some(&pending)).contains(&mark);
    pending.on.retain(|m| *m != mark);
    pending.off.retain(|m| *m != mark);
    if now_on {
        pending.off.push(mark);
    } else {
        pending.on.push(mark);
    }
    ctx.data_mut(|d| d.insert_temp(key, pending));
    false
}

/// Draw the toolbar. Call it before the field each frame.
pub fn show(ui: &mut egui::Ui, field: Id, content: &mut RichDocument, options: &Options) -> Outcome {
    let ctx = ui.ctx().clone();
    let mut outcome = Outcome::default();
    outcome.changed |= link_support(&ctx, field, content);

    let range = selection(&ctx, field);
    // Moving the caret away drops marks that were waiting for typing there.
    let key = pending_key(field);
    let pending: Option<Pending> = ctx.data(|d| d.get_temp(key)).filter(|p: &Pending| range.is_empty() && p.caret == range.start);
    if pending.is_none() {
        ctx.data_mut(|d| d.remove::<Pending>(key));
    }

    // Ctrl/Cmd+B, I, U while the field is being written in.
    if ctx.memory(|m| m.has_focus(field)) {
        for (key, mark) in [(Key::B, InlineMark::Bold), (Key::I, InlineMark::Italic), (Key::U, InlineMark::Underline)] {
            if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, key)) {
                outcome.changed |= toggle(&ctx, field, content, &range, mark);
            }
        }
    }
    let pending: Option<Pending> = ctx.data(|d| d.get_temp(key));
    let active = active_marks(content, &range, pending.as_ref());

    Frame::new()
        .fill(PANEL_2)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if let Some(sheet) = options.styles {
                    outcome.changed |= style_menu(ui, field, content, &range, sheet);
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                }
                for (label, mark, tip) in MARKS {
                    let on = active.contains(&mark);
                    let mut text = RichText::new(label).font(ui_strong()).color(if on { ACCENT_INK } else { TEXT });
                    text = match mark {
                        InlineMark::Bold => text.strong(),
                        InlineMark::Italic => text.italics(),
                        InlineMark::Underline => text.underline(),
                        InlineMark::Strikethrough => text.strikethrough(),
                    };
                    let button = egui::Button::new(text)
                        .fill(if on { ACCENT_SOFT } else { Color32::TRANSPARENT })
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(5))
                        .min_size(Vec2::new(28.0, 26.0));
                    if ui.add(button).on_hover_text(tip).clicked() {
                        outcome.changed |= toggle(&ctx, field, content, &range, mark);
                    }
                }
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                // A link needs text to link: a selection, or a link the caret is in.
                let existing = content.link_at(range.start, true);
                let can_link = !range.is_empty() || existing.is_some();
                let linked = existing.is_some() && range.is_empty() || content.link_at(range.start, false).is_some();
                let button = egui::Button::new(RichText::new("Link").font(ui_small()).color(if linked { ACCENT_INK } else { BLUE }))
                    .fill(if linked { ACCENT_SOFT } else { Color32::TRANSPARENT })
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(5))
                    .min_size(Vec2::new(40.0, 26.0));
                if ui
                    .add_enabled(can_link, button)
                    .on_hover_text("Make the selection a link, or edit the link at the caret (Ctrl/Cmd+K)")
                    .clicked()
                {
                    let target = if range.is_empty() { existing.map(|(_, whole)| whole) } else { Some(range.clone()) };
                    if let Some(target) = target {
                        begin_link_edit(&ctx, field, content, target);
                    }
                }
            });
        });
    outcome
}

fn style_menu(ui: &mut egui::Ui, field: Id, content: &mut RichDocument, range: &Range<usize>, sheet: &StyleSheet) -> bool {
    let current = content.style_at(range.start).unwrap_or(&sheet.default_style).to_string();
    let label = sheet
        .styles
        .iter()
        .find(|style| style.id == current)
        .map_or(current.clone(), |style| style.name.clone());
    let mut chosen = None;
    egui::ComboBox::from_id_salt(("rich-toolbar-style", field))
        .selected_text(RichText::new(label).font(ui_small()).color(MUTED))
        .width(120.0)
        .show_ui(ui, |ui| {
            for style in &sheet.styles {
                if ui.selectable_label(style.id == current, &style.name).clicked() {
                    chosen = Some(style.id.clone());
                }
            }
        });
    chosen.is_some_and(|id| content.set_paragraph_style(range.clone(), &id))
}

/// After the field's edited text has been synced back into `content`: give
/// the text just typed (or pasted) the marks the writer switched on or off at
/// the caret. `caret` is where the caret is now, after the typing.
pub fn after_edit(ctx: &egui::Context, field: Id, content: &mut RichDocument, caret: Option<usize>) -> bool {
    let key = pending_key(field);
    let Some(pending) = ctx.data(|d| d.get_temp::<Pending>(key)) else { return false };
    let typed: usize = ctx.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Text(text) | egui::Event::Paste(text) => Some(text.chars().count()),
                _ => None,
            })
            .sum()
    });
    let Some(caret) = caret.filter(|_| typed > 0) else { return false };
    ctx.data_mut(|d| d.remove::<Pending>(key));
    if caret < pending.caret || caret.saturating_sub(typed) != pending.caret {
        return false; // the typing was somewhere else
    }
    content.set_marks(pending.caret..caret, &pending.on, &pending.off)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_frame<T>(ctx: &egui::Context, input: egui::RawInput, f: impl FnOnce(&egui::Context) -> T) -> T {
        let mut out = None;
        let mut f = Some(f);
        ctx.run_ui(input, |ui| out = Some((f.take().unwrap())(ui.ctx()))).drop_without_applying_deltas();
        out.unwrap()
    }

    #[test]
    fn a_caret_toggle_waits_for_typing_then_marks_exactly_what_was_typed() {
        let ctx = egui::Context::default();
        let field = Id::unique("note-body");
        let mut content = RichDocument::from_plain_text("hello", "body");
        // Caret at the end; press Bold; type " world".
        let range = 5..5;
        run_frame(&ctx, egui::RawInput::default(), |ctx| assert!(!toggle(ctx, field, &mut content, &range, InlineMark::Bold)));
        content.replace_range(5..5, " world", &portmanteau_core::StyleSheet::novel_prose(&portmanteau_core::ElementStyles::default()));
        let input = egui::RawInput { events: vec![egui::Event::Text(" world".into())], ..Default::default() };
        let changed = run_frame(&ctx, input, |ctx| after_edit(ctx, field, &mut content, Some(11)));
        assert!(changed);
        assert_eq!(content.marks_in(0..5), []);
        assert_eq!(content.marks_in(5..11), [InlineMark::Bold]);
        // The pending marks were used up: more typing is plain again.
        let more = egui::RawInput { events: vec![egui::Event::Text("!".into())], ..Default::default() };
        assert!(!run_frame(&ctx, more, |ctx| after_edit(ctx, field, &mut content, Some(12))));
    }

    #[test]
    fn switching_a_mark_off_inside_formatted_text_makes_the_next_typing_plain() {
        let ctx = egui::Context::default();
        let field = Id::unique("note-body-2");
        let sheet = portmanteau_core::StyleSheet::novel_prose(&portmanteau_core::ElementStyles::default());
        let mut content = RichDocument::from_plain_text("bold", "body");
        content.set_marks(0..4, &[InlineMark::Bold], &[]);
        let range = 4..4;
        assert_eq!(active_marks(&content, &range, None), [InlineMark::Bold], "the caret inherits bold");
        run_frame(&ctx, egui::RawInput::default(), |ctx| {
            toggle(ctx, field, &mut content, &range, InlineMark::Bold);
        });
        let pending: Pending = ctx.data(|d| d.get_temp(pending_key(field))).unwrap();
        assert_eq!(active_marks(&content, &range, Some(&pending)), [], "the button shows Bold is off");
        content.replace_range(4..4, "x", &sheet);
        let input = egui::RawInput { events: vec![egui::Event::Text("x".into())], ..Default::default() };
        run_frame(&ctx, input, |ctx| after_edit(ctx, field, &mut content, Some(5)));
        assert_eq!(content.marks_in(0..4), [InlineMark::Bold]);
        assert_eq!(content.marks_in(4..5), []);
    }

    #[test]
    fn a_selection_toggle_applies_straight_away() {
        let ctx = egui::Context::default();
        let field = Id::unique("note-body-3");
        let mut content = RichDocument::from_plain_text("one two", "body");
        run_frame(&ctx, egui::RawInput::default(), |ctx| assert!(toggle(ctx, field, &mut content, &(0..3), InlineMark::Italic)));
        assert_eq!(content.marks_in(0..3), [InlineMark::Italic]);
        run_frame(&ctx, egui::RawInput::default(), |ctx| assert!(toggle(ctx, field, &mut content, &(0..3), InlineMark::Italic)));
        assert_eq!(content.marks_in(0..3), [], "the second press turns it off");
    }
}
