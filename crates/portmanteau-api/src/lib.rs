//! The command bus.
//!
//! The UI, the MCP server and the CLI all speak the same JSON commands. The
//! desktop layer is a view onto this API rather than a second implementation,
//! which keeps agent testing honest.

use std::path::{Path, PathBuf};

pub mod storage;

use portmanteau_core::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub ok: bool,
    pub command: String,
    pub message: String,
    pub data: Value,
}

impl ApiResponse {
    fn ok(command: &str, message: impl Into<String>, data: Value) -> Self {
        Self {
            ok: true,
            command: command.into(),
            message: message.into(),
            data,
        }
    }
    fn err(command: &str, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            command: command.into(),
            message: message.into(),
            data: Value::Null,
        }
    }
}

pub struct Workspace {
    pub project: Project,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub log: Vec<String>,
    pub last_export: Option<portmanteau_export::ExportInfo>,
    pub last_message: String,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::demo()
    }
}

impl Workspace {
    pub fn demo() -> Self {
        Self {
            project: sample::demo(),
            path: None,
            dirty: false,
            log: vec!["Opened the demonstration project".into()],
            last_export: None,
            last_message: "Ready".into(),
        }
    }

    pub fn new_project(title: &str, kind: ProjectKind) -> Self {
        let mut project = Project::new(title, kind);
        project.author = "Your name".into();
        Self {
            project,
            path: None,
            dirty: true,
            log: vec![format!("Created {}", title)],
            last_export: None,
            last_message: "New project".into(),
        }
    }

    pub fn open(path: &Path) -> Result<Self, String> {
        if !path.is_dir() {
            return Err("Not a Portmanteau project (expected a .portmanteau folder)".into());
        }
        let project = storage::load_folder(path)?;
        Ok(Self {
            project,
            path: Some(path.to_path_buf()),
            dirty: false,
            log: vec![format!("Opened {}", path.display())],
            last_export: None,
            last_message: "Project opened".into(),
        })
    }

    pub fn save(&mut self, path: Option<PathBuf>) -> Result<String, String> {
        let target = path
            .or_else(|| self.path.clone())
            .unwrap_or_else(|| default_save_path(&self.project));
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        storage::save_folder(&self.project, &target)?;
        self.path = Some(target.clone());
        self.dirty = false;
        let msg = format!("Saved {}", target.display());
        self.log.push(msg.clone());
        Ok(msg)
    }

    pub fn execute(&mut self, command: &str, payload: Value) -> ApiResponse {
        match command {
            "project.info" => ApiResponse::ok(command, "Project", self.project_summary()),
            "project.new" => {
                let title = str_arg(&payload, "title").unwrap_or_else(|| "Untitled".into());
                let kind = payload
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .and_then(parse_project_kind)
                    .unwrap_or(ProjectKind::Novel);
                self.project = Project::new(&title, kind);
                self.dirty = true;
                self.log.push(format!("Created {}", title));
                ApiResponse::ok(command, "Project created", self.project_summary())
            }
            "project.open" => {
                let Some(path) = str_arg(&payload, "path") else {
                    return ApiResponse::err(command, "A path is required");
                };
                match Workspace::open(Path::new(&path)) {
                    Ok(ws) => {
                        *self = ws;
                        ApiResponse::ok(command, "Project opened", self.project_summary())
                    }
                    Err(e) => ApiResponse::err(command, e),
                }
            }
            "project.save" => {
                let path = str_arg(&payload, "path").map(PathBuf::from);
                match self.save(path) {
                    Ok(msg) => ApiResponse::ok(command, msg.clone(), json!({"message": msg})),
                    Err(e) => ApiResponse::err(command, e),
                }
            }
            "project.update" => {
                if let Some(v) = payload.get("title").and_then(|v| v.as_str()) {
                    self.project.title = v.into();
                }
                if let Some(v) = payload.get("subtitle").and_then(|v| v.as_str()) {
                    self.project.subtitle = v.into();
                }
                if let Some(v) = payload.get("author").and_then(|v| v.as_str()) {
                    self.project.author = v.into();
                }
                if let Some(v) = payload.get("genre").and_then(|v| v.as_str()) {
                    self.project.genre = v.into();
                }
                if let Some(v) = payload.get("logline").and_then(|v| v.as_str()) {
                    self.project.logline = v.into();
                }
                if let Some(v) = payload.get("daily_target").and_then(|v| v.as_u64()) {
                    self.project.daily_target = v as u32;
                }
                self.project.touch();
                self.dirty = true;
                ApiResponse::ok(command, "Project details updated", json!({"updated": true}))
            }
            "project.checkpoint" => {
                let label = str_arg(&payload, "label").unwrap_or_else(|| "Manual checkpoint".into());
                let id = self.project.checkpoint(label, "manual");
                self.dirty = true;
                ApiResponse::ok(command, "Checkpoint saved", json!({"id": id}))
            }
            "project.restore_checkpoint" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A checkpoint id is required");
                };
                if self.project.restore_checkpoint(&id) {
                    self.dirty = true;
                    ApiResponse::ok(command, "Project restored", self.project_summary())
                } else {
                    ApiResponse::err(command, "Checkpoint not found")
                }
            }
            "project.snapshots" => {
                let mut list = Vec::new();
                for doc in &self.project.documents {
                    for snap in &doc.snapshots {
                        list.push(json!({
                            "id": snap.id,
                            "document_id": doc.id,
                            "document": doc.title,
                            "name": snap.name,
                            "note": snap.note,
                            "created": snap.created,
                            "words": snap.words,
                        }));
                    }
                }
                ApiResponse::ok(command, "Snapshots", Value::Array(list))
            }
            "document.list" => {
                let list: Vec<Value> = self
                    .project
                    .ordered_documents()
                    .into_iter()
                    .map(|d| {
                        json!({
                            "id": d.id,
                            "parent": d.parent,
                            "title": d.title,
                            "kind": d.kind,
                            "medium": d.medium,
                            "words": d.words(),
                            "pages": d.pages(),
                            "status": d.status,
                            "label": d.label,
                            "include_in_export": d.include_in_export,
                        })
                    })
                    .collect();
                ApiResponse::ok(command, "Documents", Value::Array(list))
            }
            "document.get" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A document id is required");
                };
                match self.project.document(&id) {
                    Some(doc) => ApiResponse::ok(
                        command,
                        "Document",
                        serde_json::to_value(doc).unwrap_or(Value::Null),
                    ),
                    None => ApiResponse::err(command, "Document not found"),
                }
            }
            "document.create" => {
                let parent = str_arg(&payload, "parent")
                    .or_else(|| self.project.folder_id("Draft"));
                let title = str_arg(&payload, "title").unwrap_or_else(|| "Untitled".into());
                let kind = payload
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .and_then(parse_doc_kind)
                    .unwrap_or(DocKind::Text);
                let medium = payload
                    .get("medium")
                    .and_then(|v| v.as_str())
                    .and_then(parse_medium)
                    .unwrap_or(Medium::Prose);
                let id = self
                    .project
                    .add_document(parent, &title, kind, medium);
                self.dirty = true;
                ApiResponse::ok(command, "Document created", json!({"id": id}))
            }
            "document.update" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A document id is required");
                };
                let Some(doc) = self.project.document_mut(&id) else {
                    return ApiResponse::err(command, "Document not found");
                };
                if let Some(v) = payload.get("title").and_then(|v| v.as_str()) {
                    doc.title = v.into();
                }
                if let Some(v) = payload.get("synopsis").and_then(|v| v.as_str()) {
                    doc.synopsis = v.into();
                }
                if let Some(v) = payload.get("notes").and_then(|v| v.as_str()) {
                    doc.notes = v.into();
                }
                if let Some(v) = payload.get("text").and_then(|v| v.as_str()) {
                    if doc.medium.is_script() {
                        doc.blocks = script::parse_loose(v);
                    } else {
                        doc.text = v.into();
                    }
                }
                if let Some(v) = payload.get("label").and_then(|v| v.as_str()) {
                    doc.label = v.into();
                }
                if let Some(v) = payload.get("status").and_then(|v| v.as_str()) {
                    doc.status = v.into();
                }
                if let Some(v) = payload.get("include_in_export").and_then(|v| v.as_bool()) {
                    doc.include_in_export = v;
                }
                if let Some(v) = payload.get("word_target").and_then(|v| v.as_u64()) {
                    doc.word_target = v as u32;
                }
                if let Some(v) = payload.get("medium").and_then(|v| v.as_str()) {
                    if let Some(m) = parse_medium(v) {
                        doc.convert_medium(m);
                    }
                }
                doc.touch();
                self.dirty = true;
                ApiResponse::ok(command, "Document updated", json!({"id": id}))
            }
            "document.delete" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A document id is required");
                };
                self.project.delete_document(&id);
                self.dirty = true;
                ApiResponse::ok(command, "Document deleted", json!({"id": id}))
            }
            "document.move" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A document id is required");
                };
                let moved = if payload.get("parent").is_some() {
                    let parent = payload
                        .get("parent")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    self.project.reparent_document(&id, parent)
                } else {
                    let direction = match payload
                        .get("direction")
                        .and_then(|v| v.as_str())
                        .unwrap_or("down")
                        .to_ascii_lowercase()
                        .as_str()
                    {
                        "up" | "-1" => -1,
                        "down" | "1" => 1,
                        other => {
                            return ApiResponse::err(
                                command,
                                format!("Unknown direction “{other}”; use up or down"),
                            )
                        }
                    };
                    self.project.move_document(&id, direction)
                };
                if moved {
                    self.dirty = true;
                    ApiResponse::ok(command, "Document moved", json!({"id": id}))
                } else {
                    ApiResponse::err(command, "The document cannot move there")
                }
            }
            "document.snapshot" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A document id is required");
                };
                let name = str_arg(&payload, "name").unwrap_or_else(|| "Snapshot".into());
                let note = str_arg(&payload, "note").unwrap_or_default();
                match self.project.document_mut(&id) {
                    Some(doc) => {
                        doc.snapshot(name.clone(), note);
                        self.dirty = true;
                        ApiResponse::ok(
                            command,
                            format!("Snapshot “{name}” saved"),
                            json!({"id": id, "name": name}),
                        )
                    }
                    None => ApiResponse::err(command, "Document not found"),
                }
            }
            "snapshot.restore" => {
                let doc_id = str_arg(&payload, "document_id").unwrap_or_default();
                let Some(snap_id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A snapshot id is required");
                };
                match self.project.document_mut(&doc_id) {
                    Some(doc) => {
                        if doc.restore_snapshot(&snap_id) {
                            self.dirty = true;
                            ApiResponse::ok(command, "Snapshot restored", json!({"id": doc_id}))
                        } else {
                            ApiResponse::err(command, "Snapshot not found")
                        }
                    }
                    None => ApiResponse::err(command, "Document not found"),
                }
            }
            "snapshot.diff" => {
                let doc_id = str_arg(&payload, "document_id").unwrap_or_default();
                let Some(snap_id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A snapshot id is required");
                };
                let Some(doc) = self.project.document(&doc_id) else {
                    return ApiResponse::err(command, "Document not found");
                };
                let Some(snap) = doc.snapshots.iter().find(|s| s.id == snap_id) else {
                    return ApiResponse::err(command, "Snapshot not found");
                };
                let before = snap.body_text();
                let after = doc.body_text();
                let lines = portmanteau_core::diff_lines(&before, &after);
                let (added, removed) = portmanteau_core::diff_counts(&lines);
                ApiResponse::ok(
                    command,
                    format!("{added} added · {removed} removed"),
                    json!({
                        "document_id": doc_id,
                        "document": doc.title,
                        "snapshot_id": snap.id,
                        "snapshot": snap.name,
                        "created": snap.created,
                        "words_before": snap.words,
                        "words_after": doc.words(),
                        "added": added,
                        "removed": removed,
                        "lines": lines,
                    }),
                )
            }
            "character.add" => {
                let name = str_arg(&payload, "name").unwrap_or_else(|| "New character".into());
                let role = str_arg(&payload, "role").unwrap_or_default();
                let mut c = Character::new(name, role);
                if let Some(v) = payload.get("description").and_then(|v| v.as_str()) {
                    c.description = v.into();
                }
                let id = c.id.clone();
                self.project.characters.push(c);
                self.dirty = true;
                ApiResponse::ok(command, "Character added", json!({"id": id}))
            }
            "character.update" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A character id is required");
                };
                let Some(c) = self.project.characters.iter_mut().find(|c| c.id == id) else {
                    return ApiResponse::err(command, "Character not found");
                };
                for (key, field) in [
                    ("name", &mut c.name),
                    ("role", &mut c.role),
                    ("description", &mut c.description),
                    ("want", &mut c.want),
                    ("need", &mut c.need),
                    ("flaw", &mut c.flaw),
                    ("arc", &mut c.arc),
                ] {
                    if let Some(v) = payload.get(key).and_then(|v| v.as_str()) {
                        *field = v.into();
                    }
                }
                self.dirty = true;
                ApiResponse::ok(command, "Character updated", json!({"id": id}))
            }
            "research.add" => {
                let title = str_arg(&payload, "title").unwrap_or_else(|| "New note".into());
                let body = str_arg(&payload, "body").unwrap_or_default();
                let kind = str_arg(&payload, "kind").unwrap_or_else(|| "Note".into());
                let source = str_arg(&payload, "source").unwrap_or_default();
                let item = ResearchItem {
                    id: new_id("res"),
                    title: title.clone(),
                    body: body.clone(),
                    kind,
                    source,
                    created: now_stamp(),
                };
                let id = item.id.clone();
                self.project.research.push(item);
                if let Some(folder) = self.project.folder_id("Research") {
                    let doc_id =
                        self.project
                            .add_document(Some(folder), &title, DocKind::Research, Medium::Note);
                    if let Some(doc) = self.project.document_mut(&doc_id) {
                        doc.text = body;
                        doc.include_in_export = false;
                    }
                }
                self.dirty = true;
                ApiResponse::ok(command, "Research note added", json!({"id": id}))
            }
            "idea.add" => {
                let title = str_arg(&payload, "title").unwrap_or_else(|| "New idea".into());
                let body = str_arg(&payload, "body").unwrap_or_default();
                let kind = str_arg(&payload, "kind").unwrap_or_else(|| "Idea".into());
                let card = IdeaCard {
                    id: new_id("idea"),
                    title,
                    body,
                    kind,
                    color: self.project.ideas.len() as u8 % 6,
                    pinned: false,
                };
                let id = card.id.clone();
                self.project.ideas.push(card);
                self.dirty = true;
                ApiResponse::ok(command, "Idea added", json!({"id": id}))
            }
            "idea.update" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "An idea id is required");
                };
                let Some(card) = self.project.ideas.iter_mut().find(|i| i.id == id) else {
                    return ApiResponse::err(command, "Idea not found");
                };
                if let Some(v) = payload.get("title").and_then(|v| v.as_str()) {
                    card.title = v.into();
                }
                if let Some(v) = payload.get("body").and_then(|v| v.as_str()) {
                    card.body = v.into();
                }
                if let Some(v) = payload.get("pinned").and_then(|v| v.as_bool()) {
                    card.pinned = v;
                }
                self.dirty = true;
                ApiResponse::ok(command, "Idea updated", json!({"id": id}))
            }
            "idea.delete" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "An idea id is required");
                };
                self.project.ideas.retain(|i| i.id != id);
                self.dirty = true;
                ApiResponse::ok(command, "Idea deleted", json!({"id": id}))
            }
            "beat.update" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "A beat id is required");
                };
                let Some(beat) = self.project.beats.iter_mut().find(|b| b.id == id) else {
                    return ApiResponse::err(command, "Beat not found");
                };
                if let Some(v) = payload.get("summary").and_then(|v| v.as_str()) {
                    beat.summary = v.into();
                }
                if let Some(v) = payload.get("done").and_then(|v| v.as_bool()) {
                    beat.done = v;
                }
                self.dirty = true;
                ApiResponse::ok(command, "Beat updated", json!({"id": id}))
            }
            "search" => {
                let q = str_arg(&payload, "query").unwrap_or_default();
                let hits: Vec<Value> = self
                    .project
                    .search(&q)
                    .into_iter()
                    .map(|(doc, excerpt)| json!({"id": doc.id, "title": doc.title, "excerpt": excerpt}))
                    .collect();
                ApiResponse::ok(command, format!("{} matches", hits.len()), Value::Array(hits))
            }
            "stats" => ApiResponse::ok(command, "Statistics", self.stats()),
            "script.parse" => {
                let text = str_arg(&payload, "text").unwrap_or_default();
                let medium = payload
                    .get("medium")
                    .and_then(|v| v.as_str())
                    .and_then(parse_medium)
                    .unwrap_or(Medium::Screenplay);
                let blocks = script::parse_loose(&text);
                ApiResponse::ok(
                    command,
                    "Parsed script",
                    json!({"count": blocks.len(), "fountain": script::to_fountain(&blocks, medium)}),
                )
            }
            "export.formats" => {
                let formats: Vec<Value> = ExportKind::all()
                    .iter()
                    .map(|k| {
                        json!({
                            "id": k.label(),
                            "extension": k.extension(),
                            "description": k.description(),
                        })
                    })
                    .collect();
                ApiResponse::ok(command, "Export formats", Value::Array(formats))
            }
            "export.preview" => {
                let sections: Vec<Value> = self
                    .project
                    .manuscript_documents()
                    .into_iter()
                    .filter(|d| !matches!(d.kind, DocKind::FrontMatter | DocKind::BackMatter))
                    .map(|d| {
                        json!({
                            "title": d.title,
                            "medium": d.medium,
                            "words": d.words(),
                            "pages": d.pages(),
                        })
                    })
                    .collect();
                ApiResponse::ok(
                    command,
                    "Compile preview",
                    json!({
                        "title": self.project.title,
                        "author": self.project.author,
                        "sections": sections,
                        "words": self.project.words(),
                        "pages": sections.iter().filter_map(|s| s["pages"].as_u64()).sum::<u64>(),
                    }),
                )
            }
            "export.run" => {
                let kind = match payload.get("format") {
                    Some(value) => {
                        let raw = value
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| value.to_string());
                        match parse_export_kind(&raw) {
                            Some(k) => k,
                            None => {
                                return ApiResponse::err(
                                    command,
                                    format!("Unknown format: {}", raw),
                                )
                            }
                        }
                    }
                    None => self.project.export.kind,
                };
                let mut settings = self.project.settings_for(kind);
                settings.kind = kind;
                if let Err(e) = apply_settings_overrides(&mut settings, &payload) {
                    return ApiResponse::err(command, e);
                }
                self.project.set_settings(settings.clone());
                self.project.export = settings.clone();
                let target = if settings.output_path.trim().is_empty() {
                    None
                } else {
                    let candidate = PathBuf::from(settings.output_path.trim());
                    // A folder (no extension) receives the standard filename;
                    // a full path is used exactly as given.
                    if candidate.extension().is_some() {
                        Some(candidate)
                    } else {
                        Some(candidate.join(portmanteau_export::default_filename(
                            &self.project,
                            settings.kind,
                        )))
                    }
                };
                match portmanteau_export::export(&self.project, &settings, target.as_deref()) {
                    Ok(info) => {
                        self.last_message = info.message.clone();
                        self.log.push(format!("Exported {} → {}", info.format, info.path));
                        self.project.checkpoint(format!("Before export · {}", info.format), "export");
                        self.last_export = Some(info.clone());
                        self.dirty = true;
                        ApiResponse::ok(
                            command,
                            info.message.clone(),
                            serde_json::to_value(info).unwrap_or(Value::Null),
                        )
                    }
                    Err(e) => ApiResponse::err(command, e),
                }
            }
            "export.settings.get" => {
                let kind = match payload.get("format") {
                    Some(value) => {
                        let raw = value
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| value.to_string());
                        match parse_export_kind(&raw) {
                            Some(k) => k,
                            None => {
                                return ApiResponse::err(
                                    command,
                                    format!("Unknown format: {}", raw),
                                )
                            }
                        }
                    }
                    None => return ApiResponse::err(command, "A format is required"),
                };
                let settings = self.project.settings_for(kind);
                ApiResponse::ok(
                    command,
                    "Export settings",
                    serde_json::to_value(settings).unwrap_or(Value::Null),
                )
            }
            "export.settings.update" => {
                let kind = match payload.get("format") {
                    Some(value) => {
                        let raw = value
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| value.to_string());
                        match parse_export_kind(&raw) {
                            Some(k) => k,
                            None => {
                                return ApiResponse::err(
                                    command,
                                    format!("Unknown format: {}", raw),
                                )
                            }
                        }
                    }
                    None => return ApiResponse::err(command, "A format is required"),
                };
                let mut settings = self.project.settings_for(kind);
                settings.kind = kind;
                if let Err(e) = apply_settings_overrides(&mut settings, &payload) {
                    return ApiResponse::err(command, e);
                }
                self.project.set_settings(settings.clone());
                self.dirty = true;
                ApiResponse::ok(
                    command,
                    "Export settings updated",
                    serde_json::to_value(settings).unwrap_or(Value::Null),
                )
            }
            "style.get" => ApiResponse::ok(
                command,
                "Project style",
                serde_json::to_value(&self.project.style).unwrap_or(Value::Null),
            ),
            "style.update" => {
                // Validate every numeric field before changing anything, so a
                // rejected value cannot leave the style half-updated.
                let parsed: Result<[Option<f32>; 4], String> = (|| {
                    let prose_size = payload
                        .get("prose_size")
                        .map(|v| parse_finite_f32(v, "prose_size", 4.0, 72.0))
                        .transpose()?;
                    let prose_leading = payload
                        .get("prose_leading")
                        .map(|v| parse_finite_f32(v, "prose_leading", 0.5, 3.0))
                        .transpose()?;
                    let script_size = payload
                        .get("script_size")
                        .map(|v| parse_finite_f32(v, "script_size", 4.0, 72.0))
                        .transpose()?;
                    let script_leading = payload
                        .get("script_leading")
                        .map(|v| parse_finite_f32(v, "script_leading", 0.5, 3.0))
                        .transpose()?;
                    Ok([prose_size, prose_leading, script_size, script_leading])
                })();
                let [prose_size, prose_leading, script_size, script_leading] = match parsed {
                    Ok(values) => values,
                    Err(e) => return ApiResponse::err(command, e),
                };
                if let Some(v) = payload.get("prose_font").and_then(|v| v.as_str()) {
                    self.project.style.prose_font = v.into();
                }
                if let Some(v) = prose_size {
                    self.project.style.prose_size = v;
                }
                if let Some(v) = prose_leading {
                    self.project.style.prose_leading = v;
                }
                if let Some(v) = payload.get("script_font").and_then(|v| v.as_str()) {
                    self.project.style.script_font = v.into();
                }
                if let Some(v) = script_size {
                    self.project.style.script_size = v;
                }
                if let Some(v) = script_leading {
                    self.project.style.script_leading = v;
                }
                self.dirty = true;
                ApiResponse::ok(
                    command,
                    "Project style updated",
                    serde_json::to_value(&self.project.style).unwrap_or(Value::Null),
                )
            }
            "commands" => ApiResponse::ok(command, "Command catalogue", json!(command_catalogue())),
            _ => ApiResponse::err(command, format!("Unknown command: {command}")),
        }
    }

    pub fn project_summary(&self) -> Value {
        json!({
            "id": self.project.id,
            "title": self.project.title,
            "subtitle": self.project.subtitle,
            "author": self.project.author,
            "kind": self.project.kind,
            "words": self.project.words(),
            "documents": self.project.documents.len(),
            "characters": self.project.characters.len(),
            "research": self.project.research.len(),
            "checkpoints": self.project.checkpoints.len(),
            "today_words": self.project.today_words(),
            "daily_target": self.project.daily_target,
            "dirty": self.dirty,
            "path": self.path.as_ref().map(|p| p.display().to_string()),
        })
    }

    pub fn stats(&self) -> Value {
        let docs = self.project.manuscript_documents();
        let script_words: u32 = docs
            .iter()
            .filter(|d| d.medium.is_script())
            .map(|d| d.words())
            .sum();
        let prose_words: u32 = docs
            .iter()
            .filter(|d| !d.medium.is_script())
            .map(|d| d.words())
            .sum();
        json!({
            "words": self.project.words(),
            "prose_words": prose_words,
            "script_words": script_words,
            "pages": docs.iter().map(|d| d.pages()).sum::<u32>(),
            "documents": self.project.documents.len(),
            "scenes": self.project.documents.iter().filter(|d| d.kind == DocKind::Scene).count(),
            "chapters": self.project.documents.iter().filter(|d| d.kind == DocKind::Chapter).count(),
            "characters": self.project.characters.len(),
            "research": self.project.research.len(),
            "ideas": self.project.ideas.len(),
            "locations": self.project.locations().len(),
            "today": self.project.today_words(),
            "target": self.project.daily_target,
            "session": self.project.session_log,
        })
    }
}

pub fn default_save_path(project: &Project) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join("Portmanteau")
        .join(format!("{}.portmanteau", storage::slugify(&project.title)))
}

fn str_arg(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Parse a numeric setting that must be a finite number in `min..=max`.
/// This runs at the command boundary so an overflowing value such as `1e39`
/// is rejected instead of becoming f32 infinity (which serde_json writes as
/// `null` and makes the project unopenable).
fn parse_finite_f32(value: &Value, key: &str, min: f32, max: f32) -> Result<f32, String> {
    let raw = value
        .as_f64()
        .ok_or_else(|| format!("{key} must be a number"))?;
    let parsed = raw as f32;
    if !parsed.is_finite() || parsed < min || parsed > max {
        return Err(format!(
            "{key} must be a finite number between {min} and {max}"
        ));
    }
    Ok(parsed)
}

/// Merge the settings fields a caller may supply on top of a resolved set.
/// Unknown keys are ignored, so a newer client can send extra fields safely.
fn apply_settings_overrides(settings: &mut ExportSettings, payload: &Value) -> Result<(), String> {
    if let Some(v) = payload.get("path").and_then(|v| v.as_str()) {
        settings.output_path = v.into();
    }
    if let Some(v) = payload.get("output_path").and_then(|v| v.as_str()) {
        settings.output_path = v.into();
    }
    if let Some(v) = payload.get("paper").and_then(|v| v.as_str()) {
        if let Some(p) = parse_paper(v) {
            settings.paper = p;
        }
    }
    if let Some(v) = payload.get("font").and_then(|v| v.as_str()) {
        settings.font = v.into();
    }
    if let Some(v) = payload.get("font_size") {
        settings.font_size = parse_finite_f32(v, "font_size", 4.0, 72.0)?;
    }
    if let Some(v) = payload.get("line_spacing") {
        settings.line_spacing = parse_finite_f32(v, "line_spacing", 0.5, 3.0)?;
    }
    if let Some(v) = payload.get("cover_image").and_then(|v| v.as_str()) {
        settings.cover_image = v.into();
    }
    for (key, field) in [
        ("include_page_numbers", &mut settings.include_page_numbers),
        ("include_running_headers", &mut settings.include_running_headers),
        ("include_title_page", &mut settings.include_title_page),
        ("include_copyright", &mut settings.include_copyright),
        ("include_dedication", &mut settings.include_dedication),
        ("include_acknowledgements", &mut settings.include_acknowledgements),
        ("include_preface", &mut settings.include_preface),
        ("include_toc", &mut settings.include_toc),
        ("include_chapter_numbers", &mut settings.include_chapter_numbers),
        ("include_scene_numbers", &mut settings.include_scene_numbers),
        ("include_synopsis", &mut settings.include_synopsis),
        ("include_notes", &mut settings.include_notes),
        ("include_author_bio", &mut settings.include_author_bio),
    ] {
        if let Some(v) = payload.get(key).and_then(|v| v.as_bool()) {
            *field = v;
        }
    }
    Ok(())
}

fn parse_project_kind(s: &str) -> Option<ProjectKind> {
    ProjectKind::all()
        .into_iter()
        .find(|k| format!("{k:?}").eq_ignore_ascii_case(s) || k.label().eq_ignore_ascii_case(s))
}

fn parse_doc_kind(s: &str) -> Option<DocKind> {
    [
        DocKind::Folder,
        DocKind::Chapter,
        DocKind::Scene,
        DocKind::Text,
        DocKind::Research,
        DocKind::Character,
        DocKind::FrontMatter,
        DocKind::BackMatter,
    ]
    .into_iter()
    .find(|k| format!("{k:?}").eq_ignore_ascii_case(s) || k.label().eq_ignore_ascii_case(s))
}

fn parse_medium(s: &str) -> Option<Medium> {
    Medium::all()
        .into_iter()
        .find(|k| format!("{k:?}").eq_ignore_ascii_case(s) || k.label().eq_ignore_ascii_case(s))
}

fn parse_export_kind(s: &str) -> Option<ExportKind> {
    ExportKind::all().into_iter().find(|k| {
        format!("{k:?}").eq_ignore_ascii_case(s)
            || k.label().eq_ignore_ascii_case(s)
            || k.short().eq_ignore_ascii_case(s)
    })
}

fn parse_paper(s: &str) -> Option<PaperSize> {
    PaperSize::all()
        .into_iter()
        .find(|p| format!("{p:?}").eq_ignore_ascii_case(s) || p.label().eq_ignore_ascii_case(s))
}

/// One-line catalogue used by the UI's agent panel and by MCP tool discovery.
pub fn command_catalogue() -> Vec<Value> {
    let entries = [
        ("project.info", "Summarise the open project", "{}"),
        ("project.new", "Create a project", r#"{"title":"...","kind":"Novel"}"#),
        ("project.open", "Open a project folder package", r#"{"path":"..."}"#),
        ("project.save", "Save the project to a folder package", r#"{"path":"..."}"#),
        ("project.checkpoint", "Save a full version checkpoint", r#"{"label":"..."}"#),
        ("project.restore_checkpoint", "Restore a checkpoint", r#"{"id":"cp-..."}"#),
        ("project.snapshots", "List document snapshots", "{}"),
        ("document.list", "List every binder document", "{}"),
        ("document.get", "Read one document", r#"{"id":"doc-..."}"#),
        ("document.create", "Add a document", r#"{"parent":"doc-...","title":"...","kind":"Chapter","medium":"Prose"}"#),
        ("document.update", "Change title, text, synopsis or metadata", r#"{"id":"doc-...","text":"..."}"#),
        ("document.delete", "Delete a document and its children", r#"{"id":"doc-..."}"#),
        ("document.move", "Move a document up, down or into a folder", r#"{"id":"doc-...","direction":"up"}"#),
        ("document.snapshot", "Snapshot a document", r#"{"id":"doc-...","name":"..."}"#),
        ("snapshot.restore", "Restore a document snapshot", r#"{"document_id":"...","id":"snap-..."}"#),
        ("snapshot.diff", "Compare a snapshot with the current document", r#"{"document_id":"...","id":"snap-..."}"#),
        ("character.add", "Add a character", r#"{"name":"...","role":"..."}"#),
        ("research.add", "Add a research note", r#"{"title":"...","body":"..."}"#),
        ("idea.add", "Add an idea card", r#"{"title":"...","body":"..."}"#),
        ("beat.update", "Tick or rewrite a story beat", r#"{"id":"beat-...","done":true}"#),
        ("search", "Search the whole manuscript", r#"{"query":"..."}"#),
        ("stats", "Word, page and session statistics", "{}"),
        ("script.parse", "Parse plain text into script blocks", r#"{"text":"INT. ..."}"#),
        ("export.preview", "Preview the compiled manuscript", "{}"),
        ("export.formats", "List supported output formats", "{}"),
        ("export.settings.get", "Read the settings for one output format", r#"{"format":"PDF"}"#),
        ("export.settings.update", "Change the settings for one output format", r#"{"format":"PDF","font_size":11.5}"#),
        ("style.get", "Read the project's base prose and script style", "{}"),
        ("style.update", "Change the project's base style", r#"{"prose_font":"Georgia","prose_size":12.0}"#),
        ("export.run", "Export the manuscript", r#"{"format":"PDF","path":"exports"}"#),
        ("commands", "This catalogue", "{}"),
    ];
    entries
        .iter()
        .map(|(name, description, example)| {
            json!({"name": name, "description": description, "example": example})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_roundtrip_creates_document() {
        let mut ws = Workspace::demo();
        let before = ws.project.documents.len();
        let res = ws.execute(
            "document.create",
            json!({"title":"New scene","kind":"Scene","medium":"Screenplay"}),
        );
        assert!(res.ok);
        assert_eq!(ws.project.documents.len(), before + 1);
    }

    #[test]
    fn unknown_command_fails() {
        let mut ws = Workspace::demo();
        assert!(!ws.execute("does.not.exist", json!({})).ok);
    }

    #[test]
    fn stats_counts_manuscript() {
        let ws = Workspace::demo();
        let s = ws.stats();
        assert!(s["words"].as_u64().unwrap() > 500);
        assert_eq!(s["chapters"].as_u64().unwrap(), 3);
    }

    #[test]
    fn move_command_reorders_siblings() {
        let mut ws = Workspace::demo();
        let ids: Vec<String> = ws
            .project
            .children(ws.project.folder_id("Draft").as_deref())
            .iter()
            .map(|d| d.id.clone())
            .collect();
        assert!(ids.len() >= 2);
        let res = ws.execute("document.move", json!({"id": ids[1], "direction":"up"}));
        assert!(res.ok, "{}", res.message);
        let after: Vec<String> = ws
            .project
            .children(ws.project.folder_id("Draft").as_deref())
            .iter()
            .map(|d| d.id.clone())
            .collect();
        assert_eq!(after[0], ids[1]);
    }

    #[test]
    fn snapshot_diff_compares_against_current_text() {
        let mut ws = Workspace::demo();
        let id = ws
            .project
            .documents
            .iter()
            .find(|d| d.kind == DocKind::Chapter)
            .map(|d| d.id.clone())
            .unwrap();
        ws.execute("document.snapshot", json!({"id": id, "name":"Before"}));
        let snap = ws.project.document(&id).unwrap().snapshots[0].id.clone();
        ws.execute(
            "document.update",
            json!({"id": id, "text":"A completely different chapter."}),
        );
        let res = ws.execute(
            "snapshot.diff",
            json!({"document_id": id, "id": snap}),
        );
        assert!(res.ok, "{}", res.message);
        assert!(res.data["added"].as_u64().unwrap() > 0);
        assert!(res.data["lines"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn export_command_writes_a_file() {
        let mut ws = Workspace::demo();
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("api-export"));
        let res = ws.execute(
            "export.run",
            json!({"format":"Fountain","path":dir.display().to_string()}),
        );
        assert!(res.ok, "{}", res.message);
        assert!(ws.last_export.is_some());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn export_settings_are_per_format_and_round_trip() {
        let mut ws = Workspace::demo();
        let epub_before = ws.execute("export.settings.get", json!({"format":"EPUB"}));
        assert!(epub_before.ok, "{}", epub_before.message);
        assert_eq!(epub_before.data["kind"], "Epub");
        assert_eq!(epub_before.data["font_size"].as_f64().unwrap(), 12.0);

        let updated = ws.execute(
            "export.settings.update",
            json!({
                "format":"PDF",
                "font_size":15.5,
                "line_spacing":1.9,
                "include_page_numbers":false,
                "include_running_headers":true,
                "cover_image":"assets/cover.png"
            }),
        );
        assert!(updated.ok, "{}", updated.message);
        assert_eq!(updated.data["font_size"].as_f64().unwrap(), 15.5);
        assert_eq!(updated.data["include_page_numbers"], false);

        assert_eq!(ws.project.settings_for(ExportKind::Pdf).font_size, 15.5);
        assert_eq!(ws.project.settings_for(ExportKind::Pdf).line_spacing, 1.9);
        assert!(ws.project.settings_for(ExportKind::Pdf).include_running_headers);
        assert_eq!(ws.project.settings_for(ExportKind::Pdf).cover_image, "assets/cover.png");

        let epub_after = ws.execute("export.settings.get", json!({"format":"EPUB"}));
        assert_eq!(epub_after.data, epub_before.data);
        assert_eq!(ws.project.settings_for(ExportKind::Epub).font_size, 12.0);
    }

    #[test]
    fn export_commands_reject_unknown_formats() {
        let mut ws = Workspace::demo();
        assert!(!ws.execute("export.settings.get", json!({"format":"BOGUS"})).ok);
        assert!(
            !ws.execute(
                "export.settings.update",
                json!({"format":"BOGUS","font_size":14.0})
            )
            .ok
        );
        assert!(
            !ws.execute(
                "export.run",
                json!({"format":"BOGUS","path":std::env::temp_dir().join("nope").display().to_string()})
            )
            .ok
        );
    }

    #[test]
    fn export_settings_require_a_format() {
        let mut ws = Workspace::demo();
        assert!(!ws.execute("export.settings.get", json!({})).ok);
        assert!(!ws.execute("export.settings.update", json!({})).ok);
    }

    #[test]
    fn style_get_and_update_round_trip() {
        let mut ws = Workspace::demo();
        let before = ws.execute("style.get", json!({}));
        assert!(before.ok);
        assert_eq!(before.data["prose_font"], "Liberation Serif");

        let updated = ws.execute(
            "style.update",
            json!({
                "prose_font":"Georgia",
                "prose_size":12.5,
                "prose_leading":1.6,
                "script_font":"Courier Prime",
                "script_size":11.5,
                "script_leading":1.1
            }),
        );
        assert!(updated.ok, "{}", updated.message);
        assert_eq!(updated.data["prose_font"], "Georgia");
        assert_eq!(updated.data["script_size"].as_f64().unwrap(), 11.5);
        assert_eq!(ws.project.style.prose_size, 12.5);

        let again = ws.execute("style.get", json!({}));
        assert_eq!(again.data, updated.data);
    }

    #[test]
    fn folder_roundtrip_preserves_project() {
        let ws = Workspace::demo();
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("api-folder"));
        storage::save_folder(&ws.project, &dir).expect("save folder");
        assert!(dir.join("project.json").is_file());
        assert!(dir.join("assets").is_dir());
        let reloaded = storage::load_folder(&dir).expect("load folder");
        assert_eq!(
            serde_json::to_string(&ws.project).unwrap(),
            serde_json::to_string(&reloaded).unwrap()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn workspace_saves_and_opens_a_folder() {
        let mut ws = Workspace::demo();
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("api-ws-folder"));
        ws.save(Some(dir.clone())).expect("save workspace");
        assert!(dir.is_dir());
        let reopened = Workspace::open(&dir).expect("open workspace");
        assert_eq!(
            serde_json::to_string(&ws.project).unwrap(),
            serde_json::to_string(&reopened.project).unwrap()
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
