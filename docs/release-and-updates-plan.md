# Release pipeline, updates and the web build — plan

Written 2026-11-22. Status: **Phases 0-3 built, Phase 4 started (2026-11-22): W1 spike and W3 zip done, W2 partly (see the progress log). The CI and release workflows have not run on GitHub yet, so they are unproven until the first push and tag. Next: Phase 4 (web), and the real end-to-end update test described in `docs/releasing.md` once two releases exist.** Work through the
phases in order; each one leaves the app runnable and is worth having on its own.

## In plain language

1. **Every push gets built and tested** on Linux, Windows and macOS by GitHub,
   so a change that breaks one platform shows up straight away.
2. **Pushing a version tag (e.g. `v0.2.0`) makes a release**: GitHub builds the
   three desktop downloads and publishes them on the Releases page with the
   notes from `docs/RELEASE_NOTES.md`, as easyCue3 does.
3. **The app tells you when a new version is out**: a small icon appears in the
   top bar, and **Help › Check for Updates…** checks whenever you ask.
4. **Later, the app installs the update itself**: "Install and restart"
   downloads the right file, checks it really came from you (signature) and
   replaces itself.
5. **Last, a web version** runs the same app in a browser from GitHub Pages.
   Projects are kept in the browser's own storage and go in and out as `.zip`
   files.

## Decisions taken (interview, 2026-11-22)

| Question | Decision |
|---|---|
| What an update does | Notify first (icon + link); in-app install in a later phase |
| Automatic check | On, at most once a day, at startup; a Settings switch turns it off. The menu check always works |
| Desktop packages | Plain archives: Linux `.tar.gz`, Windows `.zip`, macOS universal `.app` in a `.zip` |
| Web build | Full app, projects in browser storage, `.zip` in and out; its own phase after desktop |

Defaults I've picked that you can still change: x86_64 only for Linux and Windows
(no ARM builds yet), macOS universal (Intel + Apple Silicon), stable releases only
(GitHub "pre-release" tags are never offered as updates), no code signing
certificates (see "What you will see as a user").

### Rule changes these decisions need

- **Network**: "No network" becomes "no network except the update check and the
  update download, which only talk to `api.github.com` / `github.com`, send
  nothing about the writer or their projects, and can be switched off."
- **Native only**: becomes "native desktop, plus a web build of the same app."
  Native stays the main target; the web build must not make desktop worse.
- **Command layer**: checking for updates is about the program, not the
  project, so it is **not** a project command or MCP tool. The logic lives in a
  small crate with tests, the CLI gets `check-updates`, and the egui shell calls
  the same crate. The project `.zip` in/out (Phase 4) is a normal command.

---

## Added requirement: the welcome project (owner, 2026-11-22)

A fresh install opens a clearly labelled **Welcome to Portmanteau** project, not
a blank screen: a "Start here" page of getting-started instructions, then one
small sample in every part of the binder (a prose chapter with a saved
version, a screenplay scene, front matter, back matter, a research note, and
a character and a place in the database). After that, the app reopens the last
project; if it cannot be found, it comes back to the welcome project.

Built: `sample::welcome()`, `Workspace::welcome()`, `startup_workspace()` in
`src/app.rs` (tested: first launch, missing last project, found project). The
welcome project lives in memory until the writer saves it. Its "Start here"
page is excluded from compile so it never lands in a manuscript. The Salt Road
sample stays for screenshots and tests (`PORTMANTEAU_PROJECT=welcome` shows the
welcome project in a screenshot). Still to decide: whether the first launch of
a release should also offer to open the Salt Road sample from the Help menu.

## Phase 0 — Groundwork (small, one session)

Things a real release needs that the app doesn't have yet.

- **One version number**: `Cargo.toml` `version` is the only source. Every crate
  uses `version.workspace = true` so they never drift.
- **Help menu** next to Insert: *About Portmanteau* (version, licence, link to
  the repository) and *Check for Updates…* (greyed out until Phase 2).
- **Windows console window**: add `#![cfg_attr(windows, windows_subsystem = "windows")]`
  to `src/main.rs`, or Windows users get a black terminal window behind the app.
- **App icon**: one square PNG (1024 px) in `assets/icon/`, used for the window
  icon, the macOS `.icns` and the Windows `.ico` (embedded with `winresource`).
  *Needs artwork from you, or I'll draw a simple placeholder.*
- **`docs/RELEASE_NOTES.md`** with a `## v0.1.0` section; release notes are taken
  from the newest section, the same as easyCue3.
- **`.github/workflows/ci.yml`**, on every push and pull request:
  - Linux, Windows, macOS: `cargo build --workspace` and `cargo test --workspace`.
  - Linux also installs `poppler-utils` so the PDF export tests really run
    instead of skipping, plus `libgtk-3-dev` (needed by the file dialogs) and
    the usual X11/Wayland headers.
  - Linux runs the `--screenshot` smoke test under `xvfb` and fails on a blank PNG.
  - Cache with `Swatinem/rust-cache`.

Done when: CI is green on all three systems and the Help menu screenshot shows.

## Phase 1 — Release pipeline (one session)

`.github/workflows/release.yml`, modelled on easyCue3's `release.yml`.

- Triggers: a pushed tag `v*`, or run by hand with a tag name.
- **First job checks the tag matches `Cargo.toml`** (tag `v0.2.0` needs
  `version = "0.2.0"`) and stops otherwise. This is what lets the app compare
  its own version against the release.
- Build jobs (all `--release`, tests run first):
  - `portmanteau-linux-x86_64.tar.gz` — the app, the CLI, the MCP server, README,
    LICENSE, THIRD_PARTY.md and a `.desktop` file + icon for anyone who wants a
    menu entry.
  - `portmanteau-windows-x86_64.zip` — `portmanteau.exe`, CLI, MCP server, docs.
  - `portmanteau-macos-universal.zip` — `Portmanteau.app` (both architectures
    joined with `lipo`, an `Info.plist`, the `.icns`), signed **ad hoc**
    (`codesign -s - --deep`) so Apple Silicon will start it. CLI and MCP go
    beside the `.app`.
- Release job: `SHA256SUMS` file for all archives, then
  `softprops/action-gh-release` with the notes. Published straight away (no
  draft), and nothing is published if any platform fails.
- **How you make a release**: bump `version` in `Cargo.toml`, add a section to
  `RELEASE_NOTES.md`, commit, `git tag v0.2.0 && git push --tags`. A short
  `docs/releasing.md` will spell this out.

Done when: a test tag (e.g. `v0.1.0`) produces a release with three archives
that each start on their system.

### What you will see as a user (no paid certificates)

- macOS: the first launch says the developer can't be verified. Right-click ›
  Open once (or System Settings › Privacy & Security › Open Anyway). Updates
  installed by the app itself (Phase 3) don't trigger this again, because only
  files downloaded by browsers are marked "from the internet".
- Windows: SmartScreen may say "Windows protected your PC" › More info › Run anyway.
- An Apple Developer account ($99/yr) or a Windows signing certificate would
  remove these; both can be added to the workflow later without other changes.

## Phase 2 — "An update is available" (one session)

New crate **`portmanteau-update`** (desktop only; no egui, so the CLI can use it):

- `current_version()`, `parse_version("v1.2.3")`, `is_newer(a, b)` — using the
  `semver` crate so `0.10.0 > 0.9.0` and pre-release tags are understood.
- `check() -> Result<Release, CheckError>`: one request to
  `https://api.github.com/repos/davidroberthoare/portmanteau/releases/latest`
  with `ureq` (rustls, 10-second timeout, `User-Agent: portmanteau/<version>`).
  `Release` holds the version, the page URL, the notes and the asset list.
- Pure functions are unit-tested; the network call is not called by any test
  (tests feed it saved JSON instead).

In the app (`src/update.rs` + small UI pieces):

- **Settings** (`AppSettings.updates`): `auto_check` (default on),
  `last_checked` (time), `skipped_version`. Shown in Settings as "Check for
  updates automatically" with "Last checked: …".
- **Startup check**: on a background thread, if `auto_check` is on and the last
  check was over 24 hours ago. Never during `--screenshot` or tests. Failures
  are silent (offline is normal).
- **Toolbar icon**: a small up-arrow badge at the right of the top bar, only
  when a newer, non-skipped version is known. Tooltip "Portmanteau 0.3.0 is
  available". Clicking opens a small panel: version, the first lines of the
  release notes, **Download** (opens the release page), **Skip this version**,
  **Later**. In Phase 3 **Download** becomes **Install and restart**.
- **Help › Check for Updates…**: always checks now, and always answers in the
  same panel: "Checking…", "You're up to date (0.2.0)", "Couldn't reach GitHub —
  try again later", or the update details.
- **CLI**: `portmanteau-cli check-updates` prints the same answer (handy for
  testing the pipeline from a terminal).

Tests: `egui_kittest` — the icon shows for a newer version, hides for an
equal/older/skipped one, the menu entry opens the panel; unit tests for version
compare and the 24-hour throttle. Screenshot of the top bar with the icon
(`PORTMANTEAU_FAKE_UPDATE=0.9.0` shows it without the network).

Done when: an older build run against the real Releases page shows the icon,
and the menu check reports correctly online and offline.

## Phase 3 — Install the update from inside the app (one or two sessions)

Safety first: the app must only ever replace itself with a file **you** made.

- **Signed manifest**: the release workflow writes `latest.json` (version and,
  per platform, file name, size and SHA-256) and signs it with an Ed25519 key.
  The signing key lives only in the GitHub secret `UPDATE_SIGNING_KEY`; the app
  carries the matching public key. A release without a valid signature is
  shown as "update available" but can only be downloaded by hand.
  *One-time job for you: generate the key pair (`docs/releasing.md` will give the
  one command) and paste the private half into the repository's secrets.*
- **Download** on a background thread with a progress bar in the update panel
  and a Cancel button; into a temporary folder beside the app.
- **Check**: size, SHA-256 and signature all match, or nothing is touched.
- **Unpack and swap**:
  - Linux and Windows: replace the running program with the `self-replace`
    crate (Windows can't overwrite a running `.exe`; the crate handles it).
    The CLI and MCP server beside it are replaced too.
  - macOS: unpack the new `Portmanteau.app` next to the old one, then swap the
    two folders by rename.
  - The previous version is kept as `*.old` until the new one has started once,
    then removed; if the new one fails to start, the old one is still there.
- **Before restarting**: if the project has unsaved changes, save it (or ask,
  if it has never been saved). Then start the new version on the same project
  and quit.
- **If the app's folder isn't writable** (installed under `/opt`, `Program
  Files`, a read-only disk image), say so and fall back to opening the release
  page.

Tests: signature and checksum rejection (tampered byte, wrong key, wrong size),
archive unpacking into a temp folder, "folder not writable" fallback. The swap
itself is tested by hand on each system once per change (written up in
`docs/releasing.md`).

Done when: on each of the three systems, an old release offers the new one,
installs it, restarts into it with the same project open.

## Phase 4 — Web version (several sessions)

The same egui app compiled to WebAssembly, served from GitHub Pages
(`https://davidroberthoare.github.io/portmanteau/`). Desktop remains the main
app; the web build gets no updater (a page reload is the update).

### What is already web-friendly

Fonts and spelling dictionaries are compiled in (`include_bytes!`/`include_str!`),
images are drawn from bytes, export produces bytes before writing them, and the
`glow` renderer works in WebGL2.

### What has to change

| Area | Desktop today | Web |
|---|---|---|
| Project storage | folder package via `std::fs` (`portmanteau-api/src/storage.rs`, research attachments and images in `api/src/lib.rs`) | an in-memory project folder, saved to the browser's IndexedDB |
| Open/save dialogs | `rfd` (blocking) | `rfd`'s async dialogs: pick a file to upload, save = download |
| Exports | written to a path | handed to the browser as a download |
| Imports | read from a path | read from an uploaded file's bytes |
| Settings | `~/.config/…` JSON | browser storage (`eframe` storage) |
| Links | `xdg-open` / `open` / `explorer` | `window.open` (http/https only, as now) |
| Clipboard | `arboard` (text + pictures) | egui's browser clipboard, text only at first |
| Research previews | `ffmpeg`, `pdftoppm` | not available: plain file icon |
| Timers | `std::time::Instant` (panics in a browser) | `web-time` crate (same API) |
| Background threads | research previews only | none (not needed once previews are off) |
| CLI, MCP server | built | not built for the web |

### Steps

- **W1 Spike (half a session)**: `cargo check --target wasm32-unknown-unknown`
  for core, import and export. Expected fixes: `uuid` `js` feature, `getrandom`
  web backend, `chrono` `wasmbind`. The unknown is `krilla` (PDF); if it won't
  build, PDF export is left out of the web build at first and the rest goes
  ahead. Write down the findings here before continuing.
- **W2 Storage layer**: a small `Store` trait in `portmanteau-api` (read, write,
  list, remove, rename, exists) with `DiskStore` (today's `std::fs` code, keeping
  every symlink and path check) and `MemStore` (a map of paths to bytes). All
  project, attachment and image file access goes through it. Desktop behaviour
  and all existing tests stay the same; this step is worth doing even without
  the web, because tests can run on `MemStore`.
- **W3 Project as `.zip`**: commands `project.pack` (folder package → `.zip`
  bytes) and `project.unpack`, with CLI, MCP tool, README row and tests. On the
  desktop this becomes File › Export Project as Zip… / Open Project Zip…
  (backups and sending a project to someone). On the web it is how projects get
  in and out.
- **W4 Shell split**: `main.rs` gets a desktop entry and a web entry
  (`eframe::WebRunner`); platform-only code moves behind small functions in one
  `src/platform/` module (`native.rs`, `web.rs`) so widgets don't fill up with
  `#[cfg]`. File dialogs become "request now, result arrives on a later frame"
  on both platforms.
- **W5 Browser projects**: a start page listing projects saved in this browser
  (open, rename, delete, download as zip, upload a zip); saving writes the
  project to IndexedDB; ask the browser to keep the storage
  (`navigator.storage.persist()`); a visible note that projects live only in
  this browser, with a "Download a backup" reminder.
- **W6 Build and publish**: `trunk` builds `index.html` + `.wasm`;
  `.github/workflows/pages.yml` builds on every release tag and publishes to
  GitHub Pages; CI gains a `wasm32` build check so the web build can't silently
  break. *One-time job for you: turn on Pages (source: GitHub Actions) in the
  repository settings.*

Expected size: roughly 10–20 MB of `.wasm` before compression (the fonts and
dictionaries are about 7 MB of that), a few MB over the network after gzip.
Supported: current desktop Chrome, Edge, Firefox and Safari. Phones and tablets
are not a goal.

Done when: the Pages URL opens the app, a new project survives a page reload,
a zip from the desktop opens in the browser and back, and exports download.

---

## Order, size and what's needed from you

| Phase | Size | Needs from you |
|---|---|---|
| 0 Groundwork + CI | 1 session | icon artwork (or accept a placeholder) |
| 1 Release pipeline | 1 session | push the first tag when ready |
| 2 Update notice | 1 session | — |
| 3 In-app install | 1–2 sessions | create the signing key and GitHub secret; try the update on Windows/macOS |
| 4 Web | 3–5 sessions | turn on GitHub Pages |

Phases 2 and 3 only work for versions that already contain them: the first
release with Phase 2 is the first one that can notice the next release, and the
first with Phase 3 is the first that can install one. So it's worth shipping
Phase 2 before making many releases.

## Risks and open points

- **`krilla` on WebAssembly** is unknown until W1; worst case, no PDF on the web
  at first.
- **macOS swap** of a running `.app` is the fiddliest part of Phase 3 and needs
  testing on a real Mac (this machine is Linux).
- **Browser storage can be cleared** by the user or the browser; the backup
  reminder and `storage.persist()` reduce but don't remove the risk.
- **GitHub API limit**: 60 unauthenticated requests an hour per address; one a
  day per user is far below it.
- **ARM Linux/Windows builds** are left out for now; adding one is a few lines
  in the release workflow.

## Progress log

**2026-11-22 — Phases 0, 1 and 2 built.** Verified: `cargo build --workspace`,
all tests (including new ones), screenshots of the Help menu, the welcome
project, the top-bar update icon and the update panel, and the real
`portmanteau-cli check-updates` against GitHub. Not verified (cannot be from this
Linux machine, and nothing has been pushed): the CI and release workflows
themselves, the Windows icon embedding (`build.rs` + `winresource`), the macOS
`.app` packaging, and the Windows/macOS builds of the update check.

Deviations from the plan as first written:
- Phase 0: the app icon is the owner's typewriter drawing on a soft-white
  rounded square (`assets/icon/`); the `.icns` is made on the macOS runner from
  `icon-1024.png`. The Windows console-window flag is release-only.
- The Help menu got *About* and *Releases and source code* in Phase 0, and
  *Check for Updates…* in Phase 2 (a disabled entry would have broken the
  "no dead controls" rule).
- The first public release has no earlier release to compare with: GitHub
  answers 404, which the app reports as "no releases have been published yet",
  not as an error.
- `PORTMANTEAU_FAKE_UPDATE=0.9.0` shows the icon and panel without the network
  (a development aid, like `PORTMANTEAU_VIEW`). Tests never touch the network:
  `Updater::start` does nothing in test builds.
- The update crate also re-exports `Version` so the app needs no `semver`
  dependency of its own.

**2026-11-22 — Phase 3 built** (`crates/portmanteau-update/src/install.rs`,
`src/update.rs`). Verified by tests: signature and checksum acceptance and
rejection (tampered archive, forged list, wrong key, garbage), refusal of
archives that escape their folder, of links, and of downloads from outside this
project's releases, unpacking of `.tar.gz` and `.zip`, the swap of a loose
install and of a macOS bundle on temporary folders, rollback when a swap fails,
the panel's install/blocked/download states. **Not verified**: a real
download and swap of the running program (needs two published releases), the
macOS bundle swap and the Windows self-replace on real machines, and the
release workflow's signing step.

Deviations from the plan as first written:
- Signing covers `SHA256SUMS` (signature `SHA256SUMS.minisig`) instead of a
  separate `latest.json`: one file fewer to keep in step. Tool: minisign, key
  generated by the owner; public key in `update-public-key.pub`.
- Unsaved work blocks the install (the panel says to save first) rather than the
  app saving a never-saved project for the writer. A saved project is saved
  again before the restart; if that fails, the app stays open.
- Windows: the running program is replaced with the `self-replace` crate; the CLI
  and MCP server beside it are replaced by rename-then-copy, keeping `*.old`
  until the next start.

**2026-11-22 — Phase 4 started (web).**
- **W1 spike: done, better than expected.** `portmanteau-core`, `-import`, `-export`
  (including **krilla, the PDF engine**) and `-api` all build for
  `wasm32-unknown-unknown`. The only fix needed was two target-specific features
  in `portmanteau-core` (`uuid/js`, `chrono/wasmbind`). CI now has a `wasm` job so
  they stay that way. So PDF export can be in the web build from the start.
- **W2 storage layer: done the small way.** Instead of a `Store` trait over every
  file operation, saving is split into pure steps: `storage::package_files(project)`
  (a project as a list of files) and `storage::project_from_files(read)` (back
  again). Disk save and load are built on them (all earlier tests unchanged), and
  so are zip and, later, browser storage. Not done: images and research
  attachments (`assets/…`) still go through `std::fs` in the app and API, so the
  first web build will have no pictures or attachments (Insert › Image and Research
  attachments hidden there). A `Store` trait for assets comes after the web build runs.
- **W3 project as zip: done** (`portmanteau-api/src/archive.rs`; commands
  `project.pack`, `project.open_zip`; MCP `project_pack`, `project_open_zip`;
  File › Export project as zip… and Open project zip…). A zip is untrusted:
  unsafe names, links, oversized files, a second project, and an existing target
  folder are all refused (tests in `crates/portmanteau-api/tests/zip_projects.rs`).
- **Next, W4 (shell split).** `cargo check --target wasm32-unknown-unknown -p portmanteau`
  fails first on `getrandom` (pulled in by the update check's TLS stack), which
  shows the shape of the job: put native-only dependencies (`portmanteau-update`,
  `arboard`, `rfd`'s blocking dialogs) behind `cfg(not(target_arch = "wasm32"))`,
  replace `std::time::Instant` (panics in a browser) with the `web-time` crate,
  add a web entry point (`eframe::WebRunner`), then W5 (projects kept in
  IndexedDB) and W6 (`trunk` build and the Pages workflow). Testing the running
  web build needs a browser: headless Chromium opening the built files, which
  has not been tried yet.

### Progress: web version, W4 (2026-11-22)

The whole app now compiles for the browser and runs there: `scripts/build-web.sh`
builds into `../webroot` (wasm-bindgen CLI matching `Cargo.lock`), `web/index.html`
hosts the canvas, and headless Chromium over a throwaway static server draws the
Welcome project. How: desktop-only crates (`arboard`, `rfd`, the update crate)
are target-gated; `src/platform.rs` gives the browser inert file dialogs and
clipboard; `src/update_web.rs` replaces the updater; `web-time` replaces
`std::time::Instant`. Still to do: opening and saving projects in the browser
(W5: IndexedDB plus zip upload/download), images and attachments, a release-size
build, and the Pages workflow (W6).
