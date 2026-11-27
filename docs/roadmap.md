# Roadmap and next work

Only what is still open, ordered by usefulness to the writer. Finished plans
were removed on 2026-11-22; the git history keeps them, and `docs/notes.md` is
the running build log. Read `AGENTS.md` first for the architecture and rules.

**The goal:** an all-in-one, friendly app where a writer plans, researches and
writes, brings existing work in from other programs, and gets it out as
ebook, print and manuscript formats, all from one project.

> The RTF exporter was removed on 2026-11-25 (owner: not a useful format).

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
4. Per-document compile overrides (font, spacing) for mixed manuscripts. (The
   top bar's Compile button, which compiles with the last settings, is done.)
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
4. Markdown reader.

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

## Statistics, goals and encouragement (first version 2026-11-26)

Done: activity log in hourly buckets per document (`portmanteau-core/src/activity.rs`),
typing and reading time from input (`src/activity.rs`), added/edited/deleted
words from the undo history's closed steps, goals (daily and weekly, words or
minutes), streaks, badges and kind messages (`progress.rs`), prose analysis
(`analysis.rs`), and the Writing statistics window (`src/ui/stats_window.rs`)
with the top-bar meter. Still to do: analyse scripts (dialogue by element),
per-document and per-chapter style comparison, a pacing view across chapters,
restore/paste not counted as writing, rest days that keep a streak, a weekly
summary, export of the log as CSV, and a "write today" reminder (needs no
daemon: only when the app is open).

## Saving and safety (2026-11-26)

Done: a saved project autosaves in place after a pause; automatic dated backups
(every 15 minutes while working, newest ten kept, kind "auto"); a quit/close
question when work has no home or the last autosave failed; a recovery copy of
an unsaved project in `<config>/recovery/` offered back on the next start
(`src/recovery.rs`). Not done: the same question when New or Open would replace
an unsaved project (its recovery copy is kept, so it can be brought back), and
tidying the large nested checkpoints that older projects already contain.

## Smaller items

- Keyboard navigation in the binder (arrows, collapse and expand, rename); rename
  documents inline, not only in the inspector.
- Search filters (title only, current part) and search inside the Paged view.
- Lists (bullets, numbering) need a list concept in the text model first.
- A character relationship map derived from appearances; project statistics
  (scenes per part, dialogue versus action, reading time).
- An optional dark theme on the same colour tokens (`src/theme.rs`).
- Remember editor zoom, write mode and inspector tab per project.

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

## Compile redesign (agreed 2026-11-23)

Done (2026-11-23, uncommitted at the time of writing):
- Contents / Layout / Styles tabs; one "What goes in" list replaces the
  Manuscript and Scope dropdowns (`Project::compile_types_in`).
- One style editor (`views::style_editor`): type picker, style picker, one form.
  Used by Compile (per format) and Settings. The stylesheet editor (next-style
  rules, display uppercase, script margins) stays in Settings only.
- Trim sizes (5x8, 5.25x8, 5.5x8.5, 6x9, B-format, A4/A5/Letter, Custom w x h),
  `mirror_margins` (left = inside, right = outside; PDF shifts left-hand pages,
  DOCX writes `w:mirrorMargins`). Automatic mirrored margins
  are 23 mm inside, 17 mm outside.
- Headers and footers: `HeaderFooter` is now six slots with fields
  (`{page} {pages} {title} {author} {chapter} {chapter_number}`), right/left-hand
  variants, a first-page variant per region, no header on chapter openers, and
  front matter / body / back matter regions (same as body, custom, none; own
  numerals; restart numbering). PDF only; scripts keep their own top-right number.
- Project cover (`Project.cover_image`, command `project.set_cover`) with a
  per-format override and `include_cover`; EPUB uses it, and prose PDF opens with it
  as an uncounted first page (`add_cover_page`, added after numbering and mirroring so
  the title page stays page one). DOCX cover: not yet.

Not done / next:
- **Per-document sections** ("start a new section", same as previous) on top of
  the front/body/back regions.
- DOCX headers and footers are DONE for the whole file using the body templates
  (Word fields PAGE/NUMPAGES/STYLEREF, odd/even, `titlePg`, page-number format
  and start). Still PDF-only: front/back regions, a bare title page, no header
  on chapter openers (need real Word sections).
- **Pictures in prose** (owner: important). FIRST PASS DONE 2026-11-24: a
  picture on its own paragraph is drawn by PDF (krilla), DOCX (inline drawing),
  EPUB (packaged `images/`) and HTML (data URI), centred, scaled
  to the measure, PNG/JPEG only, read from `<project>/assets` (`picture.rs`;
  `Project.asset_root` carries the folder). The Compile preview draws them too.
  Second round 2026-11-24: a picture's caption prints under it in every writer
  (generated paragraph in `prose_paragraphs`; PDF keeps it on the picture's page);
  Insert image offers Small/Medium/Large/Full width (default 4.5 in).
  Third round 2026-11-24: position (left/centre/right), "text wraps around it"
  and padding, like Word's Position/Wrap Text. Model: `ImageObject.align/wrap/padding`;
  command `document.picture` (MCP `update_picture`); the inspector's Document tab
  lists each picture with Width, Position, Wrap, Space around, Caption and
  Description. PDF (and so the Compile preview) really wraps text beside the
  picture (`Flow.float`, `wrap_chars_with`, caption set inside the picture's
  width); HTML/EPUB float a `<figure>`; DOCX writes an anchored picture with
  square wrapping (its caption prints as the line after).
  The editor wraps too (2026-11-25): `script_flow_galley` treats a wrapped picture as a
  float box (`FloatBox`) and lays the following paragraphs out narrower beside it,
  splitting a paragraph that passes the picture's bottom (`job_from_char`).
  STILL TO DO: pictures inside a line of text; numbered figures; a picture's controls on the
  picture itself (click to select, drag handles); picture edits in undo; GIF/WebP; Markdown/Fountain/FDX (placeholders); large-image downsampling. Old note: the PDF writer draws no pictures (it prints `[Image]`), so no cover page in PDF
  and no inline pictures. Needs picture support first.
- "Start chapters on a right-hand page": DONE for prose PDF (`chapters_start_right`; blank left-hand pages carry no furniture but count in the numbering). Not yet DOCX (page-break-before odd page needs section breaks).
- `.tex` export (text writer over `content` and `layout`; no TeX dependency).
- **Table of contents customization** (the last item of this work): title text,
  depth (parts/chapters/scenes), page numbers on/off, dot leaders, which
  documents appear, numbering style, per-level styles, and a real linked TOC in
  PDF/EPUB.
- Grey out style properties that do not apply to a format (EPUB ignores some).

- Table of contents, first pass done 2026-11-24: heading text, page numbers and dot leaders (PDF) via `toc_title`, `toc_page_numbers`, `toc_leaders` (Compile > Contents). Still to do: depth/which documents appear, numbering style, per-level styles, linked entries in PDF, page numbers in DOCX (needs a TOC field), EPUB/HTML styling.
