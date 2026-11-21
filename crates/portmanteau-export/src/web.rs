use super::*;

/// CSS declarations that carry one paragraph's resolved appearance. Inline
/// declarations keep EPUB/XHTML output correct even when two project sheets
/// define a style with the same id.
pub(crate) fn prose_inline_style(format: &ElementStyle) -> String {
    // The declarations live inside a double-quoted attribute, so family names
    // use CSS single quotes rather than the double quotes from the shared
    // stylesheet helper.
    let family = html_escape(&element_css_font(format, "Liberation Serif").replace('"', "'"));
    let italic = if format.italic {
        "font-style: italic; "
    } else {
        ""
    };
    format!(
        "font-family: {family}; {italic}font-size: {:.1}pt; line-height: {:.2}; text-align: {}; margin-left: {:.1}pt; margin-right: {:.1}pt; text-indent: {:.1}pt; margin-top: {:.1}pt; margin-bottom: {:.1}pt;",
        format.size.max(8.0),
        format.leading.max(1.0),
        css_align(format.align),
        format.indent.max(0.0),
        format.right_indent.max(0.0),
        format.first_line_indent,
        format.before.max(0.0),
        format.after.max(0.0),
    )
}

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

pub(crate) fn html_prose_paragraph(paragraph: &ProseParagraph) -> String {
    let style = prose_inline_style(&paragraph.format);
    let runs = html_runs(paragraph);
    if paragraph.heading {
        format!("<h2 style=\"{style}\">{runs}</h2>\n")
    } else {
        format!("<p style=\"{style}\">{runs}</p>\n")
    }
}

/// A CSS font stack built from the settings family. The bundled Liberation
/// face is the explicit fallback so an EPUB looks the same whether or not the
/// reader happens to have the named family installed.
pub(crate) fn css_font_family(family: &str) -> String {
    let safe = family.replace('"', "").replace('\\', "");
    let lower = safe.to_ascii_lowercase();
    let (generic, bundled) = if lower.contains("mono") || lower.contains("courier") {
        ("monospace", "Liberation Mono")
    } else if lower.contains("sans") || lower.contains("helvetica") || lower.contains("arial") {
        ("sans-serif", "Liberation Sans")
    } else {
        ("serif", "Liberation Serif")
    };
    if safe.eq_ignore_ascii_case(bundled) || safe.is_empty() {
        format!("\"{bundled}\", {generic}")
    } else {
        format!("\"{safe}\", \"{bundled}\", {generic}")
    }
}

/// CSS family for one element, falling back when its family is unset.
pub(crate) fn element_css_font(style: &ElementStyle, fallback: &str) -> String {
    let family = if style.font.trim().is_empty() {
        fallback
    } else {
        style.font.as_str()
    };
    css_font_family(family)
}

/// CSS left margin: a point indent converted to characters, or the old percentage.
pub(crate) fn element_css_indent(style: &ElementStyle, fallback: &str) -> String {
    if style.indent > 0.0 {
        format!("{:.2}ch", style.indent / 7.2)
    } else {
        fallback.to_string()
    }
}

/// CSS keyword for an element's paragraph alignment.
pub(crate) fn css_align(align: Alignment) -> &'static str {
    match align {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "justify",
    }
}

/// CSS declarations for left/right indents, empty when both are zero.
pub(crate) fn css_indent_margins(style: &ElementStyle) -> String {
    let mut out = String::new();
    if style.indent > 0.0 {
        out.push_str(&format!("margin-left: {:.2}pt; ", style.indent));
    }
    if style.right_indent > 0.0 {
        out.push_str(&format!("margin-right: {:.2}pt; ", style.right_indent));
    }
    out
}

pub(crate) fn font_is_mono(family: &str) -> bool {
    let family = family.to_ascii_lowercase();
    family.contains("courier") || family.contains("mono")
}
