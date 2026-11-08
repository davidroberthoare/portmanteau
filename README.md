# Scriptorium

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
./target/debug/myscript
```

Screenshot mode (used by the build checks, and useful for visual reviews):

```sh
./target/debug/myscript --screenshot shots/now.png
```

Development aids for reviewing individual screens:

```sh
SCRIPTORIUM_VIEW=corkboard ./target/debug/myscript --screenshot shots/corkboard.png
SCRIPTORIUM_DOC="Cold Harbor" ./target/debug/myscript --screenshot shots/script.png
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

**Structure**
- Binder tree with parts, chapters, scenes, research, front matter and back
  matter; drag-free ordering, collapse/expand, include/exclude from compile.
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

**Automation**
- Every important action is a JSON command on a single workspace object.
- `msc-api` is the command bus; the UI uses it for creations and exports.
- `msc-mcp` exposes the same commands as MCP tools over stdio.
- `msc-cli` runs one command and prints JSON.
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
      inspector.rs      metadata, notes, versions, comments
      views/            corkboard, outline, research, develop,
                        characters, history, compile, command desk
  crates/
    msc-core/           domain model, script parser, sample project
    msc-export/         PDF, EPUB, DOCX, FDX, Fountain, HTML, MD, RTF, TXT, ZIP
    msc-api/            Workspace and JSON command bus
    msc-mcp/            MCP server (stdio JSON-RPC)
    msc-cli/            one-command CLI client
  assets/               bundled Liberation and DejaVu fonts
  docs/                 architecture and running notes
```

The dependency direction is one-way:

```
msc-core  <-  msc-export
    ^             ^
    |             |
  msc-api  <----  msc-mcp / msc-cli
    ^
  desktop UI
```

`msc-core` knows nothing about files, UI or export. `msc-export` knows about
the manuscript, not the window. `msc-api` owns persistence and the command
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
| `document.snapshot` / `snapshot.restore` | Document versions |
| `comment.add` / `comment.resolve` | Review comments |
| `character.add` / `character.update` | Character sheets |
| `research.add` | Research note plus its binder entry |
| `idea.add` / `idea.update` / `idea.delete` | Idea board |
| `beat.update` | Tick or rewrite a story beat |
| `search` | Full-text search |
| `stats` | Words, pages, scenes, chapters, session log |
| `script.parse` | Parse loose text into typed blocks and Fountain |
| `export.preview` / `export.formats` | Compile preview and format list |
| `export.run` | Write a real export file |
| `commands` | The catalogue the UI and MCP discovery read |

Examples:

```sh
cargo run -p msc-cli -- stats
cargo run -p msc-cli -- export.run '{"format":"EPUB","path":"exports"}'
cargo run -p msc-cli -- document.create '{"title":"New scene","kind":"Scene","medium":"Screenplay"}'
```

Project files are JSON with the extension `.scriptorium.json`. Saving without a
path uses `~/Scriptorium/the-salt-road.scriptorium.json`.

## Collaboration, deliberately staged

This first version is solo-first, as requested. It already keeps review
comments with authors, resolve state, timestamps and restore points, and the
project bundle carries them. Real-time shared editing needs a sync protocol,
presence and conflict handling; that belongs in a later milestone rather than a
half-working toggle.

## Known limits and next steps

- PDF/EPUB layout is generated by the built-in engine, not a full typesetter.
  Complex running heads, widows/orphans and embedded images are next.
- Drag-and-drop reordering of binder rows is not implemented yet; documents are
  appended within their parent. Reordering can be added to the command bus
  first, then the binder.
- File dialogs are not bundled; paths are typed. A native picker can be added
  without changing the export engine.
- Real-time collaboration, a shared review server, and revision comparison
  (not just restore) are the next large milestones.
- Windows/macOS packaging needs a release pipeline. The code is platform-neutral
  Rust/egui.
