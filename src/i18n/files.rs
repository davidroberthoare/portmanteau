//! Language files in the `languages` folder beside the app's settings.
//! Same approach as theme files: unreadable files are skipped and reported.

use std::path::{Path, PathBuf};

use super::{builtin_languages, Language};

pub fn languages_dir() -> PathBuf {
    let config = crate::settings::config_path();
    config.parent().map(Path::to_path_buf).unwrap_or_default().join("languages")
}

/// The built-in languages, then every readable `*.json` in `dir`. A file whose
/// `code` matches a built-in replaces it (that is how a translation is
/// corrected); otherwise it is added. The second value lists one plain-words
/// problem per skipped file.
pub fn load_all(dir: &Path) -> (Vec<Language>, Vec<String>) {
    let mut languages = builtin_languages();
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
        let result = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| Language::from_json(&text));
        match result {
            Ok(language) => match languages.iter().position(|l| l.code == language.code) {
                Some(index) if language.code == "en" => {
                    problems.push(format!("{file}: English is built in and cannot be replaced"));
                    let _ = index;
                }
                Some(index) => languages[index] = language,
                None => languages.push(language),
            },
            Err(why) => problems.push(format!("{file}: {why}")),
        }
    }
    (languages, problems)
}

/// Copy a language file into `dir` after checking it reads as one. A file for
/// a language already there replaces that language's own file.
pub fn import(dir: &Path, source: &Path) -> Result<Language, String> {
    let text = std::fs::read_to_string(source).map_err(|e| format!("Could not read the file: {e}"))?;
    if text.len() > 2 * 1024 * 1024 {
        return Err("That file is too large to be a language file".into());
    }
    let language = Language::from_json(&text)?;
    if language.code == "en" {
        return Err("English is built in and cannot be replaced".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create the languages folder: {e}"))?;
    std::fs::write(dir.join(format!("{}.json", language.code)), language.to_json())
        .map_err(|e| format!("Could not save the language: {e}"))?;
    Ok(language)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pm-lang-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn user_files_add_languages_replace_built_ins_and_bad_ones_are_reported() {
        let dir = temp("load");
        std::fs::write(dir.join("de.json"), r#"{"code":"de","name":"Deutsch","strings":{"File":"Datei"}}"#).unwrap();
        std::fs::write(dir.join("fr.json"), r#"{"code":"fr","name":"Français (mien)","strings":{"File":"Fichier!"}}"#).unwrap();
        std::fs::write(dir.join("en.json"), r#"{"code":"en","name":"English","strings":{"File":"Phile"}}"#).unwrap();
        std::fs::write(dir.join("broken.json"), "{").unwrap();
        let (languages, problems) = load_all(&dir);
        assert!(languages.iter().any(|l| l.code == "de"));
        let fr = languages.iter().find(|l| l.code == "fr").unwrap();
        assert_eq!(fr.name, "Français (mien)");
        assert_eq!(languages.iter().filter(|l| l.code == "en").count(), 1);
        assert!(languages.iter().find(|l| l.code == "en").unwrap().strings.is_empty());
        assert_eq!(problems.len(), 2, "{problems:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn importing_copies_the_file_into_the_folder() {
        let dir = temp("import");
        let src = dir.join("it.json");
        std::fs::write(&src, r#"{"code":"it","name":"Italiano","strings":{"File":"File"}}"#).unwrap();
        let dest = dir.join("languages");
        let l = import(&dest, &src).unwrap();
        assert_eq!(l.code, "it");
        assert!(dest.join("it.json").exists());
        std::fs::write(&src, "x").unwrap();
        assert!(import(&dest, &src).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
