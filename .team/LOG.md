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
