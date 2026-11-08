//! Script elements, loose parsing and Fountain conversion.
//!
//! Scriptorium stores scripts as a sequence of typed blocks. That keeps the
//! editor predictable and makes exports to Fountain, Final Draft and PDF a
//! matter of walking the same list.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ElementKind {
    SceneHeading,
    Action,
    Character,
    Parenthetical,
    Dialogue,
    Transition,
    Shot,
    Act,
    Scene,
    StageDirection,
    Lyric,
    General,
}

impl ElementKind {
    pub fn label(self) -> &'static str {
        match self {
            ElementKind::SceneHeading => "Scene heading",
            ElementKind::Action => "Action",
            ElementKind::Character => "Character",
            ElementKind::Parenthetical => "Parenthetical",
            ElementKind::Dialogue => "Dialogue",
            ElementKind::Transition => "Transition",
            ElementKind::Shot => "Shot",
            ElementKind::Act => "Act",
            ElementKind::Scene => "Scene",
            ElementKind::StageDirection => "Stage direction",
            ElementKind::Lyric => "Lyric",
            ElementKind::General => "General",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            ElementKind::SceneHeading => "Scene",
            ElementKind::Action => "Action",
            ElementKind::Character => "Character",
            ElementKind::Parenthetical => "Parenthetical",
            ElementKind::Dialogue => "Dialogue",
            ElementKind::Transition => "Transition",
            ElementKind::Shot => "Shot",
            ElementKind::Act => "Act",
            ElementKind::Scene => "Scene",
            ElementKind::StageDirection => "Stage dir.",
            ElementKind::Lyric => "Lyric",
            ElementKind::General => "General",
        }
    }

    pub fn is_dialogue(self) -> bool {
        matches!(
            self,
            ElementKind::Character
                | ElementKind::Parenthetical
                | ElementKind::Dialogue
                | ElementKind::Lyric
        )
    }

    pub fn is_uppercase(self) -> bool {
        matches!(
            self,
            ElementKind::SceneHeading
                | ElementKind::Character
                | ElementKind::Transition
                | ElementKind::Shot
                | ElementKind::Act
        )
    }

    pub fn is_centered(self) -> bool {
        matches!(self, ElementKind::Transition)
    }

    /// Screenplay indentation in characters, used by text exports.
    pub fn indent(self) -> usize {
        match self {
            ElementKind::Character => 22,
            ElementKind::Parenthetical => 16,
            ElementKind::Dialogue => 10,
            ElementKind::Transition => 0,
            ElementKind::Act => 0,
            _ => 0,
        }
    }

    pub fn all(medium: crate::Medium) -> Vec<ElementKind> {
        match medium {
            crate::Medium::StagePlay => vec![
                ElementKind::Act,
                ElementKind::Scene,
                ElementKind::StageDirection,
                ElementKind::Character,
                ElementKind::Parenthetical,
                ElementKind::Dialogue,
                ElementKind::General,
            ],
            crate::Medium::TvScript | crate::Medium::Screenplay => vec![
                ElementKind::SceneHeading,
                ElementKind::Action,
                ElementKind::Character,
                ElementKind::Parenthetical,
                ElementKind::Dialogue,
                ElementKind::Transition,
                ElementKind::Shot,
                ElementKind::General,
            ],
            _ => vec![ElementKind::General],
        }
    }

    pub fn next(self, medium: crate::Medium) -> ElementKind {
        let all = Self::all(medium);
        if let Some(pos) = all.iter().position(|k| *k == self) {
            all[(pos + 1) % all.len()]
        } else {
            all[0]
        }
    }

    pub fn prev(self, medium: crate::Medium) -> ElementKind {
        let all = Self::all(medium);
        if let Some(pos) = all.iter().position(|k| *k == self) {
            all[(pos + all.len() - 1) % all.len()]
        } else {
            all[0]
        }
    }
}

fn is_all_caps(s: &str) -> bool {
    let letters: Vec<char> = s.chars().filter(|c| c.is_alphabetic()).collect();
    !letters.is_empty() && letters.iter().all(|c| c.is_uppercase())
}

/// Classify one paragraph when the author has not chosen an element.
pub fn classify(text: &str) -> ElementKind {
    let t = text.trim();
    if t.is_empty() {
        return ElementKind::Action;
    }
    let upper = t.to_uppercase();
    if upper.starts_with("INT.")
        || upper.starts_with("EXT.")
        || upper.starts_with("INT/EXT")
        || upper.starts_with("I/E")
        || upper.starts_with("EST.")
    {
        return ElementKind::SceneHeading;
    }
    if upper.starts_with("ACT ") || upper.starts_with("ACT\t") {
        return ElementKind::Act;
    }
    if upper.starts_with("SCENE ") {
        return ElementKind::Scene;
    }
    if upper.ends_with(" TO:") || upper.ends_with(" CUT TO:") || upper == "FADE OUT." {
        return ElementKind::Transition;
    }
    if t.starts_with('(') {
        return ElementKind::Parenthetical;
    }
    if is_all_caps(t) && t.len() <= 45 {
        return ElementKind::Character;
    }
    ElementKind::Action
}

/// Parse loose plain text into blocks, alternating character/dialogue when
/// the shape of the source suggests a script.
pub fn parse_loose(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut expect_dialogue = false;
    for raw in text.split("\n\n") {
        let para = raw.trim();
        if para.is_empty() {
            continue;
        }
        for line in para.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let kind = if expect_dialogue && !is_all_caps(line) && !line.ends_with(':') {
                ElementKind::Dialogue
            } else {
                classify(line)
            };
            expect_dialogue = kind == ElementKind::Character;
            blocks.push(Block {
                kind,
                text: line.to_string(),
            });
        }
    }
    if blocks.is_empty() {
        blocks.push(Block {
            kind: ElementKind::Action,
            text: String::new(),
        });
    }
    blocks
}

use crate::Block;

/// Render a block to Fountain.
pub fn block_to_fountain(block: &Block, medium: crate::Medium) -> String {
    let t = block.text.trim_end();
    match block.kind {
        ElementKind::SceneHeading => {
            if is_all_caps(t) {
                t.to_string()
            } else {
                t.to_uppercase()
            }
        }
        ElementKind::Action | ElementKind::General | ElementKind::StageDirection => t.to_string(),
        ElementKind::Character => {
            if medium == crate::Medium::StagePlay {
                format!("\n{}", t.to_uppercase())
            } else {
                format!("\n{}", t.to_uppercase())
            }
        }
        ElementKind::Parenthetical => format!("\n({})", t.trim_matches(|c| c == '(' || c == ')')),
        ElementKind::Dialogue => format!("\n{}", t),
        ElementKind::Transition => format!("\n> {}", t.to_uppercase()),
        ElementKind::Shot => format!("\n{}", t.to_uppercase()),
        ElementKind::Act => format!("\n# {}", t),
        ElementKind::Scene => format!("\n## {}", t),
        ElementKind::Lyric => format!("\n~{}", t),
    }
}

pub fn to_fountain(blocks: &[Block], medium: crate::Medium) -> String {
    let mut out = String::new();
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        out.push_str(&block_to_fountain(b, medium));
    }
    out.trim().to_string() + "\n"
}

/// Render a block as aligned plain text for screenplay PDFs and .txt.
pub fn block_to_plain(block: &Block) -> String {
    let text = if block.kind.is_uppercase() {
        block.text.to_uppercase()
    } else {
        block.text.clone()
    };
    if block.kind == ElementKind::Transition {
        return format!("{:>70}", text);
    }
    match block.kind {
        ElementKind::Character => format!("{:>26}{}", "", text.to_uppercase()),
        ElementKind::Parenthetical => format!("{:>18}{}", "", text),
        ElementKind::Dialogue => format!("{:>12}{}", "", text),
        _ => text,
    }
}

/// Convert title-page metadata into Fountain key/value form.
pub fn fountain_title_page(project: &crate::Project) -> String {
    let mut out = String::new();
    out.push_str(&format!("Title: {}\n", project.title));
    if !project.subtitle.is_empty() {
        out.push_str(&format!("Subtitle: {}\n", project.subtitle));
    }
    out.push_str(&format!("Author: {}\n", project.author));
    if !project.genre.is_empty() {
        out.push_str(&format!("Genre: {}\n", project.genre));
    }
    out.push_str("Draft date: ");
    out.push_str(&chrono::Local::now().format("%B %Y").to_string());
    out.push_str("\n\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_headings_are_recognised() {
        assert_eq!(classify("INT. HARBOUR OFFICE - NIGHT"), ElementKind::SceneHeading);
        assert_eq!(classify("EXT. PIER - DAWN"), ElementKind::SceneHeading);
    }

    #[test]
    fn character_names_are_recognised() {
        assert_eq!(classify("MARA"), ElementKind::Character);
        assert_eq!(classify("fade out."), ElementKind::Transition);
    }

    #[test]
    fn loose_script_alternates_dialogue() {
        let blocks = parse_loose("INT. ROOM - DAY\n\nMARA\n\nWe should go.\n\nShe leaves.");
        assert_eq!(blocks[0].kind, ElementKind::SceneHeading);
        assert_eq!(blocks[1].kind, ElementKind::Character);
        assert_eq!(blocks[2].kind, ElementKind::Dialogue);
    }

    #[test]
    fn fountain_roundtrip_keeps_scene() {
        let blocks = vec![Block {
            kind: ElementKind::SceneHeading,
            text: "INT. BOAT - NIGHT".into(),
        }];
        let fountain = to_fountain(&blocks, crate::Medium::Screenplay);
        assert!(fountain.contains("INT. BOAT - NIGHT"));
    }
}
