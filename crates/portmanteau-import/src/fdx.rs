//! Final Draft `.fdx`: the industry's XML screenplay format.
//!
//! The body is a flat run of `<Paragraph Type="…">` elements, each holding
//! styled `<Text>` pieces. Anything we have no home for — an unknown paragraph
//! type, an embedded image, a script note — is set aside with a warning rather
//! than failing the import.

use portmanteau_core::rich_text::script_style_id;
use portmanteau_core::{
    DocKind, ElementKind, InlineContent, InlineMark, Medium, RichDocument, RichParagraph, TextRun,
};
use roxmltree::Node;

use crate::{ImportedDocument, Notes, Parsed};

pub(crate) fn import(xml: &str, medium: Medium, notes: &mut Notes) -> Result<Parsed, String> {
    let document = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 4_000_000,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .map_err(|error| format!("Could not read the FDX file as XML: {error}"))?;
    if document.root_element().tag_name().name() != "FinalDraft" {
        return Err("This XML file is not a Final Draft document".into());
    }

    let (title, author) = title_page(&document);
    let mut paragraphs = Vec::new();
    let mut unknown: Vec<String> = Vec::new();

    let content = document
        .root_element()
        .children()
        .find(|node| node.has_tag_name("Content"));
    if let Some(content) = content {
        // Dual dialogue wraps ordinary paragraphs, so look inside it too.
        for node in content.descendants().filter(|n| n.has_tag_name("Paragraph")) {
            if node.ancestors().skip(1).any(|a| a.has_tag_name("Paragraph")) {
                continue; // a note's own paragraph, handled below
            }
            let fdx_type = node.attribute("Type").unwrap_or("General");
            let kind = element_kind(fdx_type).unwrap_or_else(|| {
                if !unknown.iter().any(|t| t == fdx_type) {
                    unknown.push(fdx_type.to_string());
                }
                ElementKind::General
            });
            notes.dropped("embedded image", count(&node, "Image") + count(&node, "Movie"));
            notes.dropped("script note", count(&node, "ScriptNote"));

            let runs: Vec<InlineContent> = node
                .children()
                .filter(|child| child.has_tag_name("Text"))
                .filter_map(|text| {
                    let value = text.text().unwrap_or_default();
                    (!value.is_empty()).then(|| {
                        InlineContent::Text(TextRun::with_marks(value, marks(text.attribute("Style"))))
                    })
                })
                .collect();
            if runs.is_empty() {
                continue; // blank spacing paragraphs carry no writing
            }
            paragraphs.push(RichParagraph {
                style_id: script_style_id(kind).into(),
                content: runs,
            });
        }
    }
    if !unknown.is_empty() {
        notes.say(format!(
            "Unknown Final Draft paragraph types were imported as General: {}",
            unknown.join(", ")
        ));
    }

    Ok(Parsed {
        title,
        author,
        contact: None,
        draft_date: None,
        documents: vec![ImportedDocument {
            title: String::new(),
            kind: DocKind::Scene,
            medium,
            content: RichDocument { paragraphs },
            synopsis: String::new(),
        }],
    })
}

fn count(node: &Node, tag: &str) -> usize {
    node.descendants().filter(|n| n.has_tag_name(tag)).count()
}

/// The title page is a few centred paragraphs: the title first, then a
/// "Written by" line, then the author.
fn title_page(document: &roxmltree::Document) -> (Option<String>, Option<String>) {
    let Some(page) = document.descendants().find(|n| n.has_tag_name("TitlePage")) else {
        return (None, None);
    };
    let lines: Vec<String> = page
        .descendants()
        .filter(|n| n.has_tag_name("Paragraph"))
        .map(|p| {
            p.descendants()
                .filter(|t| t.has_tag_name("Text"))
                .filter_map(|t| t.text())
                .collect::<String>()
                .trim()
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect();
    let title = lines.first().cloned();
    let author = lines.iter().enumerate().find_map(|(index, line)| {
        let lower = line.to_lowercase();
        for prefix in ["written by ", "by "] {
            if lower.starts_with(prefix) {
                return Some(line[prefix.len()..].trim().to_string());
            }
        }
        // "Written by" alone on a line: the author is the next line.
        (index > 0 && (lower == "written by" || lower == "by"))
            .then(|| lines.get(index + 1).cloned())
            .flatten()
    });
    (title, author)
}

pub(crate) fn element_kind(fdx_type: &str) -> Option<ElementKind> {
    let normalized: String = fdx_type
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect();
    Some(match normalized.as_str() {
        "sceneheading" | "slugline" => ElementKind::SceneHeading,
        "action" => ElementKind::Action,
        "character" | "charactername" => ElementKind::Character,
        "parenthetical" => ElementKind::Parenthetical,
        "dialogue" => ElementKind::Dialogue,
        "transition" => ElementKind::Transition,
        "shot" => ElementKind::Shot,
        "extension" => ElementKind::Extension,
        "intercut" => ElementKind::Intercut,
        "subheading" => ElementKind::Subheading,
        "act" | "newact" | "endofact" => ElementKind::Act,
        "scene" => ElementKind::Scene,
        "stagedirection" => ElementKind::StageDirection,
        "lyric" | "lyrics" => ElementKind::Lyric,
        "general" => ElementKind::General,
        _ => return None,
    })
}

/// FDX writes combined styles as `Bold+Italic`.
fn marks(style: Option<&str>) -> Vec<InlineMark> {
    let lower = style.unwrap_or_default().to_lowercase();
    [
        ("bold", InlineMark::Bold),
        ("italic", InlineMark::Italic),
        ("underline", InlineMark::Underline),
        ("strike", InlineMark::Strikethrough),
    ]
    .into_iter()
    .filter(|(needle, _)| lower.contains(needle))
    .map(|(_, mark)| mark)
    .collect()
}
