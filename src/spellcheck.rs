//! Loading the bundled dictionary for the desktop editor.
//!
//! The Hunspell dictionary and suggestion word list are compiled into the
//! binary so the app stays offline and self-contained.

use portmanteau_core::spell::SpellChecker;

/// Build the one checker the app keeps for its lifetime.
pub fn load() -> SpellChecker {
    SpellChecker::new(
        include_str!("../assets/dict/en_US.aff"),
        include_str!("../assets/dict/en_US.dic"),
        include_str!("../assets/words.txt"),
    )
}
