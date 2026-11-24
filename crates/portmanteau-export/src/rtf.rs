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
                let href = run.link.as_deref().and_then(export_href);
                if href.is_some() {
                    prefix.push_str("\\ul ");
                    suffix.insert_str(0, "\\ulnone ");
                }
                let text = rtf_escape(&paragraph.transform(&run.text));
                match href {
                    // The address has no backslash or brace left in it (export_href).
                    Some(href) => out.push_str(&format!(
                        "{{\\field{{\\*\\fldinst{{HYPERLINK \"{href}\"}}}}{{\\fldrslt{{{prefix}{text}{suffix}}}}}}}"
                    )),
                    None => {
                        out.push_str(&prefix);
                        out.push_str(&text);
                        out.push_str(&suffix);
                    }
                }
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

/// Font-table index: 0 is the book face, 1 is the monospace face.
fn rtf_font_index(style: &ElementStyle, fallback_font: &str) -> u8 {
    let family = if style.font.trim().is_empty() {
        fallback_font
    } else {
        &style.font
    };
    u8::from(font_is_mono(family))
}

/// The one place an `ElementStyle` becomes RTF paragraph formatting. Starting
/// with `\pard` resets everything, so no paragraph inherits its neighbour's
/// alignment or indents.
pub(crate) fn rtf_paragraph_format(
    style: &ElementStyle,
    fallback_font: &str,
    page_break_before: bool,
) -> String {
    let mut out = String::from("\\pard");
    if page_break_before {
        out.push_str("\\pagebb");
    }
    out.push_str(match style.align {
        Alignment::Center => "\\qc",
        Alignment::Right => "\\qr",
        Alignment::Justify => "\\qj",
        Alignment::Left => "\\ql",
    });
    let left = pt_to_twips(style.indent.max(0.0));
    let right = pt_to_twips(style.right_indent.max(0.0));
    let first = pt_to_twips(style.first_line_indent);
    if left > 0 {
        out.push_str(&format!("\\li{left}"));
    }
    if right > 0 {
        out.push_str(&format!("\\ri{right}"));
    }
    if first != 0 {
        out.push_str(&format!("\\fi{first}"));
    }
    out.push_str(&format!(
        "\\sb{}\\sa{}\\sl{}\\slmult1",
        pt_to_twips(style.before.max(0.0)),
        pt_to_twips(style.after.max(0.0)),
        (style.leading.max(1.0) * 240.0).round() as i64,
    ));
    out.push_str(&format!(
        "\\f{}\\fs{}",
        rtf_font_index(style, fallback_font),
        (style.size.max(8.0) * 2.0).round() as i64,
    ));
    if style.italic {
        out.push_str("\\i");
    }
    out.push(' ');
    out
}

/// A paragraph that is one plain string in one style (title page lines,
/// headings, script elements).
pub(crate) fn rtf_plain_paragraph(
    style: &ElementStyle,
    fallback_font: &str,
    text: &str,
    bold: bool,
    page_break_before: bool,
) -> String {
    let (on, off) = if bold { ("\\b ", "\\b0 ") } else { ("", "") };
    format!(
        "{}{on}{}{off}\\par\n",
        rtf_paragraph_format(style, fallback_font, page_break_before),
        rtf_escape(text)
    )
}

pub(crate) fn rtf_prose_paragraph(paragraph: &ProseParagraph, fallback_font: &str) -> String {
    let mut out = rtf_paragraph_format(&paragraph.format, fallback_font, false);
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
    let st = &settings.styles;
    let serif = settings.font.as_str();
    let mono = "Courier New";
    let mut out = String::new();
    let mut scenes = SceneNumbers::default();
    out.push_str(&format!(
        "{{\\rtf1\\ansi\\deff0{{\\fonttbl{{\\f0 {};}}{{\\f1 {mono};}}}}\n",
        rtf_escape(serif)
    ));
    let (page_w, page_h) = paper_twips(settings);
    let margins = page_margins(project, settings);
    out.push_str(&format!(
        "\\paperw{page_w}\\paperh{page_h}{}\\margl{}\\margr{}\\margt{}\\margb{}\n",
        if settings.mirror_margins && !project.compile_plan(settings).kind.is_script() {
            "\\margmirror"
        } else {
            ""
        },
        pt_to_twips(margins.left),
        pt_to_twips(margins.right),
        pt_to_twips(margins.top),
        pt_to_twips(margins.bottom),
    ));

    let is_script = project.compile_plan(settings).kind.is_script();
    if settings.include_title_page && is_script {
        for line in script_cover_lines(project, settings) {
            out.push_str(&rtf_plain_paragraph(&line.style, mono, &line.text, false, false));
        }
    } else if settings.include_title_page {
        let title = ElementStyle {
            before: st.title_page_title.before + title_drop_pt(project, settings),
            ..st.title_page_title.clone()
        };
        out.push_str(&rtf_plain_paragraph(&title, serif, &project.title, true, false));
        if !project.subtitle.trim().is_empty() {
            out.push_str(&rtf_plain_paragraph(&st.title_page_subtitle, serif, &project.subtitle, false, false));
        }
        if !project.author.trim().is_empty() {
            out.push_str(&rtf_plain_paragraph(&st.title_page_author, serif, &format!("by {}", project.author), false, false));
        }
    }
    if settings.include_toc {
        out.push_str(&rtf_plain_paragraph(&st.chapter_heading, serif, "Contents", true, true));
        let entry = ElementStyle {
            first_line_indent: 0.0,
            ..st.body.clone()
        };
        for title in contents_entries(project, settings) {
            out.push_str(&rtf_plain_paragraph(&entry, serif, &title, false, false));
        }
    }
    for doc in front_matter(project, settings) {
        out.push_str(&rtf_plain_paragraph(&st.chapter_heading, serif, &doc.title, true, true));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&rtf_prose_paragraph(&para, serif));
        }
    }
    for (i, section) in manuscript_sections(project, settings).iter().enumerate() {
        if section.doc.medium.is_script() {
            // A script prints only what the writer typed, as in the PDF.
            let mut first_block = true;
            for block in screenplay_blocks(project, section.doc) {
                let style = st.style_for_kind(block.kind);
                let opening;
                let style = if opens_script(block.kind, &block.text) {
                    opening = opening_transition_style(style);
                    &opening
                } else {
                    style
                };
                let text = if style.uppercase {
                    block.text.to_uppercase()
                } else {
                    block.text.clone()
                };
                let text = scenes.apply(block.kind, text, settings.include_scene_numbers);
                let new_page = first_block
                    && if i == 0 { settings.include_title_page } else { starts_new_page(section.doc) };
                first_block = false;
                out.push_str(&rtf_plain_paragraph(style, mono, &text, false, new_page));
            }
        } else {
            let title = section_title(project, section.doc, i, settings);
            out.push_str(&rtf_plain_paragraph(&st.chapter_heading, serif, &title, true, true));
            for para in prose_paragraphs(project, section.doc, settings) {
                out.push_str(&rtf_prose_paragraph(&para, serif));
            }
        }
    }
    for doc in back_matter(project, settings) {
        out.push_str(&rtf_plain_paragraph(&st.chapter_heading, serif, &doc.title, true, true));
        for para in prose_paragraphs(project, doc, settings) {
            out.push_str(&rtf_prose_paragraph(&para, serif));
        }
    }
    out.push_str("}\n");
    out
}
