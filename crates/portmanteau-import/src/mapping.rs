//! How the paragraph styles of a source file become Portmanteau's.
//!
//! A Word or Final Draft file names a style for every paragraph ("Heading 1",
//! "Scene Heading"). Each distinct name is a `SourceStyle` with a default
//! `Target`; the writer can override a target, and the importer applies the
//! overrides it is given. Fountain and plain text have no named styles, so
//! they report none.

use std::collections::BTreeMap;

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

/// Collects the styles seen while importing and applies overrides.
pub(crate) struct StyleLog<'a> {
    overrides: &'a BTreeMap<String, String>,
    seen: Vec<SourceStyle>,
}

impl<'a> StyleLog<'a> {
    pub(crate) fn new(overrides: &'a BTreeMap<String, String>) -> Self {
        Self { overrides, seen: Vec::new() }
    }

    /// Record a paragraph in style `name` and say what it becomes. Names are
    /// matched without regard to case.
    pub(crate) fn resolve(&mut self, name: &str, default: Target) -> Target {
        let key = name.trim().to_lowercase();
        let overridden = self
            .overrides
            .iter()
            .find(|(source, _)| source.trim().to_lowercase() == key)
            .and_then(|(_, token)| Target::parse(token));
        let target = overridden.unwrap_or_else(|| default.clone());
        match self.seen.iter_mut().find(|s| s.name.to_lowercase() == key) {
            Some(style) => style.count += 1,
            None => self.seen.push(SourceStyle {
                name: name.trim().to_string(),
                count: 1,
                default,
                target: target.clone(),
            }),
        }
        target
    }

    pub(crate) fn finish(self) -> Vec<SourceStyle> {
        self.seen
    }
}
