# Portmanteau

**An all-in-one, friendly writing studio**: one place to plan, research and write a novel, screenplay, TV script or stage play, then get it out into whatever form you need: a Word file for an editor, a Final Draft file for a producer, an EPUB for e-readers, or a PDF laid out for print.

The aim is that a writer never has to juggle a notes app, a manuscript editor, a screenwriting program and a formatting tool. You keep your research and your story world beside the writing, bring existing work in from other programs, and publish from the same project.

It runs as a desktop program on Linux, Windows and macOS, and **in your web browser** with nothing to install.

It's written in Rust with egui, and I'm building it for my own use and for fun, so it's unfinished, rough in places, and comes with no guarantees. It's not intended for commercial use.

Suggestions, bug reports and ideas are very welcome — open an issue.

<img width="2864" height="1906" alt="image" src="https://github.com/user-attachments/assets/828a721b-dbd9-4897-bb21-901efdbae7f4" />

## Where it stands

Everything below works today unless it says otherwise.

### Write

- One rich-text editor for prose and scripts. Paragraph styles (with "next style" rules, like Enter after a character cue giving dialogue) decide how things look; bold, italic, underline, strikethrough, links and inline pictures are supported. Projects own their style sheets and you can edit them.
- Screenplay, TV and stage formats, with character-name suggestions drawn from the story database and the script.
- A paged view that shows each document as the compiled pages will really look, with real page counts.
- Undo and redo for the open document's text, styles and marks.
- Offline spell-checking (British and American English), a personal dictionary, a distraction-free focus mode, adjustable text size, a daily word goal.
- Find and replace in the open document, and across the whole project with options for case, whole words and regular expressions.

### Plan and keep track

- Binder, corkboard and outline; each document has a synopsis, notes, status, label and word target.
- A story database for characters, places, events and props, with tags and notes. Type `@` in any text to link to an item or a research note; each item shows every place it appears.
- Research notes with attached files, pictures, links and source details. Research never goes into a compile.
- Snapshots and checkpoints, with comparison and restore.
- A project page for title, author, genre, logline, contact details and draft label.

### Bring work in

- Import Fountain, Final Draft (FDX), Word (DOCX) and plain text, with a review screen that shows how the file will be split into documents and how its styles map to yours before anything is added.

### Get work out

- Export to PDF, EPUB, DOCX, Final Draft, Fountain, HTML, Markdown, RTF, plain text, or a project bundle. **File › Export project as zip…** saves the whole project (with its pictures and attachments) as one file for backup or sending, and **Open project zip…** brings one back.
- Each compile is one kind of manuscript (prose, film, TV or stage), drawn from the whole draft or one part of it. TV and stage scripts can compile as one file per script.
- **Ebook and print:** EPUB for e-readers and a PDF with embedded fonts, laid out for the paper size you choose (US Letter by default, plus A4, A5 and trade paperback), with margins, title page, contents and scene numbering you can set per format.
- Not there yet for publishing: print-ready extras such as bleed and cover wraps, ISBN and copyright-page templates, and PDF hyperlinks.

### Everything else

- On the desktop your work stays on your own disk, in a plain folder of small files (rich text, Fountain, JSON) that you can back up or put under version control; in the browser it stays in the browser. No accounts, no telemetry.
- The only thing the desktop program does on the network is check GitHub for a newer version of itself (once a day, and from **Help › Check for Updates…**). It sends nothing about you or your writing, and you can turn it off in Settings. The browser version makes no network requests after the page loads.
- A first launch opens a small **Welcome to Portmanteau** project with instructions and a sample of everything; after that the app reopens the last project you used.

## Install, or try it in your browser

- **In your browser:** open **https://davidroberthoare.github.io/portmanteau/**. It's the same app, with your project kept in the browser (see below). Nothing is uploaded anywhere.
- **On your computer:** download the file for your system from the [Releases page](https://github.com/davidroberthoare/portmanteau/releases): a `.tar.gz` for Linux, a `.zip` for Windows, and a `.zip` holding the app for macOS. Unpack it anywhere you can write to and run `portmanteau`. The builds aren't signed with an Apple or Microsoft certificate, so macOS and Windows will warn the first time you open the app; `docs/releasing.md` explains how to get past that.
- **Updates:** the desktop app tells you when a new version is out (a small icon in the top bar, or **Help › Check for Updates…**) and can install it for you after checking the download's signature. The browser version simply updates when you reload the page.

### The browser version

It's the whole app running in your browser, including the PDF and EPUB maker, spell-check, find and replace, and the Welcome project.

- Your project is saved **inside the browser** a moment after each change, and is there again when you come back to the page on the same browser. Clearing the site's data removes it, so use **File › Export project as zip…** now and then as a backup; **File › Open project zip…** brings one back (a project zip made by the desktop app opens here too, and the other way round).
- **Compile & export** works as usual and downloads the finished PDF, EPUB, Word file and so on.
- Not there yet in the browser: pictures inside text and research attachments (the files in a project zip are carried along untouched, just not shown), importing Fountain, Final Draft or Word files, and clipboard paste of research clippings.

## Run from source

```sh
cargo run
```

I mostly develop and test on Linux. Builds for Windows and macOS are set up in the automatic checks but haven't been tried by hand.

To build the browser version yourself you need the `wasm32-unknown-unknown` Rust target and `wasm-bindgen-cli` (the version in `Cargo.lock`), then `scripts/build-web.sh some-folder --release` and serve that folder with any static web server.

There is also a command-line tool and an MCP server that expose the same actions as the app:

```sh
cargo run -p portmanteau-cli -- commands      # list every command
cargo run -p portmanteau-cli -- check-updates # ask GitHub for a newer release
```

## Known gaps and what's next

- No drag-and-drop in the binder, no tracked changes, no word-level comparison of versions.
- Undo doesn't cover binder changes, research or database text yet; use Versions to go back.
- PDF output has no clickable links; PDF layout is solid but not a full typesetter.
- No lists (bullets or numbering) in the text model.
- The browser version can't show pictures or attachments, or import files, yet (see above).
- Planned next: pictures and importing in the browser, per-document compile overrides, and more publishing options.

More detail on plans is in `docs/roadmap.md` and `docs/release-and-updates-plan.md`; `docs/architecture.md` and `AGENTS.md` describe how the code is laid out.

## Built with, and thanks to

Portmanteau stands on a lot of open-source work. The main pieces:

- [Rust](https://www.rust-lang.org/) and [egui / eframe](https://github.com/emilk/egui): the language and the interface toolkit.
- [krilla](https://github.com/LaurenzV/krilla): writes the PDFs, with embedded fonts.
- [zip](https://github.com/zip-rs/zip2), [roxmltree](https://github.com/RazrFalcon/roxmltree), [serde](https://serde.rs/), [chrono](https://github.com/chronotope/chrono), [regex](https://github.com/rust-lang/regex): file formats, data and search.
- [spellbook](https://github.com/helix-editor/spellbook) (Hunspell-format engine) and the [LibreOffice Hunspell dictionaries](https://github.com/LibreOffice/dictionaries): spell-checking.
- [Liberation fonts](https://github.com/liberationfonts/liberation-fonts) (SIL Open Font License) and [DejaVu Sans](https://dejavu-fonts.github.io/): the bundled typefaces.
- [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) for running in the browser, [rfd](https://github.com/PolyMeilex/rfd) for native file dialogs, [arboard](https://github.com/1Password/arboard) for the clipboard, [ureq](https://github.com/algesten/ureq) for the update check, [minisign](https://jedisct1.github.io/minisign/) (via [minisign-verify](https://github.com/jedisct1/rust-minisign-verify)) for signed updates, and [self-replace](https://github.com/mitsuhiko/self-replace) for installing them.
- File formats and tools I lean on for testing: [Fountain](https://fountain.io/), Final Draft's FDX, EPUB, [poppler](https://poppler.freedesktop.org/) (PDF checks) and [Calibre](https://calibre-ebook.com/) (ebook checks).

**App icon:** the typewriter drawing is `streamline-freehand:content-typewriter` from the **Streamline Freehand (free)** icon set by [Streamline](https://github.com/webalys-hq/streamline-vectors), found through [Iconify](https://icon-sets.iconify.design/streamline-freehand/), licensed [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). I placed it on a rounded square and resized it. Full credits and licences for bundled files are in `THIRD_PARTY.md`.

## Licence

GPL-3.0-or-later — see `LICENSE`. Bundled fonts and dictionaries are listed in `THIRD_PARTY.md`.
