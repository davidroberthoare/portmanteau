use super::*;

pub(crate) fn txt_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    if settings.include_title_page && project.compile_plan(settings).kind.is_script() {
        // The script cover as plain text: centred on a 60-column page, with
        // a blank line wherever the cover has a gap, then a rule.
        for line in script_cover_lines(project, settings) {
            if line.style.before > 0.0 {
                out.push('\n');
            }
            let room = 60usize.saturating_sub(line.text.chars().count());
            let pad = match line.style.align {
                Alignment::Center => room / 2,
                Alignment::Right => room,
                _ => 0,
            };
            out.push_str(&format!("{}{}\n", " ".repeat(pad), line.text));
        }
        out.push_str("\n\n");
    } else if settings.include_title_page {
        out.push_str(&format!("{}\n", project.title.to_uppercase()));
        if !project.subtitle.is_empty() {
            out.push_str(&format!("{}\n", project.subtitle));
        }
        out.push_str(&format!("\nby {}\n\n", project.author));
        out.push_str(&"-".repeat(40));
        out.push_str("\n\n");
    }
    if settings.include_toc {
        out.push_str("CONTENTS\n\n");
        for (index, title) in contents_entries(project, settings).iter().enumerate() {
            out.push_str(&format!("{}. {}\n", index + 1, title));
        }
        out.push('\n');
    }
    for doc in front_matter(project, settings) {
        out.push_str(&format!("{}\n\n", doc.title.to_uppercase()));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&para.plain_text());
            out.push_str("\n\n");
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        out.push_str(&format!("\n{}\n{}\n\n", title.to_uppercase(), "-".repeat(title.len())));
        if section.doc.medium.is_script() {
            let blocks = screenplay_blocks(project, section.doc);
            for block in &blocks {
                out.push_str(&script::block_to_plain(block));
                out.push_str("\n\n");
            }
        } else {
            for para in prose_paragraphs(project, section.doc, settings) {
                out.push_str(&para.plain_text());
                out.push_str("\n\n");
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&format!("\n{}\n\n", doc.title.to_uppercase()));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&para.plain_text());
            out.push_str("\n\n");
        }
    }
    out
}
