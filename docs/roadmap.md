# Roadmap and next work

Ordered by useful-to-the-writer, then by how much structure it needs. Each item
names an entry point and a way to know it is done. Read `AGENTS.md` first for
the architecture and the rules.

## Current focus: one rich-text flow for every long-form field

Replace the prose `text` / screenplay `blocks` split with one shared content
model and editing surface. Paragraphs keep semantic styles, while caret
movement, selection, clipboard operations and deletion flow continuously across
paragraph boundaries. Inline text marks include bold, italic, underline and
strikethrough; images can be inline or standalone with alt text/captions.

- Start with Novel/Prose and Film Screenplay style sheets. A style carries its
  visual formatting and configurable next-style behavior (for example,
  Character Name → Dialogue on Enter).
- Projects own editable style sheets. Program-wide defaults are copied into a
  new project; changing those defaults does not silently restyle existing
  projects. No legacy-document migration is required for this fresh project.
- Use the same content model for manuscript bodies and other long-form fields.
  Store image binaries as project assets referenced from the content.
- Treat Fountain and FDX as critical import/export formats, not as the
  canonical model. Map their screenplay semantics to/from internal styles. When
  export cannot represent content faithfully, warn and include a placeholder.

### Progress log — 2026-11-17

**Implemented foundation**

- Added a shared `RichDocument` model with styled paragraphs, inline text marks,
  image objects, a single document-wide position space, and cross-paragraph
  range replacement.
- Added a selection-based B/I/U/strikethrough toolbar in prose and screenplay;
  the marks render from rich-text runs and a UI probe checks applying Bold to
  a text selection.
- Added Novel/Prose and Film Screenplay default style sheets, semantic script
  styles, next-style rules, and canonical-case storage with uppercase display
  transforms for screenplay cues. New projects copy the program's style-sheet
  defaults.
- Replaced free-string synopsis, document notes, research bodies and database
  notes with the same rich document structure. The inspector, Research and
  Database editors now use the rich-format controls. Existing screenwriting defaults
  now use common U.S. margins: 12-point Courier, 1.5-inch left / 1-inch right
  page margins, Character at about 3.5 inches, Dialogue 2.5–6 inches,
  Parenthetical 3–5.5 inches, and right-aligned transitions. Added Intercut,
  Subheading and Extension style types.
- Added a stylesheet editor for global defaults and project-level next-style,
  display-uppercase and paragraph-format settings.
- Added screenplay style defaults for Scene Heading, Action, Character,
  Dialogue, Parenthetical, Transition, Shot, Extension, Intercut and Subheading,
  with measurements based on the supplied references.
- Applied the reference screenplay measurements to Film Screenplay defaults and
  exposed Extension, Intercut and Subheading styles in the style palette.
- Added an image insertion dialog (for saved projects), project-local image
  assets, inline/standalone rich image nodes, image rendering, alt text/captions,
  and an export warning with alt-text placeholders where images are not embedded.
- Replaced the screenplay's per-paragraph TextEdits with one TextEdit for the
  complete flow. Prose also edits its `RichDocument` through one flow. The
  project package stores rich bodies and snapshots as `.rich.json`.
- Added headless probes for cross-style drag selection, paragraph navigation,
  and Enter applying Character Name → Dialogue. Character completion remains
  in the flow and stores the canonical character name.
- Added `document.import` (API and `import_document` MCP tool) for Fountain,
  FDX, Word (DOCX) and plain-text files, plus a File-menu import path. FDX
  export now preserves inline bold, italic, underline and strikethrough runs and
  writes image alt-text placeholders. Added Fountain and FDX round-trip tests.
- Added a DOCX reader for `word/document.xml` with paragraph styles and inline
  marks, and a plain-text reader that joins hard-wrapped prose into paragraphs.
- Added screenshot verification for the unified screenplay flow. The latest
  workspace run passes 162 tests.

**Still required**

- Allow image insertion before a project has been saved, and package original
  image assets in portable bundles. Current insertion copies into a saved
  project's `assets/images` folder; other export formats use warned placeholders.
- Continue polishing rich formatting in the corkboard synopsis editor and
  keep formatting/selection stable as more fields adopt the same editor.
- Ensure stylesheet appearance edits also drive every export writer, and add
  focused UI tests for stylesheet customization.
- The reusable FDX import style/object mapping dialog remains deferred below.

**Done when** headless UI tests can drag-select and edit across different
paragraph styles, script Enter follows the stylesheet's next-style rule, prose
and screenplay share the same editor/model, the sample project uses the new
model, and Fountain/FDX conversion has round-trip tests for supported
screenplay semantics.

## Next focus: a light, quiet, Mac-native interface

Move the shell from a dark, button-heavy high-tech look to a calm writing
environment. The manuscript should dominate; chrome should recede. Vellum is
the primary reference for editor/preview composition, with iA Writer for
focus and restraint, Ulysses for document navigation, and Apple's Human
Interface Guidelines for native sidebar and toolbar behavior. The full brief is
in `docs/ui-style-guidelines.md`.

**Progress on branch `ui/light-mac-redesign`**

- Replaced the dark palette with a warm-gray chrome, soft-white manuscript
  paper, charcoal text, muted blue-green accent and restrained status colors.
- Switched egui to its light visual theme, softened window shadows, and moved
  the top bar, status bar, rail, binder, inspector, editor toolbar, style
  palettes and primary buttons onto the light surfaces.
- Kept the editor as the focal point: screenplay columns and prose typography
  are unchanged, and the page now reads as paper rather than a bright card on a
  dark background.
- Consolidated binder actions into one `+` menu; the binder and inspector can
  each be hidden from their header or the View menu, and the last layout is
  remembered in the program settings. Hidden panels leave a slim edge handle
  instead of a permanent button row.
- Focus mode now drops the rail, status bar, binder, inspector and editor
  toolbar, leaving a single slim title bar with a visible Exit focus control.
  Nothing about the document or its editing behaviour changes.
- Darkened secondary text and semantic label colours to clear a comfortable
  contrast ratio at chrome type sizes, and added ink-strength variants for
  sage, blue, red, purple and the accent.
- Rebuilt the Settings → Preferences card around Writing, Spelling language and
  Workspace sections, with panel visibility stored alongside the other global
  defaults. Unit tests no longer write to the writer's real config file.
- Verified Write, hidden-panel, focus-mode, Corkboard, Compile, Settings,
  Research, Versions and Characters screenshots at 1440×930.

**Still required**

- Continue reducing persistent control rows in the remaining workspaces
  (Compile still carries a long format list and export action) as they are
  touched.
- Check contrast for disabled states after the optional dark theme exists, and
  keep the dark theme on the same token set rather than reverting to the old
  palette.
- Consider remembering editor zoom, write mode and inspector tab per project
  alongside the panel layout.

## 1. Import a manuscript

**Why.** Professional workflows start with a script someone else sent, or a
novel in Word. Exporting without importing is a one-way door.

**State.** The four importers live in `crates/portmanteau-import`, one module
per format over a shared `Imported` result (documents, title, author,
warnings). `document.import` places the documents in the binder; the MCP tool
and File → Import call the same command.

- **Fountain 1.1**: title page to metadata, boneyard and `[[notes]]` removed,
  forced elements, scene numbers, two-space speech continuation, emphasis to
  marks, `#` sections to separate documents with `=` synopses.
- **Final Draft**: unknown paragraph types, images and script notes are
  reported, never fatal; title page read.
- **Word**: `styles.xml` decides what a heading is (renamed or localised
  styles work); headings split chapters; text boxes are not duplicated;
  footnotes, images, tables and list formatting are counted in the warnings.
- **Plain text**: hard-wrap rejoining, chapter detection, UTF-8/UTF-16/
  Windows-1252.
- Round trips through our own Fountain, FDX, DOCX and plain-text exporters are
  tested; that is how the non-standard Fountain export was found and fixed.

**Not done yet** (ordered by value):

1. A review step before the documents are created: show the split (titles and
   word counts), let the writer merge, rename or skip, and choose the parent.
2. Dual dialogue, centred text and Fountain outline markers are flattened;
   model them if writers use them.
3. Word: images (needs asset embedding), footnotes, tables, tracked changes,
   comments.
4. Character extraction: offer to create database characters from the cues.
5. Markdown and RTF readers.

**Deferred import workflow.** A review dialog for mapping unknown paragraph
styles/objects to built-in or project custom styles, with mapping profiles for
reuse. Recorded for later; do not build it as part of the unified-editor work.

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

- Extend `portmanteau-core/src/text.rs` with a word-level diff, or add a `diff` module.
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

- `crates/portmanteau-export/src/lib.rs`: one assembly pass, one writer per format.
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

- Build `portmanteau` for Linux, macOS and Windows. The code is platform-neutral;
  the work is CI, icons, installers and signing.
- Keep the screenshot flag working in release builds; it is the visual check.
- Add a version number and a changelog before the first public build.

**Done when** a non-programmer can install the app from a signed package and
open the sample project.

## Smaller polish items

- Future UI visual redesign: move from the current dark, button-heavy chrome to
  a light, sparse, Mac-native writing environment. See
  `docs/ui-style-guidelines.md`. Begin after the unified editor work is stable.
- A compile preview that opens as a modal page rather than a list.
- Per-document compile overrides (font, spacing) for mixed manuscripts.
- A proper empty state for the binder when a new project has no chapters.
- Keyboard navigation in the binder (arrow keys, collapse/expand, rename).
- Rename documents inline, not only through the inspector title field.
- Undo history length and a visible "edited" marker per document.
- Search filters (title only, research only, current part).
- A character relationship map derived from appearances.
- Project statistics: scenes per part, dialogue vs action ratio, reading time.

## UI polish pass (selection popup, style dropdown, type scale, binder icons)
- Floating B/I/U/S popup above selected text in every rich editor; Image moved to Insert ▸ Image….
- Script paragraph styles are a toolbar dropdown instead of a button palette.
- Interface type scale lives in `theme.rs` (`ui_small`/`ui_body`/`ui_strong`) with a Settings ▸ Interface text size preference.
- Binder uses drawn folder and page icons, tinted by document type.

## Undo / redo history (done)

`src/history.rs` watches the selected document once per frame in which input
arrived and records the changed paragraph window as a step; typing merges until
the writer pauses (0.7 s). It covers the body, synopsis and notes, every edit
path (typing, styles, marks, images, spell fixes, snapshot restores), and is
per document for the session. ⌘Z / ⇧⌘Z / Ctrl+Y and Edit > Undo / Redo.

Not yet: undo for binder changes (move, delete, rename), research and database
text, and a visible history list. Steps are not saved with the project.

## Session log: UI polish, character cues and export styling (latest)

Done and merged to `master`:
- Light Mac-style redesign, collapsible binder/inspector, focus mode, shared type scale (`ui_small/ui_body/ui_strong`) with a Settings text-size preference, drawn binder icons.
- Zoom reads 100% at the comfortable default (prose 1.4x, script 1.2x; `PROSE_ZOOM_BASE`/`SCRIPT_ZOOM_BASE`).
- Floating B/I/U/S popup on selection; Insert ▸ Image…; script paragraph style is a toolbar dropdown.
- Character cues: caret-anchored popup (4 visible rows, flips above when short), opens only while typing or in a fresh empty cue; other speaker first (↺), short script forms (MARA, BOY) merged into database names; Enter splits at the caret (fixes wrong-style new paragraph); empty-paragraph caret drawn at the style's indent; auto-scroll keeps ~2in below the caret.
- All prose exporters and the page preview honour per-paragraph stylesheet styles; preview pages are a constant, clipped size scaled to fit the column.
- Toolbar, search and Compile field alignment; corkboard synopsis scrolls.

Next session candidates:
- Verify cue popup and empty-paragraph caret by hand on more screenplays (stage play, prose with custom styles); only headless-tested.
- Preview page-fit estimate vs real export pagination (clipped lines when estimate is off).
- FDX style/object mapping review dialog; optional dark theme on the same tokens; further reduction of persistent control rows in Compile.
- Remove dead `legacy_script_page` in `src/ui/editor.rs` and the unused `insert_image` field on `InlineToolbarResult`.

## Editing on pages (decided: one editor)

The continuous editor is one egui `TextEdit` over the whole document; the page
layout comes from the exporter's own text engine. Editing directly on the
exported pages would need a second editing engine, so every editing feature
would be written twice. Instead the single editor learns about pages:

- Done: page markers in the continuous view ("PAGE 7", the number printed on
  the page) from `page_counts().starts`; click a paragraph in Paged to open
  Continuous with the caret there (`CaretRequest`, `hit_source`); Paged stays
  read-only.
- Layout lines carry `PdfSource` (document, paragraph, offset, line) so they can
  point back at the text. Offsets are approximate by a few characters because
  wrapping drops the spaces it breaks on.
- The editor wraps with egui and the PDF with its own engine, so a marker can
  sit a line or two from the printed break. If that proves too loose, match the
  editor's wrap width to the page's text width.
