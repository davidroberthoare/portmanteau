use super::*;

fn demo() -> Project {
    portmanteau_core::sample::demo()
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
fn epub_css_honours_body_alignment_and_first_line_indent() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Epub);
    s.styles.body.align = Alignment::Center;
    s.styles.body.first_line_indent = 21.0;
    s.styles.chapter_heading.align = Alignment::Right;
    let bytes = epub_manuscript(&p, &s);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut css = String::new();
    std::io::Read::read_to_string(&mut archive.by_name("OEBPS/style.css").unwrap(), &mut css)
        .unwrap();
    assert!(css.contains("text-indent: 21.00pt"), "EPUB CSS: {css}");
    assert!(css.contains("text-align: center"), "EPUB CSS: {css}");
    assert!(css.contains("text-align: right"), "EPUB CSS: {css}");
}

#[test]
fn html_css_honours_body_alignment_and_first_line_indent() {
    let p = demo();
    let mut s = p.settings_for(ExportKind::Html);
    s.styles.body.align = Alignment::Justify;
    s.styles.body.first_line_indent = 17.0;
    let html = html_manuscript(&p, &s);
    assert!(html.contains("text-indent:17.0pt"), "HTML CSS: {html}");
    assert!(html.contains("text-align:justify"), "HTML CSS: {html}");
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
    assert!(
        html.lines().any(|line| line.contains("A centred epigraph.")
            && line.contains("text-align: center")
            && line.contains("font-size: 9.0pt")),
        "custom paragraph style should reach HTML: {html}"
    );
    assert!(
        !html.contains("style=\"font-family: \""),
        "font names inside a style attribute must not close the attribute"
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
    assert!(
        html.lines().any(|line| line.contains("The stylesheet is the source of truth.")
            && line.contains("font-size: 13.0pt")
            && line.contains("text-align: right")),
        "project sheet edit should win over a stale seeded format: {html}"
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
