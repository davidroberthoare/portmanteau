# Release notes

The newest section is used as the body of the GitHub release. Start each release
with a line `## vX.Y.Z` (matching `version` in the top-level `Cargo.toml`).

## v0.1.2

- **Portmanteau now runs in your web browser**, at
  https://davidroberthoare.github.io/portmanteau/ : the whole app, with your
  project saved in the browser, PDF/EPUB/Word and other exports that download,
  and File › Export / Open project as zip to back up or move a project (zips
  made by the desktop app open there, and the other way round).
- In the browser, pictures, research attachments and importing other files are
  not available yet.

## v0.1.1

- Spell-check no longer flags words like "I'm", "I'll" and names that the
  dictionary knows, and suggestions now come from the same dictionary.
- Linux builds now run on older distributions.

## v0.1.0

First public build of Portmanteau: a native, offline writing studio for novels,
screenplays, television and stage plays.

- Binder, corkboard and outline; prose and script editors with styles.
- Research notes, a story database, find and replace, spell-check.
- Snapshots and version comparison.
- Import from Fountain, Final Draft, Word and plain text.
- Export to PDF, EPUB, DOCX, RTF, Final Draft, Fountain, HTML, Markdown and text.
- A command-line tool and an MCP server for the same commands.
