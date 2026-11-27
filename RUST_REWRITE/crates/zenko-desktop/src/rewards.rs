//! What a finished session or lock-in earns (zenko.py ZenkoLife + the zenko14 garden + zenko15/22 journey + zenko22
//! badges): her tummy fills, ema badges (plain, tiered and secret), the garden tree grows, the journey moves on,
//! koi bait, and the counters in zenko_life.json. Everything is written to zenko_life.json the way save_life does
//! (indent 1, ASCII escapes, through a .tmp file), so zenko.py reads it as its own.

use crate::{Companion, Ev, Session, data_dir, read_json, stats, today_iso, unix_now};
use serde_json::{Value, json};
use std::sync::OnceLock;

pub fn rdata() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/rewards.json")).expect("rewards.json"))
}

fn arr(v: &Value) -> &[Value] {
    v.as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

/// save_life: json.dump(d, f, indent=1) through LIFE_PATH + ".tmp"
pub fn save_life(v: &Value) {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let tmp = dir.join("zenko_life.json.tmp");
    if std::fs::write(&tmp, crate::py_json_ascii_indent(v, b" ")).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join("zenko_life.json"));
    }
}

/// load_life (a dict, or {}) with the setdefaults the parts used here rely on (life_init, books_init, journey,
/// fishing), then f, then save_life.
pub fn update_life<R>(rand: &mut dyn FnMut() -> f64, f: impl FnOnce(&mut Value) -> R) -> R {
    let mut l = read_json("zenko_life.json");
    if !l.is_object() {
        l = json!({});
    }
    ensure(&mut l, rand);
    let r = f(&mut l);
    save_life(&l);
    r
}

fn setdef(v: &mut Value, k: &str, d: Value) {
    if v.get(k).is_none() {
        v[k] = d;
    }
}

/// The setdefaults zenko.py runs on zenko_life.json at start-up, in its order (life_init, study_init, books_init with
/// the zenko14 garden, screen and journey, z18_init, z22_migrate with fishing), so a file this writes has the same
/// keys in the same order as one zenko.py wrote.
fn ensure(l: &mut Value, rand: &mut dyn FnMut() -> f64) {
    // life_init
    setdef(l, "free", rdata()["DEFAULT_FREE"].clone());
    for k in ["free_dates", "plan_hours", "studied", "facts", "badges", "said", "stats", "counted"] {
        setdef(l, k, json!({}));
    }
    for k in ["journal", "cards", "found", "notes"] {
        setdef(l, k, json!([]));
    }
    setdef(l, "hunger", json!(30.0));
    setdef(l, "hunger_t", json!(unix_now()));
    // study_init
    for k in ["slog", "dlog", "hourlog", "win_subj", "subj_words", "goals", "unsorted", "nb_hist"] {
        setdef(l, k, json!({}));
    }
    // books_init (FoxBuddy, then Zenko)
    setdef(l, "grades", json!([]));
    setdef(l, "fr_vocab", json!({}));
    setdef(l, "book_state", json!({}));
    setdef(l, "ibtasks", json!([]));
    setdef(l, "garden", json!({}));
    if l["garden"].is_object() {
        let seed = 1 + ((rand() * 9998.0) as i64).min(9997);
        let g = &mut l["garden"];
        setdef(g, "xp", json!(0.0));
        setdef(g, "health", json!(1.0));
        setdef(g, "active", json!([]));
        setdef(g, "log", json!([]));
        setdef(g, "grove", json!([]));
        setdef(g, "seed", json!(seed));
        setdef(g, "planted", json!(today_iso()));
    }
    setdef(l, "screen", json!({"days": {}}));
    journey(l);
    // z18_init
    setdef(l, "ema", json!([]));
    // z21_migrate_free: whole free hours become both their half hours
    if !l.get("free_half").is_some_and(|v| v.as_bool() == Some(true) || v.as_f64().is_some_and(|x| x != 0.0)) {
        for k in ["free", "free_dates"] {
            let src = l.get(k).and_then(|v| v.as_object()).cloned().unwrap_or_default();
            let m: serde_json::Map<String, Value> = src.into_iter().map(|(d, v)| (d, json!(hours_to_slots(&v)))).collect();
            l[k] = Value::Object(m);
        }
        l["free_half"] = json!(true);
    }
    // z22_migrate
    let st_ok = l["stats"].get("sessions").is_some() && l["stats"].get("lockins").is_some();
    if !st_ok {
        let log = arr(&l["garden"]["log"]).to_vec();
        let n = |k: &str| log.iter().filter(|e| e["k"] == k).count() as i64;
        let (fo, lo) = (n("focus"), n("lockin"));
        setdef(&mut l["stats"], "sessions", json!(fo));
        setdef(&mut l["stats"], "lockins", json!(lo));
    }
    setdef(l, "btier", json!({}));
    setdef(l, "badge_pins", json!([]));
    journey(l);
    setdef(&mut l["journey"], "quiz", json!([]));
    setdef(l, "fishing", json!({}));
    let f = &mut l["fishing"];
    for (k, d) in [("bait", json!(5.0)), ("gold", json!(0)), ("book", json!({})), ("log", json!([])), ("pond", json!([])),
                   ("right", json!(0)), ("asked", json!(0)), ("row", json!(0)), ("free_day", json!("")),
                   ("depth", json!(1)), ("src", json!("mix")), ("subj", json!("any"))] {
        setdef(f, k, d);
    }
}

/// hours_to_slots: whole hours -> both half-hour slots of each hour
pub fn hours_to_slots(v: &Value) -> Vec<f64> {
    let mut out: Vec<f64> = vec![];
    for h in v.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        let h = h.as_f64().unwrap_or(0.0);
        if h == h.trunc() {
            out.extend([h, h + 0.5]);
        } else {
            out.push((h * 2.0).round_ties_even() / 2.0);
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out.dedup();
    out.retain(|s| (0.0..24.0).contains(s));
    out
}

/// journey(): the setdefaults, with lock_min counted once from the garden log
fn journey(l: &mut Value) {
    setdef(l, "journey", json!({}));
    setdef(&mut l["journey"], "seen", json!(["inari"]));
    if l["journey"].get("lock_min").is_none() {
        let mut s = 0.0;
        let mut all_int = true;
        for e in arr(&l["garden"]["log"]).iter().filter(|e| e["k"] == "lockin") {
            match &e["m"] {
                Value::Number(n) if n.is_i64() => s += n.as_f64().unwrap_or(0.0),
                Value::Number(n) => {
                    all_int = false;
                    s += n.as_f64().unwrap_or(0.0)
                }
                _ => {}
            }
        }
        l["journey"]["lock_min"] = if all_int { json!(s as i64) } else { json!(s) };
    }
}

fn py_round1(x: f64) -> f64 {
    // round(x, 1): Python rounds the decimal value half-even; the float is close enough to its shortest repr
    let s = format!("{:.1}", x);
    s.parse().unwrap_or(x)
}

/// Days since 1970-01-01 in local time, and the ISO date `k` days before today.
fn local_days() -> i64 {
    let (y, m, d, _, _, _) = crate::local_now();
    days_from_civil(y as i64, m as i64, d as i64)
}

pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn iso(days: i64) -> String {
    let (y, m, d) = crate::civil_from_days_pub(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn parse_iso(s: &str) -> Option<i64> {
    let p: Vec<&str> = s.split('-').collect();
    if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 {
        return None;
    }
    Some(days_from_civil(p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?))
}

fn stat_secs(st: &Value, day: &str) -> f64 {
    st.get(day).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

/// streak(): days in a row with 10+ minutes (today counts once it has them)
pub fn streak() -> i64 {
    let st = stats();
    let mut d = local_days();
    if stat_secs(&st, &iso(d)) < 600.0 {
        d -= 1;
    }
    let mut n = 0;
    while stat_secs(&st, &iso(d)) >= 600.0 {
        n += 1;
        d -= 1;
    }
    n
}

pub fn total_hours() -> f64 {
    stats().as_object().map(|o| o.values().filter_map(|v| v.as_f64()).sum::<f64>()).unwrap_or(0.0) / 3600.0
}

/// z22_moon_age: days since the last new moon, from local wall time
pub fn moon_age(local_unix: f64) -> f64 {
    let reference = days_from_civil(2000, 1, 6) as f64 * 86400.0 + 18.0 * 3600.0 + 14.0 * 60.0;
    ((local_unix - reference) / 86400.0).rem_euclid(29.530588853)
}

/// journey_state: (hours, ids reached in order, next place)
pub fn journey_state(l: &Value) -> (f64, Vec<String>) {
    let hrs = total_hours() + l["journey"]["lock_min"].as_f64().unwrap_or(0.0) / 60.0;
    let got = arr(&rdata()["JOURNEY_ALL"])
        .iter()
        .filter(|p| hrs >= p[3].as_f64().unwrap_or(1e9))
        .map(|p| p[0].as_str().unwrap_or("").to_string())
        .collect();
    (hrs, got)
}

/// z22_unlocked("accent") -> accent keys you've travelled far enough for (cached for 30 s; read by theme())
pub fn unlocked_accents() -> Vec<String> {
    use std::sync::Mutex;
    static C: Mutex<(f64, Vec<String>)> = Mutex::new((-1e18, vec![]));
    let mut g = C.lock().unwrap();
    let now = unix_now();
    if now - g.0 > 30.0 {
        let l = read_json("zenko_life.json");
        let (_, got) = journey_state(&l);
        g.1 = arr(&rdata()["MAP_REWARDS"])
            .iter()
            .filter(|r| r[1] == "accent" && got.iter().any(|p| r[0] == p.as_str()))
            .filter_map(|r| r[2].as_str().map(String::from))
            .collect();
        g.0 = now;
    }
    g.1.clone()
}

/// Jobs zenko.py puts on root.after: a line said later, a badge check later.
#[derive(Clone, Debug)]
pub enum Later {
    Say { text: String, secs: f64, choices: Vec<(String, String)>, only_idle: bool, beep: Option<String> },
    BadgeCheck,
    /// pomodoro: start_break(m) unless a session or break is on
    Break(i64),
    /// pomodoro: the next round, if it's still the same pomodoro
    PomoGo(u64),
    MusicApply,
    /// tablet_press next/previous: forget the last song so the new one is announced
    ForgetSong,
    /// root.after(ms, ...) of something the window side does (Ev::Command)
    Command(String),
    /// chat_line's reply sound, 260 ms later
    Ui(String),
    /// zenko23: plan_maybe a moment after an ema changed
    PlanMaybe,
    /// zenko23: z23_announce(ema id)
    Announce(String),
}

#[derive(Clone, Debug, Default)]
pub struct Rewards {
    pub later: Vec<(f64, Later)>,
    pub metrics_t: f64,
    pub next_tick: Option<f64>,
}

fn garden_stage(xp: f64) -> usize {
    let mut i = 0;
    for (k, st) in arr(&rdata()["GARDEN_STAGES"]).iter().enumerate() {
        if xp >= st[0].as_f64().unwrap_or(0.0) {
            i = k;
        }
    }
    i
}

/// garden_health: one day off is a free rest day, after that each skipped day costs a quarter
fn garden_health(g: &Value) -> f64 {
    let h = g["health"].as_f64().unwrap_or(1.0);
    let Some(last) = arr(&g["active"]).last().and_then(|v| v.as_str()) else { return h };
    let Some(ld) = parse_iso(last) else { return h };
    let missed = local_days() - ld - 1;
    let eff = (missed - 1).max(0);
    (h - 0.25 * eff as f64).clamp(0.0, 1.0)
}

fn notes_count() -> i64 {
    fn walk(p: &std::path::Path, n: &mut i64) {
        let Ok(rd) = std::fs::read_dir(p) else { return };
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                if path.file_name().and_then(|s| s.to_str()) == Some(".bin") {
                    continue;
                }
                walk(&path, n);
            } else {
                let low = path.to_string_lossy().to_lowercase();
                if low.ends_with(".md") || low.ends_with(".txt") {
                    *n += 1;
                }
            }
        }
    }
    let mut n = 0;
    walk(&data_dir().join("notebooks"), &mut n);
    n
}

/// z22_metrics: what the tiered badges count
pub fn metrics(l: &Value) -> serde_json::Map<String, Value> {
    let st = &l["stats"];
    let f = &l["fishing"];
    let (hrs, got) = journey_state(l);
    let gi = |v: &Value| v.as_i64().unwrap_or(0);
    let mut m = serde_json::Map::new();
    let book = f["book"].as_object();
    m.insert("fish".into(), json!(book.map(|b| b.values().map(|v| gi(&v["n"])).sum::<i64>()).unwrap_or(0)));
    m.insert("species".into(), json!(book.map(|b| b.len()).unwrap_or(0)));
    m.insert("places".into(), json!(got.len()));
    m.insert("quizzes".into(), json!(arr(&l["journey"]["quiz"]).len()));
    for k in ["reviews", "sessions", "lockins", "breaths"] {
        m.insert(k.into(), json!(gi(&st[k])));
    }
    m.insert("hours".into(), json!(hrs));
    m.insert("checkins".into(), json!(arr(&l["journal"]).len()));
    m.insert("fishq".into(), json!(gi(&f["right"])));
    m.insert("trees".into(), json!(arr(&l["garden"]["grove"]).len()));
    let s = stats();
    let mut days: Vec<i64> = s
        .as_object()
        .map(|o| {
            o.iter()
                .filter(|(_, v)| v.as_f64().unwrap_or(0.0) >= 600.0)
                .filter_map(|(k, _)| parse_iso(k))
                .collect()
        })
        .unwrap_or_default();
    days.sort();
    let (mut best, mut run, mut prev) = (0, 0, None::<i64>);
    for d in days {
        run = if prev.is_some_and(|p| d - p == 1) { run + 1 } else { 1 };
        best = best.max(run);
        prev = Some(d);
    }
    m.insert("beststreak".into(), json!(best));
    m.insert("notes".into(), json!(notes_count()));
    let mut tot: std::collections::HashMap<String, f64> = Default::default();
    if let Some(o) = l["slog"].as_object() {
        for day in o.values() {
            if let Some(d) = day.as_object() {
                for (sid, secs) in d {
                    *tot.entry(sid.clone()).or_default() += secs.as_f64().unwrap_or(0.0);
                }
            }
        }
    }
    let ids = arr(&rdata()["SUBJECT_IDS"]);
    let mn = ids.iter().map(|i| tot.get(i.as_str().unwrap_or("")).copied().unwrap_or(0.0)).fold(f64::INFINITY, f64::min);
    m.insert("subjects".into(), json!(if ids.is_empty() { 0.0 } else { mn / 3600.0 }));
    m
}

impl Companion {
    fn rnd(&mut self) -> f64 {
        self.rand()
    }

    pub(crate) fn life_do<R>(&mut self, f: impl FnOnce(&mut Value) -> R) -> R {
        let mut seq: Vec<f64> = (0..4).map(|_| self.rnd()).collect();
        let mut r = || seq.pop().unwrap_or(0.5);
        update_life(&mut r, f)
    }

    /// start-up: zenko.py's init setdefaults + z22_migrate's life_save
    pub fn life_startup(&mut self) {
        self.life_do(|_| ());
    }

    pub(crate) fn later(&mut self, at: f64, job: Later) {
        self.rewards.later.push((at, job));
    }

    /// Run the jobs that are due (called every frame by the window side).
    pub fn pump(&mut self, now: f64) {
        self.planner_timers(now);
        let (due, keep): (Vec<_>, Vec<_>) = self.rewards.later.drain(..).partition(|(at, _)| *at <= now);
        self.rewards.later = keep;
        for (_, job) in due {
            match job {
                Later::Say { text, secs, choices, only_idle, beep } => {
                    if only_idle && self.session.is_some() {
                        continue;
                    }
                    self.events.push(Ev::Say { text, secs, choices });
                    if let Some(b) = beep {
                        self.events.push(Ev::Beep { kind: b, variety: false });
                    }
                }
                Later::BadgeCheck => self.z22_badge_check(now),
                Later::Break(m) => {
                    if self.session.is_none() && self.brk.is_none() {
                        self.start_break(now, Some(m));
                    }
                }
                Later::PomoGo(id) => self.pomo_go(now, id),
                Later::MusicApply => self.music_apply(),
                Later::ForgetSong => self.music.last_song = None,
                Later::Command(c) => self.events.push(crate::Ev::Command(c)),
                Later::Ui(k) => self.events.push(crate::Ev::Ui(k)),
                Later::PlanMaybe => self.plan_maybe(),
                Later::Announce(id) => self.z23_announce(&id),
            }
        }
        // z22_badge_tick: 30 s after start, then every 3 minutes
        match self.rewards.next_tick {
            None => self.rewards.next_tick = Some(now + 30.0),
            Some(t) if now >= t => {
                self.rewards.next_tick = Some(now + 180.0);
                self.z22_badge_check(now);
            }
            _ => {}
        }
    }

    /// feed(n): hunger catches up (3 points an hour), then goes down by n
    pub fn feed_tummy(&mut self, n: f64) {
        self.life_do(|l| {
            let now = unix_now();
            let h0 = l["hunger"].as_f64().unwrap_or(30.0);
            let t = l["hunger_t"].as_f64().unwrap_or(now);
            let h = (h0 + (now - t) / 3600.0 * 3.0).clamp(0.0, 100.0);
            l["hunger"] = json!((h - n).max(0.0));
            l["hunger_t"] = json!(now);
        });
    }

    pub fn stat_add(&mut self, key: &str, n: i64) -> i64 {
        self.life_do(|l| {
            let v = l["stats"][key].as_i64().unwrap_or(0) + n;
            l["stats"][key] = json!(v);
            v
        })
    }

    /// award (zenko22 + ZenkoLife): a new ema badge, announced 2.5 s later
    pub fn award(&mut self, now: f64, bid: &str) -> bool {
        let d = rdata();
        let Some(b) = arr(&d["BADGES"]).iter().find(|b| b[0] == bid).cloned() else {
            return false;
        };
        let tiered = arr(&d["BADGE_TIERED"]).iter().any(|t| t[0] == bid);
        let won = self.life_do(|l| {
            if tiered && l["btier"][bid].as_i64().unwrap_or(0) < 1 {
                l["btier"][bid] = json!(1);
            }
            if l["badges"].get(bid).is_some() {
                return false;
            }
            l["badges"][bid] = json!(today_iso());
            true
        });
        if !won {
            return false;
        }
        let s = |i: usize| b[i].as_str().unwrap_or("").to_string();
        let line = format!("New ema badge! \u{3010}{}\u{3011} {} - {}. Sugoi~!", s(1), s(2), s(3));
        self.later(now + 2.5, Later::Say { text: line, secs: 10.0, choices: vec![], only_idle: false, beep: Some("gift".into()) });
        self.set_mood(now, "happy", 5.0);
        true
    }

    /// check_badges: streak and total-hours badges
    pub fn check_badges(&mut self, now: f64) {
        let s = streak();
        for (n, b) in [(3, "streak3"), (7, "streak7"), (30, "streak30")] {
            if s >= n {
                self.award(now, b);
            }
        }
        let h = total_hours();
        for (n, b) in [(10.0, "hours10"), (50.0, "hours50"), (100.0, "hours100")] {
            if h >= n {
                self.award(now, b);
            }
        }
    }

    /// life_session_done (ZenkoLife, then zenko22): called from end_session.
    pub fn life_session_done(&mut self, now: f64, s: &Session, completed: bool, focused: f64) {
        let fm = focused / 60.0;
        self.feed_tummy(if completed { (fm * 1.5).min(60.0) } else { fm * 0.5 });
        let wall = unix_now();
        let started_local = wall - s.elapsed + crate::local_offset_pub() as f64;
        let hour_of = |t: f64| (t.rem_euclid(86400.0) / 3600.0) as i64;
        if completed && s.elapsed >= 60.0 {
            self.award(now, "first");
            if s.planned >= 90.0 * 60.0 {
                self.award(now, "marathon");
            }
            if s.planned >= 25.0 * 60.0 && s.dist < 5.0 {
                self.award(now, "spotless");
            }
            let h = hour_of(started_local);
            if (4..8).contains(&h) {
                self.award(now, "early");
            }
        }
        self.check_badges(now);
        if completed && self.cfg.b("pet_needs", true) && self.rnd() < 0.5 {
            let lines = ["*munch munch* Yatta~ that session fed me an inari sushi! Thank you~kon!",
                         "Ehehe~ my foxfire got brighter! Focus is my favourite food~"];
            let l = lines[(self.rnd() * 2.0) as usize % 2].to_string();
            self.later(now + 16.0, Later::Say { text: l, secs: 7.0, choices: vec![], only_idle: true, beep: None });
        }
        // zenko22
        if completed && s.elapsed >= 60.0 {
            self.stat_add("sessions", 1);
            let h = hour_of(started_local);
            if (4..7).contains(&h) && self.stat_add("dawn", 1) >= 5 {
                self.award(now, "s_dawn");
            }
            let mut k = 0.0;
            while k <= s.elapsed.floor() {
                if hour_of(started_local + k) == 3 {
                    self.award(now, "s_3am");
                    break;
                }
                k += 600.0;
            }
            self.day_secrets(now);
        }
        self.z22_badge_check(now);
    }

    /// z22_day_secrets
    fn day_secrets(&mut self, now: f64) {
        let (_, m, d, h, _, _) = crate::local_now();
        if (m, d) == (1, 1) {
            self.award(now, "s_newyear");
        }
        if crate::local_weekday() == 4 && d == 13 {
            self.award(now, "s_fri13");
        }
        let local = unix_now() + crate::local_offset_pub() as f64;
        let age = moon_age(local);
        if (13.8..=15.8).contains(&age) && (h >= 18 || h < 5) {
            self.award(now, "s_fullmoon");
        }
        let bd = read_json("zenko_life.json")["facts"]["birthday"].as_str().map(String::from);
        if bd.is_some_and(|b| b == format!("{m:02}-{d:02}")) {
            self.award(now, "s_birthday");
        }
    }

    /// garden_add (zenko22 bait, journey lock minutes, the garden tree)
    pub fn garden_add(&mut self, now: f64, minutes: f64, kind: &str) {
        // journey minutes from lock-ins (zenko15)
        if kind == "lockin" && minutes >= 3.0 {
            self.life_do(|l| {
                let v = l["journey"]["lock_min"].as_f64().unwrap_or(0.0) + minutes;
                l["journey"]["lock_min"] = json!(py_round1(v));
            });
        }
        // _garden_add_core
        if minutes >= 3.0 {
            let (h, m) = {
                let (_, _, _, h, m, _) = crate::local_now();
                (h, m)
            };
            let (before, after) = self.life_do(|l| {
                let g = &mut l["garden"];
                let before = garden_stage(g["xp"].as_f64().unwrap_or(0.0));
                let hl = garden_health(g);
                let xp = py_round1(g["xp"].as_f64().unwrap_or(0.0) + minutes);
                g["xp"] = json!(xp);
                g["health"] = json!((hl + if kind == "lockin" { 0.5 } else { 0.35 }).min(1.0));
                let today = today_iso();
                let act = g["active"].as_array_mut().unwrap();
                if act.last().and_then(|v| v.as_str()) != Some(today.as_str()) {
                    act.push(json!(today));
                    let n = act.len();
                    if n > 120 {
                        act.drain(..n - 120);
                    }
                }
                let log = g["log"].as_array_mut().unwrap();
                log.push(json!({"d": today, "t": format!("{h:02}:{m:02}"), "m": minutes.round_ties_even() as i64, "k": kind}));
                let n = log.len();
                if n > 60 {
                    log.drain(..n - 60);
                }
                (before, garden_stage(xp))
            });
            if after > before {
                let st = &arr(&rdata()["GARDEN_STAGES"])[after];
                let text = format!("Kon! Your garden tree grew: {} ({}) {}~ Come see it in the Garden tab!",
                                   st[1].as_str().unwrap_or(""), st[3].as_str().unwrap_or(""), "*sparkle*");
                self.later(now + 9.0, Later::Say { text, secs: 10.0, choices: vec![("Show me".into(), "hub:garden".into())],
                                                   only_idle: false, beep: None });
            }
        }
        self.journey_check(now);
        // zenko22: one koi bait per 10 minutes, a golden one for a 25-minute lock-in
        self.life_do(|l| {
            let f = &mut l["fishing"];
            f["bait"] = json!((f["bait"].as_f64().unwrap_or(0.0) + minutes.max(0.0) / 10.0).min(99.0));
            if kind == "lockin" && minutes >= 25.0 {
                f["gold"] = json!((f["gold"].as_i64().unwrap_or(0) + 1).min(20));
            }
        });
    }

    /// journey_check (zenko22): newly reached places, announced 4 s later
    pub fn journey_check(&mut self, now: f64) {
        let new: Vec<String> = self.life_do(|l| {
            let (_, got) = journey_state(l);
            let seen: Vec<String> = arr(&l["journey"]["seen"]).iter().filter_map(|v| v.as_str().map(String::from)).collect();
            let new: Vec<String> = got.into_iter().filter(|p| !seen.contains(p)).collect();
            if let Some(s) = l["journey"]["seen"].as_array_mut() {
                for p in &new {
                    s.push(json!(p));
                }
            }
            new
        });
        let Some(last) = new.last() else { return };
        let d = rdata();
        let p = arr(&d["JOURNEY_ALL"]).iter().find(|p| p[0] == last.as_str()).cloned().unwrap_or(json!([]));
        let rew: Vec<String> = arr(&d["MAP_REWARDS"])
            .iter()
            .filter(|r| new.iter().any(|n| r[0] == n.as_str()))
            .filter_map(|r| r[3].as_str().map(String::from))
            .collect();
        let extra = if rew.is_empty() { String::new() } else { format!("\nIt unlocked: {}~", rew.join(", ")) };
        let s = |i: usize| p.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let text = format!("Kon! Our journey reached {} {}~ {}{}", s(1), s(2), s(4), extra);
        self.later(now + 4.0, Later::Say { text, secs: 12.0, choices: vec![("See the map".into(), "hub:map".into())],
                                           only_idle: false, beep: None });
        self.later(now + 5.0, Later::BadgeCheck);
    }

    /// z22_badge_check: tiered badges from the counters, journey and study badges
    pub fn z22_badge_check(&mut self, now: f64) {
        if now - self.rewards.metrics_t < 5.0 {
            return;
        }
        self.rewards.metrics_t = now;
        let l = {
            let mut seq: Vec<f64> = (0..4).map(|_| self.rnd()).collect();
            let mut r = || seq.pop().unwrap_or(0.5);
            update_life(&mut r, |l| l.clone())
        };
        let m = metrics(&l);
        let d = rdata();
        let names = arr(&d["BADGE_TIER_NAME"]);
        for t in arr(&d["BADGE_TIERED"]) {
            let (bid, key) = (t[0].as_str().unwrap_or(""), t[3].as_str().unwrap_or(""));
            let v = m.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0);
            let th: Vec<f64> = arr(&t[4]).iter().filter_map(|x| x.as_f64()).collect();
            let tier = th.iter().filter(|&&x| v >= x).count() as i64;
            let old = l["btier"][bid].as_i64().unwrap_or(0);
            if tier > old {
                self.life_do(|l| l["btier"][bid] = json!(tier));
                if old == 0 {
                    self.award(now, bid);
                } else {
                    let thv = &arr(&t[4])[(tier - 1) as usize];
                    let how = t[5].as_str().unwrap_or("").replace("{n}", &thv.to_string());
                    let line = format!("Your ema \u{3010}{}\u{3011} {} turned {}! {} \u{2713}", t[1].as_str().unwrap_or(""),
                                       t[2].as_str().unwrap_or(""), names[tier as usize].as_str().unwrap_or(""), how);
                    self.later(now + 2.5, Later::Say { text: line, secs: 9.0, choices: vec![], only_idle: false,
                                                       beep: Some("gift".into()) });
                }
            }
        }
        let (_, got) = journey_state(&l);
        let has = |id: &str| got.iter().any(|g| g == id);
        if arr(&d["JMAP_PLACES"]).iter().all(|p| has(p.as_str().unwrap_or(""))) {
            self.award(now, "japan");
        }
        let region = |r: &str| -> Vec<String> {
            arr(&d["JOURNEY_ALL"])
                .iter()
                .filter_map(|p| p[0].as_str())
                .filter(|id| d["JOURNEY_REGION_OF"][*id] == r)
                .map(String::from)
                .collect()
        };
        if region("kyoto").iter().all(|i| has(i)) {
            self.award(now, "kyoto4");
        }
        for (r, b) in [("korea", "korea"), ("china", "china")] {
            if region(r).iter().any(|i| has(i)) {
                self.award(now, b);
            }
        }
        for (id, b) in [("bukhara", "silk"), ("guanghan", "moon"), ("kyubi", "s_kyubi")] {
            if has(id) {
                self.award(now, b);
            }
        }
        let quiz: Vec<String> = arr(&l["journey"]["quiz"]).iter().filter_map(|v| v.as_str().map(String::from)).collect();
        if arr(&d["JOURNEY_REGIONS"]).iter().any(|r| {
            let lst = region(r.as_str().unwrap_or(""));
            !lst.is_empty() && lst.iter().all(|i| quiz.contains(i))
        }) {
            self.award(now, "passport");
        }
        let st = stats();
        let today = local_days();
        let sec = |dd: i64| stat_secs(&st, &iso(dd));
        if sec(today) >= 6.0 * 3600.0 {
            self.award(now, "s_iron");
        }
        for back in 0..14 {
            let dd = today - back;
            // weekday() == 6: Sunday
            if (dd + 3).rem_euclid(7) == 6 && sec(dd) >= 600.0 && sec(dd - 1) >= 600.0 {
                self.award(now, "weekend");
                break;
            }
        }
        if sec(today) >= 600.0 {
            let gap = (1..8).all(|k| sec(today - k) < 600.0);
            let cut = iso(today - 7);
            let before = st.as_object().is_some_and(|o| {
                o.iter().any(|(k, v)| {
                    v.as_f64().unwrap_or(0.0) >= 600.0 && k.as_str() < cut.as_str() && k.len() >= 5
                        && k.as_bytes()[..4].iter().all(|c| c.is_ascii_digit()) && k.as_bytes()[4] == b'-'
                })
            });
            if gap && before {
                self.award(now, "s_comeback");
            }
        }
        let mut seen: Vec<String> = vec![];
        for k in 0..7 {
            if let Some(o) = l["slog"][iso(today - k).as_str()].as_object() {
                for (sid, secs) in o {
                    if secs.as_f64().unwrap_or(0.0) >= 300.0 && !seen.contains(sid) {
                        seen.push(sid.clone());
                    }
                }
            }
        }
        if arr(&d["SUBJECT_IDS"]).iter().all(|i| seen.iter().any(|s| i == s.as_str())) {
            self.award(now, "allsix");
        }
    }

    /// What end_session adds on top (FoxBuddy.end_session's life_session_done, then the zenko14 garden).
    pub fn session_rewards(&mut self, now: f64, s: &Session, completed: bool, focused: f64) {
        self.life_session_done(now, s, completed, focused);
        if self.lockin.is_none() && s.task != "Lock-in" {
            let fm = focused / 60.0;
            if completed || fm >= 10.0 {
                self.garden_add(now, fm * 0.6, "focus");
            }
        }
    }

    /// What lockin_stop adds on top: the lockin badge, garden minutes, the zenko22 counters.
    pub fn lockin_rewards(&mut self, now: f64, t0: f64, done: bool) {
        if done {
            self.award(now, "lockin");
        }
        let mins = (unix_now() - t0) / 60.0;
        if done || mins >= 5.0 {
            self.garden_add(now, if done { mins } else { mins * 0.8 }, "lockin");
        }
        if done {
            self.stat_add("lockins", 1);
            self.day_secrets(now);
            self.z22_badge_check(now);
        }
    }
}
