use super::*;

pub(crate) fn fountain_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    if settings.include_title_page {
        out.push_str(&script::fountain_title_page(project));
    }
    for section in manuscript_sections(project, settings) {
        if section.doc.medium.is_script() {
            out.push_str(&format!("# {}\n\n", section.heading));
            let blocks = screenplay_blocks(project, section.doc);
            out.push_str(&script::to_fountain(&blocks, section.doc.medium));
            out.push('\n');
        } else {
            out.push_str(&format!("# {}\n\n", section.heading));
            for para in paragraphs(&section.doc.body_text()) {
                out.push_str(&para);
                out.push_str("\n\n");
            }
        }
    }
    out
}
