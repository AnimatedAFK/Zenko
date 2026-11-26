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
        self.session = Some(Session::new(task, minutes));
        self.sleeping = false;
        self.mood_override = None;
        self.set_mood(now, "happy", 2.0);
        self.events.push(Ev::Beep { kind: "start".into(), variety: false });
        let line = format!("Let's go! {} min on \u{201c}{}\u{201d}. I'm watching.", minutes as i64, clip(task, 40));
        self.say(&line, 7.0);
        self.tails_n = (-1e9, 3);
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
        let focused = (s.elapsed - s.dist - s.away).max(0.0);
        add_stats(focused);
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
        } else {
            self.set_mood(now, "sad", 4.0);
            self.events.push(Ev::Beep { kind: "sad".into(), variety: false });
            let line = format!("Stopped. You focused for {} min. Come back soon.", (focused / 60.0).floor() as i64);
            self.say(&line, 8.0);
        }
    }

    /// The once-a-second watcher (sleep when away, wake, session timer). `idle` = seconds since the last input.
    pub fn tick_1s(&mut self, now: f64, dt: f64, idle: f64) {
        let away = idle >= self.cfg.f("idle_minutes", 4.0) * 60.0;
        let has = self.session.is_some();
        if self.sleeping && !away && idle < 3.0 {
            self.sleeping = false;
            self.say(if has { "Welcome back! Let's keep going." } else { "Hi again!" }, 5.0);
        }
        if away && !self.sleeping {
            self.sleeping = true;
            if self.session.as_ref().is_some_and(|s| !s.paused) {
                self.say("Still there? I'll keep the timer running.", 8.0);
            }
        }
        let idle_pause = self.cfg.b("idle_pause", true);
        let Some(s) = &mut self.session else { return };
        if s.paused {
            return;
        }
        if away && idle_pause {
            s.idle_paused = true;
            return;
        }
        if s.idle_paused {
            s.idle_paused = false;
            self.say("Okaeri~ I paused your timer while you were away. Carrying on!", 5.0);
        }
        let Some(s) = &mut self.session else { return };
        s.elapsed += dt;
        if away {
            s.away += dt;
        }
        if s.elapsed >= s.planned {
            self.end_session(now, true);
        }
    }
}
