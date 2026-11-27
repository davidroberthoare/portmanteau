//! The safety copy ("autorecover") of a project that has unsaved changes.
//!
//! Like a word processor, Portmanteau never writes over a project's folder
//! unless the writer says so (Save, or Save in the quit question). While a
//! project has unsaved changes, a copy of it is refreshed every couple of
//! minutes next to the app's settings, and removed once the project is saved or
//! the writer quits without saving. If the app stops any other way, the next
//! start offers to bring the changes back.

use std::path::{Path, PathBuf};
use std::time::Duration;

use portmanteau_api::Workspace;
use serde::{Deserialize, Serialize};

/// How often the copy is refreshed while there are unsaved changes.
pub const EVERY: Duration = Duration::from_secs(120);

/// Where a copy came from, kept beside it.
#[derive(Serialize, Deserialize)]
struct Meta {
    /// The folder the project was open from; none if it was never saved.
    original: Option<PathBuf>,
}

/// The folder the copies live in, beside the settings file.
pub fn folder() -> PathBuf {
    crate::settings::config_path()
        .parent()
        .map(|p| p.join("recovery"))
        .unwrap_or_else(|| PathBuf::from("portmanteau-recovery"))
}

fn stem(dir: &Path, project_id: &str) -> PathBuf {
    let safe: String = project_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    dir.join(safe)
}

fn write_whole(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = target.with_extension("part");
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, target).map_err(|e| e.to_string())
}

/// Write (or refresh) the copy of `ws`. Each file is written whole or not at all.
pub fn write(dir: &Path, ws: &Workspace) -> Result<(), String> {
    let bytes = ws.recovery_bytes()?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let base = stem(dir, &ws.project.id);
    let meta = serde_json::to_vec(&Meta { original: ws.path.clone() }).map_err(|e| e.to_string())?;
    write_whole(&base.with_extension("json"), &meta)?;
    write_whole(&base.with_extension("zip"), &bytes)
}

/// Remove the copy for a project, if there is one.
pub fn remove(dir: &Path, project_id: &str) {
    let base = stem(dir, project_id);
    let _ = std::fs::remove_file(base.with_extension("zip"));
    let _ = std::fs::remove_file(base.with_extension("json"));
}

/// A copy found at start-up, already opened.
pub struct Found {
    pub workspace: Workspace,
    /// When the copy was last refreshed, for the message.
    pub saved: String,
    /// The project's id, to remove the copy afterwards.
    pub project_id: String,
    /// Whether the project had a folder when the copy was made.
    pub original: Option<PathBuf>,
}

/// The newest copy that can be read, if any.
pub fn find(dir: &Path) -> Option<Found> {
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "zip"))
        .filter_map(|path| Some((path.metadata().ok()?.modified().ok()?, path)))
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    for (modified, path) in candidates {
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let original = std::fs::read(path.with_extension("json"))
            .ok()
            .and_then(|m| serde_json::from_slice::<Meta>(&m).ok())
            .and_then(|m| m.original);
        let Ok(workspace) = Workspace::from_recovery(&bytes, original.clone()) else {
            continue;
        };
        let saved = chrono::DateTime::<chrono::Local>::from(modified).format("%-d %B, %H:%M").to_string();
        let project_id = workspace.project.id.clone();
        return Some(Found { workspace, saved, project_id, original });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unsaved_new_project_is_written_found_and_removed() {
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("recovery"));
        let mut ws = Workspace::demo();
        ws.project.title = "Rescue me".into();
        write(&dir, &ws).unwrap();
        let found = find(&dir).expect("the copy is found");
        assert_eq!(found.workspace.project.title, "Rescue me");
        assert!(found.workspace.dirty, "it comes back as unsaved work");
        assert!(found.workspace.path.is_none());
        remove(&dir, &ws.project.id);
        assert!(find(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_saved_project_comes_back_attached_to_its_folder_without_touching_it() {
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("recovery-saved"));
        let home = std::env::temp_dir().join(portmanteau_core::new_id("recovery-home"));
        let mut ws = Workspace::demo();
        ws.save(Some(home.clone())).unwrap();
        let on_disk = std::fs::read_to_string(home.join("project.json")).unwrap();
        ws.project.title = "Changed but not saved".into();
        ws.dirty = true;
        write(&dir, &ws).unwrap();
        assert_eq!(std::fs::read_to_string(home.join("project.json")).unwrap(), on_disk, "the project's own files are left alone");
        let found = find(&dir).expect("found");
        assert_eq!(found.workspace.project.title, "Changed but not saved");
        assert_eq!(found.workspace.path.as_deref(), Some(home.as_path()));
        assert!(found.workspace.dirty);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn junk_in_the_folder_is_ignored() {
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("recovery-junk"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.zip"), b"not a zip").unwrap();
        assert!(find(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
