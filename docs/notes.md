# Build notes

Running notes for the first version, in the order the work happened. These are
deliberately concrete: what was decided, why, and what was seen on screen.

## Starting point

The request asks for the core of Scrivener, Final Draft and WriterDuet, solo
first, with collaboration explicitly out of scope for now. The important words
are *novel*, *theatre*, *film*, *TV*, *idea development*, *research*,
*versioning*, *many export formats* and *front/back matter*.

The stack was fixed by the person and by the house pattern: Rust, egui/eframe,
a command API and an MCP layer under a replaceable UI.

The workspace already contained a substantial domain core, export engine,
command API, MCP server, CLI, binder, editor and inspector. The gaps were the
binary entry point, the screenshot path and eight of the nine workspaces. The
first build compiled but `main` only printed a line, so the visible product did
not yet exist.

## Decisions

**One document model, two media.** A document has prose text and script blocks;
the medium decides which is authoritative. This avoids separate prose and
screenplay types and makes conversion a single operation. It also means a scene
can be moved between a novel outline and a pilot without duplication.

**Flat binder, reconstructed tree.** Parent ids plus order integers make the
project JSON readable, allow fast filtering, and map cleanly onto a future
change log. The binder paints the tree from that flat list.

**Three version layers.** Undo for typing, named snapshots for a document,
whole-project checkpoints for milestones. Export takes a checkpoint first. A
writer should never have to wonder which version produced a PDF.

**The command bus is the product surface.** The UI calls `Workspace::execute`
for additions and compilation. The MCP server is a thin mapping over the same
commands. The Command desk is a first-class screen rather than a debug corner,
because the person explicitly wants agent development and testing.

**Compile choices are model data.** `ExportSettings` lives on the project, not
in the compile screen. Presets, the screen and the command API all read the same
struct, so a headless export and a button press cannot drift apart.

**Typography is bundled.** Liberation Serif/Sans/Mono are included for a stable
page-like editor. DejaVu Sans is included as a fallback for the small set of
marks used in the chrome (pilcrows, triangles, arrows); the first screenshot
showed missing-glyph boxes for those characters, which is exactly the kind of
thing a visual check catches.

## Build log

1. **Mapped the existing workspace.** Core model 931 lines, export 1518, API
   798, MCP 224, UI around 2.5k. Identified missing modules by following
   `app.rs` references: `ui::views` and a real `main`.
2. **Wired the shell.** Added `src/main.rs` with `--screenshot` and
   `--screenshot=...`, a 1440×930 window, theme install and eframe launch.
   Added `src/ui/mod.rs`, public snapshot/new-chapter doors, and the command
   desk selection state.
3. **Built Corkboard.** Manual card painting with label-coloured top rules,
   grouped by part, hover/selected states, word/status foot, and click-to-open.
4. **Built Outline.** Fixed header, alternating rows, manual columns, include
   checkboxes and target progress. First screenshot exposed the word count and
   percentage overprinting; removed the percentage and kept the bar.
5. **Built Research.** Sourcebook list plus note editor; title/kind/source/body
   flow into the linked research document. Added a “new note” path through the
   command bus.
6. **Built Develop.** Idea cards in a responsive grid, the spine strip, and an
   editable three-act beat sheet with add/tick. Beats are direct model edits;
   ideas go through `idea.add`.
7. **Built Characters.** Cast list with initials, colour-coded portraits, and
   want/need/flaw/arc/description fields. Appearances are computed by searching
   manuscript bodies for the character name.
8. **Built Versions.** Checkpoint composer, fourteen-day bar chart, checkpoint
   list with restore, and grouped snapshots with restore. Restoring a
   checkpoint revalidates the open document.
9. **Built Compile.** Ten format cards, presets, output folder, page/type
   controls, front-matter and body/back-matter toggles, action panel, result
   card and preview list. Fixed the API so a folder path receives the standard
   filename instead of becoming a file with no extension.
10. **Built Command desk.** Searchable command list, editable name and JSON
    payload, run button, formatted JSON response and export-ready badge.
11. **Polished from screenshots.** Replaced full-width plus signs that rendered
    as boxes with ASCII `+`; added DejaVu fallback for `¶`, `▸` and the rest;
    drew the search magnifier with the painter; fixed the clipped session card;
    pluralised version counts; aligned agent defaults with the highlighted
    command; removed dead code and warnings.
12. **Verified exports.** Ran all ten formats through `msc-cli` into a temp
    folder. Confirmed PDF magic bytes, EPUB and DOCX zip structure, FDX/XML,
    Fountain title page and project bundle.
13. **Rebuild check.** Passed: build, screenshot 2880×1860, populated start
    screen. Pictures stored under `checks/round-0/` by the checker.
14. **Closed the persistence loop.** The File menu now has Open, Save and Save
    As with a typed-path modal and `Ctrl/Cmd+O` / `Ctrl/Cmd+Shift+S`. A project
    saved from the app can be reopened, which is the minimum a writing tool owes
    its writer.
15. **Final visual pass.** Re-checked the writing desk and Compile screen after
    the persistence work; the compile path field gained a proper input frame so
    it reads as editable, and the compile header action now says “Open outline”
    rather than pretending to be a preview.
16. **Removed the collaboration scaffolding.** The person asked to skip
    collaboration entirely and keep the app local-only. Deleted the comment
    model, inspector tab, `comment.add` / `comment.resolve` commands and MCP
    tool, the unused `collaborator` field, the sample review comments, and the
    “collaboration-ready” copy. The project bundle is now described as a local
    archive, and the docs say plainly that there is no sync, account or server
    layer. Existing project files still load because serde ignores the removed
    fields.

17. **Made version comparison visible.** Added a line-level diff in the domain
    core (`text::diff_lines`), `snapshot.diff` on the command bus, an MCP tool,
    and a Compare version modal. The first version used a horizontal scroll and
    dark red/green strips on a paper background; the screenshot showed clipped
    prose and an inverted feel, so it now wraps long lines and uses pale paper
    tints with dark ink. The comparison is opened from a Compare button beside
    Restore in both the Versions screen and the inspector.
18. **Made binder order editable.** Added `document.move` (`direction` or
    `parent`), `Project::move_document` and `Project::reparent_document` with a
    cycle guard. The binder gained up/down header arrows, row context-menu
    items, and matching Edit-menu actions. Every path calls the same project
    method, so an agent and a click cannot disagree.
19. **Wrote the handoff.** Added `AGENTS.md` (rules, architecture, command
    table, conventions, recipes, limits, traps) and `docs/roadmap.md` (import,
    drag-and-drop, word-level revisions, layout, pickers, future collaboration,
    packaging). Only the uppercase `AGENTS.md` is kept; a lowercase `agents.md`
    would collide with it on case-insensitive file systems. Updated the README,
    architecture and this log to match the new commands and screens.
20. **Verified state.** `cargo build --workspace` and `cargo test --workspace`
    pass; the screenshot mode still writes a populated 2880×1860 window; the
    diff screen was reviewed directly with `SCRIPTORIUM_DIFF=1`. Export paths
    were exercised again through `msc-cli`.

21. **Moved projects to a folder package.** A project is now a `.scriptorium`
    directory instead of one JSON blob: `project.json` holds the binder
    structure and short metadata, and every long body is externalised.
    Documents are `documents/<nn>-<slug>-<id8>.md` (prose, verbatim) or
    `.fountain` (scripts); research lives in `research/`, named snapshots in
    `snapshots/<docid8>/`, checkpoint payloads in `checkpoints/<id>.json`, and
    `assets/` is reserved for future binaries. Filenames are derived from the
    record, never stored, so `msc-core` stays free of filesystem knowledge.
    The `<id8>` suffix is the lookup key; the number and title slug are there
    for humans. Saves clear and rewrite the managed directories, so renaming
    or moving a document cannot leave an orphan file behind.
    Checkpoint *ordering* comes from the manifest array: the manifest keeps
    each checkpoint's id/label/created/kind/words with its payload emptied, and
    the full JSON (including payload) is read from `checkpoints/<id>.json` on
    load.
    Old `.scriptorium.json` files still open and still save as single files;
    new saves default to `~/Scriptorium/<slug>.scriptorium/`.
    Script bodies stay human-readable Fountain. Kinds Fountain cannot express
    directly — shot, stage direction and general — carry a boneyard comment
    (`/* @shot */`, `/* @stage */`, `/* @general */`) on the line before the
    block; boneyard is ignored by standard Fountain readers. The same mechanism
    covers ambiguity that Fountain's heuristics would otherwise lose: a scene
    heading without `INT.`/`EXT.`, an ALL-CAPS action line, a lone dialogue
    block, and any other typed block whose text would be reclassified. Ordinary
    prose-like action and standard slugs stay clean, and every one of the twelve
    element kinds round-trips exactly.

## Screen review notes

- The writing screen reads as a book page rather than a form. The paper surface,
  serif body and quiet chrome are doing most of that work.
- The binder needed its session card increased from 116 to 138 points; the
  seven-day streak line was being clipped by the status bar.
- The outline initially printed `328 w` and `13%` into the same pixels. The
  bar is the useful signal; the percentage was noise at this width.
- The agent header pill stretched into a tall green slab because it sat in a
  right-to-left layout with a tall left column. A plain label is calmer.
- Dialogue in the script editor is intentionally narrow and indented. At first
  glance it can look centred; it is the screenplay column doing its job.

## What is intentionally not here

- Collaboration in any form: no real-time co-writing, presence, accounts,
  shared review server, comments or network transport. The app is local-only;
  a future version could add file-based comments through version control, but
  nothing here depends on a server.
- Drag-and-drop binder reordering. One-step moves and reparenting exist on the
  command bus and in the binder menus; the drag gesture is still to come.
- Word-level changes, tracked changes and accept/reject hunks. A line-level
  comparison and restore exist; revisions are the next versioning step.
- Import of manuscripts (Fountain, FDX, DOCX). The product currently exports.
- Native file pickers. Paths are typed, and the default export folder works
  without one.
- Packaging installers for Windows/macOS. The code is cross-platform; the
  release pipeline is separate work.

## Handoff check for this build

- The cargo workspace now lives directly in `app/`, as the brief requires, and
  the desktop binary, five support crates, bundled fonts and documentation all
  build from there. Superseded prototype folders left over from an earlier
  layout were removed so there is one source tree, not two.
- `cargo build --workspace` and `cargo test --workspace` pass (49 tests across
  the core, export, API and MCP crates and the folder-format integration suite).
- The screenshot flag was exercised at the required size and every workspace
  was reviewed as a picture: writing, corkboard, outline, research, develop,
  characters, versions, compare-version, compile and the command desk.
- All ten export formats were written through the CLI and checked as files:
  PDF (17 pages), EPUB, DOCX, FDX, Fountain, HTML, Markdown, RTF, plain text
  and the portable project bundle.

### PDF proofing fixes

The first exported PDF looked right in the app but was malformed underneath:
readers reported "Kid object is wrong type (stream)" and every other page came
out blank because page dictionaries and their content streams shared the same
object numbers. `build_pdf` now uses one explicit numbering plan (catalog,
page tree, four fonts, then page/content pairs), and a test walks the `/Kids`
list and proves every reference is a page dictionary with a separate stream.

Three layout faults were visible once the pages rendered:

- The contents page was printed before the front matter, so chapter numbers
  were two low; they now add the title page and the contents page themselves.
- The title page put the subtitle at the same baseline as the title.
- Long prose and screenplay lines ran past the page edge. Wrapping is now
  measured from the real content width, and script indents (character,
  parenthetical, dialogue, transition) are derived from that width too.

A mixed project — the demonstration novel with its television pilot and stage
play — now sets prose in Times and script sections in Courier within one PDF.
The screenplay branch does the same for a pure film, TV or theatre project.
- Nothing in this build depends on a network, an account or a background
  process. Collaboration remains future work and file-based by design.

## Open questions for the next conversation

1. Should compile use the built-in PDF typesetter longer, or delegate to
   LaTeX/HTML-to-PDF for professional book interior layout?
2. Which import formats matter most: Word manuscripts, Final Draft files,
   Fountain, or Scrivener projects?
3. Does the person want the version history to stay snapshot-based, or move to
   an automatic change log with diffs and branching?

## Round 2 — per-target compile settings and a base style

The first version had one `ExportSettings` shared by all ten formats, which
meant changing the EPUB type size silently restyled the PDF. Round 2 splits it:

- `Project.formats` is a small list of `ExportSettings`, keyed by `ExportKind`.
  `settings_for(kind)` returns the stored entry or seeds a new one;
  `set_settings(settings)` replaces or appends. Nothing is filled eagerly, so
  old manifests load with an empty list and old single-file `.json` projects
  still deserialise (the three new fields carry serde defaults).
- `Project.style` (`ProjectStyle`) is the project-wide base: separate prose and
  script font, size and leading. A novel seeds prose type; a film, TV or theatre
  project seeds script type. New formats start here, then diverge.
- The demonstration project now ships PDF, EPUB and FDX entries that differ, so
  the Compile screen has something real to switch between.

In the export engine, PDF honours `include_page_numbers` (footer) and
`include_running_headers` (a quiet 9pt title line in the top margin, drawn in
the same content stream so the page-object plan is unchanged). EPUB honours
`cover_image`: a readable JPG/PNG is stored as `cover-image/cover.<ext>`,
declared with `properties="cover-image"`, referenced by `<meta name="cover">`
and shown by a first-in-spine `cover.xhtml`. A missing file is skipped rather
than failing the export. The other writers treat the three new fields as
no-ops.

Four commands join the surface: `export.settings.get`,
`export.settings.update`, `style.get` and `style.update`, with matching MCP
tools `get_export_settings`, `update_export_settings`, `get_project_style` and
`update_project_style`; `project_open` was added alongside the corrected
folder-package description for `project_save`. The Compile screen now loads the
selected format's settings, persists edits back to `formats`, and offers a
"Reset this format to base style" button plus the base-style editor.

Save As now treats an extensionless name as a folder package (`MyBook` becomes
`MyBook.scriptorium/`) instead of inventing a legacy `.json` file. Opening and
saving still round-trip through the Round 1 folder format.
