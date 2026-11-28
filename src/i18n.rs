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
mod labels;
pub use labels::{compile_reason, folder_name, scale_name};

/// The built-in language files, compiled into the app.
const BUILTIN: &[&str] = &[include_str!("../assets/languages/fr.json"), include_str!("../assets/languages/es.json")];

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
    /// English text of the welcome project -> translation, so the sample
    /// project a new writer sees is in their language too.
    pub welcome: BTreeMap<String, String>,
    pub builtin: bool,
}

impl Language {
    pub fn english() -> Language {
        Language { code: "en".into(), name: "English".into(), strings: BTreeMap::new(), welcome: BTreeMap::new(), builtin: true }
    }

    /// Read a language file. Entries with an empty translation are ignored
    /// (a template is full of them), so a half-done file is fine.
    pub fn from_json(json: &str) -> Result<Language, String> {
        #[derive(serde::Deserialize)]
        struct File {
            code: String,
            name: String,
            #[serde(default)]
            strings: BTreeMap<String, Entry>,
            #[serde(default)]
            welcome: BTreeMap<String, Entry>,
        }
        /// A translation is either the text itself or, as in the template,
        /// an object that also says what the text is for.
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Entry {
            Text(String),
            Full { #[serde(default)] translation: String },
        }
        let file: File = serde_json::from_str(json).map_err(|e| format!("not a language file: {e}"))?;
        let code = file.code.trim().to_string();
        if code.is_empty() || !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err("the \"code\" must be letters, digits, - or _ (for example fr or pt-BR)".into());
        }
        if file.name.trim().is_empty() {
            return Err("the language needs a \"name\"".into());
        }
        let keep = |map: BTreeMap<String, Entry>| -> BTreeMap<String, String> {
            map.into_iter()
                .map(|(k, v)| (k, match v { Entry::Text(t) | Entry::Full { translation: t } => t }))
                .filter(|(_, v)| !v.trim().is_empty())
                .collect()
        };
        Ok(Language { code, name: file.name.trim().to_string(), strings: keep(file.strings), welcome: keep(file.welcome), builtin: false })
    }

    pub fn to_json(&self) -> String {
        #[derive(serde::Serialize)]
        struct File<'a> {
            code: &'a str,
            name: &'a str,
            strings: &'a BTreeMap<String, String>,
            welcome: &'a BTreeMap<String, String>,
        }
        serde_json::to_string_pretty(&File { code: &self.code, name: &self.name, strings: &self.strings, welcome: &self.welcome }).unwrap_or_default()
    }

    /// The template with this language's translations filled in (and its code
    /// and name), so someone correcting it sees the English, the current
    /// translation and the notes together. It loads back as this language.
    pub fn to_filled_template(&self) -> String {
        let Ok(mut template) = serde_json::from_str::<serde_json::Value>(TEMPLATE) else { return self.to_json() };
        template["code"] = self.code.clone().into();
        template["name"] = self.name.clone().into();
        for (section, translations) in [("strings", &self.strings), ("welcome", &self.welcome)] {
            if let Some(entries) = template.get_mut(section).and_then(|v| v.as_object_mut()) {
                for (english, entry) in entries.iter_mut() {
                    if let Some(text) = translations.get(english) {
                        entry["translation"] = text.clone().into();
                    }
                }
            }
        }
        format!("{}\n", serde_json::to_string_pretty(&template).unwrap_or_default())
    }

    /// The text of the welcome project in this language: the translation, or
    /// the English when there is none.
    pub fn welcome_text(&self, english: &str) -> String {
        self.welcome.get(english).cloned().unwrap_or_else(|| english.to_string())
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

/// The language the operating system (or browser) is set to, as a tag such as
/// `fr-CA` or `en_US`, if it can say.
pub fn system_locale() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        sys_locale::get_locale()
    }
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().and_then(|w| w.navigator().language())
    }
}

/// Which of `languages` best fits a locale tag: the same code (`pt-BR`), else
/// the same base language (`fr-CA` -> `fr`). `None` when nothing fits.
pub fn match_locale<'a>(languages: &'a [Language], locale: &str) -> Option<&'a Language> {
    let norm = |s: &str| s.replace('_', "-").split('.').next().unwrap_or("").to_lowercase();
    let wanted = norm(locale);
    if wanted.is_empty() {
        return None;
    }
    let base = |s: &str| s.split('-').next().unwrap_or("").to_string();
    languages
        .iter()
        .find(|l| norm(&l.code) == wanted)
        .or_else(|| languages.iter().find(|l| base(&norm(&l.code)) == base(&wanted) && norm(&l.code) == base(&wanted)))
        .or_else(|| languages.iter().find(|l| base(&norm(&l.code)) == base(&wanted)))
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

/// The texts a `fn label(…)` returns, which the app shows through `tr_dyn`
/// (the model crate has no translations of its own).
#[cfg(test)]
fn extract_labels(source: &str) -> Vec<String> {
    let mut labels = Vec::new();
    let mut from = 0;
    // `fn label(` and `fn description(` both return texts shown through `tr_dyn`.
    let markers = ["fn label(", "fn description("];
    loop {
        let Some((at, _)) = markers.iter().filter_map(|m| source[from..].find(m).map(|i| (i, m))).min_by_key(|(i, _)| *i) else { break };
        let start = from + at;
        // Export formats, paper sizes and number styles keep their own names.
        let owner = source[..start].rfind("impl ").map(|i| source[i..].lines().next().unwrap_or("")).unwrap_or("");
        let skip = source[start..].starts_with("fn label(") && ["ExportKind", "PaperSize", "NumberFormat"].iter().any(|n| owner.contains(n));
        let Some(open) = source[start..].find('{').map(|i| start + i) else { break };
        let mut depth = 0;
        let mut end = open;
        for (i, c) in source[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        // String literals on the right of `=>` (or the whole body for a plain return).
        for line in source[open..end].lines() {
            if skip {
                break;
            }
            let Some(arm) = line.split("=>").nth(1) else { continue };
            let mut parts = arm.split('"');
            parts.next();
            if let Some(text) = parts.next() {
                labels.push(text.to_string());
            }
        }
        from = end.max(start + 1);
    }
    labels
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

    /// Every English text in the program's source, sorted, with the source
    /// files that use it.
    fn source_keys() -> Vec<(String, Vec<String>)> {
        fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("src/i18n.rs") {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        out.extend(extract_keys(&text).into_iter().map(|k| (k, name.clone())));
                        out.extend(extract_labels(&text).into_iter().map(|k| (k, name.clone())));
                    }
                }
            }
        }
        let mut keys = Vec::new();
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut keys);
        // The model crate's `label()` texts, shown through `tr_dyn`.
        // Sentences in progress.rs (badges and encouragement) are shown through `tr_dyn`.
        let progress = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/portmanteau-core/src/progress.rs");
        if let Ok(text) = std::fs::read_to_string(&progress) {
            let code = text.split("#[cfg(test)]").next().unwrap_or("");
            for line in code.lines().filter(|l| !l.trim_start().starts_with("//")) {
                for (i, part) in line.split('"').enumerate() {
                    if i % 2 == 1 && part.contains(' ') && part.chars().any(|c| c.is_alphabetic()) && !part.contains("{}") && !part.starts_with("{") {
                        keys.push((part.to_string(), "progress.rs".to_string()));
                    }
                }
            }
        }
        // Section names the exporters give the Compile preview (`label: "Cover".into()`).
        if let Ok(entries) = std::fs::read_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/portmanteau-export/src")) {
            for entry in entries.flatten() {
                if let Ok(text) = std::fs::read_to_string(entry.path()) {
                    let code = text.split("#[cfg(test)]").next().unwrap_or("");
                    for part in code.split("label: \"").skip(1) {
                        if let Some(label) = part.split('"').next() {
                            keys.push((label.to_string(), "export".to_string()));
                        }
                    }
                }
            }
        }
        for file in ["model.rs", "script.rs", "find.rs", "compile.rs"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/portmanteau-core/src").join(file);
            if let Ok(text) = std::fs::read_to_string(&path) {
                keys.extend(extract_labels(&text).into_iter().map(|k| (k, file.to_string())));
            }
        }
        keys.retain(|(k, _)| k.chars().filter(|c| c.is_alphabetic()).count() >= 2);
        keys.sort();
        keys.dedup();
        let mut grouped: Vec<(String, Vec<String>)> = Vec::new();
        for (key, file) in keys {
            match grouped.last_mut() {
                Some((last, files)) if *last == key => files.push(file),
                _ => grouped.push((key, vec![file])),
            }
        }
        grouped
    }

    /// Hand-written notes on what a text is for (`assets/languages/context.json`).
    fn contexts() -> BTreeMap<String, String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/languages/context.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// `assets/languages/template.json` lists every English text in the
    /// source. When it is out of date, run the tests once with
    /// `UPDATE_LANGUAGE_TEMPLATE=1` to rewrite it.
    #[test]
    fn the_template_lists_every_text_in_the_source() {
        let keys = source_keys();
        let contexts = contexts();
        let strings: BTreeMap<String, serde_json::Value> = keys
            .iter()
            .map(|(k, files)| {
                let mut entry = serde_json::Map::new();
                entry.insert("translation".into(), "".into());
                if let Some(context) = contexts.get(k) {
                    entry.insert("context".into(), context.clone().into());
                }
                entry.insert("used_in".into(), files.join(", ").into());
                (k.clone(), serde_json::Value::Object(entry))
            })
            .collect();
        let welcome: BTreeMap<String, serde_json::Value> = portmanteau_core::sample::welcome_texts()
            .into_iter()
            .map(|k| (k, serde_json::json!({ "translation": "" })))
            .collect();
        let fresh = format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({
                "code": "template",
                "name": "Template: change code and name, then fill in the translations",
                "about": "GENERATED FILE: do not edit the notes here (they come from assets/languages/context.json). To translate: change \"code\" and \"name\" above, then type each translation into the \"translation\" field and leave everything else as it is. \"context\" says what a text is for and \"used_in\" which source file shows it; both are ignored when the file loads, so there is nothing to remove. Texts left empty stay in English. Keep {placeholders} like {count} unchanged. See docs/localization.md.",
                "strings": strings,
                "welcome": welcome,
            }))
            .unwrap()
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/languages/template.json");
        if std::env::var("UPDATE_LANGUAGE_TEMPLATE").is_ok() {
            std::fs::write(&path, &fresh).unwrap();
        }
        // Windows checks text files out with CRLF line endings; the content is the same.
        assert_eq!(
            TEMPLATE.replace("\r\n", "\n"), fresh,
            "assets/languages/template.json is out of date: run `UPDATE_LANGUAGE_TEMPLATE=1 cargo test the_template` and rebuild"
        );
    }

    #[test]
    fn built_in_translations_match_the_source() {
        let keys: HashSet<String> = source_keys().into_iter().map(|(k, _)| k).collect();
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

    /// Single words are ambiguous ("Draft", "Label"), so each needs a note on what
    /// it is for; and a note for a text that no longer exists is stale.
    #[test]
    fn short_texts_have_a_context_note() {
        let contexts = contexts();
        let keys = source_keys();
        for (key, _) in &keys {
            let words = key.split_whitespace().count();
            assert!(
                words > 1 || contexts.contains_key(key),
                "\"{key}\" has no note in assets/languages/context.json (say where it appears and what it means)"
            );
        }
        let all: HashSet<&String> = keys.iter().map(|(k, _)| k).collect();
        for key in contexts.keys() {
            assert!(all.contains(key), "context.json has a note for \"{key}\", which is no longer in the app");
        }
    }

    #[test]
    fn built_in_welcome_translations_match_the_welcome_project() {
        let texts: HashSet<String> = portmanteau_core::sample::welcome_texts().into_iter().collect();
        for language in builtin_languages().iter().filter(|l| l.code != "en") {
            for key in language.welcome.keys() {
                assert!(texts.contains(key), "{}: welcome text \"{key}\" is no longer in the welcome project", language.code);
            }
        }
    }

    #[test]
    fn an_exported_language_loads_back_the_same() {
        for language in builtin_languages().into_iter().filter(|l| l.code != "en") {
            let again = Language::from_json(&language.to_filled_template()).unwrap();
            assert_eq!(again.strings, language.strings, "{}", language.code);
            assert_eq!(again.welcome, language.welcome, "{}", language.code);
            assert_eq!(again.code, language.code);
        }
    }

    #[test]
    fn a_system_locale_finds_the_closest_language() {
        let lang = |code: &str| Language { code: code.into(), name: code.into(), ..Language::english() };
        let all = vec![lang("en"), lang("fr"), lang("pt-BR")];
        assert_eq!(match_locale(&all, "fr-CA").map(|l| l.code.as_str()), Some("fr"));
        assert_eq!(match_locale(&all, "fr_FR.UTF-8").map(|l| l.code.as_str()), Some("fr"));
        assert_eq!(match_locale(&all, "pt-BR").map(|l| l.code.as_str()), Some("pt-BR"));
        assert_eq!(match_locale(&all, "pt-PT").map(|l| l.code.as_str()), Some("pt-BR"));
        assert_eq!(match_locale(&all, "en-GB").map(|l| l.code.as_str()), Some("en"));
        assert!(match_locale(&all, "ja-JP").is_none());
        assert!(match_locale(&all, "").is_none());
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
        let full = Language::from_json(r#"{"code":"pt","name":"P","strings":{"A":{"translation":"B","context":"x"},"C":{"translation":""}}}"#).unwrap();
        assert_eq!(full.strings.get("A").map(String::as_str), Some("B"), "the template's object form loads");
        assert_eq!(full.strings.len(), 1);
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

#[cfg(test)]
mod welcome_tests {
    use super::*;

    #[test]
    fn the_welcome_project_follows_the_language() {
        let all = builtin_languages();
        let fr = all.iter().find(|l| l.code == "fr").unwrap();
        let p = portmanteau_core::sample::welcome_with(&|e| fr.welcome_text(e));
        assert_eq!(p.title, "Bienvenue dans Portmanteau");
        // Everything in the welcome project has a text in every built-in language.
        for language in all.iter().filter(|l| l.code != "en") {
            for text in portmanteau_core::sample::welcome_texts() {
                assert!(language.welcome.contains_key(&text), "{} is missing a welcome text: {text}", language.code);
            }
        }
    }
}
