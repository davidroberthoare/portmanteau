# SPEC — Scriptorium: project bundle + per-target compile

## Goal

Three things, in priority order:

1. Assess & stabilise the inherited state, then fill the gaps the user named.
2. Project file format — stop storing a project as one big JSON blob; store it as a
   folder "package" of small, mostly human-readable files.
3. Compile customisation — per-target (per output format) settings, plus a per-project
   base style that every format derives from.

## Non-goals

- Do NOT rewrite the application architecture. Keep the crate split (msc-core,
  msc-export, msc-api, msc-mcp, msc-cli) and the in-memory Project/Document model.
- Do NOT implement manuscript import, drag-and-drop, word-level diff, or a new PDF
  typesetter. Those stay on the roadmap.
- Do NOT add collaboration, network, accounts, or telemetry.
- No long-running processes; --screenshot must keep working.

## Current state (verified)

- Workspace lives in app/. `cargo build --workspace` and `cargo test --workspace` pass
  (29 tests) from app/.
- A previous agent moved the workspace into app/ and churned Cargo.lock, then started
  (and abandoned) a divergent whole-app rewrite. That fragment has been cleared; the
  working tree is clean. No functional work on the three asks had been done, so we start
  from the v1 baseline described in AGENTS.md.

---

## ROUND 1 — Project folder format (persistence)

### Industry rationale (already researched)

Scrivener stores a project as a `.scriv` FOLDER PACKAGE: a small XML index
(project.scrivx) plus one human-readable content file per binder item, with the binder
tree kept in the index. Ulysses and iA Writer store plain Markdown files on disk.
Scriptorium follows the same pattern: a folder package with a small JSON manifest (the
index — structure + short metadata) and one Markdown / Fountain file per document body
(the writing itself). Legacy `.scriptorium.json` files must still open.

### On-disk layout

A project is a directory with extension `.scriptorium`:

    <Slug>.scriptorium/
      project.json                     # manifest (index + metadata, NO long bodies)
      documents/                       # one file per binder document (body only)
        <nn>-<slug>-<id8>.md           #   prose  -> verbatim UTF-8 text
        <nn>-<slug>-<id8>.fountain     #   script -> lossless Fountain (see below)
      research/                        # research note bodies
        <nn>-<slug>-<id8>.md
      snapshots/                       # named document snapshots (body only)
        <docid8>/<snapid8>-<nameslug>.<ext>
      checkpoints/                     # whole-project checkpoints
        <cpid>.json                    #   full Checkpoint JSON incl. payload
      assets/                          # reserved (future binaries: images, PDFs)

- <nn> is the record's `order` (or positional index for research) zero-padded to 2.
- <slug> = slugify(title): lowercase alphanumeric + '-', readability only, NOT used for lookup.
- <id8> = first 8 chars of the record id; this is the canonical key used to re-attach a
  body to its manifest record on load.

### Manifest project.json

It is the Project struct serialised (pretty-printed) with long bodies emptied:
- every Document.text -> "" and Document.blocks -> [],
- every Snapshot.text -> "" and Snapshot.blocks -> [],
- every ResearchItem.body -> "",
- Project.checkpoints -> [] (checkpoint payloads live in files).

Everything else (titles, ids, parent/order, labels, statuses, synopses, notes,
characters, ideas, beats, session_log, export settings, presets, snapshot metadata
[id/name/note/created/words], checkpoint metadata [id/label/created/kind/words]) stays
in the manifest. Bodies are the only thing externalised.

Checkpoints: on save, write each Checkpoint (full JSON, including its `payload` which is
the old single-file Project serialisation) to checkpoints/<cpid>.json. Preserve order on
load deterministically — either write a checkpoints.json index array of
{id,label,created,kind,words} inside the package, or store the metadata array in the
manifest. Do NOT put `payload` in the manifest. Pick one, be consistent, document in
docs/notes.md.

### Filenames are derived, not stored

Do NOT add a file/path field to the domain model (msc-core has no filesystem concerns).
The storage module in msc-api derives each filename from record fields. `save` fully
rewrites documents/, research/, snapshots/ and checkpoints/ each time, so renames/moves
never leave orphans.

### Lossless script storage (Fountain)

Script bodies are stored as Fountain so they are human-readable, but must round-trip
losslessly (all 12 ElementKinds preserved).

- script::to_fountain (interchange export) stays as-is.
- Add script::to_storage_fountain(blocks, medium) and
  script::parse_fountain(text, medium) -> Vec<Block> in crates/msc-core/src/script.rs.
- The three kinds Fountain cannot natively express — Shot, StageDirection, General — are
  emitted with a Fountain BONEYARD marker comment (/* @shot */, /* @stage */,
  /* @general */) on its own line immediately before the block. Standard Fountain readers
  ignore boneyard; parse_fountain reads it back.
- parse_fountain precedence: '# '->Act, '## '->Scene, '> '->Transition, '~'->Lyric,
  '(...)'->Parenthetical, INT./EXT./INT-EXT/I-E/EST.->SceneHeading, ALL-CAPS short->Character,
  otherwise Action — plus the boneyard markers.
- Prose (non-script) bodies are written verbatim as .md and read back verbatim (no
  Markdown parsing).

### Open / save / migrate (crates/msc-api/src/lib.rs)

- New module crates/msc-api/src/storage.rs owns save_folder(project, dir) and
  load_folder(dir) -> Result<Project, String>.
- Workspace::open(path): directory -> load_folder; *.json -> legacy single-file; else error.
- Workspace::save(path): *.json -> legacy save; otherwise -> save_folder (create dir).
  save(None): use self.path if set (respect its format); else default folder
  ~/Scriptorium/<slug>.scriptorium/.
- Update default_save_path() to the folder path.
- Update project.open/project.save catalogue text and MCP descriptions to mention
  "folder or legacy .json".

### UI copy (src/app.rs)

Update the Save/Open modal hint and default-path string to the folder form
(<slug>.scriptorium/), and any `.scriptorium.json` mentions, while still accepting legacy
.json paths.

### Documentation

- docs/architecture.md: replace the "persistence is plain JSON" paragraph.
- docs/notes.md: add an entry for the new format, migration story, Fountain boneyard choice.
- README.md: update the project-file paragraph and default-path/extension mentions.

### Round 1 acceptance criteria (each checkable)

1. cargo build --workspace and cargo test --workspace pass from app/.
2. New tests: msc-core parse_fountain/to_storage_fountain round-trips all 12 kinds;
   msc-api saves the demo to a folder, reloads it, asserts project identical (bodies,
   snapshots, research, checkpoints, settings all match).
3. CLI smoke test:
     ./target/debug/msc-cli project.save '{"path":"/tmp/rt1/the-salt-road.scriptorium"}' --save
     find /tmp/rt1/the-salt-road.scriptorium -type f
     # expects project.json, documents/*.md|*.fountain, research/*.md, snapshots/**, checkpoints/*.json
     ./target/debug/msc-cli project.open '{"path":"/tmp/rt1/the-salt-road.scriptorium"}'
   project.open must succeed and document.list after it must match the demo.
4. Legacy round-trip: save demo to /tmp/x.scriptorium.json, reopen, stats unchanged.
5. --screenshot still writes a populated PNG.

---

## ROUND 2 — Per-target compile settings + base style

### Model changes (crates/msc-core/src/model.rs)

    pub struct ProjectStyle {           // project-wide base, per medium
        pub prose_font: String,          // "Liberation Serif"
        pub prose_size: f32,             // 11.0
        pub prose_leading: f32,          // 1.4
        pub script_font: String,         // "Courier New"
        pub script_size: f32,            // 12.0
        pub script_leading: f32,         // 1.0
    }

Extend ExportSettings with: include_page_numbers (bool, default true, PDF only),
include_running_headers (bool, default false, PDF only), cover_image (String, default "",
EPUB asset filename/path).

Add to Project: `style: ProjectStyle` and `formats: Vec<ExportSettings>`.

Provide Project::settings_for(kind) -> ExportSettings (returns stored entry or seeds a
default from ProjectStyle) and Project::set_settings(kind, settings). Project::new seeds
nothing eagerly. sample::demo() still builds; optionally seed formats so PDF vs EPUB
differ.

Serde: new fields use #[serde(default)] so old files/manifests load (AGENTS.md section 12).

### Effective settings at export

Workspace::execute("export.run") resolves the target format's settings:
parse format; settings = project.settings_for(kind); apply payload overrides; persist via
set_settings; call msc_export::export.

### Export engine behaviour (crates/msc-export/src/lib.rs)

- PDF: honour include_page_numbers (footer page number, omit when false) and
  include_running_headers (running head with title/author, omit when false), prose path.
- EPUB: when cover_image non-empty, include cover.xhtml with <img>, add image to manifest
  with properties="cover-image", reference <meta name="cover">. page-number/header fields
  intentionally ignored.
- Others (DOCX/FDX/Fountain/HTML/MD/RTF/TXT): page-number/header/cover are no-ops.
- Fonts: PDF(prose), EPUB CSS, HTML CSS, DOCX styles.xml, RTF honour the format's
  font/font_size/line_spacing where a mechanism already exists.

### New commands + MCP

- export.settings.get {"format":"PDF"}
- export.settings.update {"format":"PDF", ...fields...}
- style.get {}
- style.update {...fields...}
Update command_catalogue(), README command table, and msc-mcp tool definitions/mappings.

### Compile UI (src/ui/views/compile.rs)

- Switching format loads that format's saved settings into the editable draft; edits
  persist back to formats (PDF and EPUB stop sharing one blob).
- Add a base style section (prose font/size/leading + script font/size/leading) editing
  project.style.
- Add toggles: Page numbers, Running headers (print-only hint), Cover image text field
  (EPUB hint). Keep existing matter toggles (now per-format).

### Round 2 acceptance criteria

1. cargo build --workspace / cargo test --workspace pass.
2. New tests: ProjectStyle defaults; settings_for seeds missing + returns stored; serde
   round-trip keeps new fields; old JSON without new fields still deserialises; changing
   PDF settings does not change EPUB settings; export.settings.get/style.get round-trip.
3. Real-file checks: PDF page numbers appear iff include_page_numbers set.
4. EPUB with cover set contains cover-image/cover.xhtml; EPUB without does not.
5. --screenshot still works; Compile screen screenshot populated.
