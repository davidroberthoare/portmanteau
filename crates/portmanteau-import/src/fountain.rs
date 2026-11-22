//! Fountain 1.1 (fountain.io): plain text screenplays.
//!
//! Read, in order: boneyard comments and notes are removed; a leading
//! `Key: value` block is the title page; the rest is split into elements by
//! the spec's rules (forced prefixes first, then scene headings, transitions,
//! character/dialogue blocks, and everything else as action); `#` sections
//! cut the file into documents.
//!
//! This is the *interchange* reader for files written elsewhere. Portmanteau's
//! own storage format lives in `portmanteau-core` and is read there.

use portmanteau_core::rich_text::script_style_id;
use portmanteau_core::{DocKind, ElementKind, Medium, RichDocument, RichParagraph};

use crate::{marks, ImportedDocument, Notes, Parsed, Split};

pub(crate) fn import(text: &str, medium: Medium, split: Split, notes: &mut Notes) -> Parsed {
    let (text, boneyards) = remove_spans(text, "/*", "*/");
    let (text, script_notes) = remove_spans(&text, "[[", "]]");
    notes.dropped("script note", script_notes);
    let _ = boneyards; // boneyard is the writer's own deleted text; nothing to report

    let (page, body) = title_page(&text);
    if !page.other.is_empty() {
        notes.say(format!("Title page details not imported: {}", page.other.join(", ")));
    }
    let items = parse_items(&body, notes);
    let documents = build_documents(items, medium, split, notes);
    Parsed {
        title: page.title,
        author: page.author,
        contact: page.contact,
        draft_date: page.draft_date,
        documents,
        styles: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Comments and notes
// ---------------------------------------------------------------------------

/// Remove every `open … close` span, across lines. An unclosed opener is
/// ordinary text, not a reason to swallow the rest of the file.
fn remove_spans(text: &str, open: &str, close: &str) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut removed = 0;
    while let Some(start) = rest.find(open) {
        let after = &rest[start + open.len()..];
        match after.find(close) {
            Some(end) => {
                out.push_str(&rest[..start]);
                rest = &after[end + close.len()..];
                removed += 1;
            }
            None => break,
        }
    }
    out.push_str(rest);
    (out, removed)
}

// ---------------------------------------------------------------------------
// Title page
// ---------------------------------------------------------------------------

#[derive(Default)]
struct TitlePage {
    title: Option<String>,
    author: Option<String>,
    /// The contact block, one line per line of the file.
    contact: Option<String>,
    draft_date: Option<String>,
    /// Keys the project has nowhere to put (Copyright, Revision, …).
    other: Vec<String>,
}

const TITLE_KEYS: [&str; 12] = [
    "title", "credit", "author", "authors", "source", "notes", "draft date", "date", "contact",
    "copyright", "revision", "watermark",
];

fn key_value(line: &str) -> Option<(String, String)> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    let valid = !key.is_empty()
        && key.len() <= 24
        && key.chars().all(|c| c.is_alphanumeric() || c == ' ');
    valid.then(|| (key.to_string(), value.trim().to_string()))
}

fn plain(text: &str) -> String {
    marks::runs(text).into_iter().map(|run| run.text).collect::<String>().trim().to_string()
}

/// Split off the title page, if the file starts with one.
fn title_page(text: &str) -> (TitlePage, String) {
    let lines: Vec<&str> = text.split('\n').collect();
    let first = lines.iter().position(|l| !l.trim().is_empty());
    let starts_with_key = first
        .and_then(|i| key_value(lines[i]))
        .is_some_and(|(key, _)| TITLE_KEYS.contains(&key.to_lowercase().as_str()));
    if !starts_with_key {
        return (TitlePage::default(), text.to_string());
    }

    let mut entries: Vec<(String, Vec<String>)> = Vec::new();
    let mut i = first.unwrap_or(0);
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            break;
        }
        if line.starts_with(char::is_whitespace) {
            if let Some((_, values)) = entries.last_mut() {
                values.push(line.trim().to_string());
            }
        } else if let Some((key, value)) = key_value(line) {
            entries.push((key, if value.is_empty() { vec![] } else { vec![value] }));
        } else {
            break;
        }
        i += 1;
    }

    let mut page = TitlePage::default();
    for (key, values) in entries {
        let joined = plain(&values.join(" "));
        if joined.is_empty() {
            continue;
        }
        if key.eq_ignore_ascii_case("contact") {
            // Contact is address-shaped: its line breaks matter.
            let lines: Vec<String> = values.iter().map(|v| plain(v)).filter(|v| !v.is_empty()).collect();
            page.contact = Some(lines.join("\n"));
            continue;
        }
        match key.to_lowercase().as_str() {
            "title" => page.title = Some(joined),
            "author" | "authors" => page.author = Some(joined),
            "draft date" | "date" => page.draft_date = Some(joined),
            // Credit ("Written by") is wording, not content.
            "credit" => {}
            _ => page.other.push(key),
        }
    }
    (page, lines[i.min(lines.len())..].join("\n"))
}

// ---------------------------------------------------------------------------
// Elements
// ---------------------------------------------------------------------------

enum Item {
    Element(ElementKind, String),
    /// A `#` section: depth and its title.
    Section(usize, String),
    Synopsis(String),
}

fn is_upper_line(text: &str) -> bool {
    // A parenthetical extension — (V.O.), (cont'd) — does not count.
    let mut outside = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => outside.push(c),
            _ => {}
        }
    }
    outside.chars().any(char::is_alphabetic) && !outside.chars().any(char::is_lowercase)
}

fn is_scene_heading(text: &str) -> bool {
    const PREFIXES: [&str; 12] = [
        "INT./EXT.", "INT/EXT.", "INT./EXT ", "INT/EXT ", "I/E.", "I/E ", "INT.", "INT ", "EXT.",
        "EXT ", "EST.", "EST ",
    ];
    let upper = text.to_uppercase();
    PREFIXES.iter().any(|prefix| upper.starts_with(prefix))
}

fn is_transition(text: &str) -> bool {
    if !is_upper_line(text) {
        return false;
    }
    text.ends_with("TO:")
        || (text.starts_with("FADE ") && (text.ends_with(':') || text.ends_with('.')))
        || text.ends_with(" OUT.")
}

fn is_character(text: &str) -> bool {
    is_upper_line(text)
        && text.len() <= 60
        && !text.ends_with(':')
        && !text.starts_with('(')
        && !is_scene_heading(text)
}

/// `EXT. HARBOUR - DAWN #12A#` -> `EXT. HARBOUR - DAWN`.
fn strip_scene_number(text: &str) -> &str {
    if let Some(stripped) = text.strip_suffix('#') {
        if let Some(at) = stripped.rfind(" #") {
            let number = &stripped[at + 2..];
            if !number.is_empty() && !number.contains(' ') {
                return stripped[..at].trim_end();
            }
        }
    }
    text
}

fn parse_items(body: &str, notes: &mut Notes) -> Vec<Item> {
    let lines: Vec<&str> = body.split('\n').collect();
    let blank = |index: usize| lines.get(index).map_or(true, |l| l.trim().is_empty());
    let mut items = Vec::new();
    let mut i = 0;
    let mut previous_blank = true;
    let mut dual = false;
    let mut centred = false;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            previous_blank = true;
            i += 1;
            continue;
        }
        let next_blank = blank(i + 1);

        if line.len() >= 3 && line.chars().all(|c| c == '=') {
            // A page break: layout, not content.
        } else if line.starts_with('#') {
            let depth = line.chars().take_while(|&c| c == '#').count();
            items.push(Item::Section(depth, plain(line[depth..].trim())));
        } else if let Some(synopsis) = line.strip_prefix('=') {
            items.push(Item::Synopsis(plain(synopsis.trim())));
        } else if let Some(heading) = line.strip_prefix('.').filter(|h| !h.starts_with('.')) {
            items.push(Item::Element(
                ElementKind::SceneHeading,
                strip_scene_number(heading.trim()).to_string(),
            ));
        } else if let Some(rest) = line.strip_prefix('>') {
            if line.ends_with('<') {
                centred = true;
                let inner = rest.trim_end_matches('<').trim();
                items.push(Item::Element(ElementKind::Action, inner.to_string()));
            } else {
                items.push(Item::Element(ElementKind::Transition, rest.trim().to_string()));
            }
        } else if line.starts_with('~') {
            let (text, next) = gather(&lines, i, |l| l.trim_start_matches('~').trim().to_string());
            items.push(Item::Element(ElementKind::Lyric, text));
            i = next;
            previous_blank = false;
            continue;
        } else if is_scene_heading(line) && previous_blank {
            items.push(Item::Element(
                ElementKind::SceneHeading,
                strip_scene_number(line).to_string(),
            ));
        } else if is_transition(line) && previous_blank && next_blank {
            items.push(Item::Element(ElementKind::Transition, line.to_string()));
        } else if (line.starts_with('@') || (is_character(line) && previous_blank && !next_blank))
            && !line.starts_with('!')
        {
            let mut name = line.trim_start_matches('@').trim().to_string();
            if let Some(stripped) = name.strip_suffix('^') {
                dual = true;
                name = stripped.trim_end().to_string();
            }
            items.push(Item::Element(ElementKind::Character, name));
            i += 1;
            i = dialogue(&lines, i, &mut items);
            previous_blank = false;
            continue;
        } else {
            let (text, next) = gather(&lines, i, |l| l.trim().to_string());
            // `!` forces action; it is not part of the text.
            let text = if text.starts_with('!') { text[1..].to_string() } else { text };
            items.push(Item::Element(ElementKind::Action, text));
            i = next;
            previous_blank = false;
            continue;
        }
        previous_blank = false;
        i += 1;
    }
    if dual {
        notes.say("Dual dialogue was imported as ordinary sequential dialogue");
    }
    if centred {
        notes.say("Centred text was imported as action");
    }
    items
}

/// Consecutive non-blank lines from `start`, joined by newlines.
fn gather(lines: &[&str], start: usize, clean: impl Fn(&str) -> String) -> (String, usize) {
    let mut end = start;
    let mut parts = Vec::new();
    while end < lines.len() && !lines[end].trim().is_empty() {
        parts.push(clean(lines[end]));
        end += 1;
    }
    (parts.join("\n"), end)
}

/// A line of two or more spaces and nothing else: Fountain's way to keep a
/// speech going across a paragraph break.
fn is_speech_break(line: &str) -> bool {
    line.len() >= 2 && line.chars().all(|c| c == ' ')
}

/// The parentheticals and dialogue after a character cue, up to a blank line.
fn dialogue(lines: &[&str], mut i: usize, items: &mut Vec<Item>) -> usize {
    let mut speech: Vec<String> = Vec::new();
    let flush = |speech: &mut Vec<String>, items: &mut Vec<Item>| {
        if !speech.is_empty() {
            items.push(Item::Element(ElementKind::Dialogue, speech.join("\n")));
            speech.clear();
        }
    };
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            // Two spaces keep the speech going; a truly empty line ends it.
            let continues = is_speech_break(lines[i])
                && lines.get(i + 1).is_some_and(|next| !next.trim().is_empty());
            if !continues {
                break;
            }
            flush(&mut speech, items);
        } else if line.starts_with('(') && line.ends_with(')') {
            flush(&mut speech, items);
            items.push(Item::Element(ElementKind::Parenthetical, line.to_string()));
        } else {
            speech.push(line.to_string());
        }
        i += 1;
    }
    flush(&mut speech, items);
    i
}

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

fn paragraph(kind: ElementKind, text: &str) -> RichParagraph {
    // Cues and headings are typed as written; emphasis is for prose-like lines.
    let content = match kind {
        ElementKind::SceneHeading | ElementKind::Character | ElementKind::Transition => {
            marks::content(&text.replace('*', "\\*").replace('_', "\\_"))
        }
        _ => marks::content(text),
    };
    RichParagraph {
        style_id: script_style_id(kind).into(),
        content,
    }
}

fn build_documents(
    items: Vec<Item>,
    medium: Medium,
    split: Split,
    notes: &mut Notes,
) -> Vec<ImportedDocument> {
    let by_sections = split == Split::Auto
        && items.iter().any(|item| matches!(item, Item::Section(1, _)));
    let kind = if medium.is_script() { DocKind::Scene } else { DocKind::Chapter };
    let new_doc = |title: String| ImportedDocument {
        title,
        kind,
        medium,
        content: RichDocument::default(),
        synopsis: String::new(),
    };

    let mut documents: Vec<ImportedDocument> = Vec::new();
    let mut current = new_doc(String::new());
    let mut outline_markers = 0;
    for item in items {
        match item {
            Item::Section(1, title) if by_sections => {
                if !current.content.paragraphs.is_empty() || !current.title.is_empty() {
                    documents.push(current);
                }
                current = new_doc(title);
            }
            Item::Section(..) => outline_markers += 1,
            Item::Synopsis(text) => {
                // A synopsis directly under a section describes that document.
                if by_sections && current.content.paragraphs.is_empty() && !text.is_empty() {
                    if !current.synopsis.is_empty() {
                        current.synopsis.push(' ');
                    }
                    current.synopsis.push_str(&text);
                } else {
                    outline_markers += 1;
                }
            }
            Item::Element(element, text) => {
                current.content.paragraphs.push(paragraph(element, &text));
            }
        }
    }
    if !current.content.paragraphs.is_empty() || documents.is_empty() {
        documents.push(current);
    }
    notes.dropped("outline marker", outline_markers);
    documents
}
