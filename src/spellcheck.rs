//! Loading the bundled dictionary for the desktop editor.
//!
//! The Hunspell dictionaries are compiled into the
//! binary so the app stays offline and self-contained.

use portmanteau_core::spell::SpellChecker;

/// Spelling languages offered in Settings, as `(code, label)`.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("en_US", "English (US)"),
    ("en_GB", "English (UK)"),
    ("fr_FR", "Français"),
    ("es_ES", "Español"),
];

/// The dictionary that goes with an interface language, when there is one.
/// English keeps whichever English the writer already uses (`current`).
pub fn for_interface(code: &str, current: &str) -> Option<&'static str> {
    let base = code.split(['-', '_']).next().unwrap_or("");
    match base {
        "fr" => Some("fr_FR"),
        "es" => Some("es_ES"),
        "en" if !current.starts_with("en_") => Some("en_US"),
        _ => None,
    }
}

/// Build the checker for `language`, falling back to US English for an
/// unknown code.
pub fn load(language: &str) -> SpellChecker {
    let (aff, dic) = match language {
        "en_GB" => (
            include_str!("../assets/dict/en_GB.aff"),
            include_str!("../assets/dict/en_GB.dic"),
        ),
        "fr_FR" => (
            include_str!("../assets/dict/fr_FR.aff"),
            include_str!("../assets/dict/fr_FR.dic"),
        ),
        "es_ES" => (
            include_str!("../assets/dict/es_ES.aff"),
            include_str!("../assets/dict/es_ES.dic"),
        ),
        _ => (
            include_str!("../assets/dict/en_US.aff"),
            include_str!("../assets/dict/en_US.dic"),
        ),
    };
    SpellChecker::new(aff, dic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dictionary_loads_and_checks_its_own_language() {
        for (code, word, wrong) in [("en_US", "garden", "gardn"), ("fr_FR", "jardin", "jardn"), ("es_ES", "jardín", "jardn")] {
            let checker = load(code);
            assert!(checker.is_correct(word), "{code} should accept {word}");
            assert!(!checker.is_correct(wrong), "{code} should reject {wrong}");
        }
    }

    #[test]
    fn interface_languages_pick_a_dictionary() {
        assert_eq!(for_interface("fr", "en_US"), Some("fr_FR"));
        assert_eq!(for_interface("es-MX", "en_US"), Some("es_ES"));
        assert_eq!(for_interface("en", "en_GB"), None);
        assert_eq!(for_interface("en", "fr_FR"), Some("en_US"));
        assert_eq!(for_interface("de", "en_US"), None);
    }
}
