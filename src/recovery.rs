//! A safety copy of a project that has no home yet.
//!
//! A saved project is autosaved in place, but a new or sample project that has
//! never been saved lives only in memory. While it has changes, a copy is
//! written (as a project `.zip`) next to the app's settings, and removed when
//! the project is saved, or the writer chooses to quit without it. If the app
//! stops any other way, the next start offers to bring the work back.

use std::path::{Path, PathBuf};
use std::time::Duration;

use portmanteau_api::Workspace;

/// How often the copy is refreshed while there are unsaved changes.
pub const EVERY: Duration = Duration::from_secs(10);

/// The folder the copies live in, beside the settings file.
pub fn folder() -> PathBuf {
    crate::settings::config_path()
        .parent()
        .map(|p| p.join("recovery"))
        .unwrap_or_else(|| PathBuf::from("portmanteau-recovery"))
}

fn file_for(dir: &Path, project_id: &str) -> PathBuf {
    let safe: String = project_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    dir.join(format!("{safe}.zip"))
}

/// Write (or refresh) the copy of `ws`. The file is written whole or not at all.
pub fn write(dir: &Path, ws: &Workspace) -> Result<(), String> {
    let bytes = ws.zip_bytes()?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let target = file_for(dir, &ws.project.id);
    let temp = target.with_extension("zip.part");
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, &target).map_err(|e| e.to_string())
}

/// Remove the copy for a project, if there is one.
pub fn remove(dir: &Path, project_id: &str) {
    let _ = std::fs::remove_file(file_for(dir, project_id));
}

/// A copy found at start-up, already opened.
pub struct Found {
    pub path: PathBuf,
    pub workspace: Workspace,
    pub saved: String,
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
        let Ok(mut workspace) = Workspace::from_zip_bytes(&bytes) else {
            continue;
        };
        workspace.dirty = true;
        let saved = chrono::DateTime::<chrono::Local>::from(modified).format("%-d %B, %H:%M").to_string();
        return Some(Found { path, workspace, saved });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_is_written_found_and_removed() {
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
    fn junk_in_the_folder_is_ignored() {
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("recovery-junk"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.zip"), b"not a zip").unwrap();
        assert!(find(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
