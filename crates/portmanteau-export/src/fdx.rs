use super::*;

pub(crate) fn fdx_type(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::SceneHeading => "Scene Heading",
        ElementKind::Action => "Action",
        ElementKind::Character => "Character",
        ElementKind::Parenthetical => "Parenthetical",
        ElementKind::Dialogue => "Dialogue",
        ElementKind::Transition => "Transition",
        ElementKind::Shot => "Shot",
        ElementKind::Act => "General",
        ElementKind::Scene => "General",
        ElementKind::StageDirection => "Action",
        ElementKind::Lyric => "Lyric",
        ElementKind::General => "General",
        ElementKind::Extension => "Extension",
        ElementKind::Intercut => "Transition",
        ElementKind::Subheading => "Scene Heading",
    }
}

pub(crate) fn fdx_style_attr(marks: &[InlineMark]) -> String {
    let mut parts = Vec::new();
    for (mark, name) in [
        (InlineMark::Bold, "Bold"),
        (InlineMark::Italic, "Italic"),
        (InlineMark::Underline, "Underline"),
        (InlineMark::Strikethrough, "Strikethrough"),
    ] {
        if marks.contains(&mark) {
            parts.push(name);
        }
    }
    parts.join("+")
}

/// Write one paragraph's runs, preserving inline emphasis in Final Draft's
/// `<Text Style="...">` attribute. Rich images contribute an explicit
/// placeholder because FDX cannot embed their pixels here.
pub(crate) fn fdx_runs(paragraph: &RichParagraph, uppercase: bool) -> String {
    let mut out = String::new();
    for item in &paragraph.content {
        match item {
            InlineContent::Text(run) => {
                if run.text.is_empty() {
                    continue;
                }
                let text = if uppercase {
                    run.text.to_uppercase()
                } else {
                    run.text.clone()
                };
                let style = fdx_style_attr(&run.marks);
                if style.is_empty() {
                    out.push_str(&format!("<Text>{}</Text>", xml_escape(&text)));
                } else {
                    out.push_str(&format!(
                        "<Text Style=\"{style}\">{}</Text>",
                        xml_escape(&text)
                    ));
                }
            }
            InlineContent::HardBreak => out.push_str("<Text> </Text>"),
            InlineContent::Image(image) => {
                let label = if image.alt_text.is_empty() {
                    "[Image]".to_owned()
                } else {
                    format!("[Image: {}]", image.alt_text)
                };
                out.push_str(&format!("<Text>{}</Text>", xml_escape(&label)));
            }
        }
    }
    out
}

pub(crate) fn fdx_manuscript(project: &Project, settings: &ExportSettings) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n");
    out.push_str("<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"5\">\n");
    if settings.include_title_page {
        out.push_str("  <TitlePage><Content>\n");
        out.push_str(&format!(
            "    <Paragraph Type=\"Title\"><Text>{}</Text></Paragraph>\n",
            xml_escape(&project.title)
        ));
        if !project.subtitle.is_empty() {
            out.push_str(&format!(
                "    <Paragraph Type=\"Subtitle\"><Text>{}</Text></Paragraph>\n",
                xml_escape(&project.subtitle)
            ));
        }
        out.push_str(&format!(
            "    <Paragraph Type=\"Author\"><Text>{}</Text></Paragraph>\n",
            xml_escape(&project.author)
        ));
        out.push_str("  </Content></TitlePage>\n");
    }
    out.push_str("  <Content>\n");
    for section in manuscript_sections(project, settings) {
        if section.doc.medium.is_script() {
            let fallback = StyleSheet::film_screenplay(&project.styles);
            let sheet = project
                .style_sheets
                .iter()
                .find(|sheet| sheet.id == section.doc.style_sheet_id)
                .unwrap_or(&fallback);
            for paragraph in &section.doc.content.paragraphs {
                let style = sheet.style(&paragraph.style_id);
                let kind = style
                    .and_then(|style| style.semantic_kind)
                    .or_else(|| {
                        portmanteau_core::rich_text::script_kind_for_style_id(
                            &paragraph.style_id,
                        )
                    })
                    .unwrap_or(ElementKind::General);
                let uppercase = style
                    .map(|style| style.text_transform == TextTransform::Uppercase)
                    .unwrap_or_else(|| settings.styles.style_for_kind(kind).uppercase);
                let runs = fdx_runs(paragraph, uppercase);
                if runs.is_empty() {
                    continue;
                }
                out.push_str(&format!(
                    "    <Paragraph Type=\"{}\">{runs}</Paragraph>\n",
                    fdx_type(kind)
                ));
            }
        } else {
            out.push_str(&format!(
                "    <Paragraph Type=\"General\"><Text>{}</Text></Paragraph>\n",
                xml_escape(&section.heading)
            ));
            for para in paragraphs(&section.doc.body_text()) {
                out.push_str(&format!(
                    "    <Paragraph Type=\"Action\"><Text>{}</Text></Paragraph>\n",
                    xml_escape(&para)
                ));
            }
        }
    }
    out.push_str("  </Content>\n</FinalDraft>\n");
    out
}
