//! The few things that differ between the desktop program and the browser.
//!
//! On the desktop these are the system's own file dialogs and clipboard. In the
//! browser the same names exist but do nothing (no local folders to browse, and
//! the clipboard is only reachable from a user gesture), so the rest of the app
//! compiles for both and the web version adds its own open/save-as-zip actions
//! instead (see `web.rs`).

#[cfg(not(target_arch = "wasm32"))]
pub use rfd::FileDialog;

/// A picture read from the clipboard: RGBA pixels.
pub struct ClipImage {
    pub width: usize,
    pub height: usize,
    pub bytes: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clipboard_text() -> Result<String, String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clipboard_image() -> Option<ClipImage> {
    let image = arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_image()).ok()?;
    Some(ClipImage { width: image.width, height: image.height, bytes: image.bytes.into_owned() })
}

#[cfg(target_arch = "wasm32")]
pub fn clipboard_text() -> Result<String, String> {
    Err("Pasting from the clipboard isn't available in the browser version yet. Paste into the note instead.".into())
}

#[cfg(target_arch = "wasm32")]
pub fn clipboard_image() -> Option<ClipImage> {
    None
}

/// Stand-in for `rfd::FileDialog` in the browser: every dialog is "cancelled".
#[cfg(target_arch = "wasm32")]
#[derive(Default)]
pub struct FileDialog;

#[cfg(target_arch = "wasm32")]
impl FileDialog {
    pub fn new() -> Self {
        Self
    }
    pub fn set_title(self, _: impl Into<String>) -> Self {
        self
    }
    pub fn set_file_name(self, _: impl Into<String>) -> Self {
        self
    }
    pub fn set_directory(self, _: impl AsRef<std::path::Path>) -> Self {
        self
    }
    pub fn add_filter(self, _: impl Into<String>, _: &[impl ToString]) -> Self {
        self
    }
    pub fn pick_file(self) -> Option<std::path::PathBuf> {
        None
    }
    pub fn pick_files(self) -> Option<Vec<std::path::PathBuf>> {
        None
    }
    pub fn pick_folder(self) -> Option<std::path::PathBuf> {
        None
    }
    pub fn save_file(self) -> Option<std::path::PathBuf> {
        None
    }
}
