//! Text measurement helpers shared by the UI and exporters.

use serde::{Deserialize, Serialize};

/// Count words the way a writer expects: whitespace-separated tokens.
/// Hyphenated words count once; standalone punctuation does not count.
pub fn count_words(text: &str) -> u32 {
    text.split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_alphanumeric()))
        .count() as u32
}

pub fn reading_minutes(words: u32) -> u32 {
    ((words as f32) / 238.0).ceil() as u32
}

pub fn truncate_words(text: &str, limit: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= limit {
        text.to_string()
    } else {
        format!("{}…", words[..limit].join(" "))
    }
}

/// Rough character count excluding whitespace, used by the status bar.
pub fn visible_chars(text: &str) -> usize {
    text.chars().filter(|c| !c.is_whitespace()).count()
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Same,
    Add,
    Remove,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
}

/// A line-level comparison suitable for showing a writer what changed.
///
/// This is a longest-common-subsequence diff, not a character diff. It is kept
/// in the domain core so the desktop inspector, an agent and a future review
/// export all describe a revision the same way. Very large inputs fall back to
/// a plain remove/add listing rather than allocating a quadratic table.
pub fn diff_lines(old: &str, new: &str) -> Vec<DiffLine> {
    const MAX_CELLS: usize = 4_000_000;

    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let n = a.len();
    let m = b.len();

    let mut out = Vec::new();
    if n.saturating_mul(m) > MAX_CELLS {
        for (i, line) in a.iter().enumerate() {
            out.push(DiffLine {
                kind: DiffKind::Remove,
                text: (*line).to_string(),
                old_line: Some(i as u32 + 1),
                new_line: None,
            });
        }
        for (j, line) in b.iter().enumerate() {
            out.push(DiffLine {
                kind: DiffKind::Add,
                text: (*line).to_string(),
                old_line: None,
                new_line: Some(j as u32 + 1),
            });
        }
        return out;
    }

    let width = m + 1;
    let mut table = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i * width + j] = if a[i] == b[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }

    let (mut i, mut j) = (0usize, 0usize);
    let (mut old_no, mut new_no) = (1u32, 1u32);
    while i < n && j < m {
        if a[i] == b[j] {
            out.push(DiffLine {
                kind: DiffKind::Same,
                text: a[i].to_string(),
                old_line: Some(old_no),
                new_line: Some(new_no),
            });
            i += 1;
            j += 1;
            old_no += 1;
            new_no += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            out.push(DiffLine {
                kind: DiffKind::Remove,
                text: a[i].to_string(),
                old_line: Some(old_no),
                new_line: None,
            });
            i += 1;
            old_no += 1;
        } else {
            out.push(DiffLine {
                kind: DiffKind::Add,
                text: b[j].to_string(),
                old_line: None,
                new_line: Some(new_no),
            });
            j += 1;
            new_no += 1;
        }
    }
    while i < n {
        out.push(DiffLine {
            kind: DiffKind::Remove,
            text: a[i].to_string(),
            old_line: Some(old_no),
            new_line: None,
        });
        i += 1;
        old_no += 1;
    }
    while j < m {
        out.push(DiffLine {
            kind: DiffKind::Add,
            text: b[j].to_string(),
            old_line: None,
            new_line: Some(new_no),
        });
        j += 1;
        new_no += 1;
    }
    out
}

/// Counts of added and removed lines, used for the compact change summary.
pub fn diff_counts(lines: &[DiffLine]) -> (usize, usize) {
    let added = lines.iter().filter(|l| l.kind == DiffKind::Add).count();
    let removed = lines.iter().filter(|l| l.kind == DiffKind::Remove).count();
    (added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_ignore_punctuation() {
        assert_eq!(count_words("The sea — and the salt."), 5);
        assert_eq!(count_words(""), 0);
    }

    #[test]
    fn truncation_adds_ellipsis() {
        assert_eq!(truncate_words("one two three four", 2), "one two…");
    }

    #[test]
    fn diff_finds_changed_line() {
        let old = "The tide went out.\nMara waited.\nA boy arrived.";
        let new = "The tide went out.\nMara waited.\nA boy arrived.\nHe held a bucket.";
        let lines = diff_lines(old, new);
        let (added, removed) = diff_counts(&lines);
        assert_eq!((added, removed), (1, 0));
        assert_eq!(lines.last().unwrap().kind, DiffKind::Add);
        assert_eq!(lines.last().unwrap().new_line, Some(4));
    }

    #[test]
    fn diff_handles_replacement() {
        let lines = diff_lines("one\ntwo\nthree", "one\nTWO\nthree");
        assert_eq!(diff_counts(&lines), (1, 1));
    }

    #[test]
    fn diff_of_identical_text_is_all_same() {
        let lines = diff_lines("one\ntwo", "one\ntwo");
        assert!(lines.iter().all(|l| l.kind == DiffKind::Same));
    }
}
