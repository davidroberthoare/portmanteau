//! Folder-package persistence.
//!
//! A project is a directory ending in `.portmanteau`. `project.json` is the
//! manifest: the whole `Project` with its long bodies emptied. Each document
//! (research notes included), snapshot and checkpoint body lives in its own small file.
//! Filenames are derived from record fields, never stored in the model, so the
//! domain core stays free of filesystem concerns.

use std::path::Path;

use portmanteau_core::*;

#[derive(serde::Serialize, serde::Deserialize)]
struct DocumentBodies {
    content: RichDocument,
    synopsis: RichDocument,
    notes: RichDocument,
}

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

/// A safe single path component derived from an id. Keeps only ASCII
/// alphanumerics and `-`, so a hostile id such as `../../evil` cannot escape
/// the package. Normal ids (`doc-xxxxxxxx`, `cp-xxxxxxxx`) are unaffected.
fn safe_component(s: &str) -> String {
    let out: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    if out.is_empty() {
        "x".into()
    } else {
        out
    }
}

fn id8(id: &str) -> String {
    safe_component(id).chars().take(8).collect()
}

fn document_name(doc: &Document) -> String {
    format!(
        "{:02}-{}-{}.rich.json",
        doc.order,
        slugify(&doc.title),
        id8(&doc.id)
    )
}

fn item_name(index: usize, item: &DatabaseItem) -> String {
    format!(
        "{:02}-{}-{}.rich.json",
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

/// Copy an existing directory tree into the freshly built package. Used for
/// `assets/`: attachment binaries live there and are not part of the manifest.
fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("Could not create {}: {e}", to.display()))?;
    let entries =
        std::fs::read_dir(from).map_err(|e| format!("Could not read {}: {e}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Could not read {}: {e}", from.display()))?;
        let target = to.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Could not inspect {}: {e}", entry.path().display()))?;
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target).map_err(|e| {
                format!(
                    "Could not copy {} to {}: {e}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

fn reset_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_dir_all(path)
            .map_err(|e| format!("Could not clear {}: {e}", path.display()))?;
    }
    std::fs::create_dir_all(path)
        .map_err(|e| format!("Could not create {}: {e}", path.display()))
}

/// A project as the files of its folder package: package-relative paths with
/// forward slashes (`project.json`, `documents/…`, …) and their bytes. Pure: no
/// disk, so the same list feeds a folder on disk, a zip, or browser storage.
/// Attachment binaries under `assets/` are not included; they are not part of
/// the manifest.
pub type PackageFiles = Vec<(String, Vec<u8>)>;

/// The files that make up `project`. See [`PackageFiles`].
pub fn package_files(project: &Project) -> Result<PackageFiles, String> {
    let mut files: PackageFiles = Vec::new();

    let mut manifest = project.clone();
    for doc in &mut manifest.documents {
        doc.content = RichDocument::default();
        doc.synopsis = RichDocument::default();
        doc.notes = RichDocument::default();
        for snap in &mut doc.snapshots {
            snap.content = RichDocument::default();
        }
    }
    for item in &mut manifest.items {
        item.notes = RichDocument::default();
    }
    for cp in &mut manifest.checkpoints {
        cp.payload.clear();
    }
    let manifest_text =
        serde_json::to_string_pretty(&manifest).map_err(|e| format!("Could not serialise: {e}"))?;
    files.push(("project.json".into(), manifest_text.into_bytes()));

    for doc in &project.documents {
        let body = serde_json::to_string_pretty(&DocumentBodies {
            content: doc.content.clone(),
            synopsis: doc.synopsis.clone(),
            notes: doc.notes.clone(),
        })
        .map_err(|e| format!("Could not serialise {}: {e}", doc.title))?;
        files.push((format!("documents/{}", document_name(doc)), body.into_bytes()));
    }

    for (index, item) in project.items.iter().enumerate() {
        let body = serde_json::to_string_pretty(&item.notes)
            .map_err(|e| format!("Could not serialise item {}: {e}", item.title))?;
        files.push((format!("items/{}", item_name(index, item)), body.into_bytes()));
    }

    for doc in &project.documents {
        for snap in &doc.snapshots {
            let body = serde_json::to_string_pretty(&snap.content)
                .map_err(|e| format!("Could not serialise snapshot {}: {e}", snap.name))?;
            files.push((
                format!("snapshots/{}/{}.rich.json", id8(&doc.id), snapshot_stem(snap)),
                body.into_bytes(),
            ));
        }
    }

    for cp in &project.checkpoints {
        let text =
            serde_json::to_string_pretty(cp).map_err(|e| format!("Could not serialise: {e}"))?;
        files.push((format!("checkpoints/{}.json", safe_component(&cp.id)), text.into_bytes()));
    }
    Ok(files)
}

/// Write the full package into `dir`, replacing its managed directories.
/// `dir` is expected to be a fresh temporary directory when called from
/// `save_folder`.
fn write_package(project: &Project, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    for managed in ["documents", "items", "snapshots", "checkpoints"] {
        reset_dir(&dir.join(managed))?;
    }
    std::fs::create_dir_all(dir.join("assets"))
        .map_err(|e| format!("Could not create assets: {e}"))?;
    for (path, bytes) in package_files(project)? {
        let target = dir.join(&path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
        }
        write_file(&target, bytes)?;
    }
    Ok(())
}

/// A unique suffix for the temporary and backup sibling directories. No
/// external crate is needed; the process id plus the current time in
/// nanoseconds is enough to keep concurrent saves apart.
fn unique_suffix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}", std::process::id(), nanos)
}

/// Write a project as a folder package atomically.
///
/// The package is built in a fresh sibling directory and only swapped into
/// place once it is complete, so an interruption cannot destroy the existing
/// project.
pub fn save_folder(project: &Project, dir: &Path) -> Result<(), String> {
    let parent = match dir.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());
    let suffix = unique_suffix();
    let temp = parent.join(format!(".{name}.tmp-{suffix}"));
    let backup = parent.join(format!(".{name}.bak-{suffix}"));

    if temp.exists() {
        let _ = std::fs::remove_dir_all(&temp);
    }
    if let Err(e) = write_package(project, &temp) {
        let _ = std::fs::remove_dir_all(&temp);
        return Err(e);
    }

    // The package is rebuilt from scratch, so carry the attachment binaries
    // across before the swap. `project.json` only stores their basenames.
    let existing_assets = dir.join("assets");
    if existing_assets.is_dir() {
        if let Err(e) = copy_dir(&existing_assets, &temp.join("assets")) {
            let _ = std::fs::remove_dir_all(&temp);
            return Err(e);
        }
    }

    let had_existing = dir.exists();
    if had_existing {
        if backup.exists() {
            let _ = std::fs::remove_dir_all(&backup);
        }
        if let Err(e) = std::fs::rename(dir, &backup) {
            let _ = std::fs::remove_dir_all(&temp);
            return Err(format!("Could not replace {}: {e}", dir.display()));
        }
    }

    match std::fs::rename(&temp, dir) {
        Ok(()) => {
            if had_existing {
                let _ = std::fs::remove_dir_all(&backup);
            }
            Ok(())
        }
        Err(e) => {
            if had_existing {
                let _ = std::fs::rename(&backup, dir);
            }
            let _ = std::fs::remove_dir_all(&temp);
            Err(format!("Could not replace {}: {e}", dir.display()))
        }
    }
}

/// Read a folder package, re-attaching bodies from their derived filenames.
/// A missing body file is left empty; a missing manifest is an error.
pub fn load_folder(dir: &Path) -> Result<Project, String> {
    project_from_files(&|path| std::fs::read(dir.join(path)).ok())
}

/// Rebuild a project from package files. `read` returns the bytes of a
/// package-relative path (as in [`package_files`]), or `None` if absent.
pub fn project_from_files(read: &dyn Fn(&str) -> Option<Vec<u8>>) -> Result<Project, String> {
    let text = read("project.json")
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(|| "Not a Portmanteau project: could not read project.json".to_string())?;
    let mut project: Project =
        serde_json::from_str(&text).map_err(|e| format!("Not a Portmanteau project: {e}"))?;
    let read_text = |path: &str| read(path).and_then(|bytes| String::from_utf8(bytes).ok());

    for doc in &mut project.documents {
        if let Some(body) = read_text(&format!("documents/{}", document_name(doc))) {
            let bodies: DocumentBodies = serde_json::from_str(&body)
                .map_err(|e| format!("Could not read rich text for {}: {e}", doc.title))?;
            doc.content = bodies.content;
            doc.synopsis = bodies.synopsis;
            doc.notes = bodies.notes;
        }
    }

    // A project file from elsewhere is untrusted: an attachment name that is
    // not a bare file name could point outside the note's folder.
    for doc in &mut project.documents {
        if let Some(meta) = &mut doc.research {
            meta.attachments.retain(|name| crate::is_safe_attachment_name(name));
        }
    }

    for (index, item) in project.items.iter_mut().enumerate() {
        if let Some(body) = read_text(&format!("items/{}", item_name(index, item))) {
            item.notes = serde_json::from_str(&body)
                .map_err(|e| format!("Could not read rich text for item {}: {e}", item.title))?;
        }
    }

    for doc in &mut project.documents {
        let stem_dir = id8(&doc.id);
        for snap in &mut doc.snapshots {
            let path = format!("snapshots/{stem_dir}/{}.rich.json", snapshot_stem(snap));
            if let Some(body) = read_text(&path) {
                snap.content = serde_json::from_str(&body)
                    .map_err(|e| format!("Could not read snapshot {}: {e}", snap.name))?;
            }
        }
    }

    for cp in &mut project.checkpoints {
        let path = format!("checkpoints/{}.json", safe_component(&cp.id));
        if let Some(body) = read_text(&path) {
            if let Ok(full) = serde_json::from_str::<Checkpoint>(&body) {
                cp.payload = full.payload;
            }
        }
    }

    Ok(project)
}
