//! An offline spell-checker backed by a bundled Hunspell dictionary.
//!
//! Detection uses [`zspell`], a pure-Rust Hunspell implementation, so inflected
//! words (`running`, `bananas`, `cat's`) are accepted the way a real word
//! processor accepts them. Suggestions stay on the simpler bundled word list:
//! zspell's suggestion API lives behind an unstable feature flag, so the
//! existing Levenshtein suggester is kept here instead.
//!
//! The desktop shell loads one [`SpellChecker`] at start-up and asks it for
//! misspellings and suggestions while drawing an editor.

use std::collections::HashSet;

/// A Hunspell dictionary plus a word list for edit-distance suggestions.
pub struct SpellChecker {
    dict: zspell::Dictionary,
    words: HashSet<String>,
}

impl SpellChecker {
    /// Build a checker from the Hunspell `.aff`/`.dic` contents and a
    /// newline-separated word list used for suggestions. Blank word-list lines
    /// are ignored and every entry is trimmed and lowercased.
    pub fn new(aff: &str, dic: &str, wordlist: &str) -> Self {
        let dict = zspell::builder()
            .config_str(aff)
            .dict_str(dic)
            .build()
            .expect("bundled Hunspell dictionary should be valid");
        let words = wordlist
            .lines()
            .map(|line| line.trim().to_lowercase())
            .filter(|word| !word.is_empty())
            .collect();
        Self { dict, words }
    }

    /// Is `word` in the dictionary? Leading/trailing punctuation is ignored;
    /// possessives and inflections are handled by Hunspell.
    pub fn check_word(&self, word: &str) -> bool {
        let word = normalize_word(word);
        if word.is_empty() {
            return true;
        }
        self.dict.check_word(&word)
    }

    /// Unique misspelled words in first-seen order.
    ///
    /// Hunspell's own word splitter drives tokenisation, so contractions are
    /// left alone and hyphenated compounds are split into parts the dictionary
    /// can verify.
    pub fn misspellings(&self, text: &str) -> Vec<String> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut out = Vec::new();
        for (_, token) in self.dict.check_indices(text) {
            let word = normalize_word(token);
            // Skip empty fragments and single letters. Hunspell's splitter
            // leaves the `s` of a possessive on its own, and one-letter words
            // are never worth underlining.
            if word.chars().filter(|c| c.is_alphabetic()).count() <= 1 {
                continue;
            }
            if !seen.insert(word.clone()) {
                continue;
            }
            out.push(word);
        }
        out
    }

    /// Words within edit distance two, sorted by distance then alphabetically.
    pub fn suggest(&self, word: &str, limit: usize) -> Vec<String> {
        let target = normalize_word(word);
        if target.is_empty() {
            return Vec::new();
        }
        let target_len = target.chars().count();
        let mut scored: Vec<(usize, String)> = Vec::new();
        for candidate in &self.words {
            let candidate_len = candidate.chars().count();
            if candidate_len.abs_diff(target_len) > 2 {
                continue;
            }
            let distance = levenshtein(&target, candidate);
            if distance <= 2 {
                scored.push((distance, candidate.clone()));
            }
        }
        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        scored
            .into_iter()
            .take(limit)
            .map(|(_, word)| word)
            .collect()
    }
}

/// Lowercase a token and strip punctuation from its edges while keeping
/// internal apostrophes and hyphens (`cat's`, `well-known`).
fn normalize_word(word: &str) -> String {
    let lower = word.to_lowercase();
    let trimmed = lower.trim_matches(|c: char| !c.is_alphabetic() && c != '\'' && c != '-');
    trimmed
        .trim_matches(|c: char| c == '\'' || c == '-')
        .to_string()
}

/// Levenshtein edit distance, two rolling rows so memory stays linear.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        current[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            current[j] = (previous[j] + 1)
                .min(current[j - 1] + 1)
                .min(previous[j - 1] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled() -> SpellChecker {
        SpellChecker::new(
            include_str!("../../../assets/dict/en_US.aff"),
            include_str!("../../../assets/dict/en_US.dic"),
            include_str!("../../../assets/words.txt"),
        )
    }

    #[test]
    fn known_and_unknown_words() {
        let spell = bundled();
        assert!(spell.check_word("hello"));
        assert!(!spell.check_word("zzzzqqq"));
        assert!(spell.check_word("hello,"));
    }

    #[test]
    fn reports_unique_misspellings_in_order() {
        let spell = bundled();
        let miss = spell.misspellings("the qik brown foxx qik");
        assert!(miss.contains(&"qik".to_string()));
        assert!(miss.contains(&"foxx".to_string()));
        assert!(!miss.contains(&"the".to_string()));
        assert!(!miss.contains(&"brown".to_string()));
        assert_eq!(miss.iter().filter(|w| *w == "qik").count(), 1);
        assert!(miss.iter().position(|w| w == "qik") < miss.iter().position(|w| w == "foxx"));
    }

    #[test]
    fn suggests_closest_words() {
        let spell = bundled();
        let suggestions = spell.suggest("helo", 5);
        assert!(suggestions.contains(&"hello".to_string()), "{suggestions:?}");
    }

    #[test]
    fn possessive_and_plural_are_accepted() {
        let spell = bundled();
        assert!(spell.check_word("cat's"));
        assert!(spell.check_word("runs"));
        assert!(spell.check_word("cats"));
        assert!(!spell.check_word("catz's"));
    }

    #[test]
    fn inflected_forms_are_understood() {
        let spell = bundled();
        assert!(spell.check_word("running"));
        assert!(spell.check_word("bananas"));
        assert!(!spell.check_word("runnning"));
        assert!(!spell.check_word("bananaz"));
    }

    #[test]
    fn contractions_and_hyphenated_words_do_not_false_positive() {
        let spell = bundled();
        let miss = spell.misspellings("don't well-known qik's qik");
        // Hunspell's splitter leaves contractions alone and splits a
        // hyphenated compound into words it can verify.
        assert!(!miss.contains(&"don't".to_string()));
        assert!(!miss.contains(&"well-known".to_string()));
        assert!(!miss.contains(&"well".to_string()));
        assert!(!miss.contains(&"known".to_string()));
        assert!(!miss.contains(&"qik's".to_string()));
        // The possessive's trailing `s` fragment must not be reported.
        assert!(!miss.contains(&"s".to_string()));
        assert!(miss.contains(&"qik".to_string()));
        assert!(!miss.contains(&"-".to_string()));
    }
}
