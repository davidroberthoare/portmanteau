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
                let mut properties = docx_run_properties(paragraph, &run.marks, fallback_font);
                let href = run.link.as_deref().and_then(export_href);
                if href.is_some() {
                    // Hyperlink blue, underlined (unless the text already is).
                    properties.push_str("<w:color w:val=\"0563C1\"/>");
                    if !run.marks.contains(&InlineMark::Underline) {
                        properties.push_str("<w:u w:val=\"single\"/>");
                    }
                }
                let text_run = docx_text_run(&paragraph.transform(&run.text), &properties);
                match href {
                    // A field, not a relationship: no extra package part to keep in step.
                    Some(href) => out.push_str(&format!(
                        "<w:fldSimple w:instr=\" HYPERLINK &quot;{}&quot; \">{text_run}</w:fldSimple>",
                        xml_escape(&href)
                    )),
                    None => out.push_str(&text_run),
                }
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

/// A paragraph that only names its style. Appearance lives in `styles.xml`,
/// so a writer's style change reaches every paragraph that uses it.
pub(crate) fn docx_styled_paragraph(body: &mut String, text: &str, style_id: &str) {
    docx_paragraph(body, text, style_id, false);
}

/// A styled paragraph, optionally forced to the top of a new page.
pub(crate) fn docx_paragraph(body: &mut String, text: &str, style_id: &str, new_page: bool) {
    let break_before = if new_page { "<w:pageBreakBefore/>" } else { "" };
    body.push_str(&format!(
        "<w:p><w:pPr><w:pStyle w:val=\"{style_id}\"/>{break_before}</w:pPr><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        xml_escape(text)
    ));
}

/// Style id for a screenplay element, e.g. `ScriptSceneHeading`.
pub(crate) fn docx_script_style_id(kind: ElementKind) -> String {
    format!("Script{kind:?}")
}

/// One `<w:style>` generated from an `ElementStyle`. Child order follows the
/// schema Word enforces: pPr = pageBreakBefore, spacing, ind, jc, outlineLvl;
/// rPr = rFonts, b, i, sz.
pub(crate) fn docx_style(
    id: &str,
    name: &str,
    style: &ElementStyle,
    fallback_font: &str,
    bold: bool,
    page_break_before: bool,
    outline_level: Option<u8>,
) -> String {
    let font = xml_escape(if style.font.trim().is_empty() {
        fallback_font
    } else {
        &style.font
    });
    let first_line = if style.first_line_indent >= 0.0 {
        format!("w:firstLine=\"{}\"", pt_to_twips(style.first_line_indent))
    } else {
        format!("w:hanging=\"{}\"", pt_to_twips(-style.first_line_indent))
    };
    format!(
        "<w:style w:type=\"paragraph\" w:styleId=\"{id}\"><w:name w:val=\"{name}\"/>{based}<w:qFormat/><w:pPr>{page_break}\
         <w:spacing w:before=\"{before}\" w:after=\"{after}\" w:line=\"{line}\" w:lineRule=\"auto\"/>\
         <w:ind w:left=\"{left}\" w:right=\"{right}\" {first_line}/>{jc}{outline}</w:pPr>\
         <w:rPr><w:rFonts w:ascii=\"{font}\" w:hAnsi=\"{font}\"/>{b}{i}<w:sz w:val=\"{size}\"/></w:rPr></w:style>\n",
        based = if id == "Normal" { "" } else { "<w:basedOn w:val=\"Normal\"/>" },
        page_break = if page_break_before { "<w:pageBreakBefore/>" } else { "" },
        before = pt_to_twips(style.before.max(0.0)),
        after = pt_to_twips(style.after.max(0.0)),
        line = (style.leading.max(1.0) * 240.0).round() as i64,
        left = pt_to_twips(style.indent.max(0.0)),
        right = pt_to_twips(style.right_indent.max(0.0)),
        jc = docx_align(style.align)
            .map(|value| format!("<w:jc w:val=\"{value}\"/>"))
            .unwrap_or_default(),
        outline = outline_level
            .map(|level| format!("<w:outlineLvl w:val=\"{level}\"/>"))
            .unwrap_or_default(),
        b = if bold { "<w:b/>" } else { "" },
        i = if style.italic { "<w:i/>" } else { "" },
        size = (style.size.max(8.0) * 2.0).round() as i64,
    )
}

/// The whole of `word/styles.xml`, generated from the compile's element styles.
pub(crate) fn docx_styles(project: &Project, settings: &ExportSettings) -> String {
    let st = &settings.styles;
    let serif = settings.font.as_str();
    let mono = "Courier New";
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\n",
    );
    let body_font = xml_escape(if st.body.font.trim().is_empty() { serif } else { &st.body.font });
    out.push_str(&format!(
        "<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"{body_font}\" w:hAnsi=\"{body_font}\"/><w:sz w:val=\"{}\"/></w:rPr></w:rPrDefault></w:docDefaults>\n",
        (st.body.size.max(8.0) * 2.0).round() as i64
    ));
    out.push_str("<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>\n");
    out.push_str(&docx_style("Body", "Body", &st.body, serif, false, false, None));
    // The title sits where the PDF puts it: dropped from the top margin.
    let title = ElementStyle {
        before: st.title_page_title.before + title_drop_pt(project, settings),
        ..st.title_page_title.clone()
    };
    out.push_str(&docx_style("Title", "Title", &title, serif, true, false, None));
    out.push_str(&docx_style("Subtitle", "Subtitle", &st.title_page_subtitle, serif, false, false, None));
    out.push_str(&docx_style("Byline", "Byline", &st.title_page_author, serif, false, false, None));
    out.push_str(&docx_style("Heading1", "heading 1", &st.chapter_heading, serif, true, true, Some(0)));
    out.push_str(&docx_style("Heading2", "heading 2", &st.chapter_heading, serif, true, false, Some(1)));
    // The Contents list: a heading that starts a page but stays out of the
    // outline, and plain entries without the body's first-line indent.
    out.push_str(&docx_style("ContentsHeading", "Contents Heading", &st.chapter_heading, serif, true, true, None));
    let entry = ElementStyle {
        first_line_indent: 0.0,
        ..st.body.clone()
    };
    out.push_str(&docx_style("ContentsEntry", "Contents Entry", &entry, serif, false, false, None));
    for kind in ALL_KINDS {
        out.push_str(&docx_style(
            &docx_script_style_id(kind),
            &format!("Script {}", kind.label()),
            st.style_for_kind(kind),
            mono,
            false,
            false,
            None,
        ));
    }
    // The opening FADE IN: is the transition set flush left.
    out.push_str(&docx_style(
        DOCX_OPENING_STYLE,
        "Script Opening Transition",
        &opening_transition_style(st.style_for_kind(ElementKind::Transition)),
        mono,
        false,
        false,
        None,
    ));
    if project.compile_plan(settings).kind.is_script() {
        // The cover's three roles; each line's own space above is applied on
        // the paragraph, so the style itself carries none.
        for line in script_cover_lines(project, settings) {
            let id = docx_cover_style_id(line.role);
            if out.contains(&format!("w:styleId=\"{id}\"")) {
                continue;
            }
            let style = ElementStyle { before: 0.0, ..line.style.clone() };
            out.push_str(&docx_style(&id, &id, &style, mono, false, false, None));
        }
    }
    out.push_str("</w:styles>");
    out
}

const DOCX_OPENING_STYLE: &str = "ScriptOpeningTransition";

/// The style id for a script block: its kind's, or the opening transition's.
pub(crate) fn docx_block_style_id(kind: ElementKind, text: &str) -> String {
    if opens_script(kind, text) {
        DOCX_OPENING_STYLE.to_string()
    } else {
        docx_script_style_id(kind)
    }
}

/// Style id for a line of a script's cover page.
pub(crate) fn docx_cover_style_id(role: TitleRole) -> String {
    format!("ScriptCover{role:?}")
}

/// One header or footer part for Word: three slots (left, centre, right) set on
/// tab stops, with `{fields}` written as Word fields so Word keeps them live.
fn docx_furniture_part(tag: &str, slots: &[String; 3], text_width: i64, size_pt: f32, font: &str) -> String {
    // The part's root element is `hdr` or `ftr`.
    let tag = if tag == "header" { "hdr" } else { "ftr" };
    let mut runs = String::new();
    for (i, slot) in slots.iter().enumerate() {
        if i > 0 {
            runs.push_str("<w:r><w:tab/></w:r>");
        }
        runs.push_str(&docx_field_runs(slot, size_pt, font));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:{tag} xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:p><w:pPr><w:tabs><w:tab w:val=\"center\" w:pos=\"{}\"/><w:tab w:val=\"right\" w:pos=\"{}\"/></w:tabs><w:spacing w:before=\"0\" w:after=\"0\"/></w:pPr>{runs}</w:p></w:{tag}>",
        text_width / 2,
        text_width
    )
}

fn docx_field_runs(text: &str, size_pt: f32, font: &str) -> String {
    let inner = format!(
        "<w:rFonts w:ascii=\"{f}\" w:hAnsi=\"{f}\"/><w:sz w:val=\"{}\"/>",
        (size_pt * 2.0).round() as i32,
        f = xml_escape(font)
    );
    let props = format!("<w:rPr>{inner}</w:rPr>");
    let field = |instr: &str, shown: &str| {
        format!(
            "<w:r>{props}<w:fldChar w:fldCharType=\"begin\"/></w:r><w:r>{props}<w:instrText xml:space=\"preserve\"> {instr} </w:instrText></w:r><w:r>{props}<w:fldChar w:fldCharType=\"separate\"/></w:r><w:r>{props}<w:t>{shown}</w:t></w:r><w:r>{props}<w:fldChar w:fldCharType=\"end\"/></w:r>"
        )
    };
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let Some(open) = rest.find('{') else { break };
        let Some(close) = rest[open..].find('}') else { break };
        let name = &rest[open..open + close + 1];
        let known = ["{page}", "{pages}", "{title}", "{author}", "{chapter}", "{chapter_number}"];
        if !known.contains(&name) {
            // Not a field: keep the brace as text and carry on after it.
            out.push_str(&docx_text_run(&rest[..open + 1], &inner));
            rest = &rest[open + 1..];
            continue;
        }
        if open > 0 {
            out.push_str(&docx_text_run(&rest[..open], &inner));
        }
        match name {
            "{page}" => out.push_str(&field("PAGE", "1")),
            "{pages}" => out.push_str(&field("NUMPAGES", "1")),
            "{chapter}" => out.push_str(&field("STYLEREF \"heading 1\"", "")),
            "{chapter_number}" => out.push_str(&field("STYLEREF \"heading 1\" \\n", "")),
            // Title and author are known now; they go in as plain text below.
            other => out.push_str(&docx_text_run(other, &inner)),
        }
        rest = &rest[open + close + 1..];
    }
    if !rest.is_empty() {
        out.push_str(&docx_text_run(rest, &inner));
    }
    out
}

pub(crate) fn docx_manuscript(project: &Project, settings: &ExportSettings) -> Vec<u8> {
    let mut body = String::new();
    let mut scenes = SceneNumbers::default();

    let is_script = project.compile_plan(settings).kind.is_script();
    if settings.include_title_page && is_script {
        for line in script_cover_lines(project, settings) {
            body.push_str(&format!(
                "<w:p><w:pPr><w:pStyle w:val=\"{}\"/><w:spacing w:before=\"{}\"/></w:pPr><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                docx_cover_style_id(line.role),
                pt_to_twips(line.style.before),
                xml_escape(&line.text)
            ));
        }
    } else if settings.include_title_page {
        docx_styled_paragraph(&mut body, &project.title, "Title");
        if !project.subtitle.trim().is_empty() {
            docx_styled_paragraph(&mut body, &project.subtitle, "Subtitle");
        }
        if !project.author.trim().is_empty() {
            docx_styled_paragraph(&mut body, &format!("by {}", project.author), "Byline");
        }
    }
    if settings.include_toc {
        docx_styled_paragraph(&mut body, "Contents", "ContentsHeading");
        for title in contents_entries(project, settings) {
            docx_styled_paragraph(&mut body, &title, "ContentsEntry");
        }
    }
    for doc in front_matter(project, settings) {
        docx_styled_paragraph(&mut body, &doc.title, "Heading1");
        for para in prose_paragraphs(project, doc, settings) {
            body.push_str(&docx_prose_paragraph(&para, &settings.font));
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        if section.doc.medium.is_script() {
            // A script prints only what the writer typed, as in the PDF.
            let mut first_block = true;
            for block in screenplay_blocks(project, section.doc) {
                let text = if settings.styles.style_for_kind(block.kind).uppercase {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                let text = scenes.apply(block.kind, text, settings.include_scene_numbers);
                let new_page = first_block
                    && if i == 0 { settings.include_title_page } else { starts_new_page(section.doc) };
                first_block = false;
                docx_paragraph(&mut body, &text, &docx_block_style_id(block.kind, &text), new_page);
            }
        } else {
            let title = section_title(project, section.doc, i, settings);
            docx_styled_paragraph(&mut body, &title, "Heading1");
            for para in prose_paragraphs(project, section.doc, settings) {
                body.push_str(&docx_prose_paragraph(&para, &settings.font));
            }
        }
    }
    for doc in back_matter(project, settings) {
        docx_styled_paragraph(&mut body, &doc.title, "Heading1");
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
    // Headers and footers (prose only; scripts print their own page number).
    // Word has one section here, so the body's templates apply to the whole file;
    // front and back matter regions, a bare title page and "no header on chapter
    // openers" are PDF features.
    let hf = &settings.header_footer;
    let mut furniture_parts: Vec<(String, String)> = Vec::new();
    let mut furniture_refs = String::new();
    let mut furniture_rels = String::new();
    let mut furniture_types = String::new();
    let mut section_extra = String::new();
    if !is_script {
        let text_width = (page_w as i64 - ml as i64 - mr as i64).max(1);
        let mut variants: Vec<(&str, &PageSlots)> = vec![("default", &hf.body.odd)];
        if hf.different_odd_even {
            variants.push(("even", &hf.body.even));
        }
        if hf.different_first_page {
            variants.push(("first", &hf.body.first));
        }
        if variants.iter().any(|(_, slots)| !slots.is_empty()) {
            let clean = |text: &String| {
                text.replace("{title}", project.title.trim())
                    .replace("{author}", project.author.trim())
                    .replace("{page}", if hf.page_numbers { "{page}" } else { "" })
            };
            let size = hf.size;
            for (n, (kind, slots)) in variants.iter().enumerate() {
                for (tag, texts) in [("hdr", &slots.header), ("ftr", &slots.footer)] {
                    let word_tag = if tag == "hdr" { "header" } else { "footer" };
                    let name = format!("{word_tag}{}.xml", n + 1);
                    let id = format!("rIdHf{}{}", tag, n + 1);
                    let cleaned = [clean(&texts[0]), clean(&texts[1]), clean(&texts[2])];
                    furniture_parts.push((
                        format!("word/{name}"),
                        docx_furniture_part(word_tag, &cleaned, text_width, size, &settings.font),
                    ));
                    furniture_refs.push_str(&format!(
                        "<w:{word_tag}Reference w:type=\"{kind}\" r:id=\"{id}\"/>"
                    ));
                    furniture_rels.push_str(&format!(
                        "<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/{word_tag}\" Target=\"{name}\"/>"
                    ));
                    furniture_types.push_str(&format!(
                        "<Override PartName=\"/word/{name}\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.{word_tag}+xml\"/>"
                    ));
                }
            }
            if hf.different_first_page {
                section_extra.push_str("<w:titlePg/>");
            }
        }
        let fmt = match hf.number_format {
            NumberFormat::Arabic => None,
            NumberFormat::RomanLower => Some("lowerRoman"),
            NumberFormat::RomanUpper => Some("upperRoman"),
        };
        if fmt.is_some() || hf.start_number != 1 {
            let fmt_attr = fmt.map(|f| format!(" w:fmt=\"{f}\"")).unwrap_or_default();
            section_extra = format!("<w:pgNumType{fmt_attr} w:start=\"{}\"/>{section_extra}", hf.start_number);
        }
    }
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>{}\n<w:sectPr>{furniture_refs}<w:pgSz w:w=\"{page_w}\" w:h=\"{page_h}\"/><w:pgMar w:top=\"{mt}\" w:right=\"{mr}\" w:bottom=\"{mb}\" w:left=\"{ml}\"/>{section_extra}</w:sectPr></w:body></w:document>",
        body
    );
    let styles = docx_styles(project, settings);
    let content_types = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
<Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
{furniture_types}</Types>"#);
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#;
    let doc_rels = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
{furniture_rels}</Relationships>"#);
    // Word's "Mirror margins": the left margin becomes the inside one.
    let mirror = settings.mirror_margins && !project.compile_plan(settings).kind.is_script();
    let word_settings = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:settings xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">{}</w:settings>",
        format!(
            "{}{}",
            if mirror { "<w:mirrorMargins/>" } else { "" },
            if !is_script && hf.different_odd_even && !furniture_parts.is_empty() { "<w:evenAndOddHeaders/>" } else { "" }
        )
    );
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
            ("[Content_Types].xml", content_types.as_str()),
            ("_rels/.rels", rels),
            ("word/document.xml", &document),
            ("word/styles.xml", styles.as_str()),
            ("word/settings.xml", word_settings.as_str()),
            ("word/_rels/document.xml.rels", doc_rels.as_str()),
            ("docProps/core.xml", &core),
        ] {
            zip.start_file(name, opts).unwrap();
            zip.write_all(data.as_bytes()).unwrap();
        }
        for (name, data) in &furniture_parts {
            zip.start_file(name.as_str(), opts).unwrap();
            zip.write_all(data.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    buf
}
