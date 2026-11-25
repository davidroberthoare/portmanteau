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
fn docx_page_size_follows_the_paper_setting() {
    let p = demo();
    // Width and height in twentieths of a point for every paper the
    // Compile screen offers.
    let expected = [
        (PaperSize::Letter, 12240, 15840),
        (PaperSize::A4, 11906, 16838),
        (PaperSize::A5, 8391, 11906),
        (PaperSize::Trade6x9, 8640, 12960),
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
..Default::default()
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
..Default::default()
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
fn custom_margins_reach_docx() {
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
        ExportKind::Txt => txt_manuscript(project, settings),
        other => panic!("not a prose text format: {other:?}"),
    }
}

const PROSE_FORMATS: [ExportKind; 6] = [
    ExportKind::Pdf,
    ExportKind::Epub,
    ExportKind::Docx,
    ExportKind::Html,
    ExportKind::Markdown,
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
    ];
    for kind in formats {
        let mut settings = p.settings_for(kind);
        settings.compile_type = Some(CompileType::TvScript);
        settings.include_title_page = false;
        settings.include_toc = false;
        settings.include_scene_numbers = true;
        let numbered = output_text(kind, &p, &settings);
        let first = "1\u{a0}\u{a0}\u{a0}EXT. COLD HARBOR";
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

#[test]
fn links_become_anchors_in_web_formats_and_never_unsafe_ones() {
    let mut p = Project::new("Links", ProjectKind::Novel);
    let id = p.add_document(None, "Chapter", DocKind::Chapter, Medium::Prose);
    {
        let doc = p.document_mut(&id).unwrap();
        doc.content = RichDocument::from_plain_text("See the chart, the log and the trap.", "body");
        doc.content.set_link(4..13, Some("https://example.org/a?b=1&c=2"));
        doc.content.set_link(18..25, Some("note:doc-123"));
        doc.content.set_link(30..35, Some("javascript:alert(1)"));
    }
    let mut s = p.settings_for(ExportKind::Html);
    s.compile_type = Some(CompileType::Prose);
    s.include_title_page = false;
    let html = html_manuscript(&p, &s);
    assert!(html.contains(r#"<a href="https://example.org/a?b=1&amp;c=2">the chart</a>"#), "{html}");
    assert!(!html.contains("note:doc-123") && !html.contains("javascript:"), "internal and unsafe links are not written out");
    assert!(html.contains("the log") && html.contains("trap"), "their text still prints");

    let mut m = p.settings_for(ExportKind::Markdown);
    m.compile_type = Some(CompileType::Prose);
    m.include_title_page = false;
    let md = markdown_manuscript(&p, &m);
    assert!(md.contains("[the chart](https://example.org/a?b=1&c=2)"), "{md}");
    assert!(!md.contains("javascript:"));
    assert_eq!(export_href(" HTTPS://X.test "), Some("HTTPS://X.test".to_string()));
    assert_eq!(export_href("ftp://x"), None);
    assert_eq!(export_href("mailto:a@b.test"), None, "email links are not written out");

    // An address cannot break out of its attribute, whatever the project file says.
    let hostile = r#"https://x.test/" onmouseover="alert(1)" data-x='1'><script>"#;
    let encoded = export_href(hostile).unwrap();
    assert!(!encoded.contains(['"', '\'', '<', '>', ' ']), "{encoded}");
    {
        let doc = p.document_mut(&id).unwrap();
        doc.content = RichDocument::from_plain_text("click me", "body");
        doc.content.set_link(0..8, Some(hostile));
    }
    let html = html_manuscript(&p, &s);
    assert!(!html.contains(r#"onmouseover=""#) && !html.contains("<script>"), "{html}");
    assert!(html.contains(r#"href="https://x.test/%22%20onmouseover=%22alert%281%29%22%20data-x=%271%27%3E%3Cscript%3E""#), "{html}");
    let md = markdown_manuscript(&p, &m);
    assert!(!md.contains(" onmouseover") && !md.contains('<'), "{md}");
}

#[test]
fn the_hyperlinks_option_controls_links_in_every_format_that_can_carry_them() {
    let mut p = Project::new("Links", ProjectKind::Novel);
    let id = p.add_document(None, "Chapter", DocKind::Chapter, Medium::Prose);
    {
        let doc = p.document_mut(&id).unwrap();
        doc.content = RichDocument::from_plain_text("See the chart for more.", "body");
        doc.content.set_link(4..13, Some("https://example.org/chart?a=1&b=2"));
    }
    let carries = |kind: ExportKind, text: &str| match kind {
        ExportKind::Html | ExportKind::Epub => text.contains(r#"<a href="https://example.org/chart?a=1&amp;b=2">the chart</a>"#),
        ExportKind::Markdown => text.contains("[the chart](https://example.org/chart?a=1&b=2)"),
        ExportKind::Docx => text.contains(r#"HYPERLINK &quot;https://example.org/chart?a=1&amp;b=2&quot;"#),
        _ => false,
    };
    for kind in [ExportKind::Html, ExportKind::Epub, ExportKind::Markdown, ExportKind::Docx] {
        assert!(kind.writes_hyperlinks());
        let mut s = p.settings_for(kind);
        s.compile_type = Some(CompileType::Prose);
        s.include_title_page = false;
        s.include_toc = false;
        assert!(s.include_hyperlinks, "{kind:?}: on by default");
        let with = match kind {
            ExportKind::Docx => docx_document_xml(docx_manuscript(&p, &s)),
            ExportKind::Epub => epub_file(&epub_manuscript(&p, &s), "OEBPS/chapter-001.xhtml")
                .to_string(),
            _ => output_text(kind, &p, &s),
        };
        assert!(carries(kind, &with), "{kind:?} should carry the link: {with}");
        assert!(with.contains("the chart"), "{kind:?}: the words are there either way");

        s.include_hyperlinks = false;
        let without = match kind {
            ExportKind::Docx => docx_document_xml(docx_manuscript(&p, &s)),
            ExportKind::Epub => epub_file(&epub_manuscript(&p, &s), "OEBPS/chapter-001.xhtml").to_string(),
            _ => output_text(kind, &p, &s),
        };
        assert!(!carries(kind, &without) && !without.contains("example.org"), "{kind:?} should print plain text: {without}");
        assert!(without.contains("the chart"));
    }
    for kind in [ExportKind::Pdf, ExportKind::Txt, ExportKind::Fountain, ExportKind::Fdx] {
        assert!(!kind.writes_hyperlinks(), "{kind:?} has no links to write yet");
    }
}

#[test]
fn mirrored_margins_swap_the_text_block_on_left_hand_pages() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Pdf);
    s.paper = PaperSize::Trade5x8;
    s.margins = Some(PageMargins { left: 80.0, right: 30.0, top: 54.0, bottom: 54.0 });
    s.mirror_margins = true;
    s.header_footer.page_numbers = false;
    let layout = pdf_layout(&p, &s);
    assert!(layout.pages.len() >= 4);
    let body_left = |page: usize| {
        layout.pages[page]
            .items
            .iter()
            .filter(|t| t.source.is_some())
            .map(|t| t.x)
            .fold(f32::MAX, f32::min)
    };
    // Right-hand pages start at the inside margin; left-hand pages at the outside.
    assert!((body_left(2) - 80.0).abs() < 0.5, "recto at {}", body_left(2));
    assert!((body_left(3) - 30.0).abs() < 0.5, "verso at {}", body_left(3));
    // Same page count as without mirroring: the block only moves.
    s.mirror_margins = false;
    assert_eq!(pdf_layout(&p, &s).pages.len(), layout.pages.len());
}

#[test]
fn docx_carries_mirrored_margins() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Docx);
    s.mirror_margins = true;
    let docx = docx_manuscript(&p, &s);
    assert!(epub_file(&docx, "word/settings.xml").contains("<w:mirrorMargins/>"));
    s.mirror_margins = false;
    assert!(!epub_file(&docx_manuscript(&p, &s), "word/settings.xml").contains("mirrorMargins"));
}

#[test]
fn custom_paper_uses_its_own_size_and_book_trims_are_offered() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Docx);
    s.paper = PaperSize::Custom;
    s.custom_paper_mm = (127.0, 203.2);
    let xml = docx_document_xml(docx_manuscript(&p, &s));
    assert!(xml.contains("<w:pgSz w:w=\"7200\" w:h=\"11520\"/>"), "{xml}");
    assert!(PaperSize::all().contains(&PaperSize::Trade5_5x8_5));
}

#[test]
fn epub_uses_the_project_cover_unless_the_format_has_its_own_or_it_is_switched_off() {
    let mut p = demo();
    let dir = std::env::temp_dir().join(portmanteau_core::new_id("project-cover"));
    fs::create_dir_all(dir.join("assets/images")).unwrap();
    fs::write(dir.join("assets/images/cover.png"), TINY_PNG).unwrap();
    p.cover_image = "assets/images/cover.png".into();
    let has_cover = |settings: &ExportSettings| {
        let bytes = epub_manuscript(&p, settings);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let found = (0..archive.len())
            .any(|i| archive.by_index(i).unwrap().name() == "OEBPS/cover-image/cover.png");
        found
    };
    let mut settings = p.settings_for(ExportKind::Epub);
    // A project-relative path needs the folder the project lives in.
    assert!(!has_cover(&settings));
    settings.asset_root = Some(dir.clone());
    assert!(has_cover(&settings), "the project cover reaches the EPUB");
    settings.include_cover = false;
    assert!(!has_cover(&settings), "switched off");
    settings.include_cover = true;
    settings.cover_image = dir.join("missing.png").display().to_string();
    assert!(!has_cover(&settings), "the format's own picture wins, even when it is missing");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn docx_headers_and_footers_use_word_fields() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Docx);
    s.header_footer.body.odd.header = ["{author}".into(), "".into(), "{chapter}".into()];
    s.header_footer.different_odd_even = true;
    s.header_footer.body.even.footer[1] = "{page} of {pages}".into();
    s.header_footer.different_first_page = true;
    let docx = docx_manuscript(&p, &s);
    let doc = epub_file(&docx, "word/document.xml");
    assert!(doc.contains("w:headerReference w:type=\"default\""));
    assert!(doc.contains("w:footerReference w:type=\"even\""));
    assert!(doc.contains("<w:titlePg/>"));
    assert!(epub_file(&docx, "word/settings.xml").contains("evenAndOddHeaders"));
    let header = epub_file(&docx, "word/header1.xml");
    assert!(header.contains("<w:hdr "));
    assert!(header.contains("STYLEREF"));
    let even_footer = epub_file(&docx, "word/footer2.xml");
    assert!(even_footer.contains(" PAGE ") && even_footer.contains(" NUMPAGES "));
    assert!(epub_file(&docx, "[Content_Types].xml").contains("/word/header1.xml"));
    // No templates, no parts.
    let plain = docx_manuscript(&p, &{ let mut n = p.settings_for(ExportKind::Docx); n.header_footer.body = PageFurniture::default(); n });
    assert!(!epub_file(&plain, "word/document.xml").contains("headerReference"));
}

#[test]
fn a_project_cover_cannot_point_outside_the_project() {
    let dir = std::env::temp_dir().join(format!("pm-cover-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("assets/images")).unwrap();
    let secret = dir.join("secret.png");
    std::fs::write(&secret, [0x89, b'P', b'N', b'G', 1, 2, 3]).unwrap();
    std::fs::write(dir.join("assets/images/cover.png"), [0x89, b'P', b'N', b'G', 1]).unwrap();
    let mut p = demo();
    let mut s = p.settings_for(ExportKind::Epub);
    s.include_cover = true;
    s.asset_root = Some(dir.clone());
    p.cover_image = "assets/images/cover.png".into();
    assert!(epub::cover_image(&p, &s).is_some());
    for bad in ["../secret.png", secret.to_str().unwrap(), "assets/../secret.png"] {
        p.cover_image = bad.into();
        assert!(epub::cover_image(&p, &s).is_none(), "{bad} must be refused");
    }
    // A file that is not the picture its name says.
    std::fs::write(dir.join("assets/images/cover.png"), b"not a picture").unwrap();
    p.cover_image = "assets/images/cover.png".into();
    assert!(epub::cover_image(&p, &s).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A saved-looking project whose first chapter holds text, a picture and a caption.
fn picture_project(tag: &str) -> (Project, PathBuf) {
    let dir = std::env::temp_dir().join(format!("pm-pic-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("assets/images")).unwrap();
    std::fs::write(dir.join("assets/images/dot.png"), TINY_PNG).unwrap();
    let mut p = demo();
    let id = p
        .compile_plan(&p.settings_for(ExportKind::Pdf))
        .included
        .iter()
        .find(|d| !matches!(d.kind, DocKind::FrontMatter | DocKind::BackMatter))
        .unwrap()
        .id
        .clone();
    let text = |t: &str, style: &str| RichParagraph {
        style_id: style.into(),
        content: vec![InlineContent::Text(TextRun::with_marks(t, vec![]))],
    };
    p.document_mut(&id).unwrap().content = RichDocument {
        paragraphs: vec![
            text("Before the picture.", "body"),
            RichParagraph {
                style_id: "body".into(),
                content: vec![InlineContent::Image(ImageObject {
                    asset_id: "images/dot.png".into(),
                    alt_text: "A red dot".into(),
                    caption: String::new(),
                    width: Some(120.0),
                    height: Some(60.0),
..Default::default()
})],
            },
            text("After the picture.", "body"),
        ],
    };
    (p, dir)
}

#[test]
fn the_pdf_draws_a_picture_that_sits_on_its_own_line() {
    let (p, dir) = picture_project("pdf");
    let mut s = p.settings_for(ExportKind::Pdf);
    s.asset_root = Some(dir.clone());
    let layout = pdf_layout(&p, &s);
    let images: Vec<_> = layout.pages.iter().flat_map(|page| &page.images).collect();
    assert_eq!(images.len(), 1);
    assert!((images[0].width - 120.0).abs() < 0.01 && (images[0].height - 60.0).abs() < 0.01);
    let bytes = pdf_manuscript(&p, &s);
    assert!(bytes.windows(14).any(|w| w == b"/Subtype/Image" || w == b"/Subtype /Imag"), "the file embeds a picture");
    // Without the project folder the same text prints the placeholder instead.
    s.asset_root = None;
    let layout = pdf_layout(&p, &s);
    assert!(layout.pages.iter().all(|page| page.images.is_empty()));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Writes a picture-bearing PDF/DOCX/EPUB for a human to look at. Run with
/// `PM_DUMP=/some/dir cargo test -p portmanteau-export dump_picture_exports -- --ignored`
/// and `PM_PICTURE=/path/to/picture.png`.
#[test]
#[ignore]
fn dump_picture_exports() {
    let (Ok(out), Ok(picture)) = (std::env::var("PM_DUMP"), std::env::var("PM_PICTURE")) else {
        return;
    };
    let (mut p, dir) = wrapped_picture_project("dump", PictureAlign::Left);
    std::fs::copy(&picture, dir.join("assets/images/dot.png")).unwrap();
    for doc in p.documents.iter_mut() {
        for para in doc.content.paragraphs.iter_mut() {
            for item in para.content.iter_mut() {
                if let InlineContent::Image(i) = item {
                    i.width = Some(160.0);
                    i.height = None;
                }
            }
        }
    }
    for (kind, name) in [(ExportKind::Pdf, "pic.pdf"), (ExportKind::Docx, "pic.docx"), (ExportKind::Epub, "pic.epub"), (ExportKind::Html, "pic.html")] {
        let mut s = p.settings_for(kind);
        s.asset_root = Some(dir.clone());
        let bytes = match kind {
            ExportKind::Pdf => pdf_manuscript(&p, &s),
            ExportKind::Docx => docx_manuscript(&p, &s),
            ExportKind::Epub => epub_manuscript(&p, &s),
            _ => html_manuscript(&p, &s).into_bytes(),
        };
        std::fs::write(std::path::Path::new(&out).join(name), bytes).unwrap();
    }
}

#[test]
fn every_prose_writer_carries_a_picture_that_sits_on_its_own_line() {
    let (p, dir) = picture_project("writers");
    let with_root = |kind| {
        let mut s = p.settings_for(kind);
        s.asset_root = Some(dir.clone());
        s
    };

    let docx = docx_manuscript(&p, &with_root(ExportKind::Docx));
    assert!(epub_file(&docx, "word/document.xml").contains("r:embed=\"rIdImg1\""));
    assert!(epub_file(&docx, "word/_rels/document.xml.rels").contains("media/img1.png"));
    assert!(epub_file(&docx, "[Content_Types].xml").contains("Extension=\"png\""));
    assert!(epub_file(&docx, "word/document.xml").contains("descr=\"A red dot\""));

    let epub = epub_manuscript(&p, &with_root(ExportKind::Epub));
    assert!(epub_file(&epub, "OEBPS/content.opf").contains("images/img1.png"));

    let html = html_manuscript(&p, &with_root(ExportKind::Html));
    assert!(html.contains("<img src=\"data:image/png;base64,"));
    assert!(html.contains("alt=\"A red dot\""));

    // No project folder, or a picture outside assets/: the placeholder stays.
    let mut s = p.settings_for(ExportKind::Html);
    s.asset_root = None;
    assert!(html_manuscript(&p, &s).contains("[Image: A red dot]"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_caption_prints_under_its_picture_in_every_writer() {
    let (mut p, dir) = picture_project("caption");
    for doc in p.documents.iter_mut() {
        for para in doc.content.paragraphs.iter_mut() {
            for item in para.content.iter_mut() {
                if let InlineContent::Image(image) = item {
                    image.caption = "Figure one: a red dot".into();
                }
            }
        }
    }
    let with_root = |kind| {
        let mut s = p.settings_for(kind);
        s.asset_root = Some(dir.clone());
        s
    };
    assert!(html_manuscript(&p, &with_root(ExportKind::Html)).contains("Figure one: a red dot"));
    let docx = docx_manuscript(&p, &with_root(ExportKind::Docx));
    assert!(epub_file(&docx, "word/document.xml").contains("Figure one: a red dot"));
    let layout = pdf_layout(&p, &with_root(ExportKind::Pdf));
    let text: String = layout
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| format!("{item:?}"))
        .collect();
    assert!(text.contains("Figure one"), "the PDF lays the caption out");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A project whose picture sits at the left, wrapped, followed by a long paragraph.
fn wrapped_picture_project(tag: &str, align: PictureAlign) -> (Project, PathBuf) {
    let (mut p, dir) = picture_project(tag);
    let long = "The tide came in over the flats and the gulls wheeled and cried above the harbour wall while the boats rocked at their moorings. ".repeat(14);
    for doc in p.documents.iter_mut() {
        let has_picture = doc.content.paragraphs.iter().any(|para| para.has_image());
        if !has_picture {
            continue;
        }
        for para in doc.content.paragraphs.iter_mut() {
            for item in para.content.iter_mut() {
                if let InlineContent::Image(image) = item {
                    image.wrap = true;
                    image.align = align;
                    image.padding = 12.0;
                    image.width = Some(150.0);
                    image.height = Some(100.0);
                    image.caption = "A red dot".into();
                }
            }
        }
        doc.content.paragraphs.push(RichParagraph {
            style_id: "body".into(),
            content: vec![InlineContent::Text(TextRun::with_marks(long.clone(), vec![]))],
        });
    }
    (p, dir)
}

#[test]
fn the_pdf_runs_text_beside_a_wrapped_picture_and_below_it_in_full() {
    for (align, tag) in [(PictureAlign::Left, "wl"), (PictureAlign::Right, "wr")] {
        let (p, dir) = wrapped_picture_project(tag, align);
        let mut s = p.settings_for(ExportKind::Pdf);
        s.asset_root = Some(dir.clone());
        let layout = pdf_layout(&p, &s);
        let page = layout.pages.iter().find(|page| !page.images.is_empty()).expect("a page with the picture");
        let image = &page.images[0];
        let bottom = image.y + image.height + 12.0;
        let beside: Vec<_> = page
            .items
            .iter()
            .filter(|t| t.baseline > image.y + 2.0 && t.baseline < image.y + image.height - 2.0)
            .collect();
        assert!(!beside.is_empty(), "text sits beside the picture");
        for t in &beside {
            let width: f32 = t.segments.iter().map(|seg| text_width(&seg.text, seg.face, t.size)).sum();
            if align == PictureAlign::Left {
                assert!(t.x >= image.x + image.width + 11.0, "text starts right of a left picture: {} vs {}", t.x, image.x + image.width);
            } else {
                assert!(t.x + width <= image.x - 11.0 + 0.5, "text ends left of a right picture");
            }
        }
        let below: Vec<_> = page.items.iter().filter(|t| t.baseline > bottom + 20.0).collect();
        assert!(below.iter().any(|t| (t.x - beside.iter().map(|b| b.x).fold(f32::MAX, f32::min)).abs() > 1.0 || align == PictureAlign::Right), "text below the picture uses the full width again");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_wrapped_picture_is_anchored_in_docx_and_floated_in_html() {
    let (p, dir) = wrapped_picture_project("wd", PictureAlign::Right);
    let with_root = |kind| {
        let mut s = p.settings_for(kind);
        s.asset_root = Some(dir.clone());
        s
    };
    let docx = docx_manuscript(&p, &with_root(ExportKind::Docx));
    let xml = epub_file(&docx, "word/document.xml");
    assert!(xml.contains("<wp:anchor") && xml.contains("<wp:wrapSquare") && xml.contains("<wp:align>right</wp:align>"));
    assert!(xml.contains("distL=\"152400\""), "12 pt of padding on the text side");
    let html = html_manuscript(&p, &with_root(ExportKind::Html));
    assert!(html.contains("float:right") && html.contains("<figcaption"));
    assert_eq!(html.matches("A red dot</figcaption>").count(), 1, "the caption prints once");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_pdf_opens_with_the_cover_as_an_uncounted_first_page() {
    let (mut p, dir) = picture_project("cover");
    std::fs::write(dir.join("assets/cover.png"), TINY_PNG).unwrap();
    p.cover_image = "assets/cover.png".into();
    let mut s = p.settings_for(ExportKind::Pdf);
    s.asset_root = Some(dir.clone());
    s.include_title_page = true;
    s.include_cover = false;
    let without = pdf_layout(&p, &s);
    s.include_cover = true;
    let with = pdf_layout(&p, &s);
    assert_eq!(with.pages.len(), without.pages.len() + 1);
    assert!(with.pages[0].items.is_empty() && with.pages[0].images.len() == 1, "page one is the picture alone");
    assert!((with.pages[0].images[0].width / with.pages[0].images[0].height - 1.0).abs() < 0.01 || with.pages[0].images[0].width <= with.width + 0.01);
    let first_text = |layout: &PdfLayout, page: usize| layout.pages[page].items.iter().flat_map(|i| i.segments.iter().map(|g| g.text.clone())).collect::<String>();
    assert_eq!(first_text(&with, 1), first_text(&without, 0), "the title page follows the cover unchanged");
    assert_eq!(with.sections[0].label, "Cover");
    // Without the project folder (an unsaved project) there is no cover to read.
    s.asset_root = None;
    assert_eq!(pdf_layout(&p, &s).pages.len(), without.pages.len());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_contents_heading_page_numbers_and_leaders_follow_the_settings() {
    let p = demo();
    let text_of = |layout: &PdfLayout| -> Vec<String> {
        layout
            .pages
            .iter()
            .flat_map(|page| page.items.iter())
            .map(|item| item.segments.iter().map(|g| g.text.clone()).collect::<String>())
            .collect()
    };
    let mut s = p.settings_for(ExportKind::Pdf);
    s.include_toc = true;
    s.toc_title = "Table of Contents".into();
    let plain = text_of(&pdf_layout(&p, &s));
    assert!(plain.iter().any(|t| t == "Table of Contents") && !plain.iter().any(|t| t == "Contents"));
    assert!(!plain.iter().any(|t| t.starts_with(". .")), "no leaders unless asked for");
    s.toc_leaders = true;
    assert!(text_of(&pdf_layout(&p, &s)).iter().any(|t| t.starts_with(". . .")), "dots join title and number");
    s.toc_page_numbers = false;
    assert!(!text_of(&pdf_layout(&p, &s)).iter().any(|t| t.starts_with(". .")), "no numbers, no leaders");

    if let Ok(out) = std::env::var("PM_TOC") {
        s.toc_page_numbers = true;
        std::fs::write(out, pdf_manuscript(&p, &s)).unwrap();
    }
    let mut d = p.settings_for(ExportKind::Docx);
    d.include_toc = true;
    d.toc_title = "In This Book".into();
    assert!(epub_file(&docx_manuscript(&p, &d), "word/document.xml").contains("In This Book"));
    let mut h = p.settings_for(ExportKind::Html);
    h.include_toc = true;
    h.toc_title = "In This Book".into();
    assert!(html_manuscript(&p, &h).contains(">In This Book</h2>"));
}

#[test]
fn a_picture_cannot_be_read_from_outside_the_assets_folder() {
    let (_, dir) = picture_project("escape");
    std::fs::write(dir.join("outside.png"), TINY_PNG).unwrap();
    assert!(load_picture(Some(&dir), "images/dot.png").is_some());
    for bad in ["../outside.png", "images/../../outside.png", "/etc/passwd"] {
        assert!(load_picture(Some(&dir), bad).is_none(), "{bad}");
    }
    // Not a picture, whatever its name says.
    std::fs::write(dir.join("assets/images/fake.png"), b"just text").unwrap();
    assert!(load_picture(Some(&dir), "images/fake.png").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_picture_is_scaled_to_the_page_and_keeps_its_proportions() {
    let picture = LoadedPicture { bytes: vec![], ext: "png", media: "image/png", px: (600, 400) };
    let mut image = ImageObject { asset_id: "x".into(), alt_text: String::new(), caption: String::new(), width: None, height: None, ..Default::default() };
    // Natural size is 96 dpi: 450 x 300 pt, scaled down to a 225 pt measure.
    assert_eq!(picture_size_pt(&image, &picture, 225.0, 1000.0), (225.0, 150.0));
    image.width = Some(100.0);
    let (w, h) = picture_size_pt(&image, &picture, 225.0, 1000.0);
    assert!((w - 100.0).abs() < 0.01 && (h - 66.667).abs() < 0.01);
    image.height = Some(80.0);
    assert_eq!(picture_size_pt(&image, &picture, 225.0, 1000.0), (100.0, 80.0));
}

#[test]
fn chapters_can_open_on_right_hand_pages() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Pdf);
    s.chapters_start_right = true;
    let layout = pdf_layout(&p, &s);
    let openers: Vec<usize> = layout
        .sections
        .iter()
        .filter(|section| {
            section.doc_id.as_deref().and_then(|id| p.document(id)).is_some_and(|d| !matches!(d.kind, DocKind::FrontMatter | DocKind::BackMatter) && !d.kind.is_folder())
        })
        .map(|section| section.start_page)
        .collect();
    assert!(openers.len() >= 2);
    assert!(openers.iter().all(|page| page % 2 == 0), "0-based even index = right-hand page: {openers:?}");
    s.chapters_start_right = false;
    assert!(pdf_layout(&p, &s).page_count() <= layout.page_count());
}

#[cfg(unix)]
#[test]
fn a_symlinked_picture_or_folder_is_not_read() {
    let (_, dir) = picture_project("links");
    let outside = std::env::temp_dir().join(format!("pm-outside-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.png"), TINY_PNG).unwrap();
    std::os::unix::fs::symlink(outside.join("secret.png"), dir.join("assets/images/link.png")).unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("assets/elsewhere")).unwrap();
    assert!(load_picture(Some(&dir), "images/link.png").is_none());
    assert!(load_picture(Some(&dir), "elsewhere/secret.png").is_none());
    assert!(load_picture(Some(&dir), "images/dot.png").is_some());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn a_picture_that_declares_a_huge_size_is_refused() {
    let (_, dir) = picture_project("huge");
    let mut png = TINY_PNG.to_vec();
    png[16..20].copy_from_slice(&40_000u32.to_be_bytes());
    png[20..24].copy_from_slice(&40_000u32.to_be_bytes());
    std::fs::write(dir.join("assets/images/huge.png"), png).unwrap();
    assert!(load_picture(Some(&dir), "images/huge.png").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
