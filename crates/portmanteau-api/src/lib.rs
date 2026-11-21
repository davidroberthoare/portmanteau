//! The command bus.
//!
//! The UI, the MCP server and the CLI all speak the same JSON commands. The
//! desktop layer is a view onto this API rather than a second implementation,
//! which keeps agent testing honest.

use std::path::{Path, PathBuf};

pub mod storage;

use portmanteau_core::*;
use portmanteau_import::{detect as import_detect, ImportFormat, ImportOptions, Split};
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
        let mut project = storage::load_folder(path)?;
        project.ensure_style_sheets();
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
                if let Some(v) = payload.get("contact").and_then(|v| v.as_str()) {
                    self.project.contact = v.into();
                }
                if let Some(v) = payload.get("draft_label").and_then(|v| v.as_str()) {
                    self.project.draft_label = v.into();
                }
                if let Some(v) = payload.get("draft_date").and_then(|v| v.as_str()) {
                    self.project.draft_date = v.into();
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
            "document.import" => {
                let inline_text = str_arg(&payload, "text");
                let path = str_arg(&payload, "path");
                let bytes = match (&inline_text, &path) {
                    (Some(text), _) => text.clone().into_bytes(),
                    (None, Some(path)) => match std::fs::read(path) {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            return ApiResponse::err(
                                command,
                                format!("Could not read {path}: {error}"),
                            )
                        }
                    },
                    (None, None) => {
                        return ApiResponse::err(
                            command,
                            "A path or inline text is required to import",
                        )
                    }
                };
                let format = match str_arg(&payload, "format") {
                    Some(name) => match ImportFormat::from_name(&name) {
                        Some(format) => format,
                        None => {
                            return ApiResponse::err(
                                command,
                                format!("Unsupported import format {name:?}; use Fountain, FDX, DOCX or plain text"),
                            )
                        }
                    },
                    None => match import_detect(path.as_deref().map(Path::new), &bytes) {
                        Some(format) => format,
                        None => {
                            return ApiResponse::err(
                                command,
                                "Could not tell the file format; name it with \"format\" (fountain, fdx, docx or text)",
                            )
                        }
                    },
                };
                let options = ImportOptions {
                    medium: payload.get("medium").and_then(|v| v.as_str()).and_then(parse_medium),
                    split: match payload.get("split") {
                        Some(v) if v.as_bool() == Some(false) || v.as_str() == Some("never") => {
                            Split::Never
                        }
                        _ => Split::Auto,
                    },
                };
                let imported = match portmanteau_import::import(format, &bytes, &options) {
                    Ok(imported) => imported,
                    Err(error) => return ApiResponse::err(command, error),
                };

                // The title a document falls back on: what the caller said,
                // then what the file says, then the file's name.
                let base = str_arg(&payload, "title")
                    .or_else(|| imported.title.clone())
                    .or_else(|| {
                        path.as_deref()
                            .and_then(|path| Path::new(path).file_stem())
                            .and_then(|stem| stem.to_str())
                            .map(|stem| stem.to_owned())
                    })
                    .unwrap_or_else(|| {
                        if imported.medium.is_script() {
                            "Imported screenplay".into()
                        } else {
                            "Imported manuscript".into()
                        }
                    });
                let parent = str_arg(&payload, "parent")
                    .or_else(|| self.project.folder_id("Draft"));
                let kind_override = payload
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .and_then(parse_doc_kind);
                let many = imported.documents.len() > 1;
                let mut created = Vec::new();
                for (index, part) in imported.documents.iter().enumerate() {
                    let title = if !part.title.is_empty() {
                        part.title.clone()
                    } else if many {
                        format!("{base} · {}", index + 1)
                    } else {
                        base.clone()
                    };
                    let id = self.project.add_document(
                        parent.clone(),
                        &title,
                        kind_override.unwrap_or(part.kind),
                        part.medium,
                    );
                    if let Some(doc) = self.project.document_mut(&id) {
                        doc.content = part.content.clone();
                        if !part.synopsis.is_empty() {
                            doc.synopsis.set_plain_text(part.synopsis.clone(), "body");
                        }
                        doc.touch();
                    }
                    created.push(json!({
                        "id": id,
                        "title": title,
                        "paragraphs": part.content.paragraphs.len(),
                    }));
                }
                self.dirty = true;

                let paragraphs = imported.paragraph_count();
                let mut message = format!(
                    "Imported {} document{} ({paragraphs} paragraphs) from {}",
                    created.len(),
                    if created.len() == 1 { "" } else { "s" },
                    format.label()
                );
                if !imported.warnings.is_empty() {
                    message.push_str(" — ");
                    message.push_str(&imported.warnings.join("; "));
                }
                ApiResponse::ok(
                    command,
                    message,
                    json!({
                        "id": created[0]["id"],
                        "documents": created,
                        "paragraphs": paragraphs,
                        "format": format.label(),
                        "title": imported.title,
                        "author": imported.author,
                        "contact": imported.contact,
                        "draft_date": imported.draft_date,
                        "warnings": imported.warnings,
                    }),
                )
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
                    doc.set_synopsis_text(v);
                }
                if let Some(v) = payload.get("notes").and_then(|v| v.as_str()) {
                    doc.set_notes_text(v);
                }
                if let Some(v) = payload.get("text").and_then(|v| v.as_str()) {
                    if doc.medium.is_script() {
                        doc.set_script_blocks(script::parse_loose(v));
                    } else {
                        doc.set_plain_text(v);
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
            "item.add" => {
                let kind = payload
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .and_then(parse_item_kind)
                    .unwrap_or(ItemKind::Character);
                let title = str_arg(&payload, "title").unwrap_or_else(|| "New item".into());
                let mut item = DatabaseItem::new(kind, title);
                if let Some(v) = payload.get("notes").and_then(|v| v.as_str()) {
                    item.notes.set_plain_text(v, "body");
                }
                if let Some(Value::Array(tags)) = payload.get("tags") {
                    item.tags = string_list(tags);
                }
                let id = item.id.clone();
                self.project.items.push(item);
                self.dirty = true;
                ApiResponse::ok(command, "Item added", json!({"id": id}))
            }
            "item.update" => {
                let Some(id) = str_arg(&payload, "id") else {
                    return ApiResponse::err(command, "An item id is required");
                };
                let Some(item) = self.project.items.iter_mut().find(|i| i.id == id) else {
                    return ApiResponse::err(command, "Item not found");
                };
                if let Some(kind) = payload
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .and_then(parse_item_kind)
                {
                    item.kind = kind;
                }
                if let Some(v) = payload.get("title").and_then(|v| v.as_str()) {
                    item.title = v.into();
                }
                if let Some(v) = payload.get("notes").and_then(|v| v.as_str()) {
                    item.notes.set_plain_text(v, "body");
                }
                if let Some(Value::Array(tags)) = payload.get("tags") {
                    item.tags = string_list(tags);
                }
                if let Some(Value::Array(related)) = payload.get("related") {
                    item.related = string_list(related);
                }
                if let Some(v) = payload.get("color").and_then(|v| v.as_u64()) {
                    item.color = v as u8;
                }
                self.dirty = true;
                ApiResponse::ok(command, "Item updated", json!({"id": id}))
            }
            "research.add" => {
                let title = str_arg(&payload, "title").unwrap_or_else(|| "New note".into());
                let body = str_arg(&payload, "body").unwrap_or_default();
                let kind = str_arg(&payload, "kind").unwrap_or_else(|| "Note".into());
                let source = str_arg(&payload, "source").unwrap_or_default();
                let item = ResearchItem {
                    id: new_id("res"),
                    title: title.clone(),
                    body: RichDocument::from_plain_text(&body, "body"),
                    kind,
                    source,
                    created: now_stamp(),
                    attachments: Vec::new(),
                    html: String::new(),
                };
                let id = item.id.clone();
                self.project.research.push(item);
                if let Some(folder) = self.project.folder_id("Research") {
                    let doc_id =
                        self.project
                            .add_document(Some(folder), &title, DocKind::Research, Medium::Note);
                    if let Some(doc) = self.project.document_mut(&doc_id) {
                        doc.set_plain_text(body);
                        doc.include_in_export = false;
                    }
                }
                self.dirty = true;
                ApiResponse::ok(command, "Research note added", json!({"id": id}))
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
                let kind = parse_export_kind(
                    payload.get("format").and_then(|v| v.as_str()).unwrap_or(""),
                )
                .unwrap_or(self.project.export.kind);
                let mut settings = self.project.settings_for(kind);
                if let Err(e) = apply_settings_overrides(&mut settings, &payload) {
                    return ApiResponse::err(command, e);
                }
                let plan = self.project.compile_plan(&settings);
                let sections: Vec<Value> = plan
                    .included
                    .iter()
                    .filter(|d| !matches!(d.kind, DocKind::FrontMatter | DocKind::BackMatter))
                    .map(|d| {
                        json!({
                            "id": d.id,
                            "title": d.title,
                            "medium": d.medium,
                            "words": d.words(),
                            "pages": d.pages(),
                        })
                    })
                    .collect();
                let skipped: Vec<Value> = plan
                    .skipped
                    .iter()
                    .map(|s| json!({"id": s.doc.id, "title": s.doc.title, "reason": s.reason}))
                    .collect();
                let files = if settings.each_separately && plan.kind.is_script() {
                    self.project.compile_groups(&plan).len()
                } else {
                    1
                };
                ApiResponse::ok(
                    command,
                    "Compile preview",
                    json!({
                        "title": self.project.title,
                        "author": self.project.author,
                        "compile_type": plan.kind.key(),
                        "sections": sections,
                        "included": sections.len(),
                        "skipped": skipped,
                        "files": files,
                        "words": plan.included.iter().map(|d| d.words()).sum::<u32>(),
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
                serde_json::to_value(&self.project.styles).unwrap_or(Value::Null),
            ),
            "style.update" => {
                // Work on a clone so a rejected numeric value cannot leave the
                // style half-updated.
                let mut styles = self.project.styles.clone();
                if let Err(e) = apply_style_overrides(&mut styles, &payload) {
                    return ApiResponse::err(command, e);
                }
                self.project.styles = styles;
                self.dirty = true;
                ApiResponse::ok(
                    command,
                    "Project style updated",
                    serde_json::to_value(&self.project.styles).unwrap_or(Value::Null),
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
            "genre": self.project.genre,
            "logline": self.project.logline,
            "contact": self.project.contact,
            "draft_label": self.project.draft_label,
            "draft_date": self.project.draft_date,
            "kind": self.project.kind,
            "words": self.project.words(),
            "documents": self.project.documents.len(),
            "items": self.project.items.len(),
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
            "items": self.project.items.len(),
            "characters": self.count_items(ItemKind::Character),
            "places": self.count_items(ItemKind::Place),
            "events": self.count_items(ItemKind::Event),
            "props": self.count_items(ItemKind::Prop),
            "custom": self.count_items(ItemKind::Custom),
            "research": self.project.research.len(),
            "locations": self.project.locations().len(),
            "today": self.project.today_words(),
            "target": self.project.daily_target,
            "session": self.project.session_log,
        })
    }

    fn count_items(&self, kind: ItemKind) -> usize {
        self.project.items.iter().filter(|i| i.kind == kind).count()
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

/// Merge a `{"<element>":{...}}` JSON object into an element-style set.
/// Unknown element names and unknown subfields are ignored, so a newer client
/// can send extra fields safely. Numeric values go through `parse_finite_f32`.
fn apply_style_overrides(styles: &mut ElementStyles, payload: &Value) -> Result<(), String> {
    let Some(object) = payload.as_object() else {
        return Ok(());
    };
    for (name, fields) in object {
        let Some(element) = styles.element_mut(name) else {
            continue;
        };
        if let Some(v) = fields.get("font").and_then(|v| v.as_str()) {
            element.font = v.into();
        }
        if let Some(v) = fields.get("uppercase").and_then(|v| v.as_bool()) {
            element.uppercase = v;
        }
        if let Some(v) = fields.get("align").and_then(|v| v.as_str()) {
            element.align = Alignment::all()
                .into_iter()
                .find(|a| format!("{a:?}").eq_ignore_ascii_case(v) || a.label().eq_ignore_ascii_case(v))
                .ok_or_else(|| {
                    format!("align must be Left, Center, Right or Justify (got {v:?})")
                })?;
        }
        for (field, value, min, max) in [
            ("size", fields.get("size"), 4.0, 72.0),
            ("leading", fields.get("leading"), 0.5, 3.0),
            ("indent", fields.get("indent"), 0.0, 800.0),
            ("first_line_indent", fields.get("first_line_indent"), 0.0, 800.0),
            ("right_indent", fields.get("right_indent"), 0.0, 800.0),
            ("before", fields.get("before"), 0.0, 800.0),
            ("after", fields.get("after"), 0.0, 800.0),
        ] {
            let Some(value) = value else { continue };
            let parsed = parse_finite_f32(value, field, min, max)?;
            match field {
                "size" => element.size = parsed,
                "leading" => element.leading = parsed,
                "indent" => element.indent = parsed,
                "first_line_indent" => element.first_line_indent = parsed,
                "right_indent" => element.right_indent = parsed,
                "before" => element.before = parsed,
                "after" => element.after = parsed,
                _ => {}
            }
        }
    }
    Ok(())
}

/// Merge the settings fields a caller may supply on top of a resolved set.
/// Unknown keys are ignored, so a newer client can send extra fields safely.
/// `null` or `"auto"` returns to automatic margins; an object sets any of
/// `top`, `bottom`, `left`, `right` (points), starting from the current values.
fn parse_margins(
    value: &Value,
    current: Option<PageMargins>,
) -> Result<Option<PageMargins>, String> {
    if value.is_null() || value.as_str().is_some_and(|s| s.trim().eq_ignore_ascii_case("auto")) {
        return Ok(None);
    }
    let Some(map) = value.as_object() else {
        return Err("margins must be an object like {\"top\":72,\"left\":90} or \"auto\"".into());
    };
    let mut margins = current.unwrap_or_default();
    for (key, slot) in [
        ("top", &mut margins.top),
        ("bottom", &mut margins.bottom),
        ("left", &mut margins.left),
        ("right", &mut margins.right),
    ] {
        if let Some(v) = map.get(key) {
            *slot = parse_finite_f32(v, key, 0.0, 300.0)?;
        }
    }
    Ok(Some(margins))
}

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
        ("include_title_page", &mut settings.include_title_page),
        ("include_toc", &mut settings.include_toc),
        ("include_chapter_numbers", &mut settings.include_chapter_numbers),
        ("include_scene_numbers", &mut settings.include_scene_numbers),
        ("include_synopsis", &mut settings.include_synopsis),
        ("include_notes", &mut settings.include_notes),
    ] {
        if let Some(v) = payload.get(key).and_then(|v| v.as_bool()) {
            *field = v;
        }
    }
    if let Some(v) = payload.get("compile_type") {
        settings.compile_type = match v.as_str().map(str::trim) {
            None | Some("") | Some("auto") => None,
            Some(raw) => Some(
                CompileType::from_key(raw)
                    .ok_or_else(|| format!("Unknown compile_type: {raw} (prose, screenplay, tv, stage or auto)"))?,
            ),
        };
    }
    if let Some(v) = payload.get("scope") {
        settings.scope = match v.as_str() {
            Some(key) => CompileScope::from_key(key),
            None => CompileScope::Draft,
        };
    }
    if let Some(v) = payload.get("each_separately").and_then(|v| v.as_bool()) {
        settings.each_separately = v;
    }
    if let Some(v) = payload.get("margins") {
        settings.margins = parse_margins(v, settings.margins)?;
    }
    merge_header_footer(&mut settings.header_footer, payload)?;
    if let Some(v) = payload.get("styles") {
        apply_style_overrides(&mut settings.styles, v)?;
    }
    Ok(())
}

fn parse_alignment(value: &str) -> Option<Alignment> {
    Alignment::all().into_iter().find(|a| {
        format!("{a:?}").eq_ignore_ascii_case(value) || a.label().eq_ignore_ascii_case(value)
    })
}

/// Merge a nested `{"header_footer":{...}}` object. Unknown subfields are
/// ignored; invalid values are rejected before the caller commits the clone.
fn merge_header_footer(header_footer: &mut HeaderFooter, payload: &Value) -> Result<(), String> {
    let Some(fields) = payload.get("header_footer") else {
        return Ok(());
    };
    let Some(object) = fields.as_object() else {
        return Ok(());
    };
    for (key, field) in [
        ("page_numbers", &mut header_footer.page_numbers),
        ("number_on_first_page", &mut header_footer.number_on_first_page),
        ("running_header", &mut header_footer.running_header),
    ] {
        if let Some(v) = object.get(key).and_then(|v| v.as_bool()) {
            *field = v;
        }
    }
    if let Some(v) = object.get("number_format").and_then(|v| v.as_str()) {
        header_footer.number_format = NumberFormat::all()
            .into_iter()
            .find(|f| format!("{f:?}").eq_ignore_ascii_case(v) || f.label().eq_ignore_ascii_case(v))
            .ok_or_else(|| {
                format!("number_format must be Arabic, RomanLower or RomanUpper (got {v:?})")
            })?;
    }
    if let Some(v) = object.get("number_location").and_then(|v| v.as_str()) {
        header_footer.number_location = HfLocation::all()
            .into_iter()
            .find(|l| format!("{l:?}").eq_ignore_ascii_case(v) || l.label().eq_ignore_ascii_case(v))
            .ok_or_else(|| format!("number_location must be Header or Footer (got {v:?})"))?;
    }
    if let Some(v) = object.get("number_align").and_then(|v| v.as_str()) {
        header_footer.number_align =
            parse_alignment(v).ok_or_else(|| format!("number_align invalid (got {v:?})"))?;
    }
    if let Some(v) = object.get("header_align").and_then(|v| v.as_str()) {
        header_footer.header_align =
            parse_alignment(v).ok_or_else(|| format!("header_align invalid (got {v:?})"))?;
    }
    for (key, field) in [
        ("number_spacing", &mut header_footer.number_spacing),
        ("header_spacing", &mut header_footer.header_spacing),
    ] {
        if let Some(v) = object.get(key) {
            *field = parse_finite_f32(v, key, 0.0, 300.0)?;
        }
    }
    if let Some(v) = object.get("start_number") {
        let n = v
            .as_u64()
            .ok_or_else(|| "start_number must be a non-negative integer".to_string())?;
        header_footer.start_number = n.min(9999) as u32;
    }
    if let Some(v) = object.get("header_text").and_then(|v| v.as_str()) {
        header_footer.header_text = v.into();
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

fn parse_item_kind(s: &str) -> Option<ItemKind> {
    ItemKind::all()
        .into_iter()
        .find(|k| format!("{k:?}").eq_ignore_ascii_case(s) || k.label().eq_ignore_ascii_case(s))
}

fn string_list(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect()
}

/// One-line catalogue used by the UI's agent panel and by MCP tool discovery.
pub fn command_catalogue() -> Vec<Value> {
    let entries = [
        ("project.info", "Summarise the open project", "{}"),
        ("project.update", "Change the project's details: title, subtitle, author, genre, logline, contact (one line each), draft_label, draft_date (blank = today) and daily_target", r#"{"author":"Ada Marlowe","contact":"Ada Marlowe\nada@example.com","draft_date":"14 March 2027"}"#),
        ("project.new", "Create a project", r#"{"title":"...","kind":"Novel"}"#),
        ("project.open", "Open a project folder package", r#"{"path":"..."}"#),
        ("project.save", "Save the project to a folder package", r#"{"path":"..."}"#),
        ("project.checkpoint", "Save a full version checkpoint", r#"{"label":"..."}"#),
        ("project.restore_checkpoint", "Restore a checkpoint", r#"{"id":"cp-..."}"#),
        ("project.snapshots", "List document snapshots", "{}"),
        ("document.list", "List every binder document", "{}"),
        ("document.get", "Read one document", r#"{"id":"doc-..."}"#),
        ("document.create", "Add a document", r#"{"parent":"doc-...","title":"...","kind":"Chapter","medium":"Prose"}"#),
        ("document.import", "Import a Fountain, FDX, DOCX or plain-text file as one or more documents (chapters split at headings, sections or chapter lines); returns warnings for anything not carried across", r#"{"path":"manuscript.docx","split":true}"#),
        ("document.update", "Change title, text, synopsis or metadata", r#"{"id":"doc-...","text":"..."}"#),
        ("document.delete", "Delete a document and its children", r#"{"id":"doc-..."}"#),
        ("document.move", "Move a document up, down or into a folder", r#"{"id":"doc-...","direction":"up"}"#),
        ("document.snapshot", "Snapshot a document", r#"{"id":"doc-...","name":"..."}"#),
        ("snapshot.restore", "Restore a document snapshot", r#"{"document_id":"...","id":"snap-..."}"#),
        ("snapshot.diff", "Compare a snapshot with the current document", r#"{"document_id":"...","id":"snap-..."}"#),
        ("item.add", "Add a story-database item", r#"{"kind":"Character","title":"...","notes":"...","tags":["..."]}"#),
        ("item.update", "Change a story-database item", r#"{"id":"item-...","title":"...","tags":["..."]}"#),
        ("research.add", "Add a research note", r#"{"title":"...","body":"..."}"#),
        ("search", "Search the whole manuscript", r#"{"query":"..."}"#),
        ("stats", "Word, page and session statistics", "{}"),
        ("script.parse", "Parse plain text into script blocks", r#"{"text":"INT. ..."}"#),
        ("export.preview", "Preview the compiled manuscript: included and skipped documents for one manuscript type", r#"{"compile_type":"screenplay","scope":"draft"}"#),
        ("export.formats", "List supported output formats", "{}"),
        ("export.settings.get", "Read the settings for one output format", r#"{"format":"PDF"}"#),
        ("export.settings.update", "Change the settings for one output format (paper, margins in points or \"auto\", header/footer, element styles, include flags)", r#"{"format":"PDF","margins":{"left":90,"right":54},"font_size":11.5}"#),
        ("style.get", "Read the project's per-element styles", "{}"),
        ("style.update", "Change one or more per-element styles", r#"{"body":{"size":12,"font":"Georgia"},"dialogue":{"indent":80}}"#),
        ("export.run", "Export the manuscript", r#"{"format":"PDF","path":"exports","compile_type":"tv","scope":"draft","each_separately":false}"#),
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
    fn importing_chapters_creates_one_document_each_in_order_under_draft() {
        let mut ws = Workspace::demo();
        let draft = ws.project.folder_id("Draft").expect("draft folder");
        let before = ws.project.documents.len();
        let text = "Prologue\n\nIt began.\n\nChapter 1\n\nOut it went.\n\nChapter 2\n\nAnd back.\n";
        let res = ws.execute("document.import", json!({"format":"text","title":"My Book","text":text}));
        assert!(res.ok, "{}", res.message);
        assert_eq!(ws.project.documents.len(), before + 3);
        let docs = res.data["documents"].as_array().expect("document list");
        let titles: Vec<&str> = docs.iter().map(|d| d["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Prologue", "Chapter 1", "Chapter 2"]);
        let orders: Vec<u32> = docs
            .iter()
            .map(|d| ws.project.document(d["id"].as_str().unwrap()).unwrap().order as u32)
            .collect();
        assert!(orders.windows(2).all(|w| w[0] < w[1]), "binder order follows the file: {orders:?}");
        assert_eq!(
            ws.project.document(docs[0]["id"].as_str().unwrap()).unwrap().parent.as_deref(),
            Some(draft.as_str())
        );
        assert_eq!(res.data["id"], docs[0]["id"], "id points at the first document");

        // "split": false keeps it whole, and untitled parts fall back on the title given.
        let res = ws.execute(
            "document.import",
            json!({"format":"text","title":"Whole","text":text,"split":false}),
        );
        assert!(res.ok);
        assert_eq!(res.data["documents"].as_array().unwrap().len(), 1);
        assert_eq!(res.data["documents"][0]["title"], "Whole");
    }

    #[test]
    fn import_reports_what_it_could_not_carry_across() {
        let mut ws = Workspace::demo();
        let fdx = r#"<FinalDraft><Content>
            <Paragraph Type="Action"><Text>Kept.</Text></Paragraph>
            <Paragraph Type="Strange"><Text>Odd one.</Text></Paragraph>
        </Content></FinalDraft>"#;
        let res = ws.execute("document.import", json!({"format":"fdx","text":fdx}));
        assert!(res.ok, "an unknown paragraph type must not fail the import");
        let warnings = res.data["warnings"].as_array().expect("warnings");
        assert!(warnings.iter().any(|w| w.as_str().unwrap().contains("Strange")), "{warnings:?}");
        assert!(res.message.contains("Strange"), "the message carries the warning: {}", res.message);
    }

    #[test]
    fn import_detects_the_format_from_the_file_extension_and_rejects_the_unknown() {
        let mut ws = Workspace::demo();
        let dir = std::env::temp_dir().join(format!("portmanteau-api-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("pilot.fountain");
        std::fs::write(&script, "Title: Pilot\nAuthor: Ada\n\nINT. ROOM - DAY\n\nMARA\nHi.\n").unwrap();
        let res = ws.execute("document.import", json!({"path": script.display().to_string()}));
        assert!(res.ok, "{}", res.message);
        assert_eq!(res.data["title"], "Pilot");
        assert_eq!(res.data["author"], "Ada");
        let id = res.data["id"].as_str().unwrap();
        assert_eq!(ws.project.document(id).unwrap().title, "Pilot", "the file's own title wins over its file name");

        let odd = dir.join("notes.pdf");
        std::fs::write(&odd, "x").unwrap();
        let res = ws.execute("document.import", json!({"path": odd.display().to_string()}));
        assert!(!res.ok && res.message.contains("format"), "{}", res.message);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn document_import_parses_fountain_and_fdx_into_rich_documents() {
        let mut ws = Workspace::demo();
        let fountain = "INT. HARBOR - DAWN\n\nMARA\nWe missed the tide.\n";
        let res = ws.execute(
            "document.import",
            json!({"format":"fountain","title":"Fountain import","text":fountain}),
        );
        assert!(res.ok, "{}", res.message);
        let id = res.data["id"].as_str().expect("import id").to_owned();
        let doc = ws.project.document(&id).expect("imported document exists");
        assert_eq!(doc.medium, Medium::Screenplay);
        assert!(doc.content.plain_text().contains("We missed the tide."));

        let res = ws.execute(
            "document.import",
            json!({"format":"text","title":"Text import","text":"A wrapped prose line\ncontinues here.\n\nSecond paragraph."}),
        );
        assert!(res.ok, "{}", res.message);
        let id = res.data["id"].as_str().expect("import id").to_owned();
        let doc = ws.project.document(&id).expect("imported text document exists");
        assert_eq!(doc.medium, Medium::Prose);
        assert_eq!(doc.content.paragraphs.len(), 2);
        assert_eq!(
            doc.content.paragraphs[0].plain_text(),
            "A wrapped prose line continues here."
        );

        let fdx = r#"<FinalDraft><Content>
            <Paragraph Type="Scene Heading"><Text>EXT. PIER - NIGHT</Text></Paragraph>
            <Paragraph Type="Action"><Text>The lamp swings.</Text></Paragraph>
        </Content></FinalDraft>"#;
        let res = ws.execute(
            "document.import",
            json!({"format":"fdx","title":"FDX import","text":fdx}),
        );
        assert!(res.ok, "{}", res.message);
        let id = res.data["id"].as_str().expect("import id").to_owned();
        let doc = ws.project.document(&id).expect("imported FDX document exists");
        assert_eq!(doc.content.paragraphs[0].style_id, "scene-heading");
        assert_eq!(doc.content.paragraphs[1].plain_text(), "The lamp swings.");
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
                "header_footer":{"page_numbers":false,"number_format":"RomanLower","start_number":3},
                "cover_image":"assets/cover.png"
            }),
        );
        assert!(updated.ok, "{}", updated.message);
        assert_eq!(updated.data["font_size"].as_f64().unwrap(), 15.5);
        assert_eq!(updated.data["header_footer"]["page_numbers"], false);
        assert_eq!(updated.data["header_footer"]["number_format"], "RomanLower");
        assert_eq!(updated.data["header_footer"]["start_number"], 3);

        assert_eq!(ws.project.settings_for(ExportKind::Pdf).font_size, 15.5);
        assert_eq!(ws.project.settings_for(ExportKind::Pdf).line_spacing, 1.9);
        assert!(!ws.project.settings_for(ExportKind::Pdf).header_footer.page_numbers);
        assert_eq!(
            ws.project.settings_for(ExportKind::Pdf).header_footer.number_format,
            portmanteau_core::NumberFormat::RomanLower
        );
        assert_eq!(
            ws.project.settings_for(ExportKind::Pdf).header_footer.start_number,
            3
        );
        assert_eq!(ws.project.settings_for(ExportKind::Pdf).cover_image, "assets/cover.png");

        let epub_after = ws.execute("export.settings.get", json!({"format":"EPUB"}));
        assert_eq!(epub_after.data, epub_before.data);
        assert_eq!(ws.project.settings_for(ExportKind::Epub).font_size, 12.0);
    }

    #[test]
    fn export_settings_styles_are_per_format() {
        let mut ws = Workspace::demo();
        let epub_before = ws.execute("export.settings.get", json!({"format":"EPUB"}));
        assert!(epub_before.ok, "{}", epub_before.message);
        let epub_body_size = epub_before.data["styles"]["body"]["size"]
            .as_f64()
            .unwrap();
        assert_eq!(epub_body_size, 11.0);

        let updated = ws.execute(
            "export.settings.update",
            json!({"format":"PDF","styles":{"body":{"size":15.0}}}),
        );
        assert!(updated.ok, "{}", updated.message);

        let pdf = ws.execute("export.settings.get", json!({"format":"PDF"}));
        assert_eq!(pdf.data["styles"]["body"]["size"].as_f64().unwrap(), 15.0);
        assert_eq!(ws.project.settings_for(ExportKind::Pdf).styles.body.size, 15.0);

        let epub_after = ws.execute("export.settings.get", json!({"format":"EPUB"}));
        assert_eq!(
            epub_after.data["styles"]["body"]["size"].as_f64().unwrap(),
            epub_body_size,
            "editing PDF styles must not leak into EPUB"
        );
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
    fn style_update_accepts_alignment_and_paragraph_indents() {
        let mut ws = Workspace::demo();
        let updated = ws.execute(
            "style.update",
            json!({
                "body":{"align":"justify","first_line_indent":18,"right_indent":6.5}
            }),
        );
        assert!(updated.ok, "{}", updated.message);
        assert_eq!(updated.data["body"]["align"], "Justify");
        assert_eq!(
            updated.data["body"]["first_line_indent"].as_f64().unwrap(),
            18.0
        );
        assert_eq!(
            updated.data["body"]["right_indent"].as_f64().unwrap(),
            6.5
        );
        assert_eq!(ws.project.styles.body.align, Alignment::Justify);

        let bad = ws.execute("style.update", json!({"body":{"align":"Sideways"}}));
        assert!(!bad.ok);
        assert!(bad.message.to_lowercase().contains("align"), "{}", bad.message);
        assert_eq!(
            ws.project.styles.body.align,
            Alignment::Justify,
            "a rejected update must not change the style"
        );
    }

    #[test]
    fn style_get_and_update_round_trip() {
        let mut ws = Workspace::demo();
        let before = ws.execute("style.get", json!({}));
        assert!(before.ok);
        assert_eq!(before.data["body"]["font"], "Liberation Serif");

        let updated = ws.execute(
            "style.update",
            json!({
                "body":{"font":"Georgia","size":12.5,"leading":1.6},
                "character":{"font":"Courier Prime","size":11.5,"leading":1.1,"indent":150.0}
            }),
        );
        assert!(updated.ok, "{}", updated.message);
        assert_eq!(updated.data["body"]["font"], "Georgia");
        assert_eq!(updated.data["body"]["size"].as_f64().unwrap(), 12.5);
        assert_eq!(updated.data["character"]["size"].as_f64().unwrap(), 11.5);
        assert_eq!(updated.data["character"]["indent"].as_f64().unwrap(), 150.0);
        assert_eq!(ws.project.styles.body.size, 12.5);
        assert_eq!(ws.project.styles.character.indent, 150.0);

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
    fn project_details_are_set_by_command_and_survive_a_save() {
        let mut ws = Workspace::demo();
        let res = ws.execute(
            "project.update",
            json!({"contact":"Ada Marlowe\nada@example.com","draft_label":"Revised","draft_date":"1 May 2027"}),
        );
        assert!(res.ok, "{}", res.message);
        let info = ws.execute("project.info", json!({}));
        assert_eq!(info.data["contact"], "Ada Marlowe\nada@example.com");
        let dir = std::env::temp_dir().join(portmanteau_core::new_id("api-details"));
        ws.save(Some(dir.clone())).expect("save");
        let reopened = Workspace::open(&dir).expect("open");
        assert_eq!(reopened.project.contact, "Ada Marlowe\nada@example.com");
        assert_eq!(reopened.project.draft_label, "Revised");
        assert_eq!(reopened.project.draft_date, "1 May 2027");
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
