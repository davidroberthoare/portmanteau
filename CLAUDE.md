# CLAUDE.md — Portmanteau

Everything an agent needs is in `AGENTS.md`, imported below (one source of truth, so the two files cannot drift apart).

@AGENTS.md

Two rules matter often enough to repeat here:

- **Every visible text goes through `tr()`.** New or changed menus, buttons, labels, messages or welcome-project text need `tr("English")`/`trf(…)`, a note in `assets/languages/context.json` when the text is a single word, a regenerated template (`UPDATE_LANGUAGE_TEMPLATE=1 cargo test the_template`, then rebuild) and updated translations in `assets/languages/*.json`. See `docs/localization.md`.
- **Keep the docs in step.** Any new feature or major change is also written up in the GitHub wiki (drafts in `docs/wiki-draft/`) and, if it changes what the product is or how to run it, in `README.md`.
