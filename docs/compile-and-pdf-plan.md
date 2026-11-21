# Compile types, scope and the PDF writer — plan

Two linked pieces of work, delivered as two commits. Decisions were agreed with
the person commissioning the app; this file is the contract for both.

## 1. Why

- A compile that mixes screenplay and prose into one file is not something a
  writer wants. Final Draft and WriterDuet only ever hold one script per file;
  Scrivener lets you mix anything and leaves the writer to filter by hand;
  Celtx exports each typed item on its own. We follow the Celtx model: a project
  may hold several kinds of document, but **each export is one kind**.
- The PDF writer (`build_pdf`, ~800 lines in `portmanteau-export`) draws with the
  four standard PDF fonts, embeds nothing, drops characters outside WinAnsi
  (an en dash became a hyphen), ignores the per-format stylesheets, and lays
  script pages out on the novel's A5 page.

## 2. Commit A — compile types, scope and selection

### Model (`portmanteau-core`)

- `CompileType`: `Prose`, `Screenplay`, `TvScript`, `StagePlay`.
  - Maps to a `Medium` (`Prose` accepts `Prose` and `Note`), to the stylesheet
    id (`prose`, `film-screenplay`, `tv-script`, `stage-play`) and has a label.
- `CompileScope`: `Draft` (everything in the binder, default), `Subtree(id)`
  (a folder or one document and its descendants), `Documents(ids)` (used
  internally for "each script separately").
- New `ExportSettings` fields (all `#[serde(default)]`):
  `compile_type: Option<CompileType>` (`None` = automatic), `scope`,
  `each_separately: bool`.
- `Project::compile_plan(&ExportSettings) -> CompilePlan`:
  - `kind`: the explicit type, otherwise the type of the first included
    document, otherwise the project kind's default.
  - `included`: documents in binder order that are in scope, flagged
    `include_in_export`, not in Research, and match `kind`.
  - `skipped`: every in-scope document that was excluded, with a reason
    ("Different format: stage play", "Excluded from compile", "Research").
- `Project::manuscript_documents()` stays as the unfiltered list for statistics;
  every writer builds its sections from the plan instead.

### Export (`portmanteau-export`)

- `manuscript_sections(project, settings)` uses the plan; front and back matter
  documents only apply to the `Prose` type.
- Script types: no front/back matter, contents or chapter numbering; a script
  title page (title, "written by", author, draft date) when `include_title_page`.
- `each_separately` (script types only): group the included documents — a
  document whose parent is a root-level folder (such as Draft) is its own group;
  documents inside a deeper folder (an episode folder) share one group. One file
  per group, named `<project>-<group title>.<ext>`. `ExportInfo` gains
  `files: Vec<String>`.
- The "N pages estimated" message and `export.preview` use the plan.

### UI (`src/ui/views/compile.rs`)

- A **Manuscript type** selector (Auto / Prose / Film screenplay / TV script /
  Stage play). Auto follows the document selected in the binder when Compile
  opens, then the first included document.
- A **Scope** selector: whole Draft, or the selected binder item.
- Option panels show only what applies to the type (book options for Prose;
  script options otherwise).
- The preview lists included documents and the skipped ones with their reason.
- "Compile each script separately" checkbox for script types.

### Command surface

`export.preview` and `export.run` accept `compile_type`, `scope`
(`"draft"` or a document id) and `each_separately`; the CLI, MCP and README
command table are updated; `export.preview` returns `included` / `skipped`.

### Tests

Type filtering, skip reasons, scope subtree, grouping for separate exports, and
a CLI export of the demo project for each type.

## 3. Commit B — PDF via `krilla`

### Decision

Adopt `krilla` (Typst team; builds on `pdf-writer`; font subsetting, Unicode,
images, outlines, metadata, PDF/A). It deliberately does no layout, which suits
us: we keep our own line breaking and pagination (scripts need exact control)
and krilla only draws finished, positioned lines. If a requirement ever needs a
hack inside krilla rather than on top of it, revisit.

### Structure

1. **Layout** produces `Vec<PdfPage>`, each a list of positioned items
   (text runs with font, size, x, baseline y, colour; rules; images). It lives
   in `portmanteau-export` and has no PDF code.
2. **Drawing** (`pdf_krilla`) turns pages into a PDF with embedded, subsetted
   fonts. The bundled Liberation fonts are embedded: Mono (Courier metrics) for
   script, Serif and Sans for prose, with bold/italic faces. Characters outside
   a font are not dropped.
3. The old `build_pdf` / `pdf_text` / `pdf_font` code is deleted.

### Script pages

Page size follows the compile's **Paper** setting: A4, US Letter, A5 or Trade
paperback. A script project starts on US Letter (the industry default) and the
writer can choose A4 or another size in Page & type. The same choice is honoured
by every page-based writer — PDF, DOCX and RTF — not just PDF.

Margins come from the format's stylesheet (`StyleSheet.margins`); element styles,
italic, alignment, spacing and hanging indents come from the document's own
stylesheet (film, TV, stage); 12 pt Courier at exactly 12 pt leading; page number
top right ("12.", none on the title page, first script page is 1); scene numbers
per the setting; no bold serif document title. The whole script sheet — margins,
type size, leading, spacing and indents — scales with the paper width relative
to Letter, so a smaller page is the editor's sheet reduced and never collapses
dialogue into one-word lines. Letter itself stays at the industry-standard 12 pt
and 1.5"/1". Prose keeps the book layout (paper size, headers, footers, contents,
front/back matter).

The compiler never invents content. In particular it does not add a `FADE IN:`,
`CUT TO:`, `THE END` or any other transition: a script contains exactly the
paragraphs the writer typed. `FADE IN:` appears only when it is a real transition
block in the document.

### Verification (automated, in `cargo test`)

Tests export a PDF and measure it with `pdftotext -bbox` when it is installed
(skipped otherwise): page size follows the chosen paper (US Letter and A4),
fonts embedded (`pdffonts`), left edge of each element within half a point of
the style, characters per line, lines per page, an en dash survives, page count
equals the layout's page count. DOCX and RTF page size follow the same setting.
The compile screen's page estimate must come from the same layout code.

## 4. Out of scope for now

FDX/Fountain/DOCX/EPUB/RTF changes beyond using the shared selection (those are
next), per-act page breaks, bold/underline in element styles, tagged PDF/PDF-A
output (krilla supports it; add when wanted).
