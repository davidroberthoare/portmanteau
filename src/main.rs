//! Portmanteau — a native writing studio for novels, film, television and theatre.
//!
//! The binary is intentionally thin: it installs the visual language and hands
//! the window to [`app::PortmanteauApp`]. Everything below the window lives in
//! the `portmanteau-*` crates: a domain core, a command API, an MCP server and the
//! export engine.

// A Windows release build is a window app, so no black terminal opens behind it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod history;
mod icons;
mod platform;
mod settings;
mod spellcheck;
mod theme;
mod ui;
#[cfg(not(target_arch = "wasm32"))]
mod update;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
#[path = "update_web.rs"]
mod update;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
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

/// The window and dock icon, compiled in from `assets/icon/icon.png`.
#[cfg(not(target_arch = "wasm32"))]
fn app_icon() -> eframe::egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon/icon.png"))
        .expect("assets/icon/icon.png is a valid PNG")
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let screenshot = screenshot_path();
    // Open at the size the window had when it was last closed.
    let [width, height] = settings::load_settings()
        .window_size
        .map(|[w, h]| [w.clamp(1080.0, 7680.0), h.clamp(700.0, 4320.0)])
        .unwrap_or([1440.0, 930.0]);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([width, height])
            .with_min_inner_size([1080.0, 700.0])
            .with_title("Portmanteau")
            .with_icon(app_icon()),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Portmanteau",
        options,
        Box::new(move |cc| {
            let app = app::PortmanteauApp::new(screenshot);
            theme::set_ui_scale(app.settings.ui_scale);
            theme::install(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

/// The browser version: draws the same app into the `<canvas id="portmanteau">`
/// of `web/index.html`. Projects live in the page (see `web.rs`), not on disk.
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    wasm_bindgen_futures::spawn_local(async {
        // The project left open last time lives in this browser's storage.
        let saved = web::load_saved().await;
        let document = eframe::web_sys::window()
            .and_then(|window| window.document())
            .expect("a page to draw on");
        let canvas = document
            .get_element_by_id("portmanteau")
            .expect("web/index.html has a canvas with id \"portmanteau\"")
            .dyn_into::<eframe::web_sys::HtmlCanvasElement>()
            .expect("#portmanteau is a canvas");
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    web::set_context(cc.egui_ctx.clone());
                    let mut app = app::PortmanteauApp::new(None);
                    if let Some(bytes) = saved {
                        app.restore_from_browser(&bytes);
                    }
                    theme::set_ui_scale(app.settings.ui_scale);
                    theme::install(&cc.egui_ctx);
                    Ok(Box::new(app))
                }),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            loading.remove();
        }
        if let Err(error) = started {
            if let Some(failed) = document.get_element_by_id("failed") {
                failed.set_inner_html(&format!("Portmanteau could not start in this browser: {error:?}"));
            }
        }
    });
}
