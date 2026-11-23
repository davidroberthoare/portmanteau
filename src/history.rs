//! Undo and redo for rich text: documents, research notes and database notes.
//!
//! The editor changes `doc.content` (and the synopsis and notes) in about
//! thirty places: typing, paragraph styles, marks, images, spell fixes,
//! snapshot restores. Rather than ask each of them to record a step, this
//! module *watches*: once a frame in which input arrived, the app shows it the
//! selected document, and any difference from the last state it saw is an edit.
//! New ways to change text are covered without touching this file.
//!
//! A step stores only the paragraphs that changed (the window between the
//! common start and the common end of the old and new text), so memory follows
//! the size of the edit, not the size of the manuscript. Typing is merged: a
//! burst of edits becomes one step once the writer pauses.
//!
//! History is per document and lasts for the session. It complements named
//! snapshots and project checkpoints; it does not replace them.

use std::collections::HashMap;
use std::time::Duration;
use web_time::Instant;

use portmanteau_core::{DatabaseItem, Document, RichParagraph};

/// How long the writer must pause before a burst of typing closes as one step.
pub const PAUSE: Duration = Duration::from_millis(700);
/// Longest a single step may grow, so an hour of typing is not one undo.
const LONGEST_STEP: Duration = Duration::from_secs(4);
/// Steps kept per document.
const MAX_STEPS: usize = 200;

/// Which rich-text field of the thing being edited a step belongs to. A
/// document (research notes included) has three (`BODY`, `SYNOPSIS`,
/// `NOTES`); a database item has one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Field(pub usize);

impl Field {
    pub const BODY: Field = Field(0);
    #[cfg(test)]
    pub const SYNOPSIS: Field = Field(1);
    #[cfg(test)]
    pub const NOTES: Field = Field(2);
}

/// Anything with rich-text fields the history can watch. Adding another kind of
/// editable text means implementing this, nothing else.
pub trait Tracked {
    /// A key unique across kinds ("doc:…", "item:…").
    fn history_key(&self) -> String;
    fn field_count(&self) -> usize;
    fn field(&self, index: usize) -> &Vec<RichParagraph>;
    fn field_mut(&mut self, index: usize) -> &mut Vec<RichParagraph>;
}

impl Tracked for Document {
    fn history_key(&self) -> String {
        format!("doc:{}", self.id)
    }
    fn field_count(&self) -> usize {
        3
    }
    fn field(&self, index: usize) -> &Vec<RichParagraph> {
        match index {
            0 => &self.content.paragraphs,
            1 => &self.synopsis.paragraphs,
            _ => &self.notes.paragraphs,
        }
    }
    fn field_mut(&mut self, index: usize) -> &mut Vec<RichParagraph> {
        match index {
            0 => &mut self.content.paragraphs,
            1 => &mut self.synopsis.paragraphs,
            _ => &mut self.notes.paragraphs,
        }
    }
}

impl Tracked for DatabaseItem {
    fn history_key(&self) -> String {
        format!("item:{}", self.id)
    }
    fn field_count(&self) -> usize {
        1
    }
    fn field(&self, _index: usize) -> &Vec<RichParagraph> {
        &self.notes.paragraphs
    }
    fn field_mut(&mut self, _index: usize) -> &mut Vec<RichParagraph> {
        &mut self.notes.paragraphs
    }
}

/// One undoable step: at paragraph `start`, `before` became `after`.
#[derive(Clone, Debug)]
struct Step {
    field: Field,
    start: usize,
    before: Vec<RichParagraph>,
    after: Vec<RichParagraph>,
}

/// The changed window between two versions of a paragraph list, or `None`
/// when they are equal.
fn changed_window(old: &[RichParagraph], new: &[RichParagraph]) -> Option<(usize, Vec<RichParagraph>, Vec<RichParagraph>)> {
    if old == new {
        return None;
    }
    let start = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let room = old.len().min(new.len()) - start;
    let end = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    Some((
        start,
        old[start..old.len() - end].to_vec(),
        new[start..new.len() - end].to_vec(),
    ))
}

/// Edits since the writer last paused, not yet closed into a step.
struct Burst {
    /// The text as it was before the burst began.
    before: Vec<RichParagraph>,
    started: Instant,
    last_edit: Instant,
}

#[derive(Default)]
struct Tracker {
    /// The text as the history last saw it.
    seen: Option<Vec<RichParagraph>>,
    burst: Option<Burst>,
}

#[derive(Default)]
struct DocHistory {
    undo: Vec<Step>,
    redo: Vec<Step>,
    trackers: Vec<Tracker>,
}

impl DocHistory {
    /// Close one field's open burst into a step, comparing with what was last seen.
    fn close_burst(&mut self, field: Field) {
        let tracker = &mut self.trackers[field.0];
        let (Some(burst), Some(seen)) = (tracker.burst.take(), tracker.seen.as_ref()) else {
            return;
        };
        if let Some((start, before, after)) = changed_window(&burst.before, seen) {
            self.undo.push(Step { field, start, before, after });
        }
        if self.undo.len() > MAX_STEPS {
            let excess = self.undo.len() - MAX_STEPS;
            self.undo.drain(..excess);
        }
    }

    fn close_bursts(&mut self) {
        for index in 0..self.trackers.len() {
            self.close_burst(Field(index));
        }
    }
}

/// Where the caret should go after an undo or redo, as a character offset into
/// the field's flattened text (paragraphs joined by newlines).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applied {
    pub field: Field,
    pub caret: usize,
}

#[derive(Default)]
pub struct History {
    docs: HashMap<String, DocHistory>,
}

impl History {
    /// Show the history a document as it is now. Call once per frame in which
    /// input arrived (and while a burst is open). Returns how long until an
    /// open burst closes, so the caller can schedule a repaint.
    pub fn observe(&mut self, doc: &dyn Tracked, now: Instant) -> Option<Duration> {
        let history = self.docs.entry(doc.history_key()).or_default();
        history.trackers.resize_with(doc.field_count(), Tracker::default);
        let mut next_close: Option<Duration> = None;
        let mut edited = false;
        for index in 0..doc.field_count() {
            let field = Field(index);
            let current = doc.field(index);
            let tracker = &mut history.trackers[index];
            let changed = tracker.seen.as_ref().is_some_and(|seen| seen != current);

            // A pause (or a very long burst) closes the step before the next
            // edit starts a new one.
            let stale = tracker.burst.as_ref().is_some_and(|burst| {
                now.duration_since(burst.last_edit) >= PAUSE
                    || now.duration_since(burst.started) >= LONGEST_STEP
            });
            if stale {
                history.close_burst(field);
            }

            let tracker = &mut history.trackers[index];
            if changed {
                edited = true;
                match tracker.burst.as_mut() {
                    Some(burst) => burst.last_edit = now,
                    None => {
                        tracker.burst = Some(Burst {
                            before: tracker.seen.clone().unwrap_or_default(),
                            started: now,
                            last_edit: now,
                        });
                    }
                }
            }
            if changed || tracker.seen.is_none() {
                tracker.seen = Some(current.clone());
            }
            if let Some(burst) = &tracker.burst {
                let left = PAUSE.saturating_sub(now.duration_since(burst.last_edit));
                next_close = Some(next_close.map_or(left, |known| known.min(left)));
            }
        }
        if edited {
            // A new edit makes the undone future unreachable.
            history.redo.clear();
        }
        next_close
    }

    pub fn can_undo(&self, doc_id: &str) -> bool {
        self.docs.get(doc_id).is_some_and(|h| {
            !h.undo.is_empty()
                || h.trackers.iter().any(|t| {
                    t.burst.as_ref().zip(t.seen.as_ref()).is_some_and(|(b, seen)| b.before != *seen)
                })
        })
    }

    pub fn can_redo(&self, doc_id: &str) -> bool {
        self.docs.get(doc_id).is_some_and(|h| !h.redo.is_empty())
    }

    /// Undo the latest step. `None` when there is nothing to undo, or when the
    /// document no longer matches what the step expects (it was changed
    /// without being observed), in which case the history is discarded rather
    /// than applied to the wrong text.
    pub fn undo(&mut self, doc: &mut dyn Tracked) -> Option<Applied> {
        let key = doc.history_key();
        let history = self.docs.get_mut(&key)?;
        history.close_bursts();
        let step = history.undo.pop()?;
        let applied = apply(doc, &step, Direction::Undo);
        match applied {
            Some(applied) => {
                history.trackers[step.field.0].seen = Some(doc.field(step.field.0).clone());
                history.redo.push(step);
                Some(applied)
            }
            None => {
                self.docs.remove(&key);
                None
            }
        }
    }

    pub fn redo(&mut self, doc: &mut dyn Tracked) -> Option<Applied> {
        let key = doc.history_key();
        let history = self.docs.get_mut(&key)?;
        history.close_bursts();
        let step = history.redo.pop()?;
        let applied = apply(doc, &step, Direction::Redo);
        match applied {
            Some(applied) => {
                history.trackers[step.field.0].seen = Some(doc.field(step.field.0).clone());
                history.undo.push(step);
                Some(applied)
            }
            None => {
                self.docs.remove(&key);
                None
            }
        }
    }

    /// Forget documents that no longer exist.
    pub fn retain(&mut self, keep: impl Fn(&str) -> bool) {
        self.docs.retain(|id, _| keep(id));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Undo,
    Redo,
}

/// Swap the step's window in the document. The window currently in the
/// document must be the one the step says it replaced.
fn apply(doc: &mut dyn Tracked, step: &Step, direction: Direction) -> Option<Applied> {
    let (from, to) = match direction {
        Direction::Undo => (&step.after, &step.before),
        Direction::Redo => (&step.before, &step.after),
    };
    let paragraphs = doc.field_mut(step.field.0);
    let end = step.start.checked_add(from.len())?;
    if paragraphs.get(step.start..end)? != from.as_slice() {
        return None;
    }
    paragraphs.splice(step.start..end, to.iter().cloned());
    Some(Applied {
        field: step.field,
        caret: caret_after(&paragraphs[..], step.start, to, from),
    })
}

/// Character offset just after the part of `restored` that differs from
/// `replaced`, in the flattened text, so the caret lands where the change was.
fn caret_after(
    paragraphs: &[RichParagraph],
    start: usize,
    restored: &[RichParagraph],
    replaced: &[RichParagraph],
) -> usize {
    let offset: usize = paragraphs[..start]
        .iter()
        .map(|p| p.editor_text().chars().count() + 1)
        .sum();
    let flat = |window: &[RichParagraph]| {
        window.iter().map(|p| p.editor_text()).collect::<Vec<_>>().join("\n")
    };
    let (restored, replaced) = (flat(restored), flat(replaced));
    let (a, b): (Vec<char>, Vec<char>) = (restored.chars().collect(), replaced.chars().collect());
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let room = a.len().min(b.len()) - prefix;
    let suffix = a.iter().rev().zip(b.iter().rev()).take(room).take_while(|(x, y)| x == y).count();
    offset + a.len() - suffix
}

#[cfg(test)]
mod tests {
    use super::*;
    use portmanteau_core::{DocKind, InlineMark, Medium, TextRun};

    fn doc(texts: &[&str]) -> Document {
        let mut doc = Document::new(None, 0, "Chapter", DocKind::Chapter, Medium::Prose);
        doc.content.paragraphs = texts.iter().map(|t| para(t)).collect();
        doc
    }

    fn para(text: &str) -> RichParagraph {
        RichParagraph {
            style_id: "body".into(),
            content: vec![portmanteau_core::InlineContent::Text(TextRun::plain(text))],
        }
    }

    fn texts(doc: &Document) -> Vec<String> {
        doc.content.paragraphs.iter().map(|p| p.plain_text()).collect()
    }

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    fn edit(history: &mut History, doc: &mut Document, texts: &[&str], now: Instant) {
        doc.content.paragraphs = texts.iter().map(|t| para(t)).collect();
        history.observe(doc, now);
    }

    #[test]
    fn a_window_is_the_paragraphs_between_the_common_start_and_end() {
        let old = vec![para("a"), para("b"), para("c"), para("d")];
        let new = vec![para("a"), para("B"), para("X"), para("d")];
        let (start, before, after) = changed_window(&old, &new).unwrap();
        assert_eq!(start, 1);
        assert_eq!(before.len(), 2);
        assert_eq!(after[0].plain_text(), "B");
        assert!(changed_window(&old, &old).is_none());
    }

    #[test]
    fn insertions_and_deletions_of_whole_paragraphs_round_trip() {
        let old = vec![para("a"), para("b")];
        let grown = vec![para("a"), para("new"), para("b")];
        let (start, before, after) = changed_window(&old, &grown).unwrap();
        assert_eq!((start, before.len(), after.len()), (1, 0, 1));
        let shrunk = vec![para("a")];
        let (start, before, after) = changed_window(&old, &shrunk).unwrap();
        assert_eq!((start, before.len(), after.len()), (1, 1, 0));
    }

    #[test]
    fn typing_in_a_burst_is_one_step_and_undo_restores_the_start() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut d = doc(&["Once"]);
        history.observe(&d, t0);
        for (i, text) in ["Once u", "Once up", "Once upo", "Once upon"].iter().enumerate() {
            edit(&mut history, &mut d, &[text], at(t0, 100 * (i as u64 + 1)));
        }
        assert!(history.can_undo(&d.history_key()));
        let applied = history.undo(&mut d).expect("undo");
        assert_eq!(texts(&d), ["Once"], "one undo reverts the whole burst");
        assert_eq!(applied.caret, 4, "caret returns to the end of the surviving text");
        assert!(!history.can_undo(&d.history_key()));
        assert!(history.can_redo(&d.history_key()));
        history.redo(&mut d).expect("redo");
        assert_eq!(texts(&d), ["Once upon"]);
    }

    #[test]
    fn a_pause_separates_steps() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut d = doc(&["A"]);
        history.observe(&d, t0);
        edit(&mut history, &mut d, &["AB"], at(t0, 100));
        edit(&mut history, &mut d, &["ABC"], at(t0, 100 + 800)); // after a pause
        history.undo(&mut d).unwrap();
        assert_eq!(texts(&d), ["AB"], "only the later burst is undone");
        history.undo(&mut d).unwrap();
        assert_eq!(texts(&d), ["A"]);
        assert!(history.undo(&mut d).is_none(), "nothing left");
    }

    #[test]
    fn a_long_unbroken_burst_is_split_so_one_undo_is_never_an_hour() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut d = doc(&[""]);
        history.observe(&d, t0);
        let mut text = String::new();
        for i in 1..=100u64 {
            text.push('x');
            edit(&mut history, &mut d, &[text.as_str()], at(t0, i * 100)); // 10 s of typing, no pause
        }
        history.undo(&mut d).unwrap();
        let after_first = texts(&d)[0].len();
        assert!(after_first > 0 && after_first < 100, "undo stopped part-way: {after_first}");
    }

    #[test]
    fn a_new_edit_discards_the_redo_stack() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut d = doc(&["A"]);
        history.observe(&d, t0);
        edit(&mut history, &mut d, &["AB"], at(t0, 100));
        history.undo(&mut d).unwrap();
        assert!(history.can_redo(&d.history_key()));
        edit(&mut history, &mut d, &["AZ"], at(t0, 2000));
        assert!(!history.can_redo(&d.history_key()), "typing after an undo forks the history");
    }

    #[test]
    fn style_and_mark_changes_are_steps_like_typing() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut d = doc(&["Plain"]);
        history.observe(&d, t0);
        // Bold the word and restyle the paragraph.
        d.content.paragraphs[0].content = vec![portmanteau_core::InlineContent::Text(TextRun::with_marks(
            "Plain",
            vec![InlineMark::Bold],
        ))];
        d.content.paragraphs[0].style_id = "chapter-heading".into();
        history.observe(&d, at(t0, 50));
        history.undo(&mut d).unwrap();
        assert_eq!(d.content.paragraphs[0].style_id, "body");
        assert!(matches!(
            &d.content.paragraphs[0].content[0],
            portmanteau_core::InlineContent::Text(run) if run.marks.is_empty()
        ));
    }

    #[test]
    fn synopsis_and_notes_have_their_own_history_in_the_same_stack() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut d = doc(&["Body"]);
        let original_synopsis = d.synopsis.paragraphs.clone();
        history.observe(&d, t0);
        d.synopsis.paragraphs = vec![para("A synopsis")];
        history.observe(&d, at(t0, 50));
        edit(&mut history, &mut d, &["Body changed"], at(t0, 1500));
        history.undo(&mut d).unwrap();
        assert_eq!(texts(&d), ["Body"]);
        assert_eq!(d.synopsis.paragraphs.len(), 1, "the synopsis edit is still there");
        let applied = history.undo(&mut d).unwrap();
        assert_eq!(applied.field, Field::SYNOPSIS);
        assert_eq!(d.synopsis.paragraphs, original_synopsis);
    }

    #[test]
    fn history_is_per_document() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut a = doc(&["a"]);
        let mut b = doc(&["b"]);
        history.observe(&a, t0);
        history.observe(&b, t0);
        edit(&mut history, &mut a, &["a1"], at(t0, 10));
        assert!(history.can_undo(&a.history_key()));
        assert!(!history.can_undo(&b.history_key()), "an edit to A is not B's undo");
        assert!(history.undo(&mut b).is_none());
        assert_eq!(texts(&b), ["b"]);
    }

    #[test]
    fn a_step_is_not_applied_to_text_it_does_not_match() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut d = doc(&["A"]);
        history.observe(&d, t0);
        edit(&mut history, &mut d, &["AB"], at(t0, 10));
        history.undo(&mut d).unwrap();
        history.redo(&mut d).unwrap();
        // Someone changes the text behind the history's back, then asks to undo.
        d.content.paragraphs = vec![para("completely different")];
        assert!(history.undo(&mut d).is_none(), "refuses rather than corrupting");
        assert_eq!(texts(&d), ["completely different"]);
        assert!(!history.can_undo(&d.history_key()), "and the stale history is dropped");
    }

    #[test]
    fn steps_are_capped() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut d = doc(&["0"]);
        history.observe(&d, t0);
        for i in 1..=(MAX_STEPS as u64 + 50) {
            edit(&mut history, &mut d, &[format!("{i}").as_str()], at(t0, i * 1000));
        }
        let mut undone = 0;
        while history.undo(&mut d).is_some() {
            undone += 1;
        }
        assert!(undone <= MAX_STEPS + 1, "{undone}");
    }

    #[test]
    fn the_burst_timer_is_reported_for_repaint_scheduling() {
        let t0 = Instant::now();
        let mut history = History::default();
        let mut d = doc(&["A"]);
        history.observe(&d, t0);
        edit(&mut history, &mut d, &["AB"], at(t0, 100));
        let left = history.observe(&d, at(t0, 300)).expect("a burst is open");
        assert!(left <= PAUSE - Duration::from_millis(200) && left > Duration::ZERO);
    }
}
