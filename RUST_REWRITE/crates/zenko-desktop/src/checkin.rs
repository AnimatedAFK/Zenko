//! zenko.py's daily mood check-in: open_checkin (the little "how are you feeling today?" card beside her),
//! checkin_save (the mood journal in zenko_life.json) and chat_line (the chat log her answer goes into).

use crate::rewards::Later;
use crate::ui::{Fill, Font, Pack, Side, Ui};
use crate::{Companion, Ev, today_iso, unix_now};
use serde_json::{Value, json};
use zenko_art::palette::mixhex;

/// KEY: the colour that becomes see-through
pub const KEY: &str = "#ff00ff";

pub const MOODS: [(&str, &str, &str); 5] = [("great", "Great", "#5fd39a"), ("good", "Good", "#9ad35f"),
                                            ("okay", "Okay", "#e9c46a"), ("meh", "Meh", "#f29b5a"),
                                            ("rough", "Rough", "#ff6b7d")];

pub fn mood_reply(mood: &str) -> &'static [&'static str] {
    match mood {
        "great" => &["Yatta~! I love that for you! Let's ride that energy, ne~?",
                     "Waai~ a great day! Kon kon~ let's make it even better!"],
        "good" => &["Ehehe~ good is good! Let's do our best today~kon!", "Kon! Glad to hear it. One step at a time today, ne~"],
        "okay" => &["Okay is perfectly okay~ Let's start small: one short session?",
                    "Fumu~ an okay day. I'll keep you company, ne~"],
        "meh" => &["Uu~ meh days happen. Let's try just 10 minutes, and I'll cheer the whole time~",
                   "Gyu~ *fluffy tail hug* Small steps today, ne?"],
        "rough" => &["Auu~ I'm sorry it's rough. Want to do a breathing exercise with me first? Say 'breathe'.",
                     "*curls up next to you* Rough days are allowed. Be gentle with yourself today, ne~ Drink some water too."],
        _ => &["Kon~ thank you for telling me."],
    }
}

impl Companion {
    /// user_short(): the first word of the user's name
    pub fn user_short(&self) -> String {
        self.cfg.s("user_name", "").trim().split(' ').next().unwrap_or("").to_string()
    }

    /// open_checkin's window, built the way zenko.py builds it. Button commands: "mood:<key>" and "skip"; the
    /// note entry is named "note".
    pub fn checkin_ui(&self, font_ui: &str) -> Ui {
        let th = crate::theme::theme(&self.cfg);
        let t = &th;
        let mut u = Ui::new(KEY);
        let f = u.round_frame(0, &t.bub, &t.gold, 16, "round", 10, &t.ink);
        u.pack(f, Pack::default().fill(Fill::Both).expand());
        let bg = t.bub.clone();
        let nm = self.user_short();
        let head = format!("Kon kon~ how are you feeling today{}?", if nm.is_empty() { String::new() } else { format!(", {nm}") });
        let l = u.label(f, &head, &bg, &t.ink, Font::new(font_ui, 11, true));
        u.pack(l, Pack::default().padx(12, 12).pady(10, 6));
        let r = u.frame(f, &bg);
        u.pack(r, Pack::default().padx(12, 12));
        for (key, lab, col) in MOODS {
            let b = u.button(r, lab, &format!("mood:{key}"), &mixhex(&bg, col, 0.55), &t.ink, font_ui, 9, 7, 1, true);
            u.pack(b, Pack::default().side(Side::Left).padx(2, 2));
        }
        let e = u.entry(f, 34, Font::new(font_ui, 11, false), "", &t.field);
        u.named(e, "note");
        u.pack(e, Pack::default().fill(Fill::X).padx(14, 14).pady(8, 2).ipady(1));
        let l = u.label(f, "(optional note: what's on your mind?)", &bg, &t.grey, Font::new(font_ui, 8, false));
        u.pack(l, Pack::default());
        let b = u.button(f, "Not now", "skip", &t.field, &t.ink, font_ui, 8, 8, 0, true);
        u.pack(b, Pack::default().pady(4, 10));
        u
    }

    /// where open_checkin puts its window (ww x hh), given her window's top-left and the screen size
    pub fn checkin_pos(&self, root: (i32, i32), ww: i32, hh: i32, screen: (i32, i32)) -> (i32, i32) {
        let mut x = root.0 + self.w - ww - (10.0 * self.u) as i32;
        let mut y = root.1 - hh - (4.0 * self.u) as i32;
        if y < 0 {
            (x, y) = (root.0 + self.colw - ww, root.1 + (10.0 * self.u) as i32);
        }
        (0.max((screen.0 - ww).min(x)), 0.max((screen.1 - hh).min(y)))
    }

    /// open_checkin(force): false when a session or lock-in keeps it closed
    pub fn checkin_opening(&mut self, now: f64, force: bool) -> bool {
        if !force && (self.session.is_some() || self.lockin.is_some()) {
            return false;
        }
        self.events.push(Ev::Beep { kind: "hello".into(), variety: false });
        self.set_mood(now, "happy", 3.0);
        true
    }

    /// checkin_save(mood, note); the window side closes the card
    pub fn checkin_save(&mut self, now: f64, mood: &str, note: &str) {
        let today = today_iso();
        let note: String = note.trim().chars().take(300).collect();
        let t = unix_now().floor() as i64;
        let n = self.life_do(|l| {
            let mut j: Vec<Value> = l["journal"].as_array().cloned().unwrap_or_default().into_iter()
                .filter(|e| e.get("date").and_then(|v| v.as_str()) != Some(today.as_str())).collect();
            j.push(json!({"date": today, "mood": mood, "note": note, "t": t}));
            let k = j.len().saturating_sub(400);
            l["journal"] = Value::Array(j.split_off(k));
            l["last_checkin"] = json!(today);
            l["journal"].as_array().map(|a| a.len()).unwrap_or(0)
        });
        let opts: Vec<String> = mood_reply(mood).iter().map(|s| s.to_string()).collect();
        let line = self.pick_s(&opts);
        self.events.push(Ev::Say { text: line.clone(), secs: 10.0, choices: vec![] });
        self.chat_line(now, "Fox", &line);
        let good = matches!(mood, "great" | "good");
        self.set_mood(now, if good { "happy" } else { "sad" }, 4.0);
        let k = if good || mood == "okay" { "happy" } else { "pet" };
        self.events.push(Ev::Beep { kind: k.into(), variety: false });
        if n >= 7 {
            self.award(now, "journal7");
        }
    }

    /// chat_line(who, text): into the chat log; her lines get the reply sound and, with speak_mode "always", her voice
    pub fn chat_line(&mut self, now: f64, who: &str, text: &str) {
        self.chat_log.push((who.into(), text.into()));
        let k = self.chat_log.len().saturating_sub(200);
        self.chat_log.drain(..k);
        if who == "You" {
            self.events.push(Ev::Ui("send".into()));
        } else {
            self.later(now + 0.26, Later::Ui("reply".into()));
        }
        if who == "Fox" && self.should_speak(text) {
            self.events.push(Ev::Speak(text.into()));
        }
    }

    /// should_speak(text) without a voice conversation running
    pub fn should_speak(&self, text: &str) -> bool {
        let mode = self.cfg.s("speak_mode", "voice");
        if mode == "off" || text.is_empty() {
            return false;
        }
        if ["(thinking...)", "Looking that up...", "Checking...", "On it...", "Looking at your screen..."]
            .contains(&text.trim()) || text.starts_with("(thinking") || text.starts_with("Reading \u{201c}")
        {
            return false;
        }
        mode == "always"
    }
}
