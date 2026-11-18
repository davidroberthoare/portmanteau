//! Command desk: the same JSON command bus that the MCP server and CLI speak,
//! exposed to a person for testing and for teaching an agent what to call.

use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke,
    TextEdit, Vec2,
};

use crate::app::{PortmanteauApp, ToastKind};
use crate::theme::*;
use crate::ui::views::{content_frame, meta_line, page_head, primary_button};

pub fn show(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    page_head(
        ui,
        "Automation",
        "Command desk",
        "Every action in this studio is a JSON command. Agents use the same commands through MCP.",
        |ui| {
            ui.label(
                RichText::new("API · MCP · CLI")
                    .font(ui_small())
                    .color(SAGE),
            );
        },
    );

    let catalogue = portmanteau_api::command_catalogue();
    content_frame().show(ui, |ui| {
        let height = (ui.available_height() - 4.0).max(400.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(300.0, height),
                Layout::top_down(Align::Min),
                |ui| {
                    Frame::new()
                        .fill(PANEL)
                        .stroke(Stroke::new(1.0, BORDER_SOFT))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(9))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                section_label(ui, "Commands");
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    meta_line(ui, &[&format!("{}", catalogue.len())]);
                                });
                            });
                            ui.add_space(6.0);
                            ScrollArea::vertical()
                                .id_salt("command_list")
                                .auto_shrink([false, false])
                                .max_height(height - 44.0)
                                .show(ui, |ui| {
                                    for (i, entry) in catalogue.iter().enumerate() {
                                        let name = entry["name"].as_str().unwrap_or("");
                                        let desc = entry["description"].as_str().unwrap_or("");
                                        let selected = app.cmd_selected == i;
                                        let (rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), 43.0),
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
                                        ui.painter().text(
                                            rect.left_top() + Vec2::new(8.0, 7.0),
                                            egui::Align2::LEFT_TOP,
                                            name,
                                            mono(9.5),
                                            if selected { ACCENT_INK } else { MUTED },
                                        );
                                        let short: String = desc.chars().take(40).collect();
                                        ui.painter().text(
                                            rect.left_top() + Vec2::new(8.0, 23.0),
                                            egui::Align2::LEFT_TOP,
                                            short,
                                            ui_small(),
                                            FAINT,
                                        );
                                        if resp.clicked() {
                                            app.cmd_selected = i;
                                            app.cmd_name = name.to_string();
                                            app.cmd_payload = entry["example"]
                                                .as_str()
                                                .unwrap_or("{}")
                                                .to_string();
                                        }
                                    }
                                });
                        });
                },
            );
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                // Runner.
                Frame::new()
                    .fill(BG)
                    .stroke(Stroke::new(1.0, BORDER_SOFT))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            section_label(ui, "Call");
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                meta_line(ui, &["portmanteau-mcp exposes these as agent tools"]);
                            });
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add(
                                TextEdit::singleline(&mut app.cmd_name)
                                    .font(mono(11.0))
                                    .desired_width(220.0),
                            );
                            ui.label(RichText::new("payload").font(ui_small()).color(FAINT));
                        });
                        ui.add_space(4.0);
                        ui.add(
                            TextEdit::multiline(&mut app.cmd_payload)
                                .font(mono(10.0))
                                .desired_rows(3)
                                .desired_width(f32::INFINITY),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if primary_button(ui, "Run command  ▸").clicked() {
                                run(app);
                            }
                            ui.add_space(6.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Fill example").font(ui_small()).color(MUTED),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                if let Some(entry) = catalogue.get(app.cmd_selected) {
                                    app.cmd_name =
                                        entry["name"].as_str().unwrap_or("stats").to_string();
                                    app.cmd_payload = entry["example"]
                                        .as_str()
                                        .unwrap_or("{}")
                                        .to_string();
                                }
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if app.ws.last_export.is_some() {
                                    pill(ui, "export ready", SAGE, SAGE_SOFT);
                                }
                            });
                        });
                    });
                ui.add_space(10.0);
                Frame::new()
                    .fill(PANEL_2)
                    .stroke(Stroke::new(1.0, BORDER_SOFT))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            section_label(ui, "Response");
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if !app.cmd_output.is_empty()
                                    && ui.small_button("Clear").clicked()
                                {
                                    app.cmd_output.clear();
                                }
                            });
                        });
                        ui.add_space(4.0);
                        let output = if app.cmd_output.is_empty() {
                            "Run a command to see its JSON response. Try “stats”, “document.list” or “export.preview”."
                                .to_string()
                        } else {
                            app.cmd_output.clone()
                        };
                        ScrollArea::vertical()
                            .id_salt("command_output")
                            .auto_shrink([false, false])
                            .max_height(height - 250.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(output)
                                            .font(mono(9.5))
                                            .color(if app.cmd_output.is_empty() { FAINT } else { TEXT }),
                                    )
                                    .wrap(),
                                );
                            });
                    });
            });
        });
    });
}

fn run(app: &mut PortmanteauApp) {
    let command = app.cmd_name.trim().to_string();
    let payload: serde_json::Value =
        serde_json::from_str(&app.cmd_payload).unwrap_or_else(|_| serde_json::json!({}));
    let res = app.ws.execute(&command, payload);
    let doc = serde_json::json!({
        "ok": res.ok,
        "command": res.command,
        "message": res.message,
        "data": res.data
    });
    app.cmd_output = serde_json::to_string_pretty(&doc).unwrap_or_default();
    if res.ok {
        app.toast("Command ran", res.message, ToastKind::Info);
    } else {
        app.toast("Command failed", res.message, ToastKind::Error);
    }
}
