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

/// `@font-face` rules for the embedded faces only.
fn font_face_css(fonts: &[&EpubFont]) -> String {
    fonts
        .iter()
        .map(|font| {
            format!(
                "@font-face {{ font-family: \"{}\"; src: url(\"fonts/{}\") format(\"truetype\"); font-style: {}; font-weight: {}; }}\n",
                font.family, font.file, font.style, font.weight
            )
        })
        .collect()
}

fn epub_font_id(file: &str) -> String {
    format!("font-{}", file.trim_end_matches(".ttf"))
}

/// Faces worth shipping: every face of each family the stylesheet names. A
/// book set in serif carries the three serif faces, not all nine.
fn used_fonts(css: &str) -> Vec<&'static EpubFont> {
    EPUB_FONTS
        .iter()
        .filter(|font| css.contains(&format!("\"{}\"", font.family)))
        .collect()
}

/// Resolve an optional EPUB cover: this format's own picture if it names one,
/// else the project's. A missing or unreadable file is not an export error:
/// the book is still valid without its cover.
pub(crate) fn cover_image(
    project: &Project,
    settings: &ExportSettings,
) -> Option<(String, &'static str, Vec<u8>)> {
    if !settings.include_cover {
        return None;
    }
    let own = settings.cover_image.trim();
    let from_project = own.is_empty();
    let raw = if from_project { project.cover_image.trim() } else { own };
    if raw.is_empty() {
        return None;
    }
    let mut path = std::path::PathBuf::from(raw);
    if from_project {
        if let Some(root) = &settings.asset_root {
            // A saved project's cover is its own copy under `assets/`. A path
            // read from a project file must stay inside that folder: no absolute
            // paths, no `..`, and nothing that a symlink leads out of it.
            if !path.is_relative()
                || path.components().any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return None;
            }
            let real_assets = fs::canonicalize(root.join("assets")).ok()?;
            let real = fs::canonicalize(root.join(&path)).ok()?;
            if !real.starts_with(&real_assets) {
                return None;
            }
            path = real;
        }
    } else if path.is_relative() {
        if let Some(root) = &settings.asset_root {
            path = root.join(path);
        }
    }
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let media = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        _ => return None,
    };
    const MAX_COVER_BYTES: u64 = 30 * 1024 * 1024;
    if fs::metadata(&path).ok()?.len() > MAX_COVER_BYTES {
        return None;
    }
    let bytes = fs::read(&path).ok()?;
    // The file must really be the picture its extension claims.
    let looks_right = match media {
        "image/png" => bytes.starts_with(&[0x89, b'P', b'N', b'G']),
        _ => bytes.starts_with(&[0xFF, 0xD8]),
    };
    if !looks_right {
        return None;
    }
    Some((ext, media, bytes))
}


// ---------------------------------------------------------------------------
// Package assembly
// ---------------------------------------------------------------------------

/// One XHTML page of the book.
struct Page {
    file: String,
    title: String,
    body: String,
    /// Listed in the reader's table of contents.
    in_nav: bool,
}

const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";

fn xhtml_document(title: &str, body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"{XHTML_NS}\" xml:lang=\"en\">\n\
         <head><title>{}</title><link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/></head>\n\
         <body>\n{body}</body>\n</html>\n",
        xml_escape(title),
    )
}

const CONTAINER_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n\
  <rootfiles><rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles>\n\
</container>";

/// The visible Contents page and its reader-menu twin share the same list.
fn contents_list(pages: &[Page]) -> String {
    pages
        .iter()
        .filter(|page| page.in_nav)
        .map(|page| format!("<li><a href=\"{}\">{}</a></li>\n", page.file, xml_escape(&page.title)))
        .collect()
}

fn nav_document(pages: &[Page]) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n\
         <html xmlns=\"{XHTML_NS}\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n\
         <head><title>Contents</title></head><body>\n\
         <nav epub:type=\"toc\" id=\"toc\"><h1>Contents</h1><ol>\n{}</ol></nav>\n</body></html>",
        contents_list(pages)
    )
}

fn package_document(
    project: &Project,
    manifest: &str,
    spine: &str,
    metadata_extra: &str,
) -> String {
    let modified = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\">\n\
         <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
         <dc:identifier id=\"bookid\">{}</dc:identifier>\n<dc:title>{}</dc:title>\n<dc:creator>{}</dc:creator>\n\
         <dc:language>en</dc:language>\n<dc:description>{}</dc:description>\n\
         <meta property=\"dcterms:modified\">{modified}</meta>\n{metadata_extra}</metadata>\n\
         <manifest>\n{manifest}</manifest>\n<spine>\n{spine}</spine>\n</package>\n",
        xml_escape(&project.id),
        xml_escape(&project.title),
        xml_escape(&project.author),
        xml_escape(&project.logline),
    )
}

/// Front matter, title page, contents, chapters and back matter, in reading
/// order, with the paragraph classes their bodies need.
fn book_pages(
    project: &Project,
    settings: &ExportSettings,
    classes: &mut ParagraphClasses,
) -> Vec<Page> {
    let mut pages: Vec<Page> = Vec::new();
    if settings.include_title_page {
        pages.push(Page {
            file: "titlepage.xhtml".into(),
            title: "Title page".into(),
            body: html_title_page(project, settings),
            in_nav: false,
        });
    }
    let sections = manuscript_sections(project, settings);
    if settings.include_toc {
        // Filled in after the pages exist, because it lists them.
        pages.push(Page {
            file: "contents.xhtml".into(),
            title: "Contents".into(),
            body: String::new(),
            in_nav: false,
        });
    }
    for doc in front_matter(project, settings) {
        pages.push(Page {
            file: format!("front-{}.xhtml", slugify(&doc.title)),
            title: doc.title.clone(),
            body: format!(
                "<h2 class=\"heading\">{}</h2>\n{}",
                xml_escape(&doc.title),
                html_document_body(project, doc, settings, classes)
            ),
            in_nav: true,
        });
    }
    for (index, section) in sections.iter().enumerate() {
        let title = section_title(project, section.doc, index, settings);
        // A script prints only what the writer typed, as in the PDF; its title
        // still names the page in the reader's menu.
        let heading = if section.doc.medium.is_script() {
            String::new()
        } else {
            format!("<h1 class=\"heading chapter\">{}</h1>\n", xml_escape(&title))
        };
        pages.push(Page {
            file: format!("chapter-{:03}.xhtml", index + 1),
            title,
            body: format!(
                "{heading}{}",
                html_document_body(project, section.doc, settings, classes)
            ),
            in_nav: true,
        });
    }
    for doc in back_matter(project, settings) {
        pages.push(Page {
            file: format!("back-{}.xhtml", slugify(&doc.title)),
            title: doc.title.clone(),
            body: format!(
                "<h2 class=\"heading\">{}</h2>\n{}",
                xml_escape(&doc.title),
                html_document_body(project, doc, settings, classes)
            ),
            in_nav: true,
        });
    }
    if let Some(contents) = pages.iter().position(|page| page.file == "contents.xhtml") {
        let list = contents_list(&pages);
        pages[contents].body = format!(
            "<h2 class=\"heading\">Contents</h2>\n<nav class=\"toc\"><ol>\n{list}</ol></nav>\n"
        );
    }
    pages
}

pub(crate) fn epub_manuscript(project: &Project, settings: &ExportSettings) -> Vec<u8> {
    let mut classes = ParagraphClasses::default();
    let pages = book_pages(project, settings, &mut classes);
    let css = stylesheet(project, settings, WebTarget::Epub, &classes);
    let fonts = used_fonts(&css);
    let css = format!("{}{css}", font_face_css(&fonts));
    let cover = cover_image(project, settings);

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut add = |name: &str, options: SimpleFileOptions, bytes: &[u8]| {
            zip.start_file(name, options).unwrap();
            zip.write_all(bytes).unwrap();
        };

        // The mimetype entry must come first and be stored uncompressed.
        add("mimetype", stored, b"application/epub+zip");
        add("META-INF/container.xml", deflated, CONTAINER_XML.as_bytes());
        add("OEBPS/style.css", deflated, css.as_bytes());

        let mut manifest = String::new();
        let mut spine = String::new();
        let mut metadata_extra = String::new();

        for font in &fonts {
            add(&format!("OEBPS/fonts/{}", font.file), deflated, font.bytes);
            manifest.push_str(&format!(
                "<item id=\"{}\" href=\"fonts/{}\" media-type=\"font/ttf\"/>\n",
                epub_font_id(font.file),
                font.file
            ));
        }

        if let Some((ext, media, bytes)) = &cover {
            add(&format!("OEBPS/cover-image/cover.{ext}"), deflated, bytes);
            let page = xhtml_document(
                "Cover",
                &format!("<div class=\"cover\"><img src=\"cover-image/cover.{ext}\" alt=\"cover\"/></div>\n"),
            );
            add("OEBPS/cover.xhtml", deflated, page.as_bytes());
            manifest.push_str("<item id=\"cover\" href=\"cover.xhtml\" media-type=\"application/xhtml+xml\"/>\n");
            manifest.push_str(&format!(
                "<item id=\"cover-img\" href=\"cover-image/cover.{ext}\" media-type=\"{media}\" properties=\"cover-image\"/>\n"
            ));
            spine.push_str("<itemref idref=\"cover\"/>\n");
            metadata_extra.push_str("<meta name=\"cover\" content=\"cover-img\"/>\n");
        }

        for (index, page) in pages.iter().enumerate() {
            add(
                &format!("OEBPS/{}", page.file),
                deflated,
                xhtml_document(&page.title, &page.body).as_bytes(),
            );
            manifest.push_str(&format!(
                "<item id=\"f{index}\" href=\"{}\" media-type=\"application/xhtml+xml\"/>\n",
                page.file
            ));
            spine.push_str(&format!("<itemref idref=\"f{index}\"/>\n"));
        }

        add("OEBPS/nav.xhtml", deflated, nav_document(&pages).as_bytes());
        manifest.push_str("<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n");
        manifest.push_str("<item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\n");

        let opf = package_document(project, &manifest, &spine, &metadata_extra);
        add("OEBPS/content.opf", deflated, opf.as_bytes());
        zip.finish().unwrap();
    }
    buf
}
