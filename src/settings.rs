//! Global defaults stored outside any project: `$XDG_CONFIG_HOME` (or
//! `~/.config`) under `portmanteau/config.json`. The Settings workspace edits
//! these and can copy them into the current project.

use std::path::PathBuf;

use portmanteau_core::AppSettings;

/// Where the global configuration lives. Follows the XDG base directory
/// convention, then `$HOME`, then the working directory.
pub fn config_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir).join("portmanteau").join("config.json");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home)
                .join(".config")
                .join("portmanteau")
                .join("config.json");
        }
    }
    PathBuf::from("portmanteau-config.json")
}

/// Read the global defaults. A missing or malformed file is not an error:
/// the built-in defaults are used instead.
pub fn load_settings() -> AppSettings {
    match std::fs::read_to_string(config_path()) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

/// Write the global defaults as pretty JSON, creating the folder if needed.
///
/// Unit tests exercise the shell and can toggle layout controls, so they must
/// never overwrite the writer's real configuration. The test binary gets a
/// no-op save; the app itself still persists normally.
pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}
