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
- No backwards compatibility with old project files until a major release (owner's explicit decision). Don't add migrations/aliases.
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

- Build green, 193 tests pass. Last commit `afcb0f4` (Letter default).
- **Uncommitted, working:** `crates/portmanteau-export/src/lib.rs` embeds bundled Liberation fonts in EPUB, builds a font stack with a Liberation fallback, honours italic in prose styles, and drops the first-paragraph indent override in HTML. Review, run the export tests, then commit.
- Last OpenCode session was "Script formatting in the UI" (ended `failed`); its exact stopping point was not inspected. Treat the screenshots in `../shots/` (newest: script/compile) as the best hint, and verify script-editor behaviour by hand.

## Next work (priority order)

1. Commit the EPUB font work above after review.
2. **Undo/redo** for all rich editors (largest missing editing feature; see roadmap "Planned: undo / redo").
3. **Multi-page view**: Continuous vs Print/Paged, with page count (`../notes/next_tasks.md`). Preview estimate currently differs from real export pagination.
4. Spellcheck in every text editor (prose/script done; confirm other fields).
5. Per-project, per-target, per-element compile customisation (font, margins, spacing, e.g. title-page font differing between PDF and EPUB): model exists; finish UI coverage and make all writers (DOCX/RTF too) consume it.
6. Cleanup: remove dead `legacy_script_page` in `src/ui/editor.rs` and unused `insert_image` on `InlineToolbarResult`; fix the 6 warnings in `portmanteau-export`.
7. Later: drag-and-drop binder, native file pickers (`rfd`), word-level diff, FDX style-mapping dialog, image insertion before first save, packaging.

Open question from the owner's notes: a rename away from "Scriptorium" was decided as **Portmanteau**; confirm no remaining "Scriptorium" strings (`grep -ri scriptorium .`).

## Working style

Plan briefly, change small, run build + tests + a screenshot of any screen touched, and report what was verified versus unverified. Keep `docs/notes.md` as the running build log and update `docs/roadmap.md` when priorities shift.
