use super::*;

pub(crate) fn docx_align(align: Alignment) -> Option<&'static str> {
    match align {
        Alignment::Left => None,
        Alignment::Center => Some("center"),
        Alignment::Right => Some("right"),
        Alignment::Justify => Some("both"),
    }
}

pub(crate) fn docx_text_run(text: &str, properties: &str) -> String {
    format!(
        "<w:r><w:rPr>{properties}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r>",
        xml_escape(text)
    )
}

pub(crate) fn docx_run_properties(
    paragraph: &ProseParagraph,
    marks: &[InlineMark],
    fallback_font: &str,
) -> String {
    let mut properties = String::new();
    let font = if paragraph.format.font.trim().is_empty() {
        fallback_font
    } else {
        &paragraph.format.font
    };
    if !font.trim().is_empty() {
        let font = xml_escape(font);
        properties.push_str(&format!(
            "<w:rFonts w:ascii=\"{font}\" w:hAnsi=\"{font}\"/>"
        ));
    }
    let half_points = (paragraph.format.size.max(8.0) * 2.0).round() as i32;
    properties.push_str(&format!("<w:sz w:val=\"{half_points}\"/>"));
    if paragraph.heading || marks.contains(&InlineMark::Bold) {
        properties.push_str("<w:b/>");
    }
    if marks.contains(&InlineMark::Italic) {
        properties.push_str("<w:i/>");
    }
    if marks.contains(&InlineMark::Underline) {
        properties.push_str("<w:u w:val=\"single\"/>");
    }
    if marks.contains(&InlineMark::Strikethrough) {
        properties.push_str("<w:strike/>");
    }
    properties
}

pub(crate) fn docx_prose_runs(paragraph: &ProseParagraph, fallback_font: &str) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                let properties = docx_run_properties(paragraph, &run.marks, fallback_font);
                out.push_str(&docx_text_run(&paragraph.transform(&run.text), &properties));
            }
            InlineContent::HardBreak => out.push_str("<w:r><w:br/></w:r>"),
            InlineContent::Image(image) => {
                let label = if image.alt_text.is_empty() {
                    "[Image]".to_owned()
                } else {
                    format!("[Image: {}]", paragraph.transform(&image.alt_text))
                };
                let properties = docx_run_properties(paragraph, &[], fallback_font);
                out.push_str(&docx_text_run(&label, &properties));
            }
        }
    }
    out
}

pub(crate) fn docx_prose_paragraph(paragraph: &ProseParagraph, fallback_font: &str) -> String {
    let style = &paragraph.format;
    let mut properties = String::new();
    properties.push_str(if paragraph.heading {
        "<w:pStyle w:val=\"Heading2\"/>"
    } else {
        "<w:pStyle w:val=\"Normal\"/>"
    });
    let before = (style.before.max(0.0) * 20.0).round() as i32;
    let after = (style.after.max(0.0) * 20.0).round() as i32;
    let line = (style.leading.max(1.0) * 240.0).round() as i32;
    properties.push_str(&format!(
        "<w:spacing w:before=\"{before}\" w:after=\"{after}\" w:line=\"{line}\" w:lineRule=\"auto\"/>"
    ));
    let left = (style.indent.max(0.0) * 20.0).round() as i32;
    let right = (style.right_indent.max(0.0) * 20.0).round() as i32;
    let first = (style.first_line_indent * 20.0).round() as i32;
    if first >= 0 {
        properties.push_str(&format!(
            "<w:ind w:left=\"{left}\" w:right=\"{right}\" w:firstLine=\"{first}\"/>"
        ));
    } else {
        properties.push_str(&format!(
            "<w:ind w:left=\"{left}\" w:right=\"{right}\" w:hanging=\"{}\"/>",
            -first
        ));
    }
    if let Some(align) = docx_align(style.align) {
        properties.push_str(&format!("<w:jc w:val=\"{align}\"/>"));
    }
    format!(
        "<w:p><w:pPr>{properties}</w:pPr>{}</w:p>",
        docx_prose_runs(paragraph, fallback_font)
    )
}

pub(crate) fn docx_basic_paragraph(
    body: &mut String,
    text: &str,
    style: &str,
    bold: bool,
    center: bool,
    mono: bool,
    indent: i32,
    prose_font: &str,
) {
    let jc = if center { "<w:jc w:val=\"center\"/>" } else { "" };
    let fonts = if mono {
        "<w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/>".to_string()
    } else {
        format!(
            "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\"/>",
            prose_font, prose_font
        )
    };
    // Element order inside w:pPr matters to Word: pStyle, ind, jc.
    body.push_str(&format!(
        "<w:p><w:pPr><w:pStyle w:val=\"{}\"/><w:ind w:left=\"{}\"/>{}</w:pPr><w:r><w:rPr>{}{}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        style,
        indent,
        jc,
        fonts,
        if bold { "<w:b/>" } else { "" },
        xml_escape(text)
    ));
}

pub(crate) fn docx_manuscript(project: &Project, settings: &ExportSettings) -> Vec<u8> {
    let mut body = String::new();
    let prose_font = xml_escape(&settings.font);

    if settings.include_title_page {
        docx_basic_paragraph(&mut body, &project.title, "Title", true, true, false, 0, &prose_font);
        if !project.subtitle.is_empty() {
            docx_basic_paragraph(
                &mut body,
                &project.subtitle,
                "Subtitle",
                false,
                true,
                false,
                0,
                &prose_font,
            );
        }
        docx_basic_paragraph(
            &mut body,
            &format!("by {}", project.author),
            "Normal",
            false,
            true,
            false,
            0,
            &prose_font,
        );
        docx_basic_paragraph(&mut body, "", "Normal", false, false, false, 0, &prose_font);
    }
    for doc in front_matter(project, settings) {
        docx_basic_paragraph(&mut body, &doc.title, "Heading1", true, false, false, 0, &prose_font);
        for para in prose_paragraphs(project, doc, settings) {
            body.push_str(&docx_prose_paragraph(&para, &settings.font));
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        let title = section_title(project, section.doc, i, settings);
        docx_basic_paragraph(&mut body, &title, "Heading1", true, false, false, 0, &prose_font);
        if section.doc.medium.is_script() {
            let blocks = screenplay_blocks(project, section.doc);
            for block in &blocks {
                let (indent, mono, bold) = match block.kind {
                    ElementKind::Character => (4320, true, false),
                    ElementKind::Parenthetical => (3240, true, false),
                    ElementKind::Dialogue => (2160, true, false),
                    ElementKind::SceneHeading => (0, true, false),
                    _ => (0, true, false),
                };
                let text = if settings.styles.style_for_kind(block.kind).uppercase {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                docx_basic_paragraph(
                    &mut body,
                    &text,
                    if block.kind == ElementKind::SceneHeading { "Heading2" } else { "Normal" },
                    bold,
                    block.kind == ElementKind::Transition,
                    mono,
                    indent,
                    &prose_font,
                );
            }
        } else {
            for para in prose_paragraphs(project, section.doc, settings) {
                body.push_str(&docx_prose_paragraph(&para, &settings.font));
            }
        }
    }
    for doc in back_matter(project, settings) {
        docx_basic_paragraph(&mut body, &doc.title, "Heading1", true, false, false, 0, &prose_font);
        for para in prose_paragraphs(project, doc, settings) {
            body.push_str(&docx_prose_paragraph(&para, &settings.font));
        }
    }

    let (page_w, page_h) = paper_twips(settings);
    let margins = page_margins(project, settings);
    let (mt, mr, mb, ml) = (
        pt_to_twips(margins.top),
        pt_to_twips(margins.right),
        pt_to_twips(margins.bottom),
        pt_to_twips(margins.left),
    );
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{}\n<w:sectPr><w:pgSz w:w=\"{page_w}\" w:h=\"{page_h}\"/><w:pgMar w:top=\"{mt}\" w:right=\"{mr}\" w:bottom=\"{mb}\" w:left=\"{ml}\"/></w:sectPr></w:body></w:document>",
        body
    );
    let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
 <w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman"/><w:sz w:val="24"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120" w:line="360" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
 <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
 <w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="2400" w:after="240"/></w:pPr><w:rPr><w:sz w:val="52"/></w:rPr></w:style>
 <w:style w:type="paragraph" w:styleId="Subtitle"><w:name w:val="Subtitle"/><w:basedOn w:val="Normal"/><w:pPr><w:jc w:val="center"/></w:pPr><w:rPr><w:sz w:val="26"/><w:i/></w:rPr></w:style>
 <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:pPr><w:pageBreakBefore/><w:spacing w:before="720" w:after="360"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="36"/></w:rPr></w:style>
 <w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="240" w:after="120"/></w:pPr><w:rPr><w:b/></w:rPr></w:style>
</w:styles>"#.replace("Times New Roman", &prose_font);
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#;
    let doc_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;
    let core = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{}</dc:title><dc:creator>{}</dc:creator></cp:coreProperties>",
        xml_escape(&project.title),
        xml_escape(&project.author)
    );

    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, data) in [
            ("[Content_Types].xml", content_types),
            ("_rels/.rels", rels),
            ("word/document.xml", &document),
            ("word/styles.xml", styles.as_str()),
            ("word/_rels/document.xml.rels", doc_rels),
            ("docProps/core.xml", &core),
        ] {
            zip.start_file(name, opts).unwrap();
            zip.write_all(data.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    buf
}
