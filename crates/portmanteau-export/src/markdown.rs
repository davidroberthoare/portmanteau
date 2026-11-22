use super::*;

pub(crate) fn markdown_runs(paragraph: &ProseParagraph) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                let mut text = paragraph.transform(&run.text);
                if text.is_empty() {
                    continue;
                }
                if run.marks.contains(&InlineMark::Italic) {
                    text = format!("*{text}*");
                }
                if run.marks.contains(&InlineMark::Bold) {
                    text = format!("**{text}**");
                }
                if run.marks.contains(&InlineMark::Underline) {
                    text = format!("<u>{text}</u>");
                }
                if run.marks.contains(&InlineMark::Strikethrough) {
                    text = format!("~~{text}~~");
                }
                if let Some(href) = run.link.as_deref().and_then(export_href) {
                    text = format!("[{text}]({})", href.replace(')', "%29").replace(' ', "%20"));
                }
                out.push_str(&text);
            }
            InlineContent::HardBreak => out.push_str("<br>"),
            InlineContent::Image(image) if image.alt_text.is_empty() => out.push_str("[Image]"),
            InlineContent::Image(image) => out.push_str(&format!(
                "[Image: {}]",
                paragraph.transform(&image.alt_text)
            )),
        }
    }
    out
}

pub(crate) fn markdown_prose_paragraph(paragraph: &ProseParagraph) -> String {
    if paragraph.heading {
        format!("### {}", markdown_runs(paragraph))
    } else {
        markdown_runs(paragraph)
    }
}

pub(crate) fn markdown_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    if settings.include_title_page {
        out.push_str(&format!("# {}\n\n", project.title));
        if !project.subtitle.is_empty() {
            out.push_str(&format!("*{}*\n\n", project.subtitle));
        }
        out.push_str(&format!("by {}\n\n---\n\n", project.author));
    }
    if settings.include_toc {
        out.push_str("## Contents\n\n");
        for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
            let title = section_title(project, section.doc, i, settings);
            out.push_str(&format!("{}. {}\n", i + 1, title));
        }
        out.push('\n');
    }
    for doc in front_matter(project, settings) {
        out.push_str(&format!("## {}\n\n", doc.title));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&markdown_prose_paragraph(&para));
            out.push_str("\n\n");
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        out.push_str(&format!("## {}\n\n", title));
        if section.doc.medium.is_script() {
            out.push_str("```fountain\n");
            let blocks = screenplay_blocks(project, section.doc);
            out.push_str(&script::to_fountain(&blocks, section.doc.medium));
            out.push_str("```\n\n");
        } else {
            for para in prose_paragraphs(project, section.doc, settings) {
                out.push_str(&markdown_prose_paragraph(&para));
                out.push_str("\n\n");
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&format!("## {}\n\n", doc.title));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&markdown_prose_paragraph(&para));
            out.push_str("\n\n");
        }
    }
    out
}
