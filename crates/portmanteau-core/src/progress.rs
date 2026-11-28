//! Goals, progress, badges and gentle encouragement, all worked out from the
//! activity log. Nothing here is stored: badges are earned by the history, so
//! they cannot get out of step with it, and a writer who switches the
//! encouragement off loses nothing.

use serde::Serialize;

use crate::activity::{per_day, per_hour_of_day, streak, total, ActivityRecord, ActivityTotals};
use crate::model::WritingGoals;

/// Where the writer stands today and this week.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Progress {
    pub today: ActivityTotals,
    pub week: ActivityTotals,
    /// 0 to 1 or more; `None` when there is no such goal.
    pub daily_words: Option<f32>,
    pub weekly_words: Option<f32>,
    pub daily_minutes: Option<f32>,
    pub weekly_minutes: Option<f32>,
    /// Days in a row meeting the daily goal (any words, if there is no goal).
    pub streak: u32,
    pub longest_streak: u32,
}

impl Progress {
    /// The fullest goal's share, for a single meter: the first goal that is set.
    pub fn headline(&self) -> Option<f32> {
        self.daily_words.or(self.daily_minutes).or(self.weekly_words).or(self.weekly_minutes)
    }
}

fn share(done: u32, goal: u32) -> Option<f32> {
    (goal > 0).then(|| done as f32 / goal as f32)
}

/// The Monday and Sunday (as `YYYY-MM-DD`) of the week holding `today`.
pub fn week_bounds(today: &str) -> Option<(String, String)> {
    use chrono::Datelike;
    let day = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").ok()?;
    let monday = day - chrono::Duration::days(day.weekday().num_days_from_monday() as i64);
    let sunday = monday + chrono::Duration::days(6);
    Some((monday.format("%Y-%m-%d").to_string(), sunday.format("%Y-%m-%d").to_string()))
}

/// The longest run of days on which `met` held.
fn longest_run(days: &std::collections::BTreeMap<String, ActivityTotals>, met: impl Fn(&ActivityTotals) -> bool) -> u32 {
    let mut best = 0;
    let mut run = 0;
    let mut previous: Option<chrono::NaiveDate> = None;
    for (day, totals) in days {
        let Ok(date) = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d") else { continue };
        if !met(totals) {
            run = 0;
            previous = Some(date);
            continue;
        }
        run = match previous {
            Some(p) if date - p == chrono::Duration::days(1) && run > 0 => run + 1,
            _ => 1,
        };
        best = best.max(run);
        previous = Some(date);
    }
    best
}

pub fn progress(log: &[ActivityRecord], goals: &WritingGoals, today: &str) -> Progress {
    let day_totals = total(log, |r| r.hour.starts_with(today));
    let (monday, sunday) = week_bounds(today).unwrap_or_default();
    let week = total(log, |r| {
        let d = r.hour.get(..10).unwrap_or("");
        d >= monday.as_str() && d <= sunday.as_str()
    });
    let days = per_day(log);
    let bar = goals.daily_words.max(1);
    Progress {
        daily_words: share(day_totals.added, goals.daily_words),
        weekly_words: share(week.added, goals.weekly_words),
        daily_minutes: share(day_totals.typing_secs / 60, goals.daily_minutes),
        weekly_minutes: share(week.typing_secs / 60, goals.weekly_minutes),
        streak: streak(&days, today, bar),
        longest_streak: longest_run(&days, |t| t.added >= bar),
        today: day_totals,
        week,
    }
}

/// One thing the writer can earn.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Badge {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub earned: bool,
}

pub fn badges(log: &[ActivityRecord], goals: &WritingGoals, today: &str) -> Vec<Badge> {
    let all = total(log, |_| true);
    let days = per_day(log);
    let best_day = days.values().map(|t| t.added).max().unwrap_or(0);
    let longest = progress(log, goals, today).longest_streak;
    let hours = per_hour_of_day(log);
    let early: u32 = hours[..8].iter().map(|t| t.added).sum();
    let late: u32 = hours[22..].iter().map(|t| t.added).sum();
    let met_goal = goals.daily_words > 0 && days.values().any(|t| t.added >= goals.daily_words);
    let b = |id, name, description, earned| Badge { id, name, description, earned };
    vec![
        b("first-words", "First words", "Write your first 100 words here.", all.added >= 100),
        b("goal-met", "On target", "Reach your daily word goal.", met_goal),
        b("big-day", "A big day", "Write 1,000 words in a day.", best_day >= 1000),
        b("three-days", "Three in a row", "Write on three days in a row.", longest >= 3),
        b("week", "A whole week", "Write on seven days in a row.", longest >= 7),
        b("month", "A habit", "Write on thirty days in a row.", longest >= 30),
        b("ten-hours", "Ten hours", "Spend ten hours writing.", all.typing_secs >= 36_000),
        b("fifty-hours", "Fifty hours", "Spend fifty hours writing.", all.typing_secs >= 180_000),
        b("ten-k", "Ten thousand", "Add 10,000 words.", all.added >= 10_000),
        b("fifty-k", "Fifty thousand", "Add 50,000 words.", all.added >= 50_000),
        b("novel", "Novel length", "Add 80,000 words.", all.added >= 80_000),
        b("editor", "An editor's eye", "Rework 5,000 words of existing text.", all.edited >= 5_000),
        b("early", "Early bird", "Add 500 words before 8 in the morning.", early >= 500),
        b("late", "Night owl", "Add 500 words after 10 at night.", late >= 500),
    ]
}

/// A short, kind line for the writer's progress today. Always true to the
/// numbers; never scolds.
pub fn encouragement(p: &Progress, goals: &WritingGoals) -> String {
    encouragement_with(p, goals, &|english| english.to_string())
}

/// [`encouragement`] with each piece of its text passed through `t` (a
/// translation; `{n}` in a text is filled in afterwards).
pub fn encouragement_with(p: &Progress, goals: &WritingGoals, t: &dyn Fn(&str) -> String) -> String {
    let streak_note = match p.streak {
        0 | 1 => String::new(),
        n => t(" That is {n} days in a row.").replace("{n}", &n.to_string()),
    };
    let pick = |lines: &[&str]| t(lines[(p.today.added as usize / 7) % lines.len()]);
    let Some(share) = p.headline() else {
        return if p.today.added > 0 {
            format!("{}{streak_note}", t("{n} words today. Nice work.").replace("{n}", &p.today.added.to_string()))
        } else {
            t("Set a goal in Project › Writing goals if you would like a gentle target.")
        };
    };
    let line = if p.today.is_empty() {
        pick(&[
            "A blank page is just a page waiting. Even ten minutes counts.",
            "Start small: one sentence is a start.",
            "Whenever you are ready. The words will keep.",
        ])
    } else if share < 0.5 {
        pick(&["You have begun, and that is the hardest part.", "Good, steady start.", "Words are arriving. Keep going."])
    } else if share < 1.0 {
        pick(&["More than halfway to today's goal.", "Nearly there. A little more.", "The finish line is in sight."])
    } else if share < 1.5 {
        pick(&["Goal reached. Everything from here is a bonus.", "Today's goal is done. Well written.", "You did what you set out to do."])
    } else {
        pick(&["Well past your goal today. Remember to rest too.", "A very productive day.", "Plenty written. Be proud of it."])
    };
    let weekly = match (p.weekly_words, goals.weekly_words) {
        (Some(w), g) if g > 0 && w >= 1.0 => t(" Your weekly goal is already met."),
        _ => String::new(),
    };
    format!("{line}{streak_note}{weekly}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::record;

    fn log(days: &[(&str, u32)]) -> Vec<ActivityRecord> {
        let mut log = Vec::new();
        for (day, words) in days {
            record(&mut log, &format!("{day}T10"), "d", ActivityTotals { added: *words, typing_secs: words * 3, ..Default::default() });
        }
        log
    }

    #[test]
    fn progress_measures_today_and_the_week() {
        let goals = WritingGoals { daily_words: 500, weekly_words: 2000, daily_minutes: 0, ..Default::default() };
        // 2026-11-26 is a Thursday; the week runs 23rd to 29th.
        let l = log(&[("2026-11-22", 900), ("2026-11-24", 600), ("2026-11-26", 250)]);
        let p = progress(&l, &goals, "2026-11-26");
        assert_eq!(p.today.added, 250);
        assert_eq!(p.week.added, 850, "the 22nd was last week");
        assert_eq!(p.daily_words, Some(0.5));
        assert_eq!(p.weekly_words, Some(850.0 / 2000.0));
        assert_eq!(p.daily_minutes, None, "no minutes goal set");
    }

    #[test]
    fn streaks_and_badges_follow_the_history() {
        let goals = WritingGoals { daily_words: 500, ..Default::default() };
        let l = log(&[("2026-11-22", 600), ("2026-11-23", 700), ("2026-11-24", 800), ("2026-11-25", 100), ("2026-11-26", 550)]);
        let p = progress(&l, &goals, "2026-11-26");
        assert_eq!(p.streak, 1, "the 25th fell short of 500");
        assert_eq!(p.longest_streak, 3);
        let earned: Vec<_> = badges(&l, &goals, "2026-11-26").into_iter().filter(|b| b.earned).map(|b| b.id).collect();
        assert!(earned.contains(&"first-words") && earned.contains(&"goal-met") && earned.contains(&"three-days"));
        assert!(!earned.contains(&"week") && !earned.contains(&"big-day"));
    }

    #[test]
    fn encouragement_is_always_kind_and_true() {
        let goals = WritingGoals { daily_words: 500, ..Default::default() };
        let none = progress(&[], &goals, "2026-11-26");
        assert!(!encouragement(&none, &goals).is_empty());
        let met = progress(&log(&[("2026-11-26", 600)]), &goals, "2026-11-26");
        assert!(encouragement(&met, &goals).to_lowercase().contains("goal") || encouragement(&met, &goals).contains("done"));
        let off = WritingGoals { daily_words: 0, ..Default::default() };
        assert!(encouragement(&progress(&[], &off, "2026-11-26"), &off).contains("goal"));
    }
}
