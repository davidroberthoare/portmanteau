# Architecture

## Design rule

The window is replaceable. Everything that makes Scriptorium a writing tool
lives below the window and is reachable through JSON commands.

That gives three useful properties:

1. The same behaviour is testable from `msc-cli`, `msc-mcp` and automated tests.
2. A future mobile, web or terminal client can reuse the core without egui.
3. An agent can read and write the project with the same operations a person
   uses, rather than scraping the interface.

## Crates

### `msc-core` — the domain

Pure Rust + serde. No file system, UI or export code.

| Module | Responsibility |
| --- | --- |
| `model.rs` | `Project`, `Document`, `Block`, `Character`, `ResearchItem`, `IdeaCard`, `Beat`, `Snapshot`, `Checkpoint`, export settings |
| `script.rs` | `ElementKind`, loose text classification, Fountain conversion, aligned plain text |
| `text.rs` | Word counts, reading time, truncation, line-level diff |
| `sample.rs` | The demonstration project, `The Salt Road` |

A `Document` carries both a prose `text` and script `blocks`. Only one is
authoritative, decided by `medium.is_script()`. `convert_medium` parses prose
into blocks when moving into a script medium and joins blocks back into prose
when moving out. This keeps one document model rather than parallel prose and
screenplay types.

Hierarchy is a parent id plus an order integer, not a nested tree. That makes
serialisation flat, filtering cheap, and each document an addressable record.
`ordered_documents` and `children` reconstruct the tree for display.
`move_document` swaps the order of two siblings, and `reparent_document` moves a
subtree to the end of another folder while refusing cycles. Both are commands,
so the binder arrows and an agent share one implementation.

Versions are stored twice, at different scopes:

- `Document.snapshots` — a named copy of one document's text and blocks.
- `Project.checkpoints` — a JSON serialisation of the whole project, capped at
  30. Restoring keeps the existing checkpoint list so the restore itself does
  not destroy the history that made it possible.

### `msc-export` — compilation

One assembly pass (`manuscript_sections`) and one writer per format. Front and
back matter are collected from the binder folders and filtered by the export
settings. Script media are written with their typed elements, not flattened
into prose.

- PDF is a self-contained PDF 1.4 writer (no external typesetter).
- EPUB and DOCX are zip containers written with `zip`.
- FDX is Final Draft XML; Fountain is the plain-text interchange format.
- HTML, Markdown, RTF and TXT share the same section assembly.
- Bundle zips the project JSON, notes and exports for archive or hand-off.

`export()` takes an optional output path. If the path looks like a folder (no
extension) the caller joins the standard filename; the API does this so a person
can type a folder in the Compile screen.

### `msc-api` — the command bus and persistence

`Workspace` owns the `Project`, the open path, a dirty flag and a short log.
`execute(command, payload)` is a single match with one response shape. Commands
are deliberately coarse and intention-revealing (`document.snapshot`,
`export.run`, `project.checkpoint`) rather than CRUD-only.

`command_catalogue()` is the shared source of truth for:

- the Command desk list in the UI,
- the MCP tool environment's examples,
- documentation and future client generation.

Persistence is plain JSON. A project is portable, diffable and easy to inspect.
That is a deliberate trade for a first version; a transactional store can sit
behind the same commands later.

### `msc-mcp` and `msc-cli`

`msc-mcp` serves newline-delimited JSON-RPC over stdio, exposes the catalogue as
MCP tools, and maps each tool onto one command. `msc-cli` executes a single
command for smoke tests and shell automation.

### Desktop layer

`src/app.rs` is the shell: top bar, navigation rail, binder, inspector, status
bar, modals, shortcuts and screenshot mode. It owns view state only; the
document state stays in `Workspace`.

`src/ui/views/` contains one module per workspace. Each is a function over
`&mut ScriptoriumApp` and `&mut egui::Ui`. Mutations are collected into small
action enums or local options and applied after the draw pass, which avoids
borrow conflicts and keeps the painting code readable.

The visual language is centralised in `theme.rs`: a dark ink-and-paper palette,
serif for prose, sans for chrome, mono for scripts and code. Fonts are bundled
so the app looks the same on any machine. `icons.rs` draws the rail glyphs with
the painter, so no image assets or icon font are required.

## Screen map

| View | Purpose | Primary command hooks |
| --- | --- | --- |
| Write | Prose/screenplay editing and inspector | direct document edits |
| Corkboard | Story shape as cards | open document, new document |
| Outline | Table of labels, status and length | include/exclude, open |
| Research | Sourcebook and linked notes | `research.add` |
| Develop | Idea board and beat sheet | `idea.add`, direct beat edits |
| Characters | Character sheets and appearances | direct character edits |
| Versions | Checkpoints, snapshots, compare, calendar | checkpoint/restore/diff |
| Compile | Format, matter and style choices | `export.run` |
| Command desk | Raw API console | any command |

## Versioning model

There are three layers, chosen so a writer can recover at the scale they need:

1. **Undo** inside text widgets (egui's built-in edit history).
2. **Document snapshot** before a structural revision: cheap, named, restorable.
3. **Project checkpoint** before a milestone or export: complete, restorable,
   capped and never silently pruned during a restore.

Exporting always writes an `export` checkpoint first. This means a file that
leaves the app can always be traced back to the project state that produced it.

Comparison sits beside restore, not instead of it. `snapshot.diff` reconstructs
the saved body (`Snapshot::body_text`) and runs a line-level longest-common-
subsequence diff against the current document. The result is data — `added`,
`removed`, word counts, and lines marked `same`, `add` or `remove` — so the
Compare version dialog, an agent and a future review export describe a revision
identically. The diff lives in `msc-core::text`, not in the modal.

## Testing and checks

- Unit tests in `msc-core` cover scaffold creation, snapshot restore, checkpoint
  restore, document move/reparent, snapshot body text and line diff.
- `msc-api` tests cover command round-trips, unknown commands, statistics,
  reordering, snapshot comparison and a real export file.
- `msc-core::script` tests cover scene and character classification, loose
  parsing and Fountain output.
- The desktop binary has a `--screenshot` mode that draws 120 frames, captures
  the viewport as PNG and exits. It is both a visual review tool and the build
  check used by the surrounding tooling.
- `SCRIPTORIUM_VIEW` and `SCRIPTORIUM_DOC` open a specific screen and document,
  so every workspace can be inspected without clicking.

## Extension points

- **New export format:** add an `ExportKind` variant, a writer function and the
  match arm in `export()`. The UI list and API parsing follow from the enum.
- **New command:** add a match arm to `Workspace::execute`, then a catalogue
  entry and, if agents should see it, an MCP tool mapping.
- **New workspace:** add a `View` variant, an icon, a module in `ui/views/`, and
  a dispatch arm. It can call any command.
- **Local commenting:** if it is revisited, keep it file-based and versioned:
  store annotations in the project file or a sidecar file the writer owns, with
  no account, server or presence layer.
- **Drag-and-drop:** `document.move` already handles sibling order and
  reparenting. Wire egui drop zones in the binder to those methods.
- **Word-level revisions:** add a finer diff unit and revision records beside
  `snapshot.diff`; keep the line-level command backward compatible.
