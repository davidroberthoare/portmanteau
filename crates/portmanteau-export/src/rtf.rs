use super::*;

pub(crate) fn rtf_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\n' => out.push_str("\\line "),
            c if c.is_ascii() => out.push(c),
            c => out.push_str(&format!("\\u{}?", c as i32)),
        }
    }
    out
}

pub(crate) fn rtf_prose_runs(paragraph: &ProseParagraph) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                let mut prefix = String::new();
                let mut suffix = String::new();
                if run.marks.contains(&InlineMark::Bold) {
                    prefix.push_str("\\b ");
                    suffix.insert_str(0, "\\b0 ");
                }
                if run.marks.contains(&InlineMark::Italic) {
                    prefix.push_str("\\i ");
                    suffix.insert_str(0, "\\i0 ");
                }
                if run.marks.contains(&InlineMark::Underline) {
                    prefix.push_str("\\ul ");
                    suffix.insert_str(0, "\\ulnone ");
                }
                if run.marks.contains(&InlineMark::Strikethrough) {
                    prefix.push_str("\\strike ");
                    suffix.insert_str(0, "\\strike0 ");
                }
                out.push_str(&prefix);
                out.push_str(&rtf_escape(&paragraph.transform(&run.text)));
                out.push_str(&suffix);
            }
            InlineContent::HardBreak => out.push_str("\\line "),
            InlineContent::Image(image) if image.alt_text.is_empty() => out.push_str("[Image]"),
            InlineContent::Image(image) => out.push_str(&rtf_escape(&format!(
                "[Image: {}]",
                paragraph.transform(&image.alt_text)
            ))),
        }
    }
    out
}

pub(crate) fn rtf_prose_paragraph(paragraph: &ProseParagraph) -> String {
    let style = &paragraph.format;
    let mut out = String::from("\\pard");
    match style.align {
        Alignment::Center => out.push_str("\\qc"),
        Alignment::Right => out.push_str("\\qr"),
        Alignment::Justify => out.push_str("\\qj"),
        Alignment::Left => out.push_str("\\ql"),
    }
    let left = (style.indent.max(0.0) * 20.0).round() as i32;
    let right = (style.right_indent.max(0.0) * 20.0).round() as i32;
    let first = (style.first_line_indent * 20.0).round() as i32;
    if left > 0 {
        out.push_str(&format!("\\li{left}"));
    }
    if right > 0 {
        out.push_str(&format!("\\ri{right}"));
    }
    if first != 0 {
        out.push_str(&format!("\\fi{first}"));
    }
    let before = (style.before.max(0.0) * 20.0).round() as i32;
    let after = (style.after.max(0.0) * 20.0).round() as i32;
    if before > 0 {
        out.push_str(&format!("\\sb{before}"));
    }
    if after > 0 {
        out.push_str(&format!("\\sa{after}"));
    }
    let half_points = (style.size.max(8.0) * 2.0).round() as i32;
    out.push_str(&format!(
        "\\f{}\\fs{} ",
        if font_is_mono(&style.font) { 1 } else { 0 },
        half_points
    ));
    if paragraph.heading {
        out.push_str("\\b ");
        out.push_str(&rtf_prose_runs(paragraph));
        out.push_str("\\b0 ");
    } else {
        out.push_str(&rtf_prose_runs(paragraph));
    }
    out.push_str("\\par\n");
    out
}

pub(crate) fn rtf_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{{\\rtf1\\ansi\\deff0{{\\fonttbl{{\\f0 {};}}{{\\f1 Courier New;}}}}\n",
        rtf_escape(&settings.font)
    ));
    let (page_w, page_h) = paper_twips(settings);
    out.push_str(&format!(
        "\\paperw{page_w}\\paperh{page_h}\\margl1440\\margr1440\\margt1440\\margb1440\n"
    ));
    if settings.include_title_page {
        out.push_str("\\qc\\f0\\fs52 ");
        out.push_str(&rtf_escape(&project.title));
        out.push_str("\\par\\fs24\\par ");
        out.push_str(&rtf_escape(&project.author));
        out.push_str("\\par\\page\n");
    }
    for doc in front_matter(project, settings) {
        out.push_str(&format!(
            "\\f0\\fs32\\b {}\\b0\\fs24\\par\n",
            rtf_escape(&doc.title)
        ));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&rtf_prose_paragraph(&para));
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        out.push_str(&format!(
            "\\page\\f0\\fs36\\b {}\\b0\\fs24\\par\n",
            rtf_escape(&title)
        ));
        if section.doc.medium.is_script() {
            let blocks = screenplay_blocks(project, section.doc);
            for block in &blocks {
                let indent = match block.kind {
                    ElementKind::Character => 4400,
                    ElementKind::Parenthetical => 3200,
                    ElementKind::Dialogue => 2000,
                    _ => 0,
                };
                let text = if settings.styles.style_for_kind(block.kind).uppercase {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                out.push_str(&format!(
                    "\\f1\\fs24\\li{}\\ri0 {}\\par\n",
                    indent,
                    rtf_escape(&text)
                ));
            }
        } else {
            for para in prose_paragraphs(project, section.doc, settings) {
                out.push_str(&rtf_prose_paragraph(&para));
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&format!(
            "\\page\\f0\\fs32\\b {}\\b0\\fs24\\par\n",
            rtf_escape(&doc.title)
        ));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&rtf_prose_paragraph(&para));
        }
    }
    out.push_str("}\n");
    out
}
