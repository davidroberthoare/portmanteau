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
- Added `document.import` (API and `import_document` MCP tool) for Fountain and
  FDX files, plus a File-menu import path. FDX export now preserves inline bold,
  italic, underline and strikethrough runs and writes image alt-text placeholders.
  Added Fountain and FDX round-trip tests.
- Added screenshot verification for the unified screenplay flow. The latest
  workspace run passes 159 tests.

**Still required**

- Allow image insertion before a project has been saved, and package original
  image assets in portable bundles. Current insertion copies into a saved
  project's `assets/images` folder; other export formats use warned placeholders.
- Continue polishing rich formatting in the corkboard synopsis editor and
  keep formatting/selection stable as more fields adopt the same editor.
- Ensure stylesheet appearance edits also drive every export writer, and add
  focused UI tests for stylesheet customization.
- Add plain-text and DOCX import beside the Fountain/FDX importer. The reusable
  FDX import style/object mapping dialog remains deferred below.

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
- Verified Write, Corkboard, Compile and Settings screenshots at 1440×930.

**Still required**

- Reduce persistent button rows further: consolidate infrequent actions into
  menus or a compact context toolbar.
- Make binder, inspector and preview independently collapsible, and remember
  the writer's last layout.
- Refine focus mode so it visibly reduces chrome without changing document
  behavior.
- Check contrast for muted labels and disabled states, and design the optional
  dark theme from the same token set rather than reverting to the old palette.

## 1. Import a manuscript

**Why.** Professional workflows start with a script someone else sent, or a
novel in Word. Exporting without importing is a one-way door.

**Start with Fountain, Final Draft and plain text**, then DOCX (confirmed future work):

- `parse_fountain` and `parse_fdx` now exist in the core, and `document.import`
  is wired through the API, MCP and the File menu. Add a DOCX reader and a
  plain-text importer beside them.
- DOCX: read `word/document.xml` out of the zip (the export side already uses
  `zip`) and map paragraphs to sections. Do not pull in a full document model.

**Done when** real `.fountain`, `.fdx`, `.docx` and plain-text files round-trip
through import and export without losing scene headings, character/dialogue
pairing or transitions, and a manuscript becomes chapters with readable prose.

**Deferred import workflow.** When FDX import is implemented, include a review
dialog for mapping unknown paragraph styles/objects to built-in or project
custom styles. Let writers revise those mappings, create custom styles, and save
mapping profiles for reuse on future imports. This is recorded for later; do not
build the dialog as part of the unified-editor work.

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
