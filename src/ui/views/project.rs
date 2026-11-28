//! Project: the details of the work itself, saved with the project.
//!
//! Title, author, contact and the draft's name and date belong to the
//! project, not to any one export. Compile decides whether a title page is
//! printed; the exporters read what is written here. Every edit goes through
//! the `project.update` command, like any other change.

use eframe::egui::{
    self, Align, CornerRadius, Layout, Frame, Margin, RichText, ScrollArea, Stroke, TextEdit, Vec2,
};
use serde_json::json;

use crate::app::PortmanteauApp;
use crate::theme::*;
use crate::ui::views::{content_frame, meta_line, page_head, quiet_button, section_label};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        "The work",
        "Project",
        "Details saved with this project and printed on its title page.",
        |ui| {
            meta_line(ui, &[&format!("{} words", app.ws.project.words())]);
        },
    );
    content_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ScrollArea::vertical()
            .id_salt("project_page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                details(app, ui);
            });
    });
}

fn card(ui: &mut egui::Ui, heading: &str, note: &str, body: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width().min(704.0);
    Frame::new()
        .fill(PANEL())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(width);
            section_label(ui, heading);
            ui.add_space(2.0);
            ui.label(RichText::new(note).font(ui_small()).color(MUTED()));
            ui.add_space(8.0);
            body(ui);
        });
    ui.add_space(12.0);
}

/// A field label, left-aligned in a fixed column so the fields line up.
fn label_cell(ui: &mut egui::Ui, text: &str) {
    ui.allocate_ui_with_layout(
        Vec2::new(96.0, 24.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            // The allocation only sets a maximum; the minimum holds the column.
            ui.set_min_size(Vec2::new(96.0, 24.0));
            ui.label(RichText::new(text).font(ui_small()).color(MUTED()));
        },
    );
}

/// One labelled text field. `rows` of 1 is a single line. Returns the new text
/// when the writer changed it.
fn field(ui: &mut egui::Ui, label: &str, value: &str, hint: &str, rows: usize) -> Option<String> {
    let mut text = value.to_string();
    ui.horizontal_top(|ui| {
        label_cell(ui, label);
        let edit = if rows <= 1 {
            TextEdit::singleline(&mut text)
        } else {
            TextEdit::multiline(&mut text).desired_rows(rows)
        };
        ui.add(
            edit.hint_text(hint)
                .font(ui_body())
                .desired_width(ui.available_width().max(160.0)),
        );
    });
    ui.add_space(4.0);
    (text != value).then_some(text)
}

fn details(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let project = &app.ws.project;
    let mut changes = serde_json::Map::new();
    let mut set = |key: &str, value: Option<String>| {
        if let Some(value) = value {
            changes.insert(key.to_string(), json!(value));
        }
    };

    card(
        ui,
        "The work",
        "How the project is named and credited.",
        |ui| {
            set("title", field(ui, "Title", &project.title, "Untitled", 1));
            set("subtitle", field(ui, "Subtitle", &project.subtitle, "Optional; books only, never on a script's cover", 1));
            set("author", field(ui, "Author", &project.author, "Your name", 1));
            set("genre", field(ui, "Genre", &project.genre, "Printed on a book's title page", 1));
            set("logline", field(ui, "Logline", &project.logline, "One or two sentences", 3));
        },
    );

    card(
        ui,
        "Contact",
        "Printed bottom-left on a script's title page. One line each: name, address, email, phone.",
        |ui| {
            set("contact", field(ui, "Contact", &project.contact, "Ada Marlowe\nada@example.com", 4));
        },
    );

    card(
        ui,
        "Draft",
        "Printed bottom-right on a script's title page.",
        |ui| {
            set("draft_label", field(ui, "Draft", &project.draft_label, "Draft, First draft, Shooting script…", 1));
            let today = chrono::Local::now().format("%d %B %Y").to_string();
            let new = field(ui, "Date", &project.draft_date, &format!("Blank prints today's date ({today})"), 1);
            set("draft_date", new);
            if !project.draft_date.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(100.0);
                    if quiet_button(ui, "Use today's date").clicked() {
                        set("draft_date", Some(String::new()));
                    }
                });
            }
        },
    );

    let mut cover_command: Option<String> = None;
    let mut open_statistics = false;
    card(
        ui,
        "Cover",
        "The picture an ebook opens with. A saved project keeps its own copy. Each output format can use a different one in Compile.",
        |ui| {
            ui.horizontal(|ui| {
                label_cell(ui, "Cover");
                let shown = if project.cover_image.is_empty() {
                    "None".to_owned()
                } else {
                    std::path::Path::new(&project.cover_image)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| project.cover_image.clone())
                };
                ui.label(RichText::new(shown).font(ui_body()));
                if quiet_button(ui, "Choose…").clicked() {
                    if let Some(path) = crate::platform::FileDialog::new()
                        .set_title("Cover picture")
                        .add_filter("Pictures", &["png", "jpg", "jpeg"])
                        .pick_file()
                    {
                        cover_command = Some(path.display().to_string());
                    }
                }
                if !project.cover_image.is_empty() && quiet_button(ui, "Remove").clicked() {
                    cover_command = Some(String::new());
                }
            });
        },
    );

    card(
        ui,
        "Writing goals",
        "Words each day, shown in the top bar. Weekly and time goals are in the statistics.",
        |ui| {
            let mut target = project.goals.daily_words;
            ui.horizontal(|ui| {
                label_cell(ui, "Daily words");
                ui.add(egui::DragValue::new(&mut target).range(0..=200_000).speed(10));
                if quiet_button(ui, "More goals and statistics…").clicked() {
                    open_statistics = true;
                }
            });
            if target != project.goals.daily_words {
                changes.insert("goals".into(), json!({"daily_words": target}));
            }
        },
    );

    if !changes.is_empty() {
        app.ws.execute("project.update", serde_json::Value::Object(changes));
    }
    if open_statistics {
        app.stats.tab = crate::ui::stats_window::StatsTab::Goals;
        app.stats.open = true;
    }
    if let Some(path) = cover_command {
        let response = app.ws.execute("project.set_cover", json!({ "path": path }));
        if !response.ok {
            app.toast("Could not set the cover", response.message, crate::app::ToastKind::Error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::{NodeT, Queryable};
    use egui_kittest::Harness;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn harness(app: PortmanteauApp) -> (Harness<'static>, Rc<RefCell<PortmanteauApp>>) {
        let app = Rc::new(RefCell::new(app));
        let ui_app = Rc::clone(&app);
        let mut themed = false;
        let harness = Harness::builder()
            .with_size(Vec2::new(1440.0, 930.0))
            .with_accessibility_check(false)
            .allow_missing_glyphs()
            .build_ui(move |ui| {
                if !themed {
                    crate::theme::install(ui.ctx());
                    themed = true;
                    return;
                }
                show(&mut ui_app.borrow_mut(), ui);
            });
        (harness, app)
    }

    #[test]
    fn typing_in_a_project_field_updates_the_project_through_the_command() {
        let (mut harness, app) = harness(PortmanteauApp::new(None));
        harness.run_steps(3);
        let author = app.borrow().ws.project.author.clone();
        // Pick the editable node among those showing the text.
        let field = harness
            .get_all_by_value(&author)
            .find(|node| node.accesskit_node().role() == egui::accesskit::Role::TextInput)
            .expect("the author field");
        field.click();
        harness.run_steps(2);
        harness
            .get_all_by_value(&author)
            .find(|node| node.accesskit_node().role() == egui::accesskit::Role::TextInput)
            .expect("the author field")
            .type_text(" Jr");
        harness.run_steps(3);
        assert_eq!(app.borrow().ws.project.author, format!("{author} Jr"));
        assert!(app.borrow().ws.dirty, "an edit marks the project unsaved");
    }

    #[test]
    fn the_blank_date_offers_no_reset_and_a_typed_date_does() {
        let (mut harness, app) = harness(PortmanteauApp::new(None));
        harness.run_steps(3);
        assert!(harness.query_by_label("Use today's date").is_none());
        app.borrow_mut().ws.project.draft_date = "14 March 2027".into();
        harness.run_steps(3);
        assert!(harness.query_by_label("Use today's date").is_some());
    }
}
