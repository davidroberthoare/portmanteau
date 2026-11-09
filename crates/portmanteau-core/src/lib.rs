//! Portmanteau's domain core.
//!
//! Everything below the UI lives here: the project graph, screenplay blocks,
//! research, story structure, snapshots and export settings. It has no UI or
//! file-system dependencies, so it can be reused by the API, MCP server and
//! the desktop app.

pub mod model;
pub mod sample;
pub mod script;
pub mod spell;
pub mod text;

pub use model::*;
pub use script::ElementKind;
pub use text::{
    count_words, diff_counts, diff_lines, reading_minutes, truncate_words, visible_chars,
    DiffKind, DiffLine,
};
