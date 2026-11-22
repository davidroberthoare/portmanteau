//! The update notice and installer: a background check against GitHub (see
//! the `portmanteau-update` crate), the small icon in the top bar, the panel
//! that says what was found, and "Install and restart", which downloads the
//! signed build for this system, swaps it in and relaunches. When a release
//! can't be installed from inside the app, "Download" opens its page.
//! Plan: `docs/release-and-updates-plan.md`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Align2, Color32, Pos2, RichText, Sense, Stroke, Vec2};
use portmanteau_core::UpdatePrefs;
use portmanteau_update::install::{self, InstallError, Installed};
use portmanteau_update::{CheckError, Outcome, Release};

use crate::theme::*;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum UpdateState {
    /// Nothing has been checked in this run.
    #[default]
    Idle,
    Checking,
    UpToDate,
    NoReleasesYet,
    Available(Release),
    Failed(String),
}

/// Where an install stands.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum InstallState {
    #[default]
    Idle,
    /// Downloading, verifying or swapping; see [`InstallProgress`].
    Running,
    Failed(String),
    /// Installed. The app restarts itself unless saving the project failed.
    Installed { restart_blocked: bool },
}

/// Shared with the install thread: what it is doing and how far it has got.
#[derive(Default)]
pub struct InstallProgress {
    done: AtomicU64,
    total: AtomicU64,
    cancel: AtomicBool,
    stage: Mutex<String>,
}

#[derive(Default)]
pub struct Updater {
    pub state: UpdateState,
    pub install: InstallState,
    install_rx: Option<Receiver<Result<Installed, InstallError>>>,
    progress: Arc<InstallProgress>,
    /// Set when an install finished: the program to launch in place of this one.
    pub restart: Option<PathBuf>,
    /// The panel with the details is showing.
    pub panel_open: bool,
    /// The running check was asked for by the writer (Help menu), not by the
    /// daily timer: only then does the answer open the panel by itself.
    manual: bool,
    started: bool,
    rx: Option<Receiver<Result<Outcome, CheckError>>>,
}

/// Seconds since 1970.
fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

impl Updater {
    /// The newer release to announce with the toolbar icon, if there is one the
    /// writer has not chosen to skip.
    pub fn announced<'a>(&'a self, prefs: &UpdatePrefs) -> Option<&'a Release> {
        match &self.state {
            UpdateState::Available(release)
                if !portmanteau_update::is_skipped(release, prefs.skipped_version.as_deref()) =>
            {
                Some(release)
            }
            _ => None,
        }
    }

    /// Ask GitHub on a background thread. `manual` checks always answer in the panel.
    pub fn start(&mut self, ctx: &egui::Context, manual: bool) {
        if self.state == UpdateState::Checking {
            self.manual |= manual;
            return;
        }
        self.manual = manual;
        self.state = UpdateState::Checking;
        if cfg!(test) {
            return; // tests never touch the network
        }
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(portmanteau_update::check(portmanteau_update::CURRENT_VERSION));
            ctx.request_repaint();
        });
    }

    /// Download, verify and install `release` on a background thread.
    pub fn start_install(&mut self, ctx: &egui::Context, release: &Release) {
        if self.install == InstallState::Running {
            return;
        }
        self.install = InstallState::Running;
        self.progress = Arc::new(InstallProgress::default());
        if cfg!(test) {
            return; // tests never touch the network or the installed program
        }
        // Taken now: after the swap, Linux reports the running file as "(deleted)".
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(error) => {
                self.install = InstallState::Failed(format!("Could not find the running program: {error}"));
                return;
            }
        };
        let (tx, rx) = mpsc::channel();
        self.install_rx = Some(rx);
        let (release, progress, ctx) = (release.clone(), Arc::clone(&self.progress), ctx.clone());
        std::thread::spawn(move || {
            let stage = |text: &str| {
                if let Ok(mut stage) = progress.stage.lock() {
                    *stage = text.to_string();
                }
                ctx.request_repaint();
            };
            let report = |done: u64, total: Option<u64>| {
                progress.done.store(done, Ordering::Relaxed);
                progress.total.store(total.unwrap_or(0), Ordering::Relaxed);
                ctx.request_repaint();
            };
            let result = install::install_update(&release, &exe, &stage, &report, &progress.cancel);
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    fn poll_install(&mut self) {
        let Some(rx) = &self.install_rx else { return };
        let Ok(result) = rx.try_recv() else { return };
        self.install_rx = None;
        match result {
            Ok(installed) => {
                self.install = InstallState::Installed { restart_blocked: false };
                self.restart = Some(installed.restart_path);
            }
            Err(InstallError::Cancelled) => self.install = InstallState::Idle,
            Err(error) => self.install = InstallState::Failed(error.to_string()),
        }
    }

    /// Once per frame: start the daily check on the first frame, and pick up a
    /// finished check. Returns true when the settings changed and need saving.
    /// `may_check_by_itself` is false for screenshots and tests.
    pub fn tick(&mut self, ctx: &egui::Context, prefs: &mut UpdatePrefs, may_check_by_itself: bool) -> bool {
        if !self.started {
            self.started = true;
            // A development aid, like PORTMANTEAU_VIEW: pretend a release exists.
            if may_check_by_itself {
                // Leftovers from the last update (the old program, kept in case it failed).
                if let Ok(exe) = std::env::current_exe() {
                    install::clean_up_old_files(&exe);
                }
            }
            if let Ok(fake) = std::env::var("PORTMANTEAU_FAKE_UPDATE") {
                self.state = UpdateState::Available(fake_release(&fake));
                self.panel_open = true;
            } else if may_check_by_itself && prefs.auto_check && portmanteau_update::is_due(prefs.last_checked, now()) {
                self.start(ctx, false);
            }
        }
        self.poll_install();
        let Some(rx) = &self.rx else { return false };
        let Ok(result) = rx.try_recv() else { return false };
        self.rx = None;
        let mut changed = false;
        self.state = match result {
            Ok(outcome) => {
                prefs.last_checked = Some(now());
                changed = true;
                match outcome {
                    Outcome::UpToDate { .. } => UpdateState::UpToDate,
                    Outcome::NoReleasesYet => UpdateState::NoReleasesYet,
                    Outcome::Available(release) => {
                        if self.manual {
                            // Asking again overrides an earlier "skip".
                            prefs.skipped_version = None;
                        }
                        UpdateState::Available(release)
                    }
                }
            }
            Err(error) => UpdateState::Failed(error.to_string()),
        };
        // A manual check always answers. The daily one speaks up only through
        // the icon, and stays quiet when offline.
        if self.manual && self.state != UpdateState::Idle {
            self.panel_open = true;
        }
        self.manual = false;
        changed
    }

    /// The icon at the right of the top bar. Nothing is drawn when there is
    /// nothing to announce.
    pub fn badge(&mut self, ui: &mut egui::Ui, prefs: &UpdatePrefs) {
        let Some(release) = self.announced(prefs) else { return };
        let tip = format!("Portmanteau {} is available", release.version);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Update available"));
        let fill = if response.hovered() { ACCENT.gamma_multiply(0.85) } else { ACCENT };
        ui.painter().circle_filled(rect.center(), 11.0, fill);
        // An arrow pointing up out of a tray: "something new to fetch".
        let c = rect.center();
        let stroke = Stroke::new(1.8, Color32::WHITE);
        ui.painter().line_segment([Pos2::new(c.x, c.y + 4.0), Pos2::new(c.x, c.y - 4.5)], stroke);
        ui.painter().line_segment([Pos2::new(c.x - 3.5, c.y - 1.0), Pos2::new(c.x, c.y - 4.5)], stroke);
        ui.painter().line_segment([Pos2::new(c.x + 3.5, c.y - 1.0), Pos2::new(c.x, c.y - 4.5)], stroke);
        if response.on_hover_text(tip).clicked() {
            self.panel_open = true;
        }
    }

    /// The panel: what was found, and what to do about it.
    pub fn panel(&mut self, ctx: &egui::Context, prefs: &mut UpdatePrefs, project_unsaved: bool) -> bool {
        if !self.panel_open {
            return false;
        }
        let mut open = true;
        let mut changed = false;
        let mut skip = false;
        let mut later = false;
        let mut retry = false;
        let mut install_now = false;
        let mut cancel_install = false;
        egui::Window::new("Software update")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(420.0)
            .show(ctx, |ui| match &self.state {
                UpdateState::Idle | UpdateState::Checking => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Checking for updates…").font(ui_body()).color(TEXT));
                    });
                }
                UpdateState::UpToDate | UpdateState::NoReleasesYet => {
                    ui.label(RichText::new("You're up to date").font(serif(20.0)).color(TEXT));
                    ui.add_space(4.0);
                    let line = if self.state == UpdateState::NoReleasesYet {
                        format!("Portmanteau {} is the newest version. No releases have been published yet.", portmanteau_update::CURRENT_VERSION)
                    } else {
                        format!("Portmanteau {} is the newest version.", portmanteau_update::CURRENT_VERSION)
                    };
                    ui.label(RichText::new(line).font(ui_body()).color(MUTED));
                }
                UpdateState::Failed(why) => {
                    ui.label(RichText::new("Couldn't check for updates").font(serif(20.0)).color(TEXT));
                    ui.add_space(4.0);
                    ui.label(RichText::new(why).font(ui_body()).color(MUTED));
                    ui.add_space(8.0);
                    if ui.button("Try again").clicked() {
                        retry = true;
                    }
                }
                UpdateState::Available(release) => {
                    ui.label(
                        RichText::new(format!("Portmanteau {} is available", release.version))
                            .font(serif(20.0))
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new(format!("You have {}.", portmanteau_update::CURRENT_VERSION))
                            .font(ui_body())
                            .color(MUTED),
                    );
                    if !release.notes.trim().is_empty() {
                        ui.add_space(8.0);
                        hairline(ui);
                        ui.add_space(6.0);
                        egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                            for line in release_note_lines(&release.notes) {
                                ui.label(RichText::new(line).font(ui_body()).color(TEXT));
                            }
                        });
                    }
                    ui.add_space(10.0);
                    let installable = install::can_install(release);
                    match &self.install {
                        InstallState::Running => {
                            let stage = self.progress.stage.lock().map(|s| s.clone()).unwrap_or_default();
                            ui.label(RichText::new(stage).font(ui_body()).color(MUTED));
                            let (done, total) = (self.progress.done.load(Ordering::Relaxed), self.progress.total.load(Ordering::Relaxed));
                            if total > 0 {
                                ui.add(egui::ProgressBar::new((done as f32 / total as f32).clamp(0.0, 1.0)).desired_width(380.0));
                            } else {
                                ui.add(egui::ProgressBar::new(0.0).animate(true).desired_width(380.0));
                            }
                            ui.add_space(6.0);
                            if ui.button("Cancel").clicked() {
                                cancel_install = true;
                            }
                        }
                        InstallState::Installed { restart_blocked } => {
                            let text = if *restart_blocked {
                                "The update is installed, but your project could not be saved. Save it, then quit and start Portmanteau again."
                            } else {
                                "The update is installed. Restarting…"
                            };
                            ui.label(RichText::new(text).font(ui_body()).color(TEXT));
                        }
                        idle_or_failed => {
                            if let InstallState::Failed(why) = idle_or_failed {
                                ui.label(RichText::new(why).font(ui_body()).color(MUTED));
                                ui.add_space(6.0);
                            }
                            if installable && project_unsaved {
                                ui.label(
                                    RichText::new("Save your project first (File › Save project as…), then install.")
                                        .font(ui_small())
                                        .color(FAINT),
                                );
                                ui.add_space(4.0);
                            }
                            ui.horizontal(|ui| {
                                if installable {
                                    let label = if matches!(idle_or_failed, InstallState::Failed(_)) { "Try again" } else { "Install and restart" };
                                    if ui.add_enabled(!project_unsaved, egui::Button::new(label)).clicked() {
                                        install_now = true;
                                    }
                                    if ui.button("Release page").clicked() {
                                        crate::app::open_release_page(&release.page_url);
                                    }
                                } else if ui.button("Download").clicked() {
                                    crate::app::open_release_page(&release.page_url);
                                }
                                if ui.button("Skip this version").clicked() {
                                    skip = true;
                                }
                                if ui.button("Later").clicked() {
                                    later = true;
                                }
                            });
                        }
                    }
                }
            });
        if cancel_install {
            self.progress.cancel.store(true, Ordering::Relaxed);
        }
        if install_now {
            if let UpdateState::Available(release) = self.state.clone() {
                self.start_install(ctx, &release);
            }
        }
        if skip {
            if let UpdateState::Available(release) = &self.state {
                prefs.skipped_version = Some(release.version.to_string());
                changed = true;
            }
            open = false;
        }
        if later {
            open = false;
        }
        if retry {
            self.start(ctx, true);
        }
        self.panel_open = open;
        changed
    }
}

/// The notes as plain lines: Markdown marks are dropped, empty runs collapse.
pub fn release_note_lines(notes: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in notes.lines() {
        let text = raw.trim().trim_start_matches('#').trim().replace("**", "").replace('`', "");
        let text = match text.strip_prefix("- ").or_else(|| text.strip_prefix("* ")) {
            Some(rest) => format!("•  {rest}"),
            None => text,
        };
        if text.is_empty() && lines.last().is_none_or(|l| l.is_empty()) {
            continue;
        }
        lines.push(text);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

fn fake_release(version: &str) -> Release {
    Release {
        version: portmanteau_update::parse_version(version).unwrap_or_else(|| semver_zero()),
        tag: format!("v{version}"),
        page_url: "https://github.com/davidroberthoare/portmanteau/releases".into(),
        notes: "## What's new\n\n- A sample release note, shown because PORTMANTEAU_FAKE_UPDATE is set.\n- Checking fetched nothing; Install and restart will fail, as this release does not exist.".into(),
        // Assets named like a real release, so the panel offers "Install and
        // restart"; clicking it fails (there is no such release) and shows the error.
        assets: [install::platform_archive_name().unwrap_or("none.zip"), "SHA256SUMS", "SHA256SUMS.minisig"]
            .into_iter()
            .map(|name| portmanteau_update::Asset {
                name: name.into(),
                url: format!("{}v{version}/{name}", install::DOWNLOAD_PREFIX),
                size: 1,
            })
            .collect(),
    }
}

fn semver_zero() -> portmanteau_update::Version {
    portmanteau_update::Version::new(0, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(version: &str) -> Release {
        fake_release(version)
    }

    #[test]
    fn notes_lose_their_markdown_and_extra_blank_lines() {
        let lines = release_note_lines("## v0.2.0\n\n\n- **Faster** search\n* `Ctrl+K` links\n\nThanks!\n\n");
        assert_eq!(lines, vec!["v0.2.0", "", "•  Faster search", "•  Ctrl+K links", "", "Thanks!"]);
    }

    #[test]
    fn only_a_newer_unskipped_release_is_announced() {
        let mut prefs = UpdatePrefs::default();
        let mut updater = Updater::default();
        assert!(updater.announced(&prefs).is_none(), "nothing known yet");
        updater.state = UpdateState::Available(release("0.9.0"));
        assert!(updater.announced(&prefs).is_some());
        prefs.skipped_version = Some("0.9.0".into());
        assert!(updater.announced(&prefs).is_none(), "skipped");
        updater.state = UpdateState::Available(release("0.9.1"));
        assert!(updater.announced(&prefs).is_some(), "a newer one than the skipped one is announced");
        updater.state = UpdateState::UpToDate;
        assert!(updater.announced(&UpdatePrefs::default()).is_none());
    }
}
