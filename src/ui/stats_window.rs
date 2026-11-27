//! Detailed writing statistics, goals and badges, in a window of their own,
//! and the little progress meter in the top bar that opens it.
//!
//! Everything is worked out from the project's activity log by
//! `portmanteau-core` (`activity`, `progress`, `analysis`); this file only
//! lays it out. Goals are changed through `project.update`.

use eframe::egui::{self, CornerRadius, Frame, Margin, RichText, Sense, Stroke, Vec2};
use portmanteau_core::activity::{self, ActivityTotals};
use portmanteau_core::analysis::{self, TextAnalysis};
use portmanteau_core::progress;
use serde_json::json;

use crate::app::{PortmanteauApp, View};
use crate::theme::*;
use crate::ui::views::quiet_button;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum StatsTab {
    #[default]
    Overview,
    Charts,
    Documents,
    Style,
    Goals,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Period {
    #[default]
    Days,
    Weeks,
    HourOfDay,
}

#[derive(Default)]
pub struct StatsState {
    pub open: bool,
    pub tab: StatsTab,
    period: Period,
    /// Show time instead of words in the charts.
    time_mode: bool,
    /// Analyse the open document instead of the whole manuscript.
    this_document: bool,
    /// The last analysis, with what it was made from (scope and word count).
    analysis: Option<(String, u32, TextAnalysis)>,
}

/// "1h 05m", "12m", "45s".
pub fn duration(secs: u32) -> String {
    match secs {
        0 => "0m".into(),
        1..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m", secs / 60),
        _ => format!("{}h {:02}m", secs / 3600, secs % 3600 / 60),
    }
}

fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

// ---------------------------------------------------------------------------
// The top-bar meter
// ---------------------------------------------------------------------------

/// The progress meter next to the search field. Clicking it opens the window.
pub fn top_bar_meter(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let goals = app.ws.project.goals.clone();
    let today = portmanteau_core::today();
    let p = progress::progress(&app.ws.project.activity, &goals, &today);
    let (text, share) = if !goals.show_meter {
        (format!("{} words today", p.today.added), None)
    } else if goals.daily_words > 0 {
        (format!("{} / {} today", p.today.added, goals.daily_words), p.daily_words)
    } else if goals.daily_minutes > 0 {
        (format!("{} / {} min today", p.today.typing_secs / 60, goals.daily_minutes), p.daily_minutes)
    } else if goals.weekly_words > 0 {
        (format!("{} / {} this week", p.week.added, goals.weekly_words), p.weekly_words)
    } else if goals.weekly_minutes > 0 {
        (format!("{} / {} min this week", p.week.typing_secs / 60, goals.weekly_minutes), p.weekly_minutes)
    } else {
        (format!("{} words today", p.today.added), None)
    };
    let response = ui
        .vertical(|ui| {
            ui.add_space(2.0);
            ui.label(RichText::new(text).font(ui_body()).color(MUTED));
            ui.add_space(2.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(88.0, 4.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::same(2), PANEL_3);
            if let Some(share) = share {
                let f = share.clamp(0.0, 1.0);
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(r.min, Vec2::new(r.width() * f, 4.0)),
                    CornerRadius::same(2),
                    if share >= 1.0 { ACCENT } else { SAGE },
                );
            }
        })
        .response
        .interact(Sense::click());
    let response = if goals.encouragement {
        response.on_hover_text(progress::encouragement(&p, &goals))
    } else {
        response.on_hover_text("Open writing statistics")
    };
    if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        app.stats.open = true;
    }
}

// ---------------------------------------------------------------------------
// The window
// ---------------------------------------------------------------------------

pub fn show(app: &mut PortmanteauApp, ctx: &egui::Context) {
    if !app.stats.open {
        return;
    }
    let mut open = true;
    egui::Window::new("Writing statistics")
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_size(Vec2::new(720.0, 600.0))
        .default_pos(ctx.content_rect().center() - Vec2::new(360.0, 320.0))
        .min_width(560.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (StatsTab::Overview, "Overview"),
                    (StatsTab::Charts, "Charts"),
                    (StatsTab::Documents, "Documents"),
                    (StatsTab::Style, "Writing style"),
                    (StatsTab::Goals, "Goals & badges"),
                ] {
                    ui.selectable_value(&mut app.stats.tab, tab, label);
                }
            });
            ui.add_space(8.0);
            hairline(ui);
            ui.add_space(8.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match app.stats.tab {
                StatsTab::Overview => overview(app, ui),
                StatsTab::Charts => charts(app, ui),
                StatsTab::Documents => documents(app, ui),
                StatsTab::Style => style(app, ui),
                StatsTab::Goals => goals(app, ui),
            });
        });
    app.stats.open = open;
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    ui.add_space(8.0);
}

fn note(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).font(ui_small()).color(FAINT));
}

// ---- Overview --------------------------------------------------------------

fn overview(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let project = &app.ws.project;
    let log = &project.activity;
    let today = portmanteau_core::today();
    let p = progress::progress(log, &project.goals, &today);
    let month_start = (chrono::Local::now().date_naive() - chrono::Duration::days(29)).format("%Y-%m-%d").to_string();
    let month = activity::total(log, |r| r.hour.get(..10).unwrap_or("") >= month_start.as_str());
    let all = activity::total(log, |_| true);

    if project.goals.encouragement {
        card(ui, |ui| {
            ui.label(RichText::new(progress::encouragement(&p, &project.goals)).font(serif(16.0)).color(TEXT));
            if p.streak > 0 || p.longest_streak > 1 {
                note(ui, &format!("Current run: {} day{} · Longest run: {} day{}", p.streak, plural(p.streak), p.longest_streak, plural(p.longest_streak)));
            }
        });
    }

    card(ui, |ui| {
        section_label(ui, "Time and words");
        ui.add_space(4.0);
        egui::Grid::new("stats_overview").num_columns(7).spacing([16.0, 6.0]).striped(true).show(ui, |ui| {
            for h in ["", "Typing", "Reading", "Added", "Edited", "Deleted", "Words / hour"] {
                ui.label(RichText::new(h).font(ui_small()).color(FAINT));
            }
            ui.end_row();
            for (label, t) in [("Today", p.today), ("This week", p.week), ("Last 30 days", month), ("All time", all)] {
                row(ui, label, &t);
            }
        });
        ui.add_space(6.0);
        note(ui, "Typing counts while you type, including pauses up to five seconds. Reading counts while the page is in front and you scroll or move about, for twenty seconds after each action. Added words are written at the end of a sentence or paragraph; edited words are typed into the middle of what is already there.");
    });

    let hours = activity::per_hour_of_day(log);
    if let Some((best, t)) = hours.iter().enumerate().max_by_key(|(_, t)| t.added).filter(|(_, t)| t.added > 0) {
        card(ui, |ui| {
            section_label(ui, "Habits");
            ui.add_space(4.0);
            ui.label(RichText::new(format!("Your most productive hour is {}:00 to {}:00, with {} words added there.", best, best + 1, thousands(t.added))).font(ui_body()).color(TEXT));
            let days = activity::per_day(log);
            let active = days.values().filter(|t| t.added > 0).count() as u32;
            if active > 0 {
                ui.label(RichText::new(format!("On the {} days you wrote, you averaged {} words added and {} of typing.", active, thousands(all.added / active), duration(all.typing_secs / active))).font(ui_body()).color(TEXT));
            }
            if let Some((day, t)) = days.iter().max_by_key(|(_, t)| t.added).filter(|(_, t)| t.added > 0) {
                ui.label(RichText::new(format!("Your best day was {} with {} words added.", day, thousands(t.added))).font(ui_body()).color(TEXT));
            }
        });
    }
    let total_words = project.words();
    if total_words > 0 {
        note(ui, &format!("The manuscript is {} words long, about {} to read.", thousands(total_words), duration(portmanteau_core::reading_minutes(total_words) * 60)));
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn row(ui: &mut egui::Ui, label: &str, t: &ActivityTotals) {
    ui.label(RichText::new(label).font(ui_body()).color(TEXT));
    for text in [
        duration(t.typing_secs),
        duration(t.reading_secs),
        thousands(t.added),
        thousands(t.edited),
        thousands(t.deleted),
        t.words_per_typing_hour().map_or("–".into(), thousands),
    ] {
        ui.label(RichText::new(text).font(ui_body()).color(MUTED));
    }
    ui.end_row();
}

// ---- Charts ----------------------------------------------------------------

struct Bar {
    label: String,
    first: f32,
    second: f32,
}

fn charts(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        for (p, label) in [(Period::Days, "Days"), (Period::Weeks, "Weeks"), (Period::HourOfDay, "Hour of day")] {
            ui.selectable_value(&mut app.stats.period, p, label);
        }
        ui.add_space(16.0);
        ui.selectable_value(&mut app.stats.time_mode, false, "Words");
        ui.selectable_value(&mut app.stats.time_mode, true, "Time");
    });
    ui.add_space(8.0);
    let log = &app.ws.project.activity;
    let time = app.stats.time_mode;
    let pick = |t: &ActivityTotals| -> (f32, f32) {
        if time {
            (t.typing_secs as f32 / 60.0, t.reading_secs as f32 / 60.0)
        } else {
            (t.added as f32, t.edited as f32)
        }
    };
    let bars: Vec<Bar> = match app.stats.period {
        Period::Days => {
            let days = activity::per_day(log);
            (0..28)
                .rev()
                .map(|back| {
                    let d = chrono::Local::now().date_naive() - chrono::Duration::days(back);
                    let key = d.format("%Y-%m-%d").to_string();
                    let (a, b) = pick(&days.get(&key).copied().unwrap_or_default());
                    Bar { label: d.format("%d").to_string(), first: a, second: b }
                })
                .collect()
        }
        Period::Weeks => {
            let weeks = activity::per_week(log);
            weeks
                .iter()
                .rev()
                .take(16)
                .rev()
                .map(|(w, t)| {
                    let (a, b) = pick(t);
                    Bar { label: w.split("-W").nth(1).unwrap_or("").to_owned(), first: a, second: b }
                })
                .collect()
        }
        Period::HourOfDay => activity::per_hour_of_day(log)
            .iter()
            .enumerate()
            .map(|(h, t)| {
                let (a, b) = pick(t);
                Bar { label: format!("{h}"), first: a, second: b }
            })
            .collect(),
    };
    let (first_name, second_name, unit) = if time { ("Typing", "Reading", "minutes") } else { ("Added", "Edited", "words") };
    card(ui, |ui| {
        ui.horizontal(|ui| {
            section_label(
                ui,
                match app.stats.period {
                    Period::Days => "The last 28 days",
                    Period::Weeks => "Recent weeks (week number)",
                    Period::HourOfDay => "By hour of the day, all time",
                },
            );
            ui.label(RichText::new(format!("■ {first_name}")).font(ui_small()).color(ACCENT));
            ui.label(RichText::new(format!("■ {second_name}")).font(ui_small()).color(ACCENT.gamma_multiply(0.45)));
            note(ui, &format!("({unit})"));
        });
        ui.add_space(6.0);
        bar_chart(ui, &bars, 150.0);
    });
    if app.ws.project.activity.is_empty() {
        note(ui, "Nothing is recorded yet. Time and words appear here as you write.");
    }
}

fn bar_chart(ui: &mut egui::Ui, bars: &[Bar], height: f32) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let max = bars.iter().map(|b| b.first + b.second).fold(0.0f32, f32::max).max(1.0);
    let count = bars.len().max(1);
    let slot = rect.width() / count as f32;
    let plot_h = height - 36.0;
    let hover = response.hover_pos();
    for (i, bar) in bars.iter().enumerate() {
        let x = rect.left() + slot * i as f32 + slot * 0.15;
        let width = slot * 0.7;
        let base = rect.bottom() - 18.0;
        let h1 = plot_h * bar.first / max;
        let h2 = plot_h * bar.second / max;
        let first = egui::Rect::from_min_max(egui::pos2(x, base - h1), egui::pos2(x + width, base));
        let second = egui::Rect::from_min_max(egui::pos2(x, base - h1 - h2), egui::pos2(x + width, base - h1));
        if bar.first + bar.second <= 0.0 {
            ui.painter().rect_filled(
                egui::Rect::from_min_max(egui::pos2(x, base - 2.0), egui::pos2(x + width, base)),
                CornerRadius::same(1),
                PANEL_3,
            );
        }
        ui.painter().rect_filled(first, CornerRadius::same(2), ACCENT);
        ui.painter().rect_filled(second, CornerRadius::same(2), ACCENT.gamma_multiply(0.45));
        if count <= 31 && (count <= 16 || i % 2 == 0 || i + 1 == count) {
            ui.painter().text(
                egui::pos2(x + width * 0.5, rect.bottom() - 3.0),
                egui::Align2::CENTER_BOTTOM,
                &bar.label,
                ui_small(),
                FAINT,
            );
        }
        let whole = egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2(x + width, rect.bottom()));
        if hover.is_some_and(|p| whole.contains(p)) {
            let total = bar.first + bar.second;
            ui.painter().text(
                egui::pos2(x + width * 0.5, (base - h1 - h2 - 3.0).max(rect.top() + 10.0)),
                egui::Align2::CENTER_BOTTOM,
                format!("{}", total.round() as i64),
                ui_small(),
                TEXT,
            );
        }
    }
}

// ---- Documents ---------------------------------------------------------------

fn documents(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let project = &app.ws.project;
    let by_doc = activity::per_document(&project.activity);
    let mut rows: Vec<(String, String, u32, ActivityTotals)> = project
        .documents
        .iter()
        .filter(|d| !d.kind.is_folder() && d.research.is_none())
        .map(|d| (d.id.clone(), d.title.clone(), d.words(), by_doc.get(&d.id).copied().unwrap_or_default()))
        .collect();
    rows.sort_by(|a, b| (b.3.typing_secs + b.3.reading_secs).cmp(&(a.3.typing_secs + a.3.reading_secs)).then(a.1.cmp(&b.1)));
    let mut open: Option<String> = None;
    card(ui, |ui| {
        section_label(ui, "Time and words by document");
        ui.add_space(4.0);
        egui::Grid::new("stats_documents").num_columns(7).spacing([14.0, 6.0]).striped(true).show(ui, |ui| {
            for h in ["Document", "Words", "Typing", "Reading", "Added", "Edited", "Deleted"] {
                ui.label(RichText::new(h).font(ui_small()).color(FAINT));
            }
            ui.end_row();
            for (id, title, words, t) in &rows {
                if ui.add(egui::Label::new(RichText::new(title).font(ui_body()).color(ACCENT)).sense(Sense::click())).on_hover_text("Open this document").clicked() {
                    open = Some(id.clone());
                }
                for text in [thousands(*words), duration(t.typing_secs), duration(t.reading_secs), thousands(t.added), thousands(t.edited), thousands(t.deleted)] {
                    ui.label(RichText::new(text).font(ui_body()).color(MUTED));
                }
                ui.end_row();
            }
        });
    });
    note(ui, "Times and counts begin from when this feature was added; older writing shows as words only.");
    if let Some(id) = open {
        app.selected = Some(id);
        app.view = View::Write;
    }
}

// ---- Writing style -----------------------------------------------------------

fn style(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.stats.this_document, false, "Whole manuscript");
        let title = app.selected.as_deref().and_then(|id| app.ws.project.document(id)).map(|d| d.title.clone());
        if let Some(title) = title {
            ui.selectable_value(&mut app.stats.this_document, true, format!("This document: {title}"));
        } else {
            app.stats.this_document = false;
        }
    });
    ui.add_space(8.0);
    let (scope, paragraphs, words): (String, Vec<String>, u32) = if app.stats.this_document {
        let id = app.selected.clone().unwrap_or_default();
        match app.ws.project.document(&id) {
            Some(doc) => (id, doc.content.paragraphs.iter().map(|p| p.plain_text()).collect(), doc.words()),
            None => (String::new(), Vec::new(), 0),
        }
    } else {
        let docs = app.ws.project.manuscript_documents();
        let prose: Vec<&portmanteau_core::Document> = docs.into_iter().filter(|d| !d.medium.is_script()).collect();
        let words = prose.iter().map(|d| d.words()).sum();
        ("manuscript".into(), prose.iter().flat_map(|d| d.content.paragraphs.iter().map(|p| p.plain_text())).collect(), words)
    };
    let stale = app.stats.analysis.as_ref().is_none_or(|(s, w, _)| *s != scope || *w != words);
    let refresh = quiet_button(ui, "Refresh").clicked();
    if stale || refresh {
        app.stats.analysis = Some((scope, words, analysis::analyze(&paragraphs)));
    }
    let Some((_, _, a)) = app.stats.analysis.clone() else { return };
    if a.words == 0 {
        note(ui, "No prose to measure yet. Scripts are not analysed.");
        return;
    }
    card(ui, |ui| {
        section_label(ui, "How it reads");
        ui.add_space(4.0);
        egui::Grid::new("stats_style").num_columns(2).spacing([24.0, 5.0]).show(ui, |ui| {
            let mut line = |name: &str, value: String, hint: &str| {
                ui.label(RichText::new(name).font(ui_body()).color(MUTED));
                ui.label(RichText::new(value).font(ui_body()).color(TEXT)).on_hover_text(hint);
                ui.end_row();
            };
            line("Words", thousands(a.words), "Words in this text");
            line("Sentences", thousands(a.sentences), "Counted at . ! and ?");
            line("Average sentence", format!("{:.1} words", a.avg_sentence_words), "Short sentences move quickly; long ones slow the reader down");
            line("Longest sentence", format!("{} words", a.longest_sentence_words), "");
            line("Sentences of 30+ words", thousands(a.long_sentences), "Worth a second look, though fine when deliberate");
            line("Average paragraph", format!("{:.0} words", a.avg_paragraph_words), "");
            line("Reading ease", format!("{:.0}  ({})", a.reading_ease, ease_name(a.reading_ease)), "Flesch reading ease: about 90 is very easy, 60 is plain English, 30 is hard going");
            line("Reading level", format!("grade {:.1}", a.grade_level), "The US school grade needed to follow it (Flesch-Kincaid)");
            line("Dialogue", format!("{:.0}% of the words", a.dialogue_share * 100.0), "Words between quotation marks");
            line("Word variety", format!("{:.0}% distinct", a.variety * 100.0), "Distinct words as a share of all words; it falls as a text gets longer, so compare like with like");
            line("Words ending in -ly", thousands(a.ly_words), "Often adverbs; they are not wrong, but they can pile up");
            line("Reading time", duration(a.reading_minutes * 60), "At about 238 words a minute");
        });
    });
    card(ui, |ui| {
        section_label(ui, "Sentence lengths");
        ui.add_space(4.0);
        let bars: Vec<Bar> = ["1–10", "11–20", "21–30", "31+"]
            .iter()
            .zip(a.sentence_lengths)
            .map(|(label, n)| Bar { label: (*label).into(), first: n as f32, second: 0.0 })
            .collect();
        bar_chart(ui, &bars, 90.0);
    });
    if !a.top_words.is_empty() {
        card(ui, |ui| {
            section_label(ui, "Most-used words");
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (word, n) in &a.top_words {
                    ui.label(RichText::new(format!("{word} ({n})")).font(ui_body()).color(TEXT));
                    ui.add_space(8.0);
                }
            });
            note(ui, "Common words like the and was are left out.");
        });
    }
    if !a.echoes.is_empty() {
        card(ui, |ui| {
            section_label(ui, "Words that echo");
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (word, n) in &a.echoes {
                    ui.label(RichText::new(format!("{word} (×{n})")).font(ui_body()).color(TEXT));
                    ui.add_space(8.0);
                }
            });
            note(ui, "A word used three or more times within sixty words. Use Find to see where.");
        });
    }
}

fn ease_name(score: f32) -> &'static str {
    match score {
        s if s >= 90.0 => "very easy",
        s if s >= 70.0 => "easy",
        s if s >= 60.0 => "plain",
        s if s >= 50.0 => "fairly hard",
        s if s >= 30.0 => "hard",
        _ => "very hard",
    }
}

// ---- Goals & badges ---------------------------------------------------------

fn goals(app: &mut PortmanteauApp, ui: &mut egui::Ui) {
    let g = app.ws.project.goals.clone();
    let today = portmanteau_core::today();
    let p = progress::progress(&app.ws.project.activity, &g, &today);
    let mut next = g.clone();

    card(ui, |ui| {
        section_label(ui, "Your goals");
        note(ui, "Set what suits you; leave a goal at 0 to skip it. Goals are kind guides, not rules.");
        ui.add_space(6.0);
        egui::Grid::new("stats_goals").num_columns(3).spacing([16.0, 8.0]).show(ui, |ui| {
            let goal_row = |ui: &mut egui::Ui, label: &str, value: &mut u32, max: u32, unit: &str, done: Option<f32>| {
                ui.label(RichText::new(label).font(ui_body()).color(MUTED));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(value).range(0..=max).speed(10.0).suffix(format!(" {unit}")));
                });
                match done {
                    Some(share) => {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(160.0, 8.0), Sense::hover());
                        ui.painter().rect_filled(r, CornerRadius::same(4), PANEL_3);
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(r.min, Vec2::new(r.width() * share.clamp(0.0, 1.0), 8.0)),
                            CornerRadius::same(4),
                            if share >= 1.0 { ACCENT } else { SAGE },
                        );
                    }
                    None => {
                        ui.label("");
                    }
                }
                ui.end_row();
            };
            goal_row(ui, "Words each day", &mut next.daily_words, 200_000, "words", p.daily_words);
            goal_row(ui, "Words each week", &mut next.weekly_words, 1_000_000, "words", p.weekly_words);
            goal_row(ui, "Typing each day", &mut next.daily_minutes, 1_440, "min", p.daily_minutes);
            goal_row(ui, "Typing each week", &mut next.weekly_minutes, 10_080, "min", p.weekly_minutes);
        });
        ui.add_space(6.0);
        ui.checkbox(&mut next.show_meter, "Show a progress meter in the top bar");
        ui.checkbox(&mut next.encouragement, "Show encouraging messages and badges");
        note(ui, "A week runs Monday to Sunday. Words count when they are added at the end of what you have written.");
    });

    if next != g {
        app.ws.execute("project.update", json!({"goals": next}));
    }

    if g.encouragement {
        let badges = progress::badges(&app.ws.project.activity, &g, &today);
        let earned = badges.iter().filter(|b| b.earned).count();
        card(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, "Badges");
                note(ui, &format!("{earned} of {} earned", badges.len()));
            });
            ui.add_space(6.0);
            let columns = 3;
            egui::Grid::new("stats_badges").num_columns(columns).spacing([8.0, 8.0]).show(ui, |ui| {
                for (i, badge) in badges.iter().enumerate() {
                    Frame::new()
                        .fill(if badge.earned { ACCENT.gamma_multiply(0.14) } else { PANEL_2 })
                        .stroke(Stroke::new(1.0, if badge.earned { ACCENT.gamma_multiply(0.6) } else { BORDER_SOFT }))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.set_width(190.0);
                            ui.set_min_height(46.0);
                            ui.vertical(|ui| {
                                let name = if badge.earned { format!("★ {}", badge.name) } else { badge.name.to_owned() };
                                ui.label(RichText::new(name).font(ui_body()).color(if badge.earned { TEXT } else { FAINT }));
                                ui.label(RichText::new(badge.description).font(ui_small()).color(FAINT));
                            });
                        });
                    if (i + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
        });
    }
}
