//! Fountain inline emphasis: `*italic*`, `**bold**`, `***bold italic***` and
//! `_underline_`, with `\` escaping the next character.
//!
//! A marker only counts when it has a partner, so a lone `*` or a
//! `snake_case_name` stays literal text.

use portmanteau_core::{InlineContent, InlineMark, TextRun};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Italic,
    Bold,
    BoldItalic,
    Underline,
}

impl Kind {
    fn marks(self) -> &'static [InlineMark] {
        match self {
            Kind::Italic => &[InlineMark::Italic],
            Kind::Bold => &[InlineMark::Bold],
            Kind::BoldItalic => &[InlineMark::Bold, InlineMark::Italic],
            Kind::Underline => &[InlineMark::Underline],
        }
    }
}

enum Token {
    Text(String),
    Marker { kind: Kind, literal: &'static str },
}

fn tokenize(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            buffer.push(chars[i + 1]);
            i += 2;
            continue;
        }
        let run = if c == '*' {
            chars[i..].iter().take_while(|&&x| x == '*').count().min(3)
        } else {
            0
        };
        let marker = match (c, run) {
            ('*', 3) => Some((Kind::BoldItalic, "***", 3)),
            ('*', 2) => Some((Kind::Bold, "**", 2)),
            ('*', 1) => Some((Kind::Italic, "*", 1)),
            ('_', _) => Some((Kind::Underline, "_", 1)),
            _ => None,
        };
        if let Some((kind, literal, width)) = marker {
            if !buffer.is_empty() {
                tokens.push(Token::Text(std::mem::take(&mut buffer)));
            }
            tokens.push(Token::Marker { kind, literal });
            i += width;
        } else {
            buffer.push(c);
            i += 1;
        }
    }
    if !buffer.is_empty() {
        tokens.push(Token::Text(buffer));
    }
    tokens
}

/// Split `text` into runs carrying their emphasis marks.
pub(crate) fn runs(text: &str) -> Vec<TextRun> {
    let tokens = tokenize(text);
    // Pair markers: a marker closes the nearest open marker of its kind.
    let mut paired = vec![false; tokens.len()];
    let mut open: Vec<(Kind, usize)> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let Token::Marker { kind, .. } = token else { continue };
        let before_space = index
            .checked_sub(1)
            .and_then(|p| match &tokens[p] {
                Token::Text(t) => t.chars().last().map(char::is_whitespace),
                _ => Some(false),
            })
            .unwrap_or(true);
        let after_space = match tokens.get(index + 1) {
            Some(Token::Text(t)) => t.chars().next().map(char::is_whitespace).unwrap_or(true),
            Some(_) => false,
            None => true,
        };
        // An underscore inside a word is part of the word (snake_case).
        let word_before = index
            .checked_sub(1)
            .and_then(|p| match &tokens[p] {
                Token::Text(t) => t.chars().last().map(char::is_alphanumeric),
                _ => None,
            })
            .unwrap_or(false);
        let word_after = match tokens.get(index + 1) {
            Some(Token::Text(t)) => t.chars().next().is_some_and(char::is_alphanumeric),
            _ => false,
        };
        let underscore = *kind == Kind::Underline;
        if let Some(position) = open.iter().rposition(|(k, _)| k == kind) {
            // Closing: must hug the text it closes.
            if !before_space && !(underscore && word_after) {
                let (_, start) = open.remove(position);
                paired[start] = true;
                paired[index] = true;
                continue;
            }
        }
        // Opening: must hug the text it opens.
        if !after_space && !(underscore && word_before) {
            open.push((*kind, index));
        }
    }

    let mut out: Vec<TextRun> = Vec::new();
    let mut active: Vec<Kind> = Vec::new();
    let mut push = |text: &str, active: &[Kind]| {
        if text.is_empty() {
            return;
        }
        let mut marks: Vec<InlineMark> = Vec::new();
        for kind in active {
            for mark in kind.marks() {
                if !marks.contains(mark) {
                    marks.push(*mark);
                }
            }
        }
        if let Some(last) = out.last_mut() {
            if last.marks == marks {
                last.text.push_str(text);
                return;
            }
        }
        out.push(TextRun::with_marks(text, marks));
    };
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Text(text) => push(text, &active),
            Token::Marker { kind, literal } => {
                if paired[index] {
                    match active.iter().rposition(|k| k == kind) {
                        Some(position) => {
                            active.remove(position);
                        }
                        None => active.push(*kind),
                    }
                } else {
                    push(literal, &active);
                }
            }
        }
    }
    out
}

/// Runs as paragraph content, with `\n` inside the text turned into hard breaks.
pub(crate) fn content(text: &str) -> Vec<InlineContent> {
    let mut out = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push(InlineContent::HardBreak);
        }
        out.extend(runs(line).into_iter().map(InlineContent::Text));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn describe(text: &str) -> Vec<(String, Vec<InlineMark>)> {
        runs(text).into_iter().map(|r| (r.text, r.marks)).collect()
    }

    #[test]
    fn emphasis_markers_become_marks_and_disappear() {
        let got = describe("plain *it* **bd** ***both*** _un_ end");
        let texts: Vec<&str> = got.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(texts, ["plain ", "it", " ", "bd", " ", "both", " ", "un", " end"]);
        assert_eq!(got[1].1, vec![InlineMark::Italic]);
        assert_eq!(got[3].1, vec![InlineMark::Bold]);
        assert!(got[5].1.contains(&InlineMark::Bold) && got[5].1.contains(&InlineMark::Italic));
        assert_eq!(got[7].1, vec![InlineMark::Underline]);
    }

    #[test]
    fn unpaired_markers_and_identifiers_stay_literal() {
        assert_eq!(describe("2 * 3 = 6")[0].0, "2 * 3 = 6");
        assert_eq!(describe("a lone * star").len(), 1);
        assert_eq!(describe("snake_case_name")[0].0, "snake_case_name");
    }

    #[test]
    fn a_backslash_escapes_a_marker() {
        let got = describe(r"not \*italic\* here");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, "not *italic* here");
    }

    #[test]
    fn nested_emphasis_combines_marks() {
        let got = describe("**bold _and underlined_ text**");
        assert!(got.iter().any(|(t, m)| t == "and underlined"
            && m.contains(&InlineMark::Bold)
            && m.contains(&InlineMark::Underline)));
    }
}
