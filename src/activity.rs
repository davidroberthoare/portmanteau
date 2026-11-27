//! Works out when the writer is typing and when they are reading, from the
//! input the window receives, and hands the totals to the project's activity
//! log (`portmanteau-core::activity`) every few seconds.
//!
//! The rules, kept simple so they can be explained to the writer:
//! * **Typing** is any typed character, Backspace, Delete, Enter or paste. A
//!   pause of up to five seconds is still typing (thinking mid-sentence).
//! * **Reading** is time with the window in front and the writer scrolling,
//!   clicking or moving with the arrow keys, but not typing. It runs for twenty
//!   seconds after the last such action, then stops (they have wandered off).
//! * Nothing counts while the window is in the background.

use std::time::Duration;
use web_time::Instant;

/// A pause this short between keystrokes is still typing.
pub const TYPING_GRACE: Duration = Duration::from_secs(5);
/// How long after the last scroll or click the writer is taken to be reading.
pub const READING_GRACE: Duration = Duration::from_secs(20);
/// Send what has built up this often.
const FLUSH_EVERY: Duration = Duration::from_secs(15);

/// What happened in one frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameInput {
    /// The window is in front.
    pub focused: bool,
    /// A text field has focus and a character, deletion or paste arrived.
    pub typed: bool,
    /// The writer scrolled, clicked or pressed a navigation key.
    pub navigated: bool,
}

/// Seconds ready to be recorded against a document.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pending {
    pub typing: f32,
    pub reading: f32,
}

#[derive(Default)]
pub struct ActivityTracker {
    last_tick: Option<Instant>,
    last_typing: Option<Instant>,
    last_nav: Option<Instant>,
    last_flush: Option<Instant>,
    pending: Pending,
    /// The document the pending seconds belong to.
    doc: String,
}

fn overlap(from: Instant, to: Instant, start: Instant, end: Instant) -> f32 {
    let a = from.max(start);
    let b = to.min(end);
    if b > a { b.duration_since(a).as_secs_f32() } else { 0.0 }
}

impl ActivityTracker {
    /// Note one frame. `doc` is the document in front, if any.
    pub fn tick(&mut self, now: Instant, doc: Option<&str>, input: FrameInput) {
        let previous = self.last_tick.replace(now);
        // Work out the time since the last frame from what happened *before*
        // this frame, so the gap before a first keystroke is not typing.
        let (last_typing, last_nav) = (self.last_typing, self.last_nav);
        if input.typed {
            self.last_typing = Some(now);
        } else if input.navigated {
            self.last_nav = Some(now);
        }
        let Some(previous) = previous else { return };
        if !input.focused {
            return;
        }
        let doc = doc.unwrap_or("");
        let mut typing = 0.0;
        let mut reading = 0.0;
        let mut typing_end = previous;
        if let Some(last) = last_typing {
            let end = last + TYPING_GRACE;
            typing = overlap(previous, now, previous, end);
            typing_end = end;
        }
        if let Some(last) = last_nav {
            reading = overlap(previous, now, typing_end, last + READING_GRACE);
        }
        if typing + reading > 0.0 {
            if self.doc != doc {
                // Seconds so far belong to the document before; the app flushes
                // on a document change (`take(.., true)`) before the next tick.
                self.doc = doc.to_owned();
            }
            self.pending.typing += typing;
            self.pending.reading += reading;
        }
    }

    /// When the next frame is useful: while typing or reading counts, check in
    /// every second so the time keeps accruing; otherwise not at all.
    pub fn next_check(&self, now: Instant) -> Option<Duration> {
        let live = |last: Option<Instant>, grace: Duration| last.is_some_and(|t| now.duration_since(t) < grace);
        (live(self.last_typing, TYPING_GRACE) || live(self.last_nav, READING_GRACE)).then(|| Duration::from_secs(1))
    }

    /// Whole seconds to record now, with the document they belong to, when it
    /// is time (or `force`d); the fractions stay behind.
    pub fn take(&mut self, now: Instant, force: bool) -> Option<(String, u32, u32)> {
        let due = self.last_flush.is_none_or(|t| now.duration_since(t) >= FLUSH_EVERY);
        if !(force || due) {
            return None;
        }
        let typing = self.pending.typing.floor();
        let reading = self.pending.reading.floor();
        if typing + reading < 1.0 {
            if due {
                self.last_flush = Some(now);
            }
            return None;
        }
        self.pending.typing -= typing;
        self.pending.reading -= reading;
        self.last_flush = Some(now);
        Some((self.doc.clone(), typing as u32, reading as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, secs: u64) -> Instant {
        base + Duration::from_secs(secs)
    }

    fn input(focused: bool, typed: bool, navigated: bool) -> FrameInput {
        FrameInput { focused, typed, navigated }
    }

    #[test]
    fn typing_counts_with_a_grace_pause_and_then_stops() {
        let base = Instant::now();
        let mut t = ActivityTracker::default();
        t.tick(at(base, 0), Some("a"), input(true, false, false));
        t.tick(at(base, 1), Some("a"), input(true, true, false));
        t.tick(at(base, 2), Some("a"), input(true, true, false));
        // Silence for a minute: only the five-second grace counts.
        t.tick(at(base, 62), Some("a"), input(true, false, false));
        assert_eq!(t.pending.typing.round(), 6.0, "1 s between the keys, then the 5 s grace");
        assert_eq!(t.pending.reading, 0.0);
    }

    #[test]
    fn reading_needs_navigation_and_runs_twenty_seconds() {
        let base = Instant::now();
        let mut t = ActivityTracker::default();
        t.tick(at(base, 0), Some("a"), input(true, false, false));
        t.tick(at(base, 3), Some("a"), input(true, false, true));
        t.tick(at(base, 100), Some("a"), input(true, false, false));
        assert_eq!(t.pending.reading.round(), 20.0);
        assert_eq!(t.pending.typing, 0.0);
    }

    #[test]
    fn a_background_window_counts_nothing() {
        let base = Instant::now();
        let mut t = ActivityTracker::default();
        t.tick(at(base, 0), Some("a"), input(true, false, false));
        t.tick(at(base, 2), Some("a"), input(true, true, false));
        t.tick(at(base, 6), Some("a"), input(false, false, false));
        assert_eq!(t.pending, Pending::default());
    }

    #[test]
    fn take_hands_over_whole_seconds_and_keeps_the_rest() {
        let base = Instant::now();
        let mut t = ActivityTracker::default();
        t.pending = Pending { typing: 12.6, reading: 0.4 };
        t.doc = "a".into();
        let (doc, typing, reading) = t.take(at(base, 20), true).unwrap();
        assert_eq!((doc.as_str(), typing, reading), ("a", 12, 0));
        assert!((t.pending.typing - 0.6).abs() < 0.001);
        assert!(t.take(at(base, 21), false).is_none(), "not due yet");
    }
}
