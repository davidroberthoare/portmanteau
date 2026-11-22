# Roadmap and next work

Only what is still open, ordered by usefulness to the writer. Finished plans
were removed on 2026-11-22; the git history keeps them, and `docs/notes.md` is
the running build log. Read `AGENTS.md` first for the architecture and rules.

**The goal:** an all-in-one, friendly app where a writer plans, researches and
writes, brings existing work in from other programs, and gets it out as
ebook, print and manuscript formats, all from one project.

## Now: releases, updates and the web build

Plan, status and deviations: `docs/release-and-updates-plan.md`. Built and
untested on GitHub so far: CI, tagged releases, the update icon and panel,
signed in-app install, the welcome project. Open: push and watch CI, the real
two-release update test (`docs/releasing.md`), then finish the web version (Phase 4:
the libraries build for the browser and projects can be zipped; the app shell,
browser storage and the Pages build remain).

**Done when** a non-programmer can download the app for their system, is told
when a new version is out, installs it from inside the app, and can open the
web version in a browser.

## Publishing: ebook and print

What exists: EPUB, and PDF with embedded fonts at US Letter, A4, A5 and trade
paperback, with per-format margins, title page, contents and scene numbers.
Still to do, roughly in order:

1. Check the EPUB against stores' rules (run `epubcheck`; add a nav document and
   one XHTML file per chapter if the output is too coarse).
2. PDF hyperlinks (link annotations), then a copyright-page template and the
   contact line on prose title pages.
3. Print extras: bleed and crop marks, mirrored margins and a gutter for
   two-sided print, a cover wrap sized from the page count.
4. Per-document compile overrides (font, spacing) for mixed manuscripts, and a
   "compile with the last settings" action in the top bar.
5. A 60,000-word novel compiles to a PDF with consistent chapter openers, page
   numbers and margins on A5 and trade paperback, and opens cleanly in Calibre.

Keep every setting in `ExportSettings`; the Compile screen stays a view of it.

## Import: what is left

The importers live in `crates/portmanteau-import`, one module per format over a
shared `Imported` result. The review screen (split, title-page details, style
mapping, remembered choices) is done (`src/ui/import_review.rs`). Not done yet,
by value:

1. Dual dialogue, centred text and Fountain outline markers are flattened;
   model them if writers use them.
2. Word: hyperlinks, images (needs asset embedding), footnotes, tables, tracked
   changes, comments.
3. Character extraction: offer to create database characters from script cues.
4. Markdown and RTF readers.

## Drag-and-drop binder

Reordering exists as buttons and `document.move` / `Project::reparent_document`
(cycle-protected, tested). Add dragging in `src/ui/binder.rs` with egui's
drag-and-drop helpers: drop above/below a row to reorder, onto a folder to
reparent. Keep the arrow buttons and context menu as the keyboard path and call
the project methods; no domain logic in the drop handler.

**Done when** dragging a chapter across parts and a scene into a folder updates
the tree, marks the project dirty and survives save and reopen.

## Word-level diff, tracked changes and review export

The line diff answers "what changed?", not "accept this sentence, reject that
one". Add a word-level diff (a `diff` module in `portmanteau-core`), local
`document.revision` records (old text, new text, author, timestamp, accepted),
and a Markdown or HTML review export with additions and removals marked. No
server.

## Undo beyond document text

Undo covers a document's body, synopsis and notes (`src/history.rs`). Not yet:
binder changes (move, delete, rename), research and database text, replace in
the editor's own history (use Versions), and a visible history list.

## Collaboration, later and deliberately

Left out of this version by the owner. If revisited, keep it file-based and
versioned: a shared project in the writer's own Git repository, a review sidecar
for comments, merging by document and snapshot id, and no accounts or live
editing. Ask the owner what they want before designing it.

## Smaller items

- A proper empty state for the binder when a project has no chapters.
- Keyboard navigation in the binder (arrows, collapse and expand, rename); rename
  documents inline, not only in the inspector.
- Search filters (title only, current part) and search inside the Paged view.
- Lists (bullets, numbering) need a list concept in the text model first.
- A character relationship map derived from appearances; project statistics
  (scenes per part, dialogue versus action, reading time).
- An optional dark theme on the same colour tokens (`src/theme.rs`).
- Remember editor zoom, write mode and inspector tab per project.
- Remove the unused `insert_image` field on `InlineToolbarResult`
  (`src/ui/editor.rs`).

## Decision: one editor, pages come to it

The continuous editor is one egui `TextEdit` over the whole document; page
layout comes from the exporter's own text engine. Editing directly on the
exported pages would need a second editing engine, so every editing feature
would be written twice. Instead the single editor learns about pages:

- Done: Paged shows the real compiled pages; clicking a paragraph in Paged opens
  Continuous with the caret there (`CaretRequest`, `hit_source`). Paged stays
  read-only.
- Tried and removed: "PAGE N" markers in the continuous view. The editor wraps
  with egui and the PDF with its own engine, so markers sat a line or two off the
  printed breaks. Revisit only if the editor's wrap width is made to match the
  page's text width.
- Layout lines carry `PdfSource` (document, paragraph, offset, line) so they can
  point back at the text; offsets are approximate by a few characters.

## Interface

The look is light and quiet: warm-gray chrome, soft-white paper, the manuscript
dominating (`src/theme.rs`, brief in `docs/ui-style-guidelines.md`). Keep
reducing persistent control rows in workspaces as they are touched (Compile still
carries a long format list), and put any dark theme on the same tokens.
