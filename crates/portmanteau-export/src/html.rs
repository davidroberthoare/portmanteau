use super::*;

pub(crate) fn html_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut body = String::new();
    if settings.include_title_page {
        body.push_str(&format!(
            "<header class=\"titlepage\"><h1>{}</h1><p class=\"sub\">{}</p><p>by {}</p></header>\n",
            html_escape(&project.title),
            html_escape(&project.subtitle),
            html_escape(&project.author)
        ));
    }
    if settings.include_toc {
        body.push_str("<nav class=\"toc\"><h2>Contents</h2><ol>\n");
        for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
            let title = section_title(project, section.doc, i, settings);
            body.push_str(&format!(
                "<li><a href=\"#s{}\">{}</a></li>\n",
                i,
                html_escape(&title)
            ));
        }
        body.push_str("</ol></nav>\n");
    }
    for doc in front_matter(project, settings) {
        body.push_str(&format!(
            "<section class=\"frontmatter\"><h2>{}</h2>\n",
            html_escape(&doc.title)
        ));
        for para in prose_paragraphs(project, doc, settings).iter() {
            body.push_str(&html_prose_paragraph(para));
        }
        body.push_str("</section>\n");
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        body.push_str(&format!(
            "<section id=\"s{}\" class=\"chapter\"><h2>{}</h2>\n",
            i,
            html_escape(&title)
        ));
        if section.doc.medium.is_script() {
            body.push_str("<div class=\"script\">\n");
            let blocks = screenplay_blocks(project, section.doc);
            for block in &blocks {
                let class = match block.kind {
                    ElementKind::SceneHeading => "scene",
                    ElementKind::Character => "character",
                    ElementKind::Parenthetical => "paren",
                    ElementKind::Dialogue => "dialogue",
                    ElementKind::Transition => "trans",
                    _ => "action",
                };
                let text = if settings.styles.style_for_kind(block.kind).uppercase {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                body.push_str(&format!(
                    "<p class=\"{}\">{}</p>\n",
                    class,
                    html_escape(&text)
                ));
            }
            body.push_str("</div>\n");
        } else {
            for para in prose_paragraphs(project, section.doc, settings).iter() {
                body.push_str(&html_prose_paragraph(para));
            }
        }
        body.push_str("</section>\n");
    }
    for doc in back_matter(project, settings) {
        body.push_str(&format!(
            "<section class=\"backmatter\"><h2>{}</h2>\n",
            html_escape(&doc.title)
        ));
        for para in prose_paragraphs(project, doc, settings).iter() {
            body.push_str(&html_prose_paragraph(para));
        }
        body.push_str("</section>\n");
    }
    let st = &settings.styles;
    let body_font = element_css_font(&st.body, "Liberation Serif");
    let script_font = element_css_font(&st.action, "Courier New");
    let chapter_size = st.chapter_heading.size.max(8.0);
    let body_align = css_align(st.body.align);
    let heading_align = css_align(st.chapter_heading.align);
    let body_indent_margins = css_indent_margins(&st.body);
    let body_first_indent = st.body.first_line_indent.max(0.0);
    let title_align = css_align(st.title_page_title.align);
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\"/>\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/>\n<title>{title}</title>\n<style>\nbody{{max-width:42rem;margin:4rem auto;padding:0 1.5rem;font-family:{body_font};line-height:{body_leading:.2};text-align:{body_align};color:#1e1c19;background:#fbf8f1}}\nh1{{font-size:{h1_size:.1}pt;line-height:1.1;text-align:{heading_align}}}h2{{font-size:{chapter_size:.1}pt;margin-top:3rem;border-bottom:1px solid #d8d0c0;padding-bottom:.4rem;text-align:{heading_align}}}\np{{margin:0 0 .9rem;{body_indent_margins}text-indent:{body_first_indent:.1}pt;text-align:{body_align}}}p.first{{text-indent:0}}\n.titlepage{{text-align:center;margin:18vh 0 22vh}}.titlepage h1{{font-size:{title_size:.1}pt;margin-bottom:.3em;text-align:{title_align}}}.titlepage p{{text-indent:0}}.sub{{font-style:italic;color:#665f54}}\n.toc ol{{line-height:2}}.toc a{{color:#2f5d50;text-decoration:none}}\n.script{{font-family:{script_font};font-size:{script_size:.1}pt}}.script p{{text-indent:0;margin:.4rem 0;text-align:left}}\n.scene{{font-weight:bold;margin-top:1.6rem!important}}.character{{margin-left:32%!important;margin-top:1rem!important}}.paren{{margin-left:24%!important}}.dialogue{{margin-left:16%!important;margin-right:12%!important}}.trans{{text-align:right!important}}\n.frontmatter,.backmatter{{margin-top:4rem}}\n</style>\n</head>\n<body>\n{body}\n</body>\n</html>\n",
        title = html_escape(&project.title),
        body_font = body_font,
        body_leading = st.body.leading.max(1.0),
        body_align = body_align,
        body_indent_margins = body_indent_margins,
        body_first_indent = body_first_indent,
        heading_align = heading_align,
        title_align = title_align,
        h1_size = chapter_size * 1.5,
        chapter_size = chapter_size,
        title_size = st.title_page_title.size.max(8.0),
        script_font = script_font,
        script_size = st.action.size.max(8.0),
        body = body,
    )
}
