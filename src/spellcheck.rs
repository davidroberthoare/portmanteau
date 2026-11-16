//! Loading the bundled dictionary for the desktop editor.
//!
//! The Hunspell dictionaries and suggestion word list are compiled into the
//! binary so the app stays offline and self-contained.

use portmanteau_core::spell::SpellChecker;

/// Spelling languages offered in Settings, as `(code, label)`.
pub const LANGUAGES: &[(&str, &str)] = &[("en_US", "English (US)"), ("en_GB", "English (UK)")];

/// Build the checker for `language`, falling back to US English for an
/// unknown code. All languages share the suggestion word list.
pub fn load(language: &str) -> SpellChecker {
    let (aff, dic) = match language {
        "en_GB" => (
            include_str!("../assets/dict/en_GB.aff"),
            include_str!("../assets/dict/en_GB.dic"),
        ),
        _ => (
            include_str!("../assets/dict/en_US.aff"),
            include_str!("../assets/dict/en_US.dic"),
        ),
    };
    SpellChecker::new(aff, dic, include_str!("../assets/words.txt"))
}
