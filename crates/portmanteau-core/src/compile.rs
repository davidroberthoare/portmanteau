//! What a compile contains. Every export is one kind of manuscript: a book,
//! a film screenplay, a TV script or a stage play. A project may hold several
//! kinds of document; the compile plan picks the documents of the chosen kind
//! inside the chosen scope and says why it left the others out.

use serde::{Deserialize, Serialize};

use crate::{DocKind, Document, Medium, Project, ProjectKind};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompileType {
    Prose,
    Screenplay,
    TvScript,
    StagePlay,
}

impl CompileType {
    pub fn all() -> [CompileType; 4] {
        [
            CompileType::Prose,
            CompileType::Screenplay,
            CompileType::TvScript,
            CompileType::StagePlay,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            CompileType::Prose => "Prose manuscript",
            CompileType::Screenplay => "Film screenplay",
            CompileType::TvScript => "TV script",
            CompileType::StagePlay => "Stage play",
        }
    }

    /// Stable name used by the command API and CLI.
    pub fn key(self) -> &'static str {
        match self {
            CompileType::Prose => "prose",
            CompileType::Screenplay => "screenplay",
            CompileType::TvScript => "tv",
            CompileType::StagePlay => "stage",
        }
    }

    pub fn from_key(key: &str) -> Option<CompileType> {
        match key.trim().to_lowercase().as_str() {
            "prose" | "book" | "manuscript" => Some(CompileType::Prose),
            "screenplay" | "film" => Some(CompileType::Screenplay),
            "tv" | "tvscript" | "tv script" | "television" => Some(CompileType::TvScript),
            "stage" | "stageplay" | "stage play" | "theatre" => Some(CompileType::StagePlay),
            _ => None,
        }
    }

    pub fn is_script(self) -> bool {
        self != CompileType::Prose
    }

    /// The document medium this type compiles. Prose also takes plain notes.
    pub fn accepts(self, medium: Medium) -> bool {
        match self {
            CompileType::Prose => matches!(medium, Medium::Prose | Medium::Note),
            CompileType::Screenplay => medium == Medium::Screenplay,
            CompileType::TvScript => medium == Medium::TvScript,
            CompileType::StagePlay => medium == Medium::StagePlay,
        }
    }

    pub fn medium(self) -> Medium {
        match self {
            CompileType::Prose => Medium::Prose,
            CompileType::Screenplay => Medium::Screenplay,
            CompileType::TvScript => Medium::TvScript,
            CompileType::StagePlay => Medium::StagePlay,
        }
    }

    pub fn stylesheet_id(self) -> &'static str {
        self.medium().default_stylesheet_id()
    }

    pub fn from_medium(medium: Medium) -> CompileType {
        match medium {
            Medium::Screenplay => CompileType::Screenplay,
            Medium::TvScript => CompileType::TvScript,
            Medium::StagePlay => CompileType::StagePlay,
            Medium::Prose | Medium::Note => CompileType::Prose,
        }
    }

    pub fn for_project_kind(kind: ProjectKind) -> CompileType {
        Self::from_medium(kind.default_medium())
    }
}

/// Which part of the binder a compile draws from.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
pub enum CompileScope {
    /// Everything in the binder.
    #[default]
    Draft,
    /// One binder item and everything beneath it.
    Subtree(String),
    /// An explicit list; used internally for "each script separately".
    Documents(Vec<String>),
}

impl CompileScope {
    /// `"draft"` (or blank) or a document id, as used by the command API.
    pub fn from_key(key: &str) -> CompileScope {
        match key.trim() {
            "" | "draft" | "all" => CompileScope::Draft,
            id => CompileScope::Subtree(id.to_owned()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkippedDocument<'a> {
    pub doc: &'a Document,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct CompilePlan<'a> {
    pub kind: CompileType,
    pub included: Vec<&'a Document>,
    pub skipped: Vec<SkippedDocument<'a>>,
}

/// Documents that belong to one output file when compiling each script
/// separately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileGroup {
    pub title: String,
    pub ids: Vec<String>,
}

impl Project {
    /// Whether `doc` sits anywhere inside the Research folder.
    pub fn in_research(&self, doc: &Document) -> bool {
        let mut parent = doc.parent.clone();
        while let Some(id) = parent {
            match self.document(&id) {
                Some(p) => {
                    if p.title == "Research" {
                        return true;
                    }
                    parent = p.parent.clone();
                }
                None => break,
            }
        }
        false
    }

    fn in_scope(&self, doc: &Document, scope: &CompileScope) -> bool {
        match scope {
            CompileScope::Draft => true,
            CompileScope::Documents(ids) => ids.iter().any(|id| *id == doc.id),
            CompileScope::Subtree(root) => {
                if doc.id == *root {
                    return true;
                }
                let mut parent = doc.parent.clone();
                while let Some(id) = parent {
                    if id == *root {
                        return true;
                    }
                    parent = self.document(&id).and_then(|p| p.parent.clone());
                }
                false
            }
        }
    }

    /// The kinds of manuscript among the documents a compile in `scope` could
    /// take (front and back matter, research and folders do not count). More
    /// than one means the writer has to say which kind this export is.
    pub fn compile_types_in(&self, scope: &CompileScope) -> Vec<CompileType> {
        let mut found = Vec::new();
        for doc in self.ordered_documents() {
            if doc.kind.is_folder()
                || !self.in_scope(doc, scope)
                || self.in_research(doc)
                || !doc.include_in_export
                || matches!(
                    doc.kind,
                    DocKind::FrontMatter | DocKind::BackMatter | DocKind::Research | DocKind::Character
                )
            {
                continue;
            }
            let kind = CompileType::from_medium(doc.medium);
            if !found.contains(&kind) {
                found.push(kind);
            }
        }
        found
    }

    /// Work out what a compile with these settings contains. `selected` is the
    /// binder selection: in automatic mode it decides the type when it names an
    /// included document.
    pub fn compile_plan_for(
        &self,
        settings: &crate::ExportSettings,
        selected: Option<&str>,
    ) -> CompilePlan<'_> {
        let candidates: Vec<&Document> = self
            .ordered_documents()
            .into_iter()
            .filter(|d| !d.kind.is_folder() && self.in_scope(d, &settings.scope))
            .collect();

        let counts_toward_auto = |d: &Document| {
            d.include_in_export
                && !self.in_research(d)
                && !matches!(d.kind, DocKind::FrontMatter | DocKind::BackMatter)
                && !matches!(d.kind, DocKind::Research | DocKind::Character)
        };
        let kind = settings.compile_type.unwrap_or_else(|| {
            selected
                .and_then(|id| candidates.iter().find(|d| d.id == id && counts_toward_auto(d)))
                .or_else(|| candidates.iter().find(|d| counts_toward_auto(d)))
                .map(|d| CompileType::from_medium(d.medium))
                .unwrap_or_else(|| CompileType::for_project_kind(self.kind))
        });

        let mut included = Vec::new();
        let mut skipped = Vec::new();
        for doc in candidates {
            let reason = if self.in_research(doc) {
                Some("In the Research folder".to_owned())
            } else if !doc.include_in_export {
                Some("Excluded from compile".to_owned())
            } else if !kind.accepts(doc.medium) {
                Some(format!(
                    "Different format: {}",
                    CompileType::from_medium(doc.medium).label().to_lowercase()
                ))
            } else if kind.is_script()
                && matches!(doc.kind, DocKind::FrontMatter | DocKind::BackMatter)
            {
                Some("Front and back matter apply to books only".to_owned())
            } else {
                None
            };
            match reason {
                Some(reason) => skipped.push(SkippedDocument { doc, reason }),
                None => included.push(doc),
            }
        }
        CompilePlan {
            kind,
            included,
            skipped,
        }
    }

    /// The plan for a compile that does not know the binder selection.
    pub fn compile_plan(&self, settings: &crate::ExportSettings) -> CompilePlan<'_> {
        self.compile_plan_for(settings, None)
    }

    /// Split a script compile into one group per output file. A document whose
    /// parent is a root-level folder (such as Draft) is its own file; documents
    /// inside a deeper folder (an episode folder) share a file.
    pub fn compile_groups(&self, plan: &CompilePlan<'_>) -> Vec<CompileGroup> {
        let mut groups: Vec<(Option<String>, CompileGroup)> = Vec::new();
        for doc in &plan.included {
            let parent = doc.parent.as_deref().and_then(|id| self.document(id));
            let shared = parent.filter(|p| p.parent.is_some());
            let key = shared.map(|p| p.id.clone());
            match key.as_ref().and_then(|k| {
                groups
                    .iter_mut()
                    .find(|(existing, _)| existing.as_ref() == Some(k))
            }) {
                Some((_, group)) => group.ids.push(doc.id.clone()),
                None => groups.push((
                    key,
                    CompileGroup {
                        title: shared.map(|p| p.title.clone()).unwrap_or_else(|| doc.title.clone()),
                        ids: vec![doc.id.clone()],
                    },
                )),
            }
        }
        groups.into_iter().map(|(_, group)| group).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sample, ExportSettings};

    fn settings(kind: Option<CompileType>, scope: CompileScope) -> ExportSettings {
        ExportSettings {
            compile_type: kind,
            scope,
            ..ExportSettings::default()
        }
    }

    fn titles(plan: &CompilePlan<'_>) -> Vec<String> {
        plan.included.iter().map(|d| d.title.clone()).collect()
    }

    #[test]
    fn mixed_projects_report_every_kind_and_single_kind_ones_one() {
        let project = sample::demo();
        let kinds = project.compile_types_in(&CompileScope::Draft);
        assert!(kinds.contains(&CompileType::Prose));
        assert!(kinds.contains(&CompileType::TvScript));
        assert!(kinds.len() > 1);
    }

    #[test]
    fn each_type_includes_only_its_own_documents() {
        let project = sample::demo();
        let prose = project.compile_plan(&settings(Some(CompileType::Prose), CompileScope::Draft));
        assert!(prose.included.iter().all(|d| CompileType::Prose.accepts(d.medium)));
        assert!(prose.included.len() >= 3);
        let tv = project.compile_plan(&settings(Some(CompileType::TvScript), CompileScope::Draft));
        assert_eq!(titles(&tv), vec!["Cold Harbor · Pilot"]);
        let stage = project.compile_plan(&settings(Some(CompileType::StagePlay), CompileScope::Draft));
        assert_eq!(titles(&stage), vec!["The Lamp Room · One act"]);
        let film = project.compile_plan(&settings(Some(CompileType::Screenplay), CompileScope::Draft));
        assert!(film.included.is_empty());
    }

    #[test]
    fn skipped_documents_carry_a_reason() {
        let project = sample::demo();
        let plan = project.compile_plan(&settings(Some(CompileType::TvScript), CompileScope::Draft));
        let reason = |title: &str| {
            plan.skipped
                .iter()
                .find(|s| s.doc.title.starts_with(title))
                .map(|s| s.reason.clone())
                .unwrap_or_default()
        };
        assert!(reason("The Lamp Room").starts_with("Different format"));
        assert_eq!(reason("Wreck of the Anna Rose"), "In the Research folder");
        assert!(reason("Chapter One").contains("prose"));
    }

    #[test]
    fn auto_follows_the_first_included_document() {
        let project = sample::demo();
        let plan = project.compile_plan(&settings(None, CompileScope::Draft));
        assert_eq!(plan.kind, CompileType::Prose);
        let pilot = project.documents.iter().find(|d| d.title.starts_with("Cold Harbor")).unwrap();
        let plan = project.compile_plan(&settings(None, CompileScope::Subtree(pilot.id.clone())));
        assert_eq!(plan.kind, CompileType::TvScript);
        assert_eq!(titles(&plan), vec!["Cold Harbor · Pilot"]);
    }

    #[test]
    fn subtree_scope_takes_a_folder_and_its_descendants() {
        let project = sample::demo();
        let part = project.documents.iter().find(|d| d.title.starts_with("Part Two")).unwrap();
        let plan = project.compile_plan(&settings(None, CompileScope::Subtree(part.id.clone())));
        assert_eq!(titles(&plan), vec!["Chapter Three · A Borrowed Name"]);
    }

    #[test]
    fn excluded_documents_are_reported_not_dropped_silently() {
        let mut project = sample::demo();
        let id = project.documents.iter().find(|d| d.title.starts_with("Chapter One")).unwrap().id.clone();
        project.document_mut(&id).unwrap().include_in_export = false;
        let plan = project.compile_plan(&settings(Some(CompileType::Prose), CompileScope::Draft));
        assert!(!titles(&plan).iter().any(|t| t.starts_with("Chapter One")));
        assert!(plan.skipped.iter().any(|s| s.doc.id == id && s.reason == "Excluded from compile"));
    }

    #[test]
    fn separate_scripts_group_by_episode_folder() {
        let mut project = sample::demo();
        let draft = project.folder_id("Draft").unwrap();
        let ep = project.add_document(Some(draft.clone()), "Episode 2", DocKind::Folder, Medium::Prose);
        let a = project.add_document(Some(ep.clone()), "Teaser", DocKind::Scene, Medium::TvScript);
        let b = project.add_document(Some(ep.clone()), "Act one", DocKind::Scene, Medium::TvScript);
        let solo = project.add_document(Some(draft), "Standalone", DocKind::Scene, Medium::TvScript);
        let plan = project.compile_plan(&settings(Some(CompileType::TvScript), CompileScope::Draft));
        let groups = project.compile_groups(&plan);
        let episode = groups.iter().find(|g| g.title == "Episode 2").expect("episode group");
        assert_eq!(episode.ids, vec![a, b]);
        assert!(groups.iter().any(|g| g.title == "Standalone" && g.ids == vec![solo.clone()]));
        assert!(groups.iter().any(|g| g.title.starts_with("Cold Harbor")));
    }
}
