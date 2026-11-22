//! `@` mentions: type `@` in a rich-text field, keep typing a name, and pick
//! what it refers to from a small list that appears under the caret.
//!
//! The choice becomes a *link* to that thing (`item:<id>` for a database item,
//! `note:<id>` for a research note or a manuscript document), so it can be
//! followed with Ctrl/Cmd+click (see `App::open_link`) and is counted as an
//! appearance on the database page. The list is filtered as you type, by title
//! first and tags second; Up/Down move, Enter or Tab choose, Escape closes it.
//!
//! # Adding it to a rich-text field
//!
//! The field must be an egui `TextEdit` editing a `RichDocument` (as the
//! research body and the database notes do). Around its `show`:
//!
//! ```ignore
//! let pick = mention::pre_edit(ctx, field_id);            // BEFORE the TextEdit: takes Up/Down/Enter/Tab/Esc
//! let output = TextEdit::multiline(&mut text).id(field_id)/* … */.show(ui);
//! /* … sync the edited text back into `content` … */
//! if mention::post_edit(ctx, field_id, &output, &mut content, pick) {
//!     changed = true; // a mention was inserted
//! }
//! ```
//!
//! `pre_edit` must come first so the keys it takes never reach the `TextEdit`
//! (Enter would otherwise start a new line). The targets come from
//! `publish_targets`, which the app calls once a frame.

use std::sync::Arc;

use eframe::egui::{self, Color32, CornerRadius, Frame, Id, Key, Margin, Modifiers, Sense, Stroke, Vec2};
use portmanteau_core::{ElementStyles, Project, RichDocument, StyleSheet};

use crate::theme::*;

/// Something an `@` can point at.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    /// The link it becomes: `item:<id>` or `note:<id>`.
    pub link: String,
    pub title: String,
    /// What kind of thing it is, shown beside the title ("Character", "Research", "Chapter").
    pub group: String,
    pub tags: Vec<String>,
}

const MAX_SHOWN: usize = 8;
const MAX_QUERY: usize = 32;

fn targets_key() -> Id {
    Id::unique("mention-targets")
}

/// Make the project's mentionable things available to every field. Database
/// items come first, then research notes, then manuscript documents.
pub fn publish_targets(ctx: &egui::Context, project: &Project) {
    let mut targets: Vec<Target> = project
        .items
        .iter()
        .filter(|item| !item.title.trim().is_empty())
        .map(|item| Target {
            link: format!("{}{}", ITEM_LINK, item.id),
            title: item.title.trim().to_string(),
            group: item.kind.label().to_string(),
            tags: item.tags.clone(),
        })
        .collect();
    targets.extend(project.research_docs().into_iter().filter(|d| !d.title.trim().is_empty()).map(|d| Target {
        link: format!("{}{}", portmanteau_core::rich_text::NOTE_LINK, d.id),
        title: d.title.trim().to_string(),
        group: "Research".into(),
        tags: Vec::new(),
    }));
    targets.extend(
        project
            .ordered_documents()
            .into_iter()
            .filter(|d| !d.kind.is_folder() && !project.in_research(d) && !d.title.trim().is_empty())
            .map(|d| Target {
                link: format!("{}{}", portmanteau_core::rich_text::NOTE_LINK, d.id),
                title: d.title.trim().to_string(),
                group: d.kind.label().to_string(),
                tags: Vec::new(),
            }),
    );
    ctx.data_mut(|data| data.insert_temp(targets_key(), Arc::new(targets)));
}

/// The targets published this frame (empty before the first publish).
pub fn published_targets(ctx: &egui::Context) -> Arc<Vec<Target>> {
    ctx.data(|d| d.get_temp(targets_key())).unwrap_or_default()
}

/// The prefix of a link to a database item.
pub const ITEM_LINK: &str = "item:";

/// Targets matching what has been typed after the `@`: titles that start with
/// it first, then titles that contain it, then tags that match; at most `limit`.
pub fn matches(targets: &[Target], query: &str, limit: usize) -> Vec<Target> {
    let needle = query.trim().to_lowercase();
    let mut starts = Vec::new();
    let mut contains = Vec::new();
    let mut tagged = Vec::new();
    for target in targets {
        let title = target.title.to_lowercase();
        if needle.is_empty() || title.starts_with(&needle) {
            starts.push(target.clone());
        } else if title.contains(&needle) {
            contains.push(target.clone());
        } else if target.tags.iter().any(|tag| tag.to_lowercase().contains(&needle)) {
            tagged.push(target.clone());
        }
    }
    starts.into_iter().chain(contains).chain(tagged).take(limit).collect()
}

/// Where the `@` being completed is: its character position in the whole
/// document, the caret, and what was typed after it. `None` when the caret is
/// not right after an `@word` (an `@` that starts a word, with no space since).
pub fn current_mention(content: &RichDocument, caret: usize) -> Option<(usize, String)> {
    let position = content.position_at(caret);
    let paragraph = content.paragraphs.get(position.paragraph)?;
    let chars: Vec<char> = paragraph.editor_text().chars().collect();
    let end = position.offset.min(chars.len());
    let mut at = end;
    while at > 0 {
        let c = chars[at - 1];
        if c == '@' {
            let starts_word = at == 1 || chars[at - 2].is_whitespace() || "([{\"'“‘—-".contains(chars[at - 2]);
            if !starts_word {
                return None;
            }
            let query: String = chars[at..end].iter().collect();
            if query.chars().count() > MAX_QUERY {
                return None;
            }
            let start = caret - (end - (at - 1));
            return Some((start, query));
        }
        if c.is_whitespace() {
            return None;
        }
        at -= 1;
    }
    None
}

#[derive(Clone, Default)]
struct State {
    visible: bool,
    selected: usize,
    /// Closed with Escape at this `@`; stays closed until it is typed afresh.
    dismissed_at: Option<usize>,
    shown: Vec<Target>,
}

fn state_key(field: Id) -> Id {
    Id::unique(("mention-state", field))
}

/// What the writer chose with the keyboard this frame.
#[derive(Debug, PartialEq)]
pub enum Pick {
    Accept(usize),
}

/// Call before the field's `TextEdit` is shown: takes Up, Down, Enter, Tab and
/// Escape while the list is open, so the field never sees them.
pub fn pre_edit(ctx: &egui::Context, field: Id) -> Option<Pick> {
    let key = state_key(field);
    let mut state: State = ctx.data(|d| d.get_temp(key))?;
    if !state.visible || state.shown.is_empty() || !ctx.memory(|m| m.has_focus(field)) {
        return None;
    }
    let count = state.shown.len();
    let mut pick = None;
    ctx.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
            state.selected = (state.selected + 1) % count;
        }
        if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
            state.selected = (state.selected + count - 1) % count;
        }
        if input.consume_key(Modifiers::NONE, Key::Enter) || input.consume_key(Modifiers::NONE, Key::Tab) {
            pick = Some(Pick::Accept(state.selected));
        }
        if input.consume_key(Modifiers::NONE, Key::Escape) {
            state.visible = false;
            state.dismissed_at = Some(usize::MAX); // set to the real `@` in post_edit
        }
    });
    ctx.data_mut(|d| d.insert_temp(key, state));
    pick
}

/// Call after the field's edited text has been synced back into `content`.
/// Shows the list when the caret is in an `@word`, and inserts the chosen
/// mention. Returns whether `content` changed (a mention was inserted).
pub fn post_edit(
    ctx: &egui::Context,
    field: Id,
    output: &egui::text_edit::TextEditOutput,
    content: &mut RichDocument,
    pick: Option<Pick>,
) -> bool {
    let key = state_key(field);
    let mut state: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    let caret = output.cursor_range.map(|r| r.primary.index.0);
    let focused = ctx.memory(|m| m.has_focus(field));
    let current = caret.filter(|_| focused).and_then(|c| current_mention(content, c).map(|(at, q)| (c, at, q)));

    // Escape pressed in pre_edit: remember which `@` it closed.
    if state.dismissed_at == Some(usize::MAX) {
        state.dismissed_at = current.as_ref().map(|(_, at, _)| *at);
    }
    let targets: Arc<Vec<Target>> = ctx.data(|d| d.get_temp(targets_key())).unwrap_or_default();

    let Some((caret, at, query)) = current else {
        state.visible = false;
        state.dismissed_at = None;
        ctx.data_mut(|d| d.insert_temp(key, state));
        return false;
    };
    let shown = matches(&targets, &query, MAX_SHOWN);
    state.visible = !shown.is_empty() && state.dismissed_at != Some(at);
    if state.shown != shown {
        state.selected = 0;
    }
    state.selected = state.selected.min(shown.len().saturating_sub(1));
    state.shown = shown;

    let mut accept = match pick {
        Some(Pick::Accept(i)) if state.visible => Some(i),
        _ => None,
    };

    if state.visible {
        let row = output.galley.pos_from_cursor(egui::text::CCursor::new(caret)).translate(output.galley_pos.to_vec2());
        let clicked = list_ui(ctx, field, row.left_bottom() + Vec2::new(0.0, 4.0), &state);
        accept = accept.or(clicked);
    }
    let changed = match accept.and_then(|i| state.shown.get(i).cloned()) {
        Some(target) => {
            insert(ctx, field, content, at, caret, &target);
            state.visible = false;
            true
        }
        None => false,
    };
    ctx.data_mut(|d| d.insert_temp(key, state));
    changed
}

/// Replace `@query` (characters `at..caret`) with the target's title, linked.
fn insert(ctx: &egui::Context, field: Id, content: &mut RichDocument, at: usize, caret: usize, target: &Target) {
    let sheet = StyleSheet::novel_prose(&ElementStyles::default());
    content.replace_range(at..caret, &target.title, &sheet);
    let length = target.title.chars().count();
    content.set_link(at..at + length, Some(&target.link));
    let mut state = egui::text_edit::TextEditState::load(ctx, field).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(at + length))));
    state.store(ctx, field);
    ctx.request_repaint();
}

/// The list under the caret. Returns the row clicked, if any.
fn list_ui(ctx: &egui::Context, field: Id, pos: egui::Pos2, state: &State) -> Option<usize> {
    let mut clicked = None;
    egui::Area::new(Id::unique(("mention-list", field)))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(CornerRadius::same(8))
                .shadow(egui::epaint::Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha(36) })
                .inner_margin(Margin::symmetric(4, 4))
                .show(ui, |ui| {
                    ui.set_min_width(260.0);
                    for (index, target) in state.shown.iter().enumerate() {
                        let selected = index == state.selected;
                        let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::click());
                        if selected {
                            ui.painter().rect_filled(rect, CornerRadius::same(5), ACCENT_SOFT);
                        } else if response.hovered() {
                            ui.painter().rect_filled(rect, CornerRadius::same(5), PANEL_2);
                        }
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(egui::Role::Button, true, selected, format!("{} · {}", target.title, target.group))
                        });
                        ui.painter().text(
                            rect.left_center() + Vec2::new(8.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            &target.title,
                            ui_body(),
                            TEXT,
                        );
                        ui.painter().text(
                            rect.right_center() - Vec2::new(8.0, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            &target.group,
                            ui_small(),
                            FAINT,
                        );
                        if response.clicked() {
                            clicked = Some(index);
                        }
                    }
                });
        });
    // A click on the list must not take focus from the field.
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(link: &str, title: &str, group: &str, tags: &[&str]) -> Target {
        Target { link: link.into(), title: title.into(), group: group.into(), tags: tags.iter().map(|t| t.to_string()).collect() }
    }

    fn sample() -> Vec<Target> {
        vec![
            target("item:1", "Mara Venn", "Character", &["family"]),
            target("item:2", "Elias Rook", "Character", &["harbour"]),
            target("item:3", "Cold Harbor", "Place", &[]),
            target("note:4", "Harbour tide tables", "Research", &[]),
        ]
    }

    #[test]
    fn matches_rank_titles_that_start_with_the_query_then_contain_it_then_tags() {
        let t = sample();
        let titles = |q: &str| matches(&t, q, 8).into_iter().map(|m| m.title).collect::<Vec<_>>();
        assert_eq!(titles("").len(), 4, "a bare @ offers everything, database first");
        assert_eq!(titles("")[0], "Mara Venn");
        assert_eq!(titles("ma"), ["Mara Venn"]);
        // "harbo": titles starting with it, then containing it, then tagged with it.
        assert_eq!(titles("harbo"), ["Harbour tide tables", "Cold Harbor", "Elias Rook"]);
        assert_eq!(titles("FAM"), ["Mara Venn"], "tags match, case does not matter");
        assert!(titles("zzz").is_empty());
        assert_eq!(matches(&t, "", 2).len(), 2, "capped");
    }

    #[test]
    fn an_at_sign_opens_a_mention_only_at_the_start_of_a_word() {
        let doc = |s: &str| RichDocument::from_plain_text(s, "body");
        // "Ask @Ma|" with the caret at the end.
        let d = doc("Ask @Ma");
        assert_eq!(current_mention(&d, 7), Some((4, "Ma".to_string())));
        assert_eq!(current_mention(&doc("@"), 1), Some((0, String::new())));
        assert_eq!(current_mention(&doc("(@Mara"), 6), Some((1, "Mara".to_string())));
        // An email address is not a mention, and a space ends it.
        assert_eq!(current_mention(&doc("write ada@exa"), 13), None);
        assert_eq!(current_mention(&doc("@Mara said"), 10), None);
        assert_eq!(current_mention(&doc("no sign here"), 5), None);
        // In the second paragraph, positions are document-wide.
        let d = doc("first\n@El");
        assert_eq!(current_mention(&d, 9), Some((6, "El".to_string())));
    }

    #[test]
    fn choosing_a_mention_replaces_the_typed_text_with_a_link_to_the_thing() {
        let ctx = egui::Context::default();
        let field = Id::unique("a-field");
        let mut doc = RichDocument::from_plain_text("Ask @Ma now", "body");
        let (at, _) = current_mention(&doc, 7).unwrap();
        insert(&ctx, field, &mut doc, at, 7, &target("item:1", "Mara Venn", "Character", &[]));
        assert_eq!(doc.plain_text(), "Ask Mara Venn now");
        assert_eq!(doc.links(), [(4..13, "item:1".to_string())]);
        let state = egui::text_edit::TextEditState::load(&ctx, field).expect("caret placed");
        assert_eq!(state.cursor.char_range().unwrap().primary.index.0, 13, "the caret lands after the name");
    }

    #[test]
    fn targets_cover_items_research_and_documents_in_that_order() {
        let ctx = egui::Context::default();
        let project = portmanteau_core::sample::demo();
        publish_targets(&ctx, &project);
        let targets: Arc<Vec<Target>> = ctx.data(|d| d.get_temp(targets_key())).unwrap();
        let groups: Vec<&str> = targets.iter().map(|t| t.group.as_str()).collect();
        let first_research = groups.iter().position(|g| *g == "Research").unwrap();
        let first_doc = groups.iter().position(|g| *g == "Chapter").unwrap();
        assert!(groups[0] == "Character" && first_research < first_doc);
        assert!(targets.iter().any(|t| t.link.starts_with("item:")));
        assert!(targets.iter().any(|t| t.link.starts_with("note:")));
    }
}
