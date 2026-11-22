use super::*;

fn demo() -> Project {
    portmanteau_core::sample::demo()
}

/// Text of one file inside an EPUB.
fn epub_file(bytes: &[u8], name: &str) -> String {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut archive.by_name(name).unwrap(), &mut text).unwrap();
    text
}

/// The CSS declarations that apply to the paragraph or heading containing
/// `text`: find its element, read its classes, and collect each class's rule
/// from `css`. This tests what a reader would see, not how it is spelled.
fn declarations_for(markup: &str, css: &str, text: &str) -> String {
    let body_start = markup.find("<body").unwrap_or(0);
    let at = markup[body_start..]
        .find(text)
        .map(|i| i + body_start)
        .unwrap_or_else(|| panic!("{text:?} not in the page body"));
    let before = &markup[..at];
    let open = ["<p class=\"", "<h1 class=\"", "<h2 class=\""]
        .iter()
        .filter_map(|tag| before.rfind(tag).map(|i| i + tag.len()))
        .max()
        .expect("paragraph element before the text");
    let classes = &markup[open..open + markup[open..].find('"').unwrap()];
    classes
        .split_whitespace()
        .filter_map(|class| {
            let start = css.find(&format!(".{class} {{"))?;
            let body = &css[start..];
            Some(body[..body.find('}')?].to_string())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn has(declarations: &str, wanted: &str) -> bool {
    declarations.contains(wanted)
}

#[test]
fn epub_css_uses_the_body_style_font() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Epub);
    s.styles.body.font = "SomeTestFont".into();
    let bytes = epub_manuscript(&p, &s);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut css = String::new();
    std::io::Read::read_to_string(&mut archive.by_name("OEBPS/style.css").unwrap(), &mut css)
        .unwrap();
    assert!(css.contains("SomeTestFont"), "EPUB CSS: {css}");
}

#[test]
fn html_uses_the_body_style_font() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Html);
    s.styles.body.font = "SomeTestFont".into();
    let html = html_manuscript(&p, &s);
    assert!(html.contains("SomeTestFont"), "HTML should carry the font family");
}

#[test]
fn epub_and_html_paragraphs_honour_alignment_and_first_line_indent() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Epub);
    s.styles.body.align = Alignment::Center;
    s.styles.body.first_line_indent = 21.0;
    s.styles.chapter_heading.align = Alignment::Right;
    let bytes = epub_manuscript(&p, &s);
    let css = epub_file(&bytes, "OEBPS/style.css");
    let chapter = epub_file(&bytes, "OEBPS/chapter-001.xhtml");
    let body = declarations_for(&chapter, &css, "The tide went out");
    assert!(has(&body, "text-indent: 21.0pt"), "{body}");
    assert!(has(&body, "text-align: center"), "{body}");
    let heading = declarations_for(&chapter, &css, "Chapter 1");
    assert!(has(&heading, "text-align: right"), "{heading}");

    let mut s = p.settings_for(ExportKind::Html);
    s.styles.body.align = Alignment::Justify;
    s.styles.body.first_line_indent = 17.0;
    let html = html_manuscript(&p, &s);
    let body = declarations_for(&html, &html, "The tide went out");
    assert!(has(&body, "text-indent: 17.0pt"), "{body}");
    assert!(has(&body, "text-align: justify"), "{body}");
}

#[test]
fn epub_has_mimetype_first() {
    let p = demo();
    let bytes = epub_manuscript(&p, &p.export);
    assert!(bytes.starts_with(b"PK"));
    assert!(bytes.windows(8).any(|w| w == b"mimetype"));
}

#[test]
fn docx_is_a_zip() {
    let p = demo();
    let bytes = docx_manuscript(&p, &p.export);
    assert!(bytes.starts_with(b"PK"));
}

#[test]
fn docx_and_rtf_page_size_follow_the_paper_setting() {
    let p = demo();
    // Width and height in twentieths of a point for every paper the
    // Compile screen offers.
    let expected = [
        (PaperSize::Letter, 12240, 15840),
        (PaperSize::A4, 11906, 16838),
        (PaperSize::A5, 8391, 11906),
        (PaperSize::UsTrade, 8640, 12960),
    ];
    for (paper, width, height) in expected {
        let mut s = p.settings_for(ExportKind::Docx);
        s.paper = paper;
        let xml = docx_document_xml(docx_manuscript(&p, &s));
        assert!(
            xml.contains(&format!("<w:pgSz w:w=\"{width}\" w:h=\"{height}\"/>")),
            "DOCX should carry {} as {width}x{height} twips",
            paper.label()
        );

        let mut s = p.settings_for(ExportKind::Rtf);
        s.paper = paper;
        let rtf = rtf_manuscript(&p, &s);
        assert!(
            rtf.contains(&format!("\\paperw{width}\\paperh{height}")),
            "RTF should carry {} as {width}x{height} twips",
            paper.label()
        );
    }
}

fn docx_document_xml(bytes: Vec<u8>) -> String {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    std::io::Read::read_to_string(
        &mut archive.by_name("word/document.xml").unwrap(),
        &mut xml,
    )
    .unwrap();
    xml
}

/// A novel with one chapter whose second paragraph uses a writer-defined
/// paragraph style stored in the project stylesheet.
fn styled_prose_project() -> Project {
    let mut project = Project::new("Style Test", ProjectKind::Novel);
    let parent = project.folder_id("Draft");
    let doc_id = project.add_document(parent, "Chapter One", DocKind::Chapter, Medium::Prose);
    let sheet_id = project.document(&doc_id).unwrap().style_sheet_id.clone();
    let sheet = project
        .style_sheets
        .iter_mut()
        .find(|sheet| sheet.id == sheet_id)
        .expect("prose stylesheet");
    sheet.styles.push(ParagraphStyle {
        id: "epigraph".into(),
        name: "Epigraph".into(),
        format: ElementStyle {
            align: Alignment::Center,
            size: 9.0,
            first_line_indent: 0.0,
            before: 6.0,
            after: 6.0,
            ..ElementStyle::default()
        },
        semantic_kind: None,
        next_style: Some("body".into()),
        text_transform: TextTransform::None,
    });
    let doc = project.document_mut(&doc_id).unwrap();
    doc.content = RichDocument {
        paragraphs: vec![
            RichParagraph {
                style_id: "body".into(),
                content: vec![InlineContent::Text(TextRun::with_marks(
                    "First paragraph.",
                    vec![],
                ))],
            },
            RichParagraph {
                style_id: "epigraph".into(),
                content: vec![InlineContent::Text(TextRun::with_marks(
                    "A centred epigraph.",
                    vec![],
                ))],
            },
            RichParagraph {
                style_id: "chapter-heading".into(),
                content: vec![InlineContent::Text(TextRun::with_marks(
                    "A heading inside the body",
                    vec![],
                ))],
            },
        ],
    };
    project
}

#[test]
fn prose_export_honours_project_paragraph_styles() {
    let project = styled_prose_project();
    let html = html_manuscript(&project, &project.settings_for(ExportKind::Html));
    let epigraph = declarations_for(&html, &html, "A centred epigraph.");
    assert!(
        has(&epigraph, "text-align: center") && has(&epigraph, "font-size: 9.0pt"),
        "custom paragraph style should reach HTML: {epigraph}"
    );
    assert!(
        has(&epigraph, "margin: 6.0pt 0.0pt 6.0pt 0.0pt"),
        "space before and after should reach HTML: {epigraph}"
    );
    assert!(
        html.lines()
            .any(|line| line.contains("<h2") && line.contains("A heading inside the body")),
        "an in-body chapter-heading paragraph should become a subheading"
    );

    let xml = docx_document_xml(docx_manuscript(
        &project,
        &project.settings_for(ExportKind::Docx),
    ));
    assert!(xml.contains("A centred epigraph."));
    assert!(
        xml.contains("<w:jc w:val=\"center\"/>") && xml.contains("<w:sz w:val=\"18\"/>"),
        "DOCX should carry the custom style's alignment and size"
    );
    assert!(
        xml.contains("<w:pStyle w:val=\"Heading2\"/>"),
        "in-body heading should use the DOCX heading style"
    );

    let text = txt_manuscript(&project, &project.settings_for(ExportKind::Txt));
    assert!(
        text.contains("First paragraph.\n\nA centred epigraph.\n\nA heading inside the body"),
        "paragraph boundaries must survive text export: {text}"
    );
}

#[test]
fn stylesheet_appearance_edits_reach_export_without_a_compile_save() {
    let mut project = Project::new("Live Style", ProjectKind::Novel);
    let parent = project.folder_id("Draft");
    let doc_id = project.add_document(parent, "Chapter", DocKind::Chapter, Medium::Prose);
    let sheet_id = project.document(&doc_id).unwrap().style_sheet_id.clone();
    let sheet = project
        .style_sheets
        .iter_mut()
        .find(|sheet| sheet.id == sheet_id)
        .unwrap();
    let body = sheet
        .styles
        .iter_mut()
        .find(|style| style.id == "body")
        .unwrap();
    body.format.size = 13.0;
    body.format.align = Alignment::Right;
    body.format.first_line_indent = 0.0;
    let doc = project.document_mut(&doc_id).unwrap();
    doc.set_plain_text("The stylesheet is the source of truth.");

    // The format settings are still seeded from the old base style, as if
    // the writer had never opened Compile after editing the sheet.
    let settings = project.settings_for(ExportKind::Html);
    let html = html_manuscript(&project, &settings);
    let body = declarations_for(&html, &html, "The stylesheet is the source of truth.");
    assert!(
        has(&body, "font-size: 13.0pt") && has(&body, "text-align: right"),
        "project sheet edit should win over a stale seeded format: {body}"
    );
}

#[test]
fn prose_export_keeps_inline_marks_and_paragraph_breaks() {
    let mut project = Project::new("Marks Test", ProjectKind::Novel);
    let parent = project.folder_id("Draft");
    let doc_id = project.add_document(parent, "Chapter", DocKind::Chapter, Medium::Prose);
    let doc = project.document_mut(&doc_id).unwrap();
    doc.content = RichDocument {
        paragraphs: vec![
            RichParagraph {
                style_id: "body".into(),
                content: vec![
                    InlineContent::Text(TextRun::with_marks("Bold", vec![InlineMark::Bold])),
                    InlineContent::Text(TextRun::plain(" and ")),
                    InlineContent::Text(TextRun::with_marks(
                        "italic.",
                        vec![InlineMark::Italic],
                    )),
                ],
            },
            RichParagraph {
                style_id: "body".into(),
                content: vec![InlineContent::Text(TextRun::plain("Second paragraph."))],
            },
        ],
    };

    let html = html_manuscript(&project, &project.settings_for(ExportKind::Html));
    assert!(html.contains("<strong>Bold</strong>"));
    assert!(html.contains("<em>italic.</em>"));
    assert!(html.contains("Second paragraph."));

    let markdown = markdown_manuscript(&project, &project.settings_for(ExportKind::Markdown));
    assert!(markdown.contains("**Bold**"));
    assert!(markdown.contains("*italic.*"));
    assert!(markdown.contains("Second paragraph.\n"));

    let rtf = rtf_manuscript(&project, &project.settings_for(ExportKind::Rtf));
    assert!(rtf.contains("\\b Bold\\b0"));
    assert!(rtf.contains("\\i italic.\\i0"));

    let xml = docx_document_xml(docx_manuscript(
        &project,
        &project.settings_for(ExportKind::Docx),
    ));
    assert!(xml.contains("<w:b/>"));
    assert!(xml.contains("<w:i/>"));
    assert!(xml.contains("Second paragraph."));

    let text = txt_manuscript(&project, &project.settings_for(ExportKind::Txt));
    assert!(text.contains("Bold and italic.\n\nSecond paragraph."));
}

#[test]
fn fdx_is_wellformed_ish() {
    let p = demo();
    let mut settings = p.export.clone();
    settings.compile_type = Some(CompileType::TvScript);
    let fdx = fdx_manuscript(&p, &settings);
    assert!(fdx.contains("<FinalDraft"));
    assert!(fdx.contains("Scene Heading"));
}

#[test]
fn fdx_export_keeps_inline_marks_and_image_alt_text() {
    let mut p = demo();
    let doc_id = p
        .documents
        .iter()
        .find(|doc| doc.medium.is_script() && doc.include_in_export)
        .map(|doc| doc.id.clone())
        .expect("demo has a screenplay document");
    let sheet = p
        .style_sheets
        .iter()
        .find(|sheet| sheet.medium == Medium::Screenplay)
        .cloned()
        .expect("screenplay sheet");
    let doc = p.document_mut(&doc_id).unwrap();
    let target = doc
        .content
        .paragraphs
        .iter_mut()
        .find(|paragraph| paragraph.style_id == "action")
        .expect("screenplay action paragraph");
    target.content = vec![
        InlineContent::Text(TextRun::with_marks(
            "Bold action",
            vec![InlineMark::Bold],
        )),
        InlineContent::Text(TextRun::with_marks(
            " and italic",
            vec![InlineMark::Italic],
        )),
    ];
    doc.content.paragraphs.push(RichParagraph {
        style_id: "action".into(),
        content: vec![InlineContent::Image(ImageObject {
            asset_id: "images/chart.png".into(),
            alt_text: "Harbor chart".into(),
            caption: String::new(),
            width: None,
            height: None,
        })],
    });

    let mut settings = p.export.clone();
    settings.compile_type = Some(CompileType::TvScript);
    let fdx = fdx_manuscript(&p, &settings);
    assert!(fdx.contains("Style=\"Bold\""), "bold run missing from FDX");
    assert!(fdx.contains("Style=\"Italic\""), "italic run missing from FDX");
    assert!(
        fdx.contains("[Image: Harbor chart]"),
        "lossy FDX export should include the image's alt text"
    );
    assert_eq!(sheet.medium, Medium::Screenplay);
}

const TINY_PNG: [u8; 67] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
    0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
    0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
    0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
    0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn epub_embeds_a_cover_only_when_the_file_exists() {
    let p = demo();
    let dir = std::env::temp_dir().join(portmanteau_core::new_id("epub-cover"));
    fs::create_dir_all(&dir).unwrap();
    let png = dir.join("cover.png");
    fs::write(&png, TINY_PNG).unwrap();

    let mut with_cover = p.settings_for(ExportKind::Epub);
    with_cover.cover_image = png.display().to_string();
    let bytes = epub_manuscript(&p, &with_cover);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(names.iter().any(|n| n == "OEBPS/cover-image/cover.png"));
    assert!(names.iter().any(|n| n == "OEBPS/cover.xhtml"));
    let mut opf = String::new();
    std::io::Read::read_to_string(&mut archive.by_name("OEBPS/content.opf").unwrap(), &mut opf)
        .unwrap();
    assert!(opf.contains("properties=\"cover-image\""));
    assert!(opf.contains("<meta name=\"cover\" content=\"cover-img\"/>"));

    let bytes = epub_manuscript(&p, &p.settings_for(ExportKind::Epub));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(!names.iter().any(|n| n.contains("cover")));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn every_format_exports_to_disk() {
    let p = demo();
    let dir = std::env::temp_dir().join(format!("portmanteau-test-{}", portmanteau_core::new_id("t")));
    for kind in ExportKind::all() {
        let mut s = p.export.clone();
        s.kind = kind;
        let path = dir.join(default_filename(&p, kind));
        let info = export(&p, &s, Some(&path)).unwrap();
        assert!(info.bytes > 100, "{:?} too small", kind);
        assert!(path.exists());
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn lossy_text_export_warns_and_uses_an_image_alt_placeholder() {
    let mut project = Project::new("Image export", ProjectKind::Novel);
    let doc_id = project.add_document(
        project.folder_id("Draft"),
        "Image scene",
        DocKind::Chapter,
        Medium::Prose,
    );
    let sheet = StyleSheet::novel_prose(&project.styles);
    let doc = project.document_mut(&doc_id).unwrap();
    doc.content = RichDocument::from_plain_text("before after", "body");
    doc.content.insert_image(
        7..7,
        ImageObject {
            asset_id: "images/harbor.png".into(),
            alt_text: "Harbor map".into(),
            caption: String::new(),
            width: None,
            height: None,
        },
        false,
        &sheet,
    );
    let settings = project.settings_for(ExportKind::Txt);
    let path = std::env::temp_dir().join(format!(
        "portmanteau-image-export-{}.txt",
        portmanteau_core::new_id("t")
    ));

    let info = export(&project, &settings, Some(&path)).unwrap();
    let output = fs::read_to_string(&path).unwrap();
    assert!(
        info.message.contains("alt-text placeholders"),
        "lossy image export should be reported"
    );
    assert!(output.contains("[Image: Harbor map]"));
    let _ = fs::remove_file(path);
}

#[test]
fn custom_margins_reach_docx_and_rtf() {
    let p = demo();
    let margins = Some(PageMargins {
        left: 90.0,
        right: 36.0,
        top: 54.0,
        bottom: 108.0,
    });
    let mut s = p.settings_for(ExportKind::Docx);
    s.margins = margins;
    let xml = docx_document_xml(docx_manuscript(&p, &s));
    assert!(
        xml.contains(r#"<w:pgMar w:top="1080" w:right="720" w:bottom="2160" w:left="1800"/>"#),
        "{xml}"
    );
    let mut s = p.settings_for(ExportKind::Rtf);
    s.margins = margins;
    let rtf = rtf_manuscript(&p, &s);
    assert!(rtf.contains("\\margl1800\\margr720\\margt1080\\margb2160"), "{rtf}");
}

#[test]
fn automatic_margins_differ_for_prose_and_scripts_and_never_swallow_the_page() {
    let p = demo();
    let prose = p.settings_for(ExportKind::Pdf);
    let auto = page_margins(&p, &prose);
    assert!((auto.left - mm_to_pt(20.0)).abs() < 0.01);
    let mut absurd = prose.clone();
    absurd.margins = Some(PageMargins {
        left: 300.0,
        right: 300.0,
        top: 0.0,
        bottom: 0.0,
    });
    let clamped = page_margins(&p, &absurd);
    assert!(clamped.left + clamped.right <= 612.0 * 2.0 / 3.0 + 0.01);
}

#[test]
fn a_hostile_font_name_cannot_break_out_of_the_style_element() {
    let p = demo();
    let hostile = "Evil</style><script>alert(1)</script>\"; } body { display:none";
    let mut s = p.settings_for(ExportKind::Html);
    s.styles.body.font = hostile.into();
    let html = html_manuscript(&p, &s);
    assert!(!html.contains("<script>"), "{html}");
    assert_eq!(html.matches("</style>").count(), 1, "exactly one closing style tag");

    let mut s = p.settings_for(ExportKind::Epub);
    s.styles.body.font = hostile.into();
    let bytes = epub_manuscript(&p, &s);
    let css = epub_file(&bytes, "OEBPS/style.css");
    assert!(!css.contains("</style>") && !css.contains("<script>"), "{css}");
}

fn docx_part(bytes: &[u8], name: &str) -> String {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut archive.by_name(name).unwrap(), &mut text).unwrap();
    text
}

/// The `<w:style>` element with this id, from styles.xml.
fn docx_style_xml(styles: &str, id: &str) -> String {
    let start = styles
        .find(&format!("w:styleId=\"{id}\""))
        .unwrap_or_else(|| panic!("style {id} missing"));
    let end = styles[start..].find("</w:style>").unwrap();
    styles[start..start + end].to_string()
}

#[test]
fn docx_styles_come_from_the_element_styles() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Docx);
    s.styles.chapter_heading.size = 30.0;
    s.styles.chapter_heading.before = 50.0;
    s.styles.chapter_heading.align = Alignment::Right;
    s.styles.body.leading = 2.0;
    s.styles.body.first_line_indent = 36.0;
    s.styles.dialogue.indent = 100.0;
    s.styles.dialogue.right_indent = 40.0;
    s.styles.title_page_title.align = Alignment::Left;
    let bytes = docx_manuscript(&p, &s);
    let styles = docx_part(&bytes, "word/styles.xml");

    let heading = docx_style_xml(&styles, "Heading1");
    assert!(heading.contains(r#"w:sz w:val="60""#), "{heading}");
    assert!(heading.contains(r#"w:before="1000""#), "{heading}");
    assert!(heading.contains(r#"<w:jc w:val="right"/>"#), "{heading}");
    assert!(heading.contains("<w:pageBreakBefore/>"), "{heading}");

    let body = docx_style_xml(&styles, "Body");
    assert!(body.contains(r#"w:line="480""#) && body.contains(r#"w:firstLine="720""#), "{body}");

    let dialogue = docx_style_xml(&styles, "ScriptDialogue");
    assert!(dialogue.contains(r#"w:left="2000""#) && dialogue.contains(r#"w:right="800""#), "{dialogue}");

    let title = docx_style_xml(&styles, "Title");
    assert!(!title.contains("<w:jc"), "a left-aligned title carries no jc: {title}");
}

#[test]
fn docx_script_paragraphs_use_script_styles_and_print_no_invented_heading() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Docx);
    s.compile_type = Some(CompileType::TvScript);
    s.include_title_page = false;
    let plan = p.compile_plan(&s);
    assert!(plan.included.iter().any(|d| d.medium.is_script()), "demo has a script");
    let xml = docx_document_xml(docx_manuscript(&p, &s));
    assert!(xml.contains(r#"<w:pStyle w:val="ScriptDialogue"/>"#), "{xml}");
    assert!(xml.contains(r#"<w:pStyle w:val="ScriptCharacter"/>"#));
    assert!(!xml.contains(r#"w:val="Heading1""#), "scripts add no chapter heading");
    assert!(!xml.contains("w:ind w:left=\"4320\""), "no hard-coded indents");
}

fn rtf_braces_balance(rtf: &str) -> bool {
    let mut depth = 0i32;
    let mut chars = rtf.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next(); // an escaped character is not a brace
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

#[test]
fn rtf_styles_reach_title_headings_and_script_elements() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Rtf);
    s.styles.title_page_title.size = 40.0;
    s.styles.chapter_heading.size = 30.0;
    s.styles.body.leading = 2.0;
    let rtf = rtf_manuscript(&p, &s);
    assert!(rtf.contains("\\fs80"), "title size from the title style");
    assert!(rtf.contains("\\fs60"), "heading size from the heading style");
    assert!(rtf.contains("\\sl480\\slmult1"), "body leading reaches RTF");
    assert!(!rtf.contains("\\fs52") && !rtf.contains("\\fs36"), "no hard-coded sizes");
    assert!(rtf_braces_balance(&rtf));

    let mut s = p.settings_for(ExportKind::Rtf);
    s.compile_type = Some(CompileType::TvScript);
    s.styles.dialogue.indent = 100.0;
    s.styles.dialogue.right_indent = 40.0;
    let rtf = rtf_manuscript(&p, &s);
    assert!(rtf.contains("\\li2000\\ri800"), "dialogue indents come from its style");
    assert!(!rtf.contains("\\li4400") && !rtf.contains("\\li3200"), "no hard-coded script indents");
    assert!(rtf_braces_balance(&rtf));
}

#[test]
fn rtf_text_with_braces_and_unicode_stays_valid() {
    let mut p = Project::new("Odd {title} \\ ünïcode — dash", ProjectKind::Novel);
    let parent = p.folder_id("Draft");
    let id = p.add_document(parent, "One", DocKind::Chapter, Medium::Prose);
    p.document_mut(&id).unwrap().set_plain_text("A } stray { brace, a backslash \\ and “quotes”.");
    let rtf = rtf_manuscript(&p, &p.settings_for(ExportKind::Rtf));
    assert!(rtf_braces_balance(&rtf), "{rtf}");
    assert!(rtf.contains("\\u8220?"), "curly quote is written as a unicode escape");
}

/// Every piece of text a format's output carries, for "is it in there?" checks.
fn output_text(kind: ExportKind, project: &Project, settings: &ExportSettings) -> String {
    match kind {
        ExportKind::Pdf => pdf_layout(project, settings)
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .flat_map(|item| item.segments.iter().map(|segment| segment.text.clone()))
            .collect::<Vec<_>>()
            .join(" "),
        ExportKind::Epub => {
            let bytes = epub_manuscript(project, settings);
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
            let names: Vec<String> = archive
                .file_names()
                .filter(|name| name.ends_with(".xhtml") && !name.ends_with("nav.xhtml"))
                .map(str::to_owned)
                .collect();
            names
                .iter()
                .map(|name| {
                    let mut text = String::new();
                    std::io::Read::read_to_string(&mut archive.by_name(name).unwrap(), &mut text).unwrap();
                    // Visible text only: the head's <title> is page metadata.
                    text.split_once("<body>").map(|(_, body)| body.to_string()).unwrap_or(text)
                })
                .collect()
        }
        ExportKind::Docx => docx_document_xml(docx_manuscript(project, settings)),
        ExportKind::Html => html_manuscript(project, settings),
        ExportKind::Markdown => markdown_manuscript(project, settings),
        ExportKind::Rtf => rtf_manuscript(project, settings),
        ExportKind::Txt => txt_manuscript(project, settings),
        other => panic!("not a prose text format: {other:?}"),
    }
}

const PROSE_FORMATS: [ExportKind; 7] = [
    ExportKind::Pdf,
    ExportKind::Epub,
    ExportKind::Docx,
    ExportKind::Html,
    ExportKind::Markdown,
    ExportKind::Rtf,
    ExportKind::Txt,
];

#[test]
fn every_prose_writer_respects_which_documents_are_included() {
    let mut p = demo();
    let chapter_two = p
        .documents
        .iter()
        .find(|d| d.title.starts_with("Chapter Two"))
        .map(|d| d.id.clone())
        .expect("demo has Chapter Two");
    p.document_mut(&chapter_two).unwrap().include_in_export = false;
    for kind in PROSE_FORMATS {
        let settings = p.settings_for(kind);
        let text = output_text(kind, &p, &settings);
        assert!(text.contains("The tide went out"), "{kind:?} should carry chapter 1");
        assert!(
            !text.contains("By noon the mud"),
            "{kind:?} must skip the excluded chapter"
        );
    }
}

#[test]
fn every_prose_writer_respects_the_title_page_and_contents_switches() {
    let p = demo();
    for kind in PROSE_FORMATS {
        let mut settings = p.settings_for(kind);
        settings.include_title_page = true;
        settings.include_toc = true;
        let with = output_text(kind, &p, &settings);
        assert!(with.contains("A novel, with a companion pilot"), "{kind:?} title page");
        assert!(with.to_lowercase().contains("contents"), "{kind:?} contents");

        settings.include_title_page = false;
        settings.include_toc = false;
        let without = output_text(kind, &p, &settings);
        assert!(!without.contains("A novel, with a companion pilot"), "{kind:?} title page off");
        assert!(!without.to_lowercase().contains("contents"), "{kind:?} contents off");
        assert!(without.contains("The tide went out"), "{kind:?} still has the book");
    }
}

#[test]
fn every_script_writer_numbers_scenes_the_same_way_and_adds_no_heading() {
    let p = demo();
    let formats = [
        ExportKind::Pdf,
        ExportKind::Epub,
        ExportKind::Docx,
        ExportKind::Html,
        ExportKind::Rtf,
    ];
    for kind in formats {
        let mut settings = p.settings_for(kind);
        settings.compile_type = Some(CompileType::TvScript);
        settings.include_title_page = false;
        settings.include_toc = false;
        settings.include_scene_numbers = true;
        let numbered = output_text(kind, &p, &settings);
        // RTF writes the non-breaking gap as a unicode escape.
        let first = if kind == ExportKind::Rtf {
            "1\\u160?\\u160?\\u160?EXT. COLD HARBOR"
        } else {
            "1\u{a0}\u{a0}\u{a0}EXT. COLD HARBOR"
        };
        assert!(numbered.contains(first), "{kind:?} should number the first scene");
        assert!(
            !numbered.contains("Cold Harbor · Pilot") || kind == ExportKind::Pdf,
            "{kind:?} must not print the document title as a heading"
        );

        settings.include_scene_numbers = false;
        let plain = output_text(kind, &p, &settings);
        assert!(!plain.contains(first), "{kind:?} numbering off");
        assert!(plain.contains("EXT. COLD HARBOR"), "{kind:?} still has the scene");
    }
}

#[test]
fn every_flowing_format_drops_the_title_block_to_where_the_pdf_puts_it() {
    let p = demo();
    // Letter, automatic book margins: 30% of 792 pt, less the top margin and
    // one ascent of the 24 pt title.
    let drop = 792.0 * 0.30 - mm_to_pt(22.0) - 24.0 * 0.93;

    let s = p.settings_for(ExportKind::Docx);
    let styles = docx_part(&docx_manuscript(&p, &s), "word/styles.xml");
    let title = docx_style_xml(&styles, "Title");
    let before = format!("w:before=\"{}\"", pt_to_twips(drop));
    assert!(title.contains(&before), "{title} should contain {before}");

    let rtf = rtf_manuscript(&p, &p.settings_for(ExportKind::Rtf));
    assert!(rtf.contains(&format!("\\sb{}", pt_to_twips(drop))), "RTF title spacing");

    let html = html_manuscript(&p, &p.settings_for(ExportKind::Html));
    assert!(html.contains(&format!(".title-page {{ margin-top: {drop:.1}pt; }}")), "HTML title drop");

    let epub = epub_manuscript(&p, &p.settings_for(ExportKind::Epub));
    assert!(
        epub_file(&epub, "OEBPS/style.css").contains(&format!(".title-page {{ margin-top: {drop:.1}pt; }}")),
        "EPUB title drop"
    );

    // A taller top margin leaves less to drop; it never goes negative.
    let mut tall = p.settings_for(ExportKind::Docx);
    tall.margins = Some(PageMargins { left: 72.0, right: 72.0, top: 400.0, bottom: 72.0 });
    assert_eq!(title_drop_pt(&p, &tall), 0.0, "a tall margin leaves nothing to drop");
}

#[test]
fn script_scenes_run_on_and_only_a_chapter_level_document_starts_a_page() {
    let mut p = Project::new("Run on", ProjectKind::Television);
    let mut ids = Vec::new();
    for (title, kind) in [
        ("Episode 1", DocKind::Chapter),
        ("Scene a", DocKind::Scene),
        ("Scene b", DocKind::Scene),
        ("Scene c", DocKind::Scene),
        ("Episode 2", DocKind::Chapter),
    ] {
        let id = p.add_document(None, title, kind, Medium::TvScript);
        let doc = p.documents.iter_mut().find(|d| d.id == id).unwrap();
        doc.content = RichDocument::from_plain_text(&format!("INT. {title} - DAY\n\nSomeone speaks."), "action");
        ids.push(id);
    }
    let mut s = p.settings_for(ExportKind::Pdf);
    s.compile_type = Some(CompileType::TvScript);
    s.include_title_page = false;
    assert_eq!(p.compile_plan(&s).included.len(), 5, "all five documents compile");

    // Five documents but only two chapter-level starts: the second episode
    // opens page two and the scenes between share page one.
    let layout = pdf_layout(&p, &s);
    assert_eq!(layout.pages.len(), 2, "scenes run on, the episode breaks the page");

    let make = |kind, medium| Document::new(None, 0, "x", kind, medium);
    assert!(!starts_new_page(&make(DocKind::Scene, Medium::TvScript)));
    assert!(starts_new_page(&make(DocKind::Chapter, Medium::TvScript)));
    assert!(starts_new_page(&make(DocKind::Scene, Medium::Prose)), "prose always starts a page");

    let docx = docx_document_xml(docx_manuscript(&p, &s));
    assert_eq!(docx.matches("<w:pageBreakBefore/>").count(), 1, "one episode break after the first");
}

#[test]
fn every_script_writer_prints_the_trade_cover_not_a_book_title_page() {
    let p = demo();
    assert!(!p.subtitle.is_empty(), "the demo has a subtitle a book cover would print");
    for kind in [
        ExportKind::Pdf,
        ExportKind::Epub,
        ExportKind::Docx,
        ExportKind::Html,
        ExportKind::Rtf,
        ExportKind::Txt,
    ] {
        let mut s = p.settings_for(kind);
        s.compile_type = Some(CompileType::TvScript);
        s.include_title_page = true;
        s.include_toc = false;
        let text = output_text(kind, &p, &s);
        assert!(text.contains("THE SALT ROAD"), "{kind:?}: title in capitals");
        assert!(text.contains("Written by"), "{kind:?}: credit line");
        assert!(text.contains("Ada Marlowe"), "{kind:?}: author");
        assert!(
            !text.contains(">by Ada") && text.lines().all(|l| !l.trim().starts_with("by Ada")),
            "{kind:?}: not a book byline"
        );
        assert!(!text.contains(&p.subtitle), "{kind:?}: no book subtitle");
        assert!(text.contains("Draft"), "{kind:?}: draft line");
    }

    // The script starts on its own page after the cover, in the flowing formats.
    let mut s = p.settings_for(ExportKind::Docx);
    s.compile_type = Some(CompileType::TvScript);
    s.include_title_page = true;
    let xml = docx_document_xml(docx_manuscript(&p, &s));
    assert!(xml.contains("<w:pageBreakBefore/>"), "DOCX breaks after the cover");
    let rtf = rtf_manuscript(&p, &s);
    assert!(rtf.contains("\\pagebb"), "RTF breaks after the cover");
    s.include_title_page = false;
    assert!(!docx_document_xml(docx_manuscript(&p, &s)).contains("<w:pageBreakBefore/>"));
}

#[test]
fn the_opening_fade_in_is_flush_left_in_every_flowing_format() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Docx);
    s.compile_type = Some(CompileType::TvScript);
    s.include_title_page = false;
    let xml = docx_document_xml(docx_manuscript(&p, &s));
    assert!(xml.contains(r#"<w:pStyle w:val="ScriptOpeningTransition"/>"#), "DOCX uses the opening style");
    let styles = docx_part(&docx_manuscript(&p, &s), "word/styles.xml");
    let opening = docx_style_xml(&styles, "ScriptOpeningTransition");
    assert!(!opening.contains("w:jc w:val=\"right\""), "{opening}");
    let transition = docx_style_xml(&styles, "ScriptTransition");
    assert!(transition.contains("w:jc w:val=\"right\""), "other transitions stay right-hand: {transition}");

    let rtf = rtf_manuscript(&p, &s);
    let fade = rtf.lines().find(|l| l.contains("FADE IN")).expect("FADE IN in the RTF");
    assert!(fade.contains("\\ql") && !fade.contains("\\qr"), "{fade}");

    let html = html_manuscript(&p, &s);
    assert!(html.contains("script-transition script-opening\">FADE IN"), "HTML marks the opening");
    let rule = html.split(".script-opening {").nth(1).and_then(|r| r.split('}').next()).expect("a rule");
    assert!(rule.contains("text-align: left"), "and left-aligns it: {rule}");
}

#[test]
fn the_cover_prints_the_projects_contact_and_the_writers_draft_date() {
    let mut p = demo();
    p.contact = "Ada Marlowe\n12 Harbour Lane\nada@example.com".into();
    p.draft_label = "Shooting script".into();
    p.draft_date = "14 March 2027".into();
    for kind in [
        ExportKind::Pdf,
        ExportKind::Epub,
        ExportKind::Docx,
        ExportKind::Html,
        ExportKind::Rtf,
        ExportKind::Txt,
    ] {
        let mut s = p.settings_for(kind);
        s.compile_type = Some(CompileType::TvScript);
        s.include_title_page = true;
        s.include_toc = false;
        let text = output_text(kind, &p, &s);
        for want in ["12 Harbour Lane", "ada@example.com", "Shooting script", "14 March 2027"] {
            assert!(text.contains(want), "{kind:?} should print {want:?}");
        }
    }
    // Blank date: today's, so an untouched project never prints a stale one.
    p.draft_date.clear();
    let s = {
        let mut s = p.settings_for(ExportKind::Txt);
        s.compile_type = Some(CompileType::TvScript);
        s
    };
    let today = chrono::Local::now().format("%d %B %Y").to_string();
    assert!(output_text(ExportKind::Txt, &p, &s).contains(&today));

    // The draft line is right-aligned; the contact lines are not.
    let mut s = p.settings_for(ExportKind::Docx);
    s.compile_type = Some(CompileType::TvScript);
    let styles = docx_part(&docx_manuscript(&p, &s), "word/styles.xml");
    assert!(docx_style_xml(&styles, "ScriptCoverDraft").contains("w:jc w:val=\"right\""));
    assert!(!docx_style_xml(&styles, "ScriptCoverContact").contains("w:jc"));

    // Fountain carries it as a multi-line Contact key and the draft date.
    let fountain = fountain_manuscript(&p, &p.settings_for(ExportKind::Fountain));
    assert!(fountain.contains("Contact:\n    Ada Marlowe\n    12 Harbour Lane"), "{fountain}");
    p.draft_date = "14 March 2027".into();
    let fountain = fountain_manuscript(&p, &p.settings_for(ExportKind::Fountain));
    assert!(fountain.contains("Draft date: 14 March 2027"));
}

#[test]
fn page_counts_come_from_the_real_layout_for_every_manuscript_type() {
    let mut p = demo();
    let counts = page_counts(&p);
    let chapter = p.documents.iter().find(|d| d.kind == DocKind::Chapter).unwrap().id.clone();
    let pilot = p.documents.iter().find(|d| d.medium == Medium::TvScript).unwrap().id.clone();
    assert!(counts.by_doc[&chapter] >= 1, "a book chapter has pages");
    assert!(counts.by_doc[&pilot] >= 1, "the pilot has pages from its own script layout");

    // The total is the pages of each manuscript type the project contains: the
    // book, the TV pilot and the stage play are three separate manuscripts.
    let mut total = 0;
    for kind in [CompileType::Prose, CompileType::TvScript, CompileType::StagePlay, CompileType::Screenplay] {
        let mut settings = p.settings_for(ExportKind::Pdf);
        settings.compile_type = Some(kind);
        let layout = pdf_layout(&p, &settings);
        if layout.sections.iter().any(|s| s.doc_id.is_some()) {
            total += layout.page_count();
        }
    }
    assert_eq!(counts.total, total);

    // A document no compile includes has no count, not a guess.
    p.document_mut(&chapter).unwrap().include_in_export = false;
    let counts = page_counts(&p);
    assert!(!counts.by_doc.contains_key(&chapter));
    assert_eq!(counts.label(&chapter), "–");
}
