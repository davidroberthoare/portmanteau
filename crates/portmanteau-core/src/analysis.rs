//! Writing analysis: plain measures of how a text reads. It reports; it does
//! not judge. Numbers like sentence length and dialogue share describe a
//! writer's style, and the right values depend on the book.

use std::collections::HashMap;

use serde::Serialize;

/// Measures of one stretch of prose (a document or a whole manuscript).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct TextAnalysis {
    pub words: u32,
    pub sentences: u32,
    pub paragraphs: u32,
    pub unique_words: u32,
    /// Distinct words as a share of all words, 0 to 1 (higher = more varied).
    pub variety: f32,
    pub avg_sentence_words: f32,
    pub longest_sentence_words: u32,
    /// Sentences of 30 words or more.
    pub long_sentences: u32,
    /// Sentences of 1 to 10, 11 to 20, 21 to 30 and 31 or more words.
    pub sentence_lengths: [u32; 4],
    pub avg_paragraph_words: f32,
    /// Share of the words that are spoken, between quotation marks, 0 to 1.
    pub dialogue_share: f32,
    /// Words ending in "-ly", often adverbs.
    pub ly_words: u32,
    /// Flesch reading ease: about 90 is very easy, 60 plain English, 30 hard.
    pub reading_ease: f32,
    /// US school grade needed to follow it (Flesch-Kincaid).
    pub grade_level: f32,
    pub reading_minutes: u32,
    /// The most-used words that are not common function words.
    pub top_words: Vec<(String, u32)>,
    /// Words used often close together: a word that appears three or more
    /// times within a 60-word stretch, with how many such stretches there are.
    pub echoes: Vec<(String, u32)>,
}

const STOP_WORDS: &[&str] = &[
    "a", "about", "after", "again", "all", "also", "am", "an", "and", "any", "are", "as", "at", "back", "be",
    "because", "been", "before", "being", "but", "by", "came", "can", "could", "did", "do", "does", "down",
    "even", "for", "from", "get", "got", "had", "has", "have", "he", "her", "here", "him", "his", "how", "i",
    "if", "in", "into", "is", "it", "its", "just", "like", "me", "more", "my", "no", "not", "now", "of",
    "off", "on", "one", "only", "or", "other", "our", "out", "over", "said", "she", "so", "some", "than",
    "that", "the", "their", "them", "then", "there", "these", "they", "this", "to", "too", "up", "us",
    "was", "we", "were", "what", "when", "where", "which", "who", "will", "with", "would", "you", "your",
];

fn is_stop(word: &str) -> bool {
    STOP_WORDS.binary_search(&word).is_ok()
}

/// A rough syllable count for English, good enough for readability scores.
fn syllables(word: &str) -> u32 {
    let letters: Vec<char> = word.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return 0;
    }
    let vowel = |c: char| "aeiouy".contains(c);
    let mut count = 0u32;
    let mut previous = false;
    for &c in &letters {
        let v = vowel(c);
        if v && !previous {
            count += 1;
        }
        previous = v;
    }
    let text: String = letters.iter().collect();
    if text.ends_with('e') && !text.ends_with("le") && count > 1 {
        count -= 1;
    }
    count.max(1)
}

fn clean(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '’')
        .trim_matches(|c| c == '\'' || c == '’')
        .to_lowercase()
}

/// Analyse paragraphs of prose, one string per paragraph.
pub fn analyze(paragraphs: &[String]) -> TextAnalysis {
    let mut out = TextAnalysis::default();
    let mut counts: HashMap<String, u32> = HashMap::new();
    let mut syllable_total = 0u32;
    let mut spoken = 0u32;
    let mut in_quote = false;
    let mut all_words: Vec<String> = Vec::new();

    for paragraph in paragraphs {
        if paragraph.trim().is_empty() {
            continue;
        }
        out.paragraphs += 1;
        // Sentences: runs of words ending at . ! ? (a closing quote may follow).
        let mut sentence_words = 0u32;
        for token in paragraph.split_whitespace() {
            // Quotes toggle "speech"; curly and straight both count.
            let opens = token.chars().filter(|c| matches!(c, '“' | '"')).count()
                + token.chars().filter(|c| *c == '”').count();
            let word = clean(token);
            let was_in_quote = in_quote;
            if opens % 2 == 1 {
                in_quote = !in_quote;
            }
            if word.is_empty() {
                continue;
            }
            out.words += 1;
            sentence_words += 1;
            syllable_total += syllables(&word);
            if was_in_quote || in_quote {
                spoken += 1;
            }
            if word.len() > 3 && word.ends_with("ly") && !matches!(word.as_str(), "only" | "family" | "early" | "really" | "july" | "lonely" | "lovely" | "belly" | "silly" | "ugly" | "holy" | "reply" | "supply" | "apply" | "fly" | "italy") {
                out.ly_words += 1;
            }
            *counts.entry(word.clone()).or_default() += 1;
            all_words.push(word);
            let ends = token.trim_end_matches(['"', '”', '’', '\'', ')', ']']);
            if ends.ends_with(['.', '!', '?', '…']) {
                finish_sentence(&mut out, sentence_words);
                sentence_words = 0;
            }
        }
        if sentence_words > 0 {
            finish_sentence(&mut out, sentence_words);
        }
        // A quote left open at the end of a paragraph does not run on.
        in_quote = false;
    }

    out.unique_words = counts.len() as u32;
    if out.words > 0 {
        out.variety = out.unique_words as f32 / out.words as f32;
        out.dialogue_share = spoken as f32 / out.words as f32;
        out.avg_paragraph_words = out.words as f32 / out.paragraphs.max(1) as f32;
    }
    if out.sentences > 0 {
        out.avg_sentence_words = out.words as f32 / out.sentences as f32;
    }
    if out.words > 0 && out.sentences > 0 {
        let words = out.words as f32;
        let per_sentence = words / out.sentences as f32;
        let per_word = syllable_total as f32 / words;
        out.reading_ease = (206.835 - 1.015 * per_sentence - 84.6 * per_word).clamp(-20.0, 120.0);
        out.grade_level = (0.39 * per_sentence + 11.8 * per_word - 15.59).max(0.0);
    }
    out.reading_minutes = crate::text::reading_minutes(out.words);

    let mut top: Vec<(String, u32)> = counts
        .iter()
        .filter(|(w, n)| !is_stop(w) && w.chars().count() > 2 && **n > 2)
        .map(|(w, n)| (w.clone(), *n))
        .collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    top.truncate(15);
    out.top_words = top;
    out.echoes = echoes(&all_words);
    out
}

fn finish_sentence(out: &mut TextAnalysis, words: u32) {
    out.sentences += 1;
    out.longest_sentence_words = out.longest_sentence_words.max(words);
    if words >= 30 {
        out.long_sentences += 1;
    }
    let bucket = match words {
        0..=10 => 0,
        11..=20 => 1,
        21..=30 => 2,
        _ => 3,
    };
    out.sentence_lengths[bucket] += 1;
}

/// Content words that show up three times or more inside any 60-word window.
fn echoes(words: &[String]) -> Vec<(String, u32)> {
    const WINDOW: usize = 60;
    let mut positions: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, w) in words.iter().enumerate() {
        if w.chars().count() > 3 && !is_stop(w) {
            positions.entry(w).or_default().push(i);
        }
    }
    let mut found: Vec<(String, u32)> = positions
        .into_iter()
        .filter_map(|(w, at)| {
            let mut clusters = 0u32;
            let mut i = 0;
            while i + 2 < at.len() {
                if at[i + 2] - at[i] < WINDOW {
                    clusters += 1;
                    i += 3;
                } else {
                    i += 1;
                }
            }
            (clusters > 0).then(|| (w.to_owned(), clusters))
        })
        .collect();
    found.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    found.truncate(10);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> TextAnalysis {
        analyze(&text.split('\n').map(String::from).collect::<Vec<_>>())
    }

    #[test]
    fn counts_words_sentences_and_paragraphs() {
        let a = run("The cat sat. The dog ran away!\nIs it over?");
        assert_eq!((a.words, a.sentences, a.paragraphs), (10, 3, 2));
        assert_eq!(a.longest_sentence_words, 4);
        assert!((a.avg_sentence_words - 10.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn dialogue_share_counts_quoted_words() {
        let a = run("“We should go,” she said quietly. He nodded.");
        // quoted: we should go = 3 of 8 words
        assert!((a.dialogue_share - 3.0 / 8.0).abs() < 0.01, "{}", a.dialogue_share);
        assert_eq!(a.ly_words, 1);
    }

    #[test]
    fn short_plain_sentences_read_easily() {
        let easy = run("The cat sat. The dog ran. We all ate.");
        let hard = run("Notwithstanding considerable organisational difficulties, the administration unquestionably demonstrated extraordinary perseverance.");
        assert!(easy.reading_ease > hard.reading_ease);
        assert!(easy.grade_level < hard.grade_level);
    }

    #[test]
    fn top_words_skip_function_words_and_echoes_find_clusters() {
        let a = run("The harbour was cold. The harbour was grey. Out past the harbour the lamp swung. Lamp, harbour, lamp.");
        assert_eq!(a.top_words.first().map(|w| w.0.as_str()), Some("harbour"));
        assert!(a.top_words.iter().all(|(w, _)| w != "the"));
        assert!(a.echoes.iter().any(|(w, _)| w == "harbour"));
    }

    #[test]
    fn empty_text_is_all_zeros() {
        assert_eq!(run(""), TextAnalysis::default());
    }

    #[test]
    fn stop_words_stay_sorted_for_the_binary_search() {
        let mut sorted = STOP_WORDS.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted, STOP_WORDS);
    }
}
