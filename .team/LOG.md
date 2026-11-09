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
