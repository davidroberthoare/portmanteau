//! Research: clippings, interviews, data and field notes beside the manuscript.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};
use portmanteau_core::StyleSheet;

use crate::app::{PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::editor::{inline_format_toolbar, script_flow_layout_job};
use crate::ui::views::{content_frame, meta_line, page_head, primary_button};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    egui_extras::install_image_loaders(ui.ctx());
    let mut add = false;
    page_head(
        ui,
        "Sources",
        "Research",
        "Keep the world of the story close: interviews, archives, images, arguments with yourself.",
        |ui| {
            if primary_button(ui, "+ New note").clicked() {
                add = true;
            }
        },
    );

    let items = app.ws.project.research.clone();
    if !items.is_empty()
        && !items
            .iter()
            .any(|i| Some(&i.id) == app.selected_research.as_ref())
    {
        app.selected_research = Some(items[0].id.clone());
    }

    content_frame().show(ui, |ui| {
        if items.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No research yet").font(serif(20.0)).color(TEXT));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("A folder of small, useful things. Start with one note.")
                        .font(sans(11.0))
                        .color(MUTED),
                );
                ui.add_space(12.0);
                if primary_button(ui, "Add the first note").clicked() {
                    add = true;
                }
            });
            return;
        }

        let height = (ui.available_height() - 8.0).max(320.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(286.0, height),
                Layout::top_down(Align::Min),
                |ui| {
                    Frame::new()
                        .fill(PANEL)
                        .stroke(Stroke::new(1.0, BORDER_SOFT))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                section_label(ui, "Sourcebook");
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    meta_line(ui, &[&format!("{} notes", items.len())]);
                                });
                            });
                            ui.add_space(6.0);
                            ScrollArea::vertical()
                                .id_salt("research_list")
                                .auto_shrink([false, false])
                                .max_height(height - 58.0)
                                .show(ui, |ui| {
                                    for item in &items {
                                        let selected =
                                            app.selected_research.as_deref() == Some(item.id.as_str());
                                        let (rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), 46.0),
                                            Sense::click(),
                                        );
                                        if selected {
                                            ui.painter().rect_filled(
                                                rect,
                                                CornerRadius::same(5),
                                                ACCENT_SOFT,
                                            );
                                        } else if resp.hovered() {
                                            ui.painter().rect_filled(
                                                rect,
                                                CornerRadius::same(5),
                                                PANEL_2,
                                            );
                                        }
                                        let kind_color = match item.kind.as_str() {
                                            "Interview" => PURPLE,
                                            "Data" => BLUE,
                                            "Archive" => ACCENT,
                                            _ => SAGE,
                                        };
                                        ui.painter().circle_filled(
                                            egui::pos2(rect.left() + 9.0, rect.top() + 15.0),
                                            3.0,
                                            kind_color,
                                        );
                                        let title_rect = egui::Rect::from_min_max(
                                            egui::pos2(rect.left() + 18.0, rect.top()),
                                            egui::pos2(rect.right() - 4.0, rect.bottom()),
                                        );
                                        ui.painter().with_clip_rect(title_rect).text(
                                            title_rect.left_top() + Vec2::new(0.0, 7.0),
                                            egui::Align2::LEFT_TOP,
                                            &item.title,
                                            sans(10.5),
                                            if selected { TEXT } else { MUTED },
                                        );
                                        ui.painter().with_clip_rect(title_rect).text(
                                            title_rect.left_top() + Vec2::new(0.0, 24.0),
                                            egui::Align2::LEFT_TOP,
                                            format!("{} · {}", item.kind, item.source),
                                            sans(8.5),
                                            FAINT,
                                        );
                                        if resp.clicked() {
                                            app.selected_research = Some(item.id.clone());
                                        }
                                    }
                                });
                        });
                },
            );
            ui.add_space(12.0);
            ui.vertical(|ui| {
                if let Some(id) = app.selected_research.clone() {
                    editor(app, ui, &id);
                }
            });
        });
    });

    if add {
        let res = app.ws.execute(
            "research.add",
            serde_json::json!({
                "title": "Untitled note",
                "body": "",
                "kind": "Note",
                "source": ""
            }),
        );
        if res.ok {
            if let Some(id) = res.data.get("id").and_then(|v| v.as_str()) {
                app.selected_research = Some(id.to_string());
            }
            app.ws.dirty = true;
        }
    }
}

#[derive(Debug)]
enum Action {
    Attach(String),
    Remove(String),
    PasteHtml,
    PasteImage,
}

fn editor(app: &mut PortmanteauApp, ui: &mut egui::Ui, id: &str) {
    let Some(pos) = app.ws.project.research.iter().position(|r| r.id == id) else {
        return;
    };
    let old_title = app.ws.project.research[pos].title.clone();
    let mut rich_body = app.ws.project.research[pos].body.clone();
    let old_body = rich_body.editor_text();
    let html = app.ws.project.research[pos].html.clone();
    let attachments = app.ws.project.research[pos].attachments.clone();
    let mut title = old_title.clone();
    let mut body = old_body.clone();
    let mut kind = app.ws.project.research[pos].kind.clone();
    let mut source = app.ws.project.research[pos].source.clone();
    let mut changed = false;
    let mut open_doc: Option<String> = None;
    let mut actions: Vec<Action> = Vec::new();
    let mut styles = app.ws.project.styles.clone();
    styles.body.size = 13.5;
    let sheet = StyleSheet::novel_prose(&styles);
    let assets_dir = app
        .ws
        .path
        .as_ref()
        .map(|root| root.join("assets").join("research").join(safe_dir_name(id)));

    Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_label(ui, "Research note");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    meta_line(ui, &[&format!("{} words", portmanteau_core::count_words(&body))]);
                });
            });
            ui.add_space(5.0);
            if ui
                .add(
                    TextEdit::singleline(&mut title)
                        .font(serif(22.0))
                        .frame(Frame::NONE)
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                changed = true;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Kind").font(sans(9.5)).color(FAINT));
                egui::ComboBox::from_id_salt("research_kind")
                    .selected_text(RichText::new(&kind).font(sans(10.0)).color(MUTED))
                    .width(118.0)
                    .show_ui(ui, |ui| {
                        for option in ["Note", "Interview", "Data", "Archive", "Image", "Place"] {
                            if ui
                                .selectable_value(&mut kind, option.to_string(), option)
                                .clicked()
                            {
                                changed = true;
                            }
                        }
                    });
                ui.add_space(8.0);
                ui.label(RichText::new("Source").font(sans(9.5)).color(FAINT));
                if ui
                    .add(
                        TextEdit::singleline(&mut source)
                            .hint_text("Where it came from")
                            .font(sans(10.0))
                            .desired_width(220.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
            ui.add_space(8.0);
            hairline(ui);
            ui.add_space(10.0);

            let rows = ((body.chars().count() as f32) / 75.0).ceil().max(16.0) as usize;
            let body_id = egui::Id::unique(("research_body", id));
            let ctx = ui.ctx().clone();
            if inline_format_toolbar(ui, &ctx, body_id, &mut rich_body) {
                changed = true;
            }
            let rich_body_layout = rich_body.clone();
            let styles_layout = styles.clone();
            let sheet_layout = sheet.clone();
            let mut layouter = |ui: &egui::Ui,
                                text: &dyn egui::TextBuffer,
                                wrap_width: f32| {
                let job = script_flow_layout_job(
                    text.as_str(),
                    &rich_body_layout,
                    &sheet_layout,
                    &styles_layout,
                    1.0,
                    wrap_width,
                    &[],
                );
                ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
            };
            if ui
                .add(
                    TextEdit::multiline(&mut body)
                        .id(body_id)
                        .layouter(&mut layouter)
                        .font(serif(13.5))
                        .desired_rows(rows.min(40))
                        .desired_width(f32::INFINITY)
                        .frame(Frame::NONE)
                        .hint_text("What did you find, and why does it matter to the story?"),
                )
                .changed()
            {
                rich_body.sync_editor_text(&body, &sheet);
                changed = true;
            }

            // ---- Attachments -------------------------------------------------
            ui.add_space(14.0);
            hairline(ui);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                section_label(ui, "Attachments");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    meta_line(
                        ui,
                        &[&format!(
                            "{} file{}",
                            attachments.len(),
                            if attachments.len() == 1 { "" } else { "s" }
                        )],
                    );
                });
            });
            ui.add_space(6.0);

            if assets_dir.is_none() {
                ui.label(
                    RichText::new("Save the project to attach files")
                        .font(sans(9.5))
                        .color(FAINT),
                );
                ui.add_space(4.0);
            }

            if attachments.is_empty() {
                ui.label(
                    RichText::new(
                        "No attachments. Images preview inline; audio, video and documents stay as local references.",
                    )
                    .font(sans(9.5))
                    .color(FAINT),
                );
            } else {
                let mut remove: Option<String> = None;
                for name in &attachments {
                    attachment_row(
                        ui,
                        id,
                        name,
                        assets_dir.as_deref(),
                        &mut remove,
                    );
                    ui.add_space(4.0);
                }
                if let Some(name) = remove {
                    actions.push(Action::Remove(name));
                }
            }

            ui.add_space(8.0);
            let draft_id = egui::Id::unique(("research_attachment_path", id));
            let mut attach_path = ui
                .ctx()
                .data(|d| d.get_temp::<String>(draft_id))
                .unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(RichText::new("File").font(sans(9.5)).color(FAINT));
                let response = ui.add(
                    TextEdit::singleline(&mut attach_path)
                        .hint_text("Path to a file on this computer")
                        .font(sans(10.0))
                        .desired_width(300.0),
                );
                if response.changed() {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(draft_id, attach_path.clone()));
                }
                if quiet_link(ui, "Attach file").clicked() {
                    actions.push(Action::Attach(attach_path.clone()));
                }
            });
            ui.horizontal(|ui| {
                if quiet_link(ui, "Paste HTML").clicked() {
                    actions.push(Action::PasteHtml);
                }
                if quiet_link(ui, "Paste image").clicked() {
                    actions.push(Action::PasteImage);
                }
                ui.label(
                    RichText::new("Copied into the project folder; nothing leaves this machine.")
                        .font(sans(8.5))
                        .color(FAINT),
                );
            });

            if !html.is_empty() {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    section_label(ui, "Pasted HTML");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        meta_line(ui, &[&format!("{} characters", html.chars().count())]);
                    });
                });
                ui.add_space(4.0);
                Frame::new()
                    .fill(PANEL_2)
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let preview: String = html.chars().take(600).collect();
                        let suffix = if html.chars().count() > 600 { "…" } else { "" };
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{preview}{suffix}"))
                                    .font(mono(9.0))
                                    .color(MUTED),
                            )
                            .wrap(),
                        );
                    });
            }

            ui.add_space(14.0);
            hairline(ui);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if quiet_link(ui, "Open as a document").clicked() {
                    if let Some(doc) = app
                        .ws
                        .project
                        .documents
                        .iter()
                        .find(|d| d.kind == portmanteau_core::DocKind::Research && d.title == old_title)
                    {
                        open_doc = Some(doc.id.clone());
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(doc) = app
                        .ws
                        .project
                        .documents
                        .iter()
                        .find(|d| d.kind == portmanteau_core::DocKind::Research && d.title == old_title)
                    {
                        meta_line(ui, &[&format!("Linked to {}", doc.title)]);
                    }
                });
            });
        });

    if changed {
        if let Some(item) = app.ws.project.research.get_mut(pos) {
            item.title = title.clone();
            rich_body.sync_editor_text(&body, &sheet);
            item.body = rich_body.clone();
            item.kind = kind;
            item.source = source;
        }
        if let Some(doc) = app
            .ws
            .project
            .documents
            .iter_mut()
            .find(|d| d.kind == portmanteau_core::DocKind::Research && d.title == old_title)
        {
            doc.title = title;
            doc.content = rich_body.clone();
            doc.touch();
        }
        app.ws.dirty = true;
    }

    for action in actions {
        match action {
            Action::Attach(path) => attach_file(app, id, &path),
            Action::Remove(name) => remove_attachment(app, id, &name),
            Action::PasteHtml => paste_html(app, id),
            Action::PasteImage => paste_image(app, id),
        }
    }

    if let Some(doc) = open_doc {
        app.open_doc(doc);
    }
}

/// Paint one attachment row. Images preview inline when the project has a
/// folder; everything else is a named local reference.
fn attachment_row(
    ui: &mut egui::Ui,
    id: &str,
    name: &str,
    assets_dir: Option<&std::path::Path>,
    remove: &mut Option<String>,
) {
    let kind = attachment_kind(name);
    let path = assets_dir.map(|dir| dir.join(name));
    let bytes = if kind == "Image" {
        path.as_ref().and_then(|p| std::fs::read(p).ok())
    } else {
        None
    };

    Frame::new()
        .fill(PANEL_2)
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if kind == "Image" {
                if let Some(bytes) = bytes {
                    let stamp = path
                        .as_ref()
                        .and_then(|p| std::fs::metadata(p).ok())
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_nanos())
                        .unwrap_or(0);
                    let uri = format!("bytes://research/{id}/{name}?v={stamp}");
                    ui.add(
                        egui::Image::from_bytes(uri, bytes)
                            .max_size(Vec2::new(220.0, 160.0))
                            .maintain_aspect_ratio(true)
                            .corner_radius(4),
                    );
                } else {
                    ui.label(
                        RichText::new("Image file is missing from assets/research.")
                            .font(sans(9.5))
                            .color(FAINT),
                    );
                }
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).font(sans(10.0)).color(TEXT));
                ui.label(RichText::new(kind).font(sans(9.0)).color(FAINT));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let clicked = ui
                        .add(
                            egui::Button::new(RichText::new("×").font(sans(12.0)).color(MUTED))
                                .frame(false),
                        )
                        .on_hover_text("Remove attachment")
                        .clicked();
                    if clicked {
                        *remove = Some(name.to_string());
                    }
                });
            });
            if kind != "Image" {
                ui.label(
                    RichText::new("Reference only — the file stays on disk.")
                        .font(sans(8.5))
                        .color(FAINT),
                );
            }
        });
}

fn attachment_kind(name: &str) -> &'static str {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "Image",
        "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" => "Audio",
        "mp4" | "mov" | "mkv" | "webm" | "avi" => "Video",
        "pdf" => "PDF",
        "doc" | "docx" | "rtf" | "txt" | "md" => "Document",
        "html" | "htm" => "Web page",
        _ => "File",
    }
}

/// Keep a research id safe as a directory name inside the project package.
fn safe_dir_name(id: &str) -> String {
    let out: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    if out.is_empty() {
        "item".into()
    } else {
        out
    }
}

/// Reject anything that could escape `assets/research/<item-id>/`.
fn safe_attachment(name: &str) -> Option<&str> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        None
    } else {
        Some(name)
    }
}

fn attach_file(app: &mut PortmanteauApp, id: &str, typed: &str) {
    let typed = typed.trim();
    if typed.is_empty() {
        app.toast(
            "No file chosen",
            "Type the path to a file, then press Attach file.",
            ToastKind::Error,
        );
        return;
    }
    let Some(project_root) = app.ws.path.clone() else {
        app.toast(
            "Save the project first",
            "Attachments are copied into the project folder.",
            ToastKind::Error,
        );
        return;
    };
    let source = std::path::PathBuf::from(typed);
    if !source.is_file() {
        app.toast(
            "Could not attach file",
            format!("{} is not a file", source.display()),
            ToastKind::Error,
        );
        return;
    }
    let Some(name) = source
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_string())
    else {
        app.toast(
            "Could not attach file",
            "The path has no file name",
            ToastKind::Error,
        );
        return;
    };
    if safe_attachment(&name).is_none() {
        app.toast(
            "Could not attach file",
            "The file name is not allowed",
            ToastKind::Error,
        );
        return;
    }
    let dir = project_root
        .join("assets")
        .join("research")
        .join(safe_dir_name(id));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        app.toast("Could not attach file", e.to_string(), ToastKind::Error);
        return;
    }
    let dest = dir.join(&name);
    if source != dest {
        if let Err(e) = std::fs::copy(&source, &dest) {
            app.toast("Could not attach file", e.to_string(), ToastKind::Error);
            return;
        }
    }

    let mut added = false;
    if let Some(item) = app.ws.project.research.iter_mut().find(|r| r.id == id) {
        if !item.attachments.iter().any(|a| a == &name) {
            item.attachments.push(name.clone());
            added = true;
        }
    }
    if added {
        app.ws.dirty = true;
        app.toast("File attached", name, ToastKind::Ok);
    } else {
        app.toast("Already attached", name, ToastKind::Info);
    }
}

fn remove_attachment(app: &mut PortmanteauApp, id: &str, name: &str) {
    if let Some(item) = app.ws.project.research.iter_mut().find(|r| r.id == id) {
        item.attachments.retain(|a| a != name);
        app.ws.dirty = true;
    }
    if let Some(root) = app.ws.path.clone() {
        if let Some(name) = safe_attachment(name) {
            let _ = std::fs::remove_file(
                root.join("assets")
                    .join("research")
                    .join(safe_dir_name(id))
                    .join(name),
            );
        }
    }
}

fn paste_html(app: &mut PortmanteauApp, id: &str) {
    let text = match read_clipboard_text() {
        Ok(text) => text,
        Err(e) => {
            app.toast("Could not paste HTML", e, ToastKind::Error);
            return;
        }
    };
    if text.trim().is_empty() {
        app.toast(
            "Clipboard has no text",
            "Copy a web page or HTML snippet first.",
            ToastKind::Info,
        );
        return;
    }

    if let Some(item) = app.ws.project.research.iter_mut().find(|r| r.id == id) {
        item.html = text.clone();
    }
    app.ws.dirty = true;

    let images = extract_data_url_images(&text);
    if images.is_empty() {
        app.toast("HTML pasted", "Stored with the note", ToastKind::Ok);
        return;
    }

    let Some(project_root) = app.ws.path.clone() else {
        app.toast(
            "HTML pasted",
            "Save the project to store its embedded images",
            ToastKind::Error,
        );
        return;
    };
    let dir = project_root
        .join("assets")
        .join("research")
        .join(safe_dir_name(id));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        app.toast("Could not store images", e.to_string(), ToastKind::Error);
        return;
    }

    let mut existing = app
        .ws
        .project
        .research
        .iter()
        .find(|r| r.id == id)
        .map(|r| r.attachments.clone())
        .unwrap_or_default();
    let mut stored = 0usize;
    for (mime, bytes) in images {
        let ext = extension_for_mime(&mime);
        let name = next_attachment_name(&dir, "pasted", ext, &existing);
        match std::fs::write(dir.join(&name), &bytes) {
            Ok(()) => {
                existing.push(name.clone());
                if let Some(item) = app.ws.project.research.iter_mut().find(|r| r.id == id) {
                    item.attachments.push(name);
                }
                stored += 1;
            }
            Err(e) => {
                app.toast("Could not store image", e.to_string(), ToastKind::Error);
                return;
            }
        }
    }
    app.ws.dirty = true;
    app.toast(
        "HTML pasted",
        format!("Stored with the note; {stored} embedded image(s) copied into the project"),
        ToastKind::Ok,
    );
}

fn paste_image(app: &mut PortmanteauApp, id: &str) {
    let image = match read_clipboard_image() {
        Ok(image) => image,
        Err(e) => {
            app.toast("No image on clipboard", e, ToastKind::Info);
            return;
        }
    };
    let Some(project_root) = app.ws.path.clone() else {
        app.toast(
            "Save the project first",
            "Pasted images are copied into the project folder.",
            ToastKind::Error,
        );
        return;
    };
    let dir = project_root
        .join("assets")
        .join("research")
        .join(safe_dir_name(id));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        app.toast("Could not store image", e.to_string(), ToastKind::Error);
        return;
    }

    let existing = app
        .ws
        .project
        .research
        .iter()
        .find(|r| r.id == id)
        .map(|r| r.attachments.clone())
        .unwrap_or_default();
    let name = next_attachment_name(&dir, "pasted-image", "png", &existing);

    let (width, height) = (image.width as u32, image.height as u32);
    let Some(buffer) = image::RgbaImage::from_raw(width, height, image.bytes.to_vec()) else {
        app.toast(
            "Could not store image",
            "The clipboard image did not have the expected size",
            ToastKind::Error,
        );
        return;
    };
    let path = dir.join(&name);
    if let Err(e) = image::DynamicImage::ImageRgba8(buffer).save(&path) {
        app.toast("Could not store image", e.to_string(), ToastKind::Error);
        return;
    }

    if let Some(item) = app.ws.project.research.iter_mut().find(|r| r.id == id) {
        item.attachments.push(name.clone());
    }
    app.ws.dirty = true;
    app.toast("Image pasted", name, ToastKind::Ok);
}

fn read_clipboard_text() -> Result<String, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Clipboard unavailable: {e}"))?;
    clipboard
        .get_text()
        .map_err(|e| format!("No text on the clipboard: {e}"))
}

fn read_clipboard_image() -> Result<arboard::ImageData<'static>, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Clipboard unavailable: {e}"))?;
    clipboard
        .get_image()
        .map_err(|e| format!("Clipboard has no image: {e}"))
}

fn next_attachment_name(
    dir: &std::path::Path,
    prefix: &str,
    ext: &str,
    existing: &[String],
) -> String {
    let mut n = 1usize;
    loop {
        let name = format!("{prefix}-{n}.{ext}");
        if !existing.iter().any(|a| a == &name) && !dir.join(&name).exists() {
            return name;
        }
        n += 1;
    }
}

fn extension_for_mime(mime: &str) -> &'static str {
    match mime.to_ascii_lowercase().as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/svg+xml" => "svg",
        _ => "bin",
    }
}

/// Find `data:image/...;base64,...` URLs in pasted HTML and decode them.
fn extract_data_url_images(html: &str) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = html[cursor..].find("data:image/") {
        let start = cursor + rel;
        let rest = &html[start + "data:".len()..];
        let Some(comma) = rest.find(',') else {
            break;
        };
        let header = &rest[..comma];
        let Some(mime) = header.strip_suffix(";base64") else {
            cursor = start + "data:".len() + comma + 1;
            continue;
        };
        let mime = mime.split(';').next().unwrap_or(mime).trim();
        let payload = &rest[comma + 1..];
        let end = payload
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='))
            .unwrap_or(payload.len());
        if end > 0 {
            if let Some(bytes) = decode_base64(&payload[..end]) {
                out.push((mime.to_string(), bytes));
            }
        }
        cursor = start + "data:".len() + comma + 1 + end;
    }
    out
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    fn value(byte: u8) -> Option<u32> {
        match byte {
            b'A'..=b'Z' => Some((byte - b'A') as u32),
            b'a'..=b'z' => Some((byte - b'a' + 26) as u32),
            b'0'..=b'9' => Some((byte - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in input.bytes() {
        if byte == b'=' {
            break;
        }
        let Some(v) = value(byte) else {
            return None;
        };
        buffer = (buffer << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1u32 << bits) - 1;
        }
    }
    Some(out)
}

fn quiet_link(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).font(sans(10.0)).color(BLUE))
            .frame(false)
            .min_size(Vec2::new(0.0, 22.0)),
    )
}

pub fn _unused(ui: &mut egui::Ui) {
    let _ = Frame::new().inner_margin(Margin::same(0)).show(ui, |_| {});
    let _ = Color32::WHITE;
}
