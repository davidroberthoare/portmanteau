# Release notes

The newest section is used as the body of the GitHub release. Start each release
with a line `## vX.Y.Z` (matching `version` in the top-level `Cargo.toml`).

## v0.2.2

- **Nothing is saved over your project unless you say so.** A project's folder is now written only by Save (Ctrl+S), or by Save in the question described below. The old behaviour of saving over it every few seconds is gone, so changes you decide you don't want never reach the file.
- **Quitting asks.** Closing the window (or File › Quit) with unsaved changes asks: Save, Don't save, or Cancel. A project that was never saved gets Save… (to choose where it lives). With nothing unsaved, quitting is instant.
- **Safety copy, like a word processor's autorecover.** While a project has unsaved changes, Portmanteau keeps a copy of it, refreshed every two minutes, off to the side (never in your project's folder). If the app stops unexpectedly, the next start offers to bring the changes back, still unsaved. Saving, or choosing Don't save, removes the copy. The copy is also kept if you open another project or install an update with unsaved changes.
- Fixed: project checkpoints were getting larger and larger (each one held a copy of all the earlier ones). They are now just the project.

## v0.2.1

- **Writing statistics.** A new Writing statistics window (inspector, View menu, or click the progress meter) shows time spent typing and reading, words added, edited and deleted, by day, week, hour of day and document, plus how your prose reads (sentence length, reading ease, dialogue share, repeated words).
- **Goals and encouragement.** Set daily and weekly goals for words or minutes of typing. The top-bar meter follows them, and there are streaks, badges and kind messages (all optional).
- Each document's word target now shows progress ("328 / 2500" with a bar).
- A **Compile** button in the top bar compiles again with the last settings; the notice that follows names the file, and clicking it opens it.
- The top-bar search field is gone. Ctrl+F's find bar has a **Search whole project** link that carries your search to the full Search page.
- Paragraphs now have a little space after them by default, and the sample projects no longer use blank lines between paragraphs.
- Pictures: a small anchor marks where each is tied to the text, they can be removed from the right-click window or the Document tab, and the right-click window can be moved.
- A friendly "create the first chapter" prompt in an empty binder.
- The Compile "Font" setting is now the Global font on the Styles tab.
- The status bar says the word total is for the whole project.

## v0.2.0

- **Pictures in your writing.** Insert an image (or drop or paste one in) and
  choose its size, put it left, centre or right, and have the text wrap around it
  with space around it, like in Word. Add a caption and a description. Right-click
  a picture for its settings in a window you can move, or use the list in the
  Document tab. Pictures and captions come out in PDF, Word, EPUB and HTML.
- **Text wraps around pictures in the editor**, not only in the exports.
- **Compile:** an optional cover page in PDF; contents page options (heading
  text, page numbers, dots between title and number); a Global font setting on the
  Styles tab that styles follow unless you give them their own.
- Book layout: trim sizes, inside and outside margins, headers and footers with
  page numbers and chapter names, and chapters starting on right-hand pages.
- **Removed:** the RTF export (Word, PDF, EPUB and the rest remain).
- Fixed: inserting a picture no longer adds a second, duplicate caption.

## v0.1.2

- **Portmanteau now runs in your web browser**, at
  https://davidroberthoare.github.io/portmanteau/ : the whole app, with your
  project saved in the browser, PDF/EPUB/Word and other exports that download,
  and File › Export / Open project as zip to back up or move a project (zips
  made by the desktop app open there, and the other way round).
- In the browser, pictures, research attachments and importing other files are
  not available yet.

## v0.1.1

- Spell-check no longer flags words like "I'm", "I'll" and names that the
  dictionary knows, and suggestions now come from the same dictionary.
- Linux builds now run on older distributions.

## v0.1.0

First public build of Portmanteau: a native, offline writing studio for novels,
screenplays, television and stage plays.

- Binder, corkboard and outline; prose and script editors with styles.
- Research notes, a story database, find and replace, spell-check.
- Snapshots and version comparison.
- Import from Fountain, Final Draft, Word and plain text.
- Export to PDF, EPUB, DOCX, RTF, Final Draft, Fountain, HTML, Markdown and text.
- A command-line tool and an MCP server for the same commands.
