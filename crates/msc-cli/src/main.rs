//! A small command-line client for the same API the desktop app and MCP server
//! use. Useful for smoke tests and for wiring Scriptorium into other tools.
//!
//! Examples:
//!   msc-cli project.info
//!   msc-cli stats
//!   msc-cli export.run '{"format":"EPUB","path":"exports"}'

use msc_api::Workspace;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!(
            "Scriptorium command line\n\nUsage:\n  msc-cli <command> [json-payload] [--project path]\n\nCommands: project.info, document.list, stats, export.run, commands …"
        );
        return;
    }
    let mut ws = if let Some(pos) = args.iter().position(|a| a == "--project") {
        match args.get(pos + 1) {
            Some(path) => Workspace::open(std::path::Path::new(path)).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            }),
            None => {
                eprintln!("--project needs a path");
                std::process::exit(2);
            }
        }
    } else {
        Workspace::demo()
    };
    let command = &args[0];
    let payload: serde_json::Value = args
        .get(1)
        .filter(|a| !a.starts_with("--"))
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));
    let res = ws.execute(command, payload);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": res.ok,
            "message": res.message,
            "data": res.data
        }))
        .unwrap()
    );
    if !res.ok {
        std::process::exit(1);
    }
}
