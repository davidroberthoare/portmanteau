#![recursion_limit = "512"]
//! MCP (Model Context Protocol) server for Portmanteau.
//!
//! The desktop API is exposed as MCP tools over newline-delimited JSON-RPC on
//! stdin/stdout. That makes the whole writing studio scriptable by an agent:
//! read the binder, write a chapter, snapshot it, export a screenplay.

use portmanteau_api::{ApiResponse, Workspace};
use serde_json::{json, Value};

pub fn tool_definitions() -> Value {
    let t = |name: &str, description: &str, schema: Value| {
        json!({"name": name, "description": description, "inputSchema": schema})
    };
    json!([
        t("project_info", "Summarise the open writing project: title, author, word count, document and version counts.", json!({"type":"object","properties":{}})),
        t("set_cover", "Set the book's cover picture (.png or .jpg), or clear it with an empty path. EPUB uses it unless that format has its own cover_image.", json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})),
        t("update_project_details", "Change the project's own details. contact is free text, one line each (name, address, email, phone), printed bottom-left on a script's title page. draft_label is the draft's name (\"Draft\"); draft_date is printed as typed, and blank means the day of export.", json!({"type":"object","properties":{"title":{"type":"string"},"subtitle":{"type":"string"},"author":{"type":"string"},"genre":{"type":"string"},"logline":{"type":"string"},"contact":{"type":"string"},"draft_label":{"type":"string"},"draft_date":{"type":"string"},"daily_target":{"type":"integer"}}})),
        t("project_create", "Create a new project.", json!({"type":"object","properties":{"title":{"type":"string"},"kind":{"type":"string","enum":["Novel","Film","Television","Theatre","Mixed"]}},"required":["title"]})),
        t("project_save", "Save the project to disk as a folder package (or legacy .json).", json!({"type":"object","properties":{"path":{"type":"string"}}})),
        t("project_open", "Open a project folder package or a legacy .json project file.", json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})),
        t("project_pack", "Write the project and its attachments to one .zip file (a backup, or to send to someone).", json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})),
        t("project_open_zip", "Unpack a project .zip into a new folder and open it.", json!({"type":"object","properties":{"path":{"type":"string"},"folder":{"type":"string"}},"required":["path","folder"]})),
        t("list_binder", "List every document in the binder with parent, kind, medium, words, status and export flag.", json!({"type":"object","properties":{}})),
        t("read_document", "Read one document, including its text, script blocks, synopsis, notes and snapshots.", json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]})),
        t("create_document", "Create a document, chapter, scene or research note.", json!({"type":"object","properties":{"parent":{"type":"string"},"title":{"type":"string"},"kind":{"type":"string"},"medium":{"type":"string"}},"required":["title"]})),
        t("import_preview", "Read a Fountain, FDX, DOCX or plain-text file and report what importing it would do, without changing the project: the documents it would create (title, kind, words), the paragraph styles it uses with their default and current targets, the title-page details found, and warnings. Takes the same arguments as import_document.", json!({"type":"object","properties":{"path":{"type":"string"},"text":{"type":"string"},"format":{"type":"string","enum":["fountain","fdx","docx","text"]},"medium":{"type":"string"},"split":{"type":"boolean"},"style_map":{"type":"object","additionalProperties":{"type":"string"}}}})),
        t("import_document", "Import a Fountain, Final Draft (FDX), Word (DOCX) or plain-text file. A long file is split into several documents at its headings, sections or chapter lines. Returns the new document ids and warnings for anything that could not be carried across (images, footnotes, unknown paragraph types).", json!({"type":"object","properties":{"path":{"type":"string"},"text":{"type":"string"},"format":{"type":"string","enum":["fountain","fdx","docx","text"],"description":"Optional when path has a known extension."},"title":{"type":"string"},"parent":{"type":"string"},"medium":{"type":"string"},"split":{"type":"boolean","description":"false keeps the whole file as one document."},"style_map":{"type":"object","additionalProperties":{"type":"string"},"description":"Word and Final Draft only: source paragraph style name to what it becomes (heading:1, title, style:<id>, auto, skip)."},"documents":{"type":"array","description":"Per-document choices from a preview, by index: {include, title, kind, parent}.","items":{"type":"object"}},"details":{"type":"object","description":"Exact title, author, contact and draft_date to set on the project (the review's choices).","additionalProperties":{"type":"string"}},"replace_details":{"type":"boolean","description":"A title page in the file fills the project's blank title, author, contact and draft date. true lets it replace values already set."}}})),
        t("update_document", "Update a document's title, prose text, synopsis, notes, status, label or export flag.", json!({"type":"object","properties":{"id":{"type":"string"},"title":{"type":"string"},"text":{"type":"string"},"synopsis":{"type":"string"},"notes":{"type":"string"},"status":{"type":"string"},"label":{"type":"string"},"include_in_export":{"type":"boolean"}},"required":["id"]})),
        t("update_picture", "Change a picture in a document: size in points (width alone keeps the proportions), align left/center/right, wrap (text runs beside a left or right picture), padding around a wrapped picture, caption, description. `paragraph` is the 0-based paragraph holding the picture.", json!({"type":"object","properties":{"id":{"type":"string"},"paragraph":{"type":"integer"},"width":{"type":"number"},"height":{"type":"number"},"align":{"type":"string","enum":["left","center","right"]},"wrap":{"type":"boolean"},"padding":{"type":"number"},"caption":{"type":"string"},"alt_text":{"type":"string"}},"required":["id","paragraph"]})),
        t("delete_document", "Delete a document and everything beneath it.", json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]})),
        t("move_document", "Reorder a document among its siblings (up/down) or move it into a folder.", json!({"type":"object","properties":{"id":{"type":"string"},"direction":{"type":"string","enum":["up","down"]},"parent":{"type":["string","null"]}},"required":["id"]})),
        t("snapshot_document", "Save a named version of a document before a risky edit.", json!({"type":"object","properties":{"id":{"type":"string"},"name":{"type":"string"},"note":{"type":"string"}},"required":["id","name"]})),
        t("list_snapshots", "List all document snapshots across the project.", json!({"type":"object","properties":{}})),
        t("restore_snapshot", "Restore a document to a saved snapshot.", json!({"type":"object","properties":{"document_id":{"type":"string"},"id":{"type":"string"}},"required":["document_id","id"]})),
        t("compare_snapshot", "Compare a saved document snapshot with the current text; returns line-level changes.", json!({"type":"object","properties":{"document_id":{"type":"string"},"id":{"type":"string"}},"required":["document_id","id"]})),
        t("checkpoint", "Save a full-project checkpoint before a milestone or export.", json!({"type":"object","properties":{"label":{"type":"string"}},"required":["label"]})),
        t("search_project", "Find text in the manuscript, front/back matter, research and database notes. Plain text by default; options for case, whole word and regular expressions; groups narrow it.", json!({"type":"object","properties":{"query":{"type":"string"},"case_sensitive":{"type":"boolean"},"whole_word":{"type":"boolean"},"regex":{"type":"boolean"},"groups":{"type":"array","items":{"type":"string","enum":["Manuscript","Matter","Research","Database"]}},"fields":{"type":"array","items":{"type":"string","enum":["Text","Synopsis","Notes"]},"description":"Which parts to search; omit for all."},"limit":{"type":"integer"}},"required":["query"]})),
        t("replace_text", "Replace matches of a search. Optionally limit to one document or item id. Each changed document is snapshotted first.", json!({"type":"object","properties":{"query":{"type":"string"},"replacement":{"type":"string"},"case_sensitive":{"type":"boolean"},"whole_word":{"type":"boolean"},"regex":{"type":"boolean"},"groups":{"type":"array","items":{"type":"string","enum":["Manuscript","Matter","Research","Database"]}},"fields":{"type":"array","items":{"type":"string","enum":["Text","Synopsis","Notes"]}},"id":{"type":"string"}},"required":["query","replacement"]})),
        t("project_stats", "Word counts, page estimates, scenes, chapters and session history.", json!({"type":"object","properties":{}})),
        t("parse_script", "Parse plain text into typed screenplay blocks and return Fountain.", json!({"type":"object","properties":{"text":{"type":"string"},"medium":{"type":"string"}},"required":["text"]})),
        t("compile_preview", "Preview what a compile would contain: included documents and skipped ones with the reason.", json!({"type":"object","properties":{"compile_type":{"type":"string","enum":["auto","prose","screenplay","tv","stage"],"description":"Which kind of manuscript to compile; auto follows the first included document."},"scope":{"type":"string","description":"draft (whole binder) or a document id (that item and everything beneath it)."},"each_separately":{"type":"boolean","description":"Script types: write one file per script."}}})),
        t("list_export_formats", "List supported output formats.", json!({"type":"object","properties":{}})),
        t("export_manuscript", "Export the manuscript to PDF, EPUB, DOCX, FDX, Fountain, HTML, Markdown, RTF, text or a project bundle.", json!({"type":"object","properties":{"compile_type":{"type":"string","enum":["auto","prose","screenplay","tv","stage"],"description":"Which kind of manuscript to compile; auto follows the first included document."},"scope":{"type":"string","description":"draft (whole binder) or a document id (that item and everything beneath it)."},"each_separately":{"type":"boolean","description":"Script types: write one file per script."},"format":{"type":"string"},"path":{"type":"string"},"paper":{"type":"string"},"include_toc":{"type":"boolean"},"toc_title":{"type":"string","description":"Heading above the table of contents (blank = Contents)."},"toc_page_numbers":{"type":"boolean","description":"PDF: print each entry's page number."},"toc_leaders":{"type":"boolean","description":"PDF: dots between title and page number."},"include_title_page":{"type":"boolean"},"include_scene_numbers":{"type":"boolean"},"header_footer":{"type":"object","description":"Prose books: templates of six slots (left, centre, right in header and footer) with fields {page} {pages} {title} {author} {chapter} {chapter_number}. body.odd applies to all pages unless different_odd_even; front and back can be same, custom or none.","properties":{"page_numbers":{"type":"boolean"},"different_odd_even":{"type":"boolean"},"different_first_page":{"type":"boolean"},"no_header_on_chapter_openers":{"type":"boolean"},"restart_body_numbering":{"type":"boolean"},"number_format":{"type":"string","enum":["Arabic","RomanLower","RomanUpper"]},"start_number":{"type":"integer"},"header_spacing":{"type":"number"},"footer_spacing":{"type":"number"},"size":{"type":"number"},"body":{"type":"object","properties":{"odd":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"even":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"first":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}}}},"front":{"type":"object","properties":{"mode":{"type":"string","enum":["same","custom","none"]},"number_format":{"type":"string","enum":["Arabic","RomanLower","RomanUpper"]},"restart_numbering":{"type":"boolean"},"odd":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"even":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"first":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}}}},"back":{"type":"object","properties":{"mode":{"type":"string","enum":["same","custom","none"]},"number_format":{"type":"string","enum":["Arabic","RomanLower","RomanUpper"]},"restart_numbering":{"type":"boolean"},"odd":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"even":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"first":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}}}}}},"cover_image":{"type":"string","description":"This format's own cover picture; blank uses the project cover."},"include_cover":{"type":"boolean"},"font_size":{"type":"number"},"line_spacing":{"type":"number"}},"required":["format"]})),
        t("get_export_settings", "Read the compile settings for one output format.", json!({"type":"object","properties":{"format":{"type":"string"}},"required":["format"]})),
        t("update_export_settings", "Change the compile settings for one output format without touching the others.", json!({"type":"object","properties":{"compile_type":{"type":"string","enum":["auto","prose","screenplay","tv","stage"],"description":"Which kind of manuscript to compile; auto follows the first included document."},"scope":{"type":"string","description":"draft (whole binder) or a document id (that item and everything beneath it)."},"each_separately":{"type":"boolean","description":"Script types: write one file per script."},"format":{"type":"string"},"paper":{"type":"string","description":"US Letter, A4, A5, 5 × 8 in, 5.25 × 8 in, 5.5 × 8.5 in, 6 × 9 in, B-format (129 × 198 mm) or Custom size."},"paper_width_mm":{"type":"number","description":"With paper_height_mm: a custom page size in millimetres."},"paper_height_mm":{"type":"number"},"mirror_margins":{"type":"boolean","description":"Book margins: margins.left is the inside edge, margins.right the outside edge, swapped on left-hand pages (prose, PDF/DOCX/RTF)."},"chapters_start_right":{"type":"boolean","description":"Prose PDF: every chapter opens on a right-hand page, with a blank left-hand page where needed."},"font":{"type":"string"},"font_size":{"type":"number"},"line_spacing":{"type":"number"},"header_footer":{"type":"object","description":"Prose books: templates of six slots (left, centre, right in header and footer) with fields {page} {pages} {title} {author} {chapter} {chapter_number}. body.odd applies to all pages unless different_odd_even; front and back can be same, custom or none.","properties":{"page_numbers":{"type":"boolean"},"different_odd_even":{"type":"boolean"},"different_first_page":{"type":"boolean"},"no_header_on_chapter_openers":{"type":"boolean"},"restart_body_numbering":{"type":"boolean"},"number_format":{"type":"string","enum":["Arabic","RomanLower","RomanUpper"]},"start_number":{"type":"integer"},"header_spacing":{"type":"number"},"footer_spacing":{"type":"number"},"size":{"type":"number"},"body":{"type":"object","properties":{"odd":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"even":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"first":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}}}},"front":{"type":"object","properties":{"mode":{"type":"string","enum":["same","custom","none"]},"number_format":{"type":"string","enum":["Arabic","RomanLower","RomanUpper"]},"restart_numbering":{"type":"boolean"},"odd":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"even":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"first":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}}}},"back":{"type":"object","properties":{"mode":{"type":"string","enum":["same","custom","none"]},"number_format":{"type":"string","enum":["Arabic","RomanLower","RomanUpper"]},"restart_numbering":{"type":"boolean"},"odd":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"even":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}},"first":{"type":"object","properties":{"header":{"type":"array","items":{"type":"string"},"description":"Three texts: left, centre, right."},"footer":{"type":"array","items":{"type":"string"}}}}}}}},"cover_image":{"type":"string","description":"This format's own cover picture; blank uses the project cover."},"include_cover":{"type":"boolean"},"margins":{"description":"Page margins in points, e.g. {\"top\":72,\"left\":90}; omitted sides keep their value; \"auto\" returns to the automatic margins.","oneOf":[{"type":"string","enum":["auto"]},{"type":"object","properties":{"top":{"type":"number"},"bottom":{"type":"number"},"left":{"type":"number"},"right":{"type":"number"}}}]},"include_title_page":{"type":"boolean"},"include_toc":{"type":"boolean"},"toc_title":{"type":"string","description":"Heading above the table of contents (blank = Contents)."},"toc_page_numbers":{"type":"boolean","description":"PDF: print each entry's page number."},"toc_leaders":{"type":"boolean","description":"PDF: dots between title and page number."},"include_chapter_numbers":{"type":"boolean"},"include_scene_numbers":{"type":"boolean"}},"required":["format"]})),
        t("get_project_style", "Read the project's per-element styles (font, size, leading, indent, spacing).", json!({"type":"object","properties":{}})),
        t("update_project_style", "Change per-element styles, which seed new output formats. Send nested objects, one per element: {\"body\":{\"size\":12,\"font\":\"Georgia\",\"align\":\"Justify\",\"first_line_indent\":18},\"dialogue\":{\"indent\":80}}. align is Left, Center, Right or Justify; indents and spacing are points.", json!({"type":"object","additionalProperties":{"type":"object","properties":{"font":{"type":"string"},"size":{"type":"number"},"leading":{"type":"number"},"indent":{"type":"number"},"first_line_indent":{"type":"number"},"right_indent":{"type":"number"},"before":{"type":"number"},"after":{"type":"number"},"align":{"type":"string","enum":["Left","Center","Right","Justify"]}}}})),
        t("add_item", "Add a story-database item: a character, place, event, prop or custom record.", json!({"type":"object","properties":{"kind":{"type":"string","enum":["Character","Place","Event","Prop","Custom"]},"title":{"type":"string"},"notes":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}}},"required":["title"]})),
        t("add_research_note", "Add a research note and its binder entry.", json!({"type":"object","properties":{"title":{"type":"string"},"body":{"type":"string"},"kind":{"type":"string"},"source":{"type":"string"}},"required":["title"]})),
        t("list_research", "List research notes with their kind, source, word count and attachments.", json!({"type":"object","properties":{}})),
        t("update_research_note", "Change a research note: title, body, kind (Note, Interview, Data, Archive, Image, Place) or source.", json!({"type":"object","properties":{"id":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"},"kind":{"type":"string"},"source":{"type":"string"}},"required":["id"]})),
        t("attach_research_file", "Copy a file into a research note's attachments. The project must have been saved.", json!({"type":"object","properties":{"id":{"type":"string"},"path":{"type":"string"}},"required":["id","path"]})),
        t("list_commands", "List every raw API command (for building new clients).", json!({"type":"object","properties":{}}))
    ])
}

pub fn call_tool(ws: &mut Workspace, name: &str, args: &Value) -> ApiResponse {
    let (command, mut payload) = match name {
        "project_info" => ("project.info", json!({})),
        "update_project_details" => ("project.update", args.clone()),
        "set_cover" => ("project.set_cover", args.clone()),
        "project_create" => ("project.new", args.clone()),
        "project_save" => ("project.save", args.clone()),
        "project_open" => ("project.open", json!({"path": arg(args, "path")})),
        "project_pack" => ("project.pack", json!({"path": arg(args, "path")})),
        "project_open_zip" => ("project.open_zip", json!({"path": arg(args, "path"), "folder": arg(args, "folder")})),
        "list_binder" => ("document.list", json!({})),
        "read_document" => ("document.get", json!({"id": arg(args, "id")})),
        "create_document" => ("document.create", args.clone()),
        "import_preview" => ("document.import_preview", args.clone()),
        "import_document" => ("document.import", args.clone()),
        "update_document" => ("document.update", args.clone()),
        "update_picture" => ("document.picture", args.clone()),
        "delete_document" => ("document.delete", json!({"id": arg(args, "id")})),
        "move_document" => ("document.move", args.clone()),
        "snapshot_document" => ("document.snapshot", args.clone()),
        "list_snapshots" => ("project.snapshots", json!({})),
        "restore_snapshot" => ("snapshot.restore", args.clone()),
        "compare_snapshot" => ("snapshot.diff", args.clone()),
        "checkpoint" => ("project.checkpoint", args.clone()),
        "search_project" => ("search.find", args.clone()),
        "replace_text" => ("search.replace", args.clone()),
        "project_stats" => ("stats", json!({})),
        "parse_script" => ("script.parse", args.clone()),
        "compile_preview" => ("export.preview", args.clone()),
        "list_export_formats" => ("export.formats", json!({})),
        "export_manuscript" => ("export.run", args.clone()),
        "get_export_settings" => ("export.settings.get", json!({"format": arg(args, "format")})),
        "update_export_settings" => ("export.settings.update", args.clone()),
        "get_project_style" => ("style.get", json!({})),
        "update_project_style" => ("style.update", args.clone()),
        "add_item" => ("item.add", args.clone()),
        "add_research_note" => ("research.add", args.clone()),
        "list_research" => ("research.list", json!({})),
        "update_research_note" => ("research.update", args.clone()),
        "attach_research_file" => ("research.attach", args.clone()),
        "list_commands" => ("commands", json!({})),
        other => {
            return ApiResponse {
                ok: false,
                command: other.into(),
                message: format!("Unknown MCP tool: {other}"),
                data: Value::Null,
            }
        }
    };
    if payload.is_null() {
        payload = json!({});
    }
    ws.execute(command, payload)
}

fn arg(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Handle one JSON-RPC message. Returns `None` for notifications.
pub fn handle_message(ws: &mut Workspace, message: &Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = message.get("params").cloned().unwrap_or(json!({}));
    let id = match id {
        Some(id) => id,
        None => return None,
    };
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "portmanteau", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Portmanteau is a writing studio for novels, film, TV and theatre. Read the project, write documents, snapshot versions and compile to professional formats."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tool_definitions()})),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let res = call_tool(ws, name, &args);
            if res.ok {
                Ok(json!({
                    "content": [{"type": "text", "text": serde_json::to_string_pretty(&res.data).unwrap_or_default()}],
                    "structuredContent": {"ok": true, "message": res.message, "data": res.data},
                    "isError": false
                }))
            } else {
                Ok(json!({
                    "content": [{"type": "text", "text": res.message}],
                    "isError": true
                }))
            }
        }
        "resources/list" => Ok(json!({"resources": [
            {"uri": "portmanteau://project", "name": "Open project", "description": "Project summary", "mimeType": "application/json"},
            {"uri": "portmanteau://binder", "name": "Binder", "description": "Every document in the project", "mimeType": "application/json"}
        ]})),
        "resources/read" => {
            let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            let res = match uri {
                "portmanteau://project" => ws.execute("project.info", json!({})),
                "portmanteau://binder" => ws.execute("document.list", json!({})),
                _ => ApiResponse {
                    ok: false,
                    command: "resources/read".into(),
                    message: "Unknown resource".into(),
                    data: Value::Null,
                },
            };
            if res.ok {
                Ok(json!({"contents": [{"uri": uri, "mimeType": "application/json", "text": serde_json::to_string_pretty(&res.data).unwrap_or_default()}]}))
            } else {
                Err((-32602, res.message))
            }
        }
        _ => Err((-32601, format!("Method not found: {method}"))),
    };
    Some(match result {
        Ok(value) => json!({"jsonrpc": "2.0", "id": id, "result": value}),
        Err((code, message)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}),
    })
}

pub fn serve_stdio() {
    use std::io::{BufRead, Write};
    let path = std::env::args()
        .position(|a| a == "--project")
        .and_then(|i| std::env::args().nth(i + 1));
    let mut ws = match path {
        Some(p) => Workspace::open(std::path::Path::new(&p)).unwrap_or_else(|e| {
            eprintln!("portmanteau-mcp: {e}; starting the demonstration project");
            Workspace::demo()
        }),
        None => Workspace::demo(),
    };
    eprintln!("portmanteau-mcp {} ready on stdio", env!("CARGO_PKG_VERSION"));
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":format!("Parse error: {e}")}});
                println!("{err}");
                let _ = std::io::stdout().flush();
                continue;
            }
        };
        if let Some(response) = handle_message(&mut ws, &message) {
            println!("{response}");
            let _ = std::io::stdout().flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_handshake() {
        let mut ws = Workspace::demo();
        let res = handle_message(
            &mut ws,
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        )
        .unwrap();
        assert_eq!(res["result"]["serverInfo"]["name"], "portmanteau");
    }

    #[test]
    fn tools_list_is_non_empty() {
        let mut ws = Workspace::demo();
        let res = handle_message(
            &mut ws,
            &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        )
        .unwrap();
        assert!(res["result"]["tools"].as_array().unwrap().len() > 10);
    }

    #[test]
    fn call_reads_binder() {
        let mut ws = Workspace::demo();
        let res = handle_message(
            &mut ws,
            &json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"project_info","arguments":{}}}),
        )
        .unwrap();
        assert_eq!(res["result"]["isError"], false);
    }
}
