//! A project as one `.zip`: the folder package ([`crate::storage`]) plus its
//! `assets/` files. Pure over bytes, so it works the same on the desktop (a
//! backup, or a project sent to someone) and in a browser, where a project has
//! no folder. A zip is untrusted input: names that leave the package, links and
//! oversized files are refused.

use std::io::{Cursor, Read, Write};

use portmanteau_core::Project;

use crate::storage::{self, PackageFiles};

/// Limits on what a zip may unpack to (a zip-bomb guard).
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 50_000;

/// A project read back from a zip, with its attachment files.
pub struct Unpacked {
    pub project: Project,
    /// Package-relative paths under `assets/` and their bytes.
    pub assets: PackageFiles,
}

/// The project's package files plus `assets`, as zip bytes. The paths in
/// `assets` are package-relative (`assets/images/…`).
pub fn pack_zip(project: &Project, assets: PackageFiles) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut files = storage::package_files(project)?;
    files.extend(assets);
    for (path, bytes) in files {
        zip.start_file(path.as_str(), options).map_err(|e| format!("Could not write the zip: {e}"))?;
        zip.write_all(&bytes).map_err(|e| format!("Could not write the zip: {e}"))?;
    }
    zip.finish().map(|c| c.into_inner()).map_err(|e| format!("Could not write the zip: {e}"))
}

/// Read a project zip. A zip made by [`pack_zip`] has `project.json` at its top;
/// one made by zipping the project folder in a file manager has a single folder
/// around it, which is looked through.
pub fn unpack_zip(bytes: &[u8]) -> Result<Unpacked, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("Not a zip file: {e}"))?;
    if zip.len() > MAX_ENTRIES {
        return Err("The zip has too many files to be a project".into());
    }
    let mut files: PackageFiles = Vec::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("Could not read the zip: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        if entry.is_symlink() {
            return Err("The zip contains a link, which a project never does".into());
        }
        let Some(name) = entry.enclosed_name() else {
            return Err(format!("The zip has an unsafe file name: {}", entry.name()));
        };
        let path = name.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
        if path.starts_with("__MACOSX/") || path.ends_with(".DS_Store") {
            continue;
        }
        if entry.size() > MAX_FILE_BYTES {
            return Err(format!("{path} is larger than any project file should be"));
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry
            .by_ref()
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut data)
            .map_err(|e| format!("Could not read {path}: {e}"))?;
        total += data.len() as u64;
        if data.len() as u64 > MAX_FILE_BYTES || total > MAX_TOTAL_BYTES {
            return Err("The zip unpacks to an unreasonable size".into());
        }
        files.push((path, data));
    }

    // Look through one wrapping folder (…/My Novel.portmanteau/project.json).
    let prefix = if files.iter().any(|(p, _)| p == "project.json") {
        String::new()
    } else {
        let wrappers: Vec<&str> = files
            .iter()
            .filter_map(|(p, _)| p.strip_suffix("/project.json"))
            .filter(|dir| !dir.contains('/'))
            .collect();
        match wrappers.as_slice() {
            [one] => format!("{one}/"),
            _ => return Err("Not a Portmanteau project: the zip has no project.json".into()),
        }
    };
    let files: PackageFiles = files
        .into_iter()
        .filter_map(|(p, b)| p.strip_prefix(&prefix).map(|rest| (rest.to_string(), b)))
        .collect();

    let lookup: std::collections::HashMap<&str, &Vec<u8>> =
        files.iter().map(|(p, b)| (p.as_str(), b)).collect();
    let mut project = storage::project_from_files(&|path| lookup.get(path).map(|b| (*b).clone()))?;
    project.ensure_style_sheets();
    let assets = files.iter().filter(|(p, _)| p.starts_with("assets/")).cloned().collect();
    Ok(Unpacked { project, assets })
}
