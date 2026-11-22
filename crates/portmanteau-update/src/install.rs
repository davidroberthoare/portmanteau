//! Installing an update: download, verify, unpack, swap, restart.
//!
//! Trust comes from one signed file. Every release carries `SHA256SUMS` and
//! `SHA256SUMS.minisig`, signed with the owner's private key (kept only in
//! GitHub's secrets). The program carries the matching public key. An archive
//! is installed only if (1) the signature on `SHA256SUMS` is valid for that
//! key and (2) the archive's SHA-256 is the one listed there. Anything else
//! leaves the installed program untouched.
//!
//! The pieces that touch files take paths, so they are tested on temporary
//! folders; [`install_update`] is the thin layer that points them at the
//! running program.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

use crate::{Asset, Release};

/// The owner's minisign public key (key id 5546518CFF710996). The matching
/// secret key is the GitHub secret `UPDATE_SIGNING_KEY`; see `docs/releasing.md`.
/// The same file is read by the release workflow to check its own signature.
pub const PUBLIC_KEY: &str = include_str!("../update-public-key.pub");

/// Where release files are allowed to come from.
pub const DOWNLOAD_PREFIX: &str = "https://github.com/davidroberthoare/portmanteau/releases/download/";

const SUMS_NAME: &str = "SHA256SUMS";
const SIG_NAME: &str = "SHA256SUMS.minisig";
/// No release archive is anywhere near this large; a bigger one is refused.
const MAX_ARCHIVE_BYTES: u64 = 400 * 1024 * 1024;
const MAX_SMALL_BYTES: u64 = 1024 * 1024;
/// Unpacked size limit (a decompression-bomb guard).
const MAX_UNPACKED_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug)]
pub enum InstallError {
    /// This system has no packaged build, or the release lacks one.
    Unsupported(String),
    Network(String),
    /// The signature, checksum or archive did not check out: nothing was installed.
    Untrusted(String),
    /// The install location can't be written to (installed system-wide, read-only disk image).
    NotWritable(PathBuf),
    Cancelled,
    Io(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::Unsupported(why) => write!(f, "This update can't be installed from inside the app: {why}"),
            InstallError::Network(why) => write!(f, "The download failed: {why}"),
            InstallError::Untrusted(why) => write!(f, "The download was not trusted, so nothing was installed: {why}"),
            InstallError::NotWritable(dir) => write!(
                f,
                "Portmanteau is installed somewhere this account can't change ({}). Download the new version from the release page instead.",
                dir.display()
            ),
            InstallError::Cancelled => write!(f, "Cancelled"),
            InstallError::Io(why) => write!(f, "Could not install the update: {why}"),
        }
    }
}

impl std::error::Error for InstallError {}

fn io(error: impl std::fmt::Display) -> InstallError {
    InstallError::Io(error.to_string())
}

// ---- Trust ---------------------------------------------------------------------

/// Check that `sig` is a valid signature of `sums` by the key in `public_key`.
pub fn verify_signature(public_key: &str, sums: &[u8], sig: &str) -> Result<(), InstallError> {
    let key = minisign_verify::PublicKey::decode(public_key)
        .map_err(|e| InstallError::Untrusted(format!("the built-in public key is unreadable ({e})")))?;
    let signature = minisign_verify::Signature::decode(sig)
        .map_err(|e| InstallError::Untrusted(format!("the signature file is unreadable ({e})")))?;
    key.verify(sums, &signature, false)
        .map_err(|e| InstallError::Untrusted(format!("the signature does not match ({e})")))
}

/// The checksum listed for `name` in a `sha256sum`-style file.
pub fn expected_sha256(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        let file = file.trim().trim_start_matches('*');
        (file == name && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Check a downloaded archive against the signed list. The only way bytes get
/// from "downloaded" to "trusted".
pub fn verify_archive(public_key: &str, sums: &[u8], sig: &str, name: &str, archive: &[u8]) -> Result<(), InstallError> {
    verify_signature(public_key, sums, sig)?;
    let sums = std::str::from_utf8(sums).map_err(|_| InstallError::Untrusted("the checksum list is not text".into()))?;
    let want = expected_sha256(sums, name)
        .ok_or_else(|| InstallError::Untrusted(format!("{name} is not in the signed checksum list")))?;
    if sha256_hex(archive) != want {
        return Err(InstallError::Untrusted(format!("{name} does not match its signed checksum")));
    }
    Ok(())
}

// ---- Finding the right download ---------------------------------------------------

/// The archive made for this system, if there is one.
pub fn platform_archive_name() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("portmanteau-linux-x86_64.tar.gz")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("portmanteau-windows-x86_64.zip")
    } else if cfg!(target_os = "macos") {
        Some("portmanteau-macos-universal.zip")
    } else {
        None
    }
}

/// The three files an install needs from a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub archive: Asset,
    pub sums: Asset,
    pub sig: Asset,
}

pub fn plan_for(release: &Release, archive_name: Option<&str>) -> Result<Plan, InstallError> {
    let archive_name = archive_name
        .ok_or_else(|| InstallError::Unsupported("there is no build for this kind of computer".into()))?;
    let find = |name: &str| {
        release
            .assets
            .iter()
            .find(|a| a.name == name)
            .filter(|a| a.url.starts_with(DOWNLOAD_PREFIX))
            .cloned()
            .ok_or_else(|| InstallError::Unsupported(format!("this release has no {name}")))
    };
    Ok(Plan { archive: find(archive_name)?, sums: find(SUMS_NAME)?, sig: find(SIG_NAME)? })
}

/// Can this release be installed from inside the app on this system?
pub fn can_install(release: &Release) -> bool {
    plan_for(release, platform_archive_name()).is_ok()
}

// ---- Downloading -------------------------------------------------------------------

/// Download one file into memory, reporting `(bytes so far, total if known)`.
pub fn download(
    url: &str,
    limit: u64,
    progress: &dyn Fn(u64, Option<u64>),
    cancel: &AtomicBool,
) -> Result<Vec<u8>, InstallError> {
    if !url.starts_with(DOWNLOAD_PREFIX) {
        return Err(InstallError::Untrusted("the download address is not this project's releases".into()));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(15)))
        .timeout_global(Some(std::time::Duration::from_secs(600)))
        .build()
        .into();
    let response = agent
        .get(url)
        .header("User-Agent", concat!("portmanteau/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| InstallError::Network(e.to_string()))?;
    let total = response.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
    if total.is_some_and(|t| t > limit) {
        return Err(InstallError::Untrusted("the file is larger than any release should be".into()));
    }
    let mut body = response.into_body().into_reader();
    let mut out = Vec::with_capacity(total.unwrap_or(0).min(limit) as usize);
    let mut chunk = [0u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(InstallError::Cancelled);
        }
        let n = body.read(&mut chunk).map_err(|e| InstallError::Network(e.to_string()))?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..n]);
        if out.len() as u64 > limit {
            return Err(InstallError::Untrusted("the file is larger than any release should be".into()));
        }
        progress(out.len() as u64, total);
    }
    Ok(out)
}

/// Fetch the signed checksum list and the archive, and return the archive's
/// bytes only once they are verified.
pub fn download_verified(
    plan: &Plan,
    stage: &dyn Fn(&str),
    progress: &dyn Fn(u64, Option<u64>),
    cancel: &AtomicBool,
) -> Result<Vec<u8>, InstallError> {
    stage("Checking the signature…");
    let sums = download(&plan.sums.url, MAX_SMALL_BYTES, &|_, _| {}, cancel)?;
    let sig = download(&plan.sig.url, MAX_SMALL_BYTES, &|_, _| {}, cancel)?;
    let sig = String::from_utf8(sig).map_err(|_| InstallError::Untrusted("the signature file is not text".into()))?;
    verify_signature(PUBLIC_KEY, &sums, &sig)?;
    stage("Downloading…");
    let archive = download(&plan.archive.url, MAX_ARCHIVE_BYTES, progress, cancel)?;
    stage("Verifying…");
    verify_archive(PUBLIC_KEY, &sums, &sig, &plan.archive.name, &archive)?;
    Ok(archive)
}

// ---- Unpacking ---------------------------------------------------------------------

/// Unpack a release archive (`.tar.gz` or `.zip`) into `dest`, which must be
/// an empty or new folder. Links, absolute paths and `..` are refused. Returns
/// the single top-level folder inside, e.g. `portmanteau-linux-x86_64`.
pub fn unpack(archive: &[u8], name: &str, dest: &Path) -> Result<PathBuf, InstallError> {
    fs::create_dir_all(dest).map_err(io)?;
    if name.ends_with(".tar.gz") {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(archive));
        let mut total = 0u64;
        for entry in tar.entries().map_err(io)? {
            let mut entry = entry.map_err(io)?;
            let kind = entry.header().entry_type();
            if !(kind.is_file() || kind.is_dir()) {
                return Err(InstallError::Untrusted("the archive contains a link or special file".into()));
            }
            total += entry.header().size().unwrap_or(0);
            if total > MAX_UNPACKED_BYTES {
                return Err(InstallError::Untrusted("the archive unpacks to an unreasonable size".into()));
            }
            // `unpack_in` refuses paths that would leave `dest`.
            if !entry.unpack_in(dest).map_err(io)? {
                return Err(InstallError::Untrusted("the archive tries to write outside its folder".into()));
            }
        }
    } else if name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(io)?;
        let mut total = 0u64;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).map_err(io)?;
            let Some(relative) = file.enclosed_name() else {
                return Err(InstallError::Untrusted("the archive tries to write outside its folder".into()));
            };
            if file.is_symlink() {
                return Err(InstallError::Untrusted("the archive contains a link".into()));
            }
            let target = dest.join(relative);
            if file.is_dir() {
                fs::create_dir_all(&target).map_err(io)?;
                continue;
            }
            total += file.size();
            if total > MAX_UNPACKED_BYTES {
                return Err(InstallError::Untrusted("the archive unpacks to an unreasonable size".into()));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(io)?;
            }
            let mut out = fs::File::create(&target).map_err(io)?;
            std::io::copy(&mut file, &mut out).map_err(io)?;
            out.flush().map_err(io)?;
            #[cfg(unix)]
            if let Some(mode) = file.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&target, fs::Permissions::from_mode(mode & 0o777)).map_err(io)?;
            }
        }
    } else {
        return Err(InstallError::Unsupported(format!("unknown archive type: {name}")));
    }
    let mut tops: Vec<PathBuf> = fs::read_dir(dest)
        .map_err(io)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    if tops.len() != 1 || !tops[0].is_dir() {
        return Err(InstallError::Untrusted("the archive does not hold exactly one folder".into()));
    }
    Ok(tops.remove(0))
}

// ---- Swapping files ---------------------------------------------------------------

/// Is `dir` somewhere this account can create files?
pub fn can_write_to(dir: &Path) -> bool {
    let probe = dir.join(format!(".portmanteau-write-test-{}", std::process::id()));
    let ok = fs::File::create(&probe).is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

/// Where the running program lives, and what kind of install it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Layout {
    /// A loose program file in a folder (Linux, Windows, or a macOS binary run directly).
    Loose { exe: PathBuf, dir: PathBuf },
    /// Inside a macOS `Portmanteau.app` bundle.
    MacBundle { bundle: PathBuf, exe: PathBuf },
}

impl Layout {
    pub fn detect(current_exe: &Path) -> Layout {
        // …/Portmanteau.app/Contents/MacOS/portmanteau
        let bundle = current_exe
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .filter(|b| b.extension().is_some_and(|e| e == "app"));
        match bundle {
            Some(bundle) if current_exe.parent().and_then(Path::file_name).is_some_and(|n| n == "MacOS") => {
                Layout::MacBundle { bundle: bundle.to_path_buf(), exe: current_exe.to_path_buf() }
            }
            _ => Layout::Loose {
                exe: current_exe.to_path_buf(),
                dir: current_exe.parent().map(Path::to_path_buf).unwrap_or_default(),
            },
        }
    }

    /// The folder files are replaced in. Staging happens here too, so every
    /// swap is a rename inside one file system.
    pub fn install_dir(&self) -> PathBuf {
        match self {
            Layout::Loose { dir, .. } => dir.clone(),
            Layout::MacBundle { bundle, .. } => bundle.parent().map(Path::to_path_buf).unwrap_or_default(),
        }
    }

    /// What to run to start the new version.
    pub fn restart_path(&self) -> PathBuf {
        match self {
            Layout::Loose { exe, .. } | Layout::MacBundle { exe, .. } => exe.clone(),
        }
    }
}

const EXTRA_PROGRAMS: [&str; 2] = ["portmanteau-cli", "portmanteau-mcp"];

fn exe_name(base: &str) -> String {
    if cfg!(windows) { format!("{base}.exe") } else { base.to_string() }
}

/// Replace `target` with `new`, keeping the old file as `<target>.old` so a
/// failed copy is undone. Works for a program that is currently running.
fn replace_file(target: &Path, new: &Path) -> Result<(), InstallError> {
    let old = old_name(target);
    let _ = fs::remove_file(&old);
    let had_old = target.exists();
    if had_old {
        fs::rename(target, &old).map_err(io)?;
    }
    if let Err(error) = fs::copy(new, target) {
        if had_old {
            let _ = fs::rename(&old, target);
        }
        return Err(io(error));
    }
    Ok(())
}

fn old_name(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".old");
    PathBuf::from(name)
}

/// Replace the macOS bundle with the new one, keeping the old as `<bundle>.old`.
fn swap_bundle(bundle: &Path, new_bundle: &Path) -> Result<(), InstallError> {
    let old = old_name(bundle);
    if old.exists() {
        fs::remove_dir_all(&old).map_err(io)?;
    }
    fs::rename(bundle, &old).map_err(io)?;
    if let Err(error) = fs::rename(new_bundle, bundle) {
        let _ = fs::rename(&old, bundle);
        return Err(io(error));
    }
    Ok(())
}

/// Put the unpacked `package` (the folder inside the archive) in place of the
/// installed program. `replace_running` swaps the running program's own file;
/// the real one is `self_replace::self_replace`, a test passes a stand-in.
pub fn swap_in(
    package: &Path,
    layout: &Layout,
    replace_running: &dyn Fn(&Path) -> std::io::Result<()>,
) -> Result<(), InstallError> {
    let install_dir = layout.install_dir();
    match layout {
        Layout::MacBundle { bundle, .. } => {
            let new_bundle = package.join("Portmanteau.app");
            if !new_bundle.is_dir() {
                return Err(InstallError::Untrusted("the archive has no Portmanteau.app".into()));
            }
            swap_bundle(bundle, &new_bundle)?;
        }
        Layout::Loose { .. } => {
            let new_exe = package.join(exe_name("portmanteau"));
            if !new_exe.is_file() {
                return Err(InstallError::Untrusted("the archive has no portmanteau program".into()));
            }
            replace_running(&new_exe).map_err(io)?;
        }
    }
    // The command-line tool and MCP server travel with the app; update the
    // ones that are installed beside it.
    for base in EXTRA_PROGRAMS {
        let name = exe_name(base);
        let (new, target) = (package.join(&name), install_dir.join(&name));
        if new.is_file() && target.exists() {
            replace_file(&target, &new)?;
        }
    }
    Ok(())
}

/// Remove files an earlier update left behind (`*.old`) next to the program.
/// Call once at startup; failures are ignored.
pub fn clean_up_old_files(current_exe: &Path) {
    let layout = Layout::detect(current_exe);
    let dir = layout.install_dir();
    let mut candidates: Vec<PathBuf> = EXTRA_PROGRAMS.iter().map(|b| old_name(&dir.join(exe_name(b)))).collect();
    match &layout {
        Layout::MacBundle { bundle, .. } => {
            let old = old_name(bundle);
            if old.is_dir() {
                let _ = fs::remove_dir_all(old);
            }
        }
        Layout::Loose { exe, .. } => candidates.push(old_name(exe)),
    }
    for path in candidates {
        let _ = fs::remove_file(path);
    }
    // Staging folders from an update that was interrupted.
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(".portmanteau-update-") {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

/// What a finished install hands back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub restart_path: PathBuf,
}

/// The whole install: download and verify, unpack beside the program, swap.
/// Call from a background thread. `current_exe` must be taken *before* the
/// swap (on Linux the running file's path reads "(deleted)" afterwards).
pub fn install_update(
    release: &Release,
    current_exe: &Path,
    stage: &dyn Fn(&str),
    progress: &dyn Fn(u64, Option<u64>),
    cancel: &AtomicBool,
) -> Result<Installed, InstallError> {
    let plan = plan_for(release, platform_archive_name())?;
    let layout = Layout::detect(current_exe);
    let dir = layout.install_dir();
    if !can_write_to(&dir) {
        return Err(InstallError::NotWritable(dir));
    }
    let archive = download_verified(&plan, stage, progress, cancel)?;
    if cancel.load(Ordering::Relaxed) {
        return Err(InstallError::Cancelled);
    }
    stage("Installing…");
    let staging = dir.join(format!(".portmanteau-update-{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    let result = unpack(&archive, &plan.archive.name, &staging)
        .and_then(|package| swap_in(&package, &layout, &|new| self_replace::self_replace(new)));
    let _ = fs::remove_dir_all(&staging);
    result?;
    Ok(Installed { restart_path: layout.restart_path() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// A throwaway key pair, the public half in `.minisign` text form, and a
    /// signature over `message` made the way the release workflow does.
    fn sign(message: &[u8]) -> (String, String) {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let public = pair.pk.to_box().unwrap().to_string();
        let sig = minisign::sign(Some(&pair.pk), &pair.sk, Cursor::new(message), Some("test"), None).unwrap();
        (public, sig.to_string())
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("portmanteau-install-{tag}-{}-{}", std::process::id(), sha256_hex(tag.as_bytes())));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tar_gz(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, body) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, *body).unwrap();
        }
        let tar = builder.into_inner().unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&tar).unwrap();
        gz.finish().unwrap()
    }

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, body) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default().unix_permissions(0o755)).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    // ---- trust -----------------------------------------------------------------

    #[test]
    fn the_built_in_public_key_is_the_owners() {
        let key = minisign_verify::PublicKey::decode(PUBLIC_KEY).expect("the embedded key parses");
        let _ = key;
        assert!(PUBLIC_KEY.contains("5546518CFF710996"));
    }

    #[test]
    fn a_good_signature_and_checksum_are_accepted() {
        let archive = b"pretend archive";
        let sums = format!("{}  portmanteau-linux-x86_64.tar.gz\n", sha256_hex(archive));
        let (public, sig) = sign(sums.as_bytes());
        verify_archive(&public, sums.as_bytes(), &sig, "portmanteau-linux-x86_64.tar.gz", archive).unwrap();
    }

    #[test]
    fn a_changed_archive_is_refused() {
        let sums = format!("{}  a.zip\n", sha256_hex(b"real"));
        let (public, sig) = sign(sums.as_bytes());
        let error = verify_archive(&public, sums.as_bytes(), &sig, "a.zip", b"tampered").unwrap_err();
        assert!(matches!(error, InstallError::Untrusted(_)), "{error}");
    }

    #[test]
    fn a_changed_checksum_list_is_refused() {
        let sums = format!("{}  a.zip\n", sha256_hex(b"real"));
        let (public, sig) = sign(sums.as_bytes());
        // An attacker swaps in their own file and rewrites the list to match.
        let forged = format!("{}  a.zip\n", sha256_hex(b"evil"));
        assert!(verify_archive(&public, forged.as_bytes(), &sig, "a.zip", b"evil").is_err());
    }

    #[test]
    fn a_signature_from_another_key_is_refused() {
        let sums = format!("{}  a.zip\n", sha256_hex(b"real"));
        let (_, sig_from_stranger) = sign(sums.as_bytes());
        let (owners_public, _) = sign(b"unrelated");
        assert!(verify_archive(&owners_public, sums.as_bytes(), &sig_from_stranger, "a.zip", b"real").is_err());
    }

    #[test]
    fn the_real_key_rejects_a_stranger_and_garbage() {
        let sums = b"x  a.zip\n";
        let (_, sig) = sign(sums);
        assert!(verify_signature(PUBLIC_KEY, sums, &sig).is_err());
        assert!(verify_signature(PUBLIC_KEY, sums, "not a signature").is_err());
    }

    #[test]
    fn an_archive_missing_from_the_list_is_refused() {
        let sums = format!("{}  other.zip\n", sha256_hex(b"x"));
        let (public, sig) = sign(sums.as_bytes());
        assert!(verify_archive(&public, sums.as_bytes(), &sig, "a.zip", b"x").is_err());
    }

    #[test]
    fn checksum_lines_are_read_like_sha256sum_writes_them() {
        let hash = "a".repeat(64);
        let text = format!("{hash}  one.zip\n{}  *two.zip\nnot-a-line\n", "B".repeat(64));
        assert_eq!(expected_sha256(&text, "one.zip"), Some(hash));
        assert_eq!(expected_sha256(&text, "two.zip"), Some("b".repeat(64)));
        assert_eq!(expected_sha256(&text, "three.zip"), None);
        assert_eq!(expected_sha256("zzzz  one.zip", "one.zip"), None, "a short or non-hex hash is not a checksum");
    }

    // ---- finding the download ---------------------------------------------------

    fn release_with(names: &[&str]) -> Release {
        Release {
            version: crate::Version::new(0, 2, 0),
            tag: "v0.2.0".into(),
            page_url: "https://github.com/davidroberthoare/portmanteau/releases/tag/v0.2.0".into(),
            notes: String::new(),
            assets: names
                .iter()
                .map(|n| Asset { name: n.to_string(), url: format!("{DOWNLOAD_PREFIX}v0.2.0/{n}"), size: 1 })
                .collect(),
        }
    }

    #[test]
    fn a_plan_needs_the_archive_the_list_and_the_signature() {
        let all = release_with(&["a.zip", SUMS_NAME, SIG_NAME]);
        assert!(plan_for(&all, Some("a.zip")).is_ok());
        assert!(plan_for(&release_with(&["a.zip", SUMS_NAME]), Some("a.zip")).is_err(), "unsigned release");
        assert!(plan_for(&release_with(&[SUMS_NAME, SIG_NAME]), Some("a.zip")).is_err());
        assert!(plan_for(&all, None).is_err(), "no build for this system");
    }

    #[test]
    fn files_from_other_places_are_never_downloaded() {
        let mut release = release_with(&["a.zip", SUMS_NAME, SIG_NAME]);
        release.assets[0].url = "https://evil.example/a.zip".into();
        assert!(plan_for(&release, Some("a.zip")).is_err());
        let cancel = AtomicBool::new(false);
        assert!(matches!(
            download("http://github.com/davidroberthoare/portmanteau/releases/download/x", 10, &|_, _| {}, &cancel),
            Err(InstallError::Untrusted(_))
        ));
    }

    // ---- unpacking ----------------------------------------------------------------

    #[test]
    fn a_tar_gz_unpacks_to_its_one_folder() {
        let dir = scratch("tar");
        let archive = tar_gz(&[("pkg/portmanteau", b"exe"), ("pkg/README.md", b"hi")]);
        let package = unpack(&archive, "x.tar.gz", &dir).unwrap();
        assert_eq!(package.file_name().unwrap(), "pkg");
        assert_eq!(fs::read(package.join("portmanteau")).unwrap(), b"exe");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_zip_unpacks_and_keeps_programs_executable() {
        let dir = scratch("zip");
        let archive = zip_of(&[("pkg/portmanteau", b"exe"), ("pkg/sub/file.txt", b"x")]);
        let package = unpack(&archive, "x.zip", &dir).unwrap();
        assert_eq!(fs::read(package.join("sub/file.txt")).unwrap(), b"x");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(package.join("portmanteau")).unwrap().permissions().mode() & 0o111, 0o111);
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn archives_that_escape_their_folder_are_refused() {
        let dir = scratch("escape");
        let evil = zip_of(&[("../evil.txt", b"x"), ("pkg/a", b"y")]);
        assert!(matches!(unpack(&evil, "x.zip", &dir), Err(InstallError::Untrusted(_))));
        assert!(!dir.parent().unwrap().join("evil.txt").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn an_archive_with_two_top_folders_or_none_is_refused() {
        let dir = scratch("tops");
        assert!(unpack(&zip_of(&[("a/x", b"1"), ("b/y", b"2")]), "x.zip", &dir).is_err());
        let dir2 = scratch("tops-loose");
        assert!(unpack(&zip_of(&[("loose.txt", b"1")]), "x.zip", &dir2).is_err());
        assert!(unpack(b"junk", "x.rar", &scratch("rar")).is_err());
        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(dir2);
    }

    // ---- swapping -------------------------------------------------------------------

    #[test]
    fn layouts_are_recognised() {
        assert!(matches!(
            Layout::detect(Path::new("/Applications/Portmanteau.app/Contents/MacOS/portmanteau")),
            Layout::MacBundle { .. }
        ));
        let loose = Layout::detect(Path::new("/opt/portmanteau/portmanteau"));
        assert_eq!(loose.install_dir(), Path::new("/opt/portmanteau"));
        assert!(matches!(Layout::detect(Path::new("/x/Foo.app/portmanteau")), Layout::Loose { .. }));
    }

    #[test]
    fn a_loose_install_replaces_the_program_and_its_companions() {
        let root = scratch("loose");
        let (install, package) = (root.join("install"), root.join("package"));
        fs::create_dir_all(&install).unwrap();
        fs::create_dir_all(&package).unwrap();
        let exe = install.join(exe_name("portmanteau"));
        fs::write(&exe, "old app").unwrap();
        fs::write(install.join(exe_name("portmanteau-cli")), "old cli").unwrap();
        // portmanteau-mcp is not installed here, so it is not added.
        for (name, body) in [("portmanteau", "new app"), ("portmanteau-cli", "new cli"), ("portmanteau-mcp", "new mcp")] {
            fs::write(package.join(exe_name(name)), body).unwrap();
        }
        let layout = Layout::detect(&exe);
        let exe_for_stand_in = exe.clone();
        swap_in(&package, &layout, &move |new| fs::copy(new, &exe_for_stand_in).map(|_| ())).unwrap();
        assert_eq!(fs::read_to_string(&exe).unwrap(), "new app");
        assert_eq!(fs::read_to_string(install.join(exe_name("portmanteau-cli"))).unwrap(), "new cli");
        assert!(!install.join(exe_name("portmanteau-mcp")).exists());
        assert_eq!(fs::read_to_string(old_name(&install.join(exe_name("portmanteau-cli")))).unwrap(), "old cli");

        clean_up_old_files(&exe);
        assert!(!old_name(&install.join(exe_name("portmanteau-cli"))).exists(), "leftovers are cleaned up");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_failed_replacement_leaves_the_old_program() {
        let root = scratch("failed");
        let (install, package) = (root.join("install"), root.join("package"));
        fs::create_dir_all(&install).unwrap();
        fs::create_dir_all(&package).unwrap();
        let exe = install.join(exe_name("portmanteau"));
        fs::write(&exe, "old app").unwrap();
        fs::write(package.join(exe_name("portmanteau")), "new app").unwrap();
        let error = swap_in(&package, &Layout::detect(&exe), &|_| Err(std::io::Error::other("disk full"))).unwrap_err();
        assert!(matches!(error, InstallError::Io(_)));
        assert_eq!(fs::read_to_string(&exe).unwrap(), "old app");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_mac_bundle_is_swapped_and_the_old_one_kept_until_cleanup() {
        let root = scratch("mac");
        let bundle = root.join("Portmanteau.app");
        let exe = bundle.join("Contents/MacOS/portmanteau");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, "old").unwrap();
        let package = root.join("package");
        let new_exe = package.join("Portmanteau.app/Contents/MacOS/portmanteau");
        fs::create_dir_all(new_exe.parent().unwrap()).unwrap();
        fs::write(&new_exe, "new").unwrap();

        let layout = Layout::detect(&exe);
        assert_eq!(layout.install_dir(), root);
        swap_in(&package, &layout, &|_| unreachable!("a bundle never uses the running-file swap")).unwrap();
        assert_eq!(fs::read_to_string(&exe).unwrap(), "new");
        assert_eq!(fs::read_to_string(old_name(&bundle).join("Contents/MacOS/portmanteau")).unwrap(), "old");
        clean_up_old_files(&exe);
        assert!(!old_name(&bundle).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_folder_that_cannot_be_written_is_detected() {
        let root = scratch("ro");
        assert!(can_write_to(&root));
        assert!(!can_write_to(&root.join("does-not-exist")));
        let _ = fs::remove_dir_all(root);
    }
}
