# Roadmap and next work

Ordered by useful-to-the-writer, then by how much structure it needs. Each item
names an entry point and a way to know it is done. Read `AGENTS.md` first for
the architecture and the rules.

## 1. Import a manuscript

**Why.** Professional workflows start with a script someone else sent, or a
novel in Word. Exporting without importing is a one-way door.

**Start with Fountain and Final Draft**, then DOCX:

- `crates/msc-core/src/script.rs` already parses loose text into typed blocks.
  Add `parse_fountain` and `parse_fdx` next to `parse_loose`.
- Add `project.import` in `crates/msc-api/src/lib.rs` with `format` and `path`.
- DOCX: read `word/document.xml` out of the zip (the export side already uses
  `zip`) and map paragraphs to sections. Do not pull in a full document model.
- Surface it in the File menu beside Open/Save, and as an MCP tool.

**Done when** a real `.fountain` and `.fdx` file round-trips through import and
export without losing scene headings, character/dialogue pairing or transitions,
and a DOCX manuscript becomes chapters with readable prose.

## 2. Drag-and-drop binder

**Why.** Reordering exists as buttons and a command, but dragging is the gesture
writers expect.

- `document.move` and `Project::reparent_document` already exist with cycle
  protection and tests.
- Use egui's drag-and-drop helpers in `src/ui/binder.rs`. Drop above/below a row
  for a reorder; drop onto a folder to reparent.
- Keep the existing arrow buttons and context menu as the keyboard-accessible
  path. Do not put domain logic in the drop handler; call the project methods.

**Done when** dragging a chapter across parts, and a scene into a folder, updates
the tree, marks the project dirty and survives save/reopen.

## 3. Word-level diff, tracked changes and review export

**Why.** The line diff answers "what changed?" but not "let me accept this
sentence and reject that one." A revision workflow needs a finer unit and a
record that can be shared.

- Extend `msc-core/src/text.rs` with a word-level diff, or add a `diff` module.
- Keep `snapshot.diff` backward compatible; add a `unit` field or a separate
  `snapshot.diff_words`.
- Add `document.revision` records (old text, new text, author, timestamp,
  accepted state) as a local, file-based history. No server.
- Export a review document (Markdown or HTML) with additions and removals
  marked, plus a cover page and a summary.

**Done when** a writer can compare a snapshot, read word-level changes, accept or
reject a hunk, and export the revision notes without any network service.

## 4. Professional PDF and EPUB layout

**Why.** The current engines produce valid files, but not typeset books.

- `crates/msc-export/src/lib.rs`: one assembly pass, one writer per format.
  Improve the PDF writer first (running heads, margins, chapter openers,
  widows/orphans, embedded fonts) before adding an external toolchain.
- For EPUB, add a proper NCX/nav document and per-chapter XHTML files rather
  than one flat document if the current output is too coarse for stores.
- Keep all settings in `ExportSettings`; the Compile screen must remain a view
  of that struct.

**Done when** a 60,000-word novel compiles to a PDF with consistent chapter
openers, page numbers and margins on A5 and trade paperback, and an EPUB that
opens cleanly in Calibre and a browser reader.

## 5. Native file pickers and recent projects

**Why.** Typing full paths is the largest everyday friction left.

- Add `rfd` (or a platform-native picker) behind a thin abstraction so the core
  stays free of UI dependencies.
- Reuse the existing `PathDialog` state and `Workspace::open/save`; only the
  source of the path changes.
- Add a recent-project list to the File menu, stored in a small local config
  file, not in the project.

**Done when** Open, Save As and Export can all be completed without typing a
path, and a missing recent file fails gracefully.

## 6. Collaboration, later and deliberately

**Why.** The person said collaboration will matter eventually, but asked for a
solo-first product and later chose to leave it out of this version.

If it is revisited, keep it **file-based and versioned**:

- A shared project file in the writer's own Git repository, with a review
  sidecar for comments and suggested changes.
- Merge by document id and snapshot id, never by screen state.
- No accounts, presence, hosted server or live co-editing in the first step.
- Ask the person which of those they actually want before designing it.

**Done when** two writers can exchange a project and review notes through a
plain file (or a Git remote they own) without either installing a service.

## 7. Packaging and release pipeline

- Build `myscript` for Linux, macOS and Windows. The code is platform-neutral;
  the work is CI, icons, installers and signing.
- Keep the screenshot flag working in release builds; it is the visual check.
- Add a version number and a changelog before the first public build.

**Done when** a non-programmer can install the app from a signed package and
open the sample project.

## Smaller polish items

- A compile preview that opens as a modal page rather than a list.
- Per-document compile overrides (font, spacing) for mixed manuscripts.
- A proper empty state for the binder when a new project has no chapters.
- Keyboard navigation in the binder (arrow keys, collapse/expand, rename).
- Rename documents inline, not only through the inspector title field.
- Undo history length and a visible "edited" marker per document.
- Search filters (title only, research only, current part).
- A character relationship map derived from appearances.
- Project statistics: scenes per part, dialogue vs action ratio, reading time.
