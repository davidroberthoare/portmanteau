use super::*;

pub(crate) fn txt_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    if settings.include_title_page {
        out.push_str(&format!("{}\n", project.title.to_uppercase()));
        if !project.subtitle.is_empty() {
            out.push_str(&format!("{}\n", project.subtitle));
        }
        out.push_str(&format!("\nby {}\n\n", project.author));
        out.push_str(&"-".repeat(40));
        out.push_str("\n\n");
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
