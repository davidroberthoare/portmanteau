//! Folder-package persistence.
//!
//! A project is a directory ending in `.scriptorium`. `project.json` is the
//! manifest: the whole `Project` with its long bodies emptied. Each document,
//! research note, snapshot and checkpoint body lives in its own small file.
//! Filenames are derived from record fields, never stored in the model, so the
//! domain core stays free of filesystem concerns.

use std::path::Path;

use msc_core::*;

/// A readable filename fragment: lowercase ASCII alphanumerics, with runs of
/// other characters collapsed to a single dash.
pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "untitled".into()
    } else {
        out
    }
}

fn id8(id: &str) -> String {
    id.chars().take(8).collect()
}

fn document_name(doc: &Document) -> String {
    let ext = if doc.medium.is_script() {
        "fountain"
    } else {
        "md"
    };
    format!(
        "{:02}-{}-{}.{}",
        doc.order,
        slugify(&doc.title),
        id8(&doc.id),
        ext
    )
}

fn research_name(index: usize, item: &ResearchItem) -> String {
    format!(
        "{:02}-{}-{}.md",
        index,
        slugify(&item.title),
        id8(&item.id)
    )
}

fn snapshot_stem(snap: &Snapshot) -> String {
    format!("{}-{}", id8(&snap.id), slugify(&snap.name))
}

fn write_file(path: &Path, body: impl AsRef<[u8]>) -> Result<(), String> {
    std::fs::write(path, body).map_err(|e| format!("Could not write {}: {e}", path.display()))
}

fn reset_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_dir_all(path)
            .map_err(|e| format!("Could not clear {}: {e}", path.display()))?;
    }
    std::fs::create_dir_all(path)
        .map_err(|e| format!("Could not create {}: {e}", path.display()))
}

/// Write a project as a folder package, replacing the managed directories.
pub fn save_folder(project: &Project, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;

    let documents = dir.join("documents");
    let research = dir.join("research");
    let snapshots = dir.join("snapshots");
    let checkpoints = dir.join("checkpoints");
    let assets = dir.join("assets");
    reset_dir(&documents)?;
    reset_dir(&research)?;
    reset_dir(&snapshots)?;
    reset_dir(&checkpoints)?;
    std::fs::create_dir_all(&assets).map_err(|e| format!("Could not create assets: {e}"))?;

    let mut manifest = project.clone();
    for doc in &mut manifest.documents {
        doc.text.clear();
        doc.blocks.clear();
        for snap in &mut doc.snapshots {
            snap.text.clear();
            snap.blocks.clear();
        }
    }
    for item in &mut manifest.research {
        item.body.clear();
    }
    for cp in &mut manifest.checkpoints {
        cp.payload.clear();
    }
    let manifest_text =
        serde_json::to_string_pretty(&manifest).map_err(|e| format!("Could not serialise: {e}"))?;
    write_file(&dir.join("project.json"), manifest_text)?;

    for doc in &project.documents {
        let body = if doc.medium.is_script() {
            script::to_storage_fountain(&doc.blocks, doc.medium)
        } else {
            doc.text.clone()
        };
        write_file(&documents.join(document_name(doc)), body)?;
    }

    for (index, item) in project.research.iter().enumerate() {
        write_file(&research.join(research_name(index, item)), &item.body)?;
    }

    for doc in &project.documents {
        if doc.snapshots.is_empty() {
            continue;
        }
        let doc_dir = snapshots.join(id8(&doc.id));
        std::fs::create_dir_all(&doc_dir)
            .map_err(|e| format!("Could not create {}: {e}", doc_dir.display()))?;
        for snap in &doc.snapshots {
            let (ext, body) = if snap.blocks.is_empty() {
                ("md", snap.text.clone())
            } else {
                (
                    "fountain",
                    script::to_storage_fountain(&snap.blocks, doc.medium),
                )
            };
            let name = format!("{}.{}", snapshot_stem(snap), ext);
            write_file(&doc_dir.join(name), body)?;
        }
    }

    for cp in &project.checkpoints {
        let text =
            serde_json::to_string_pretty(cp).map_err(|e| format!("Could not serialise: {e}"))?;
        write_file(&checkpoints.join(format!("{}.json", cp.id)), text)?;
    }

    Ok(())
}

/// Read a folder package, re-attaching bodies from their derived filenames.
/// A missing body file is left empty; a missing manifest is an error.
pub fn load_folder(dir: &Path) -> Result<Project, String> {
    let manifest_path = dir.join("project.json");
    let text = std::fs::read_to_string(&manifest_path).map_err(|e| {
        format!(
            "Not a Scriptorium project: could not read {}: {e}",
            manifest_path.display()
        )
    })?;
    let mut project: Project =
        serde_json::from_str(&text).map_err(|e| format!("Not a Scriptorium project: {e}"))?;

    let documents = dir.join("documents");
    for doc in &mut project.documents {
        let path = documents.join(document_name(doc));
        if let Ok(body) = std::fs::read_to_string(&path) {
            if doc.medium.is_script() {
                doc.blocks = script::parse_fountain(&body, doc.medium);
                doc.text.clear();
            } else {
                doc.text = body;
                doc.blocks.clear();
            }
        }
    }

    let research = dir.join("research");
    for (index, item) in project.research.iter_mut().enumerate() {
        let path = research.join(research_name(index, item));
        if let Ok(body) = std::fs::read_to_string(&path) {
            item.body = body;
        }
    }

    let snapshots = dir.join("snapshots");
    for doc in &mut project.documents {
        if doc.snapshots.is_empty() {
            continue;
        }
        let doc_dir = snapshots.join(id8(&doc.id));
        for snap in &mut doc.snapshots {
            let stem = snapshot_stem(snap);
            let fountain = doc_dir.join(format!("{stem}.fountain"));
            let markdown = doc_dir.join(format!("{stem}.md"));
            if let Ok(body) = std::fs::read_to_string(&fountain) {
                snap.blocks = script::parse_fountain(&body, doc.medium);
                snap.text.clear();
            } else if let Ok(body) = std::fs::read_to_string(&markdown) {
                snap.text = body;
                snap.blocks.clear();
            }
        }
    }

    let checkpoints = dir.join("checkpoints");
    for cp in &mut project.checkpoints {
        let path = checkpoints.join(format!("{}.json", cp.id));
        if let Ok(body) = std::fs::read_to_string(&path) {
            if let Ok(full) = serde_json::from_str::<Checkpoint>(&body) {
                cp.payload = full.payload;
            }
        }
    }

    Ok(project)
}
