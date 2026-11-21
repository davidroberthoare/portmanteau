use super::*;

/// A bundled face carried inside the EPUB. The reader may ignore the CSS
/// family when the writer chooses their own, but every fallback in the stack
/// then resolves to the same Liberation face the editor draws with.
pub(crate) struct EpubFont {
    file: &'static str,
    family: &'static str,
    style: &'static str,
    weight: &'static str,
    bytes: &'static [u8],
}

pub(crate) const EPUB_FONTS: [EpubFont; 9] = [
    EpubFont {
        file: "LiberationSerif-Regular.ttf",
        family: "Liberation Serif",
        style: "normal",
        weight: "normal",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/LiberationSerif-Regular.ttf"
        )),
    },
    EpubFont {
        file: "LiberationSerif-Bold.ttf",
        family: "Liberation Serif",
        style: "normal",
        weight: "bold",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/LiberationSerif-Bold.ttf"
        )),
    },
    EpubFont {
        file: "LiberationSerif-Italic.ttf",
        family: "Liberation Serif",
        style: "italic",
        weight: "normal",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/LiberationSerif-Italic.ttf"
        )),
    },
    EpubFont {
        file: "LiberationSans-Regular.ttf",
        family: "Liberation Sans",
        style: "normal",
        weight: "normal",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/LiberationSans-Regular.ttf"
        )),
    },
    EpubFont {
        file: "LiberationSans-Bold.ttf",
        family: "Liberation Sans",
        style: "normal",
        weight: "bold",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/LiberationSans-Bold.ttf"
        )),
    },
    EpubFont {
        file: "LiberationSans-Italic.ttf",
        family: "Liberation Sans",
        style: "italic",
        weight: "normal",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/LiberationSans-Italic.ttf"
        )),
    },
    EpubFont {
        file: "LiberationMono-Regular.ttf",
        family: "Liberation Mono",
        style: "normal",
        weight: "normal",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/LiberationMono-Regular.ttf"
        )),
    },
    EpubFont {
        file: "LiberationMono-Bold.ttf",
        family: "Liberation Mono",
        style: "normal",
        weight: "bold",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/LiberationMono-Bold.ttf"
        )),
    },
    EpubFont {
        file: "LiberationMono-Italic.ttf",
        family: "Liberation Mono",
        style: "italic",
        weight: "normal",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/LiberationMono-Italic.ttf"
        )),
    },
];

pub(crate) fn epub_font_css() -> String {
    let mut out = String::new();
    for font in &EPUB_FONTS {
        out.push_str(&format!(
            "@font-face {{ font-family: \"{}\"; src: url(\"fonts/{}\") format(\"truetype\"); font-style: {}; font-weight: {}; }}\n",
            font.family, font.file, font.style, font.weight
        ));
    }
    out
}

pub(crate) fn epub_font_id(file: &str) -> String {
    format!("font-{}", file.trim_end_matches(".ttf"))
}

/// Resolve an optional EPUB cover. A missing or unreadable file is not an
/// export error: the book is still valid without its cover.
pub(crate) fn cover_image(settings: &ExportSettings) -> Option<(String, &'static str, Vec<u8>)> {
    let raw = settings.cover_image.trim();
    if raw.is_empty() {
        return None;
    }
    let path = Path::new(raw);
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let media = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        _ => return None,
    };
    let bytes = fs::read(path).ok()?;
    Some((ext, media, bytes))
}

pub(crate) fn epub_manuscript(project: &Project, settings: &ExportSettings) -> Vec<u8> {
    let mut buf = Vec::new();
    let cover = cover_image(settings);
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        if let Some((ext, _, bytes)) = &cover {
            zip.start_file(format!("OEBPS/cover-image/cover.{}", ext), deflated)
                .unwrap();
            zip.write_all(bytes).unwrap();
        }

        zip.start_file("META-INF/container.xml", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/style.css", deflated).unwrap();
        let st = &settings.styles;
        let body_font = element_css_font(&st.body, "Liberation Serif");
        let scene_font = element_css_font(&st.scene_heading, "Courier New");
        let action_font = element_css_font(&st.action, "Courier New");
        let character_font = element_css_font(&st.character, "Courier New");
        let parenthetical_font = element_css_font(&st.parenthetical, "Courier New");
        let dialogue_font = element_css_font(&st.dialogue, "Courier New");
        let transition_font = element_css_font(&st.transition, "Courier New");
        let character_indent = element_css_indent(&st.character, "32%");
        let parenthetical_indent = element_css_indent(&st.parenthetical, "26%");
        let dialogue_indent = element_css_indent(&st.dialogue, "18%");
        let body_align = css_align(st.body.align);
        let heading_align = css_align(st.chapter_heading.align);
        let body_margins = css_indent_margins(&st.body);
        let heading_margins = css_indent_margins(&st.chapter_heading);
        let title_align = css_align(st.title_page_title.align);
        let subtitle_align = css_align(st.title_page_subtitle.align);
        let author_align = css_align(st.title_page_author.align);
        let body_first_indent = st.body.first_line_indent.max(0.0);
        let css = format!(
            r#"{font_faces}body {{ font-family: {body_font}; line-height: {body_leading:.2}; margin: 5%; {body_margins}text-align: {body_align}; color: #1b1b1b; }}
h1 {{ font-size: {chapter_size:.1}pt; margin: 2em 0 1em; page-break-before: always; text-align: {heading_align}; {heading_margins}}}
h2 {{ font-size: 1.25em; margin: 1.5em 0 .8em; text-align: {heading_align}; }}
p {{ margin: 0 0 0 .2em; text-indent: {body_first_indent:.2}pt; text-align: {body_align}; }}
p.first {{ text-indent: 0; }}
.center {{ text-align: center; text-indent: 0; }}
.scene {{ font-family: {scene_font}; font-size: {scene_size:.1}pt; font-weight: bold; margin-top: 1.5em; text-indent: 0; }}
.action {{ font-family: {action_font}; font-size: {action_size:.1}pt; margin: .5em 0 .8em; text-indent: 0; }}
.character {{ font-family: {character_font}; font-size: {character_size:.1}pt; margin: 1em 0 0 {character_indent}; text-indent: 0; }}
.parenthetical {{ font-family: {parenthetical_font}; font-size: {parenthetical_size:.1}pt; margin: 0 0 0 {parenthetical_indent}; text-indent: 0; }}
.dialogue {{ font-family: {dialogue_font}; font-size: {dialogue_size:.1}pt; margin: 0 10% .7em {dialogue_indent}; text-indent: 0; }}
.transition {{ font-family: {transition_font}; font-size: {transition_size:.1}pt; text-align: right; text-indent: 0; }}
.title-page {{ text-align: center; margin-top: 26%; }}
.title-page h1 {{ page-break-before: avoid; font-size: {title_size:.1}pt; text-align: {title_align}; }}
.title-page .subtitle {{ font-size: {subtitle_size:.1}pt; text-align: {subtitle_align}; }}
.byline {{ font-size: {author_size:.1}pt; margin-top: 1.5em; text-align: {author_align}; }}
.imprint {{ margin-top: 4em; font-size: .85em; color: #555; }}
.cover {{ text-align: center; margin: 0; padding: 0; }}
.cover img {{ max-width: 100%; }}
"#,
            font_faces = epub_font_css(),
            body_font = body_font,
            body_leading = st.body.leading.max(1.0),
            body_align = body_align,
            body_margins = body_margins,
            body_first_indent = body_first_indent,
            heading_align = heading_align,
            heading_margins = heading_margins,
            title_align = title_align,
            subtitle_align = subtitle_align,
            author_align = author_align,
            chapter_size = st.chapter_heading.size.max(8.0),
            scene_font = scene_font,
            scene_size = st.scene_heading.size.max(8.0),
            action_font = action_font,
            action_size = st.action.size.max(8.0),
            character_font = character_font,
            character_size = st.character.size.max(8.0),
            character_indent = character_indent,
            parenthetical_font = parenthetical_font,
            parenthetical_size = st.parenthetical.size.max(8.0),
            parenthetical_indent = parenthetical_indent,
            dialogue_font = dialogue_font,
            dialogue_size = st.dialogue.size.max(8.0),
            dialogue_indent = dialogue_indent,
            transition_font = transition_font,
            transition_size = st.transition.size.max(8.0),
            title_size = st.title_page_title.size.max(8.0),
            subtitle_size = st.title_page_subtitle.size.max(8.0),
            author_size = st.title_page_author.size.max(8.0),
        );
        zip.write_all(css.as_bytes()).unwrap();

        let mut manifest = String::new();
        for font in &EPUB_FONTS {
            zip.start_file(format!("OEBPS/fonts/{}", font.file), deflated)
                .unwrap();
            zip.write_all(font.bytes).unwrap();
            manifest.push_str(&format!(
                "<item id=\"{}\" href=\"fonts/{}\" media-type=\"font/ttf\"/>\n",
                epub_font_id(font.file),
                font.file
            ));
        }
        let mut spine = String::new();
        let mut toc_entries = String::new();
        let mut metadata_extra = String::new();
        let mut files: Vec<(String, String, String)> = Vec::new();

        if let Some((ext, media, _)) = &cover {
            let body = format!(
                "<div class=\"cover\"><img src=\"cover-image/cover.{}\" alt=\"cover\"/></div>",
                ext
            );
            let xhtml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"en\">\n<head><title>Cover</title><link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/></head>\n<body class=\"front\">\n{}\n</body>\n</html>\n",
                body
            );
            zip.start_file("OEBPS/cover.xhtml", deflated).unwrap();
            zip.write_all(xhtml.as_bytes()).unwrap();
            manifest.push_str(
                "<item id=\"cover\" href=\"cover.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
            );
            manifest.push_str(&format!(
                "<item id=\"cover-img\" href=\"cover-image/cover.{}\" media-type=\"{}\" properties=\"cover-image\"/>\n",
                ext, media
            ));
            spine.push_str("<itemref idref=\"cover\"/>\n");
            metadata_extra.push_str("<meta name=\"cover\" content=\"cover-img\"/>\n");
        }

        if settings.include_title_page {
            let body = format!(
                "<div class=\"title-page\"><h1>{}</h1><p class=\"center subtitle\">{}</p><p class=\"byline\">by {}</p><p class=\"imprint\">{}</p></div>",
                xml_escape(&project.title),
                xml_escape(&project.subtitle),
                xml_escape(&project.author),
                xml_escape(&project.genre)
            );
            files.push(("titlepage.xhtml".into(), "Title page".into(), body));
        }

        let mut chapter_no = 0usize;
        for doc in front_matter(project, settings) {
            files.push((
                format!("front-{}.xhtml", slugify(&doc.title)),
                doc.title.clone(),
                xhtml_body(project, doc, None, settings),
            ));
        }
        for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
            chapter_no += 1;
            let title = section_title(project, section.doc, i, settings);
            files.push((
                format!("chapter-{:03}.xhtml", chapter_no),
                title.clone(),
                xhtml_body(project, section.doc, Some(&title), settings),
            ));
        }
        for doc in back_matter(project, settings) {
            files.push((
                format!("back-{}.xhtml", slugify(&doc.title)),
                doc.title.clone(),
                xhtml_body(project, doc, None, settings),
            ));
        }

        for (i, (file, title, body)) in files.iter().enumerate() {
            zip.start_file(format!("OEBPS/{}", file), deflated).unwrap();
            let xhtml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"en\">\n<head><title>{}</title><link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/></head>\n<body class=\"front\">\n{}\n</body>\n</html>\n",
                xml_escape(title),
                body
            );
            zip.write_all(xhtml.as_bytes()).unwrap();
            manifest.push_str(&format!(
                "<item id=\"f{}\" href=\"{}\" media-type=\"application/xhtml+xml\"/>\n",
                i, file
            ));
            spine.push_str(&format!("<itemref idref=\"f{}\"/>\n", i));
            if i > 0 {
                toc_entries.push_str(&format!(
                    "<li><a href=\"{}\">{}</a></li>\n",
                    file,
                    xml_escape(title)
                ));
            }
        }

        // Navigation document.
        let nav = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n<head><title>Contents</title></head><body>\n<nav epub:type=\"toc\" id=\"toc\"><h1>Contents</h1><ol>{}</ol></nav>\n</body></html>",
            toc_entries
        );
        zip.start_file("OEBPS/nav.xhtml", deflated).unwrap();
        zip.write_all(nav.as_bytes()).unwrap();
        manifest.push_str("<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n");
        manifest.push_str("<item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\n");

        let modified = chrono::Local::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let opf = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\">\n<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n<dc:identifier id=\"bookid\">{}</dc:identifier>\n<dc:title>{}</dc:title>\n<dc:creator>{}</dc:creator>\n<dc:language>en</dc:language>\n<dc:description>{}</dc:description>\n<meta property=\"dcterms:modified\">{}</meta>\n{}</metadata>\n<manifest>\n{}</manifest>\n<spine>\n{}</spine>\n</package>\n",
            xml_escape(&project.id),
            xml_escape(&project.title),
            xml_escape(&project.author),
            xml_escape(&project.logline),
            modified,
            metadata_extra,
            manifest,
            spine
        );
        zip.start_file("OEBPS/content.opf", deflated).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    buf
}

pub(crate) fn xhtml_body(
    project: &Project,
    doc: &Document,
    heading: Option<&str>,
    settings: &ExportSettings,
) -> String {
    let mut out = String::new();
    if let Some(h) = heading {
        out.push_str(&format!("<h1>{}</h1>\n", xml_escape(h)));
    } else {
        out.push_str(&format!("<h2>{}</h2>\n", xml_escape(&doc.title)));
    }
    if doc.medium.is_script() {
        let blocks = screenplay_blocks(project, doc);
        for block in &blocks {
            let class = match block.kind {
                ElementKind::SceneHeading => "scene",
                ElementKind::Action | ElementKind::Shot => "action",
                ElementKind::Character | ElementKind::Extension => "character",
                ElementKind::Parenthetical => "parenthetical",
                ElementKind::Dialogue => "dialogue",
                ElementKind::Transition => "transition",
                ElementKind::Act => "scene",
                ElementKind::Scene => "scene",
                ElementKind::Intercut => "trans",
                ElementKind::Subheading => "scene",
                ElementKind::StageDirection => "action",
                ElementKind::Lyric => "dialogue",
                ElementKind::General => "action",
            };
            let text = if settings.styles.style_for_kind(block.kind).uppercase {
                block.text.to_uppercase()
            } else {
                block.text.clone()
            };
            out.push_str(&format!(
                "<p class=\"{}\">{}</p>\n",
                class,
                html_escape(&text)
            ));
        }
    } else {
        for para in prose_paragraphs(project, doc, settings).iter() {
            out.push_str(&html_prose_paragraph(para));
        }
    }
    out
}
