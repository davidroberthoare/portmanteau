//! Compilation and export.
//!
//! One manuscript model, many finish lines: print-ready PDF, reflowable EPUB,
//! editor-friendly DOCX, Final Draft FDX, Fountain, HTML, Markdown, RTF and
//! plain text — plus a project bundle for local archiving and backup.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use portmanteau_core::script;
use portmanteau_core::*;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

mod pdf;
pub use pdf::{
    pdf_layout, pdf_manuscript, pdf_page_count, text_width, Face, Family, PdfLayout,
    PdfLayoutSection, PdfPageLayout, PdfSegment, PdfText,
};

mod content;
mod util;
mod web;
mod epub;
mod docx;
mod layout;
mod script_title;
mod fdx;
mod fountain;
mod html;
mod markdown;
mod rtf;
mod txt;
mod bundle;
use content::*;
use util::*;
use web::*;
use epub::*;
use docx::*;
use layout::*;
use script_title::*;
use fdx::*;
use fountain::*;
use html::*;
use markdown::*;
use rtf::*;
use txt::*;
use bundle::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportInfo {
    pub path: String,
    pub bytes: usize,
    pub words: u32,
    pub format: String,
    pub message: String,
    /// Every file written; more than one when compiling scripts separately.
    #[serde(default)]
    pub files: Vec<String>,
}

pub fn default_filename(project: &Project, kind: ExportKind) -> String {
    let base: String = project
        .title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() { "manuscript" } else { &base };
    format!("{}.{}", base, kind.extension())
}

/// Compile the manuscript. One export is one kind of manuscript (see
/// `CompilePlan`); with `each_separately` a script compile writes one file per
/// script.
pub fn export(
    project: &Project,
    settings: &ExportSettings,
    output: Option<&Path>,
) -> Result<ExportInfo, String> {
    let plan = project.compile_plan(settings);
    let mut settings = settings.clone();
    settings.compile_type = Some(plan.kind);
    if plan.kind.is_script() {
        settings.include_toc = false;
        settings.include_chapter_numbers = false;
    }
    let settings = &settings;
    if settings.each_separately && plan.kind.is_script() {
        let groups = project.compile_groups(&plan);
        if !groups.is_empty() {
            let first = output_path(project, settings, output);
            let dir = first.parent().map(Path::to_path_buf).unwrap_or_default();
            let stem = first
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "manuscript".into());
            let (mut files, mut bytes, mut words) = (Vec::new(), 0usize, 0u32);
            for group in &groups {
                let mut each = settings.clone();
                each.scope = portmanteau_core::CompileScope::Documents(group.ids.clone());
                each.each_separately = false;
                let name = format!("{stem}-{}.{}", slugify(&group.title), settings.kind.extension());
                let info = export_single(project, &each, Some(&dir.join(name)))?;
                bytes += info.bytes;
                words += info.words;
                files.extend(info.files);
            }
            return Ok(ExportInfo {
                path: dir.display().to_string(),
                bytes,
                words,
                format: settings.kind.label().to_string(),
                message: format!(
                    "{} exported · {} separate {} files",
                    settings.kind.label(),
                    files.len(),
                    plan.kind.label().to_lowercase()
                ),
                files,
            });
        }
    }
    export_single(project, settings, output)
}

fn output_path(project: &Project, settings: &ExportSettings, output: Option<&Path>) -> PathBuf {
    match output {
        Some(p) => p.to_path_buf(),
        None => {
            let name = default_filename(project, settings.kind);
            if settings.output_path.trim().is_empty() {
                PathBuf::from("exports").join(name)
            } else {
                PathBuf::from(settings.output_path.trim()).join(name)
            }
        }
    }
}

fn export_single(
    project: &Project,
    settings: &ExportSettings,
    output: Option<&Path>,
) -> Result<ExportInfo, String> {
    let path = output_path(project, settings, output);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not make folder: {e}"))?;
        }
    }

    let bytes: Vec<u8> = match settings.kind {
        ExportKind::Pdf => pdf_manuscript(project, settings),
        ExportKind::Epub => epub_manuscript(project, settings),
        ExportKind::Docx => docx_manuscript(project, settings),
        ExportKind::Fdx => fdx_manuscript(project, settings).into_bytes(),
        ExportKind::Fountain => fountain_manuscript(project, settings).into_bytes(),
        ExportKind::Html => html_manuscript(project, settings).into_bytes(),
        ExportKind::Markdown => markdown_manuscript(project, settings).into_bytes(),
        ExportKind::Rtf => rtf_manuscript(project, settings).into_bytes(),
        ExportKind::Txt => txt_manuscript(project, settings).into_bytes(),
        ExportKind::Bundle => bundle_manuscript(project)?,
    };

    fs::write(&path, &bytes).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    let plan = project.compile_plan(settings);
    let has_images = plan.included.iter().any(|doc| {
        rich_document_has_images(&doc.content)
            || (settings.include_synopsis && rich_document_has_images(&doc.synopsis))
            || (settings.include_notes && rich_document_has_images(&doc.notes))
    });
    // The PDF's own layout is the authority for its page count; the other
    // writers still use the quick per-document estimate.
    let pages: u32 = if settings.kind == ExportKind::Pdf {
        pdf_page_count(project, settings) as u32
    } else {
        plan.included.iter().map(|d| d.pages()).sum()
    };
    let page_word = if pages == 1 { "page" } else { "pages" };
    let message = if has_images {
        format!(
            "{} exported · {pages} {page_word} estimated · Images represented by alt-text placeholders; source assets are not embedded",
            settings.kind.label(),
        )
    } else {
        format!(
            "{} exported · {pages} {page_word} estimated",
            settings.kind.label()
        )
    };
    Ok(ExportInfo {
        path: path.display().to_string(),
        bytes: bytes.len(),
        words: plan.included.iter().map(|d| d.words()).sum(),
        format: settings.kind.label().to_string(),
        message,
        files: vec![path.display().to_string()],
    })
}

#[cfg(test)]
mod tests;
