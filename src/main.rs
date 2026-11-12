//! Portmanteau — a native writing studio for novels, film, television and theatre.
//!
//! The binary is intentionally thin: it installs the visual language and hands
//! the window to [`app::PortmanteauApp`]. Everything below the window lives in
//! the `portmanteau-*` crates: a domain core, a command API, an MCP server and the
//! export engine.

mod app;
mod icons;
mod settings;
mod spellcheck;
mod theme;
mod ui;

use std::path::PathBuf;

fn screenshot_path() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--screenshot" {
            if let Some(path) = args.get(i + 1) {
                return Some(PathBuf::from(path));
            }
        } else if let Some(path) = args[i].strip_prefix("--screenshot=") {
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
        i += 1;
    }
    None
}

fn main() -> eframe::Result<()> {
    let screenshot = screenshot_path();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 930.0])
            .with_min_inner_size([1080.0, 700.0])
            .with_title("Portmanteau"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Portmanteau",
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(app::PortmanteauApp::new(screenshot)))
        }),
    )
}
