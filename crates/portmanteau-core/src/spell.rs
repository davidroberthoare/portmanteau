//! A small, offline spell-checker.
//!
//! The bundled word list is a plain newline-separated file. Everything here is
//! deliberately pure: no crates, no network and no Hunspell dependencies. The
//! desktop shell loads one [`SpellChecker`] at start-up and asks it for
//! misspellings and suggestions while drawing an editor.

use std::collections::HashSet;

/// A set of lowercase words plus edit-distance suggestions.
pub struct SpellChecker {
    words: HashSet<String>,
}

impl SpellChecker {
    /// Build a checker from newline-separated words. Blank lines are ignored
    /// and every entry is trimmed and lowercased.
    pub fn new(wordlist: &str) -> Self {
        let words = wordlist
            .lines()
            .map(|line| line.trim().to_lowercase())
            .filter(|word| !word.is_empty())
            .collect();
        Self { words }
    }

    /// Is `word` in the dictionary? Leading/trailing punctuation is ignored,
    /// and a possessive (`cat's`) is accepted when its stem (`cat`) is known.
    pub fn check_word(&self, word: &str) -> bool {
        let word = normalize_word(word);
        if word.is_empty() {
            return true;
        }
        if self.words.contains(&word) {
            return true;
        }
        // Possessive: cat's -> cat.
        if let Some(stem) = word.strip_suffix("'s") {
            if !stem.is_empty() && self.words.contains(stem) {
                return true;
            }
        }
        // A plain plural that the list does not carry separately.
        if let Some(stem) = word.strip_suffix('s') {
            if stem.chars().count() > 1 && self.words.contains(stem) {
                return true;
            }
        }
        false
    }

    /// Unique misspelled words in first-seen order. Tokens are split on
    /// non-alphabetic characters, but internal apostrophes and hyphens stay
    /// attached so `don't` and `well-known` are checked as one word.
    pub fn misspellings(&self, text: &str) -> Vec<String> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut out = Vec::new();
        for token in tokens(text) {
            let word = normalize_word(&token);
            if word.chars().filter(|c| c.is_alphabetic()).count() <= 1 {
                continue;
            }
            if !seen.insert(word.clone()) {
                continue;
            }
            if !self.check_word(&word) {
                out.push(word);
            }
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

/// Split text into word-like runs. A run may contain internal apostrophes and
/// hyphens, but other punctuation and whitespace are separators.
fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphabetic() || ((ch == '\'' || ch == '-') && !current.is_empty()) {
            current.push(ch);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
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
        SpellChecker::new(include_str!("../../../assets/words.txt"))
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
        // Fall back to the stem even when the possessive is not listed.
        let small = SpellChecker::new("cat\nruns\n");
        assert!(small.check_word("cat's"));
        assert!(small.check_word("cats"));
        assert!(!small.check_word("catz's"));
    }

    #[test]
    fn keeps_internal_apostrophes_and_hyphens() {
        let spell = bundled();
        let miss = spell.misspellings("don't well-known qik's");
        assert!(!miss.contains(&"don't".to_string()));
        // The bundled list has no hyphenated entries, but the token must stay
        // whole instead of becoming "well" and "known".
        assert!(miss.contains(&"well-known".to_string()));
        assert!(!miss.contains(&"well".to_string()));
        assert!(!miss.contains(&"known".to_string()));
        assert!(miss.contains(&"qik's".to_string()));
    }
}
