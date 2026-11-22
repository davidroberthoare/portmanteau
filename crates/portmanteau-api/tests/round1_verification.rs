//! Round 1 verification: adversarial checks on the folder-package format.
//!
//! Independent of the unit tests inside `src/lib.rs`: they exercise the public
//! API from outside the crate and assert the specific properties the Round 1
//! acceptance criteria call for.

use std::path::PathBuf;

use portmanteau_api::{storage, Workspace};
use serde_json::json;
use portmanteau_core::{
    script, Block, DocKind, Document, ElementKind, Medium, Project, ProjectKind,
    Snapshot,
};

fn scratch(tag: &str) -> PathBuf {
    std::env::temp_dir().join(portmanteau_core::new_id(tag))
}

fn make_doc(id: &str, title: &str, medium: Medium) -> Document {
    let mut doc = Document::new(None, 0, title, DocKind::Chapter, medium);
    doc.id = id.to_string();
    doc
}

/// Round-trip a document through the folder package and compare block by block.
fn roundtrip_doc(doc: &Document) -> Document {
    let mut project = Project::new("Probe", ProjectKind::Novel);
    project.documents.push(doc.clone());
    let dir = scratch("rt1-blocks");
    storage::save_folder(&project, &dir).expect("save folder");
    let reloaded = storage::load_folder(&dir).expect("load folder");
    let _ = std::fs::remove_dir_all(dir);
    // `Project::new` seeds scaffold documents, so pick ours out by id.
    reloaded
        .documents
        .into_iter()
        .find(|d| d.id == doc.id)
        .expect("the probed document came back")
}

fn script_blocks(doc: &Document) -> Vec<Block> {
    let sheet = doc
        .style_sheet_id
        .as_str();
    let styles = portmanteau_core::ElementStyles::default();
    let style_sheet = if sheet == "film-screenplay" {
        portmanteau_core::StyleSheet::film_screenplay(&styles)
    } else {
        portmanteau_core::StyleSheet::film_screenplay(&styles)
    };
    doc.content.to_script_blocks(&style_sheet)
}

const ALL_KINDS: [ElementKind; 15] = [
    ElementKind::SceneHeading,
    ElementKind::Action,
    ElementKind::Character,
    ElementKind::Parenthetical,
    ElementKind::Dialogue,
    ElementKind::Transition,
    ElementKind::Act,
    ElementKind::Scene,
    ElementKind::Lyric,
    ElementKind::Shot,
    ElementKind::StageDirection,
    ElementKind::General,
    ElementKind::Extension,
    ElementKind::Intercut,
    ElementKind::Subheading,
];

#[test]
fn script_document_blocks_survive_a_folder_roundtrip_unchanged() {
    let blocks = vec![
        Block { kind: ElementKind::Transition, text: "FADE IN:".into() },
        Block { kind: ElementKind::SceneHeading, text: "EXT. COLD HARBOR - DAWN".into() },
        Block { kind: ElementKind::Action, text: "The sea has gone out.".into() },
        Block { kind: ElementKind::Character, text: "MARA".into() },
        Block { kind: ElementKind::Parenthetical, text: "(to herself)".into() },
        Block { kind: ElementKind::Dialogue, text: "The chart says there's a channel here.".into() },
        Block { kind: ElementKind::Act, text: "Act One".into() },
        Block { kind: ElementKind::Scene, text: "Later".into() },
        Block { kind: ElementKind::Lyric, text: "a song about the sea".into() },
        Block { kind: ElementKind::Shot, text: "WIDE ON THE HARBOUR".into() },
        Block { kind: ElementKind::StageDirection, text: "Lights up.".into() },
        Block { kind: ElementKind::General, text: "A general note.".into() },
        // A lone Dialogue with no Character cue before it.
        Block { kind: ElementKind::Dialogue, text: "Stray line.".into() },
        // Dialogue directly after a Character (no parenthetical).
        Block { kind: ElementKind::Character, text: "BOY".into() },
        Block { kind: ElementKind::Dialogue, text: "The chart's wrong.".into() },
    ];
    let mut doc = make_doc("doc-script-probe", "Probe Script", Medium::Screenplay);
    doc.set_script_blocks(blocks.clone());
    let back = roundtrip_doc(&doc);
    let back_blocks = script_blocks(&back);
    assert_eq!(
        back_blocks.len(),
        blocks.len(),
        "block count changed: {:?}",
        back_blocks
    );
    for (i, (before, after)) in blocks.iter().zip(back_blocks.iter()).enumerate() {
        assert_eq!(before.kind, after.kind, "block {i} kind changed");
        assert_eq!(before.text, after.text, "block {i} text changed");
    }
}

/// Text that is unambiguously of the given kind, so the round trip is testing
/// the storage format rather than the classifier.
fn representative_text(kind: ElementKind) -> String {
    match kind {
        ElementKind::SceneHeading => "EXT. COLD HARBOR BREAKWATER - DAWN".into(),
        ElementKind::Action => "The sea has gone out.".into(),
        ElementKind::Character => "MARA".into(),
        ElementKind::Parenthetical => "(to herself)".into(),
        ElementKind::Dialogue => "The chart says there is a channel here.".into(),
        ElementKind::Transition => "FADE IN:".into(),
        ElementKind::Act => "Act One".into(),
        ElementKind::Scene => "Later".into(),
        ElementKind::Lyric => "a song about the sea".into(),
        ElementKind::Shot => "WIDE ON THE HARBOUR".into(),
        ElementKind::StageDirection => "The lights rise.".into(),
        ElementKind::General => "A general note.".into(),
        ElementKind::Extension => "V.O.".into(),
        ElementKind::Intercut => "INTERCUT WITH THE HARBOUR".into(),
        ElementKind::Subheading => "THE OLD ROAD".into(),
    }
}

#[test]
fn every_element_kind_roundtrips_in_isolation() {
    assert_eq!(ALL_KINDS.len(), 15, "the interchange vocabulary has fifteen kinds");
    for kind in ALL_KINDS {
        let mut doc = make_doc("doc-kind-probe", "Kind Probe", Medium::Screenplay);
        let body = representative_text(kind);
        doc.set_script_blocks(vec![Block { kind, text: body.clone() }]);
        let back = roundtrip_doc(&doc);
        let blocks = script_blocks(&back);
        assert_eq!(blocks.len(), 1, "{kind:?} lost: {blocks:?}");
        assert_eq!(blocks[0].kind, kind, "{kind:?} changed kind");
        assert_eq!(blocks[0].text, body, "{kind:?} changed text");
    }
}

/// A scene heading that does not use the `INT.`/`EXT.` shorthand keeps its kind.
#[test]
fn scene_heading_without_int_ext_shorthand_keeps_its_kind() {
    for body in ["COLD HARBOR - DAY", "THE HARBOUR - NIGHT", "EXT/INT. THE WRECK - CONTINUOUS"] {
        let mut doc = make_doc("doc-scene-probe", "Scene Probe", Medium::Screenplay);
        doc.set_script_blocks(vec![Block { kind: ElementKind::SceneHeading, text: body.into() }]);
        let back = roundtrip_doc(&doc);
        let blocks = script_blocks(&back);
        assert_eq!(blocks.len(), 1, "{body}: lost the block");
        assert_eq!(
            blocks[0].kind,
            ElementKind::SceneHeading,
            "{body}: came back as {:?}",
            blocks[0].kind
        );
    }
}

/// An action line written in capitals keeps its kind.
#[test]
fn action_written_in_capitals_keeps_its_kind() {
    let mut doc = make_doc("doc-action-probe", "Action Probe", Medium::Screenplay);
    doc.set_script_blocks(vec![Block { kind: ElementKind::Action, text: "THE SEA HAS GONE OUT".into() }]);
    let back = roundtrip_doc(&doc);
    let blocks = script_blocks(&back);
    assert_eq!(blocks.len(), 1, "the action block was lost");
    assert_eq!(
        blocks[0].kind,
        ElementKind::Action,
        "an ALL-CAPS action came back as {:?}",
        blocks[0].kind
    );
}

#[test]
fn prose_document_text_is_verbatim() {
    let original = "The tide went out that morning, and did not come back.\n\nLine two stays.";
    let mut doc = make_doc("doc-prose-probe", "Prose Probe", Medium::Prose);
    doc.set_plain_text(original);
    let back = roundtrip_doc(&doc);
    assert_eq!(back.body_text(), original, "prose body was not verbatim");
}

#[test]
fn manifest_carries_no_long_bodies() {
    let dir = scratch("rt1-manifest");
    let ws = Workspace::demo();
    storage::save_folder(&ws.project, &dir).expect("save folder");

    let manifest = std::fs::read_to_string(dir.join("project.json")).expect("read manifest");
    assert!(
        !manifest.contains("The tide went out"),
        "a document body leaked into project.json"
    );

    // Rich document bodies must be externalised from the manifest.
    let value: serde_json::Value = serde_json::from_str(&manifest).expect("manifest is JSON");
    for doc in value["documents"].as_array().expect("documents array") {
        assert_eq!(
            doc["content"]["paragraphs"].as_array().expect("paragraphs").len(),
            0,
            "document content not emptied in manifest"
        );
        assert_eq!(
            doc["synopsis"]["paragraphs"].as_array().expect("paragraphs").len(),
            0,
            "document synopsis not emptied in manifest"
        );
        assert_eq!(
            doc["notes"]["paragraphs"].as_array().expect("paragraphs").len(),
            0,
            "document notes not emptied in manifest"
        );
        for snap in doc["snapshots"].as_array().expect("snapshots array") {
            assert_eq!(
                snap["content"]["paragraphs"].as_array().expect("paragraphs").len(),
                0,
                "snapshot content not emptied in manifest"
            );
        }
    }
    for item in value["items"].as_array().expect("items array") {
        assert_eq!(
            item["notes"]["paragraphs"].as_array().expect("paragraphs").len(),
            0,
            "database item notes not emptied in manifest"
        );
    }
    for cp in value["checkpoints"].as_array().expect("checkpoints array") {
        assert_eq!(cp["payload"], "", "checkpoint payload not emptied in manifest");
    }

    // The phrase must live in a rich-document body file instead.
    let docs_dir = dir.join("documents");
    let mut found_in_a_body_file = false;
    for entry in std::fs::read_dir(&docs_dir).expect("read documents dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let body = std::fs::read_to_string(&path).expect("read body");
            if body.contains("The tide went out") {
                found_in_a_body_file = true;
            }
        }
    }
    assert!(
        found_in_a_body_file,
        "the prose body was not written to documents/*.rich.json"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn checkpoint_payload_lives_in_its_own_file() {
    let dir = scratch("rt1-checkpoint");
    let ws = Workspace::demo();
    storage::save_folder(&ws.project, &dir).expect("save folder");

    let manifest = std::fs::read_to_string(dir.join("project.json")).expect("read manifest");
    let value: serde_json::Value = serde_json::from_str(&manifest).expect("manifest is JSON");
    let checkpoints = value["checkpoints"].as_array().expect("checkpoints array");
    assert!(!checkpoints.is_empty(), "the demo project has no checkpoint to externalise");

    for cp in checkpoints {
        let id = cp["id"].as_str().expect("checkpoint id");
        assert_eq!(cp["payload"], "", "manifest holds a checkpoint payload");
        let path = dir.join("checkpoints").join(format!("{id}.json"));
        assert!(path.is_file(), "missing checkpoint file {id}.json");
        let text = std::fs::read_to_string(&path).expect("read checkpoint");
        let full: serde_json::Value = serde_json::from_str(&text).expect("checkpoint JSON");
        let payload = full["payload"].as_str().expect("payload is a string");
        assert!(!payload.is_empty(), "checkpoint file {id}.json has an empty payload");
        // The payload is the old single-file project serialisation.
        let inner: serde_json::Value =
            serde_json::from_str(payload).expect("checkpoint payload is the project JSON");
        assert!(inner.get("documents").is_some(), "payload is not a project serialisation");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn folder_roundtrip_keeps_its_statistics() {
    let mut ws = Workspace::demo();
    let before = ws.execute("stats", json!({}));
    assert!(before.ok, "stats failed: {}", before.message);
    let dir = scratch("rt1-stats");
    ws.save(Some(dir.clone())).expect("save folder");
    assert!(dir.is_dir(), "folder save did not create a directory");
    assert!(
        dir.join("project.json").is_file(),
        "folder save did not write a manifest"
    );

    let mut reopened = Workspace::open(&dir).expect("open folder");
    let after = reopened.execute("stats", json!({}));
    assert!(after.ok, "stats failed after reopen: {}", after.message);
    assert_eq!(
        before.data, after.data,
        "folder round trip changed the statistics"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_json_path_is_not_a_project() {
    let path = scratch("rt1-legacy-json").with_extension("portmanteau.json");
    let err = Workspace::open(&path)
        .err()
        .expect("a .json path must not open");
    assert!(
        err.contains("expected a .portmanteau folder"),
        "unclear error: {err}"
    );
}

#[test]
fn folder_save_leaves_no_orphans_after_a_rename() {
    let dir = scratch("rt1-orphans");
    let mut ws = Workspace::demo();
    ws.save(Some(dir.clone())).expect("first save");
    let first: Vec<String> = std::fs::read_dir(dir.join("documents"))
        .expect("documents")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();

    // Rename a document and re-save: the old file must not survive.
    let mut ws2 = Workspace::open(&dir).expect("reopen");
    let victim = ws2
        .project
        .documents
        .iter()
        .find(|d| d.title == "Chapter One · The Cartographer")
        .expect("chapter one")
        .clone();
    let renamed = ws2.execute(
        "document.update",
        json!({ "id": victim.id, "title": "Chapter One · Renamed" }),
    );
    assert!(renamed.ok, "rename failed: {}", renamed.message);
    ws2.save(Some(dir.clone())).expect("second save");

    let second: Vec<String> = std::fs::read_dir(dir.join("documents"))
        .expect("documents")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_ne!(first, second, "the rename did not change any file name");
    for name in &first {
        if !second.contains(name) {
            let path = dir.join("documents").join(name);
            assert!(!path.exists(), "orphan left behind: {name}");
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn storage_fountain_roundtrips_every_kind() {
    // Direct check of the two new core entry points.
    for kind in ALL_KINDS {
        let body = representative_text(kind);
        let blocks = vec![Block { kind, text: body.clone() }];
        let text = script::to_storage_fountain(&blocks, Medium::Screenplay);
        let back = script::parse_fountain(&text, Medium::Screenplay);
        assert_eq!(back.len(), 1, "{kind:?} did not parse back to one block: {text:?}");
        assert_eq!(back[0].kind, kind, "{kind:?} kind lost");
        assert_eq!(back[0].text, body, "{kind:?} text lost");
    }
}

#[test]
fn a_research_note_is_a_document_whose_details_and_body_round_trip() {
    let mut project = Project::new("Probe", ProjectKind::Novel);
    let folder = project.folder_id("Research").expect("research folder");
    let id = project.add_document(Some(folder), "Probe Note", portmanteau_core::DocKind::Research, Medium::Note);
    let doc = project.document_mut(&id).unwrap();
    doc.set_plain_text("A distinctive research body sentence.");
    let meta = doc.research_meta();
    meta.kind = "Interview".into();
    meta.source = "Recorded Tuesday".into();
    meta.attachments.push("tape.mp3".into());
    let dir = scratch("rt1-research");
    storage::save_folder(&project, &dir).expect("save");
    let manifest = std::fs::read_to_string(dir.join("project.json")).expect("manifest");
    assert!(
        !manifest.contains("A distinctive research body sentence."),
        "research body leaked into the manifest"
    );
    assert!(manifest.contains("Recorded Tuesday"), "the research details are in the manifest");
    let back = storage::load_folder(&dir).expect("load");
    let note = back.document(&id).expect("note");
    assert_eq!(note.content.plain_text(), "A distinctive research body sentence.");
    let meta = note.research.clone().expect("details");
    assert_eq!((meta.kind.as_str(), meta.source.as_str()), ("Interview", "Recorded Tuesday"));
    assert_eq!(meta.attachments, ["tape.mp3"]);
    assert_eq!(back.research_docs().len(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn long_form_metadata_and_database_notes_roundtrip_as_rich_documents() {
    let mut project = Project::new("Rich fields", ProjectKind::Novel);
    let mut doc = make_doc("doc-rich-fields", "Chapter", Medium::Prose);
    doc.set_synopsis_text("A rich synopsis.");
    doc.set_notes_text("A rich note.");
    assert!(doc.synopsis.toggle_mark(2..6, portmanteau_core::InlineMark::Italic));
    project.documents.push(doc);

    let mut item = portmanteau_core::DatabaseItem::new(
        portmanteau_core::ItemKind::Character,
        "Mara Venn",
    );
    item.notes.set_plain_text("A character's long note.", "body");
    project.items.push(item);

    let dir = scratch("rt1-rich-long-fields");
    storage::save_folder(&project, &dir).expect("save");
    let restored = storage::load_folder(&dir).expect("load");
    let doc = restored
        .document("doc-rich-fields")
        .expect("document was restored");
    assert_eq!(doc.synopsis.plain_text(), "A rich synopsis.");
    assert_eq!(doc.notes.plain_text(), "A rich note.");
    assert_eq!(restored.items[0].notes.plain_text(), "A character's long note.");
    let run = doc.synopsis.paragraphs[0]
        .content
        .iter()
        .find_map(|item| match item {
            portmanteau_core::InlineContent::Text(run) if run.text == "rich" => Some(run),
            _ => None,
        })
        .expect("the selected synopsis run should survive the round trip");
    assert!(run.marks.contains(&portmanteau_core::InlineMark::Italic));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn resaving_keeps_attachment_assets() {
    let project = Project::new("Probe", ProjectKind::Novel);
    let dir = scratch("rt25-assets");
    storage::save_folder(&project, &dir).expect("save");

    let asset_dir = dir.join("assets").join("research").join("res-probe");
    std::fs::create_dir_all(&asset_dir).expect("asset dir");
    std::fs::write(asset_dir.join("note.png"), b"PNG-DATA").expect("asset");

    // The package is rebuilt in a temporary directory and swapped in, so a
    // second save must carry the attachment binaries across.
    storage::save_folder(&project, &dir).expect("re-save");
    let kept = std::fs::read(asset_dir.join("note.png")).expect("asset survived the save");
    assert_eq!(kept, b"PNG-DATA");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn snapshot_body_is_externalised_and_restored() {
    let mut project = Project::new("Probe", ProjectKind::Novel);
    let mut doc = make_doc("doc-snap-probe", "Probe Doc", Medium::Prose);
    doc.snapshots.push(Snapshot {
        id: "snap-probe".into(),
        name: "First Pass".into(),
        created: portmanteau_core::now_stamp(),
        note: String::new(),
        content: portmanteau_core::RichDocument::from_plain_text(
            "A distinctive snapshot body sentence.",
            "body",
        ),
        words: 6,
    });
    project.documents.push(doc);
    let dir = scratch("rt1-snapshot");
    storage::save_folder(&project, &dir).expect("save");
    let manifest = std::fs::read_to_string(dir.join("project.json")).expect("manifest");
    assert!(
        !manifest.contains("A distinctive snapshot body sentence."),
        "snapshot body leaked into the manifest"
    );
    let back = storage::load_folder(&dir).expect("load");
    let doc = back
        .documents
        .iter()
        .find(|d| d.id == "doc-snap-probe")
        .expect("the probed document came back");
    assert_eq!(
        doc.snapshots[0].body_text(),
        "A distinctive snapshot body sentence."
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// A checkpoint stored in a folder package can still be restored.
#[test]
fn checkpoint_can_be_restored_from_a_folder_package() {
    let dir = scratch("rt1-cp-restore");
    let mut ws = Workspace::demo();
    ws.save(Some(dir.clone())).expect("save folder");
    let cp_id = ws.project.checkpoints[0].id.clone();

    let mut reopened = Workspace::open(&dir).expect("open folder");
    assert!(
        !reopened.project.checkpoints[0].payload.is_empty(),
        "the checkpoint payload was not re-attached on load"
    );

    let target = reopened.project.documents[2].id.clone();
    let renamed = reopened.execute(
        "document.update",
        json!({ "id": target, "title": "MUTATED TITLE" }),
    );
    assert!(renamed.ok, "mutation failed: {}", renamed.message);
    assert!(reopened
        .project
        .documents
        .iter()
        .any(|d| d.title == "MUTATED TITLE"));

    let restored = reopened.execute("project.restore_checkpoint", json!({ "id": cp_id }));
    assert!(restored.ok, "restore failed: {}", restored.message);
    assert!(
        !reopened
            .project
            .documents
            .iter()
            .any(|d| d.title == "MUTATED TITLE"),
        "restoring the checkpoint did not revert the mutation"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// The folder package holds every file type the spec lists.
#[test]
fn folder_package_contains_every_expected_file_type() {
    let dir = scratch("rt1-layout");
    let mut ws = Workspace::demo();
    ws.save(Some(dir.clone())).expect("save folder");

    assert!(dir.join("project.json").is_file(), "no project.json manifest");
    assert!(dir.join("assets").is_dir(), "no assets directory");

    let mut rich_docs = 0;
    for entry in std::fs::read_dir(dir.join("documents")).expect("documents dir") {
        let path = entry.expect("entry").path();
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("json"));
        assert!(path.file_name().unwrap().to_string_lossy().ends_with(".rich.json"));
        rich_docs += 1;
    }
    assert!(rich_docs > 0, "no rich document bodies written");

    let mut item_notes = 0;
    for entry in std::fs::read_dir(dir.join("items")).expect("items dir") {
        let path = entry.expect("entry").path();
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("json"));
        assert!(path.file_name().unwrap().to_string_lossy().ends_with(".rich.json"));
        item_notes += 1;
    }
    assert!(item_notes > 0, "no database item rich notes written");

    let mut snapshots = 0;
    for entry in std::fs::read_dir(dir.join("snapshots")).expect("snapshots dir") {
        let doc_dir = entry.expect("entry").path();
        assert!(doc_dir.is_dir(), "snapshots/ should hold per-document folders");
        for snap in std::fs::read_dir(&doc_dir).expect("snapshot dir") {
            let path = snap.expect("entry").path();
            let ext = path.extension().and_then(|e| e.to_str()).expect("snapshot extension");
            assert_eq!(ext, "json", "unexpected snapshot extension {ext}");
            assert!(
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(".rich.json")
            );
            snapshots += 1;
        }
    }
    assert!(snapshots > 0, "no snapshots written");

    let mut checkpoints = 0;
    for entry in std::fs::read_dir(dir.join("checkpoints")).expect("checkpoints dir") {
        let path = entry.expect("entry").path();
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("json"));
        checkpoints += 1;
    }
    assert!(checkpoints > 0, "no checkpoints written");

    let _ = std::fs::remove_dir_all(dir);
}
