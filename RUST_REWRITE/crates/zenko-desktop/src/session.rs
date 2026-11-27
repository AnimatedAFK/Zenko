//! Focus sessions (zenko.py start_session, toggle_pause, end_session and the session part of _tick).
//! Distraction checks, nags, pep talks, breaks and the session dialog come in the next stage.

use crate::{Companion, Ev, Session, data_dir, read_json, today_iso};
use serde_json::Value;

/// mins(seconds)
pub fn mins(seconds: f64) -> i64 {
    1.max((seconds / 60.0).round_ties_even() as i64)
}

pub fn clip(text: &str, n: usize) -> String {
    if text.chars().count() <= n {
        text.to_string()
    } else {
        text.chars().take(n - 1).collect::<String>() + "\u{2026}"
    }
}

/// add_stats(seconds): stats.json is {"YYYY-MM-DD": seconds}, written like json.dump(st, f).
pub fn add_stats(seconds: f64) {
    let mut st = read_json("stats.json");
    if !st.is_object() {
        st = Value::Object(Default::default());
    }
    let day = today_iso();
    let old = st.get(&day).and_then(|v| v.as_f64()).unwrap_or(0.0);
    st[day.as_str()] = Value::from((old + seconds).round_ties_even() as i64);
    let mut out = String::from("{");
    if let Some(o) = st.as_object() {
        for (i, (k, v)) in o.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&serde_json::to_string(k).unwrap_or_default());
            out.push_str(": ");
            out.push_str(&serde_json::to_string(v).unwrap_or_default());
        }
    }
    out.push('}');
    let _ = std::fs::create_dir_all(data_dir());
    let _ = std::fs::write(data_dir().join("stats.json"), out);
}

impl Companion {
    pub fn set_mood(&mut self, now: f64, mood: &str, secs: f64) {
        self.mood_override = Some((mood.into(), now + secs));
    }

    pub fn start_session(&mut self, now: f64, task: &str, minutes: f64) {
        self.start_session_item(now, task, minutes, None);
    }

    /// start_session(task, minutes, item): `item` = the timetable item it came from
    pub fn start_session_item(&mut self, now: f64, task: &str, minutes: f64, item: Option<usize>) {
        let mut ns = Session::new(task, minutes);
        ns.item = item;
        self.session = Some(ns);
        self.sleeping = false;
        self.mood_override = None;
        self.set_mood(now, "happy", 2.0);
        self.events.push(Ev::Beep { kind: "start".into(), variety: false });
        let line = format!("Let's go! {} min on \u{201c}{}\u{201d}. I'm watching.", minutes as i64, clip(task, 40));
        self.say(&line, 7.0);
        self.tails_n = (-1e9, 3);
        self.music_apply_soon(now);
    }

    pub fn toggle_pause(&mut self) {
        let Some(s) = &mut self.session else { return };
        s.paused = !s.paused;
        let p = s.paused;
        self.mood_override = None;
        self.say(if p { "Paused. Don't wander off..." } else { "Back to work!" }, 5.0);
    }

    pub fn end_session(&mut self, now: f64, completed: bool) {
        let Some(s) = self.session.take() else { return };
        if completed && self.cfg.b("focus_glow", true) && self.hub_page.is_some() {
            self.glow_burst = Some(now); // z19 end_session: petals along the window frame
        }
        self.music_apply_soon(now);
        let focused = (s.elapsed - s.dist - s.away).max(0.0);
        add_stats(focused);
        self.session_rewards(now, &s, completed, focused);
        if self.cfg.b("learn_habits", true) && s.elapsed >= 120.0 {
            crate::watch::learn_session(s.planned / 60.0, focused / 60.0, completed, s.dist / 60.0);
        }
        if completed {
            if let Some(i) = s.item {
                self.tt_mark(now, i, true);
            }
        }
        self.watch.alerted = false;
        self.tails_n = (-1e9, 3);
        self.mood_override = None;
        if completed {
            self.set_mood(now, "happy", 9.0);
            self.events.push(Ev::Beep { kind: "done".into(), variety: false });
            let verdict = if s.dist < 60.0 {
                "Spotless!"
            } else if s.dist < s.planned * 0.25 {
                "Not bad."
            } else {
                "We'll do better."
            };
            let line = format!("Done! {} min on \u{201c}{}\u{201d}. Off-task: {} min. {}", mins(s.planned),
                               clip(&s.task, 30), (s.dist / 60.0).floor() as i64, verdict);
            self.say(&line, 14.0);
            if self.pomo_session_done(now, true) {
            } else if self.cfg.b("auto_break_after_session", true) && self.brk.is_none() {
                let bm = self.cfg.f("break_minutes", 5.0) as i64 * if s.planned >= 3000.0 { 2 } else { 1 };
                self.watch.auto_break = Some((now + 7.0, bm));
            }
        } else {
            self.pomo_session_done(now, false);
            self.set_mood(now, "sad", 4.0);
            self.events.push(Ev::Beep { kind: "sad".into(), variety: false });
            let line = format!("Stopped. You focused for {} min. Come back soon.", (focused / 60.0).floor() as i64);
            self.say(&line, 8.0);
        }
    }
}
