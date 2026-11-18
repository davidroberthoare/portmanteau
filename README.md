# Portmanteau

A small desktop writing app for novels, screenplays, TV and stage scripts, written in Rust with egui. I'm building it for my own use and for fun, so it's unfinished, rough in places, and comes with no guarantees. It's not intended for commercial use.

Suggestions, bug reports and ideas are very welcome — open an issue.

<img width="2864" height="1906" alt="image" src="https://github.com/user-attachments/assets/828a721b-dbd9-4897-bb21-901efdbae7f4" />

## What it does so far

- One rich-text editor for prose and scripts. Paragraph styles (with "next style" rules, like Enter after a character cue giving dialogue) decide how things look; bold, italic, underline, strikethrough and inline images are supported.
- Screenplay helpers: style dropdown, character-name suggestions drawn from the story database and the script, and a paged print preview.
- Binder, corkboard, outline, search, a story database (characters, places and so on), research notes, snapshots and a writing calendar.
- Import Fountain, Final Draft (FDX), Word (DOCX) and plain text.
- Export to PDF, EPUB, DOCX, FDX, Fountain, HTML, Markdown, RTF, plain text, or a project bundle.
- Offline spell-checking, light interface, adjustable text size.
- Everything stays on your own disk. No accounts, no network.

## Run it

```sh
cargo run
```

I mostly develop and test on Linux. It should build elsewhere but I haven't tried.

There is also a command-line tool and an MCP server that expose the same actions as the app:

```sh
cargo run -p portmanteau-cli -- commands
```

## Known gaps

- No undo/redo yet.
- PDF and EPUB layout is a simple built-in engine, not a real typesetter, so the preview is only an estimate.
- No drag-and-drop in the binder, no native file dialogs, no tracked changes.

More detail on plans is in `docs/roadmap.md`; `docs/architecture.md` and `AGENTS.md` describe how the code is laid out.

## Licence

GPL-3.0-or-later — see `LICENSE`. Bundled fonts and dictionaries are listed in `THIRD_PARTY.md`.
