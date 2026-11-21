use super::*;
use portmanteau_core::{InlineContent, InlineMark, RichParagraph};
use std::io::Write;

fn opts() -> ImportOptions {
    ImportOptions::default()
}

fn text_of(paragraph: &RichParagraph) -> String {
    paragraph
        .content
        .iter()
        .map(|item| match item {
            InlineContent::Text(run) => run.text.clone(),
            InlineContent::HardBreak => "\n".into(),
            InlineContent::Image(_) => "[image]".into(),
        })
        .collect()
}

fn styles_of(imported: &Imported) -> Vec<String> {
    imported
        .documents
        .iter()
        .flat_map(|d| d.content.paragraphs.iter().map(|p| p.style_id.clone()))
        .collect()
}

fn fountain(source: &str) -> Imported {
    import(ImportFormat::Fountain, source.as_bytes(), &opts()).expect("fountain imports")
}

fn has_warning(imported: &Imported, needle: &str) -> bool {
    imported.warnings.iter().any(|w| w.contains(needle))
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

#[test]
fn format_is_detected_from_the_extension_then_from_the_bytes() {
    let by = |name: &str| detect(Some(Path::new(name)), b"");
    assert_eq!(by("a.fountain"), Some(ImportFormat::Fountain));
    assert_eq!(by("a.FDX"), Some(ImportFormat::Fdx));
    assert_eq!(by("a.docx"), Some(ImportFormat::Docx));
    assert_eq!(by("a.txt"), Some(ImportFormat::PlainText));
    assert_eq!(by("a.pdf"), None);
    assert_eq!(detect(None, b"PK\x03\x04rest"), Some(ImportFormat::Docx));
    assert_eq!(detect(None, b"<?xml?><FinalDraft DocumentType=\"Script\">"), Some(ImportFormat::Fdx));
    assert_eq!(detect(None, b"just words"), None);
}

// ---------------------------------------------------------------------------
// Fountain
// ---------------------------------------------------------------------------

const SPEC_SAMPLE: &str = "Title:
    _**BRICK & STEEL**_
    _**FULL RETIRED**_
Credit: Written by
Author: Stu Maschwitz
Source: Story by KTM
Draft date: 1/20/2012
Contact:
    Next Level Productions
    1588 Mission Dr.

# ACT ONE
= Steel arrives.

EXT. BRICK'S PATIO - DAY #12A#

A gorgeous day.  The sun is shining.
It is *very* quiet. [[check this]]

INT. TRAILER HOME - DAY

MARA (V.O.)
(quietly)
Hello there.
Second line.

SMASH CUT TO:

> BURN TO PINK.

.SNIPER SCOPE POV

@McCLANE
Yippee.

!SCANNING THE AREA

FADE OUT.
";

#[test]
fn the_fountain_title_page_becomes_metadata_not_action() {
    let imported = fountain(SPEC_SAMPLE);
    assert_eq!(imported.title.as_deref(), Some("BRICK & STEEL FULL RETIRED"));
    assert_eq!(imported.author.as_deref(), Some("Stu Maschwitz"));
    let all: String = imported.documents[0].content.paragraphs.iter().map(text_of).collect();
    assert!(!all.contains("Draft date") && !all.contains("Mission Dr"));
    assert!(has_warning(&imported, "Source, Draft date, Contact"), "{:?}", imported.warnings);
}

#[test]
fn fountain_elements_follow_the_spec() {
    let imported = fountain(SPEC_SAMPLE);
    assert_eq!(imported.documents.len(), 1, "one # section -> one document");
    let doc = &imported.documents[0];
    assert_eq!(doc.title, "ACT ONE");
    assert_eq!(doc.synopsis, "Steel arrives.");

    let got: Vec<(String, String)> = doc
        .content
        .paragraphs
        .iter()
        .map(|p| (p.style_id.clone(), text_of(p)))
        .collect();
    let kinds: Vec<&str> = got.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        kinds,
        [
            "scene-heading", "action", "scene-heading", "character", "parenthetical", "dialogue",
            "transition", "transition", "scene-heading", "character", "dialogue", "action",
            "transition",
        ],
        "{got:#?}"
    );
    assert_eq!(got[0].1, "EXT. BRICK'S PATIO - DAY", "scene number is stripped");
    assert_eq!(got[1].1, "A gorgeous day.  The sun is shining.\nIt is very quiet.", "note removed");
    assert_eq!(got[3].1, "MARA (V.O.)");
    assert_eq!(got[5].1, "Hello there.\nSecond line.", "dialogue lines are kept");
    assert_eq!(got[7].1, "BURN TO PINK.");
    assert_eq!(got[8].1, "SNIPER SCOPE POV");
    assert_eq!(got[9].1, "McCLANE", "@ forces a character");
    assert_eq!(got[11].1, "SCANNING THE AREA", "! forces action and is dropped");
    assert!(has_warning(&imported, "1 script note not imported"));
}

#[test]
fn fountain_emphasis_becomes_marks() {
    let imported = fountain("A **loud** and _underlined_ word, ***both***.");
    let paragraph = &imported.documents[0].content.paragraphs[0];
    let marked = |text: &str| {
        paragraph.content.iter().find_map(|item| match item {
            InlineContent::Text(run) if run.text == text => Some(run.marks.clone()),
            _ => None,
        })
    };
    assert_eq!(marked("loud"), Some(vec![InlineMark::Bold]));
    assert_eq!(marked("underlined"), Some(vec![InlineMark::Underline]));
    assert!(marked("both").is_some_and(|m| m.contains(&InlineMark::Bold) && m.contains(&InlineMark::Italic)));
}

#[test]
fn fountain_boneyard_is_removed_but_an_unclosed_one_is_just_text() {
    let imported = fountain("Before /* a deleted\nspan across lines */ after.");
    assert_eq!(text_of(&imported.documents[0].content.paragraphs[0]), "Before  after.");
    let unclosed = fountain("Keep /* this and everything after it");
    assert!(text_of(&unclosed.documents[0].content.paragraphs[0]).contains("this and everything"));
}

#[test]
fn fountain_without_a_title_page_starts_straight_in() {
    let imported = fountain("INT. KITCHEN - NIGHT\n\nMARA\nWho is there?\n");
    assert_eq!(imported.title, None);
    assert_eq!(styles_of(&imported), ["scene-heading", "character", "dialogue"]);
}

#[test]
fn fountain_sections_split_only_when_asked() {
    let source = "# One\n\nINT. A - DAY\n\n## beat\n\nAction.\n\n# Two\n\nINT. B - DAY\n";
    let split = fountain(source);
    assert_eq!(split.documents.len(), 2);
    assert_eq!(split.documents[1].title, "Two");
    assert!(has_warning(&split, "1 outline marker not imported"), "{:?}", split.warnings);

    let whole = import(
        ImportFormat::Fountain,
        source.as_bytes(),
        &ImportOptions { split: Split::Never, ..opts() },
    )
    .unwrap();
    assert_eq!(whole.documents.len(), 1);
    assert_eq!(whole.documents[0].content.paragraphs.len(), 3);
    assert!(has_warning(&whole, "3 outline markers not imported"), "{:?}", whole.warnings);
}

#[test]
fn fountain_dual_dialogue_and_centred_text_are_reported() {
    let imported = fountain("BRICK\nScrew retirement.\n\nSTEEL ^\nAgreed.\n\n> THE END <\n");
    assert!(has_warning(&imported, "Dual dialogue"));
    assert!(has_warning(&imported, "Centred text"));
    let kinds = styles_of(&imported);
    assert_eq!(kinds.iter().filter(|k| *k == "character").count(), 2);
}

#[test]
fn windows_1252_fountain_is_decoded_and_reported() {
    let mut bytes = b"INT. CAF".to_vec();
    bytes.push(0xC9);
    bytes.extend(b" - DAY\n\nMARA\n");
    bytes.push(0x93);
    bytes.extend(b"Hello");
    bytes.push(0x94);
    bytes.push(b'\n');
    let imported = import(ImportFormat::Fountain, &bytes, &opts()).unwrap();
    let doc = &imported.documents[0].content.paragraphs;
    assert_eq!(text_of(&doc[0]), "INT. CAFÉ - DAY");
    assert_eq!(text_of(&doc[2]), "“Hello”");
    assert!(has_warning(&imported, "Windows-1252"));
}

#[test]
fn an_empty_file_is_an_error_not_a_blank_document() {
    for format in [ImportFormat::Fountain, ImportFormat::PlainText] {
        assert!(import(format, b"   \n\n  ", &opts()).is_err(), "{format:?}");
    }
}

// ---------------------------------------------------------------------------
// Plain text
// ---------------------------------------------------------------------------

#[test]
fn plain_text_rejoins_hard_wrapped_lines_into_paragraphs() {
    let text = "The tide went\nout that morning.\n\nA second paragraph.\n";
    let imported = import(ImportFormat::PlainText, text.as_bytes(), &opts()).unwrap();
    let paragraphs = &imported.documents[0].content.paragraphs;
    assert_eq!(paragraphs.len(), 2);
    assert_eq!(text_of(&paragraphs[0]), "The tide went out that morning.");
    assert!(paragraphs.iter().all(|p| p.style_id == "body"));
}

#[test]
fn plain_text_without_blank_lines_is_one_paragraph_per_line() {
    let imported = import(ImportFormat::PlainText, b"One\nTwo\nThree", &opts()).unwrap();
    assert_eq!(imported.documents[0].content.paragraphs.len(), 3);
}

#[test]
fn plain_text_chapters_split_into_documents_titled_by_their_heading() {
    let text = "Prologue\n\nIt began.\n\nChapter 1: The Tide\n\nOut it went.\n\nCHAPTER TWO\n\nAnd back.\n";
    let imported = import(ImportFormat::PlainText, text.as_bytes(), &opts()).unwrap();
    let titles: Vec<&str> = imported.documents.iter().map(|d| d.title.as_str()).collect();
    assert_eq!(titles, ["Prologue", "Chapter 1: The Tide", "CHAPTER TWO"]);
    assert_eq!(text_of(&imported.documents[1].content.paragraphs[0]), "Out it went.");

    let whole = import(
        ImportFormat::PlainText,
        text.as_bytes(),
        &ImportOptions { split: Split::Never, ..opts() },
    )
    .unwrap();
    assert_eq!(whole.documents.len(), 1);
    assert_eq!(whole.documents[0].content.paragraphs.len(), 6);
}

#[test]
fn a_lone_number_is_a_list_but_several_are_chapters() {
    let one = import(ImportFormat::PlainText, b"Intro text.\n\n1.\n\nSome item.\n", &opts()).unwrap();
    assert_eq!(one.documents.len(), 1);
    let several = import(
        ImportFormat::PlainText,
        b"I.\n\nFirst chapter text.\n\nII.\n\nSecond chapter text.\n",
        &opts(),
    )
    .unwrap();
    assert_eq!(several.documents.len(), 2);
}

#[test]
fn prose_that_merely_mentions_a_chapter_is_not_split() {
    let text = "Chapter one of the manual explains the process in considerable detail, with examples.\n\nMore.\n";
    let imported = import(ImportFormat::PlainText, text.as_bytes(), &opts()).unwrap();
    assert_eq!(imported.documents.len(), 1);
}

#[test]
fn plain_text_can_be_read_as_a_screenplay() {
    let imported = import(
        ImportFormat::PlainText,
        b"INT. HARBOUR - DAWN\n\nMARA\nWe missed the tide.\n",
        &ImportOptions { medium: Some(Medium::Screenplay), ..opts() },
    )
    .unwrap();
    assert_eq!(styles_of(&imported), ["scene-heading", "character", "dialogue"]);
}

// ---------------------------------------------------------------------------
// Final Draft
// ---------------------------------------------------------------------------

const FDX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<FinalDraft DocumentType="Script" Template="No" Version="2">
  <Content>
    <Paragraph Type="Scene Heading"><SceneProperties Length="1/8"/><Text>INT. KITCHEN - DAY</Text></Paragraph>
    <Paragraph Type="Action"><Text>Mara </Text><Text Style="Bold+Italic">freezes</Text><Text>.</Text></Paragraph>
    <Paragraph Type="Action"><Text></Text></Paragraph>
    <DualDialogue>
      <Paragraph Type="Character"><Text>MARA</Text></Paragraph>
      <Paragraph Type="Dialogue"><Text>Now.</Text></Paragraph>
    </DualDialogue>
    <Paragraph Type="Mystery Type"><Text>Who knows.</Text></Paragraph>
    <Paragraph Type="General">
      <Text>Noted.</Text>
      <ScriptNote><Paragraph><Text>private remark</Text></Paragraph></ScriptNote>
    </Paragraph>
    <Paragraph Type="Action"><Text>Photo</Text><Image>AAAA</Image></Paragraph>
  </Content>
  <TitlePage>
    <Content>
      <Paragraph Alignment="Center"><Text>THE SALT ROAD</Text></Paragraph>
      <Paragraph Alignment="Center"><Text>Written by</Text></Paragraph>
      <Paragraph Alignment="Center"><Text>Ada Marlowe</Text></Paragraph>
    </Content>
  </TitlePage>
</FinalDraft>"#;

#[test]
fn fdx_imports_known_types_marks_and_the_title_page() {
    let imported = import(ImportFormat::Fdx, FDX.as_bytes(), &opts()).unwrap();
    assert_eq!(imported.title.as_deref(), Some("THE SALT ROAD"));
    assert_eq!(imported.author.as_deref(), Some("Ada Marlowe"));
    let doc = &imported.documents[0].content.paragraphs;
    assert_eq!(
        doc.iter().map(|p| p.style_id.as_str()).collect::<Vec<_>>(),
        ["scene-heading", "action", "character", "dialogue", "general", "general", "action"]
    );
    let freezes = doc[1].content.iter().find_map(|item| match item {
        InlineContent::Text(run) if run.text == "freezes" => Some(run.marks.clone()),
        _ => None,
    });
    assert_eq!(freezes, Some(vec![InlineMark::Bold, InlineMark::Italic]));
    // Title-page paragraphs are not part of the script.
    assert!(!doc.iter().any(|p| text_of(p).contains("Ada Marlowe")));
}

#[test]
fn fdx_does_not_fail_on_unknown_types_images_or_notes() {
    let imported = import(ImportFormat::Fdx, FDX.as_bytes(), &opts()).unwrap();
    assert!(has_warning(&imported, "Mystery Type"), "{:?}", imported.warnings);
    assert!(has_warning(&imported, "1 embedded image not imported"));
    assert!(has_warning(&imported, "1 script note not imported"));
    // The note's own paragraph must not leak into the script text.
    let all: String = imported.documents[0].content.paragraphs.iter().map(text_of).collect();
    assert!(!all.contains("private remark"));
}

#[test]
fn fdx_rejects_files_that_are_not_final_draft() {
    assert!(import(ImportFormat::Fdx, b"<html><body/></html>", &opts()).is_err());
    assert!(import(ImportFormat::Fdx, b"not xml at all", &opts()).is_err());
}

// ---------------------------------------------------------------------------
// Word
// ---------------------------------------------------------------------------

fn docx(styles: Option<&str>, core: Option<&str>, body: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = zip::write::SimpleFileOptions::default();
        let mut put = |name: &str, text: &str| {
            zip.start_file(name, options).unwrap();
            zip.write_all(text.as_bytes()).unwrap();
        };
        put(
            "word/document.xml",
            &format!(
                r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
            ),
        );
        if let Some(styles) = styles {
            put(
                "word/styles.xml",
                &format!(r#"<?xml version="1.0"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">{styles}</w:styles>"#),
            );
        }
        if let Some(core) = core {
            put("docProps/core.xml", core);
        }
        zip.finish().unwrap();
    }
    buffer
}

fn p(style: Option<&str>, runs: &str) -> String {
    let properties = style
        .map(|s| format!(r#"<w:pPr><w:pStyle w:val="{s}"/></w:pPr>"#))
        .unwrap_or_default();
    format!("<w:p>{properties}{runs}</w:p>")
}

fn r(text: &str) -> String {
    format!(r#"<w:r><w:t xml:space="preserve">{text}</w:t></w:r>"#)
}

const STYLES: &str = r#"
<w:style w:type="paragraph" w:styleId="Titulo1"><w:name w:val="heading 1"/></w:style>
<w:style w:type="paragraph" w:styleId="MyChapter"><w:name w:val="My Chapter"/><w:basedOn w:val="Titulo1"/></w:style>
<w:style w:type="paragraph" w:styleId="Outlined"><w:name w:val="Odd"/><w:pPr><w:outlineLvl w:val="1"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Titel"><w:name w:val="Title"/></w:style>"#;

#[test]
fn word_headings_are_found_through_styles_xml_and_split_into_chapters() {
    let body = [
        p(Some("Titel"), &r("The Salt Road")),
        p(None, &r("Preface text.")),
        p(Some("Titulo1"), &r("Chapter One")),
        p(None, &r("First.")),
        p(Some("MyChapter"), &r("Chapter Two")), // inherits heading 1 via basedOn
        p(None, &r("Second.")),
        p(Some("Outlined"), &r("A subheading")), // outline level 2: stays inside
        p(None, &r("Third.")),
    ]
    .concat();
    let imported = import(
        ImportFormat::Docx,
        &docx(Some(STYLES), None, &body),
        &opts(),
    )
    .unwrap();
    assert_eq!(imported.title.as_deref(), Some("The Salt Road"));
    let titles: Vec<&str> = imported.documents.iter().map(|d| d.title.as_str()).collect();
    assert_eq!(titles, ["", "Chapter One", "Chapter Two"], "preface, then two chapters");
    let two = &imported.documents[2].content.paragraphs;
    assert_eq!(
        two.iter().map(|p| (p.style_id.as_str(), text_of(p))).collect::<Vec<_>>(),
        [
            ("body", "Second.".to_string()),
            ("chapter-heading", "A subheading".to_string()),
            ("body", "Third.".to_string()),
        ]
    );
}

#[test]
fn word_split_never_keeps_one_document_with_headings_inside() {
    let body = [p(Some("Titulo1"), &r("One")), p(None, &r("a")), p(Some("Titulo1"), &r("Two")), p(None, &r("b"))].concat();
    let imported = import(
        ImportFormat::Docx,
        &docx(Some(STYLES), None, &body),
        &ImportOptions { split: Split::Never, ..opts() },
    )
    .unwrap();
    assert_eq!(imported.documents.len(), 1);
    assert_eq!(imported.documents[0].content.paragraphs.len(), 4);
}

#[test]
fn word_marks_breaks_and_core_properties_are_read() {
    let core = r#"<cp:coreProperties xmlns:cp="a" xmlns:dc="b"><dc:title>Core Title</dc:title><dc:creator>Core Author</dc:creator></cp:coreProperties>"#;
    let runs = format!(
        "{}{}{}{}",
        r#"<w:r><w:rPr><w:b/><w:i w:val="0"/></w:rPr><w:t>bold</w:t></w:r>"#,
        r#"<w:r><w:rPr><w:rStyle w:val="Emphasis"/></w:rPr><w:t> em</w:t></w:r>"#,
        r#"<w:r><w:br/><w:t>next line</w:t></w:r>"#,
        r#"<w:r><w:br w:type="page"/><w:t> same page</w:t></w:r>"#,
    );
    let imported = import(
        ImportFormat::Docx,
        &docx(None, Some(core), &p(None, &runs)),
        &opts(),
    )
    .unwrap();
    assert_eq!(imported.title.as_deref(), Some("Core Title"));
    assert_eq!(imported.author.as_deref(), Some("Core Author"));
    let paragraph = &imported.documents[0].content.paragraphs[0];
    assert_eq!(text_of(paragraph), "bold em\nnext line same page");
    let marks = |text: &str| {
        paragraph.content.iter().find_map(|item| match item {
            InlineContent::Text(run) if run.text == text => Some(run.marks.clone()),
            _ => None,
        })
    };
    assert_eq!(marks("bold"), Some(vec![InlineMark::Bold]), "w:i w:val=0 is off");
    assert_eq!(marks(" em"), Some(vec![InlineMark::Italic]));
}

#[test]
fn word_text_boxes_are_not_duplicated_and_losses_are_reported() {
    let textbox = r#"<w:r><w:pict><w:txbxContent><w:p><w:r><w:t>inside the box</w:t></w:r></w:p></w:txbxContent></w:pict></w:r>"#;
    let body = [
        p(None, &format!("{}{}", r("Outer text."), textbox)),
        p(None, &format!("{}{}", r("Note"), r#"<w:r><w:footnoteReference w:id="2"/></w:r>"#)),
        p(None, r#"<w:r><w:drawing/></w:r><w:r><w:t>Picture caption</w:t></w:r>"#),
        r#"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#.to_string(),
    ]
    .concat();
    let imported = import(ImportFormat::Docx, &docx(None, None, &body), &opts()).unwrap();
    let texts: Vec<String> = imported.documents[0].content.paragraphs.iter().map(text_of).collect();
    assert_eq!(texts.iter().filter(|t| t.contains("inside the box")).count(), 1, "{texts:?}");
    assert_eq!(texts[0], "Outer text.");
    assert!(has_warning(&imported, "1 footnote or endnote not imported"));
    assert!(has_warning(&imported, "1 image not imported"));
    assert!(has_warning(&imported, "1 table not imported"), "{:?}", imported.warnings);
    assert!(texts.contains(&"cell".to_string()), "table text is kept as paragraphs");
}

#[test]
fn word_screenplay_styles_map_to_script_elements() {
    let styles = r#"
<w:style w:type="paragraph" w:styleId="SH"><w:name w:val="Scene Heading"/></w:style>
<w:style w:type="paragraph" w:styleId="CH"><w:name w:val="Character"/></w:style>
<w:style w:type="paragraph" w:styleId="DL"><w:name w:val="Dialogue"/></w:style>"#;
    let body = [
        p(Some("SH"), &r("INT. ROOM - DAY")),
        p(Some("CH"), &r("MARA")),
        p(Some("DL"), &r("Hello.")),
        p(None, &r("EXT. YARD - NIGHT")),
    ]
    .concat();
    let imported = import(
        ImportFormat::Docx,
        &docx(Some(styles), None, &body),
        &ImportOptions { medium: Some(Medium::Screenplay), ..opts() },
    )
    .unwrap();
    assert_eq!(styles_of(&imported), ["scene-heading", "character", "dialogue", "scene-heading"]);
}

#[test]
fn word_rejects_files_that_are_not_documents() {
    assert!(import(ImportFormat::Docx, b"definitely not a zip", &opts()).is_err());
    let empty = docx(None, None, "");
    assert!(import(ImportFormat::Docx, &empty, &opts()).is_err());
}

// ---------------------------------------------------------------------------
// Round trips through our own exporters
// ---------------------------------------------------------------------------

mod round_trip {
    use super::*;
    use portmanteau_core::{sample, CompileType, ExportKind};

    fn exported(kind: ExportKind, compile: Option<CompileType>, tag: &str) -> Vec<u8> {
        let project = sample::demo();
        let mut settings = project.settings_for(kind);
        settings.compile_type = compile;
        let dir = std::env::temp_dir().join(format!("portmanteau-import-{}-{tag}", std::process::id()));
        let info = portmanteau_export::export(&project, &settings, Some(&dir.join(format!("out.{}", kind.extension()))))
            .expect("export");
        let bytes = std::fs::read(&info.path).expect("read export");
        let _ = std::fs::remove_dir_all(&dir);
        bytes
    }

    /// What kinds of element the demo's TV script holds, from the project itself.
    fn demo_script_kinds() -> Vec<String> {
        let project = sample::demo();
        let doc = project
            .documents
            .iter()
            .find(|d| d.title.starts_with("Cold Harbor"))
            .expect("demo script");
        doc.content.paragraphs.iter().map(|p| p.style_id.clone()).collect()
    }

    #[test]
    fn a_fountain_export_imports_back_to_the_same_elements() {
        let bytes = exported(ExportKind::Fountain, Some(CompileType::TvScript), "fountain");
        let imported = import(ImportFormat::Fountain, &bytes, &opts()).unwrap();
        let expected = demo_script_kinds();
        let got: Vec<String> = styles_of(&imported);
        // The export writes a title page and nothing the writer did not type.
        assert_eq!(got, expected, "element kinds survive Fountain");
        assert_eq!(imported.title.as_deref(), Some("The Salt Road"));
    }

    #[test]
    fn an_fdx_export_imports_back_to_the_same_elements() {
        let bytes = exported(ExportKind::Fdx, Some(CompileType::TvScript), "fdx");
        let imported = import(ImportFormat::Fdx, &bytes, &opts()).unwrap();
        assert_eq!(styles_of(&imported), demo_script_kinds());
        assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    }

    #[test]
    fn a_docx_export_splits_back_into_its_chapters() {
        let bytes = exported(ExportKind::Docx, Some(CompileType::Prose), "docx");
        let imported = import(ImportFormat::Docx, &bytes, &opts()).unwrap();
        let titles: Vec<&str> = imported
            .documents
            .iter()
            .map(|d| d.title.as_str())
            .filter(|t| !t.is_empty())
            .collect();
        for expected in ["Chapter 1", "Chapter 2", "Chapter 3"] {
            assert!(titles.contains(&expected), "{expected} missing from {titles:?}");
        }
        assert_eq!(imported.title.as_deref(), Some("The Salt Road"));
        // Every paragraph of chapter one's text came back.
        let all: String = imported
            .documents
            .iter()
            .flat_map(|d| d.content.paragraphs.iter().map(text_of))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(all.contains("The tide went out that morning"));
    }

    #[test]
    fn a_plain_text_export_imports_back_as_chapters() {
        let bytes = exported(ExportKind::Txt, Some(CompileType::Prose), "txt");
        let imported = import(ImportFormat::PlainText, &bytes, &opts()).unwrap();
        assert!(imported.paragraph_count() > 10);
        let all: String = imported
            .documents
            .iter()
            .flat_map(|d| d.content.paragraphs.iter().map(text_of))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(all.contains("The tide went out that morning"));
    }
}
