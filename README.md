# Portmanteau

A native writing studio for novels, film, television and theatre, built as a
modular Rust workspace with a command-driven core and an egui desktop layer.

This is the first working version. It is deliberately shaped so the product can
grow without rewriting its foundations.

## Run it

```sh
cargo run
```

Or run a built binary:

```sh
cargo build
./target/debug/portmanteau
```

Screenshot mode (used by the build checks, and useful for visual reviews):

```sh
./target/debug/portmanteau --screenshot shots/now.png
```

Development aids for reviewing individual screens:

```sh
PORTMANTEAU_VIEW=corkboard ./target/debug/portmanteau --screenshot shots/corkboard.png
PORTMANTEAU_DOC="Cold Harbor" ./target/debug/portmanteau --screenshot shots/script.png
```

## What is in this version

**Writing**
- Prose editor on a paper surface, with title, body, live word and page counts.
- Typed screenplay editor for film, TV and stage: scene headings, action,
  character, parenthetical, dialogue, transitions, shots, acts, scenes and
  stage directions.
- Per-element colour and gutter, Tab/Shift-Tab to change type, Enter to add the
  next logical element, Backspace on an empty element to remove it.
- Focus mode, medium conversion, custom word targets and status labels.
- Continuous vs Paged view: Paged is a read-only print preview that estimates
  page breaks from the active export format and shows page numbers.

**Structure**
- Binder tree with parts, chapters, scenes, research, front matter and back
  matter; collapse/expand, include/exclude from compile, and one-step
  reordering from the header arrows, row context menu or Edit menu.
- Corkboard of chapter and scene cards grouped by part.
- Outline table with labels, status, length and target progress.
- Full-text search across titles and manuscript bodies.

**Development**
- Idea board with premise, theme, image, scene and voice cards.
- Eight-beat, three-act story structure, editable and tickable.
- Character sheets: role, description, want, need, flaw, arc, appearances.
- Research sourcebook with kinds, sources and a linked document per note.

**Versions**
- Per-document named snapshots with restore.
- A line-level **Compare version** dialog: additions and removals against the
  current text, with word counts and one-click restore. The same comparison is
  available to agents as `snapshot.diff`.
- Whole-project checkpoints with restore.
- Fourteen-day writing calendar.
- Export automatically takes a project checkpoint before writing a file.

**Compile and export**
- PDF, EPUB, DOCX, Final Draft (FDX), Fountain, HTML, Markdown, RTF, plain text
  and a complete project bundle.
- Front matter toggles: title page, copyright, dedication, author's note, table
  of contents, chapter numbers.
- Body and back matter toggles: scene numbers, synopses, author notes,
  acknowledgements, about the author.
- Paper size and type-size/leading controls (applied where the format supports
  them), export presets, output folder and a compile preview.
- Per-format settings: PDF, EPUB, DOCX and the rest each remember their own
  paper, type and matter choices, seeded from a project-wide base style for
  prose and script. Page numbers and running headers apply to PDF; an EPUB can
  reference a JPG or PNG cover image.

**Defaults**
- A **Settings** workspace edits the global per-element type defaults.
- Defaults are saved to `$XDG_CONFIG_HOME/portmanteau/config.json` (falling back
  to `~/.config/portmanteau/config.json`), applied to the current project on
  demand, and copied into every new project.

**Automation**
- Every important action is a JSON command on a single workspace object.
- `portmanteau-api` is the command bus; the UI uses it for creations and exports.
- `portmanteau-mcp` exposes the same commands as MCP tools over stdio.
- `portmanteau-cli` runs one command and prints JSON.
- The **Command desk** view lets a person inspect, edit and run the same calls.

## Workspace layout

```
app/
  Cargo.toml            workspace + desktop binary
  src/                  egui desktop layer
    app.rs              shell, navigation, modals, screenshot mode
    theme.rs            palette, fonts, shared visual language
    icons.rs            hand-drawn rail icons
    ui/
      binder.rs         binder tree and session target
      editor.rs         prose and screenplay surfaces
      inspector.rs      metadata, notes and versions
      views/            corkboard, outline, research, develop,
                        characters, history, compile, command desk,
                        settings
  crates/
    portmanteau-core/           domain model, script parser, sample project
    portmanteau-export/         PDF, EPUB, DOCX, FDX, Fountain, HTML, MD, RTF, TXT, ZIP
    portmanteau-api/            Workspace and JSON command bus
    portmanteau-mcp/            MCP server (stdio JSON-RPC)
    portmanteau-cli/            one-command CLI client
  assets/               bundled Liberation and DejaVu fonts
  docs/                 architecture and running notes
```

The dependency direction is one-way:

```
portmanteau-core  <-  portmanteau-export
    ^             ^
    |             |
  portmanteau-api  <----  portmanteau-mcp / portmanteau-cli
    ^
  desktop UI
```

`portmanteau-core` knows nothing about files, UI or export. `portmanteau-export` knows about
the manuscript, not the window. `portmanteau-api` owns persistence and the command
surface. The desktop layer is a view onto that API, not a second model.

## Commands

A command always returns `{ ok, command, message, data }`.

| Command | Purpose |
| --- | --- |
| `project.info` | Summary of the open project |
| `project.new` | Create a project (`title`, `kind`) |
| `project.open` / `project.save` | Open or save a project file |
| `project.update` | Title, author, genre, logline, daily target |
| `project.checkpoint` / `project.restore_checkpoint` | Whole-project versions |
| `project.snapshots` | Every document snapshot |
| `document.list` / `document.get` | Read the binder or one document |
| `document.create` / `document.update` / `document.delete` | Write the binder |
| `document.move` | Reorder within a folder, or move into one |
| `document.snapshot` / `snapshot.restore` / `snapshot.diff` | Save, restore and compare document versions |
| `character.add` / `character.update` | Character sheets |
| `research.add` | Research note plus its binder entry |
| `idea.add` / `idea.update` / `idea.delete` | Idea board |
| `beat.update` | Tick or rewrite a story beat |
| `search` | Full-text search |
| `stats` | Words, pages, scenes, chapters, session log |
| `script.parse` | Parse loose text into typed blocks and Fountain |
| `export.preview` / `export.formats` | Compile preview and format list |
| `export.settings.get` / `export.settings.update` | Read or change one format's compile settings |
| `style.get` / `style.update` | Read or change the project base prose/script style |
| `export.run` | Write a real export file |
| `commands` | The catalogue the UI and MCP discovery read |

Examples:

```sh
cargo run -p portmanteau-cli -- stats
cargo run -p portmanteau-cli -- export.run '{"format":"EPUB","path":"exports"}'
cargo run -p portmanteau-cli -- document.create '{"title":"New scene","kind":"Scene","medium":"Screenplay"}'
cargo run -p portmanteau-cli -- document.move '{"id":"doc-…","direction":"up"}' \
  --project book.portmanteau --save
```

The CLI is one command per process. Add `--save` to write a mutating command
back to the project; otherwise it runs against an in-memory copy.

Projects are folder packages with the extension `.portmanteau`. `project.json`
is the manifest — binder structure and short metadata — and each document body
is a Markdown or Fountain file in `documents/`, so the writing stays readable
and diffable outside Portmanteau. Saving without a path uses
`~/Portmanteau/the-salt-road.portmanteau/`. Opening a legacy
`.portmanteau.json` file still works, and saving to a `.json` path keeps that
single-file form. The File menu has Open, Save and Save As, with `Ctrl/Cmd+O`
and `Ctrl/Cmd+Shift+S` shortcuts.

## Local-only, deliberately

Portmanteau has no accounts, network code, shared review server or sync layer.
Everything lives in a project folder on the writer's own disk, and the project
bundle is a local archive rather than a collaboration format. Any future
commenting should be built on versioned files in the writer's own repository,
not on a hosted service.

## Known limits and next steps

- PDF/EPUB layout is generated by the built-in engine, not a full typesetter.
  Complex running heads, widows/orphans and embedded images are next.
- Reordering is one step at a time from the binder, context menu, Edit menu or
  `document.move`; drag-and-drop is not implemented yet. Reparenting exists in
  the API but has no drag gesture.
- File dialogs are not bundled; paths are typed. A native picker can be added
  without changing the export engine.
- Revision comparison is line-level. Word-level changes, tracked changes with
  accept/reject, and importing manuscripts are the next milestones.
- Collaboration is out of scope for this version: no real-time co-writing, no
  shared review server, and no comment system. Any future review should stay
  file-based and versioned.
- Windows/macOS packaging needs a release pipeline. The code is platform-neutral
  Rust/egui.

## Documentation for the next developer

- `AGENTS.md` — the full handoff: rules, architecture, commands, conventions,
  recipes, limits and traps.
- `docs/architecture.md` — why the workspace is split this way.
- `docs/notes.md` — the running build log and screen-review decisions.
- `docs/roadmap.md` — prioritised next work with concrete entry points.
