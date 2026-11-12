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
9. **Built Compile.** Ten format cards, output folder, page/type
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
12. **Verified exports.** Ran all ten formats through `portmanteau-cli` into a temp
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
    diff screen was reviewed directly with `PORTMANTEAU_DIFF=1`. Export paths
    were exercised again through `portmanteau-cli`.

21. **Moved projects to a folder package.** A project is now a `.portmanteau`
    directory instead of one JSON blob: `project.json` holds the binder
    structure and short metadata, and every long body is externalised.
    Documents are `documents/<nn>-<slug>-<id8>.md` (prose, verbatim) or
    `.fountain` (scripts); research lives in `research/`, named snapshots in
    `snapshots/<docid8>/`, checkpoint payloads in `checkpoints/<id>.json`, and
    `assets/` is reserved for future binaries. Filenames are derived from the
    record, never stored, so `portmanteau-core` stays free of filesystem knowledge.
    The `<id8>` suffix is the lookup key; the number and title slug are there
    for humans. Saves clear and rewrite the managed directories, so renaming
    or moving a document cannot leave an orphan file behind.
    Checkpoint *ordering* comes from the manifest array: the manifest keeps
    each checkpoint's id/label/created/kind/words with its payload emptied, and
    the full JSON (including payload) is read from `checkpoints/<id>.json` on
    load.
    Old `.portmanteau.json` files still open and still save as single files;
    new saves default to `~/Portmanteau/<slug>.portmanteau/`.
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
  database, versions, compare-version, compile and the command desk.
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

In the export engine, PDF honours `HeaderFooter`: page numbers can be Arabic or
lower/upper Roman, start at any number, sit in the header or footer, align
left/centre/right and be suppressed on the first page; an optional running
header repeats either custom text or the project title. Both are drawn in the
same content stream so the page-object plan is unchanged. EPUB honours
`cover_image`: a readable JPG/PNG is stored as `cover-image/cover.<ext>`,
declared with `properties="cover-image"`, referenced by `<meta name="cover">`
and shown by a first-in-spine `cover.xhtml`. A missing file is skipped rather
than failing the export. The other writers treat the header/footer settings as
no-ops.

Round 28b replaces the old `include_page_numbers` / `include_running_headers`
booleans with a nested `HeaderFooter`. `export.run` and
`export.settings.update` accept `{"header_footer":{...}}` and merge only the
fields supplied, so a caller can change the format and start number without
resetting placement. `HeaderFooter` carries its own serde default, so older
project files missing the key load with page numbers on and running headers
off. The Compile panel drops the redundant global type-size/leading controls;
paper size stays for PDF, DOCX and RTF.

Four commands join the surface: `export.settings.get`,
`export.settings.update`, `style.get` and `style.update`, with matching MCP
tools `get_export_settings`, `update_export_settings`, `get_project_style` and
`update_project_style`; `project_open` was added alongside the corrected
folder-package description for `project_save`. The Compile screen now loads the
selected format's settings, persists edits back to `formats`, and offers a
"Reset this format to base style" button plus the base-style editor.

Save As now treats an extensionless name as a folder package (`MyBook` becomes
`MyBook.portmanteau/`) instead of inventing a legacy `.json` file. Opening and
saving still round-trip through the Round 1 folder format.

## Round 3 — font family reaches the page, and formats fail loudly

`settings.font` used to be stored and passed around but never read: every
writer hardcoded its family. Round 3 wires it in.

- **PDF** maps the family to a base-14 font with `pdf_font`: Courier for
  names containing "courier" or "mono", Helvetica for "sans"/"helvetica"/
  "arial", and Times otherwise (with a `-Bold` companion). The prose font
  objects use that mapping; screenplay lines keep Courier. The page-object
  plan and `/Kids` numbering are unchanged.
- **EPUB** builds `style.css` from the setting, so the body becomes
  `font-family: "<font>", serif`. Script element classes stay monospace.
- **HTML**, **DOCX** and **RTF** use the same family in their inline CSS,
  `styles.xml`/`rFonts` and `\fonttbl`; DOCX script paragraphs stay Courier.
- Fountain, FDX, TXT and Markdown have no font concept, so they are untouched.

`export.run`, `export.settings.get` and `export.settings.update` now reject a
`format` key they cannot parse with `Unknown format: <value>` instead of
silently falling back to the project's current kind. `export.settings.get` and
`update` also require a format rather than guessing one.

## Round 4 — safety fixes around persistence and storage Fountain

- **Path traversal.** `storage.rs` now derives every id-based filename through
  `safe_component`, which keeps only ASCII alphanumerics and `-` (an empty
  result becomes `x`). Alongside `id8`, this cleans `document_name`,
  `research_name`, snapshot stems, snapshot doc directories and the checkpoint
  filename used by both save and load, so a hostile `project.json` cannot make
  a save write outside the package.
- **Atomic folder save.** `save_folder` builds the whole package in a fresh
  sibling temporary directory (`.<name>.tmp-<pid>-<nanos>`), renames the
  existing package to `.<name>.bak-…`, then renames the temporary into place
  and removes the backup. A failure at any step cleans up the temporary and
  restores the backup, so an interrupted save leaves the previous project
  intact. `load_folder` still reads only `dir`; leftover temp/backup siblings
  are ignored.
- **Finite numeric settings.** `export.settings.update`, `style.update` and the
  `export.run` override path parse `font_size`/`prose_size`/`script_size`
  against 4.0–72.0 and `line_spacing`/`prose_leading`/`script_leading` against
  0.5–3.0. A value such as `1e39` now returns a clear error instead of storing
  f32 infinity, which `serde_json` writes as `null` and which used to make the
  project impossible to reopen.
- **Explicit storage Fountain.** `to_storage_fountain` now writes a boneyard
  marker before every block, and an empty block is just its marker line.
  `parse_fountain` resolves a block's kind from that marker first (falling back
  to the old heuristics only when no marker is present), and emits an empty
  block when two markers arrive without body text. This fixes lowercase action
  text after a character cue being read as dialogue, and empty blocks being
  dropped, while old saved packages still load.

## Round 6 — folder-only projects and a single Draft binder root

- **`.json` files are no longer projects.** `Workspace::open` accepts only a
  folder package; any other path (including an old `.json` file) returns
  `Not a Portmanteau project (expected a .portmanteau folder)`. `Workspace::save`
  always writes a folder package, so `project.open`/`project.save` and their
  catalogue descriptions now say "folder package".
- **Binder root is `Draft`.** `Project::ensure_folders` seeds Draft, Research,
  Front Matter and Back Matter. The demonstration project no longer creates a
  separate `Script Lab` folder: `Cold Harbor · Pilot` and `The Lamp Room · One
  act` sit alongside the novel chapters under Draft. `document.create` defaults
  to the Draft folder.
- The `manuscript_documents()` export filter still keys off the `Research`
  folder title, which is unchanged.

## Round 7 — UI cleanup: Draft everywhere, folders in the binder, fewer nav buttons

- **Draft, not Manuscript.** The remaining UI paths that looked up the old
  `Manuscript` folder now look up `Draft`, and the compile/corkboard fallback
  labels say "Draft". The `Script Lab` title check is gone; a new document's
  medium now follows the project kind (Film/Television/Theatre → screenplay,
  otherwise prose).
- **Folders are first-class in the binder.** The header has a `▣ +` button next
  to `+`, row context menus offer "New folder", and `Action::NewFolder` resolves
  a parent the same way "New inside" does (given parent → selected folder →
  selected document's parent → root). Folders are created as `DocKind::Folder`
  with a Prose medium.
- **Scattered navigation removed.** Buttons that only switched screens
  (corkboard in the editor/binder, outline on Compile, corkboard on Outline,
  research and version history in the inspector) are gone; the left rail and the
  View menu remain the way to change workspace.
- **Script margin type labels removed.** The per-block gutter no longer draws
  the element short name or cycles its type on click. Scene numbers still show,
  and the element-kind button row above the script still sets the type.

## Round 8 — per-element project styles

- **One style per element.** `ProjectStyle` (prose/script font, size, leading)
  is replaced by `ElementStyles`, fifteen `ElementStyle` records covering the
  title page, chapter heading, body, and every screenplay element. Each record
  carries `font`, `size`, `leading`, `indent`, `before` and `after`.
- **Project field renamed.** `Project.style` is now `Project.styles`. It keeps
  `#[serde(default)]`; older project folders that still carry the removed
  `style` key load with default element styles because serde ignores the old
  field.
- **Nested command payloads.** `style.update` now takes nested objects:
  `{"body":{"size":12,"font":"Georgia"},"dialogue":{"indent":80}}`. Numeric
  values are still range-checked before anything is stored.
- **Compile screen.** The old two-row base-style editor is now a scrollable
  list of elements with font, size, leading and indent. “Reset this format to
  base style” seeds from `body` for prose projects and `action` for script
  projects, matching `settings_for`.

## Round 9 — global Settings workspace

- **A tenth workspace.** A separate Settings workspace sat at the end of the
  rail. It edited the global `AppSettings` instead of the open project. It was
  later folded into the Compile screen's Display tab (Round 26b).
- **Config file.** Defaults are stored as pretty JSON at
  `$XDG_CONFIG_HOME/portmanteau/config.json`, falling back to
  `$HOME/.config/portmanteau/config.json`, then `./portmanteau-config.json`.
  A missing or malformed file quietly loads the built-in `ElementStyles`.
- **One editor, two owners.** The per-element style rows moved out of Compile
  into `views::element_styles_editor`. Compile uses it for
  `project.styles`; Settings uses it for `app.settings.styles`.
- **Defaults flow.** "Save defaults" writes the config file, "Apply to current
  project" copies the global styles into `project.styles` and marks the project
  dirty, and "Reset defaults" restores the built-in styles. Creating a new
  project copies the global defaults into it; the demonstration project keeps
  its own styles.

## Round 10a — per-format element styles

Each output format now carries its own `ElementStyles` inside `ExportSettings`.
A format that has never been configured is seeded from `project.styles`, so PDF
and EPUB can diverge without touching the project-wide base. `export.settings.get`
returns the stored styles and `export.settings.update` accepts a nested `styles`
object (`{"body":{"size":15}}`), merged element by element with the same numeric
limits as `style.update`. The Compile screen's element editor now edits the
selected format's draft; the reset button re-seeds it from the project base.
The Settings workspace still owns the project-wide defaults. Export writers are
not yet reading these per-format styles — that is the next round.

## Round 10b — writers read the per-format styles

PDF, EPUB and HTML now take their type from `ExportSettings.styles`, not from the
old `settings.font`/`font_size`/`line_spacing` copies. PDF uses
`styles.body.font` for the prose face (F1/F2), `styles.body.size`/`.leading` for
prose lines, the title-page sizes for the title block, `chapter_heading.size`,
`.before` and `.after` for chapter openings, and each screenplay element's
`size`/`indent` for script lines. EPUB and HTML build their CSS from the same
element styles, including script font families and indents. DOCX and RTF still
read `settings.font`; they are unchanged. `settings.font` remains the seeded
copy used by those writers and by previews.

## Round 11 — Continuous vs Paged view

The Write toolbar now carries a small **Continuous / Paged** segmented control.
Continuous is the existing editable paper. Paged is a deliberate read-only
print preview: the document is split into discrete paper pages using the active
export target's `PaperSize`, `styles.body` size/leading and the same 54/44/34
inner margins as the continuous paper. It is the first version of page view;
editing across pages is future work.

Page breaks are estimates, in the spirit of Scrivener's page view. Prose is
split on blank lines, each paragraph is word-wrapped with a `size * 0.5`
character-width estimate, and lines are packed into the page's text height.
Scripts are counted at roughly 55 typed lines per page, with each block's
wrapped height and dialogue spacing accounted for. The prose paper width is the
format's paper width in millimetres scaled by 3 (A5 is about 444 × 630 egui
points), which keeps the preview readable without a real typesetter. Script
pages keep the paper width but grow tall enough for 55 lines.

Paged mode shows the document title at the top of the first page, left-aligned
prose, indented script elements, transitions on the right, and a centred page
number in each footer. The toolbar shows “Page N of M” for the page at the top
of the preview and the medium switcher is hidden because the preview is
read-only. The hint above the pages says "Read-only page preview — switch to
Continuous to edit". Limits: line lengths are estimated rather than measured by
the font engine, paged mode is not editable, and running headers are not drawn.

## Round 12 — offline spell-checking

The editors now share a small pure-Rust checker in
`crates/portmanteau-core/src/spell.rs`. It keeps a `HashSet<String>` built at
start-up from `assets/words.txt` (about 104,000 entries, trimmed and
lowercased). No Hunspell, network or new crate is involved.

Tokenising splits on anything that is not alphabetic but keeps internal
apostrophes and hyphens, so `don't` and `well-known` stay one token. Tokens with
no letters or a single letter are skipped, and repeated words are only reported
once, in first-seen order. `check_word` strips edge punctuation, accepts a
possessive when its stem is known (`cat's` -> `cat`), and accepts a plain plural
when its stem is known. Suggestions compute Levenshtein distance against the
whole dictionary, keep candidates within distance two, then sort by distance and
alphabetically and take the requested limit.

The Write toolbar shows `Spelling · N` when the selected document has
misspellings (`Spelling` when it does not). Clicking it opens a strip directly
below the toolbar and above the paper. Each misspelled word is a collapsible
row with its top three suggestions; clicking a suggestion replaces that token in
`text` or in the script `blocks` after the draw pass, following the borrow-safe
view pattern. A word with no close matches shows `(no suggestions)`. The
replace is case-insensitive and preserves a leading capital. Spell-checking can
be switched off with the new "Spell-checking in editors" checkbox in Settings;
the preference lives in the global `AppSettings`.

Limits: the suggestions scan the whole word list per open row and there is no
personal dictionary or "ignore" list yet. Hyphenated compounds are checked as a
single word, so a compound that is absent from the bundled list is reported even
when both halves are known.

## Round 13 — folders are containers

Folders still use `Document` with `kind == DocKind::Folder` in the model, but the
Write view no longer treats them as editable text. Selecting a folder now opens
a small folder panel instead of the paper or script surface: an editable name,
an editable synopsis, a clickable list of the documents inside (kind glyph,
title and word count for non-folders), and the quiet hint "Folders group
documents. Add content with the + button in the binder." The script/prose
toolbar (Snapshot, Focus, medium switcher, Continuous/Paged) is not drawn for a
folder. Name and synopsis edits are read into owned strings and applied through
`document_mut` after the draw pass, matching the borrow-safe view pattern.

Two smaller guards came with it: the binder no longer paints a word count on a
folder row (its kind glyph already says what it is), and "Take snapshot" is
absent from a folder's row context menu. The binder action handler also refuses
a folder snapshot defensively. No domain or command changes were needed.

## Round 14 — Word-like paragraph styles and a compact style editor

`ElementStyle` gained `first_line_indent`, `right_indent` and `align` (Left,
Center, Right, Justify). All three use `#[serde(default)]`, so projects saved
before the change still load: the first two default to 0 pt and alignment to
Left. The built-in body style now carries a 14 pt first-line indent, the usual
novel convention. `style.update` and `export.settings.update` accept the new
fields; `align` is parsed case-insensitively and an unknown word returns a clear
error instead of silently changing the style.

The PDF prose path now lays each line out inside the left indent/right indent
box: Left and Justify start at the left edge, Center is centred and Right ends at
the right edge. The first line of every paragraph adds `first_line_indent`, and
chapter headings honour their own alignment, so a centred heading is possible.
Script elements keep their fixed columns. EPUB and HTML CSS emit the same values
as `text-align` and `text-indent: <n>pt`, plus `margin-left`/`margin-right` from
the left and right indents. The old `.front p { text-indent: 0 }` rule was
removed because every XHTML body carries the `front` class, which would have
cancelled the new indent for all prose; only `p.first` opts out now.

The shared Compile/Settings editor is grouped into three collapsible sections
(Title page, Prose, Script). Each header carries a one-line summary such as
"Body — Liberation Serif · 11 pt · 1.4×". Font is a dropdown of bundled and
common families with the current custom value kept visible; each row shows size,
leading and alignment on the first line and the five spacing/indent values on
the second. Rows still edit an owned clone and write back after the draw pass.

## Round 15 — live formatting preview and custom styles

The Settings screen now opens with a **Live preview** panel. A dropdown picks
any built-in element or a writer-defined custom style, and the paper-coloured
frame below shows the standard sample copy in that element's current style.
The preview applies the chosen family through `family_for_font` (mono, sans or
serif based on the name), the point size, line spacing from `leading`, paragraph
alignment (Justify is drawn as Left), left and first-line indentation (the first
line of each paragraph gets the extra offset), right indentation, and the
before/after spacing. Spacing values are rounded to whole pixels so egui's
debug build does not paint its fractional-coordinate warning. The caption says
plainly that this is an on-screen approximation: the compiled page is the
source of truth and line breaking or page margins may differ.

`ElementStyles` gained a `custom: Vec<CustomStyle>` field, where each
`CustomStyle` holds a display name and an ordinary `ElementStyle`. The field is
`#[serde(default)]`, so global config files, project manifests and per-format
settings written before this round still load with an empty list. Because
`ElementStyles` is embedded in all three places, custom styles persist
everywhere and "Apply to current project" copies them with the rest of the
style data; `ElementStyles::custom_named` looks one up by name.

Settings shows a **Custom styles** collapsing section below the built-in
sections, inside its own small scroll region so the Add button stays visible.
Each row has an editable name, a Remove button and the same compact type and
paragraph controls as the built-ins. "Add custom style" creates "New style",
suffixing a number if that name already exists (or collides with a built-in).
Rows edit an owned clone and write back after the draw pass; add and remove are
applied after the list has finished drawing. Applying a custom style to a
particular manuscript paragraph is deliberately out of scope for this round: it
needs per-paragraph prose structure rather than the current per-document text.

## Round 16 — the corkboard becomes an editing surface

Cards are no longer read-only thumbnails. Each one is a real `Frame` with a
single-line title field (serif) and a multi-line synopsis field (sans), plus the
same word count, status dot and label accent as before. Edits are made on the
owned card copies the view already collects, then applied by id with
`Project::document_mut` after the draw pass, so the project is never mutably
borrowed while a card is painted. Each applied change touches the document and
marks the workspace dirty.

A single click now selects the card (and clicking into either text field selects
it too). A double-click opens it in the Write editor, as does the small "Write"
button on every card, which keeps the old click-to-write behaviour discoverable.
Changing selection does not switch views.

Cards are drag sources and drop targets using egui 0.31's payload API:
`Response::dnd_set_drag_payload` on the card, `Response::dnd_hover_payload` for
the hover highlight, and `Response::dnd_release_payload` for the drop. The card's
click/drag interaction is registered before its text fields, so the fields keep
their own clicks and drag-selection; dragging from the card's padding or footer
still starts the card drag. Dropping card A onto card B calls the new
`Project::move_after(A, B)`: A adopts B's parent, is removed from its old
sibling list and is inserted directly after B, with the destination siblings
renumbered 0, 1, 2… The move refuses self-drops, missing ids and moving a folder
inside its own subtree. This gives both within-part reordering and cross-part
reparenting without a separate gesture.

"+ New card" now creates the chapter (or scene) and stays on the corkboard. The
new card is selected, and a small piece of egui temporary memory asks the next
frame to put keyboard focus in its title field so the writer can name it at
once.

The empty-synopsis placeholder is presentation only (TextEdit hint text) and is
never written back, so blank synopses stay blank.

## Round 17 — Hunspell-backed spell-checking

Detection now comes from the bundled Hunspell en_US dictionary
(`assets/dict/en_US.aff` and `assets/dict/en_US.dic`) through `zspell` 0.5, a
pure-Rust Hunspell implementation. `SpellChecker::new` takes the `.aff` and
`.dic` contents plus the old `assets/words.txt` list; the dictionary is built
once at start-up in `src/spellcheck.rs`. Because `zspell` applies the affix
rules, real inflections are accepted (`running`, `bananas`, `cat's`) and
plausible non-words are still flagged (`runnning`, `bananaz`).

`misspellings` uses `Dictionary::check_indices`, so `zspell`'s Unicode word
splitter decides token boundaries. Contractions are left alone and hyphenated
compounds are split into parts that the dictionary can verify, which removes a
class of false positives the old list-based checker had. Unique words are still
reported once, in first-seen order.

Suggestions deliberately stay on the old Levenshtein code over `words.txt`:
`zspell`'s suggestion API is behind its unstable-suggestions feature, and the
existing suggester already matches the editor's three-suggestion strip. The
toolbar, replacement and Settings switch are unchanged.

Limits: building the dictionary adds a short fixed cost to start-up, and
suggestions can still differ from Hunspell's own ordering. Proper nouns and
invented fiction names still show as misspellings, as before; a personal
dictionary remains future work.

### Round 19 — Removed Develop, Command desk leaves the rail

The **Develop** workspace was removed, together with the idea cards and the
three-act beat sheet and their backing data (`Project.ideas`, `Project.beats`,
`IdeaCard`, `Beat`). Old project files that still contain `ideas` or `beats`
keys open fine because serde ignores unknown fields; saving drops them. The
`idea.add` / `idea.update` / `idea.delete` and `beat.update` commands, the
`add_idea` MCP tool and the Develop rail icon are gone too. The Binder's
"Story development" collection entry went with them.

The **Command desk** is no longer in the left navigation rail or `View::all()`.
It stays a real workspace and command surface, but is now opened from the View
menu ("Command desk") or with `PORTMANTEAU_VIEW=agent` (also `command desk`).
The Agent icon and dispatch remain in place.

### Round 20 — Script editing ergonomics

The screenplay surface now has three editing aids that stay local to the
focused element and never change the stored project until the draw pass ends.

- **Ctrl/Cmd+1…8** sets the focused element to the matching entry in
  `ElementKind::all(medium)`. The digit key is consumed before the text widget
  sees it, so no number is typed. Palette buttons show the same shortcut in
  their hover text (e.g. "Character · Ctrl+3").
- **Character autocomplete** appears under a focused Character element. It lists
  matching names from the database's Character records, filtered
  case-insensitively by the element text, plus a "New character…" entry. The
  name list is cloned before the document is borrowed. Choosing a name writes it
  into the block; the new-character entry opens a small Name/Role window, and
  creating adds the record and fills the block with the name.
- **Next speaker** is a faint `↺ NAME` hint at the right edge of an empty
  Character element. It comes from the two most recent distinct Character names
  before the cursor, so a MARA / BOY back-and-forth suggests whichever of the
  pair did not speak last. Clicking it fills the block.

Tab/Shift-Tab cycling, Enter for the next element, Backspace on an empty
element and normal arrow-key caret movement are unchanged. All project
mutations are collected as actions and applied after the `&mut doc` scope, the
same pattern the palette, Enter and Backspace already use.

### Round 21 — Corkboard zoom, grab handle and clearer drops

The corkboard header now has a small zoom control between the card count and
"+ New card": `-`, a percentage readout and `+`. The value lives on
`PortmanteauApp` as `corkboard_zoom` (default 1.0, clamped to 0.6–1.6) and
scales the target card width from a 230pt base. Column count is derived from
that width, so zooming out fits more cards per row and zooming in fits fewer;
the card height scales mildly with the same value (clamped 180–250pt) so the
title, synopsis and footer stay visible at both ends. The control only changes
view state, not the project, so it is not saved.

Every card now paints a small three-line grab handle at its top-left, before
the kind label. It is painted rather than drawn as a `≡` character so the
bundle's fonts cannot turn it into a box, and its hover hint is "Drag to
reorder". The whole card remains the drag source; the handle is purely an
affordance.

Drag feedback is stronger and easier to read. A target card keeps a thicker
3pt accent outline and gains a vertical accent insertion bar inside its right
edge — `move_after` always inserts after the target, so the right edge is the
only possible landing side. The card that started the drag fades to 40% while
the payload is live, using `egui::DragAndDrop::payload` so the source stays
dimmed even when the pointer is over a different card. Inline title/synopsis
editing, double-click and the "Write" button are unchanged.

### Round 22 — Editor zoom and distraction-free writing

The writing surface has its own zoom, separate from the corkboard's card zoom.
`PortmanteauApp::editor_zoom` defaults to 1.0 and is clamped to 0.7–1.6; the
second row of the editor toolbar holds a compact `−` / percentage / `+` group
with "Zoom out" and "Zoom in" hints. Prose multiplies `editor_font_size` by the
zoom (and the empty-page hint follows it); screenplay elements multiply their
per-element type size by the same value, and the scene-number gutter text
scales mildly. Column widths and indents are fractions of the content width
already, so they stay proportional — at 160% the text wraps more but the page
frame does not move. Zoom changes view state only and is not saved.

Distraction-free writing is a second, independent switch:
`PortmanteauApp::distraction_free` (default false). While it is on, `update`
draws only the editor's `CentralPanel` — no top bar, rail, binder, inspector,
status bar or toasts. The editor toolbar stays, so the writer can leave without
the keyboard; **F11** toggles the mode (it used to toggle focus mode) and
**Escape** leaves it. Entering also turns on `focus_mode`, which gives the
editor its wider 860pt page and 64pt side margins; leaving does not turn focus
mode off, so the two settings stay independent. The existing "Focus" toolbar
button and the View menu still toggle focus mode on their own.

### Round 24 — Characters become a story database

The Characters workspace is now **Database**. `Project.characters: Vec<Character>` became
`Project.items: Vec<DatabaseItem>`; a record carries a `kind` (Character, Place, Event,
Prop, Custom), a `title`, free `notes`, `tags`, `related` ids and a `color`. The four
seeded characters moved across with their role, description, want, need, flaw and arc
folded into labelled note sections. The command surface replaced `character.add` /
`character.update` with `item.add` / `item.update`; `stats` now reports `items` plus
per-kind counts, and the MCP tool is `add_item`.

The screen keeps the list/detail shape: records are grouped by kind on the left, each row
showing its title and colour dot, and the right pane edits kind, title, colour, tags,
notes and related links. A related item is a clickable chip that selects it, and the
"Link an item…" dropdown adds one. Character records still show computed manuscript
appearances by matching the title against every document's `body_text()`.

Old project files with a `characters` array load (serde ignores the unknown field) but
their records are not migrated; the new `items` field defaults to empty. The script
`ElementKind::Character` and its editor autocomplete are unchanged in meaning — the
autocomplete now reads Character records from the database and uses their titles.

### Round 25 — Research attachments and pasted HTML

Research notes can now hold files beside their text. `ResearchItem` gained
`attachments: Vec<String>` and `html: String`, both `#[serde(default)]` so older
project manifests still load. Only basenames are stored in `project.json`; the
bytes live in the project folder at `assets/research/<item-id>/<basename>`.
`save_folder` keeps its atomic temp-directory swap, so it now copies the
existing `assets/` tree into the new package before the rename — without that
step the swap would have deleted every attachment on save. `load_folder` does
not read `assets/`; the view reads a file only when it draws the row.

The Research screen has an Attachments block under the note body. Image
attachments (`.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`) preview inline from the
project bytes; audio, video, PDF and document attachments show their basename
and kind as local references (there is no in-app playback). The typed "Attach
file" path copies the chosen file into `assets/research/<item-id>/` and appends
its basename once; `×` removes the row and its copied file. An unsaved project
shows "Save the project to attach files" and refuses to attach, because there
is no folder to own the bytes.

"Paste HTML" reads clipboard text with `arboard`, stores it verbatim in
`item.html` and previews the raw source in a wrapped mono snippet. Any
`data:image/...;base64,...` URLs in that HTML are decoded with a small built-in
base64 decoder (no new dependency), written as `pasted-N.<ext>`, and appended
to the attachment list. "Paste image" asks `arboard` for an OS clipboard image
and writes it as RGBA `pasted-image-N.png`. Both clipboard buttons use
`arboard`, which already arrived through `egui-winit`; the Linux build works and
the image path is best-effort (a headless machine simply reports "No image on
clipboard"). Inline previews use `egui_extras::install_image_loaders` with the
`image` crate's png/jpeg/gif/webp decoders enabled.

Known limits: "Save As" to a different folder does not yet carry the old
`assets/` tree with it, and pasted HTML is shown as source rather than rendered
as a web page. Audio and video are references only by design.

Front and back matter are now driven by the binder, not by fixed titles. Every
`DocKind::FrontMatter` document in the Front Matter folder appears in the
compile output before the manuscript, and every `DocKind::BackMatter` document
in the Back Matter folder appears after it, each controlled by its own
`include_in_export` flag. The Compile screen lists those documents instead of
the old Copyright / Dedication / Author's note / Acknowledgements / About the
author checkboxes. The generated title page is still separate and controlled by
`include_title_page`.

This removes five obsolete `ExportSettings` booleans: `include_copyright`,
`include_dedication`, `include_preface`, `include_acknowledgements` and
`include_author_bio`. Old project manifests that contain them still load because
serde ignores unknown fields. The demonstration project no longer ships a
"Title Page" front-matter document; the generated title page covers that.

### Round 26b — Compile and Display merge

The separate Settings workspace is gone. Compile is now one screen with a
vertical tab rail: **PDF, EPUB, DOCX, FDX, Fountain, HTML, Markdown, RTF, TXT,
Bundle**, a spacer, then **Display**. The separate Settings view and its rail
icon were removed; `PORTMANTEAU_VIEW=settings` no longer selects a view. The
selected tab is view state (`PortmanteauApp::compile_tab`, default
`Format(Pdf)`), not part of the project.

Each output tab keeps the output path, compile action, last-export result,
per-format element styles and the "what will be compiled" list. Type-specific
parameters only show where they matter: paper for PDF/DOCX/RTF, cover image for
EPUB/HTML, and page numbers/running headers for PDF; font, size and leading
appear everywhere. Front and back matter document lists stay in two columns;
title-page and contents toggles are hidden for plain-text targets where they have
no effect.

The Display tab holds the editor measurements (`AppSettings.display`: prose and
script size/leading), the spell-check switch, the global element-style editor,
custom styles, the live preview picker, and Save defaults / Reset defaults /
Apply to current project. The prose editor uses `display.prose_size` and
`display.prose_leading` (multiplied by the existing zoom); the script editor uses
`display.script_size`. Custom styles now edit the model that is persisted:
the Display tab writes to `app.settings.styles.custom` before "Save defaults",
and each output tab writes to that format's `settings.styles.custom`, which is
stored through `Project::set_settings` and saved with the project.

The old saved-export-shortcut model was removed from the project (`Project` no
longer carries that list); old projects still load because serde ignores the
unknown key.

### Round 26c — Live compile preview

The paging estimator that powered the editor's read-only **Paged** view was
extracted from `src/ui/editor.rs` into a new shared module, `src/ui/page.rs`
(`PageLine`, `PagePlan`, `prose_plan`, `script_plan`, `page_frame`,
`paged_view` and the wrapping helpers). The editor keeps its Paged mode and
behaviour; it now imports the same code instead of owning it.

`page::manuscript_page_plan` pages the whole compiled manuscript: the generated
title page (title, subtitle and "by <author>" centered using the title-page
element styles), front matter, the draft (prose and script sections in binder
order), then back matter. Each section starts on its own page, matching the
export engine's per-section page breaks. `PageLine` gained a `center` flag so
the title page can be set centered.

The Compile output tabs now show a **Live preview** panel directly below the
compile action. It renders the manuscript pages side by side in a horizontal
scroller, with the usual paper frame and page-number footer, rebuilt every frame
from the selected format's draft settings. Changing paper, element styles or
margins updates it immediately. The caption is deliberate: this is an
approximation driven by the real compile settings, not the export engine's
output (wrapping uses a character-width estimate and the screen scale is about
1 mm → 3 points). The Display tab does not show it.

### Round 27a — Compile three-column layout and page navigation

The Compile screen's output tabs now use three columns inside the content frame.
The far-left rail keeps the output-format buttons (and Display). The middle
column (~400 pt wide, scrolling) holds the layout parameters: output path and
action, per-format element styles, page and type settings, the reset-to-base
row, the front/back matter lists and the "What will be compiled" order. The
right column (remaining width, scrolling) is the live preview. The Display tab
keeps its old single-column layout for now.

The preview is now a **single page**, not a sideways filmstrip. It has `‹ Prev`
/ `Page N of M` / `Next ›` controls, disabled at the ends, and a **Section**
picker that lists the title page, front matter, draft sections and back matter,
and jumps to the first page of the chosen section. `PortmanteauApp::preview_page`
is view state (0-based, default 0): it resets to the first page when an output
tab button is clicked and is clamped to the page count each frame, so narrowing
the paper or toggling the title page cannot leave it out of range.

`page.rs` gained `PreviewSection { label, page_count }` and
`ManuscriptPreview { plans, sections }`, produced by
`manuscript_preview(project, settings)`. The section labels are the document
titles, with "Title page" for the generated page and "Manuscript" for the
empty-project placeholder. `ManuscriptPreview::section_start` /
`section_starts` give cumulative page offsets. `manuscript_page_plan` was
folded into `manuscript_preview`; its one caller in `compile.rs` was updated.

### Round 27b — Settings returns; Compile loses the Display tab

Display settings moved back out of Compile into a dedicated **Settings**
workspace, pinned to the foot of the rail just above the avatar (a gear icon).
Settings is a tabbed screen: **Preferences** (spell-checking, with a muted
"more coming soon" note for future options) and **Display** (the old Compile
Display tab: live element proof, editor type sizes, default element styles and
custom styles, Save/Apply/Reset actions). The default tab is Preferences.

The Display tab also gained the manuscript preview near its top. The
single-page preview panel — `‹ Prev` / `Page N of M` / `Next ›`, section
picker, paper frame — is now a shared helper
`ui::views::manuscript_preview_navigator(project, settings, page, ui)` with
`section_for_page` alongside it. Compile's right column and Settings → Display
both call it; they keep independent page cursors (`app.preview_page` and
`app.settings_preview_page`), so turning pages in one does not move the other.
Settings feeds it `project.settings_for(ExportKind::Pdf)` as the representative
page.

Compile now has only the output-format tabs; `CompileTab::Display` was removed
and the screen is a straight three-column format view. The mobile/memory cost is
nil: the paging estimate is recomputed per frame exactly as before.

### Round 28a — Compile page fixes

The live preview in `ui::page` now honours the full element style, not just
size and leading. `PageLine` gained `font`, `align`, `first_line_indent`,
`right_indent` and `after`; `page_frame` picks the bundled serif/sans/mono
family with `theme::family_for_font` (falling back to the old `mono` flag when a
style has no family), aligns left/centre/right (Justify is drawn as left),
applies the first-line indent only to the opening wrapped line of a paragraph,
shrinks the text column by the right indent, and adds the style's `after` space
below the last line of a paragraph or script block. `prose_plan`,
`script_plan` and `title_page_plan` pass the matching style fields through, so
editing one element's font, alignment, indents or before/after spacing changes
that element in the preview only. Script transitions stay right-aligned; the
title page stays centred.

The element-style rows are now a four-column `egui::Grid` (label + control
pairs) inside each card, with the element name above it. Nothing wraps
mid-row at the ~400 pt middle column. The Prose and Script sections can be
greyed out: `element_styles_editor` takes `grey_prose` / `grey_script`, Compile
disables the group that cannot apply to the project medium (Novel → script
greyed; Film/Television/Theatre → prose greyed; Mixed → neither), and Settings
passes `false, false` because its defaults apply to every project.

"Where it goes" now has separate **Folder** and **Filename** fields. The
filename defaults (as a hint) to the export engine's `<slug>.<ext>` and is
stored per format in the new `ExportSettings::output_filename`. Compile passes
the joined folder + filename as the `export.run` path; a blank filename keeps
the old folder-only behaviour (and a blank folder still means `exports/`).

## Round 28b — header & footer controls

The Compile "Page & type" panel no longer carries global type-size and leading
DragValues; element styles own those, and paper size remains for print formats.
A new "Headers & footers" panel (PDF) edits the local `HeaderFooter` clone and
commits it after the draw pass. Page numbers gain format (Arabic/lower Roman/
upper Roman), location, alignment, edge spacing, first-page visibility and a
start number. The running header can use custom text or fall back to the book
title, with its own alignment and top spacing. `export.run` and
`export.settings.update` accept the same values as a nested `header_footer`
object and merge only the supplied fields.

## Round 29 — Settings mirrors Compile; editors use the default styles

The Settings → Display tab is now a two-column layout like Compile. The left
column is a single **Display and default styles** card: the built-in and custom
element-style editors, the Save defaults / Apply to current project / Reset
defaults buttons, and the config-file path. The right column is the shared
`manuscript_preview_navigator`, previewing the whole manuscript with the global
default styles rather than the current project's per-format settings. The old
"Editor display" size/leading grid, the small per-element live preview card and
the standalone Editing card are gone; the manuscript preview is the only
preview and the spell-checking switch lives on Preferences.

The Writing surface now draws from the same default element styles. Prose takes
its font from `styles.body.font`, its size from `styles.body.size` and its line
spacing from `styles.body.leading`. Screenplay elements look up the matching
`styles.<element>` through `page::script_element_style`, so font, size and
indent match the paged preview; the editor zoom still multiplies the size.
Paragraph alignment, first-line indent and right indent cannot be represented
inside a single continuous `TextEdit`, so those remain preview/Compile-only.

`DisplaySettings` and `AppSettings::display` were removed. Older config files
still load: serde ignores the now-unknown `display` object, and the editor
falls back to the built-in element styles.

## Round 30 — Tab owns the screenplay block; the character popup survives focus loss

Two keyboard/pointer fixes on the script editor.

**Tab and Shift+Tab change the element type only.** `script_page` already
consumed Tab and set `cycle_at`, but egui's focus navigation ran first: it reads
the focused widget's `EventFilter` at the start of each pass, and `TextEdit`
installs `tab: false` while it draws. So after each block is added,
`ui.memory_mut(|m| m.set_focus_lock_filter(block_id, EventFilter { tab: true,
horizontal_arrows: true, vertical_arrows: true, escape: false }))` is called
*after* `ui.add(edit)`; the filter is stored for the next pass, where Tab and
the arrow keys would otherwise move focus between widgets or blocks. Tab still
cycles the element kind once, the caret stays in the block, and `escape: false`
leaves Escape free to drop focus as before.

**The character autocomplete is no longer tied to text-field focus.** It used
to render only while the Character block had focus, so moving the pointer to a
name unfocused the field and unmounted the popup before the click landed. The
open popup is now view state: `PortmanteauApp::character_menu:
Option<(String, usize)>` holds the document id and block index. It is set when
a Character block is clicked or its text changes, drawn whenever it matches the
current block, and cleared when a name or "+ New character…" is chosen, on
Escape, and when a non-Character block is clicked. The "↺ next speaker" hint
lives in the same popup function, so it gains the same persistent behaviour.
Runtime click-testing still needs a real window; `--screenshot` only proves the
screen draws.

## Round 31 — All-caps becomes a style option; Shift+Tab cycles backward

**Casing moved from element kind to element style.** `ElementStyle` gained a
`uppercase` flag (`#[serde(default)]`, so older project files still load). The
built-in defaults turn it on for Character, Scene heading, Transition and Act,
and leave Action, Dialogue, Parenthetical, the title-page styles, prose and
General mixed-case. `ElementStyles::style_for_kind` now owns the element-kind
mapping that used to live in `page::script_element_style`; the page module keeps
a thin wrapper so the editor and preview share one table. Every element-style
row in Compile, Settings and the per-format editors has an "All caps"
checkbox, and `style.update` / `export.settings.update` accept
`{"character":{"uppercase":true}}` through the same merge helper as the numeric
fields.

The option is authoritative in all three places the script appears. The editor
uppercases a block's stored text when its style says so and the text changes
(ASCII names keep their length, so the caret stays put; combining characters or
Unicode case pairs could move it). The paged preview uppercases `PageLine.text`.
The PDF, EPUB, DOCX, FDX, HTML, Markdown and RTF writers branch on
`settings.styles.style_for_kind(block.kind).uppercase` instead of the old
`ElementKind::is_uppercase`; the EPUB and HTML stylesheets also make their
`text-transform` conditional so switching the option off really produces mixed
case. `portmanteau-core::script` keeps its kind-based helper because Fountain
and plain text are interchange formats, not the writer's house style.

**Shift+Tab now walks backward.** `script_page` used to collapse Tab and
Shift+Tab into the same `cycle_at: Option<usize>` and always call
`ElementKind::next`. It now carries `Option<(usize, bool)>`; plain Tab records
`true` and calls `next`, Shift+Tab records `false` and calls `prev`. The block
stays selected and focused as before.

## Round 32 — Headless UI tests for the script editor

The script-editor interactions that caused the recent bugs now have
deterministic tests that run under `cargo test` with no window, display or GPU.
`src/ui/editor.rs` has a `#[cfg(test)] mod ui_tests` that drives the real
`show()` through `egui::Context::run` with hand-built `RawInput` and `Event`s —
egui's own harness, so no new dependency. Four tests cover Tab / Shift+Tab
cycling, all-caps storage on a character cue, the character menu opening while
typing, and the pure `next_speaker` seesaw.

Two limits are deliberate. First, the tests assert on project/view state and on
which widget holds focus, not on pixels or accessibility labels: finding a
widget by its visible text would need accesskit (e.g. `egui_kittest`), which is
not a dependency, so the character menu is tested as state rather than by
clicking a name. Second, each test installs the bundled fonts and builds the
demo project; that keeps the suite independent of the writer's saved settings
but costs roughly half a second for all four tests.

The Tab test also pins down a real bug in the old code. `consume_key` ignores
extra Shift/Alt modifiers, so the plain-Tab check swallowed Shift+Tab and sent
both forwards; Shift+Tab now has to be matched first. Removing the
`set_focus_lock_filter` call makes the same test fail, because egui then moves
focus to the next widget instead of leaving Tab with the block.

## Round 33 — Mouse-click tests with egui_kittest

`egui_kittest` 0.31 is now a dev-dependency. Three click tests in
`src/ui/editor.rs` use AccessKit labels instead of pixel coordinates: clicking
a suggested character name fills the Character cue and closes the autocomplete;
clicking `+ New character…` opens the new-character modal; and clicking the
`Dialogue` palette button retypes the selected block. The app sits in an
`Rc<RefCell<_>>` because `Harness` owns the drawing closure across frames. One
quirk: `Context::set_fonts` applies at the next pass, so the harness closure
installs the bundled font families and skips drawing on its first frame.


## Round 34 — egui 0.37 migration

The desktop shell and tests moved from egui/eframe 0.31.1 to 0.37.0. This was a
dependency upgrade only: no screen, feature or visual token changed. The
mechanical API updates were:

- `eframe::App::update(ctx, frame)` is now `App::ui(ui, frame)`. The root `Ui`
  is handed to the app, and `egui::Panel::{top, bottom, left, right}` replaces
  `TopBottomPanel` / `SidePanel`; panels now show into a `Ui`, not a `Context`.
  `exact_width` / `exact_height` became `exact_size`.
- `TextEdit::frame(bool)` takes a `Frame`; `frame(false)` is now
  `frame(Frame::NONE)`. The custom prose layouter now receives a
  `&dyn TextBuffer` rather than `&str`, and font layout uses `fonts_mut`.
- Menus close with `ui.close()` instead of `ui.close_menu()`;
  `Response::horizontal_align` became `align(Align2)`.
- `Context::style` / `set_style` became the theme-aware
  `style_of` / `set_style_of`. The app pins `Theme::Dark` so the ink-and-paper
  palette is independent of the host OS theme.
- Screenshots are delivered by callback (`Context::request_screenshot`) rather
  than an `Event::Screenshot`; `--screenshot` still draws 120 frames, saves the
  PNG and closes the window.
- Headless tests call `Context::run_ui` (the `run` helper was removed) and
  explicitly clear the unapplied texture deltas. `egui_kittest`'s harness now
  builds a `Ui` closure (`build_ui`) and enables an accessibility check by
  default; the click tests opt out because the editor's medium `ComboBox` has no
  accessible name, exactly as it had none in 0.31.
- `Id::new` is deprecated for `Id::unique`, and the glow backend is an explicit
  eframe feature in 0.37 so `--screenshot` keeps using the same renderer.

`Cargo.lock` carries eframe, egui, egui_extras and egui_kittest 0.37.0, with no
0.31 copies left. Build, 93 tests, all seven workspace screenshots and the CLI
stats/PDF export were re-run after the upgrade.

## Round 35 — egui_spellcheck trial (prose only)

The vendored `vendor/egui_spellcheck` crate (0.36.3 with its `egui` dependency
bumped to 0.37) is wired into the prose surface as a trial. `main` calls
`egui_spellcheck::prewarm()` once before `eframe::run_native`; on native targets
that spawns a short-lived background thread which builds the dictionary engine,
so start-up does not block and the first debounced check is fast. In
`src/ui/editor.rs`, `prose_page` now draws
`SpellCheckTextEdit::multiline(&mut doc.text)` when Settings → "Spell-checking
in editors" is on, and keeps the old plain `TextEdit` when it is off. The crate's
`show(ui)` returns `TextEditOutput`; `output.response.changed()` is what marks
the document touched and the workspace dirty, the same rule as before. The
crate supports only `multiline` / `singleline`, `hint_text` (plain `String`),
`desired_width`, `desired_rows`, `font`, `id_salt`, `interactive` and `show`.

Visible differences and limits of the trial:

- The crate owns its layouter, so the builder has no `text_color`, `frame`,
  `margin` or custom-layouter method. The paper look is kept by scoping
  `override_text_color`, `text_edit_bg_color` and the widget strokes to
  `PAPER` / `PAPER_TEXT` around just this widget. The project's body
  `line_height`/leading is therefore not applied on the spell-checked path;
  lines use egui's default spacing, which is slightly tighter. The hint text is
  also a plain string (sans, weak text colour) rather than a serif `RichText`.
- Spelling spans come from the crate's own `spellbook` dictionary, not the
  existing zspell `SpellChecker`. Both engines are compiled in for now: the
  toolbar's **Spelling · n** panel and the suggestion buttons still use zspell,
  so the two can disagree until the panel is retired or re-pointed.
- The crate's personal dictionary ("Add to dictionary") lives in egui's
  persisted memory. The desktop shell does not enable eframe's `persistence`
  feature and implements no `App::save`, so words added there survive the
  session but are lost on restart. Moving to the crate's dictionary for good
  means either enabling eframe persistence or writing the word list into the
  project file.
- The script editor is deliberately untouched: its per-block `TextEdit::id`,
  focus hand-off and Tab/Enter element logic do not map onto the crate's
  single-widget API, so screenplay blocks keep the existing plain editor. There
  is no per-block integration yet.
- Underlines appear after the crate's short typing debounce; the screenshot at
  frame 120 already shows them (for example "harbour", "licence").

