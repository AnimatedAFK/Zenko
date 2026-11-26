//! Lock-in mode (zenko.py ZenkoExtras lockin_* with the zenko21 pounce and zenko22 counters): she wears her
//! hachimaki, and any distraction that comes to the front gets pounced on and eaten (the tab closed or the app
//! minimised) until the time is up. Hold Esc for 2 seconds, or right-click > Exit lock-in, to stop early.

use crate::play::{Play, play_dur};
use crate::watch::{data, learn_distraction};
use crate::{Companion, Ev, read_json, today_iso, unix_now, update_json};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct Lockin {
    pub until: f64,
    pub eaten: i64,
    pub good: Option<isize>,
    pub last_eat: f64,
    pub last_hwnd: Option<isize>,
    pub t0: f64,
}

/// What the window side has to do for one pounce.
#[derive(Clone, Debug, PartialEq)]
pub struct Eat {
    pub hwnd: isize,
    pub site: String,
    /// the browser tab was closed straight away (Ctrl+W)
    pub tab: bool,
    /// at the end of the pounce: "close" / "minimise" the window (when it wasn't a tab), then bring `good` back
    pub action: String,
    pub good: Option<isize>,
    pub animate: bool,
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn title_case(s: &str) -> String {
    // str.title(): capitalise the first letter of every run of letters
    let mut out = String::new();
    let mut prev_alpha = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            if prev_alpha {
                out.extend(c.to_lowercase());
            } else {
                out.extend(c.to_uppercase());
            }
            prev_alpha = true;
        } else {
            out.push(c);
            prev_alpha = false;
        }
    }
    out
}

pub const LOCKIN_EXTRA: [&str; 5] = ["discord", "whatsapp", "roblox", "crunchyroll", "spotify.exe"];

impl Companion {
    pub fn lockin_active(&self, _now: f64) -> bool {
        self.lockin.is_some()
    }

    /// lock_extra_acc: the hachimaki while locked in
    pub fn lock_extra_acc(&self) -> Option<&'static str> {
        (self.lockin.is_some() && self.cfg.b("lockin_headband", true)).then_some("headband")
    }

    /// zenko_extras.json lockin_until: still locked in after a restart?
    pub fn lockin_pending() -> bool {
        read_json("zenko_extras.json").get("lockin_until").and_then(|v| v.as_f64()).unwrap_or(0.0) > unix_now() + 5.0
    }

    pub fn lockin_start(&mut self, now: f64, minutes: Option<i64>, resume: bool) -> Option<String> {
        let wall = unix_now();
        let until = if resume {
            let u = read_json("zenko_extras.json").get("lockin_until").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if u <= wall {
                return None;
            }
            u
        } else {
            let cfgm = self.cfg.f("lockin_minutes", 50.0) as i64;
            let m = minutes.filter(|&m| m != 0).unwrap_or(if cfgm != 0 { cfgm } else { 50 });
            let u = wall + m.clamp(1, 600) as f64 * 60.0;
            update_json("zenko_extras.json", |v| v["lockin_until"] = json!(u));
            u
        };
        self.lockin = Some(Lockin { until, eaten: 0, good: None, last_eat: 0.0, last_hwnd: None, t0: wall });
        let mins_left = ((until - unix_now()) / 60.0).round_ties_even() as i64;
        if self.session.is_none() && self.cfg.b("lockin_session", true) && !resume {
            self.start_session(now, "Lock-in", 1.max(mins_left) as f64);
        }
        self.clear_images();
        self.set_mood(now, "focus", 2.0);
        self.events.push(Ev::Beep { kind: "start".into(), variety: false });
        let line = format!("Lock-in mode ON for {mins_left} min! *ties hachimaki* If a distraction shows up, I'll pounce \
                            on it and eat it. To stop early: say 'unlock', right-click me > Exit lock-in, or hold Esc \
                            for 2 seconds.");
        self.say(&line, 12.0);
        Some(line)
    }

    pub fn lockin_stop(&mut self, now: f64, done: bool, quiet: bool) -> String {
        let l = self.lockin.take();
        update_json("zenko_extras.json", |v| v["lockin_until"] = json!(0));
        self.clear_images();
        let Some(l) = l else { return "Lock-in isn't on.".into() };
        let n = l.eaten;
        let s = if n == 1 { "" } else { "s" };
        let line = if done {
            self.set_mood(now, "happy", 6.0);
            self.events.push(Ev::Beep { kind: "done".into(), variety: false });
            format!("Lock-in complete! Yatta~! I ate {n} distraction{s} for you. Otsukaresama~kon!")
        } else {
            self.events.push(Ev::Beep { kind: "happy".into(), variety: false });
            format!("Lock-in off. I ate {n} distraction{s} while it was on. Rest well, ne~")
        };
        if !quiet {
            self.say(&line, 10.0);
        }
        if done {
            // zenko22 stat_add("lockins")
            update_json("zenko_life.json", |v| {
                if !v["stats"].is_object() {
                    v["stats"] = json!({});
                }
                let k = v["stats"]["lockins"].as_i64().unwrap_or(0);
                v["stats"]["lockins"] = json!(k + 1);
            });
        }
        line
    }

    pub fn lockin_distracting(&mut self, now: f64, title: &str, proc_: &str) -> Option<String> {
        if let Some(site) = self.is_distracting(now, title, proc_) {
            return Some(site);
        }
        let t = title.to_lowercase();
        for a in self.cfg_list("allow") {
            if !a.is_empty() && t.contains(&a.to_lowercase()) {
                return None;
            }
        }
        let extra: Vec<String> = match self.cfg.raw.get("lockin_extra") {
            Some(v) => strs(v),
            None => LOCKIN_EXTRA.iter().map(|s| s.to_string()).collect(),
        };
        for w in extra {
            let w = w.trim().to_lowercase();
            if w.is_empty() {
                continue;
            }
            if let Some(stem) = w.strip_suffix(".exe") {
                if proc_ == w {
                    return Some(title_case(stem));
                }
            } else if t.contains(&w) {
                return Some(
                    data().raw["PRETTY"].get(&w).and_then(|v| v.as_str()).map(String::from).unwrap_or(title_case(&w)),
                );
            }
        }
        None
    }

    /// _lockin_check, five times a second. `fg` = (hwnd, title, exe, pid) of the window in front, `esc` = Esc held,
    /// `me` = this process. Returns the pounce to carry out, if any.
    pub fn lockin_check(&mut self, now: f64, fg: (isize, &str, &str, u32), esc: bool, me: u32) -> Option<Eat> {
        let l = self.lockin.as_ref()?;
        if unix_now() >= l.until {
            self.lockin_stop(now, true, false);
            return None;
        }
        if esc {
            if self.esc_t == 0.0 {
                self.esc_t = now;
            } else if now - self.esc_t >= 2.0 {
                self.esc_t = 0.0;
                self.lockin_stop(now, false, false);
                return None;
            }
        } else {
            self.esc_t = 0.0;
        }
        if self.lock_eating {
            return None;
        }
        let (hwnd, title, proc_, pid) = fg;
        if hwnd == 0 || (pid != 0 && pid == me) {
            return None;
        }
        let Some(site) = self.lockin_distracting(now, title, proc_) else {
            if let Some(l) = &mut self.lockin {
                l.good = Some(hwnd);
            }
            return None;
        };
        let l = self.lockin.as_ref()?;
        if l.last_hwnd == Some(hwnd) && now - l.last_eat < 1.2 {
            return None;
        }
        Some(self.lockin_eat(now, hwnd, proc_, &site))
    }

    fn lockin_eat(&mut self, now: f64, hwnd: isize, proc_: &str, site: &str) -> Eat {
        let action = self.cfg.s("lockin_action", "smart");
        let browsers = strs(&data().raw["BROWSERS"]);
        let tab = browsers.iter().any(|b| b == proc_) && (action == "smart" || action == "close");
        let good = {
            let l = self.lockin.as_mut().unwrap();
            l.last_eat = now;
            l.last_hwnd = Some(hwnd);
            l.eaten += 1;
            l.good
        };
        let day = today_iso();
        update_json("zenko_extras.json", |v| {
            if !v["eaten"].is_object() {
                v["eaten"] = json!({});
            }
            let e = v["eaten"].as_object_mut().unwrap();
            let k = e.get(&day).and_then(|x| x.as_i64()).unwrap_or(0);
            e.insert(day.clone(), json!(k + 1));
            let mut keys: Vec<String> = e.keys().cloned().collect();
            keys.sort();
            if keys.len() > 60 {
                for k in &keys[..keys.len() - 60] {
                    e.remove(k);
                }
            }
        });
        if self.cfg.b("learn_habits", true) {
            learn_distraction(site);
        }
        self.lock_eating = true;
        self.play = Some(Play { kind: "eat".into(), t0: now, dur: play_dur("eat"), arg: None, found: false });
        self.events.push(Ev::Beep { kind: "chomp".into(), variety: false });
        let lines = strs(&data().raw["LOCK_LINES"]);
        let i = ((self.rand() * lines.len() as f64) as usize).min(lines.len().saturating_sub(1));
        let line = format!("{}  ({site})", lines.get(i).cloned().unwrap_or_default());
        self.say(&line, 7.0);
        Eat {
            hwnd,
            site: site.to_string(),
            tab,
            action: if action == "close" { "close".into() } else { "minimise".into() },
            good,
            animate: self.cfg.b("lockin_animation", true),
        }
    }

    /// _eat_acc: the hachimaki first, then whatever else she's wearing
    pub fn eat_acc(&mut self, now: f64) -> Vec<String> {
        let mut a = vec!["headband".to_string()];
        a.extend(self.accessories(now).into_iter().filter(|x| x != "headband"));
        a
    }

    /// One frame of the pounce, as z21_eat_overlay draws it: rig_rows("rigL:crouch:0") in smooth pixel art at
    /// EAT_HD (4), zoomed the rest of the way by whole steps.
    pub fn eat_image(&mut self, now: f64, side: &str, pose: &str, f: i32, z: usize) -> zenko_art::raster::Image {
        let hd = if z >= 4 { 4 } else { z };
        let k = 1.max(z / hd);
        let n = self.tail_count(now);
        let acc = self.eat_acc(now);
        let a: Vec<&str> = acc.iter().map(|s| s.as_str()).collect();
        let g = zenko_art::rig::rig_rows(&format!("{side}:{pose}:{f}"), n, &a, self.look_ref());
        let im = zenko_art::raster::sprite_hd(&[&g], &self.pal, hd, None);
        if k > 1 { zenko_art::raster::nearest(&im, k) } else { im }
    }

    /// the "Exit lock-in (N min left)" menu line
    pub fn lockin_left_min(&self) -> i64 {
        self.lockin.as_ref().map(|l| ((l.until - unix_now()) / 60.0).floor() as i64).unwrap_or(0).max(0)
    }
}
