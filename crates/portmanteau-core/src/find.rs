//! Find and replace across the whole project.
//!
//! One engine serves the in-editor find bar, the Search page, the command
//! `search.find`/`search.replace` and the MCP tools. A query is plain text by
//! default; options make it case-sensitive, whole-word or a regular
//! expression (the Rust `regex` syntax, which cannot run away on bad input).
//!
//! Positions are Unicode character offsets in a rich text's editor flow
//! (`RichDocument::editor_text`), so a hit can be turned into a caret with
//! `position_at`. Matches never span a paragraph.

use std::ops::Range;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use crate::{DocKind, Document, ItemKind, Project, RichDocument, StyleSheet};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct FindOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
}

/// Which part of the project a document belongs to, for filtering results.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SearchGroup {
    /// The Draft tree: chapters, scenes and their folders.
    Manuscript,
    /// Front and back matter pages.
    Matter,
    Research,
    /// Database items (characters, places…).
    Database,
}

impl SearchGroup {
    pub fn label(self) -> &'static str {
        match self {
            SearchGroup::Manuscript => "Manuscript",
            SearchGroup::Matter => "Front & back matter",
            SearchGroup::Research => "Research",
            SearchGroup::Database => "Database",
        }
    }
    pub fn all() -> [SearchGroup; 4] {
        [SearchGroup::Manuscript, SearchGroup::Matter, SearchGroup::Research, SearchGroup::Database]
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct FindQuery {
    pub text: String,
    pub options: FindOptions,
    /// Groups to search; empty means every group.
    pub groups: Vec<SearchGroup>,
    /// Parts to search (text, synopsis, notes); empty means all of them.
    pub fields: Vec<HitField>,
}

impl FindQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), options: FindOptions::default(), groups: Vec::new(), fields: Vec::new() }
    }
    fn wants(&self, group: SearchGroup) -> bool {
        self.groups.is_empty() || self.groups.contains(&group)
    }
    fn wants_field(&self, field: HitField) -> bool {
        self.fields.is_empty() || self.fields.contains(&field)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum HitField {
    /// A document's text (or a database item's notes).
    Body,
    Synopsis,
    Notes,
}

impl HitField {
    pub fn label(self) -> &'static str {
        match self {
            HitField::Body => "Text",
            HitField::Synopsis => "Synopsis",
            HitField::Notes => "Notes",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum FindSource {
    Document(String),
    Item(String),
}

impl FindSource {
    pub fn id(&self) -> &str {
        match self {
            FindSource::Document(id) | FindSource::Item(id) => id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct FindHit {
    pub source: FindSource,
    pub title: String,
    pub group: SearchGroup,
    pub field: HitField,
    /// Document-wide character range of the match.
    pub start: usize,
    pub end: usize,
    /// Where to put a caret: the paragraph and the offset inside it.
    pub paragraph: usize,
    pub offset: usize,
    /// The paragraph's text (shortened around the match) and the match
    /// inside it, in characters, for drawing a highlighted line.
    pub excerpt: String,
    pub mark: Range<usize>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReplaceScope {
    /// Everything the query finds.
    All,
    /// Everything in one document or item.
    Source(FindSource),
    /// Everything in one field of one document or item (its text, say).
    Field(FindSource, HitField),
    /// A single hit, as reported by `find_all` (source, field and start).
    Hit(FindSource, HitField, usize),
}

/// A compiled query.
pub struct Finder {
    re: Regex,
    literal_replacement: bool,
}

struct Raw {
    start: usize,
    end: usize,
    paragraph: usize,
    offset: usize,
    line: String,
    mark: Range<usize>,
    replacement: String,
}

impl Finder {
    pub fn new(text: &str, options: &FindOptions) -> Result<Self, String> {
        if text.is_empty() {
            return Err("Nothing to look for".into());
        }
        let mut pattern = if options.regex { text.to_string() } else { regex::escape(text) };
        if options.whole_word {
            pattern = format!(r"\b(?:{pattern})\b");
        }
        let re = RegexBuilder::new(&pattern)
            .case_insensitive(!options.case_sensitive)
            .size_limit(1 << 22)
            .build()
            .map_err(|e| match e {
                regex::Error::Syntax(msg) => format!("Not a valid pattern: {}", msg.lines().last().unwrap_or("").trim()),
                _ => "That pattern is too large".to_string(),
            })?;
        Ok(Self { re, literal_replacement: !options.regex })
    }

    /// Matches in one piece of text, as byte ranges (empty matches skipped).
    pub fn ranges(&self, text: &str) -> Vec<Range<usize>> {
        self.re.find_iter(text).filter(|m| !m.is_empty()).map(|m| m.range()).collect()
    }

    fn expand(&self, text: &str, at: usize, template: &str) -> String {
        if self.literal_replacement {
            return template.to_string();
        }
        match self.re.captures_at(text, at) {
            Some(caps) => {
                let mut out = String::new();
                caps.expand(template, &mut out);
                out
            }
            None => template.to_string(),
        }
    }

    fn scan(&self, doc: &RichDocument, template: &str) -> Vec<Raw> {
        let mut raws = Vec::new();
        let mut base = 0usize;
        for (index, paragraph) in doc.paragraphs.iter().enumerate() {
            let text = paragraph.editor_text();
            let length = text.chars().count();
            for range in self.ranges(&text) {
                let offset = text[..range.start].chars().count();
                let width = text[range.clone()].chars().count();
                raws.push(Raw {
                    start: base + offset,
                    end: base + offset + width,
                    paragraph: index,
                    offset,
                    line: text.clone(),
                    mark: offset..offset + width,
                    replacement: self.expand(&text, range.start, template),
                });
            }
            base += length + 1;
        }
        raws
    }

    /// Matches in a bare rich text, as character ranges in its editor flow
    /// (the in-editor find bar uses this).
    pub fn find_in(&self, doc: &RichDocument) -> Vec<Range<usize>> {
        self.scan(doc, "").into_iter().map(|raw| raw.start..raw.end).collect()
    }

    /// What one match would be replaced with (captures expanded in regex mode).
    pub fn replacement_for(&self, doc: &RichDocument, range: &Range<usize>, template: &str) -> Option<String> {
        self.scan(doc, template).into_iter().find(|raw| raw.start == range.start).map(|raw| raw.replacement)
    }
}

/// A window of the line around the match, with an ellipsis where it was cut.
fn excerpt(line: &str, mark: &Range<usize>) -> (String, Range<usize>) {
    const BEFORE: usize = 48;
    const AFTER: usize = 72;
    let chars: Vec<char> = line.chars().collect();
    let from = mark.start.saturating_sub(BEFORE);
    let to = (mark.end + AFTER).min(chars.len());
    let mut text = String::new();
    let mut shift = 0;
    if from > 0 {
        text.push('…');
        shift = 1;
    }
    text.extend(&chars[from..to]);
    if to < chars.len() {
        text.push('…');
    }
    let text = text.replace('\n', " ");
    (text, mark.start - from + shift..mark.end - from + shift)
}

type Field<'a> = (FindSource, String, SearchGroup, HitField, &'a RichDocument);

impl Project {
    /// The part of the project a document belongs to.
    pub fn search_group(&self, doc: &Document) -> SearchGroup {
        if self.in_research(doc) || doc.research.is_some() {
            return SearchGroup::Research;
        }
        let mut top = doc;
        while let Some(parent) = top.parent.as_deref().and_then(|id| self.document(id)) {
            top = parent;
        }
        if matches!(top.title.as_str(), "Front Matter" | "Back Matter")
            || matches!(doc.kind, DocKind::FrontMatter | DocKind::BackMatter)
        {
            SearchGroup::Matter
        } else {
            SearchGroup::Manuscript
        }
    }

    fn rich_fields<'a>(&'a self, query: &FindQuery) -> Vec<Field<'a>> {
        let mut fields: Vec<Field<'a>> = Vec::new();
        for doc in self.ordered_documents() {
            if doc.kind.is_folder() {
                continue;
            }
            let group = self.search_group(doc);
            if !query.wants(group) {
                continue;
            }
            let source = FindSource::Document(doc.id.clone());
            for (field, rich) in [(HitField::Body, &doc.content), (HitField::Synopsis, &doc.synopsis), (HitField::Notes, &doc.notes)] {
                if query.wants_field(field) {
                    fields.push((source.clone(), doc.title.clone(), group, field, rich));
                }
            }
        }
        if query.wants(SearchGroup::Database) && query.wants_field(HitField::Body) {
            for item in &self.items {
                fields.push((
                    FindSource::Item(item.id.clone()),
                    item.title.clone(),
                    SearchGroup::Database,
                    HitField::Body,
                    &item.notes,
                ));
            }
        }
        fields
    }

    /// Every match of the query, in binder order then database order.
    pub fn find_all(&self, query: &FindQuery) -> Result<Vec<FindHit>, String> {
        let finder = Finder::new(&query.text, &query.options)?;
        let mut hits = Vec::new();
        for (source, title, group, field, rich) in self.rich_fields(query) {
            for raw in finder.scan(rich, "") {
                let (text, mark) = excerpt(&raw.line, &raw.mark);
                hits.push(FindHit {
                    source: source.clone(),
                    title: title.clone(),
                    group,
                    field,
                    start: raw.start,
                    end: raw.end,
                    paragraph: raw.paragraph,
                    offset: raw.offset,
                    excerpt: text,
                    mark,
                });
            }
        }
        Ok(hits)
    }

    /// Replace matches; returns how many were replaced. In regex mode the
    /// replacement may use `$1`, `${name}`; otherwise it is taken literally.
    /// Each document whose text changes gets a snapshot first, so a
    /// replacement can be undone from Versions.
    pub fn replace_matches(&mut self, query: &FindQuery, replacement: &str, scope: &ReplaceScope) -> Result<usize, String> {
        let finder = Finder::new(&query.text, &query.options)?;
        struct Job {
            source: FindSource,
            field: HitField,
            edits: Vec<(Range<usize>, String)>,
        }
        // Collect first (the fields borrow the project), then apply.
        let mut jobs = Vec::new();
        for (source, _, _, field, rich) in self.rich_fields(query) {
            let in_scope = match scope {
                ReplaceScope::All => true,
                ReplaceScope::Source(wanted) => *wanted == source,
                ReplaceScope::Field(wanted, wanted_field) | ReplaceScope::Hit(wanted, wanted_field, _) => {
                    *wanted == source && *wanted_field == field
                }
            };
            if !in_scope {
                continue;
            }
            let edits: Vec<(Range<usize>, String)> = finder
                .scan(rich, replacement)
                .into_iter()
                .filter(|raw| match scope {
                    ReplaceScope::Hit(_, _, start) => raw.start == *start,
                    _ => true,
                })
                .map(|raw| (raw.start..raw.end, raw.replacement))
                .collect();
            if !edits.is_empty() {
                jobs.push(Job { source, field, edits });
            }
        }
        let mut count = 0;
        for job in jobs {
            match &job.source {
                FindSource::Document(id) => {
                    let Some(doc) = self.document(id) else { continue };
                    let sheet = self.style_sheet_for(doc);
                    let Some(doc) = self.document_mut(id) else { continue };
                    // A single replacement is too small to snapshot; bigger ones are.
                    if job.field == HitField::Body && !matches!(scope, ReplaceScope::Hit(..)) {
                        doc.snapshot("Before replace", format!("Replacing “{}”", query.text));
                    }
                    let target = match job.field {
                        HitField::Body => &mut doc.content,
                        HitField::Synopsis => &mut doc.synopsis,
                        HitField::Notes => &mut doc.notes,
                    };
                    count += apply_edits(target, job.edits, &sheet);
                }
                FindSource::Item(id) => {
                    let sheet = StyleSheet::for_medium(crate::Medium::Prose, &self.styles);
                    if let Some(item) = self.items.iter_mut().find(|i| i.id == *id) {
                        count += apply_edits(&mut item.notes, job.edits, &sheet);
                    }
                }
            }
        }
        if count > 0 {
            self.touch();
        }
        Ok(count)
    }

    /// The kind label shown beside a database hit ("Character").
    pub fn item_kind_label(&self, id: &str) -> &'static str {
        self.items.iter().find(|i| i.id == id).map(|i| i.kind.label()).unwrap_or(ItemKind::Character.label())
    }
}

fn apply_edits(rich: &mut RichDocument, mut edits: Vec<(Range<usize>, String)>, sheet: &StyleSheet) -> usize {
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let count = edits.len();
    for (range, text) in edits {
        rich.replace_range(range, &text, sheet);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocKind, Medium, ProjectKind};

    fn project() -> (Project, String, String) {
        let mut p = Project::new("T", ProjectKind::Novel);
        let draft = p.folder_id("Draft").unwrap();
        let research = p.folder_id("Research").unwrap();
        let ch = p.add_document(Some(draft), "Chapter", DocKind::Chapter, Medium::Prose);
        let note = p.add_document(Some(research), "Tides", DocKind::Research, Medium::Note);
        p.document_mut(&ch).unwrap().content = RichDocument::from_plain_text("The harbour wall.\nA Harbour light, harbourside.", "body");
        p.document_mut(&note).unwrap().content = RichDocument::from_plain_text("Harbour tides.", "body");
        (p, ch, note)
    }

    fn q(text: &str) -> FindQuery {
        FindQuery::new(text)
    }

    #[test]
    fn plain_search_ignores_case_and_reports_places() {
        let (p, ch, _) = project();
        let hits = p.find_all(&q("harbour")).unwrap();
        let in_chapter: Vec<_> = hits.iter().filter(|h| h.source.id() == ch).collect();
        assert_eq!(in_chapter.len(), 3);
        assert_eq!((in_chapter[0].paragraph, in_chapter[0].offset), (0, 4));
        assert_eq!((in_chapter[1].paragraph, in_chapter[1].offset), (1, 2));
        let first = in_chapter[0];
        assert_eq!(first.excerpt.chars().skip(first.mark.start).take(first.mark.len()).collect::<String>(), "harbour");
        assert_eq!(hits.len(), 4, "the research note counts too");
    }

    #[test]
    fn options_change_what_matches() {
        let (p, _, _) = project();
        let mut query = q("harbour");
        query.options.case_sensitive = true;
        assert_eq!(p.find_all(&query).unwrap().len(), 2);
        query.options = FindOptions { whole_word: true, ..Default::default() };
        assert_eq!(p.find_all(&query).unwrap().len(), 3, "harbourside is another word");
        query.options = FindOptions { regex: true, ..Default::default() };
        query.text = r"harb\w+side".into();
        assert_eq!(p.find_all(&query).unwrap().len(), 1);
        query.text = "harbour(".into();
        assert!(p.find_all(&query).unwrap_err().contains("valid"));
        query.options.regex = false;
        assert!(p.find_all(&query).unwrap().is_empty(), "a bracket is literal in plain mode");
    }

    #[test]
    fn groups_filter_the_results() {
        let (p, ch, note) = project();
        let mut query = q("harbour");
        query.groups = vec![SearchGroup::Research];
        let hits = p.find_all(&query).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].source.id(), note);
        query.groups = vec![SearchGroup::Manuscript];
        assert!(p.find_all(&query).unwrap().iter().all(|h| h.source.id() == ch));
    }

    #[test]
    fn replacing_keeps_each_paragraph_and_snapshots_first() {
        let (mut p, ch, _) = project();
        let mut query = q("harbour");
        query.groups = vec![SearchGroup::Manuscript];
        let n = p.replace_matches(&query, "port", &ReplaceScope::All).unwrap();
        assert_eq!(n, 3);
        let doc = p.document(&ch).unwrap();
        assert_eq!(doc.content.plain_text(), "The port wall.\nA port light, portside.");
        assert_eq!(doc.snapshots.len(), 1);
        assert!(p.find_all(&query).unwrap().is_empty());
    }

    #[test]
    fn regex_replacement_uses_captures_and_one_hit_can_be_replaced() {
        let (mut p, ch, _) = project();
        let mut query = q(r"(harbour)(side)?");
        query.options = FindOptions { regex: true, ..Default::default() };
        query.groups = vec![SearchGroup::Manuscript];
        let hits = p.find_all(&query).unwrap();
        let second = hits[1].clone();
        let scope = ReplaceScope::Hit(second.source.clone(), second.field, second.start);
        assert_eq!(p.replace_matches(&query, "<$1>", &scope).unwrap(), 1);
        assert_eq!(p.document(&ch).unwrap().content.plain_text(), "The harbour wall.\nA <Harbour> light, harbourside.");
    }

    #[test]
    fn a_field_scope_leaves_the_asides_alone() {
        let (mut p, ch, _) = project();
        p.document_mut(&ch).unwrap().synopsis = RichDocument::from_plain_text("harbour synopsis", "body");
        let scope = ReplaceScope::Field(FindSource::Document(ch.clone()), HitField::Body);
        assert_eq!(p.replace_matches(&q("harbour"), "port", &scope).unwrap(), 3);
        assert_eq!(p.document(&ch).unwrap().synopsis.plain_text(), "harbour synopsis");
    }

    #[test]
    fn database_items_and_asides_are_searched() {
        let (mut p, ch, _) = project();
        let mut item = crate::DatabaseItem::new(ItemKind::Place, "Quay");
        item.notes = RichDocument::from_plain_text("By the harbour.", "body");
        p.items.push(item);
        p.document_mut(&ch).unwrap().synopsis = RichDocument::from_plain_text("harbour synopsis", "body");
        let hits = p.find_all(&q("harbour")).unwrap();
        assert!(hits.iter().any(|h| matches!(h.source, FindSource::Item(_))));
        assert!(hits.iter().any(|h| h.field == HitField::Synopsis));
        let mut text_only = q("harbour");
        text_only.fields = vec![HitField::Body];
        assert!(p.find_all(&text_only).unwrap().iter().all(|h| h.field == HitField::Body));
        let mut synopsis_only = q("harbour");
        synopsis_only.fields = vec![HitField::Synopsis];
        let hits = p.find_all(&synopsis_only).unwrap();
        assert_eq!(hits.len(), 1, "only the synopsis, not the text or the database");
        assert_eq!(hits[0].field, HitField::Synopsis);
    }

    #[test]
    fn empty_and_zero_width_patterns_find_nothing() {
        let (p, _, _) = project();
        assert!(p.find_all(&q("")).is_err());
        let mut query = q("x*");
        query.options.regex = true;
        assert!(p.find_all(&query).unwrap().iter().all(|h| h.end > h.start));
    }

    #[test]
    fn long_lines_are_shortened_around_the_match() {
        let long = format!("{} needle {}", "a".repeat(200), "b".repeat(200));
        let (text, mark) = excerpt(&long, &(201..207));
        assert!(text.starts_with('…') && text.ends_with('…'));
        assert_eq!(text.chars().skip(mark.start).take(mark.len()).collect::<String>(), "needle");
        assert!(text.chars().count() < 140);
    }
}
