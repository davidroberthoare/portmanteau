# Build notes

Running notes for the first version, in the order the work happened. These are
deliberately concrete: what was decided, why, and what was seen on screen.

## Starting point

The request asks for the core of Scrivener, Final Draft and WriterDuet, solo
first, with collaboration coming later. The important words are *novel*,
*theatre*, *film*, *TV*, *idea development*, *research*, *versioning*, *many
export formats* and *front/back matter*.

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
   sync into the linked research document. Added a “new note” path through the
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

- Real-time co-writing, presence, or a shared review server. The request says
  collaboration will be important *eventually*, so comments, authors and
  bundles are the foundation and the network layer is a later milestone.
- Drag-and-drop binder reordering. The command (`document.move`) should exist
  before the gesture, so agents and the UI share behaviour.
- Tracked changes and a visual diff between two snapshots. Restore exists;
  comparison is the next versioning step.
- Native file pickers. Paths are typed, and the default export folder works
  without one.
- Packaging installers for Windows/macOS. The code is cross-platform; the
  release pipeline is separate work.

## Open questions for the next conversation

1. Which collaboration shape comes first: shared review comments, or live
   co-editing? They need very different infrastructure.
2. Should compile use the built-in PDF typesetter longer, or delegate to
   LaTeX/HTML-to-PDF for professional book interior layout?
3. Which import formats matter most: Word manuscripts, Final Draft files,
   Fountain, or Scrivener projects?
4. Does the person want the version history to stay snapshot-based, or move to
   an automatic change log with diffs and branching?
