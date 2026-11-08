//! MCP (Model Context Protocol) server for Scriptorium.
//!
//! The desktop API is exposed as MCP tools over newline-delimited JSON-RPC on
//! stdin/stdout. That makes the whole writing studio scriptable by an agent:
//! read the binder, write a chapter, snapshot it, export a screenplay.

use msc_api::{ApiResponse, Workspace};
use serde_json::{json, Value};

pub fn tool_definitions() -> Value {
    let t = |name: &str, description: &str, schema: Value| {
        json!({"name": name, "description": description, "inputSchema": schema})
    };
    json!([
        t("project_info", "Summarise the open writing project: title, author, word count, document and version counts.", json!({"type":"object","properties":{}})),
        t("project_create", "Create a new project.", json!({"type":"object","properties":{"title":{"type":"string"},"kind":{"type":"string","enum":["Novel","Film","Television","Theatre","Mixed"]}},"required":["title"]})),
        t("project_save", "Save the project to disk (optionally to a new path).", json!({"type":"object","properties":{"path":{"type":"string"}}})),
        t("list_binder", "List every document in the binder with parent, kind, medium, words, status and export flag.", json!({"type":"object","properties":{}})),
        t("read_document", "Read one document, including its text, script blocks, synopsis, notes and snapshots.", json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]})),
        t("create_document", "Create a document, chapter, scene or research note.", json!({"type":"object","properties":{"parent":{"type":"string"},"title":{"type":"string"},"kind":{"type":"string"},"medium":{"type":"string"}},"required":["title"]})),
        t("update_document", "Update a document's title, prose text, synopsis, notes, status, label or export flag.", json!({"type":"object","properties":{"id":{"type":"string"},"title":{"type":"string"},"text":{"type":"string"},"synopsis":{"type":"string"},"notes":{"type":"string"},"status":{"type":"string"},"label":{"type":"string"},"include_in_export":{"type":"boolean"}},"required":["id"]})),
        t("delete_document", "Delete a document and everything beneath it.", json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]})),
        t("snapshot_document", "Save a named version of a document before a risky edit.", json!({"type":"object","properties":{"id":{"type":"string"},"name":{"type":"string"},"note":{"type":"string"}},"required":["id","name"]})),
        t("list_snapshots", "List all document snapshots across the project.", json!({"type":"object","properties":{}})),
        t("restore_snapshot", "Restore a document to a saved snapshot.", json!({"type":"object","properties":{"document_id":{"type":"string"},"id":{"type":"string"}},"required":["document_id","id"]})),
        t("checkpoint", "Save a full-project checkpoint before a milestone or export.", json!({"type":"object","properties":{"label":{"type":"string"}},"required":["label"]})),
        t("search_project", "Search titles and manuscript text.", json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})),
        t("project_stats", "Word counts, page estimates, scenes, chapters and session history.", json!({"type":"object","properties":{}})),
        t("parse_script", "Parse plain text into typed screenplay blocks and return Fountain.", json!({"type":"object","properties":{"text":{"type":"string"},"medium":{"type":"string"}},"required":["text"]})),
        t("compile_preview", "Preview what a compile would contain.", json!({"type":"object","properties":{}})),
        t("list_export_formats", "List supported output formats.", json!({"type":"object","properties":{}})),
        t("export_manuscript", "Export the manuscript to PDF, EPUB, DOCX, FDX, Fountain, HTML, Markdown, RTF, text or a project bundle.", json!({"type":"object","properties":{"format":{"type":"string"},"path":{"type":"string"},"paper":{"type":"string"},"include_toc":{"type":"boolean"},"include_title_page":{"type":"boolean"},"include_scene_numbers":{"type":"boolean"}},"required":["format"]})),
        t("add_character", "Add a character sheet.", json!({"type":"object","properties":{"name":{"type":"string"},"role":{"type":"string"},"description":{"type":"string"}},"required":["name"]})),
        t("add_research_note", "Add a research note and its binder entry.", json!({"type":"object","properties":{"title":{"type":"string"},"body":{"type":"string"},"kind":{"type":"string"},"source":{"type":"string"}},"required":["title"]})),
        t("add_idea", "Add a card to the idea board.", json!({"type":"object","properties":{"title":{"type":"string"},"body":{"type":"string"},"kind":{"type":"string"}},"required":["title"]})),
        t("list_commands", "List every raw API command (for building new clients).", json!({"type":"object","properties":{}}))
    ])
}

pub fn call_tool(ws: &mut Workspace, name: &str, args: &Value) -> ApiResponse {
    let (command, mut payload) = match name {
        "project_info" => ("project.info", json!({})),
        "project_create" => ("project.new", args.clone()),
        "project_save" => ("project.save", args.clone()),
        "list_binder" => ("document.list", json!({})),
        "read_document" => ("document.get", json!({"id": arg(args, "id")})),
        "create_document" => ("document.create", args.clone()),
        "update_document" => ("document.update", args.clone()),
        "delete_document" => ("document.delete", json!({"id": arg(args, "id")})),
        "snapshot_document" => ("document.snapshot", args.clone()),
        "list_snapshots" => ("project.snapshots", json!({})),
        "restore_snapshot" => ("snapshot.restore", args.clone()),
        "checkpoint" => ("project.checkpoint", args.clone()),
        "search_project" => ("search", json!({"query": arg(args, "query")})),
        "project_stats" => ("stats", json!({})),
        "parse_script" => ("script.parse", args.clone()),
        "compile_preview" => ("export.preview", json!({})),
        "list_export_formats" => ("export.formats", json!({})),
        "export_manuscript" => ("export.run", args.clone()),
        "add_character" => ("character.add", args.clone()),
        "add_research_note" => ("research.add", args.clone()),
        "add_idea" => ("idea.add", args.clone()),
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
            "serverInfo": {"name": "scriptorium", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Scriptorium is a writing studio for novels, film, TV and theatre. Read the project, write documents, snapshot versions and compile to professional formats."
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
            {"uri": "scriptorium://project", "name": "Open project", "description": "Project summary", "mimeType": "application/json"},
            {"uri": "scriptorium://binder", "name": "Binder", "description": "Every document in the project", "mimeType": "application/json"}
        ]})),
        "resources/read" => {
            let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            let res = match uri {
                "scriptorium://project" => ws.execute("project.info", json!({})),
                "scriptorium://binder" => ws.execute("document.list", json!({})),
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
            eprintln!("scriptorium-mcp: {e}; starting the demonstration project");
            Workspace::demo()
        }),
        None => Workspace::demo(),
    };
    eprintln!("scriptorium-mcp {} ready on stdio", env!("CARGO_PKG_VERSION"));
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
        assert_eq!(res["result"]["serverInfo"]["name"], "scriptorium");
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
