//! Script elements, loose parsing and Fountain conversion.
//!
//! Portmanteau stores scripts as a sequence of typed blocks. That keeps the
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
    Extension,
    Intercut,
    Subheading,
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
            ElementKind::Extension => "Extension",
            ElementKind::Intercut => "Intercut",
            ElementKind::Subheading => "Subheading",
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
            ElementKind::Extension => "Ext.",
            ElementKind::Intercut => "Intercut",
            ElementKind::Subheading => "Subhead",
        }
    }

    pub fn is_dialogue(self) -> bool {
        matches!(
            self,
            ElementKind::Character
                | ElementKind::Parenthetical
                | ElementKind::Dialogue
                | ElementKind::Lyric
                | ElementKind::Extension
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
                | ElementKind::Intercut
                | ElementKind::Subheading
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
            ElementKind::Extension => 22,
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
            crate::Medium::TvScript => {
                let mut kinds = Self::all(crate::Medium::Screenplay);
                kinds.push(ElementKind::Act);
                kinds
            }
            crate::Medium::Screenplay => vec![
                ElementKind::SceneHeading,
                ElementKind::Action,
                ElementKind::Character,
                ElementKind::Parenthetical,
                ElementKind::Dialogue,
                ElementKind::Transition,
                ElementKind::Shot,
                ElementKind::Intercut,
                ElementKind::Subheading,
                ElementKind::Extension,
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

/// Recognise common screenplay transitions, which are all-caps lines such as
/// `FADE IN:`, `CUT TO:`, `SMASH CUT TO BLACK.`, `DISSOLVE TO:`, `MATCH CUT TO:`.
fn is_transition(s: &str) -> bool {
    let u = s
        .trim()
        .trim_end_matches(|c: char| c == '.' || c == ':')
        .trim()
        .to_uppercase();
    u == "FADE IN"
        || u == "FADE OUT"
        || u == "FADE TO"
        || u == "FADE TO BLACK"
        || u == "CUT TO BLACK"
        || u == "TO BLACK"
        || u.ends_with(" TO BLACK")
        || u.ends_with(" CUT TO")
        || u.starts_with("CUT TO")
        || u.starts_with("SMASH CUT")
        || u.starts_with("MATCH CUT")
        || u.starts_with("JUMP CUT")
        || u.starts_with("HARD CUT")
        || u.starts_with("DISSOLVE TO")
        || u.starts_with("FADE TO")
        || u.ends_with("TO:")
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
    if is_transition(t) {
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
        ElementKind::Extension => format!("\n{}", t.to_uppercase()),
        ElementKind::Intercut => format!("\n> INTERCUT: {}", t.to_uppercase()),
        ElementKind::Subheading => format!("\n\n{}", t.to_uppercase()),
    }
}

/// Fountain for other programs (Highland, Fade In, Final Draft's importer).
///
/// This is the interchange form: standard Fountain with no hidden markers.
/// Portmanteau's own storage uses `to_storage_fountain` instead, because it
/// has to keep every element kind apart. What matters here is that another
/// reader classifies each element the way the writer did:
///
/// * a cue, its parentheticals and its dialogue are consecutive lines, with no
///   blank between them (a blank would turn the dialogue into action);
/// * one speech in several paragraphs is joined by a line of two spaces;
/// * text a reader would misread is *forced*: `!` for action that is all caps
///   or opens with a Fountain symbol, `.` for a scene heading that does not
///   start with INT/EXT, `@` for a character name with no letters.
pub fn to_fountain(blocks: &[Block], _medium: crate::Medium) -> String {
    let mut out = String::new();
    let mut previous: Option<ElementKind> = None;
    for block in blocks {
        let Some(text) = interchange_text(block) else { continue };
        if let Some(before) = previous {
            out.push_str(match (before, block.kind) {
                (ElementKind::Character | ElementKind::Parenthetical | ElementKind::Dialogue,
                 ElementKind::Parenthetical) => "\n",
                (ElementKind::Character | ElementKind::Parenthetical, ElementKind::Dialogue) => "\n",
                (ElementKind::Dialogue, ElementKind::Dialogue) => "\n  \n",
                _ => "\n\n",
            });
        }
        out.push_str(&text);
        previous = Some(block.kind);
    }
    out.trim_end().to_string() + "\n"
}

/// Would a Fountain reader take this first line for something other than action?
fn reads_as_something_else(line: &str) -> bool {
    is_all_caps(line)
        || line.starts_with(['!', '@', '#', '.', '>', '~', '=', '[', '('])
        || is_standard_heading(line)
}

fn is_standard_heading(text: &str) -> bool {
    let upper = text.to_uppercase();
    ["INT.", "INT ", "EXT.", "EXT ", "EST.", "EST ", "INT./EXT", "INT/EXT", "I/E"]
        .iter()
        .any(|prefix| upper.starts_with(prefix))
}

fn interchange_text(block: &Block) -> Option<String> {
    let text = block.text.trim();
    if text.is_empty() {
        return None;
    }
    let first_line = text.lines().next().unwrap_or("");
    Some(match block.kind {
        ElementKind::SceneHeading => {
            let heading = if is_all_caps(text) { text.to_string() } else { text.to_uppercase() };
            if is_standard_heading(&heading) { heading } else { format!(".{heading}") }
        }
        ElementKind::Action | ElementKind::General | ElementKind::StageDirection => {
            if reads_as_something_else(first_line) { format!("!{text}") } else { text.to_string() }
        }
        ElementKind::Shot | ElementKind::Extension | ElementKind::Subheading => {
            format!("!{}", text.to_uppercase())
        }
        ElementKind::Character => {
            let name = text.to_uppercase();
            if name.chars().any(char::is_alphabetic) { name } else { format!("@{name}") }
        }
        ElementKind::Parenthetical => format!("({})", text.trim_matches(|c| c == '(' || c == ')')),
        ElementKind::Dialogue => text.to_string(),
        ElementKind::Transition => format!("> {}", text.to_uppercase()),
        ElementKind::Act => format!("# {text}"),
        ElementKind::Scene => format!("## {text}"),
        ElementKind::Lyric => format!("~{text}"),
        ElementKind::Intercut => format!("> INTERCUT: {}", text.to_uppercase()),
    })
}

fn boneyard_marker(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::SceneHeading => "/* @scene-heading */",
        ElementKind::Action => "/* @action */",
        ElementKind::Character => "/* @character */",
        ElementKind::Parenthetical => "/* @parenthetical */",
        ElementKind::Dialogue => "/* @dialogue */",
        ElementKind::Transition => "/* @transition */",
        ElementKind::Shot => "/* @shot */",
        ElementKind::Act => "/* @act */",
        ElementKind::Scene => "/* @scene */",
        ElementKind::StageDirection => "/* @stage */",
        ElementKind::Lyric => "/* @lyric */",
        ElementKind::General => "/* @general */",
        ElementKind::Extension => "/* @extension */",
        ElementKind::Intercut => "/* @intercut */",
        ElementKind::Subheading => "/* @subheading */",
    }
}

fn marker_kind(line: &str) -> Option<ElementKind> {
    match line {
        "/* @scene-heading */" => Some(ElementKind::SceneHeading),
        "/* @action */" => Some(ElementKind::Action),
        "/* @character */" => Some(ElementKind::Character),
        "/* @parenthetical */" => Some(ElementKind::Parenthetical),
        "/* @dialogue */" => Some(ElementKind::Dialogue),
        "/* @transition */" => Some(ElementKind::Transition),
        "/* @shot */" => Some(ElementKind::Shot),
        "/* @act */" => Some(ElementKind::Act),
        "/* @scene */" => Some(ElementKind::Scene),
        "/* @stage */" => Some(ElementKind::StageDirection),
        "/* @lyric */" => Some(ElementKind::Lyric),
        "/* @general */" => Some(ElementKind::General),
        "/* @extension */" => Some(ElementKind::Extension),
        "/* @intercut */" => Some(ElementKind::Intercut),
        "/* @subheading */" => Some(ElementKind::Subheading),
        _ => None,
    }
}

/// Render blocks for on-disk storage.
///
/// This is Fountain, but every block carries a boneyard comment on the line
/// before its body. The marker makes the stored kind explicit, so a round trip
/// loses nothing even when Fountain's heuristics would read the body as a
/// different element (a lowercase action after a character cue, for example).
/// Standard Fountain readers ignore boneyard comments. An empty block emits
/// the marker line alone, so emptiness itself survives the round trip.
pub fn to_storage_fountain(blocks: &[Block], medium: crate::Medium) -> String {
    let mut out = String::new();
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        out.push_str(boneyard_marker(b.kind));
        out.push('\n');
        if !b.text.trim().is_empty() {
            let rendered = block_to_fountain(b, medium);
            out.push_str(rendered.trim_start_matches('\n'));
        }
    }
    out.trim().to_string() + "\n"
}

fn is_scene_heading(upper: &str) -> bool {
    upper.starts_with("INT.")
        || upper.starts_with("EXT.")
        || upper.starts_with("INT-EXT")
        || upper.starts_with("I-E")
        || upper.starts_with("I/E")
        || upper.starts_with("INT/EXT")
        || upper.starts_with("EST.")
}

/// Classify one Fountain line, if it carries explicit Fountain syntax.
fn fountain_syntax(line: &str) -> Option<ElementKind> {
    if line.starts_with("## ") {
        return Some(ElementKind::Scene);
    }
    if line.starts_with("# ") {
        return Some(ElementKind::Act);
    }
    if line.starts_with("> ") {
        return Some(ElementKind::Transition);
    }
    if line.starts_with('~') {
        return Some(ElementKind::Lyric);
    }
    if line.starts_with('(') {
        return Some(ElementKind::Parenthetical);
    }
    if is_scene_heading(&line.to_uppercase()) {
        return Some(ElementKind::SceneHeading);
    }
    if is_all_caps(line) && line.chars().count() <= 45 {
        return Some(ElementKind::Character);
    }
    None
}

/// Rebuild typed blocks from storage Fountain. This is the inverse of
/// `to_storage_fountain`.
///
/// A marker resolves the kind of the next non-empty text line. Two markers in
/// a row mean the first block had no body, so an empty block is emitted before
/// the second marker takes over. When no marker is present the old heuristics
/// still apply, so folders saved by the previous format and plain Fountain
/// imports keep working.
pub fn parse_fountain(text: &str, medium: crate::Medium) -> Vec<Block> {
    let _ = medium;
    let mut blocks = Vec::new();
    let mut pending: Option<ElementKind> = None;
    let mut expect_dialogue = false;
    for raw in text.split('\n') {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(kind) = marker_kind(line) {
            if let Some(empty_kind) = pending.take() {
                blocks.push(Block {
                    kind: empty_kind,
                    text: String::new(),
                });
            }
            pending = Some(kind);
            continue;
        }
        // Ignore any other boneyard comment rather than turning it into prose.
        if line.starts_with("/*") && line.ends_with("*/") {
            continue;
        }
        let kind = if let Some(kind) = pending.take() {
            kind
        } else if line.starts_with('(') {
            // A parenthetical is always a parenthetical inside a speech.
            ElementKind::Parenthetical
        } else if expect_dialogue {
            // Dialogue may be ALL CAPS; context wins over Fountain's
            // character-name heuristic.
            ElementKind::Dialogue
        } else if let Some(kind) = fountain_syntax(line) {
            kind
        } else {
            ElementKind::Action
        };
        let body = match kind {
            ElementKind::Act => line.strip_prefix("# ").unwrap_or(line).trim(),
            ElementKind::Scene => line.strip_prefix("## ").unwrap_or(line).trim(),
            ElementKind::Transition => line.strip_prefix("> ").unwrap_or(line).trim(),
            ElementKind::Intercut => line
                .strip_prefix("> INTERCUT:")
                .unwrap_or(line)
                .trim(),
            ElementKind::Lyric => line.strip_prefix('~').unwrap_or(line).trim(),
            _ => line,
        };
        expect_dialogue = match kind {
            ElementKind::Character => true,
            ElementKind::Parenthetical => expect_dialogue,
            _ => false,
        };
        blocks.push(Block {
            kind,
            text: body.to_string(),
        });
    }
    if let Some(kind) = pending.take() {
        blocks.push(Block {
            kind,
            text: String::new(),
        });
    }
    blocks
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
        ElementKind::Extension => format!("{:>26}{}", "", text.to_uppercase()),
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
    let contact: Vec<&str> = project.contact.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if !contact.is_empty() {
        // A multi-line Fountain value: the lines are indented beneath the key.
        out.push_str("Contact:\n");
        for line in contact {
            out.push_str(&format!("    {line}\n"));
        }
    }
    out.push_str("Draft date: ");
    if project.draft_date.trim().is_empty() {
        out.push_str(&chrono::Local::now().format("%B %Y").to_string());
    } else {
        out.push_str(project.draft_date.trim());
    }
    out.push_str("\n\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interchange_fountain_keeps_a_speech_together_and_forces_what_would_be_misread() {
        let blocks = vec![
            Block::new(ElementKind::SceneHeading, "A quiet room"),
            Block::new(ElementKind::Action, "THE END"),
            Block::new(ElementKind::Character, "Mara"),
            Block::new(ElementKind::Parenthetical, "quietly"),
            Block::new(ElementKind::Dialogue, "First."),
            Block::new(ElementKind::Dialogue, "Second."),
            Block::new(ElementKind::Action, "She leaves."),
        ];
        let fountain = to_fountain(&blocks, crate::Medium::Screenplay);
        assert_eq!(
            fountain,
            ".A QUIET ROOM\n\n!THE END\n\nMARA\n(quietly)\nFirst.\n  \nSecond.\n\nShe leaves.\n",
            "a cue and its speech are consecutive lines; odd headings and all-caps action are forced"
        );
    }

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
    fn transitions_are_recognised() {
        for text in [
            "FADE IN:",
            "FADE OUT.",
            "FADE TO BLACK.",
            "CUT TO:",
            "SMASH CUT TO BLACK.",
            "DISSOLVE TO:",
            "MATCH CUT TO:",
        ] {
            assert_eq!(classify(text), ElementKind::Transition, "{text}");
        }
        assert_eq!(classify("REID"), ElementKind::Character);
        assert_eq!(classify("CUTTER"), ElementKind::Character); // not a transition
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

    #[test]
    fn storage_fountain_roundtrips_every_kind() {
        let medium = crate::Medium::Screenplay;
        let cases = [
            (ElementKind::SceneHeading, "INT. HARBOUR OFFICE - NIGHT"),
            (ElementKind::Action, "The tide went out that morning."),
            (ElementKind::Character, "MARA"),
            (ElementKind::Parenthetical, "(to herself)"),
            (ElementKind::Dialogue, "We should go."),
            (ElementKind::Transition, "CUT TO:"),
            (ElementKind::Shot, "CLOSE ON THE COMPASS"),
            (ElementKind::Act, "ACT ONE"),
            (ElementKind::Scene, "The lamp room. Night."),
            (ElementKind::StageDirection, "Nell winds the mechanism."),
            (ElementKind::Lyric, "The sea remembers."),
            (ElementKind::General, "A note to the director."),
        ];
        for (kind, text) in cases {
            let original = Block {
                kind,
                text: text.into(),
            };
            let storage = to_storage_fountain(std::slice::from_ref(&original), medium);
            let parsed = parse_fountain(&storage, medium);
            assert_eq!(parsed.len(), 1, "{kind:?}: {storage:?}");
            assert_eq!(parsed[0].kind, kind, "{kind:?}: {storage:?}");
            assert_eq!(
                parsed[0].text,
                original.text.trim(),
                "{kind:?}: {storage:?}"
            );
        }
    }

    #[test]
    fn storage_fountain_keeps_dialogue_after_character() {
        let blocks = vec![
            Block::new(ElementKind::Character, "MARA"),
            Block::new(ElementKind::Parenthetical, "(quietly)"),
            Block::new(ElementKind::Dialogue, "The chart is wrong."),
        ];
        let storage = to_storage_fountain(&blocks, crate::Medium::Screenplay);
        assert_eq!(parse_fountain(&storage, crate::Medium::Screenplay), blocks);
    }

    #[test]
    fn storage_fountain_roundtrips_an_empty_action() {
        let blocks = vec![Block::new(ElementKind::Action, "")];
        let storage = to_storage_fountain(&blocks, crate::Medium::Screenplay);
        let parsed = parse_fountain(&storage, crate::Medium::Screenplay);
        assert_eq!(parsed, blocks, "{storage:?}");
    }

    #[test]
    fn storage_fountain_keeps_lowercase_action_after_character() {
        let blocks = vec![
            Block::new(ElementKind::Character, "MARA"),
            Block::new(ElementKind::Action, "She stands."),
        ];
        let storage = to_storage_fountain(&blocks, crate::Medium::Screenplay);
        let parsed = parse_fountain(&storage, crate::Medium::Screenplay);
        assert_eq!(parsed, blocks, "{storage:?}");
        assert_eq!(parsed[1].kind, ElementKind::Action);
    }

    #[test]
    fn storage_fountain_keeps_lowercase_character() {
        let blocks = vec![Block::new(ElementKind::Character, "mara")];
        let storage = to_storage_fountain(&blocks, crate::Medium::Screenplay);
        let parsed = parse_fountain(&storage, crate::Medium::Screenplay);
        assert_eq!(parsed.len(), 1, "{storage:?}");
        assert_eq!(parsed[0].kind, ElementKind::Character, "{storage:?}");
    }
}
