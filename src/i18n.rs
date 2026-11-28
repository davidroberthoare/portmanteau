//! Interface language.
//!
//! Every piece of text the app draws is written in English in the code, as
//! `tr("Save checkpoint")`. A language file maps those English texts to
//! another language; with no file (or no entry) the English shows. The English
//! text is the key, so nothing needs numbering and a missing translation is
//! never blank.
//!
//! Language files are small JSON documents, in the same spirit as theme files:
//!
//! ```json
//! { "code": "fr", "name": "Français", "strings": { "File": "Fichier" } }
//! ```
//!
//! They live in a `languages` folder beside the app's settings (`files`),
//! and a file there with the same `code` as a built-in language replaces it,
//! which is how someone customises or corrects a translation.
//! `docs/localization.md` explains the rest.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Mutex, RwLock};

pub mod files;

/// The built-in language files, compiled into the app.
const BUILTIN: &[&str] = &[include_str!("../assets/languages/fr.json")];

/// Every English text the app knows, as an empty translation file to start from.
pub const TEMPLATE: &str = include_str!("../assets/languages/template.json");

#[derive(Clone, Debug, PartialEq)]
pub struct Language {
    /// Short code such as `en`, `fr`, `pt-BR`. Also the file name.
    pub code: String,
    /// The language's own name, as shown in the picker ("Français").
    pub name: String,
    /// English text -> translation.
    pub strings: BTreeMap<String, String>,
    pub builtin: bool,
}

impl Language {
    pub fn english() -> Language {
        Language { code: "en".into(), name: "English".into(), strings: BTreeMap::new(), builtin: true }
    }

    /// Read a language file. Entries with an empty translation are ignored
    /// (a template is full of them), so a half-done file is fine.
    pub fn from_json(json: &str) -> Result<Language, String> {
        #[derive(serde::Deserialize)]
        struct File {
            code: String,
            name: String,
            #[serde(default)]
            strings: BTreeMap<String, String>,
        }
        let file: File = serde_json::from_str(json).map_err(|e| format!("not a language file: {e}"))?;
        let code = file.code.trim().to_string();
        if code.is_empty() || !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err("the \"code\" must be letters, digits, - or _ (for example fr or pt-BR)".into());
        }
        if file.name.trim().is_empty() {
            return Err("the language needs a \"name\"".into());
        }
        let strings = file.strings.into_iter().filter(|(_, v)| !v.trim().is_empty()).collect();
        Ok(Language { code, name: file.name.trim().to_string(), strings, builtin: false })
    }

    pub fn to_json(&self) -> String {
        #[derive(serde::Serialize)]
        struct File<'a> {
            code: &'a str,
            name: &'a str,
            strings: &'a BTreeMap<String, String>,
        }
        serde_json::to_string_pretty(&File { code: &self.code, name: &self.name, strings: &self.strings }).unwrap_or_default()
    }
}

/// English first, then the built-in translations.
pub fn builtin_languages() -> Vec<Language> {
    let mut all = vec![Language::english()];
    for json in BUILTIN {
        if let Ok(language) = Language::from_json(json) {
            all.push(Language { builtin: true, ..language });
        }
    }
    all
}

static ACTIVE: RwLock<Option<HashMap<String, &'static str>>> = RwLock::new(None);
static INTERNED: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);

/// Translations are kept for the life of the program so `tr` can hand out
/// `&'static str`. Interning keeps switching languages from growing memory.
fn intern(text: &str) -> &'static str {
    let mut guard = INTERNED.lock().unwrap_or_else(|e| e.into_inner());
    let set = guard.get_or_insert_with(HashSet::new);
    if let Some(existing) = set.get(text) {
        return existing;
    }
    let leaked: &'static str = Box::leak(text.to_string().into_boxed_str());
    set.insert(leaked);
    leaked
}

/// Make a language the one the whole app speaks. Re-run the frame afterwards.
pub fn activate(language: &Language) {
    let map: HashMap<String, &'static str> =
        language.strings.iter().map(|(k, v)| (k.clone(), intern(v))).collect();
    *ACTIVE.write().unwrap_or_else(|e| e.into_inner()) = if map.is_empty() { None } else { Some(map) };
}

/// The current language's text for this English text, or the English itself.
pub fn tr(english: &'static str) -> &'static str {
    match ACTIVE.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        Some(map) => map.get(english).copied().unwrap_or(english),
        None => english,
    }
}

/// Like [`tr`] for text that is not a literal (a name from a list, say).
/// Unknown text comes back unchanged.
pub fn tr_dyn(english: &str) -> String {
    match ACTIVE.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        Some(map) => map.get(english).map(|s| (*s).to_string()).unwrap_or_else(|| english.to_string()),
        None => english.to_string(),
    }
}

/// Translate a text with `{name}` placeholders and fill them in:
/// `trf("{count} words", &[("count", n.to_string())])`.
pub fn trf(english: &'static str, args: &[(&str, String)]) -> String {
    let mut text = tr(english).to_string();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

/// Every `tr("…")` / `trf("…")` text in a piece of Rust source, unescaped.
#[cfg(test)]
fn extract_keys(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut keys = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !source.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let rest = &source[i..];
        // `tr(` or `trf(`, optional whitespace (the call may wrap), then the quote.
        let call = ["tr(", "trf("]
            .iter()
            .find_map(|head| {
                let after = rest.strip_prefix(head)?;
                let spaces = after.len() - after.trim_start().len();
                after[spaces..].starts_with('"').then_some(head.len() + spaces + 1)
            })
            .unwrap_or(0);
        let boundary = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        if call > 0 && boundary {
            let mut text = String::new();
            let mut chars = source[i + call..].char_indices();
            while let Some((_, c)) = chars.next() {
                match c {
                    '"' => {
                        keys.push(text);
                        break;
                    }
                    '\\' => match chars.next().map(|(_, e)| e) {
                        Some('n') => text.push('\n'),
                        Some('t') => text.push('\t'),
                        Some('"') => text.push('"'),
                        Some('\\') => text.push('\\'),
                        Some('\'') => text.push('\''),
                        Some('u') => {
                            // \u{2026}
                            let mut hex = String::new();
                            for (_, h) in chars.by_ref() {
                                if h == '}' {
                                    break;
                                }
                                if h != '{' {
                                    hex.push(h);
                                }
                            }
                            if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                                text.push(ch);
                            }
                        }
                        Some(other) => text.push(other),
                        None => break,
                    },
                    _ => text.push(c),
                }
            }
            i += call;
        } else {
            i += 1;
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every English text in the program's source, sorted.
    fn source_keys() -> Vec<String> {
        fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("src/i18n.rs") {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        out.extend(extract_keys(&text));
                    }
                }
            }
        }
        let mut keys = Vec::new();
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut keys);
        keys.retain(|k| k.chars().filter(|c| c.is_alphabetic()).count() >= 2);
        keys.sort();
        keys.dedup();
        keys
    }

    /// `assets/languages/template.json` lists every English text in the
    /// source. When it is out of date, run the tests once with
    /// `UPDATE_LANGUAGE_TEMPLATE=1` to rewrite it.
    #[test]
    fn the_template_lists_every_text_in_the_source() {
        let keys = source_keys();
        let strings: BTreeMap<String, String> = keys.iter().map(|k| (k.clone(), String::new())).collect();
        let fresh = format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({
                "code": "template",
                "name": "Template: change code and name, then fill in the translations",
                "strings": strings,
            }))
            .unwrap()
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/languages/template.json");
        if std::env::var("UPDATE_LANGUAGE_TEMPLATE").is_ok() {
            std::fs::write(&path, &fresh).unwrap();
        }
        assert_eq!(
            TEMPLATE, fresh,
            "assets/languages/template.json is out of date: run `UPDATE_LANGUAGE_TEMPLATE=1 cargo test the_template` and rebuild"
        );
    }

    #[test]
    fn built_in_translations_match_the_source() {
        let keys: HashSet<String> = source_keys().into_iter().collect();
        for language in builtin_languages().iter().filter(|l| l.code != "en") {
            for (key, value) in &language.strings {
                assert!(keys.contains(key), "{}: \"{key}\" is no longer a text in the app", language.code);
                // Every {placeholder} of the English must survive translation.
                for part in key.split('{').skip(1) {
                    if let Some(name) = part.split('}').next() {
                        assert!(value.contains(&format!("{{{name}}}")), "{}: \"{key}\" lost {{{name}}}", language.code);
                    }
                }
            }
        }
    }

    #[test]
    fn english_is_the_fallback_and_translations_apply() {
        let mut fr = Language::english();
        fr.code = "xx".into();
        fr.strings.insert("Save".into(), "Enregistrer".into());
        // These tests share the process-wide language, so they only ever
        // check values for keys no other test uses.
        activate(&fr);
        assert_eq!(tr("Save"), "Enregistrer");
        assert_eq!(tr("A text with no translation"), "A text with no translation");
        assert_eq!(tr_dyn("Save"), "Enregistrer");
        activate(&Language::english());
        assert_eq!(tr("Save"), "Save");
    }

    #[test]
    fn placeholders_are_filled_after_translating() {
        assert_eq!(trf("{n} words in {where}", &[("n", "12".into()), ("where", "Draft".into())]), "12 words in Draft");
    }

    #[test]
    fn language_files_are_validated() {
        assert!(Language::from_json("nope").is_err());
        assert!(Language::from_json(r#"{"code":"../x","name":"X"}"#).is_err());
        assert!(Language::from_json(r#"{"code":"fr","name":" "}"#).is_err());
        let l = Language::from_json(r#"{"code":"pt-BR","name":"Português","strings":{"A":"B","C":""}}"#).unwrap();
        assert_eq!(l.strings.len(), 1, "empty translations are ignored");
        assert_eq!(Language::from_json(&l.to_json()).unwrap(), l);
    }

    #[test]
    fn keys_are_found_in_source_and_unescaped() {
        let src = r#"ui.button(tr("Save \"it\"")); let x = trf("{n} files…\n", &[]); let attr = "tr(\"no\")"; mytr("skip")"#;
        // `mytr(` is not a call to tr, and the quoted example inside a string
        // literal is still found only if it really starts a call.
        let keys = extract_keys(src);
        assert!(keys.contains(&"Save \"it\"".to_string()));
        assert!(keys.contains(&"{n} files…\n".to_string()));
        assert!(!keys.contains(&"skip".to_string()));
    }
}
