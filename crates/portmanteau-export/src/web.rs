//! Markup and CSS shared by the two web-based writers, HTML and EPUB.
//!
//! The rule for this module: every visual decision comes from an
//! `ElementStyle` (font, size, leading, indents, spacing, alignment) or from
//! the resolved page margins. Nothing here owns a size, indent or margin of its
//! own, so changing a style in Compile changes the output.
//!
//! Layout of a generated stylesheet, in order:
//!   1. the page (body) and, for HTML, the centred text column;
//!   2. headings, the title page and the other fixed furniture;
//!   3. one class per script element (`.script-dialogue`, …);
//!   4. one class per distinct prose paragraph format (`.p1`, `.p2`, …).

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebTarget {
    /// One scrolling page, opened in a browser or printed.
    Html,
    /// Reflowable pages inside an EPUB package.
    Epub,
}

const INK: &str = "#1b1b1b";
const HTML_PAPER: &str = "#fbf8f1";

// ---------------------------------------------------------------------------
// Styles -> CSS
// ---------------------------------------------------------------------------

/// CSS keyword for an element's paragraph alignment.
pub(crate) fn css_align(align: Alignment) -> &'static str {
    match align {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "justify",
    }
}

/// A CSS font stack. The bundled Liberation face is the explicit fallback so
/// an EPUB looks the same whether or not the reader has the named family.
pub(crate) fn css_font_family(family: &str) -> String {
    // Font names are stored data and end up inside a CSS string inside a
    // <style> element, so keep only characters a family name really uses. This
    // closes quotes, backslashes and `</style>` breakouts at the source.
    let safe: String = family
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
        .collect();
    let safe = safe.trim().to_string();
    let lower = safe.to_ascii_lowercase();
    let (generic, bundled) = if font_is_mono(&lower) {
        ("monospace", "Liberation Mono")
    } else if lower.contains("sans") || lower.contains("helvetica") || lower.contains("arial") {
        ("sans-serif", "Liberation Sans")
    } else {
        ("serif", "Liberation Serif")
    };
    if safe.is_empty() || safe.eq_ignore_ascii_case(bundled) {
        format!("\"{bundled}\", {generic}")
    } else {
        format!("\"{safe}\", \"{bundled}\", {generic}")
    }
}

pub(crate) fn font_is_mono(family: &str) -> bool {
    let family = family.to_ascii_lowercase();
    family.contains("courier") || family.contains("mono")
}

/// The typeface half of a style: family, size, leading and slant.
pub(crate) fn text_declarations(style: &ElementStyle, fallback_font: &str) -> String {
    let family = if style.font.trim().is_empty() {
        fallback_font
    } else {
        style.font.as_str()
    };
    let mut out = format!(
        "font-family: {}; font-size: {:.1}pt; line-height: {:.2};",
        css_font_family(family),
        style.size.max(8.0),
        style.leading.max(1.0),
    );
    if style.italic {
        out.push_str(" font-style: italic;");
    }
    out
}

/// The paragraph half of a style: alignment, indents and spacing.
///
/// `before`/`after` map to `margin-top`/`margin-bottom`. CSS collapses
/// adjacent vertical margins (the larger wins) where a word processor adds
/// them; that is the one place the web output can differ from DOCX/PDF.
pub(crate) fn paragraph_declarations(style: &ElementStyle) -> String {
    format!(
        "text-align: {}; margin: {:.1}pt {:.1}pt {:.1}pt {:.1}pt; text-indent: {:.1}pt;",
        css_align(style.align),
        style.before.max(0.0),
        style.right_indent.max(0.0),
        style.after.max(0.0),
        style.indent.max(0.0),
        style.first_line_indent,
    )
}

/// The one place a whole `ElementStyle` becomes CSS declarations.
pub(crate) fn declarations(style: &ElementStyle, fallback_font: &str) -> String {
    format!(
        "{} {}",
        text_declarations(style, fallback_font),
        paragraph_declarations(style)
    )
}

/// Collects the distinct paragraph formats a document uses and gives each one
/// a class, so the markup stays clean (`<p class="p2">`) and the CSS is
/// generated in one place.
#[derive(Default)]
pub(crate) struct ParagraphClasses {
    rules: Vec<String>,
    /// Scene numbering runs across every script document in the compile.
    scenes: SceneNumbers,
}

impl ParagraphClasses {
    pub(crate) fn class_for(&mut self, style: &ElementStyle, fallback_font: &str) -> String {
        let rule = declarations(style, fallback_font);
        let index = match self.rules.iter().position(|known| *known == rule) {
            Some(index) => index,
            None => {
                self.rules.push(rule);
                self.rules.len() - 1
            }
        };
        format!("p{}", index + 1)
    }

    fn css(&self) -> String {
        self.rules
            .iter()
            .enumerate()
            .map(|(index, rule)| format!(".p{} {{ {rule} }}\n", index + 1))
            .collect()
    }
}

/// CSS class for a screenplay element. Exhaustive on purpose: adding an
/// `ElementKind` must be a compile error here, not a silently unstyled class.
pub(crate) fn script_class(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::SceneHeading => "script-scene-heading",
        ElementKind::Action => "script-action",
        ElementKind::Character => "script-character",
        ElementKind::Parenthetical => "script-parenthetical",
        ElementKind::Dialogue => "script-dialogue",
        ElementKind::Transition => "script-transition",
        ElementKind::Shot => "script-shot",
        ElementKind::Act => "script-act",
        ElementKind::Scene => "script-scene",
        ElementKind::StageDirection => "script-stage-direction",
        ElementKind::Lyric => "script-lyric",
        ElementKind::General => "script-general",
        ElementKind::Extension => "script-extension",
        ElementKind::Intercut => "script-intercut",
        ElementKind::Subheading => "script-subheading",
    }
}

pub(crate) const ALL_KINDS: [ElementKind; 15] = [
    ElementKind::SceneHeading,
    ElementKind::Action,
    ElementKind::Character,
    ElementKind::Parenthetical,
    ElementKind::Dialogue,
    ElementKind::Transition,
    ElementKind::Shot,
    ElementKind::Act,
    ElementKind::Scene,
    ElementKind::StageDirection,
    ElementKind::Lyric,
    ElementKind::General,
    ElementKind::Extension,
    ElementKind::Intercut,
    ElementKind::Subheading,
];

/// The whole stylesheet for a web target (without EPUB `@font-face` rules; the
/// EPUB writer prepends those for the faces it actually embeds).
pub(crate) fn stylesheet(
    project: &Project,
    settings: &ExportSettings,
    target: WebTarget,
    paragraphs: &ParagraphClasses,
) -> String {
    let st = &settings.styles;
    let serif = "Liberation Serif";
    let mono = "Courier New";
    let margins = page_margins(project, settings);
    let (paper_width, _) = paper_pt(settings);
    let mut css = String::new();

    // 1. The page.
    match target {
        WebTarget::Epub => {
            // A reflowable book has no fixed page, so the side margins are the
            // same fraction of the width the writer chose on the paper.
            css.push_str(&format!(
                "body {{ margin: 0 {:.2}% 0 {:.2}%; color: {INK}; {} }}\n",
                margins.right / paper_width * 100.0,
                margins.left / paper_width * 100.0,
                text_declarations(&st.body, serif),
            ));
        }
        WebTarget::Html => {
            css.push_str(&format!(
                "body {{ margin: 0; background: {HTML_PAPER}; color: {INK}; }}\n\
                 .page {{ box-sizing: content-box; max-width: {:.1}pt; margin: {:.1}pt auto {:.1}pt; padding: 0 1rem; }}\n",
                paper_width - margins.left - margins.right,
                margins.top,
                margins.bottom,
            ));
        }
    }

    // 2. Headings and the title page.
    css.push_str(&format!(".heading {{ {} font-weight: bold; }}\n", declarations(&st.chapter_heading, serif)));
    css.push_str(".chapter { break-before: page; page-break-before: always; }\n");
    if project.compile_plan(settings).kind.is_script() {
        // The script cover: its own lines carry the space above them, and the
        // script starts on the next page even when its first document runs on.
        css.push_str(".title-page { break-after: page; page-break-after: always; }\n");
        for line in script_cover_lines(project, settings) {
            let class = cover_class(line.role);
            let style = ElementStyle { before: 0.0, ..line.style.clone() };
            let rule = format!(".title-page .{class} {{ {} }}\n", declarations(&style, mono));
            if !css.contains(&rule) {
                css.push_str(&rule);
            }
        }
    } else {
    css.push_str(&format!(
        ".title-page {{ margin-top: {:.1}pt; }}\n.title-page .title {{ {} font-weight: bold; }}\n",
        title_drop_pt(project, settings),
        declarations(&st.title_page_title, serif)
    ));
    css.push_str(&format!(".title-page .subtitle {{ {} }}\n", declarations(&st.title_page_subtitle, serif)));
    css.push_str(&format!(".title-page .byline {{ {} }}\n", declarations(&st.title_page_author, serif)));
    css.push_str(".title-page .imprint { margin-top: 4em; font-size: .85em; color: #555; text-align: center; }\n");
    }
    css.push_str(".toc a { color: inherit; }\n.cover { text-align: center; margin: 0; padding: 0; }\n.cover img { max-width: 100%; }\n");

    // 3. Script elements, one class per kind, straight from the element styles.
    for kind in ALL_KINDS {
        css.push_str(&format!(
            ".{} {{ {} }}\n",
            script_class(kind),
            declarations(st.style_for_kind(kind), mono)
        ));
    }

    css.push_str(&format!(
        ".script-opening {{ {} }}\n",
        declarations(&opening_transition_style(st.style_for_kind(ElementKind::Transition)), mono)
    ));

    // 4. Prose paragraph formats collected while the body was rendered.
    css.push_str(&paragraphs.css());
    css
}

// ---------------------------------------------------------------------------
// Markup
// ---------------------------------------------------------------------------

pub(crate) fn html_runs(paragraph: &ProseParagraph) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                let mut text = html_escape(&paragraph.transform(&run.text));
                if run.marks.contains(&InlineMark::Italic) {
                    text = format!("<em>{text}</em>");
                }
                if run.marks.contains(&InlineMark::Bold) {
                    text = format!("<strong>{text}</strong>");
                }
                if run.marks.contains(&InlineMark::Underline) {
                    text = format!("<u>{text}</u>");
                }
                if run.marks.contains(&InlineMark::Strikethrough) {
                    text = format!("<s>{text}</s>");
                }
                out.push_str(&text);
            }
            InlineContent::HardBreak => out.push_str("<br/>"),
            InlineContent::Image(image) if image.alt_text.is_empty() => out.push_str("[Image]"),
            InlineContent::Image(image) => out.push_str(&html_escape(&format!(
                "[Image: {}]",
                paragraph.transform(&image.alt_text)
            ))),
        }
    }
    out
}

/// One prose paragraph as a `<p>` (or `<h2>` for a heading style).
pub(crate) fn html_prose_paragraph(
    paragraph: &ProseParagraph,
    classes: &mut ParagraphClasses,
    fallback_font: &str,
) -> String {
    let class = classes.class_for(&paragraph.format, fallback_font);
    let runs = html_runs(paragraph);
    if paragraph.heading {
        format!("<h2 class=\"heading {class}\">{runs}</h2>\n")
    } else {
        format!("<p class=\"{class}\">{runs}</p>\n")
    }
}

/// A screenplay document as classed paragraphs.
pub(crate) fn html_script_blocks(
    project: &Project,
    doc: &Document,
    settings: &ExportSettings,
    scenes: &mut SceneNumbers,
) -> String {
    let mut out = String::new();
    for block in screenplay_blocks(project, doc) {
        let text = if settings.styles.style_for_kind(block.kind).uppercase {
            block.text.to_uppercase()
        } else {
            block.text.clone()
        };
        let text = scenes.apply(block.kind, text, settings.include_scene_numbers);
        let class = if opens_script(block.kind, &text) {
            format!("{} script-opening", script_class(block.kind))
        } else {
            script_class(block.kind).to_string()
        };
        out.push_str(&format!("<p class=\"{class}\">{}</p>\n", html_escape(&text)));
    }
    out
}

/// The body of one document's pages: prose paragraphs or script elements.
pub(crate) fn html_document_body(
    project: &Project,
    doc: &Document,
    settings: &ExportSettings,
    classes: &mut ParagraphClasses,
) -> String {
    if doc.medium.is_script() {
        html_script_blocks(project, doc, settings, &mut classes.scenes)
    } else {
        prose_paragraphs(project, doc, settings)
            .iter()
            .map(|paragraph| html_prose_paragraph(paragraph, classes, &settings.font))
            .collect()
    }
}

fn cover_class(role: TitleRole) -> &'static str {
    match role {
        TitleRole::Title => "cover-title",
        TitleRole::Credit => "cover-credit",
        TitleRole::Footer => "cover-footer",
    }
}

/// The title page block shared by both targets.
pub(crate) fn html_title_page(project: &Project, settings: &ExportSettings) -> String {
    if project.compile_plan(settings).kind.is_script() {
        let mut out = String::from("<div class=\"title-page\">\n");
        for line in script_cover_lines(project, settings) {
            let spacing = if line.style.before > 0.0 {
                format!(" style=\"margin-top: {:.1}pt\"", line.style.before)
            } else {
                String::new()
            };
            out.push_str(&format!(
                "<p class=\"{}\"{spacing}>{}</p>\n",
                cover_class(line.role),
                html_escape(&line.text)
            ));
        }
        out.push_str("</div>\n");
        return out;
    }
    let mut out = String::from("<div class=\"title-page\">\n");
    out.push_str(&format!("<p class=\"title\">{}</p>\n", html_escape(&project.title)));
    if !project.subtitle.trim().is_empty() {
        out.push_str(&format!("<p class=\"subtitle\">{}</p>\n", html_escape(&project.subtitle)));
    }
    if !project.author.trim().is_empty() {
        out.push_str(&format!("<p class=\"byline\">by {}</p>\n", html_escape(&project.author)));
    }
    if !project.genre.trim().is_empty() {
        out.push_str(&format!("<p class=\"imprint\">{}</p>\n", html_escape(&project.genre)));
    }
    out.push_str("</div>\n");
    out
}
