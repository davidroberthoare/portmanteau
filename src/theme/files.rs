//! Theme files: JSON documents in a `themes` folder beside the app's config.
//!
//! A writer shares a theme by sending the file; installing one is copying it
//! into the folder (the Settings page's Import button does exactly that).
//! Files that cannot be read are skipped and reported, never fatal.

use std::path::{Path, PathBuf};

use super::palette::{builtin_themes, Theme};

/// `<config dir>/portmanteau/themes`, next to `config.json`.
pub fn themes_dir() -> PathBuf {
    let config = crate::settings::config_path();
    config.parent().map(Path::to_path_buf).unwrap_or_default().join("themes")
}

/// The built-in themes, then every readable `*.json` in `dir` (by file name).
/// The second value lists one plain-words problem per file that was skipped.
pub fn load_all(dir: &Path) -> (Vec<Theme>, Vec<String>) {
    let mut themes = builtin_themes();
    let mut problems = Vec::new();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("json")))
                .collect()
        })
        .unwrap_or_default();
    paths.sort();
    for path in paths {
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let id = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let result = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| Theme::from_json(&id, &text));
        match result {
            Ok(theme) if themes.iter().any(|t| t.id == theme.id) => {
                problems.push(format!("{file}: another theme already uses this file name"));
            }
            Ok(theme) => themes.push(theme),
            Err(why) => problems.push(format!("{file}: {why}")),
        }
    }
    (themes, problems)
}

/// File-name-safe id for a theme name: lowercase letters, digits and dashes.
pub fn id_for_name(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let slug = slug.split('-').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("-");
    if slug.is_empty() { "theme".into() } else { slug }
}

/// Write a theme into `dir`, picking a file name that does not exist yet.
/// Returns the saved theme (with its new id).
pub fn save_new(dir: &Path, theme: &Theme) -> Result<Theme, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create the themes folder: {e}"))?;
    let base = id_for_name(&theme.name);
    let taken = |id: &str| dir.join(format!("{id}.json")).exists() || builtin_themes().iter().any(|t| t.id == id);
    let mut id = base.clone();
    let mut n = 2;
    while taken(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    let saved = Theme { id: id.clone(), builtin: false, ..theme.clone() };
    write(dir, &saved)?;
    Ok(saved)
}

/// Overwrite an existing user theme's file.
pub fn write(dir: &Path, theme: &Theme) -> Result<(), String> {
    let path = dir.join(format!("{}.json", theme.id));
    let tmp = dir.join(format!("{}.json.tmp", theme.id));
    std::fs::write(&tmp, theme.to_json()).map_err(|e| format!("Could not save the theme: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Could not save the theme: {e}"))
}

pub fn delete(dir: &Path, theme: &Theme) -> Result<(), String> {
    if theme.builtin {
        return Err("Built-in themes cannot be deleted".into());
    }
    std::fs::remove_file(dir.join(format!("{}.json", theme.id))).map_err(|e| format!("Could not delete the theme: {e}"))
}

/// Copy a shared theme file into `dir` after checking it reads as a theme.
pub fn import(dir: &Path, source: &Path) -> Result<Theme, String> {
    let text = std::fs::read_to_string(source).map_err(|e| format!("Could not read the file: {e}"))?;
    if text.len() > 256 * 1024 {
        return Err("That file is too large to be a theme".into());
    }
    let parsed = Theme::from_json("import", &text)?;
    save_new(dir, &parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pm-themes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn user_themes_load_after_built_ins_and_bad_files_are_reported() {
        let dir = temp("load");
        std::fs::write(dir.join("sea.json"), r##"{"name":"Sea","dark":true,"colors":{"bg":"#001122"}}"##).unwrap();
        std::fs::write(dir.join("broken.json"), "{ nope").unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
        let (themes, problems) = load_all(&dir);
        assert_eq!(themes.len(), builtin_themes().len() + 1);
        let sea = themes.last().unwrap();
        assert_eq!((sea.id.as_str(), sea.name.as_str(), sea.builtin), ("sea", "Sea", false));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("broken.json"), "{problems:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_folder_just_means_built_ins_only() {
        let (themes, problems) = load_all(Path::new("/no/such/folder/for/themes"));
        assert_eq!(themes.len(), builtin_themes().len());
        assert!(problems.is_empty());
    }

    #[test]
    fn import_copies_under_a_free_name_and_refuses_junk() {
        let dir = temp("import");
        let src = dir.join("incoming.json");
        std::fs::write(&src, r##"{"name":"Light","colors":{"accent":"#aa0000"}}"##).unwrap();
        let dest = dir.join("themes");
        let first = import(&dest, &src).unwrap();
        // "light" is a built-in id, so the copy gets a different file name.
        assert_eq!(first.id, "light-2");
        let second = import(&dest, &src).unwrap();
        assert_eq!(second.id, "light-3");
        std::fs::write(&src, "not json").unwrap();
        assert!(import(&dest, &src).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names_become_safe_file_names() {
        assert_eq!(id_for_name("My  Sea / Night!"), "my-sea-night");
        assert_eq!(id_for_name("../../etc"), "etc");
        assert_eq!(id_for_name("???"), "theme");
    }
}
