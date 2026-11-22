//! Plain text prose: paragraphs, and chapters when the text announces them.
//!
//! Paragraphs are blank-line separated and hard-wrapped lines are rejoined; a
//! file with no blank lines at all is read one paragraph per line, which is
//! how most plain-text exports are written. A line that stands alone and
//! reads like a chapter heading ("Chapter 3", "PROLOGUE") starts a new
//! document and becomes its title.

use portmanteau_core::{DocKind, InlineContent, Medium, RichDocument, RichParagraph, TextRun};

use crate::{ImportedDocument, Parsed, Split};

pub(crate) fn import(text: &str, split: Split) -> Parsed {
    let blocks = blocks(text);
    let headings = if split == Split::Auto { heading_indexes(&blocks) } else { Vec::new() };

    let new_doc = |title: String| ImportedDocument {
        title,
        kind: DocKind::Chapter,
        medium: Medium::Prose,
        content: RichDocument::default(),
        synopsis: String::new(),
    };
    let mut documents: Vec<ImportedDocument> = Vec::new();
    let mut current = new_doc(String::new());
    for (index, block) in blocks.into_iter().enumerate() {
        if headings.contains(&index) {
            if !current.content.paragraphs.is_empty() || !current.title.is_empty() {
                documents.push(current);
            }
            current = new_doc(block);
        } else {
            current.content.paragraphs.push(RichParagraph {
                style_id: "body".into(),
                content: vec![InlineContent::Text(TextRun::plain(block))],
            });
        }
    }
    if !current.content.paragraphs.is_empty() || documents.is_empty() {
        documents.push(current);
    }
    Parsed {
        title: None,
        author: None,
        contact: None,
        draft_date: None,
        documents,
        styles: Vec::new(),
    }
}

/// The text's paragraphs, in order.
fn blocks(text: &str) -> Vec<String> {
    if text.lines().any(|line| line.trim().is_empty()) {
        let mut blocks = Vec::new();
        let mut block: Vec<&str> = Vec::new();
        for line in text.lines().chain(std::iter::once("")) {
            if line.trim().is_empty() {
                if !block.is_empty() {
                    blocks.push(block.join(" "));
                    block.clear();
                }
            } else {
                block.push(line.trim());
            }
        }
        blocks
    } else {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    }
}

const WORDS: [&str; 22] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
    "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
    "twenty", "thirty", "forty",
];

fn is_roman(word: &str) -> bool {
    !word.is_empty() && word.len() <= 7 && word.chars().all(|c| "IVXLCDM".contains(c.to_ascii_uppercase()))
}

fn is_number(word: &str) -> bool {
    let word = word.trim_end_matches(['.', ':']);
    (!word.is_empty() && word.len() <= 3 && word.chars().all(|c| c.is_ascii_digit()))
        || is_roman(word)
        || WORDS.contains(&word.to_lowercase().as_str())
}

/// A heading that names itself: "Chapter 3", "Part II: The Crossing", "Prologue".
fn is_keyword_heading(line: &str) -> bool {
    if line.len() > 80 || line.ends_with(['.', ',', ';']) && !line.ends_with("...") && line.split_whitespace().count() > 6 {
        return false;
    }
    let lower = line.to_lowercase();
    let first = lower.split(|c: char| c.is_whitespace() || c == ':' || c == '.').next().unwrap_or("");
    if ["prologue", "epilogue", "foreword", "preface", "afterword", "introduction", "interlude"]
        .contains(&first)
        && line.split_whitespace().count() <= 4
    {
        return true;
    }
    if ["chapter", "part", "book", "section"].contains(&first) {
        return line
            .split_whitespace()
            .nth(1)
            .is_some_and(|word| is_number(word.trim_end_matches([':', '.'])));
    }
    false
}

/// A bare "7" or "IV." standing alone: only trusted when there are several.
/// Spelled-out numbers do not count here; a line that just says "Two" is
/// as likely to be a word as a chapter.
fn is_numeral_heading(line: &str) -> bool {
    if line.split_whitespace().count() != 1 || line.len() > 6 {
        return false;
    }
    let bare = line.trim_end_matches(['.', ':']);
    let dotted = bare.len() != line.len();
    let digits = !bare.is_empty() && bare.len() <= 3 && bare.chars().all(|c| c.is_ascii_digit());
    let roman = bare.chars().all(|c| "IVXLCDM".contains(c)) && (bare.len() >= 2 || dotted);
    digits || (roman && !bare.is_empty())
}

fn heading_indexes(blocks: &[String]) -> Vec<usize> {
    let keyword: Vec<usize> = blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| is_keyword_heading(block))
        .map(|(index, _)| index)
        .collect();
    let numeral: Vec<usize> = blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| is_numeral_heading(block))
        .map(|(index, _)| index)
        .collect();
    let mut all = keyword;
    // A lone "1." is more likely a list than a chapter.
    if numeral.len() >= 2 {
        all.extend(numeral);
    }
    all.sort_unstable();
    // If every block looks like a heading, none of them is.
    if all.len() == blocks.len() {
        return Vec::new();
    }
    all
}
