# AGENTS.md — working on Portmanteau

This is the `agents.md` handoff the person asked for, kept under the standard
uppercase name so agent tooling finds it. Only one file exists: a second
lowercase copy would collide on macOS and Windows, which use case-insensitive
file systems.

This file is the handoff for the next developer, and for an AI coding agent in
particular. Read it before changing anything. It describes the product, the
rules that are not negotiable, the architecture, the command surface, how to
verify a change, and the traps that have already caused bugs.

Companion documents:

| File | What it answers |
| --- | --- |
| `README.md` | What the product is, how to run it, what works today |
| `docs/architecture.md` | Why the code is split the way it is |
| `docs/notes.md` | Running build log and the visual decisions behind the screens |
| `docs/roadmap.md` | Prioritised next work, with entry points |

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

## 2. Non-negotiable rules

These come from the request or from the surrounding build tooling. Do not trade
them away to make a build or a check pass.

- **Rust + eframe/egui.** The desktop binary must stay a native Rust/egui app.
- **Edit only inside `app/`.** The rest of the workspace (`checks/`, `capture/`,
  `investigation.cany/`, `feed.json`, …) is evidence or tooling output and is
  read-only.
- **Keep the command layer authoritative.** New behaviour should be expressible
  as a command before or alongside its UI. Do not put domain logic in a widget.
- **Keep it local.** No HTTP clients, servers, accounts, telemetry, presence or
  background daemons. Project files and export bundles stay on the writer's
  disk.
- **`--screenshot <file.png>` must always work.** It starts normally, draws 120
  frames, saves the window as a PNG and exits. A blank picture fails the build
  check. A normal launch must not require the flag.
- **No long-running processes.** Never start a dev server, watcher, `cargo
  watch` or anything that does not exit. The build tooling runs and checks the
  app itself.
- **Keep the visual language.** Dark ink-and-paper palette, serif prose, sans
  chrome, mono script/diff/code, bundled fonts. Read `src/theme.rs` before
  inventing colours or sizes.
- **Commit often, in small coherent steps.** The repository is `app/.git`.
- **Do not copy brand assets or long copyrighted text.** The demonstration
  project is original sample copy; keep it that way.

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

## 4. Where you are now

As of this handoff:

- Desktop shell, nine workspaces, prose editor, screenplay editor, binder,
  inspector, search, save/open, modals, toasts and screenshot mode all work.
- The domain core, export engine, command API, MCP server and CLI are wired.
- Versioning includes document snapshots, project checkpoints, a writing
  calendar, restore, automatic checkpoint-before-export, and a line-level
  **Compare version** dialog. The same comparison is the `snapshot.diff`
  command and the `compare_snapshot` MCP tool.
- Binder order is editable through the header arrows, the row context menu, the
  Edit menu, and the `document.move` command.
- All ten export formats have been exercised to real files through `portmanteau-cli`.
- `cargo test --workspace` passes.
- Collaboration is deliberately absent. Do not reintroduce comment, account,
  sync or presence scaffolding without asking the person first.

Read `docs/notes.md` for how the screens were built and `docs/roadmap.md` for
the next likely work.

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
- File pickers are typed paths. A native picker can be added behind the same
  Save/Open commands.
- The diff is line-level, not word-level, and does not yet offer accept/reject
  hunks or tracked changes.
- Import covers Fountain, Final Draft (FDX), Word (DOCX) and plain text through
  `document.import`, the `import_document` MCP tool and File → Import. The
  parsers live in `crates/portmanteau-import`; see `docs/roadmap.md` for what is
  not done (review step, dual dialogue, Word images and footnotes).
- Collaboration is out of scope for this version: no accounts, network, sync,
  comments or shared review. Keep future designs file-based and versioned so a
  writer owns their data.
- Windows/macOS installers and a release pipeline are not set up; the Rust/egui
  code is platform-neutral.

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
- **Project files from older builds.** Serde ignores removed fields, but a
  renamed field needs an alias or migration. Test open/save around changes.
- **Do not start background services.** The checker runs the binary; a process
  that does not exit will fail the handoff.
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
