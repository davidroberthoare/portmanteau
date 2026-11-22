# CLAUDE.md — Portmanteau

Native, offline Rust/egui writing studio (novels, screenplays, TV, stage). Local-only, GPL-3.0, personal/hobby project. The owner is not a programmer: keep the app runnable at every commit and explain changes in plain language.

**Read `AGENTS.md` first** for the architecture, command API, recipes and traps. It is still mostly accurate. Where it conflicts with this file, this file wins (see "Corrections to AGENTS.md").

## Commands

```sh
cargo build --workspace
cargo test --workspace                     # ~193 tests, all passing at handoff
./target/debug/portmanteau --screenshot shots/now.png     # must always work; look at the PNG
PORTMANTEAU_VIEW=compile ./target/debug/portmanteau --screenshot shots/compile.png
cargo run -p portmanteau-cli -- commands
```

Export tests call `pdfinfo`/`pdffonts`/`pdftotext` (poppler) and skip if absent.

## Layout

Repo root is `app/` (own `.git`, branch `master`). The parent folder `myScript/` holds only tooling and evidence: `.opencode/agents/` (old OpenCode conductor/implementer/tester team), `.kilo/`, `notes/next_tasks.md`, `shots/`, `exports/`. Edit inside `app/`. Crates: `portmanteau-core` (model, no I/O) <- `-export` (one module per format over shared `layout`, `web` and `content` modules; PDF via krilla) and `-import` (Fountain, FDX, DOCX, plain text) <- `-api` (command bus, persistence) <- `-mcp`/`-cli`, plus the egui shell in `src/`.

## Rules that still hold

- Rust + eframe/egui, native only. No network, accounts, telemetry, background daemons or long-running processes (never start dev servers/watchers).
- Command layer is authoritative: new behaviour is a command (API arm + catalogue entry + MCP tool + README row + test) before or alongside UI. No domain logic in widgets.
- `--screenshot` must keep working; a blank PNG is a failure.
- **No backwards compatibility, ever, until the owner says otherwise.** The project has no users; any change may be breaking. Don't add migrations, aliases or compatibility shims.
- No collaboration features unless the owner asks.
- Don't run `cargo fmt` repo-wide; format only files you touched.
- Don't commit `target/`, `shots/`, `exports/`.
- Commit small and often, imperative messages. Commit only when asked.

## Corrections to AGENTS.md

- **Visual language is now light**, not dark: warm-gray chrome, soft-white paper (`src/theme.rs`, `docs/ui-style-guidelines.md`). Ignore the "dark ink-and-paper" rule. A dark theme is optional future work on the same tokens.
- **Persistence is a project folder package** (small files, rich bodies as `.rich.json`, Fountain with `/* @kind */` markers, atomic temp-and-rename save), not one `.portmanteau.json`. Legacy `.json` support was dropped.
- **Content model is `RichDocument`** (styled paragraphs, inline marks, images) shared by prose, scripts and long-form fields; the old text/blocks split is gone. Projects own editable style sheets; program defaults are copied into new projects.
- Settings is a workspace (global defaults + per-project overrides). Per-format, per-element export styles exist (`Project.formats`, `ElementStyles`); PDF/EPUB/HTML consume them.
- Compile = one manuscript type per export (Prose/Screenplay/TV/Stage), scope Draft or subtree, optional one file per script. Default paper is US Letter; script geometry scales with paper width/612pt.
- Binder has a single "Draft" root; Script Lab was removed. Import supports Fountain, FDX, DOCX, plain text.
- Tests use `egui_kittest` for UI; the OpenCode conductor's rule was "UI bugs get a failing kittest first". Keep that habit.

## State at handoff (2026-11-21)

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
- **Undo/redo** (`src/history.rs`): observes the selected document, steps are
  changed-paragraph windows, typing merges until a 0.7 s pause.
- **Spell-check** now covers every prose field.
- **UI cleanup**: dead or repeated controls removed, each with a comment on
  what it should become; a shell test keeps them out.
- Bugs found by looking at output: PDF ignored Justify and always centred the
  title page; EPUB referenced fonts it never packaged; DOCX/RTF/TXT ignored
  the Contents switch; only the PDF numbered scenes; the Fountain export was
  not valid Fountain (blank line between cue and dialogue).

How to check output visually (all on this machine):
- PDF: `pdftoppm -png`, and `pdftotext -bbox-layout` to measure positions.
- EPUB / HTML: `ebook-convert x.epub x.pdf`, or `chromium --headless --print-to-pdf`,
  then `pdftoppm`. Calibre rescales type, so trust it for structure, not size.
- **DOCX / RTF: `wordcraft-cli`** (the WordCraft app). `render f out.png --page N`,
  `info f` (page count), `inspect f` (parsed paragraph properties as JSON),
  `convert f out.pdf --strict` (fails on undrawable characters or missing pictures).
  LibreOffice does not run in this sandbox. WordCraft's PNG render of RTF shows
  space-before at page tops slightly differently from its DOCX render; compare via
  `convert ... out.pdf` and `pdftotext -bbox-layout` when geometry matters.
- Rebuild with `cargo build --workspace`: a plain `cargo build` leaves the CLI stale.

- **Project page** (`src/ui/views/project.rs`, rail icon between Versions and
  Compile): title, subtitle, author, genre, logline, contact, draft label and
  date, daily goal. Edits go through `project.update`. A script cover prints
  contact bottom-left and the draft line bottom-right (`script_title.rs`); a
  blank draft date means the day of export. Prose title pages do not print
  contact yet (a copyright page would).

## Next work (priority order)

1. ~~Preview pagination~~ Done: Compile, Settings and the editor's Paged view all
   draw `pdf_layout` pages (`src/ui/preview.rs`); the old estimator is deleted.
   Paged shows a document's pages of the real compile with real page numbers.
   Page counts everywhere (inspector, paper header, Compile list, API) come from
   `portmanteau_export::page_counts`, refreshed a moment after typing pauses.
   The Paged view is read-only; see "Editing on pages" in docs/roadmap.md.
2. ~~PDF page per script document~~ Done: scenes run on; only a chapter-level
   document (episode/act) starts a page (`content::starts_new_page`, all writers).
3. **Import review step**: show the split (titles, word counts) before the
   documents are created. See `docs/roadmap.md` for the rest of the import gaps.
4. Per-document compile overrides and a "compile with last settings" action.
5. Undo for binder operations, research and database text.
6. Drag-and-drop binder, native file pickers (`rfd`), word-level diff, FDX
   style-mapping dialog, image insertion before first save, packaging.

## Working style

Plan briefly, change small, run build + tests + a screenshot of any screen touched, and report what was verified versus unverified. Keep `docs/notes.md` as the running build log and update `docs/roadmap.md` when priorities shift.
