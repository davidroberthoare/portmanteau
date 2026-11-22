//! How the paragraph styles of a source file become Portmanteau's.
//!
//! A Word or Final Draft file names a style for every paragraph ("Heading 1",
//! "Scene Heading"). Each distinct name is a `SourceStyle` with a default
//! `Target`; the writer can override a target, and the importer applies the
//! overrides it is given. Fountain and plain text have no named styles, so
//! they report none.

use std::collections::{BTreeMap, HashMap};

/// What a source style becomes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A heading that starts a new document (level 1 is the shallowest).
    Heading(u8),
    /// The work's title: used as the project's title, not imported as text.
    Title,
    /// One of the project's paragraph styles, by id ("body", "dialogue", …).
    Style(String),
    /// Let the importer decide from the text (screenplay action vs dialogue…).
    Auto,
    /// Leave the paragraph out.
    Skip,
}

impl Target {
    /// The form used in settings, commands and files: `heading:2`, `title`,
    /// `style:dialogue`, `auto`, `skip`.
    pub fn token(&self) -> String {
        match self {
            Target::Heading(level) => format!("heading:{level}"),
            Target::Title => "title".into(),
            Target::Style(id) => format!("style:{id}"),
            Target::Auto => "auto".into(),
            Target::Skip => "skip".into(),
        }
    }

    pub fn parse(token: &str) -> Option<Target> {
        let token = token.trim();
        Some(match token {
            "title" => Target::Title,
            "auto" => Target::Auto,
            "skip" => Target::Skip,
            _ => {
                if let Some(level) = token.strip_prefix("heading:") {
                    Target::Heading(level.parse::<u8>().ok()?.clamp(1, 9))
                } else {
                    let id = token.strip_prefix("style:")?;
                    (!id.is_empty()).then(|| Target::Style(id.to_string()))?
                }
            }
        })
    }
}

/// One distinct paragraph style found in a source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceStyle {
    pub name: String,
    /// How many non-empty paragraphs use it.
    pub count: usize,
    /// What the importer would do without the writer's say.
    pub default: Target,
    /// What it does now (the default unless overridden).
    pub target: Target,
}

/// Most distinct style names tracked. A file with more (a hostile one, or a
/// document pasted together from hundreds of sources) keeps importing: the
/// extra styles simply follow their defaults and are not listed for mapping.
const MAX_TRACKED_STYLES: usize = 500;

/// Collects the styles seen while importing and applies overrides.
pub(crate) struct StyleLog {
    /// Overrides, keyed by lower-cased source name, parsed once.
    overrides: HashMap<String, Target>,
    /// Lower-cased name to its place in `seen`.
    index: HashMap<String, usize>,
    seen: Vec<SourceStyle>,
}

impl StyleLog {
    pub(crate) fn new(overrides: &BTreeMap<String, String>) -> Self {
        let overrides = overrides
            .iter()
            .filter_map(|(source, token)| Some((source.trim().to_lowercase(), Target::parse(token)?)))
            .collect();
        Self { overrides, index: HashMap::new(), seen: Vec::new() }
    }

    /// Record a paragraph in style `name` and say what it becomes. Names are
    /// matched without regard to case. Constant time per paragraph.
    pub(crate) fn resolve(&mut self, name: &str, default: Target) -> Target {
        let key = name.trim().to_lowercase();
        let target = self.overrides.get(&key).cloned().unwrap_or_else(|| default.clone());
        match self.index.get(&key) {
            Some(&at) => self.seen[at].count += 1,
            None if self.seen.len() < MAX_TRACKED_STYLES => {
                self.index.insert(key, self.seen.len());
                self.seen.push(SourceStyle {
                    name: name.trim().to_string(),
                    count: 1,
                    default,
                    target: target.clone(),
                });
            }
            None => {}
        }
        target
    }

    pub(crate) fn finish(self) -> Vec<SourceStyle> {
        self.seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flood_of_distinct_style_names_is_capped_and_still_resolves() {
        let mut log = StyleLog::new(&BTreeMap::new());
        for n in 0..(MAX_TRACKED_STYLES + 200) {
            let target = log.resolve(&format!("Style {n}"), Target::Style("body".into()));
            assert_eq!(target, Target::Style("body".into()));
        }
        assert_eq!(log.finish().len(), MAX_TRACKED_STYLES);
    }

    #[test]
    fn overrides_match_without_regard_to_case_and_repeat_counts_add_up() {
        let overrides: BTreeMap<String, String> = [("  QUOTE ".to_string(), "skip".to_string())].into();
        let mut log = StyleLog::new(&overrides);
        assert_eq!(log.resolve("Quote", Target::Auto), Target::Skip);
        assert_eq!(log.resolve("quote", Target::Auto), Target::Skip);
        let seen = log.finish();
        assert_eq!((seen.len(), seen[0].count), (1, 2));
    }
}
