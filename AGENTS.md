# AGENTS.md — working on Portmanteau

This is the single handoff file for the next developer and for AI coding agents
(`CLAUDE.md` only points here). It is kept under the standard uppercase name so
tooling finds it; a lowercase copy would collide on macOS and Windows. Read it
before changing anything: the product, the rules that are not negotiable, how
things are now, the architecture, the command surface, how to verify a change,
and the traps that have already caused bugs.

The owner is not a programmer: keep the app runnable at every commit and
explain changes in plain language.

Companion documents:

| File | What it answers |
| --- | --- |
| `README.md` | What the product is, how to run it, what works today |
| `docs/architecture.md` | Why the code is split the way it is |
| `docs/notes.md` | Running build log and the visual decisions behind the screens |
| `docs/roadmap.md` | Prioritised next work, with entry points |
| `docs/localization.md` | Interface languages: using, translating, adding text |
| `docs/releasing.md`, `docs/release-and-updates-plan.md` | Releases, updates, the web build |
| `docs/wiki-draft/` | Drafts of the GitHub wiki pages (the user documentation) |

---

## 1. The product in one paragraph

Portmanteau is a native, offline writing studio for novelists and playwrights
and screenwriters. It covers the core of Scrivener, Final Draft and WriterDuet:
a binder and outline, prose and screenplay editors, corkboard, research, idea
development, character sheets, story beats, version history, and compilation to
professional formats. It is built as a modular Rust workspace with a
command-driven core, an MCP server and a CLI, and an egui desktop shell on top.
The desktop layer is a view onto the command API, not a second implementation.

The person who commissioned it cares about:

1. **Solo writing first** — long-form structure, prose and script formatting,
   research and idea development.
2. **Versioning** — named snapshots, whole-project checkpoints, comparison,
   restore, and an audit trail around export.
3. **Many real output formats** — PDF, EPUB, DOCX, Final Draft (FDX), Fountain,
   HTML, Markdown, plain text and a portable project bundle — each with
   front matter, back matter and page/type settings.
4. **Agent-testable internals** — every important action is a JSON command, also
   exposed through MCP, so a UI can be replaced or automated.
5. **Modular structure from day one** — new formats, commands, views and
   clients should be additive, not rewrites.
6. **Local-only for now** — no accounts, server, network transport, presence or
   live co-editing. Collaboration and shared review are future work and were
   explicitly deferred by the person.

The person is not a programmer. Keep the product runnable at every commit, keep
notes understandable to a non-coder, and never leave a half-wired screen.

---

---

## 2. Non-negotiable rules

Do not trade these away to make a build or a check pass.


- Rust + eframe/egui, native desktop first; a WebAssembly build of the same app is planned (`docs/release-and-updates-plan.md`) and must not make desktop worse. No network, accounts, telemetry, background daemons or long-running processes (never start dev servers/watchers). **One exception** (owner, 2026-11-22): the update check and update download, which talk only to GitHub, send nothing about the writer or projects, and can be switched off.
- Command layer is authoritative: new behaviour is a command (API arm + catalogue entry + MCP tool + README row + test) before or alongside UI. No domain logic in widgets.
- `--screenshot` must keep working; a blank PNG is a failure.
- **No backwards compatibility, ever, until the owner says otherwise.** The project has no users; any change may be breaking. Don't add migrations, aliases or compatibility shims.
- No collaboration features unless the owner asks.
- Don't run `cargo fmt` repo-wide; format only files you touched.
- Don't commit `target/`, `shots/`, `exports/`.
- **Every visible text goes through `tr()`.** Any new or changed user-facing text (menus, buttons, labels, messages, the welcome project in `sample.rs`) must use `tr("English")` / `trf(...)`, and the language template must be regenerated: `UPDATE_LANGUAGE_TEMPLATE=1 cargo test the_template`, then rebuild. Changing an English text changes its key, so update (or drop) its entry in `assets/languages/*.json` too, or the translation silently stops applying. Details in `docs/localization.md`.
- **Keep the docs in step.** Any new feature or major change must also be written up in the GitHub wiki (draft pages live in `docs/wiki-draft/`, pushed to the wiki repo) and, if it changes what the product is or how to run it, in `README.md`.
- Commit small and often, imperative messages. Commit only when asked.
- **Do not copy brand assets or long copyrighted text.** The demonstration
  project is original sample copy; keep it that way.
- **Edit only inside `app/`** (the parent folder holds tooling and evidence),
  except `notes/` in the parent.

---

## 3. Quick start

```sh
cd app

cargo build                     # desktop binary + workspace crates
cargo run                       # normal launch
cargo test --workspace          # unit tests

./target/debug/portmanteau --screenshot shots/now.png
PORTMANTEAU_VIEW=corkboard ./target/debug/portmanteau --screenshot shots/corkboard.png
PORTMANTEAU_DOC="Cold Harbor" ./target/debug/portmanteau --screenshot shots/script.png
PORTMANTEAU_DIFF=1 ./target/debug/portmanteau --screenshot shots/diff.png
```

`PORTMANTEAU_VIEW` accepts `write`, `corkboard`, `outline`, `research`,
`database`, `history`, `compile`, `settings`, `agent` (or a view's visible
label, case-insensitive). `PORTMANTEAU_DOC` matches a substring of a document
title. `PORTMANTEAU_DIFF` opens the first available snapshot comparison.

Build outputs, screenshots, exports and local project files are git-ignored.
Do not commit `target/`, `shots/`, `exports/` or `*.portmanteau.json`.

---


```sh
cargo build --workspace
cargo test --workspace                     # ~193 tests, all passing at handoff
./target/debug/portmanteau --screenshot shots/now.png     # must always work; look at the PNG
PORTMANTEAU_VIEW=compile ./target/debug/portmanteau --screenshot shots/compile.png
cargo run -p portmanteau-cli -- commands
```

Export tests call `pdfinfo`/`pdffonts`/`pdftotext` (poppler) and skip if absent.

Dev aids for `--screenshot`: `PORTMANTEAU_THEME`, `PORTMANTEAU_LANG`, `PORTMANTEAU_LANGUAGE_PICKER`, `PORTMANTEAU_PROJECT` (`welcome` shows the welcome project), `PORTMANTEAU_SETTINGS_TAB`, `PORTMANTEAU_PAGED`, `PORTMANTEAU_COMPILE_SECTION`, `PORTMANTEAU_FIND`, `PORTMANTEAU_SEARCH`. Screenshot runs never write the writer's config.

---

## 4. How things are now

This section supersedes anything older in this file. If a later section seems
to disagree with it, this section wins.

### Layout

Repo root is `app/` (own `.git`, branch `master`). The parent folder `myScript/` holds only tooling and evidence: `.opencode/agents/` (old OpenCode conductor/implementer/tester team), `.kilo/`, `notes/next_tasks.md`, `shots/`, `exports/`. Edit inside `app/`. Crates: `portmanteau-core` (model, no I/O) <- `-export` (one module per format over shared `layout`, `web` and `content` modules; PDF via krilla) and `-import` (Fountain, FDX, DOCX, plain text) <- `-api` (command bus, persistence) <- `-mcp`/`-cli`, plus the egui shell in `src/`.

### Corrections to the original design

- **Visual language is now light**, not dark: warm-gray chrome, soft-white paper (`src/theme.rs`, `docs/ui-style-guidelines.md`). Ignore the "dark ink-and-paper" rule. A dark theme is optional future work on the same tokens.
- **Persistence is a project folder package** (small files, rich bodies as `.rich.json`, Fountain with `/* @kind */` markers, atomic temp-and-rename save), not one `.portmanteau.json`. Legacy `.json` support was dropped.
- **Content model is `RichDocument`** (styled paragraphs, inline marks, images) shared by prose, scripts and long-form fields; the old text/blocks split is gone. Projects own editable style sheets; program defaults are copied into new projects.
- Settings is a workspace (global defaults + per-project overrides). Per-format, per-element export styles exist (`Project.formats`, `ElementStyles`); PDF/EPUB/HTML consume them.
- Compile = one manuscript type per export (Prose/Screenplay/TV/Stage), scope Draft or subtree, optional one file per script. Default paper is US Letter; script geometry scales with paper width/612pt.
- Binder has a single "Draft" root; Script Lab was removed. Import supports Fountain, FDX, DOCX, plain text.
- Tests use `egui_kittest` for UI; the OpenCode conductor's rule was "UI bugs get a failing kittest first". Keep that habit.

- **Themes** (`src/theme/`): colours are a palette (`palette.rs`, theme JSON in
  the `themes` folder beside the settings, editable on Settings › Appearance).
  Print previews stay white (`PRINT_PAPER`/`PRINT_INK`).
- **Interface language** (`src/i18n.rs`, `assets/languages/`): every visible
  text is `tr("English")`; built-in French and Spanish; user files in the
  `languages` folder; the welcome project is translated through
  `sample::welcome_with`; the first launch detects the system language;
  the spelling dictionary follows the language. Read `docs/localization.md`.
- **There is no backwards compatibility** (see the rules): ignore any older text
  here about aliases or migrations.

### State at handoff (2026-11-21, plus later work noted inline)

Everything below is on `master`; build is green with no warnings.

Done this session:
- **Exporters** are one module per format over shared `layout` (margins, paper,
  script scale), `web` (the HTML/EPUB stylesheet generator), `content` and
  `util`. Every writer reads styles from `ElementStyle` and margins from
  `layout::page_margins`; no writer owns a size, indent or margin constant.
  `ExportSettings.margins` (None = automatic) has a Custom margins control in
  Compile. Cross-format tests check included documents, the title-page and
  Contents switches, and scene numbering for every writer.
- **Importers** (`portmanteau-import`): Fountain, FDX, DOCX, plain text, with a
  shared `Imported` result and warnings, encoding detection, chapter splitting
  and a zip-bomb guard. Round-trip tests run through our own exporters.
- **Undo/redo** (`src/history.rs`): watches whatever rich text is being edited
  (a document's body/synopsis/notes, a research note, a database item's notes) via
  the `Tracked` trait; steps are changed-paragraph windows, typing merges until a
  0.7 s pause. Plain fields (titles, paths) keep egui's own Cmd+Z: the app only
  handles it when a rich field (registered through `remember_selection_anchor`)
  or nothing has focus.
- **Spell-check** now covers every prose field.
- **UI cleanup**: dead or repeated controls removed, each with a comment on
  what it should become; a shell test keeps them out.
- Bugs found by looking at output: PDF ignored Justify and always centred the
  title page; EPUB referenced fonts it never packaged; DOCX/TXT ignored
  the Contents switch; only the PDF numbered scenes; the Fountain export was
  not valid Fountain (blank line between cue and dialogue).

How to check output visually (all on this machine):
- PDF: `pdftoppm -png`, and `pdftotext -bbox-layout` to measure positions.
- EPUB / HTML: `ebook-convert x.epub x.pdf`, or `chromium --headless --print-to-pdf`,
  then `pdftoppm`. Calibre rescales type, so trust it for structure, not size.
- **DOCX: `wordcraft-cli`** (the WordCraft app). `render f out.png --page N`,
  `info f` (page count), `inspect f` (parsed paragraph properties as JSON),
  `convert f out.pdf --strict` (fails on undrawable characters or missing pictures).
  LibreOffice does not run in this sandbox. When geometry matters, compare via
  `convert ... out.pdf` and `pdftotext -bbox-layout`.
- Rebuild with `cargo build --workspace`: a plain `cargo build` leaves the CLI stale.

- **Research** (`src/ui/views/research.rs`): a research note is an ordinary binder
  document in the Research folder carrying `Document.research: ResearchMeta` (kind,
  source, attachments); there is no separate research list any more. Commands:
  `research.add/list/update/attach/detach/delete`. Attachment paths are checked in
  the API (`checked_research_dir`, `research_file_path`): the folder must be exactly
  `<project>/assets/research/<id>` as a real directory, names must be bare file
  names (dropped on load otherwise), and a symlink anywhere is refused. The UI
  never builds attachment paths itself.
- **Links** (`TextRun.link`): a web address or `note:<document id>`. Ctrl/Cmd+K
  or the popup's Link button edits one; Ctrl/Cmd+click follows it (`App::open_link`;
  only http/https ever reach the system opener; email links are deliberately unsupported); pasting text links the
  addresses in it; HTML and Markdown exports write anchors (hrefs percent-encoded,
  `export_href`); DOCX/PDF print the text only.
- **Fixed formatting toolbar** (`src/ui/rich_toolbar.rs`): `show(ui, field_id,
  &mut content, &Options{styles})` above any rich `TextEdit`, plus `after_edit(...)`
  after the edited text is synced back. Works on the selection, or holds "pending
  marks" for the next typing at the caret; shows which marks are active; paragraph
  style dropdown; Link button; Ctrl/Cmd+B/I/U. Used on research notes only for now;
  the other fields still use the floating popup (`inline_format_toolbar`). Both
  share the link dialog (`link_support`). Lists (bullets, numbering) were
  considered and set aside: the model has no list concept.
- **Pictures in text**: Insert > Image… (with a Browse button) puts one at the caret;
  Ctrl/Cmd+V with a picture on the clipboard pastes it at the caret (egui only passes
  text through a paste, so `paste_image_shortcut` watches the key); dropped picture
  files are inserted in Write (attached in Research). All keep the file in
  `assets/images` and need a saved project. Headings (chapter heading and any style
  named "heading") draw bold in the editor, as every export sets them.
- **Hyperlinks in exports**: `ExportSettings.include_hyperlinks` (default on),
  offered for HTML, EPUB, Markdown and DOCX (`ExportKind::writes_hyperlinks`);
  off strips links in `prose_paragraphs`, so no writer needs its own switch. PDF
  does not write links yet (would need link annotations).
- **Database page** (`ui/views/database.rs`): items have a kind, colour, tags and
  rich notes (the fixed toolbar is on the notes). The old "Related" list is gone:
  link items by typing `@` in notes (`ui/mention.rs`, also on research notes; a
  link `item:<id>` or `note:<id>`, followed with Ctrl/Cmd+click). "Linked from"
  and "Appearances" are derived (`Project::items_linking_to`, `appearances_of`):
  appearances are every document naming or linking the item (a character is also
  found by first name), and clicking one opens it at the first place
  (`App::open_appearance`, via `CaretRequest`). Tags: search, tag chips and tags
  shown in the list, "used elsewhere" suggestions on the item, and `@` matches tags.
- **Binder** no longer lists the Research folder; research is reached from the
  "Research notes" entry under Collections (and the Research page).
- **Find and replace** (`portmanteau-core/src/find.rs`; commands `search.find` /
  `search.replace`; MCP `search_project`, `replace_text`): one engine with plain,
  case, whole-word and regex options, groups (Manuscript, Front & back matter,
  Research, Database) and a choice of parts (text, synopsis, notes). Replacing snapshots
  each changed document first (not for a single-match replace). Two UIs: the
  **Search page** (`views/search.rs`, rail icon, Ctrl/Cmd+Shift+F; results open the
  document with the match selected via `CaretRequest.select`) and the editor's
  **find bar** (`ui/find_bar.rs`, Ctrl/Cmd+F, Ctrl/Cmd+H with Replace; Write view,
  Continuous mode only; shades matches via `paint_matches`). The top-bar search
  field hands its text to the Search page on Enter. Not done: replace undo in the
  editor's own history (use Versions), search inside the Paged view.
- New documents are created in the selected document's folder, else the top of
  the Draft (`Project::new_document_home`). The binder no longer lists Research.
- **Project page** (`src/ui/views/project.rs`, rail icon between Versions and
  Compile): title, subtitle, author, genre, logline, contact, draft label and
  date, daily goal. Edits go through `project.update`. A script cover prints
  contact bottom-left and the draft line bottom-right (`script_title.rs`); a
  blank draft date means the day of export. Prose title pages do not print
  contact yet (a copyright page would).

### Next work (priority order)

0. **Releases, updates and web build** — the current major item. Follow
   `docs/release-and-updates-plan.md` phase by phase. Done (2026-11-22, not yet
   pushed or run on GitHub): 0 groundwork + CI, 1 release workflow, 2 update
   notice (`crates/portmanteau-update`, `src/update.rs`), 3 in-app install
   (minisign-signed `SHA256SUMS`; `install.rs`; key in `update-public-key.pub`,
   private key is the GitHub secret `UPDATE_SIGNING_KEY`), and the welcome project
   (first launch and fallback). Phase 4 (web) started: libraries build for wasm (CI
   job), storage split into pure `package_files`/`project_from_files`, project zip
   commands. Next: W4 shell split, then the real two-release update test in
   `docs/releasing.md`.

1. ~~Preview pagination~~ Done: Compile, Settings and the editor's Paged view all
   draw `pdf_layout` pages (`src/ui/preview.rs`); the old estimator is deleted.
   Paged shows a document's pages of the real compile with real page numbers.
   Page counts everywhere (inspector, paper header, Compile list, API) come from
   `portmanteau_export::page_counts`, refreshed a moment after typing pauses.
   The Paged view is read-only by decision; clicking a paragraph in Paged opens it for editing
   in Continuous. Page markers in the editor were tried and removed (docs/roadmap.md).
2. ~~PDF page per script document~~ Done: scenes run on; only a chapter-level
   document (episode/act) starts a page (`content::starts_new_page`, all writers).
3. ~~Import review~~ Done (`src/ui/import_review.rs`, `document.import_preview`):
   documents, title-page details and Word/Final Draft style mappings are shown and
   editable before anything is added; the split, details choice and style maps are
   remembered in the config (`AppSettings.import`). See `docs/roadmap.md` for
   the remaining import gaps.
4. Per-document compile overrides and a "compile with last settings" action.
5. Undo for binder operations, research and database text.
6. Drag-and-drop binder, native file pickers (`rfd`), word-level diff, FDX
   image insertion before first save, packaging. Also: DOCX import of hyperlinks,
   hyperlinks in DOCX/PDF export, research tags.

### Working style

Plan briefly, change small, run build + tests + a screenshot of any screen touched, and report what was verified versus unverified. Keep `docs/notes.md` as the running build log and update `docs/roadmap.md` when priorities shift.

---

## 5. Repository map

```
app/
  Cargo.toml                 workspace + portmanteau binary + shared dependencies
  Cargo.lock                 committed
  src/
    main.rs                  entry point; --screenshot parsing; eframe launch
    app.rs                   shell state, top bar, rail, binder/inspector panels,
                             menus, shortcuts, status bar, modals, screenshot tick
    theme.rs                 palette, font installation, shared helpers
    icons.rs                 painter-drawn navigation glyphs (no image assets)
    ui/
      mod.rs                 re-exports
      binder.rs              binder tree, row context menu, move actions, target
      editor.rs              paper editor; prose and typed screenplay surfaces
      inspector.rs           document metadata, notes, versions tabs
      views/
        mod.rs               shared page chrome (page_head, content_frame, …)
        corkboard.rs         chapter/scene cards grouped by part
        outline.rs           table of labels, status, length, include flags
        research.rs          sourcebook list + linked note editor
        database.rs          story database: characters, places, events, props
        settings.rs          global defaults, style sheets, preferences
        history.rs           checkpoints, snapshots, compare, fourteen-day chart
        compile.rs           format cards, settings, preview, export action
        agent.rs             Command desk (searchable JSON console)
  crates/
    portmanteau-core/
      src/lib.rs             crate exports
      src/model.rs           Project, Document, Block, Snapshot, Checkpoint,
                             Character, ResearchItem, IdeaCard, Beat,
                             ExportKind, ExportSettings, PaperSize
      src/compile.rs         CompileType, CompileScope, Project::compile_plan
      src/script.rs          ElementKind, loose-text classification, Fountain
      src/text.rs            word counts, reading time, line diff
      src/sample.rs          original demonstration project, "The Salt Road"
    portmanteau-export/src/lib.rs    section assembly + ten writers, no UI dependencies
    portmanteau-api/src/lib.rs       Workspace, JSON command bus, persistence, tests
    portmanteau-mcp/src/lib.rs       MCP tool definitions and stdio JSON-RPC server
    portmanteau-mcp/src/main.rs      portmanteau-mcp entry point
    portmanteau-cli/src/main.rs      one-command CLI client for smoke tests
  assets/                    bundled Liberation + DejaVu fonts
  docs/                      architecture, notes, roadmap
  README.md                  product-facing documentation
  AGENTS.md                  this file
```

Dependency direction is one-way:

```
portmanteau-core  <-  portmanteau-export
    ^             ^
    |             |
  portmanteau-api  <----  portmanteau-mcp / portmanteau-cli
    ^
  desktop UI (portmanteau)
```

`portmanteau-core` has no filesystem, UI or export code. `portmanteau-export` knows the
manuscript but not the window. `portmanteau-api` owns persistence and the command
surface. The desktop layer only holds view state and calls the API.

---

## 6. Domain model essentials

- **Project** — `schema`, identity, title/author/genre/logline, `kind`
  (Novel, Film, Television, Theatre, Mixed), flat `documents`, `characters`,
  `research`, `ideas`, `beats`, `checkpoints`, `session_log`, `daily_target`,
  `export` settings and `presets`.
- **Document** — `id`, optional `parent`, integer `order`, `title`, `kind`,
  `medium`, `text`, `blocks`, `synopsis`, `notes`, `label`, `status`,
  `include_in_export`, `word_target`, timestamps, `snapshots`, `pinned`.
  - Prose lives in `text`; scripts live in `blocks`. `medium.is_script()`
    decides which is authoritative.
  - `convert_medium` parses prose into typed blocks and joins blocks back into
    prose, so one document model serves both media.
  - `body_text()` is the authoritative flattened text for search, counts, diff
    and export section assembly.
- **Hierarchy** — a flat list with `parent` + `order`, reconstructed by
  `children` and `ordered_documents`. Moving a document up/down swaps `order`
  with its neighbour; reparenting appends at the end of the target folder and
  refuses cycles.
- **Snapshot** — named copy of one document's `text` and `blocks`, capped at 60
  per document. `Snapshot::body_text()` reconstructs the saved body for diff.
- **Checkpoint** — serialised whole project (`payload`), labelled, capped at 30.
  Restore deliberately keeps the existing checkpoint list.
- **Export settings** are model data, not screen state: format, paper size,
  font, type size, leading, front matter flags (title page, copyright,
  dedication, author's note, contents, chapter numbers), body/back matter flags
  (scene numbers, synopsis, notes, acknowledgements, author bio) and output path.

Project files are pretty-printed JSON with the extension
`.portmanteau.json`. `schema` is currently `1`. Unknown fields are ignored by
serde, which is how old files survive model changes. If you remove a field,
mention it in `docs/notes.md`; if you rename one, add a migration or keep a
serde alias.

---

## 7. Command API

`Workspace::execute(command, payload)` always returns:

```json
{ "ok": true, "command": "document.get", "message": "Document", "data": { } }
```

The catalogue lives in `command_catalogue()` in `crates/portmanteau-api/src/lib.rs`.
`commands` returns it; the Command desk and MCP examples read the same source.
When you add a command, update **all** of these:

1. the match arm in `Workspace::execute`;
2. the catalogue entry;
3. an MCP tool in `crates/portmanteau-mcp/src/lib.rs` if agents should use it;
4. the command table in `README.md`;
5. a test in `crates/portmanteau-api/src/lib.rs` when it has real logic.

| Command | Purpose | Example payload |
| --- | --- | --- |
| `project.info` | Summary of the open project | `{}` |
| `project.update` | Change title, subtitle, author, genre, logline, contact, draft_label, draft_date, daily_target | `{"contact":"Ada\nada@example.com","draft_date":"14 March 2027"}` |
| `project.set_cover` | Set the cover picture (.png/.jpg; a saved project keeps a copy) or clear it with an empty path | `{"path":"/tmp/cover.jpg"}` |
| `research.list` | List research notes (kind, source, words, attachments) | `{}` |
| `document.picture` | Change a picture's size, alignment, wrapping, padding, caption, or remove it (`"remove":true`) | `{"id":"doc-…","paragraph":3,"align":"right","wrap":true}` |
| `research.update` | Change a note's title, body, kind or source | `{"id":"doc-…","kind":"Interview"}` |
| `research.attach` | Copy a file into a note's attachments (project must be saved) | `{"id":"doc-…","path":"/tmp/x.pdf"}` |
| `research.detach` | Remove an attachment | `{"id":"doc-…","name":"x.pdf"}` |
| `document.import_preview` | Read an import file without changing anything: documents, paragraph styles and targets, details, warnings | `{"path":"/tmp/a.docx","style_map":{"Quote":"style:body"}}` |
| `project.new` | Create a project | `{"title":"…","kind":"Novel"}` |
| `project.open` | Open a project file | `{"path":"…"}` |
| `project.save` | Save (optionally as) | `{"path":"…"}` |
| `project.update` | Title, author, genre, logline, daily target | `{"title":"…"}` |
| `project.checkpoint` | Whole-project version | `{"label":"…"}` |
| `project.restore_checkpoint` | Restore a checkpoint | `{"id":"cp-…"}` |
| `project.snapshots` | List every document snapshot | `{}` |
| `document.list` | Flat binder list with counts | `{}` |
| `document.get` | One document, including snapshots | `{"id":"doc-…"}` |
| `document.create` | Add a document | `{"parent":"doc-…","title":"…","kind":"Chapter","medium":"Prose"}` |
| `document.update` | Title, text, metadata, medium | `{"id":"doc-…","text":"…"}` |
| `document.delete` | Delete a subtree | `{"id":"doc-…"}` |
| `document.move` | Reorder or reparent | `{"id":"doc-…","direction":"up"}` or `{"id":"doc-…","parent":"doc-…"}` |
| `document.snapshot` | Save a document version | `{"id":"doc-…","name":"…"}` |
| `snapshot.restore` | Restore a document version | `{"document_id":"…","id":"snap-…"}` |
| `snapshot.diff` | Compare snapshot with current | `{"document_id":"…","id":"snap-…"}` |
| `character.add` | Add a character sheet | `{"name":"…","role":"…"}` |
| `character.update` | Edit a character | `{"id":"char-…","want":"…"}` |
| `research.add` | Research note + binder entry | `{"title":"…","body":"…"}` |
| `idea.add` / `idea.update` / `idea.delete` | Idea board | `{"title":"…","body":"…"}` |
| `beat.update` | Tick or rewrite a beat | `{"id":"beat-…","done":true}` |
| `search.find` | Find text (plain/whole word/regex; groups Manuscript, Matter, Research, Database) | `{"query":"…","regex":false,"groups":["Research"]}` |
| `search.replace` | Replace matches, optionally in one document/item; snapshots first | `{"query":"…","replacement":"…","id":"doc-…","field":"Text","start":12}` (`field` alone: every match in that field; `start` picks one) |
| `stats` | Words, pages, scenes, log | `{}` |
| `stats.activity` | Writing activity: typing and reading time, words added/edited/deleted by day, week, hour of day and document; goal progress, streaks, badges | `{}` |
| `stats.analysis` | How the prose reads (sentence length, readability, dialogue share, -ly words, top and echoed words) for a document or the manuscript | `{"id":"doc-…"}` |
| `stats.record` | Note work done in an hour (the app calls this as the writer works; reading time alone does not mark the project unsaved) | `{"hour":"2026-11-26T14","doc":"doc-…","typing_secs":120,"added":80}` |
| `script.parse` | Loose text → typed blocks + Fountain | `{"text":"INT. …"}` |
| `export.preview` | What a compile would contain: included and skipped documents for one manuscript type | `{"compile_type":"screenplay","scope":"draft"}` |
| `export.formats` | Supported formats | `{}` |
| `export.run` | Write a real file | `{"format":"PDF","path":"exports"}` |
| `commands` | The catalogue | `{}` |

Notes:

- `export.run` accepts a folder path (no extension) and joins the standard
  filename, or a full file path. If the parent folder does not exist it is
  created. Export always writes an `export` checkpoint first.
- `document.update` with `text` re-parses blocks for script media.
- `document.move` without `parent` requires `direction` (`up` or `down`); with a
  `parent` it reparents to the end of that folder. `parent: null` moves to root.
- `snapshot.diff` returns `added`, `removed`, word counts and `lines`, where each
  line has `kind` (`same` / `add` / `remove`), `text`, `old_line`, `new_line`.

### CLI

```sh
cargo run -p portmanteau-cli -- stats
cargo run -p portmanteau-cli -- document.list
cargo run -p portmanteau-cli -- document.move '{"id":"doc-…","direction":"up"}'
cargo run -p portmanteau-cli -- snapshot.diff '{"document_id":"doc-…","id":"snap-…"}'
cargo run -p portmanteau-cli -- export.run '{"format":"EPUB","path":"exports"}'
cargo run -p portmanteau-cli -- project.info --project /path/to/book.portmanteau.json
cargo run -p portmanteau-cli -- document.move '{"id":"doc-…","direction":"up"}' \
  --project /path/to/book.portmanteau.json --save
```

The CLI uses the demonstration project unless `--project` is passed. Mutating
commands are discarded when the process exits unless you add `--save`; export
commands write their own output regardless.

### MCP

`portmanteau-mcp` serves newline-delimited JSON-RPC over stdio. Tools include
`project_info`, `list_binder`, `read_document`, `create_document`,
`update_document`, `delete_document`, `move_document`, `snapshot_document`,
`list_snapshots`, `restore_snapshot`, `compare_snapshot`, `checkpoint`,
`search_project`, `replace_text`, `project_stats`, `parse_script`, `compile_preview`,
`list_export_formats`, `export_manuscript`, `add_character`,
`add_research_note`, `add_idea` and `list_commands`.

Run it with `cargo run -p portmanteau-mcp` and speak JSON-RPC on stdin/stdout. It starts
with the demonstration project in memory; persistence happens through the
project save/open tools.

---

## 8. Desktop layer conventions

- `src/app.rs` owns **view state only**: selected document, current view,
  collapsed binder ids, search text, focus mode, draft fields for modals,
  toasts, screenshot counter and the open diff. The project belongs to
  `Workspace`.
- Each workspace in `src/ui/views/` is a function `show(&mut app, &mut ui)`.
  Interactions are collected into a small local enum or `Option`, then applied
  **after** the draw pass. This avoids borrow conflicts and keeps painting
  readable. Follow that pattern rather than mutating the project mid-widget.
- Shared page chrome is in `src/ui/views/mod.rs`:
  `page_head`, `content_frame`, `meta_line`, `primary_button`, `pill`, etc.
  Reuse it so screens line up.
- The palette and fonts are in `src/theme.rs`. Bundled fonts are Liberation
  Serif/Sans/Mono plus DejaVu Sans as a symbol fallback. Do not rely on system
  fonts; a missing glyph becomes a box on another machine.
- Icons in `src/icons.rs` are painted shapes, not images. Add a variant and a
  painter arm rather than shipping an icon font.
- Text widgets hold egui's own undo history. Structural changes should create a
  snapshot/checkpoint first when they are risky.
- `src/app.rs` modals: New project, About, path Open/Save As, idea card and
  Compare version. Add new modal state to `PortmanteauApp` and call the modal
  from `modals()`, not from a view.
- Screenshot mode lives in `PortmanteauApp::screenshot_tick`. It requests a
  viewport screenshot at frame 120, saves RGBA through the `image` crate and
  closes the window. Keep the frame count and the close.

---

## 9. Verification checklist

Run these before every handoff:

```sh
cd app
cargo build --workspace
cargo test --workspace
./target/debug/portmanteau --screenshot shots/now.png
```

Then look at `shots/now.png` (and any screen you touched with
`PORTMANTEAU_VIEW`). Confirm it is populated and the intended screen is
readable. The outer checker also runs `rebuild_check`, which checks the build
and a populated 2880×1860 screenshot; it must pass.

For exports, exercise one or more real files and check the bytes, not just the
success flag:

```sh
./target/debug/portmanteau-cli export.run '{"format":"PDF","path":"/tmp/portmanteau-test"}'
./target/debug/portmanteau-cli export.run '{"format":"EPUB","path":"/tmp/portmanteau-test"}'
./target/debug/portmanteau-cli export.run '{"format":"FDX","path":"/tmp/portmanteau-test"}'
file /tmp/portmanteau-test/*
```

Current test coverage:

- `portmanteau-core`: project scaffold, snapshot restore, checkpoint restore, document
  move/reparent, snapshot body text, script classification/parsing, word counts
  and line diff.
- `portmanteau-api`: command round-trip, unknown command, stats, move, snapshot diff,
  a real Fountain export.
- `portmanteau-mcp`: tool list is non-empty and callable.

---

## 10. Recipes

### Add a command

1. Add the match arm in `crates/portmanteau-api/src/lib.rs`.
2. Add the catalogue tuple at the bottom of that file.
3. Add a stable payload parser; return `ApiResponse::err` with a human message
   on bad input.
4. Add an MCP tool definition and mapping if it belongs in the agent surface.
5. Add a test using `Workspace::demo()`.
6. Add the row to the README command table.

### Add an export format

1. Add an `ExportKind` variant, `label`, `short`, `extension` and `description`
   in `crates/portmanteau-core/src/model.rs`.
2. Add a writer in `crates/portmanteau-export/src/lib.rs` and a match arm in `export()`.
3. If the format needs a new front/back matter option, add it to
   `ExportSettings` **and** `crates/portmanteau-export` assembly **and** the Compile
   screen.
4. The Compile format grid and `export.formats` follow the enum. Verify the
   file with a real export.
5. Note any external-dependency or layout limit in `docs/notes.md`.

### Add a workspace

1. Add a `View` variant, `label()` and `icon()`; extend `View::all()`.
2. Add a painter arm in `src/icons.rs`.
3. Create `src/ui/views/<name>.rs`, register it in `views/mod.rs` and dispatch
   from `views::show`.
4. Use the shared page chrome. Collect actions after drawing.
5. Add the screen to the `PORTMANTEAU_VIEW` list in this file and in the docs.

### Change a versioning rule

Versioning is intentionally three-layered: widget undo, document snapshot,
project checkpoint. Export takes a checkpoint. Do not remove a layer without a
replacement. If you add automatic history, keep it local and file-based, and
make sure the command API can still list, compare and restore it.

---

## 11. Known limits and next work

These are documented rather than hidden:

- PDF/EPUB layout is produced by the built-in engine, not a full typesetter.
  Running heads, widows/orphans, embedded images and professional book interiors
  are future work.
- Binder reordering is button/menu based; drag-and-drop is not implemented.
  Reparenting exists in the API but the binder does not offer a drag gesture.
- Native file pickers exist on desktop (`rfd`); some fields are still typed paths.
- The diff is line-level, not word-level, and does not yet offer accept/reject
  hunks or tracked changes.
- Import covers Fountain, Final Draft (FDX), Word (DOCX) and plain text through
  `document.import`, the `import_document` MCP tool and File → Import. The
  parsers live in `crates/portmanteau-import`; see `docs/roadmap.md` for what is
  not done (dual dialogue, Word images and footnotes; the review step exists).
- Collaboration is out of scope for this version: no accounts, network, sync,
  comments or shared review. Keep future designs file-based and versioned so a
  writer owns their data.
- Releases, in-app updates and the web build exist (`docs/releasing.md`).

`docs/roadmap.md` expands these into concrete entry points and acceptance
criteria.

---

## 12. Traps already paid for

- **Missing glyphs.** Liberation does not carry `¶`, `▸`, `⌕`, arrows or some
  box-drawing marks. DejaVu Sans is installed as a fallback and inserted after
  the main families. If a new symbol renders as a box, add it to the fallback or
  paint it.
- **Borrow conflicts in views.** Mutating `app.ws.project` while a `doc`
  reference from the same project is alive will not compile. Read into a small
  owned struct, draw, then apply actions. The existing views show the pattern.
- **`export.run` paths.** A folder path without an extension is resolved by the
  API; do not append a filename twice. A full path is respected.
- **Formatting.** Do not run `cargo fmt` across the whole repository casually —
  it rewrites files that were deliberately laid out and makes review noisy.
  Format only the file you changed.
- **Screenshots are git-ignored.** `shots/` is for review, not for history. Do
  not make the build depend on a committed image.
- **Modal size.** `egui::Window` with `.max_width` still needs enough horizontal
  room; the diff uses wrapped labels for this reason. Check at 1440×930.
- **Empty documents.** Prose and script bodies can be empty; diff, word counts
  and export must handle that without panicking.
- **Project files.** There is no backwards compatibility: a renamed field may
  simply break old files. Do not add aliases or migrations.
- **Do not start background services.** The checker runs the binary; a process
  that does not exit will fail the handoff.
- **Changing an English text changes its translation key.** After editing a
  `tr("…")` text, update `assets/languages/*.json` and regenerate the template
  (see `docs/localization.md`), or the translation silently stops applying.
- **Spelling and the screenshot tests read the writer's config**: tests and
  screenshot runs override theme, language and view settings; keep it that way.
- **Do not reintroduce collaboration without asking.** The person explicitly
  chose a local-only version for now.

---

## 13. Git

The repository root is `app/`; commit there.

```sh
cd app
git status
git add -A
git commit -m "Short imperative summary"
```

Keep commits coherent: a command and its UI together, docs with the behaviour
they describe, tests with the logic they cover. The build must compile at every
commit. Do not commit generated output (`target/`, `shots/`, `exports/`,
local project files).

---

## 14. First five things to do if you are picking this up cold

1. Read this file, then `README.md`, `docs/architecture.md`, `docs/notes.md`
   and `docs/roadmap.md`.
2. Run `cargo build --workspace`, `cargo test --workspace` and the screenshot
   command. Look at the PNG.
3. Run `portmanteau-cli commands` and `portmanteau-cli stats` to see the command surface.
4. Open `src/app.rs` and `src/ui/views/mod.rs`; the shape of every screen should
   become clear from those two files.
5. Pick the next item from `docs/roadmap.md` that matters to the person, add the
   command first, then the UI, then the documentation and a test.
