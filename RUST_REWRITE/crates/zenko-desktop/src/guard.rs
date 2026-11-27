//! Stage 3g, the desktop leftovers from zenko.py: deadline_reminders (once a day, every 30 min), books_slow (早寝
//! sleep + burnout guard: wind-down, the bedtime nag with snooze / goodnight, the burnout cap, the rest-day line, late
//! nights) with gym_water_nudge every 30 s, screen_tick + screen_tell (zenko15 screen time: apps, sites, YouTube
//! study videos, private and excluded windows never recorded, the 1-hour nudges) and zk_slow_tick every minute
//! (idle_quiz_check's pop quiz, ib_nudge_check, garden_check, screen_save_maybe, journey_check).
//!
//! Like zenko.py's in-memory life, the per-second screen-time counts wait in a cache that read_json("zenko_life.json")
//! applies; the minute check (or any other life save) writes them.

use crate::plan::{day_n, iso, naive, parse_iso, truthy};
use crate::rewards::Later;
use crate::routes17::routes_data;
use crate::{Companion, Ev, read_json, update_config};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::OnceLock;

/// zenko.py's timers and the bits of state its methods keep on self
#[derive(Clone, Debug, Default)]
pub struct Guard {
    /// self._sleep: nag, snooze, last (this hour's study seconds), studying (when they last went up)
    pub nag: f64,
    pub snooze: f64,
    pub last: Option<f64>,
    pub studying: f64,
    pub water_nudged: f64,
    pub quiz_offer_t: f64,
    /// the pop quiz card on the bubble
    pub popq: Option<Value>,
    pub st_t: Option<f64>,
    pub st_private: HashSet<isize>,
    pub st_dirty: bool,
    /// next deadline_reminders / books_slow_tick / zk_slow_tick
    pub dl_next: f64,
    pub books_next: f64,
    pub zk_next: f64,
}

impl Guard {
    pub fn new(now: f64) -> Guard {
        Guard { quiz_offer_t: now - 25.0 * 60.0, dl_next: now + 15.0, books_next: now + 25.0, zk_next: now + 40.0,
                ..Default::default() }
    }

    /// tests: no timers
    pub fn off(&mut self) {
        self.dl_next = f64::MAX;
        self.books_next = f64::MAX;
        self.zk_next = f64::MAX;
    }
}

// ---- the screen-time cache (life["screen"] not saved yet) -----------------------------------------------------------

thread_local! {
    static SCREEN: RefCell<Option<Value>> = const { RefCell::new(None) };
}

pub(crate) fn apply_screen(l: &mut Value) {
    SCREEN.with(|s| {
        if let Some(v) = s.borrow().as_ref() {
            if !l.is_object() {
                *l = json!({});
            }
            l["screen"] = v.clone();
        }
    });
}

/// save_life wrote it (the next tick reloads, so changes made through life_do are kept)
pub(crate) fn clear_screen() {
    SCREEN.with(|s| *s.borrow_mut() = None);
}

fn with_screen<R>(f: impl FnOnce(&mut Value) -> R) -> R {
    let cur = SCREEN.with(|s| s.borrow().clone());
    let mut v = match cur {
        Some(v) => v,
        None => read_json("zenko_life.json").get("screen").cloned().unwrap_or(Value::Null),
    };
    if !v.is_object() {
        v = json!({});
    }
    if !v["days"].is_object() {
        v["days"] = json!({});
    }
    let r = f(&mut v);
    SCREEN.with(|s| *s.borrow_mut() = Some(v));
    r
}

// ---- screen time: zenko.py's module helpers -------------------------------------------------------------------------

fn py_re(p: &str) -> Regex {
    Regex::new(&format!("(?i){p}")).unwrap()
}

struct Rx {
    private: Regex,
    tail: Regex,
    profile: Regex,
    sites: Vec<(Regex, String)>,
}

fn rx() -> &'static Rx {
    static R: OnceLock<Rx> = OnceLock::new();
    R.get_or_init(|| {
        let d = routes_data();
        let s = |k: &str| d[k].as_str().unwrap().to_string();
        Rx {
            private: py_re(&s("SCREEN_PRIVATE")),
            tail: py_re(&s("SCREEN_BROWSER_TAIL")),
            profile: py_re(&s("SCREEN_PROFILE")),
            sites: d["SCREEN_SITES"].as_array().unwrap().iter()
                .map(|p| (py_re(p[0].as_str().unwrap()), p[1].as_str().unwrap().to_string())).collect(),
        }
    })
}

fn browsers() -> Vec<String> {
    routes_data()["BROWSERS"].as_array().unwrap().iter().filter_map(|v| v.as_str().map(String::from)).collect()
}

fn is_media(n: &str) -> bool {
    routes_data()["SCREEN_MEDIA"].as_array().unwrap().iter().any(|v| v.as_str() == Some(n))
}

/// screen_excluded(text, words): a whole-word match of any word (not inside a longer word or number)
pub fn screen_excluded(text: &str, words: &[String]) -> bool {
    let t = text.to_lowercase();
    let alnum = |c: Option<char>| c.is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    for w in words {
        let w = w.trim().to_lowercase();
        if w.is_empty() {
            continue;
        }
        let mut from = 0;
        while let Some(k) = t[from..].find(&w) {
            let a = from + k;
            let b = a + w.len();
            if !alnum(t[..a].chars().next_back()) && !alnum(t[b..].chars().next()) {
                return true;
            }
            from = a + t[a..].chars().next().map_or(1, |c| c.len_utf8());
        }
    }
    false
}

fn py_split_ws(s: &str) -> usize {
    s.split_whitespace().count()
}

/// screen_site(title): a browser window title -> a short site name ('YouTube'), or 'Other websites'
pub fn screen_site(title: &str) -> String {
    static MORE: OnceLock<Regex> = OnceLock::new();
    static SEP: OnceLock<Regex> = OnceLock::new();
    let r = rx();
    let mut t = r.tail.replace_all(title.trim(), "").to_string();
    t = MORE.get_or_init(|| Regex::new(r"(?i)\s+and \d+ more pages?$").unwrap()).replace_all(&t, "").to_string();
    let mut parts: Vec<String> = SEP.get_or_init(|| Regex::new(r"\s+[-\x{2013}\x{2014}|\x{00b7}:]\s+").unwrap())
        .split(&t).map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
    if parts.len() >= 2 && r.profile.is_match(parts.last().unwrap()) {
        parts.pop();
        t = parts.join(" - ");
    }
    for (re, name) in &r.sites {
        if re.is_match(&t) {
            return name.clone();
        }
    }
    if parts.len() >= 2 {
        let l = parts.last().unwrap();
        let n = l.chars().count();
        if (2..=28).contains(&n) && py_split_ws(l) <= 4 {
            return l.clone();
        }
    }
    "Other websites".into()
}

/// screen_classify(title, proc, own, exclude, private_hwnd): (app, site or "", is_media), or None when it must not
/// be recorded (private / incognito windows, anything on the exclusion list)
pub fn screen_classify(title: &str, proc_: &str, own: bool, exclude: &[String], private_hwnd: bool)
                       -> Option<(String, String, bool)> {
    let (title, proc_) = (title.trim(), proc_.to_lowercase());
    if own {
        return Some(("Zenko".into(), String::new(), false));
    }
    if title.is_empty() && proc_.is_empty() {
        return None;
    }
    if private_hwnd || rx().private.is_match(title) {
        return None;
    }
    let base = proc_.strip_suffix(".exe").unwrap_or(&proc_).to_string();
    if screen_excluded(title, exclude) || screen_excluded(&base, exclude) {
        return None;
    }
    let t28: String = title.chars().take(28).collect();
    let app = match routes_data()["SCREEN_APP_NAMES"].get(proc_.as_str()).and_then(|v| v.as_str()) {
        Some(a) => a.to_string(),
        None if !base.is_empty() => {
            let a = crate::routes17::py_title(base.replace(['_', '-'], " ").trim());
            if a.is_empty() { t28 } else { a }
        }
        None => t28,
    };
    if app.is_empty() {
        return None;
    }
    let site = if browsers().contains(&proc_) { screen_site(title) } else { String::new() };
    let media = is_media(if site.is_empty() { &app } else { &site });
    Some((app, site, media))
}

/// Not on Windows zenko.py guesses its own window from the title (on Windows: the window's process is ours)
pub fn screen_own_guess(title: &str, proc_: &str) -> bool {
    title.starts_with("Zenko") && ["", "python.exe", "pythonw.exe", "zenko.exe"].contains(&proc_)
}

fn py_round1(x: f64) -> f64 {
    format!("{x:.1}").parse().unwrap_or(x)
}

fn add_round(m: &mut Value, k: &str, dt: f64) -> f64 {
    let v = py_round1(m.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) + dt);
    m[k] = json!(v);
    v
}

/// datetime.now() as naive local seconds
fn nl() -> f64 {
    naive(crate::unix_now())
}

fn hour_of(nl: f64) -> i64 {
    (nl.rem_euclid(86400.0) / 3600.0) as i64
}

fn minute_of(nl: f64) -> i64 {
    (nl.rem_euclid(3600.0) / 60.0) as i64
}

fn fmt_hm(m: i64) -> String {
    crate::ema::fmt_hm(m)
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

impl Companion {
    /// the window side calls this every frame: deadline_reminders, books_slow_tick, zk_slow_tick
    pub fn guard_timers(&mut self, now: f64) {
        if now >= self.guard.dl_next {
            self.guard.dl_next = now + 30.0 * 60.0;
            self.deadline_reminders(now);
        }
        if now >= self.guard.books_next {
            self.guard.books_next = now + 30.0;
            self.books_slow(now);
            self.gym_water_nudge(now);
        }
        if now >= self.guard.zk_next {
            self.guard.zk_next = now + 60.0;
            self.zk_slow(now);
        }
    }

    fn say_ch(&mut self, text: String, secs: f64, choices: &[(&str, &str)]) {
        let choices = choices.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        self.events.push(Ev::Say { text, secs, choices });
    }

    /// deadline_reminders(): once a day, what's coming up in the next 3 days
    pub fn deadline_reminders(&mut self, _now: f64) {
        let today = day_n(nl());
        let ts = iso(today);
        if self.cfg.s("last_deadline_note", "") == ts {
            return;
        }
        let soon: Vec<Value> = self.upcoming_deadlines(10).into_iter()
            .filter(|d| parse_iso(d["date"].as_str().unwrap_or("")).is_some_and(|x| x - today <= 3)).collect();
        if let Some(first) = soon.first() {
            update_config(&mut self.cfg, |v| v["last_deadline_note"] = json!(ts));
            let more = if soon.len() > 1 { format!(" (+{} more this week)", soon.len() - 1) } else { String::new() };
            let title = match &first["title"] {
                Value::String(s) => s.clone(),
                v => v.to_string(),
            };
            let text = format!("Heads up: {} is {}{}.", title,
                               crate::study::when_text(first["date"].as_str().unwrap_or(""), today), more);
            self.events.push(Ev::Say { text, secs: 12.0, choices: vec![] });
            self.events.push(Ev::Beep { kind: "ding".into(), variety: false });
        }
    }

    // ---- 早寝 sleep + burnout guard --------------------------------------------------------------------------------

    /// _hm(key, default): "HH:MM" from the settings (DEFAULT_CONFIG has bedtime 23:00 and wake_time 06:00, so those
    /// are what a missing key means)
    fn hm(&self, key: &str, dflt: &str) -> (i64, i64) {
        let parse = |s: &str| -> Option<(i64, i64)> {
            let p: Vec<&str> = s.split(':').collect();
            if p.len() != 2 {
                return None;
            }
            Some((p[0].trim().parse().ok()?, p[1].trim().parse().ok()?))
        };
        let v = match self.cfg.raw.get(key) {
            None => dflt.to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
        };
        parse(&v).unwrap_or_else(|| parse(dflt).unwrap())
    }

    /// today_study_minutes(day): tracked study (or the focus timer's minutes today, whichever is more)
    fn today_study_minutes(&self, life: &Value, day: Option<i64>) -> f64 {
        let d = iso(day.unwrap_or_else(|| day_n(nl())));
        let tracked = life["slog"][d.as_str()].as_object().map_or(0.0, |o| o.values().map(num).sum::<f64>()) / 60.0;
        let focus = if day.is_none() { crate::slow::today_focus_minutes() as f64 } else { 0.0 };
        tracked.max(focus)
    }

    /// late_minutes(n): [(day, minutes studied after bedtime that night)] for the last n nights
    fn late_minutes(&self, life: &Value, n: i64) -> Vec<(i64, f64)> {
        let (bh, bm) = self.hm("bedtime", "23:00");
        let (wh, _) = self.hm("wake_time", "06:00");
        let hl = &life["hourlog"];
        let today = day_n(nl());
        let mut out = vec![];
        for i in (0..n).rev() {
            let d = today - i;
            let mut secs = 0.0;
            if let Some(o) = hl[iso(d).as_str()].as_object() {
                for (h, v) in o {
                    let h: i64 = h.parse().unwrap_or(0);
                    if h > bh || (h == bh && bm == 0) {
                        secs += num(&v[0]);
                    } else if h == bh {
                        secs += num(&v[0]) * (60 - bm) as f64 / 60.0;
                    }
                }
            }
            if let Some(o) = hl[iso(d + 1).as_str()].as_object() {
                for (h, v) in o {
                    if h.parse::<i64>().unwrap_or(0) < wh.min(5) {
                        secs += num(&v[0]);
                    }
                }
            }
            out.push((d, secs / 60.0));
        }
        out
    }

    pub fn books_slow(&mut self, now: f64) {
        if !self.cfg.b("sleep_guard", true) || self.brk.is_some() || self.lockin.is_some() {
            return;
        }
        let n = nl();
        let (bh, bm) = self.hm("bedtime", "23:00");
        let (wh, wm) = self.hm("wake_time", "06:00");
        let today = day_n(n);
        let bed = (today * 86400 + bh * 3600 + bm * 60) as f64;
        let life = read_json("zenko_life.json");
        let hour = hour_of(n);
        let hl = num(&life["hourlog"][iso(today).as_str()][hour.to_string().as_str()][0]);
        let mut studying = self.session.as_ref().is_some_and(|s| !s.paused);
        if self.guard.last.is_some_and(|l| hl > l) {
            self.guard.studying = now;
        }
        self.guard.last = Some(hl);
        studying = studying || now - self.guard.studying < 90.0;
        let mins_to_bed = (bed - n) / 60.0;
        let hm = (hour, minute_of(n));
        let late = n >= bed || hm < (wh.min(5), if wh < 5 { wm } else { 0 });
        if 0.0 < mins_to_bed && mins_to_bed <= 30.0 && studying && !self.said_today("winddown") {
            self.say(&format!("Wind-down time~ bedtime is at {bh:02}:{bm:02}. Finish this bit and save the rest for tomorrow."), 12.0);
            return;
        }
        if late && studying && now > self.guard.snooze && now - self.guard.nag > 20.0 * 60.0 {
            self.guard.nag = now;
            self.set_mood(now, "sleep", 4.0);
            self.say_ch(format!("It's past your bedtime ({bh:02}:{bm:02})... sleep is when your brain stores today's revision. Wrap up?"),
                        25.0, &[("10 more min", "sleep_snooze"), ("Goodnight~", "sleep_goodnight")]);
            return;
        }
        if studying && !late {
            let td = self.today_study_minutes(&life, None);
            let cap = self.cfg.f("burnout_hours", 6.0) * 60.0;
            if td >= cap && !self.said_today("burnout") {
                self.say_ch(format!("You've studied {} today, past your {} max. Your brain needs rest to keep it: take a real break~",
                                    fmt_hm(td as i64), fmt_hm(cap as i64)), 14.0,
                            &[("Take a break", "break:15"), ("I'm okay", "noop")]);
                return;
            }
            let wk = iso(today - crate::lifechat::weekday(today) as i64);
            if life["said"]["restday"].as_str() != Some(wk.as_str()) && hour >= 10 {
                let days: Vec<f64> = (1..8).map(|i| self.today_study_minutes(&life, Some(today - i))).collect();
                if days.iter().cloned().fold(f64::INFINITY, f64::min) >= 120.0 {
                    self.life_do(|l| l["said"]["restday"] = json!(wk));
                    self.say("You've studied 2h+ every day for a week. Sugoi, but plan one lighter day soon so it doesn't burn you out~", 12.0);
                    return;
                }
            }
            let ln = self.late_minutes(&life, 5).iter().filter(|(_, m)| *m >= 30.0).count();
            if ln >= 3 && !self.said_today("latenights") {
                self.say(&format!("That's {ln} late nights in the last 5. Try starting earlier today so you can stop by {bh:02}:{bm:02}~"), 12.0);
            }
        }
    }

    pub fn sleep_goodnight(&mut self, now: f64) {
        let (wh, wm) = self.hm("wake_time", "06:00");
        self.guard.snooze = now + 6.0 * 3600.0;
        if let Some(s) = self.session.as_mut().filter(|s| !s.paused) {
            s.paused = true;
        }
        self.set_mood(now, "sleep", 6.0);
        self.say(&format!("Oyasumi~ \u{304a}\u{3084}\u{3059}\u{307f}. I'll see you at {wh:02}:{wm:02}."), 8.0);
    }

    /// gym_water_nudge(): a gentle "drink some water" when you're behind for the time of day (9:00 to 21:00)
    pub fn gym_water_nudge(&mut self, now: f64) {
        if !self.cfg.b("water_nudge", true) || self.brk.is_some() || self.lockin.is_some() {
            return;
        }
        let n = nl();
        let hour = hour_of(n);
        if !(9..21).contains(&hour) {
            return;
        }
        let goal = crate::gym::GYM_WATER_GOAL;
        let life = read_json("zenko_life.json");
        let cups = life["gym"]["water"][iso(day_n(n)).as_str()].as_f64().unwrap_or(0.0) as i64;
        let expect = (goal as f64 * (hour - 8) as f64 / 13.0) as i64;
        if cups + 2 <= expect && now - self.guard.water_nudged > 100.0 * 60.0 {
            self.guard.water_nudged = now;
            self.say(&format!("Kon~ time for some water? ({cups} of {goal} glasses so far) Tap a cup in Exercise, or tell me 'drank water'."), 9.0);
        }
    }

    // ---- zenko15 screen time --------------------------------------------------------------------------------------

    /// screen_exclude(): the words never recorded (the banking/password defaults until you change them)
    fn screen_exclude(&mut self) -> Vec<String> {
        match self.cfg.raw.get("screen_exclude") {
            Some(Value::Array(a)) => a.iter().map(|v| v.as_str().map(String::from).unwrap_or_else(|| v.to_string())).collect(),
            _ => {
                let d = routes_data()["SCREEN_EXCLUDE_DEFAULT"].clone();
                self.cfg.raw["screen_exclude"] = d.clone();
                d.as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect()
            }
        }
    }

    /// screen_tick(): once a second (after study_tick), with the foreground window's handle and whether it's ours
    pub fn screen_tick(&mut self, now: f64, hwnd: isize, own: bool) {
        let dt = now - self.guard.st_t.unwrap_or(now);
        self.guard.st_t = Some(now);
        if dt <= 0.0 || dt > 5.0 || !self.cfg.b("screen_time", true) {
            return;
        }
        let paused = with_screen(|s| s.get("paused_until").map(num).unwrap_or(0.0));
        if crate::unix_now() < paused {
            return;
        }
        let Some((title, proc_, idle, _)) = self.study.fg.clone() else { return };
        if hwnd != 0 && rx().private.is_match(&title) {
            self.guard.st_private.insert(hwnd); // Chrome only names the first Incognito tab: remember that window
            if self.guard.st_private.len() > 300 {
                self.guard.st_private.clear();
            }
        }
        let ex = self.screen_exclude();
        let priv_ = hwnd != 0 && self.guard.st_private.contains(&hwnd);
        let Some((app, mut site, media)) = screen_classify(&title, &proc_, own, &ex, priv_) else { return };
        if site == "YouTube" && !matches!(self.study_video(now, &title).as_deref(), None | Some("browsing")) {
            site = "YouTube (study)".into();
        }
        if idle > if media || site == "YouTube (study)" { 1800.0 } else { 120.0 } {
            return;
        }
        let n = nl();
        let day = iso(day_n(n));
        let hour = hour_of(n) as usize;
        let is_browser = browsers().contains(&proc_.to_lowercase());
        let tell = with_screen(|s| {
            let days = &mut s["days"];
            if days.get(day.as_str()).is_none() {
                days[day.as_str()] = json!({"apps": {}, "sites": {}, "hours": vec![0; 24]});
            }
            let d = &mut days[day.as_str()];
            let at = add_round(&mut d["apps"], &app, dt);
            let st = if site.is_empty() { 0.0 } else { add_round(&mut d["sites"], &site, dt) };
            if !d["hours"].is_array() {
                d["hours"] = json!(vec![0; 24]);
            }
            let h = &mut d["hours"][hour];
            *h = json!(py_round1(num(h) + dt));
            let key = if site.is_empty() { app.clone() } else { site.clone() };
            if ["Other websites", "New tab", "Google Search"].contains(&key.as_str()) || (site.is_empty() && is_browser) {
                return None;
            }
            let tot = if site.is_empty() { at } else { st };
            if !d["told"].is_object() {
                d["told"] = Value::Object(Map::new());
            }
            let hrs = (tot / 3600.0).floor() as i64;
            if hrs >= 1 && d["told"].get(key.as_str()).and_then(|v| v.as_i64()).unwrap_or(0) < hrs {
                d["told"][key.as_str()] = json!(hrs);
                return Some((key, tot));
            }
            None
        });
        self.guard.st_dirty = true;
        if let Some((key, tot)) = tell {
            if self.cfg.b("screen_nudges", true) {
                self.later(now + 0.01, Later::ScreenTell { key, tot, own, media });
            }
        }
    }

    /// screen_tell(key, total, own, media)
    pub(crate) fn screen_tell(&mut self, key: &str, tot: f64, own: bool, media: bool) {
        if self.session.is_some() || self.brk.is_some() {
            return;
        }
        let span = crate::routes17::screen_fmt_long(tot);
        let msg = if own {
            let t = *self.brain.rng.choice(&["We've spent %s together in the Zenko window today~ I'm flattered! Is the studying happening too?",
                                             "%s in my window today, hehe. Shall we turn some of that into a focus round?"]);
            t.replacen("%s", &span, 1)
        } else if media || ["Instagram", "TikTok", "Reddit", "X (Twitter)", "Facebook", "Snapchat", "Discord"].contains(&key) {
            let t = *self.brain.rng.choice(&["You've been on %s for %s today.", "%s: %s today so far.", "Ne, that's %s for %s today~"]);
            let (a, b) = if t.starts_with("Ne") { (&span, key) } else { (&key.to_string(), span.as_str()) };
            let m = t.replacen("%s", a, 1).replacen("%s", b, 1);
            let tail = *self.brain.rng.choice(&["Want to do one focus round, then come back?", "Maybe a little break from it?",
                                                "Your future self says hi~"]);
            format!("{m} {tail}")
        } else {
            let mut c = span.chars();
            let cap: String = c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default();
            let tail = *self.brain.rng.choice(&["Remember to rest your eyes~", "Nice and steady.", "Stretch break soon?"]);
            format!("{cap} on {key} today. {tail}")
        };
        self.say_ch(msg, 11.0, &[("Screen time", "open_hub:screen"), ("Lock in", "lockin")]);
    }

    /// screen_save_maybe(): the minute's screen-time counts go to the file
    pub fn screen_save_maybe(&mut self) {
        if self.guard.st_dirty {
            self.guard.st_dirty = false;
            let keep = with_screen(|s| s.clone());
            self.life_do(|l| {
                let mut s = keep;
                if let Some(d) = s["days"].as_object_mut() {
                    let mut ks: Vec<String> = d.keys().cloned().collect();
                    ks.sort();
                    let n = ks.len().saturating_sub(60);
                    for k in &ks[..n] {
                        d.shift_remove(k);
                    }
                }
                l["screen"] = s;
            });
        }
    }

    // ---- zk_slow_tick: every minute --------------------------------------------------------------------------------

    pub fn zk_slow(&mut self, now: f64) {
        self.idle_quiz_check(now);
        self.ib_nudge_check();
        self.garden_check();
        self.screen_save_maybe();
        self.journey_check(now);
    }

    /// idle_quiz_check(): a pop quiz from your due flashcards while you're idle at the desk (7:00 to 22:00)
    pub fn idle_quiz_check(&mut self, now: f64) {
        if !self.cfg.b("idle_quiz", true) || self.session.is_some() || self.brk.is_some() || self.lockin.is_some()
            || self.quiz.is_some() {
            return;
        }
        if self.env.idle > 90.0 || self.env.bubble || self.sleeping {
            return;
        }
        let gap = self.cfg.f("idle_quiz_gap", 35.0) * 60.0;
        if now - self.guard.quiz_offer_t < gap {
            return;
        }
        let n = nl();
        let hour = hour_of(n);
        if !(7..22).contains(&hour) {
            return;
        }
        let day = iso(day_n(n));
        let life = read_json("zenko_life.json");
        let cnt = life["quiz_offers"][day.as_str()].as_i64().unwrap_or(0);
        if cnt >= 6 {
            return;
        }
        let mut due = self.cards_due();
        due.sort_by(|a, b| (num(&a["box"]), -num(&a["wrong"])).partial_cmp(&(num(&b["box"]), -num(&b["wrong"]))).unwrap());
        if due.is_empty() {
            return;
        }
        self.guard.quiz_offer_t = now;
        // zenko.py clears the dict before it reads the old count back from it, so this is always 1
        self.life_do(|l| l["quiz_offers"] = json!({day.as_str(): 1}));
        let top: Vec<Value> = due.into_iter().take(4).collect();
        let card = self.brain.rng.choice(&top).clone();
        let tag = match card.get("subject") {
            Some(s) if truthy(s) => format!(" ({})", self.subject_full(s.as_str().unwrap_or(""))),
            _ => String::new(),
        };
        let text = format!("Pop quiz~{tag}\n{}", crate::session::clip(card["q"].as_str().unwrap_or(""), 110));
        self.guard.popq = Some(card);
        self.say_ch(text, 40.0, &[("Show answer", "popq_show"), ("Later", "popq_later")]);
    }

    /// ib_nudge_check(): an IA / EE / TOK piece that's due soon or hasn't moved (15:00 to 21:00)
    pub fn ib_nudge_check(&mut self) {
        if !self.cfg.b("ib_nudges", true) || self.session.is_some() || self.lockin.is_some() {
            return;
        }
        let n = nl();
        let hour = hour_of(n);
        if !(15..21).contains(&hour) || self.env.idle > 120.0 {
            return;
        }
        let today = day_n(n);
        let stages: Vec<String> = routes_data()["IB_STAGES"].as_array().unwrap().iter()
            .map(|v| v.as_str().unwrap().to_string()).collect();
        let tasks = read_json("zenko_life.json")["ibtasks"].as_array().cloned().unwrap_or_default();
        for t in tasks {
            let st: Vec<&str> = t["stages"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
            let due_s = t["due"].as_str().unwrap_or("");
            if due_s.is_empty() || st.contains(&"Final") {
                continue;
            }
            let Some(due) = parse_iso(due_s) else { continue };
            let days = due - today;
            if !(0..=30).contains(&days) {
                continue;
            }
            let upd = t["updated"].as_str().filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(|| iso(today));
            let quiet = parse_iso(&upd).map_or(0, |u| today - u);
            if days <= 3 || (days <= 14 && quiet >= 4) || quiet >= 7 {
                let id = t["id"].as_str().unwrap_or("").to_string();
                if self.said_today(&format!("ibnudge_{id}")) {
                    continue;
                }
                let left = t["target"].as_f64().unwrap_or(0.0) - t["words"].as_f64().unwrap_or(0.0);
                let nxt = stages.iter().find(|s| !st.contains(&s.as_str())).cloned().unwrap_or("Final".into()).to_lowercase();
                let title = t["title"].as_str().unwrap_or("").to_string();
                let mut line = if days <= 3 {
                    format!("Kon! Your {title} is due in {days} day{}. Next step: {nxt}. Want a lock-in for it?",
                            if days == 1 { "" } else { "s" })
                } else {
                    let (_, m, d) = crate::civil_from_days_pub(due);
                    const MON: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                    format!("Your {title} hasn't moved in {quiet} days (due {d:02} {}). Even 20 minutes on the {nxt} helps~",
                            MON[m as usize - 1])
                };
                if left > 0.0 && t["unit"].as_str() == Some("words") && truthy(&t["words"]) {
                    line += &format!(" ({} words to go)", fmt_int_grouped(left));
                }
                self.say_ch(line, 14.0, &[("Lock in 30 min", "lockin:30"), ("Later", "noop")]);
                return;
            }
        }
    }

    /// garden_check(): once a day, if the tree is thirsty, she mentions it (10:00 to 21:00)
    pub fn garden_check(&mut self) {
        let g = read_json("zenko_life.json")["garden"].clone();
        let h = crate::rewards::garden_health(&g);
        let hour = hour_of(nl());
        if h < 0.76 && self.session.is_none() && self.env.idle < 120.0 && (10..21).contains(&hour)
            && !self.said_today("garden_thirsty") {
            let how = if h >= 0.5 { "drooping a little" } else if h > 0.0 { "wilting" } else { "very wilted" };
            self.say_ch(format!("Your garden tree is {how}... One lock-in today will perk it right up~"), 10.0,
                        &[("Lock in 30 min", "lockin:30"), ("Show me", "open_hub:garden")]);
        }
    }

    /// the buttons on these bubbles
    pub fn guard_button(&mut self, now: f64, c: &str) -> bool {
        match c {
            "noop" => {}
            "sleep_snooze" => self.guard.snooze = now + 10.0 * 60.0,
            "sleep_goodnight" => self.sleep_goodnight(now),
            "popq_later" => self.guard.quiz_offer_t = now + 60.0 * 60.0,
            "popq_show" => {
                let Some(card) = self.guard.popq.clone() else { return true };
                let a = crate::session::clip(card["a"].as_str().unwrap_or(""), 120);
                self.say_ch(format!("Answer: {a}\nDid you remember it?"), 30.0, &[("I knew it!", "popq_knew"), ("Forgot", "popq_forgot")]);
            }
            "popq_knew" | "popq_forgot" => {
                let Some(card) = self.guard.popq.clone() else { return true };
                let ok = c == "popq_knew";
                self.srs_update(now, &card, ok, "");
                self.say(if ok { "Sugoi~ that one's sticking!" } else { "I'll bring it back soon, ne~" }, 4.0);
            }
            _ => {
                if let Some(m) = c.strip_prefix("break:") {
                    self.start_break(now, m.parse().ok());
                    return true;
                }
                return false;
            }
        }
        true
    }
}

/// fmt_num(int): "1,400"
fn fmt_int_grouped(v: f64) -> String {
    let s = (v.round() as i64).abs().to_string();
    let mut o = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            o.push(',');
        }
        o.push(ch);
    }
    if v < 0.0 { format!("-{o}") } else { o }
}

/// tests: today's date and hour (local)
pub fn test_day_hour() -> (String, i64) {
    let n = nl();
    (iso(day_n(n)), hour_of(n))
}
