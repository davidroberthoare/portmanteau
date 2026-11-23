//! The browser version has no updater: a new version arrives when the page is
//! reloaded. This stand-in keeps the same names as `update.rs` so the rest of
//! the app compiles unchanged, and does nothing.

use std::path::PathBuf;

use portmanteau_core::UpdatePrefs;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum UpdateState {
    #[default]
    Idle,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum InstallState {
    #[default]
    Idle,
    Installed { restart_blocked: bool },
}

#[derive(Default)]
#[allow(dead_code)]
pub struct Updater {
    pub state: UpdateState,
    pub install: InstallState,
    pub restart: Option<PathBuf>,
    pub panel_open: bool,
}

impl Updater {
    pub fn start(&mut self, _ctx: &egui::Context, _manual: bool) {}
    pub fn tick(&mut self, _ctx: &egui::Context, _prefs: &mut UpdatePrefs, _may_check: bool) -> bool {
        false
    }
    pub fn badge(&mut self, _ui: &mut egui::Ui, _prefs: &UpdatePrefs) {}
    pub fn panel(&mut self, _ctx: &egui::Context, _prefs: &mut UpdatePrefs, _unsaved: bool) -> bool {
        false
    }
}
