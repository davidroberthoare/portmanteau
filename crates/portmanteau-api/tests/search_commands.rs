//! The `search.find` and `search.replace` commands.

use portmanteau_api::Workspace;
use serde_json::json;

fn workspace() -> Workspace {
    Workspace::demo()
}

#[test]
fn find_reports_places_and_honours_options_and_groups() {
    let mut ws = workspace();
    let all = ws.execute("search.find", json!({"query": "harbour"}));
    assert!(all.ok, "{}", all.message);
    let total = all.data["total"].as_u64().unwrap();
    assert!(total > 0);
    let first = &all.data["hits"][0];
    assert!(first["excerpt"].as_str().unwrap().to_lowercase().contains("harbour"));
    assert!(first["paragraph"].is_u64());

    let exact = ws.execute("search.find", json!({"query": "harbour", "case_sensitive": true, "whole_word": true}));
    assert!(exact.data["total"].as_u64().unwrap() <= total);

    let research = ws.execute("search.find", json!({"query": "harbour", "groups": ["Research"]}));
    assert!(research.data["hits"].as_array().unwrap().iter().all(|h| h["group"] == "Research"));

    let bad = ws.execute("search.find", json!({"query": "(", "regex": true}));
    assert!(!bad.ok && bad.message.contains("valid"), "{}", bad.message);
    assert!(!ws.execute("search.find", json!({"query": "x", "groups": ["Nowhere"]})).ok);
}

#[test]
fn replace_changes_the_text_marks_the_project_dirty_and_snapshots() {
    let mut ws = workspace();
    let before = ws.execute("search.find", json!({"query": "Mara Venn", "groups": ["Manuscript"], "asides": false}));
    let n = before.data["total"].as_u64().unwrap();
    assert!(n > 0);
    ws.dirty = false;
    let done = ws.execute("search.replace", json!({"query": "Mara Venn", "replacement": "Mara Vane", "groups": ["Manuscript"], "asides": false}));
    assert!(done.ok, "{}", done.message);
    assert_eq!(done.data["replaced"].as_u64().unwrap(), n);
    assert!(ws.dirty);
    let after = ws.execute("search.find", json!({"query": "Mara Venn", "groups": ["Manuscript"], "asides": false}));
    assert_eq!(after.data["total"], 0);
    assert!(ws.project.documents.iter().any(|d| d.snapshots.iter().any(|s| s.name == "Before replace")));
    assert!(!ws.execute("search.replace", json!({"query": "a"})).ok, "a replacement is required");
}
