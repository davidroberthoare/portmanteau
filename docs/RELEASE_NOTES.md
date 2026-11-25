# Release notes

The newest section is used as the body of the GitHub release. Start each release
with a line `## vX.Y.Z` (matching `version` in the top-level `Cargo.toml`).

## v0.2.0

- **Pictures in your writing.** Insert an image (or drop or paste one in) and
  choose its size, put it left, centre or right, and have the text wrap around it
  with space around it, like in Word. Add a caption and a description. Right-click
  a picture for its settings in a window you can move, or use the list in the
  Document tab. Pictures and captions come out in PDF, Word, EPUB and HTML.
- **Text wraps around pictures in the editor**, not only in the exports.
- **Compile:** an optional cover page in PDF; contents page options (heading
  text, page numbers, dots between title and number); a Global font setting on the
  Styles tab that styles follow unless you give them their own.
- Book layout: trim sizes, inside and outside margins, headers and footers with
  page numbers and chapter names, and chapters starting on right-hand pages.
- **Removed:** the RTF export (Word, PDF, EPUB and the rest remain).
- Fixed: inserting a picture no longer adds a second, duplicate caption.

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
