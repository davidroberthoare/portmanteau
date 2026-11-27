//! Writing activity: how much a writer typed and read, and how many words they
//! added or changed, kept in small buckets of one hour per document.
//!
//! Nothing here watches the keyboard (the app does); this module only stores
//! what it is told and answers questions about it: totals for a day, a week,
//! a document, the best time of day, streaks. It also sorts an edit into
//! "added", "edited" and "deleted" words.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::rich_text::RichParagraph;

/// One hour of work on one document. `hour` is local time, `YYYY-MM-DDTHH`.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct ActivityRecord {
    pub hour: String,
    /// A document id, or empty for work not tied to one.
    pub doc: String,
    /// Seconds spent actively typing (a pause of a few seconds is still typing).
    pub typing_secs: u32,
    /// Seconds spent reading: the editor in front, scrolling or moving about,
    /// but not typing.
    pub reading_secs: u32,
    /// Words written at the end of a sentence or paragraph.
    pub added: u32,
    /// Words typed into the middle of existing text.
    pub edited: u32,
    /// Words taken out.
    pub deleted: u32,
}

/// The same measures, for a sum or one change.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ActivityTotals {
    pub typing_secs: u32,
    pub reading_secs: u32,
    pub added: u32,
    pub edited: u32,
    pub deleted: u32,
}

impl ActivityTotals {
    pub fn is_empty(&self) -> bool {
        *self == ActivityTotals::default()
    }

    pub fn add(&mut self, other: &ActivityTotals) {
        self.typing_secs = self.typing_secs.saturating_add(other.typing_secs);
        self.reading_secs = self.reading_secs.saturating_add(other.reading_secs);
        self.added = self.added.saturating_add(other.added);
        self.edited = self.edited.saturating_add(other.edited);
        self.deleted = self.deleted.saturating_add(other.deleted);
    }

    /// Added words per hour of typing, or `None` before there is any typing.
    pub fn words_per_typing_hour(&self) -> Option<u32> {
        (self.typing_secs >= 60).then(|| (self.added as f32 * 3600.0 / self.typing_secs as f32).round() as u32)
    }
}

impl From<&ActivityRecord> for ActivityTotals {
    fn from(r: &ActivityRecord) -> Self {
        ActivityTotals {
            typing_secs: r.typing_secs,
            reading_secs: r.reading_secs,
            added: r.added,
            edited: r.edited,
            deleted: r.deleted,
        }
    }
}

/// Add `delta` to the bucket for `hour` and `doc`, making it if needed.
pub fn record(log: &mut Vec<ActivityRecord>, hour: &str, doc: &str, delta: ActivityTotals) {
    if delta.is_empty() {
        return;
    }
    let index = match log.iter().rposition(|r| r.hour == hour && r.doc == doc) {
        Some(index) => index,
        None => {
            log.push(ActivityRecord { hour: hour.into(), doc: doc.into(), ..Default::default() });
            log.len() - 1
        }
    };
    let mut total = ActivityTotals::from(&log[index]);
    total.add(&delta);
    let r = &mut log[index];
    r.typing_secs = total.typing_secs;
    r.reading_secs = total.reading_secs;
    r.added = total.added;
    r.edited = total.edited;
    r.deleted = total.deleted;
}

/// The sum of every record `keep` accepts.
pub fn total(log: &[ActivityRecord], keep: impl Fn(&ActivityRecord) -> bool) -> ActivityTotals {
    let mut sum = ActivityTotals::default();
    for r in log.iter().filter(|r| keep(r)) {
        sum.add(&ActivityTotals::from(r));
    }
    sum
}

/// Totals per day (`YYYY-MM-DD`), oldest first.
pub fn per_day(log: &[ActivityRecord]) -> BTreeMap<String, ActivityTotals> {
    group(log, |r| r.hour.get(..10).unwrap_or(&r.hour).to_owned())
}

/// Totals per ISO week (`YYYY-Www`), oldest first.
pub fn per_week(log: &[ActivityRecord]) -> BTreeMap<String, ActivityTotals> {
    group(log, |r| {
        r.hour
            .get(..10)
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .map(|d| d.format("%G-W%V").to_string())
            .unwrap_or_default()
    })
}

/// Totals per document id.
pub fn per_document(log: &[ActivityRecord]) -> BTreeMap<String, ActivityTotals> {
    group(log, |r| r.doc.clone())
}

/// Added words for each hour of the day (0 to 23), over all time: when the
/// writer does their best work.
pub fn per_hour_of_day(log: &[ActivityRecord]) -> [ActivityTotals; 24] {
    let mut out = [ActivityTotals::default(); 24];
    for r in log {
        if let Some(h) = r.hour.get(11..13).and_then(|h| h.parse::<usize>().ok()) {
            if h < 24 {
                out[h].add(&ActivityTotals::from(r));
            }
        }
    }
    out
}

fn group(log: &[ActivityRecord], key: impl Fn(&ActivityRecord) -> String) -> BTreeMap<String, ActivityTotals> {
    let mut out: BTreeMap<String, ActivityTotals> = BTreeMap::new();
    for r in log {
        out.entry(key(r)).or_default().add(&ActivityTotals::from(r));
    }
    out
}

/// Days in a row, ending today or yesterday, on which at least `goal` words
/// were added. A goal of 0 counts any day with words added.
pub fn streak(days: &BTreeMap<String, ActivityTotals>, today: &str, goal: u32) -> u32 {
    let Ok(mut day) = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d") else {
        return 0;
    };
    let met = |d: &chrono::NaiveDate| {
        days.get(&d.format("%Y-%m-%d").to_string())
            .is_some_and(|t| t.added >= goal.max(1))
    };
    // Today may simply not have started yet.
    if !met(&day) {
        day = day.pred_opt().unwrap_or(day);
    }
    let mut count = 0;
    while met(&day) {
        count += 1;
        match day.pred_opt() {
            Some(previous) => day = previous,
            None => break,
        }
    }
    count
}

// ---------------------------------------------------------------------------
// Sorting an edit into added, edited and deleted words
// ---------------------------------------------------------------------------

fn tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}

fn window_text(paragraphs: &[RichParagraph]) -> String {
    paragraphs.iter().map(|p| p.plain_text()).collect::<Vec<_>>().join("\n")
}

/// How the words of one edit count. `before` and `after` are the changed
/// paragraphs (see `history`): the text between the first and last difference.
///
/// Words typed at the end of a paragraph, or straight after a full stop,
/// question mark, exclamation mark or closing quote, are *added*: the writer is
/// moving forward. Words typed anywhere else are *edited*: the writer is
/// reworking what is already there. Words that disappeared are *deleted*.
pub fn classify_edit(before: &[RichParagraph], after: &[RichParagraph]) -> ActivityTotals {
    let old = window_text(before);
    let new = window_text(after);
    // Where the text changed, found on whole words rather than letters.
    let old_raw: Vec<&str> = old.split_whitespace().collect();
    let new_raw: Vec<&str> = new.split_whitespace().collect();
    let prefix = old_raw.iter().zip(&new_raw).take_while(|(a, b)| a == b).count();
    let room = old_raw.len().min(new_raw.len()) - prefix;
    let suffix = old_raw
        .iter()
        .rev()
        .zip(new_raw.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    let head_last = prefix.checked_sub(1).map(|i| old_raw[i]);
    let inserted_last = (new_raw.len() - suffix).checked_sub(1).filter(|&i| i >= prefix).map(|i| new_raw[i]);
    let tail_empty = suffix == 0;

    // Words that are new and words that are gone, comparing the whole windows
    // word by word. (Counting the inserted letters would score a word typed one
    // letter at a time many times over.)
    let mut remaining = tokens(&old);
    let mut gained = 0u32;
    for w in tokens(&new) {
        match remaining.iter().position(|x| *x == w) {
            Some(i) => {
                remaining.swap_remove(i);
            }
            None => gained += 1,
        }
    }
    let lost = remaining.len() as u32;

    let ends = |word: &str| word.trim_end_matches(['"', '”', '’', '\'', ')']).ends_with(['.', '!', '?', '…']);
    // Forward: at the end of the text, or a whole sentence set down after one.
    let starts_sentence = head_last.is_none_or(ends);
    let at_end = tail_empty || (starts_sentence && inserted_last.is_some_and(ends));
    let mut out = ActivityTotals { deleted: lost, ..Default::default() };
    if at_end {
        out.added = gained;
    } else {
        out.edited = gained;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str) -> Vec<RichParagraph> {
        let mut para = RichParagraph::empty("body");
        para.set_plain_text(text);
        vec![para]
    }

    #[test]
    fn writing_forward_is_added() {
        let t = classify_edit(&p("The cat sat."), &p("The cat sat. Then it left the house."));
        assert_eq!((t.added, t.edited, t.deleted), (5, 0, 0));
        let t = classify_edit(&p(""), &p("A brand new paragraph."));
        assert_eq!(t.added, 4);
    }

    #[test]
    fn typing_inside_a_sentence_is_edited() {
        let t = classify_edit(&p("The cat sat on the mat."), &p("The big black cat sat on the mat."));
        assert_eq!((t.added, t.edited, t.deleted), (0, 2, 0));
    }

    #[test]
    fn a_new_sentence_in_the_middle_of_a_paragraph_is_added() {
        let t = classify_edit(&p("One. Three."), &p("One. Two is here. Three."));
        assert_eq!(t.added, 3);
        assert_eq!(t.edited, 0);
    }

    #[test]
    fn replacing_a_word_counts_one_edited_and_one_deleted() {
        let t = classify_edit(&p("The cat sat down."), &p("The dog sat down."));
        assert_eq!((t.added, t.edited, t.deleted), (0, 1, 1));
    }

    #[test]
    fn deleting_words_is_deleted() {
        let t = classify_edit(&p("One two three four."), &p("One two."));
        assert_eq!((t.added, t.edited, t.deleted), (0, 0, 2));
    }

    #[test]
    fn records_add_into_one_bucket_and_group() {
        let mut log = Vec::new();
        record(&mut log, "2026-11-26T09", "a", ActivityTotals { typing_secs: 60, added: 100, ..Default::default() });
        record(&mut log, "2026-11-26T09", "a", ActivityTotals { reading_secs: 30, added: 20, ..Default::default() });
        record(&mut log, "2026-11-26T14", "b", ActivityTotals { added: 50, edited: 5, ..Default::default() });
        record(&mut log, "2026-11-27T09", "a", ActivityTotals { added: 10, ..Default::default() });
        assert_eq!(log.len(), 3);
        let days = per_day(&log);
        assert_eq!(days["2026-11-26"].added, 170);
        assert_eq!(days["2026-11-26"].reading_secs, 30);
        assert_eq!(per_document(&log)["a"].added, 130);
        assert_eq!(per_hour_of_day(&log)[9].added, 130);
        assert_eq!(per_week(&log).len(), 1, "both days fall in one ISO week");
        assert_eq!(days["2026-11-26"].words_per_typing_hour(), Some(10200), "120 words in 60 s of typing");
    }

    #[test]
    fn a_streak_counts_back_from_today_or_yesterday() {
        let mut log = Vec::new();
        for day in ["2026-11-23", "2026-11-24", "2026-11-25"] {
            record(&mut log, &format!("{day}T10"), "a", ActivityTotals { added: 300, ..Default::default() });
        }
        let days = per_day(&log);
        assert_eq!(streak(&days, "2026-11-25", 200), 3);
        assert_eq!(streak(&days, "2026-11-26", 200), 3, "today has not started yet");
        assert_eq!(streak(&days, "2026-11-25", 500), 0);
        assert_eq!(streak(&days, "2026-11-27", 200), 0, "yesterday was missed too");
    }
}
