//! A project as one zip: round trip, attachments, and refusal of hostile zips.

use std::io::Write;
use std::path::PathBuf;

use portmanteau_api::{archive, storage, Workspace};
use portmanteau_core::*;
use serde_json::json;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(new_id(tag));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, body) in files {
        zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(body).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[test]
fn a_project_survives_a_trip_through_a_zip_in_memory() {
    // The Salt Road has snapshots, research, database items and checkpoints.
    let original = sample::demo();
    let bytes = archive::pack_zip(&original, Vec::new()).unwrap();
    let back = archive::unpack_zip(&bytes).unwrap();
    assert_eq!(back.project.title, original.title);
    assert_eq!(back.project.documents.len(), original.documents.len());
    for (a, b) in original.documents.iter().zip(&back.project.documents) {
        assert_eq!(a.content, b.content, "{}", a.title);
        assert_eq!(a.synopsis, b.synopsis);
        assert_eq!(a.notes, b.notes);
        assert_eq!(a.snapshots.len(), b.snapshots.len());
        for (x, y) in a.snapshots.iter().zip(&b.snapshots) {
            assert_eq!(x.content, y.content);
        }
    }
    assert_eq!(original.items, back.project.items);
    assert_eq!(original.checkpoints.len(), back.project.checkpoints.len());
}

#[test]
fn the_package_files_alone_rebuild_the_project() {
    let original = sample::welcome();
    let files = storage::package_files(&original).unwrap();
    assert!(files.iter().any(|(p, _)| p == "project.json"));
    let lookup: std::collections::HashMap<_, _> = files.into_iter().collect();
    let back = storage::project_from_files(&|p| lookup.get(p).cloned()).unwrap();
    assert_eq!(back.documents.len(), original.documents.len());
    assert!(back.documents.iter().any(|d| !d.snapshots.is_empty() && !d.snapshots[0].content.is_empty()));
}

#[test]
fn a_saved_project_with_attachments_packs_and_opens_elsewhere() {
    let root = scratch("zip-roundtrip");
    let project_dir = root.join("book.portmanteau");
    let mut ws = Workspace::new_project("Zipped", ProjectKind::Novel);
    ws.save(Some(project_dir.clone())).unwrap();
    // An attachment and a picture already live under assets/.
    std::fs::create_dir_all(project_dir.join("assets/research/note1")).unwrap();
    std::fs::write(project_dir.join("assets/research/note1/map.png"), b"PNGDATA").unwrap();
    std::fs::create_dir_all(project_dir.join("assets/images")).unwrap();
    std::fs::write(project_dir.join("assets/images/cover.jpg"), b"JPGDATA").unwrap();
    // An unsaved edit travels too: the zip is of the project as it is now.
    ws.project.subtitle = "Edited after saving".into();

    let zip = root.join("backup.portmanteau.zip");
    let res = ws.execute("project.pack", json!({"path": zip.display().to_string()}));
    assert!(res.ok, "{res:?}");
    assert!(!root.join("backup.portmanteau.zip.part").exists());

    let elsewhere = root.join("elsewhere.portmanteau");
    let mut other = Workspace::new_project("Other", ProjectKind::Novel);
    let res = other.execute("project.open_zip", json!({"path": zip.display().to_string(), "folder": elsewhere.display().to_string()}));
    assert!(res.ok, "{res:?}");
    assert_eq!(other.project.title, "Zipped");
    assert_eq!(other.project.subtitle, "Edited after saving");
    assert_eq!(other.path.as_deref(), Some(elsewhere.as_path()));
    assert_eq!(std::fs::read(elsewhere.join("assets/research/note1/map.png")).unwrap(), b"PNGDATA");
    assert_eq!(std::fs::read(elsewhere.join("assets/images/cover.jpg")).unwrap(), b"JPGDATA");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn opening_a_zip_never_overwrites_an_existing_project() {
    let root = scratch("zip-no-overwrite");
    let zip = root.join("p.zip");
    std::fs::write(&zip, archive::pack_zip(&sample::welcome(), Vec::new()).unwrap()).unwrap();
    let existing = root.join("mine.portmanteau");
    std::fs::create_dir_all(&existing).unwrap();
    std::fs::write(existing.join("precious.txt"), b"keep").unwrap();
    let error = Workspace::open_zip(&zip, &existing).err().expect("refused");
    assert!(error.contains("already exists"), "{error}");
    assert_eq!(std::fs::read(existing.join("precious.txt")).unwrap(), b"keep");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_zip_made_by_zipping_the_folder_is_understood() {
    let files = storage::package_files(&sample::welcome()).unwrap();
    let wrapped: Vec<(String, Vec<u8>)> =
        files.into_iter().map(|(p, b)| (format!("My Novel.portmanteau/{p}"), b)).collect();
    let mut with_junk = wrapped.clone();
    with_junk.push(("__MACOSX/._project.json".into(), b"junk".to_vec()));
    let refs: Vec<(&str, &[u8])> = with_junk.iter().map(|(p, b)| (p.as_str(), b.as_slice())).collect();
    let back = archive::unpack_zip(&zip_of(&refs)).unwrap();
    assert_eq!(back.project.title, "Welcome to Portmanteau");
}

#[test]
fn hostile_or_wrong_zips_are_refused() {
    let escape = zip_of(&[("../evil.txt", b"x"), ("project.json", b"{}")]);
    assert!(archive::unpack_zip(&escape).err().unwrap().contains("unsafe"));
    let absolute = zip_of(&[("/etc/passwd", b"x"), ("project.json", b"{}")]);
    assert!(archive::unpack_zip(&absolute).is_err());
    let not_a_project = zip_of(&[("notes.txt", b"hello")]);
    assert!(archive::unpack_zip(&not_a_project).err().unwrap().contains("Not a Portmanteau project"));
    let two_projects = zip_of(&[("a/project.json", b"{}"), ("b/project.json", b"{}")]);
    assert!(archive::unpack_zip(&two_projects).is_err());
    let bad_manifest = zip_of(&[("project.json", b"not json")]);
    assert!(archive::unpack_zip(&bad_manifest).is_err());
    assert!(archive::unpack_zip(b"this is not a zip").is_err());
}

#[test]
fn unsafe_attachment_names_in_a_zipped_manifest_are_dropped() {
    let mut project = sample::demo();
    let note = project.documents.iter_mut().find(|d| d.research.is_some()).expect("a research note");
    note.research.as_mut().unwrap().attachments = vec!["ok.png".into(), "../../escape.png".into()];
    let back = archive::unpack_zip(&archive::pack_zip(&project, Vec::new()).unwrap()).unwrap();
    let attachments = back.project.documents.iter().find_map(|d| d.research.as_ref().filter(|r| !r.attachments.is_empty())).unwrap();
    assert_eq!(attachments.attachments, vec!["ok.png".to_string()]);
}

#[test]
fn the_catalogue_lists_the_zip_commands() {
    let names: Vec<String> = portmanteau_api::command_catalogue()
        .iter()
        .filter_map(|c| c["name"].as_str().map(str::to_string))
        .collect();
    assert!(names.contains(&"project.pack".to_string()) && names.contains(&"project.open_zip".to_string()));
}

#[test]
fn a_project_round_trips_through_zip_bytes_in_memory_with_its_attachments() {
    let dir = std::env::temp_dir().join(format!("portmanteau-bytes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("assets/images")).unwrap();
    std::fs::write(dir.join("assets/images/pic.png"), b"not really a png").unwrap();
    let mut ws = Workspace::welcome();
    ws.save(Some(dir.clone())).unwrap();
    let bytes = Workspace::open(&dir).unwrap().zip_bytes().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    let opened = Workspace::from_zip_bytes(&bytes).unwrap();
    assert!(opened.path.is_none() && !opened.dirty);
    assert_eq!(opened.project.title, ws.project.title);
    assert_eq!(opened.memory_assets.len(), 1, "the attachment is kept in memory");
    // Saved again from memory, nothing is lost.
    let again = Workspace::from_zip_bytes(&opened.zip_bytes().unwrap()).unwrap();
    assert_eq!(again.memory_assets, opened.memory_assets);
    assert_eq!(again.project.documents.len(), opened.project.documents.len());
}
