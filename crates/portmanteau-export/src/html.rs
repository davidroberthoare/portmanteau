//! Single-file HTML: one scrolling page whose text column is the compile's
//! page width, styled by the shared web stylesheet.

use super::*;

pub(crate) fn html_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut classes = ParagraphClasses::default();
    let sections = manuscript_sections(project, settings);
    let mut body = String::new();

    if settings.include_title_page {
        body.push_str("<header>\n");
        body.push_str(&html_title_page(project));
        body.push_str("</header>\n");
    }
    if settings.include_toc {
        body.push_str("<nav class=\"toc\"><h2 class=\"heading\">Contents</h2><ol>\n");
        for (index, section) in sections.iter().enumerate() {
            let title = section_title(project, section.doc, index, settings);
            body.push_str(&format!(
                "<li><a href=\"#s{index}\">{}</a></li>\n",
                html_escape(&title)
            ));
        }
        body.push_str("</ol></nav>\n");
    }
    for doc in front_matter(project, settings) {
        body.push_str(&matter_section("frontmatter", doc, project, settings, &mut classes));
    }
    for (index, section) in sections.iter().enumerate() {
        let title = section_title(project, section.doc, index, settings);
        body.push_str(&format!(
            "<section id=\"s{index}\" class=\"chapter\"><h2 class=\"heading\">{}</h2>\n",
            html_escape(&title)
        ));
        body.push_str(&html_document_body(project, section.doc, settings, &mut classes));
        body.push_str("</section>\n");
    }
    for doc in back_matter(project, settings) {
        body.push_str(&matter_section("backmatter", doc, project, settings, &mut classes));
    }

    let css = stylesheet(project, settings, WebTarget::Html, &classes);
    // Backstop: nothing in a stylesheet may end the <style> element.
    let css = css.replace("</", "<\\/");
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\"/>\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/>\n\
         <title>{}</title>\n<style>\n{css}</style>\n</head>\n<body>\n<main class=\"page\">\n{body}</main>\n</body>\n</html>\n",
        html_escape(&project.title),
    )
}

fn matter_section(
    class: &str,
    doc: &Document,
    project: &Project,
    settings: &ExportSettings,
    classes: &mut ParagraphClasses,
) -> String {
    format!(
        "<section class=\"{class}\"><h2 class=\"heading\">{}</h2>\n{}</section>\n",
        html_escape(&doc.title),
        html_document_body(project, doc, settings, classes)
    )
}
