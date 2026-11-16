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
use std::ops::Range;

/// A Hunspell dictionary plus a word list for edit-distance suggestions, the
/// writer's personal dictionary and words ignored for this session.
pub struct SpellChecker {
    dict: zspell::Dictionary,
    words: HashSet<String>,
    custom: HashSet<String>,
    ignored: HashSet<String>,
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
        Self {
            dict,
            words,
            custom: HashSet::new(),
            ignored: HashSet::new(),
        }
    }

    /// Replace the personal dictionary with `words`.
    pub fn set_custom(&mut self, words: impl IntoIterator<Item = String>) {
        self.custom = words
            .into_iter()
            .map(|word| normalise(&word))
            .filter(|word| !word.is_empty())
            .collect();
    }

    /// Add one word to the personal dictionary.
    pub fn add_custom(&mut self, word: &str) {
        let word = normalise(word);
        if !word.is_empty() {
            self.custom.insert(word);
        }
    }

    /// Ignore a word for the rest of this session. Ignored words are not
    /// persisted; use [`SpellChecker::add_custom`] for that.
    pub fn ignore(&mut self, word: &str) {
        let word = normalise(word);
        if !word.is_empty() {
            self.ignored.insert(word);
        }
    }

    /// Is `word` correct? Leading/trailing punctuation is ignored, the personal
    /// dictionary and session-ignored words are accepted, and possessives,
    /// inflections and hyphenated compounds are handled by Hunspell.
    pub fn is_correct(&self, word: &str) -> bool {
        let word = normalise(word);
        if word.is_empty() {
            return true;
        }
        self.known(&word)
    }

    /// Is the normalised `word` known to one of our sources?
    fn known(&self, word: &str) -> bool {
        if self.custom.contains(word) || self.ignored.contains(word) {
            return true;
        }
        if self.dict.check_word(word) {
            return true;
        }
        // Hunspell's splitter used to verify hyphenated compounds part by part;
        // do the same here now that tokenisation keeps the hyphen.
        if word.contains('-') {
            let mut parts = 0;
            for part in word.split('-').filter(|part| !part.is_empty()) {
                parts += 1;
                if !self.custom.contains(part)
                    && !self.ignored.contains(part)
                    && !self.dict.check_word(part)
                {
                    return false;
                }
            }
            return parts >= 2;
        }
        false
    }

    /// Is `word` in the dictionary? Leading/trailing punctuation is ignored;
    /// possessives and inflections are handled by Hunspell.
    pub fn check_word(&self, word: &str) -> bool {
        self.is_correct(word)
    }

    /// Byte ranges of misspelled word tokens.
    ///
    /// Tokens keep internal apostrophes and hyphens (`cat's`, `well-known`);
    /// a token with fewer than two alphabetic characters is never reported.
    /// Offsets come from `char_indices`, so they are exact UTF-8 byte ranges.
    pub fn misspelled_spans(&self, text: &str) -> Vec<(usize, usize)> {
        word_tokens(text)
            .into_iter()
            .filter(|(start, end)| {
                let token = &text[*start..*end];
                alphabetic_count(token) >= 2 && !self.is_correct(token)
            })
            .collect()
    }

    /// Unique misspelled words in first-seen order.
    ///
    /// Uses the same tokeniser as [`SpellChecker::misspelled_spans`]. Each
    /// alphabetic part of an incorrect compound is reported separately, so
    /// `qik's` reports `qik` rather than the possessive form.
    pub fn misspellings(&self, text: &str) -> Vec<String> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut out = Vec::new();
        for (start, end) in word_tokens(text) {
            let token = &text[start..end];
            if alphabetic_count(token) < 2 || self.is_correct(token) {
                continue;
            }
            for piece in token.split(|c| c == '\'' || c == '-') {
                let word = normalise(piece);
                if alphabetic_count(&word) < 2 || self.is_correct(&word) {
                    continue;
                }
                if seen.insert(word.clone()) {
                    out.push(word);
                }
            }
        }
        out
    }

    /// Words within edit distance two, sorted by distance then alphabetically.
    pub fn suggest(&self, word: &str, limit: usize) -> Vec<String> {
        let target = normalise(word);
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
pub fn normalise(word: &str) -> String {
    let lower = word.to_lowercase();
    let trimmed = lower.trim_matches(|c: char| !c.is_alphabetic() && c != '\'' && c != '-');
    trimmed
        .trim_matches(|c: char| c == '\'' || c == '-')
        .to_string()
}

/// The word that a caret at UTF-8 byte offset `byte` sits in or touches.
///
/// Used by the editor's right-click menu: egui reports the click as a
/// character offset, which is converted with [`byte_offset`] first. A caret
/// exactly at a word's start or end still selects that word, so clicking the
/// left half of the first letter or the right half of the last letter behaves
/// like a word processor. Punctuation away from any word returns `None`.
/// `byte` is clamped to the text and snapped back to a char boundary so a
/// stale offset can never panic on a multi-byte character.
pub fn word_at(text: &str, byte: usize) -> Option<(Range<usize>, String)> {
    if text.is_empty() {
        return None;
    }
    let mut byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    word_tokens(text)
        .into_iter()
        .find(|(start, end)| *start <= byte && byte <= *end)
        .map(|(start, end)| (start..end, text[start..end].to_string()))
}

/// Byte offset of the `char_index`-th character in `text`.
///
/// egui's text cursor reports a char index, not a byte offset; slicing needs
/// bytes. An index at or past the end maps to `text.len()`.
pub fn byte_offset(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(offset, _)| offset)
        .unwrap_or(text.len())
}

/// Number of alphabetic characters in a token. Single letters are not worth
/// reporting, and the `s` left over by a possessive is never a word.
fn alphabetic_count(word: &str) -> usize {
    word.chars().filter(|c| c.is_alphabetic()).count()
}

/// Word token byte ranges. A token starts on an alphabetic character and keeps
/// internal apostrophes and hyphens only when another letter follows, so
/// trailing punctuation stays outside the span (`foxx.`, `foxx—`).
fn word_tokens(text: &str) -> Vec<(usize, usize)> {
    let mut tokens = Vec::new();
    let mut start: Option<usize> = None;
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        match start {
            None => {
                if ch.is_alphabetic() {
                    start = Some(index);
                }
            }
            Some(begin) => {
                let internal = (ch == '\'' || ch == '-')
                    && chars
                        .peek()
                        .map(|(_, next)| next.is_alphabetic())
                        .unwrap_or(false);
                if !ch.is_alphabetic() && !internal {
                    tokens.push((begin, index));
                    start = None;
                }
            }
        }
    }
    if let Some(begin) = start {
        tokens.push((begin, text.len()));
    }
    tokens
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
        // Contractions are left alone and a hyphenated compound is verified
        // part by part.
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

    #[test]
    fn custom_and_ignored_words_are_accepted() {
        let mut spell = bundled();
        spell.add_custom("Zorbatrix");
        assert!(spell.is_correct("zorbatrix"));
        assert!(spell.check_word("Zorbatrix,"));
        spell.ignore("zzzzqqq");
        assert!(spell.is_correct("zzzzqqq"));
        // A rebuilt personal dictionary replaces the old entries.
        spell.set_custom(["saltroadish".to_string()]);
        assert!(spell.is_correct("saltroadish"));
        assert!(!spell.is_correct("zorbatrix"));
    }

    #[test]
    fn misspelled_spans_report_exact_byte_ranges() {
        let spell = bundled();
        assert_eq!(
            spell.misspelled_spans("the qik brown foxx"),
            vec![(4, 7), (14, 18)]
        );
    }

    #[test]
    fn misspelled_spans_are_empty_for_correct_text() {
        let spell = bundled();
        assert!(spell.misspelled_spans("the brown fox jumps").is_empty());
    }

    #[test]
    fn word_at_finds_the_word_under_a_caret() {
        let text = "the qik brown foxx";
        assert_eq!(word_at(text, 4), Some((4..7, "qik".to_string())));
        assert_eq!(word_at(text, 5), Some((4..7, "qik".to_string())));
        assert_eq!(word_at(text, 6), Some((4..7, "qik".to_string())));
        // A caret just after the word (its end byte) still targets it.
        assert_eq!(word_at(text, 7), Some((4..7, "qik".to_string())));
        // The first and last characters are boundaries too.
        assert_eq!(word_at(text, 0), Some((0..3, "the".to_string())));
        assert_eq!(word_at(text, text.len()), Some((14..18, "foxx".to_string())));
        // Past the end clamps to the last word rather than panicking.
        assert_eq!(word_at(text, 999), Some((14..18, "foxx".to_string())));
    }

    #[test]
    fn word_at_returns_none_on_standalone_punctuation() {
        // The comma sits between two spaces, touching neither word.
        assert_eq!(word_at("a , b", 2), None);
        assert_eq!(word_at("( )", 1), None);
        assert_eq!(word_at("", 0), None);
        // Punctuation inside a token stays part of it.
        assert_eq!(
            word_at("cat's", 4),
            Some((0..5, "cat's".to_string()))
        );
        assert_eq!(
            word_at("well-known", 6),
            Some((0..10, "well-known".to_string()))
        );
    }

    #[test]
    fn word_at_snaps_to_char_boundaries_for_multibyte_text() {
        let text = "héllo wörld";
        // "é" is two bytes, so the caret at char 2 is byte 3.
        assert_eq!(byte_offset(text, 2), 3);
        assert_eq!(word_at(text, 3), Some((0..6, "héllo".to_string())));
        // A mid-character byte (2, inside "é") snaps back to the word start.
        assert_eq!(word_at(text, 2), Some((0..6, "héllo".to_string())));
    }

    #[test]
    fn byte_offset_is_multibyte_safe() {
        let text = "héllo";
        assert_eq!(byte_offset(text, 0), 0);
        assert_eq!(byte_offset(text, 1), 1); // after 'h'
        assert_eq!(byte_offset(text, 2), 3); // after 'é'
        assert_eq!(byte_offset(text, 5), text.len());
        assert_eq!(byte_offset(text, 99), text.len());
    }
}
