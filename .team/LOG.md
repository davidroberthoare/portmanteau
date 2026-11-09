# Running log

## Round 0 — assessment & stabilisation

- Read AGENTS.md, README, docs/{architecture,notes,roadmap}, and the core/export/api/mcp/cli
  sources plus the Compile screen.
- Found the workspace at `myScript/`, moved into `app/` by a prior session. Git repo is
  `app/.git`. HEAD is the v1 baseline (crates intact). Build + 29 tests pass.
- Prior agent's committed work = move into app/ + lockfile churn only. It left one broken,
  uncommitted fragment (a divergent whole-app rewrite of `src/ui/editor.rs` referencing
  non-existent `crate::model`/`MyScriptApp`) and a stray `src/ui/theme.rs`. Cleared both.
- Verdict: no functional progress on the three asks; start from v1.
- Decided: (Round 1) folder-package project format with Markdown/Fountain bodies + a small
  JSON manifest; (Round 2) per-format export settings + per-project base style. Full
  design in .team/SPEC.md.

## Round 1 — project folder format (delegated to implementer)

- (pending)

## Round 2 — per-target compile settings + base style

- (pending)

## Round 1 — DONE (commit 955d540)

- Implementer built the folder package (manifest + documents/*.md|fountain + research + snapshots + checkpoints) with lossless Fountain round-trip (incl. a boneyard marker for lone Dialogue, a sensible deviation).
- Build green; 35 tests pass (msc-core 8, msc-api 17, msc-export 7, msc-mcp 3).
- Smoke-tested: folder save/open, legacy .json round-trip, screenshot non-blank (1.2 MB).
- Note: a concurrent process committed unrelated PDF/binder fixes (d7d8f33..c03c578) during the round; no overlap with Round 1 files, build still green.
- Learned: subagents lack `git commit` permission; the Conductor must commit.
- Gate 1 (tester): PASS — 14 independent integration tests; they exposed storage ambiguities for ALL-CAPS Action, custom scene headings and lone Dialogue. Fixed in 7e812b5; full workspace now passes 49 tests.

## Round 1 — Gate 1 (tester) verdict: PASS (at 7e812b5)

- Tester added `crates/msc-api/tests/round1_verification.rs` (14 tests) and verified all 5 criteria.
- Tester (and a concurrent fix) caught a real lossless-round-trip bug at 955d540: SceneHeading
  without INT./EXT. and ALL-CAPS Action were misread as Character. Fixed in 7e812b5 by emitting
  explicit boneyard markers for ambiguous kinds. Accepted baseline is 7e812b5 (49 tests green).
- Open follow-ups folded into Round 2: (RISK3) msc-mcp project_save description + missing
  project_open tool; (RISK4) Save-As still defaults extensionless paths to legacy .json.

## Round 2 — DONE (commit c072dec)

- Per-format ExportSettings (Project.formats) + per-project base style (Project.style),
  seeded in sample::demo(); PDF page numbers/running headers; EPUB cover embedding; four
  new commands (export.settings.get/update, style.get/update) + MCP tools + project_open;
  compile UI per-format persistence + base-style editor + new toggles; Save-As folder default.
- Build green; 58 tests pass (msc-api 10, msc-core 21, msc-export 10, msc-mcp 3, +14 round1).
- Smoke: PDF page numbers on/off differ (footer count 0 vs 17); EPUB cover embeds cover-image/
  cover.xhtml; EPUB settings distinct from PDF; compile screenshots non-blank.
- Gate 1 (tester): pending.

## Round 3 — DONE (font wiring + format validation)

- Wired settings.font into PDF (base-14 mapping), EPUB CSS, HTML CSS, DOCX styles, RTF.
- export.run/settings.get/settings.update now reject unknown/missing formats instead of silently
  targeting the wrong one.
- Build green; 64 tests pass (+6 new: pdf_font mapping, font-in-output for EPUB/HTML/PDF, bogus-format).
- Gate 1 (tester) pending; Gate 2 (torture-tester) pending.

## Round 4 — DONE (data-safety & fidelity fixes)

- Torture-tester (Gate 2) found 4 real bugs; all fixed:
  1. path traversal via hostile ids -> safe_component() sanitisation,
  2. non-atomic folder save -> write to temp sibling + atomic rename swap,
  3. f32 infinity (font_size=1e39) -> `null` lockout -> finite/range validation with clear errors,
  4. Fountain storage Action->Dialogue + dropped empty blocks -> explicit `/* @kind */` marker on EVERY block.
- Build green; 67 tests pass. Reproduced 1e39 lockout before the fix; after, it errors cleanly and the project reopens.
- Removed the torture-tester's scratch `tests/` directory (not part of deliverables).
