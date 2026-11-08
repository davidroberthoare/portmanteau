//! Text measurement helpers shared by the UI and exporters.

/// Count words the way a writer expects: whitespace-separated tokens.
/// Hyphenated words count once; standalone punctuation does not count.
pub fn count_words(text: &str) -> u32 {
    text.split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_alphanumeric()))
        .count() as u32
}

pub fn reading_minutes(words: u32) -> u32 {
    ((words as f32) / 238.0).ceil() as u32
}

pub fn truncate_words(text: &str, limit: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= limit {
        text.to_string()
    } else {
        format!("{}…", words[..limit].join(" "))
    }
}

/// Rough character count excluding whitespace, used by the status bar.
pub fn visible_chars(text: &str) -> usize {
    text.chars().filter(|c| !c.is_whitespace()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_ignore_punctuation() {
        assert_eq!(count_words("The sea — and the salt."), 5);
        assert_eq!(count_words(""), 0);
    }

    #[test]
    fn truncation_adds_ellipsis() {
        assert_eq!(truncate_words("one two three four", 2), "one two…");
    }
}
