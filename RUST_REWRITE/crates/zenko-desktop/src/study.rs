//! zenko8's study smarts (zenko.py ZenkoStudy): study_tick -> study_classify / study_context_subject / study_commit /
//! study_flush / study_tidy, the "studying Bio?" ask (study_maybe_ask / study_ask / study_sort), exam credit
//! (exam_for / planned_now / credit_minutes / log_study), the reports (subject_minutes, best_hours, best_time_text,
//! distraction_report, week_report, recap_text, weak_subject), the 30 s study_slow (study_missed, the recap, the
//! weak-subject alert, the golden hour), conjugate_fr and study_route.
//!
//! zenko.py keeps life in memory and saves it once a minute (study_flush). Here the per-second commits wait in a
//! queue that read_json("zenko_life.json") applies, and the next save writes them, so the file is written as often.

use crate::plan::{PLAN_KINDS, day_n, deadline_due, iso, load_deadlines, naive, num, parse_iso, plan_hours_default,
                  start_min, truthy};
use crate::rewards::Later;
use crate::routes17::routes_data;
use crate::session::clip;
use crate::ema::fmt_hm;
use crate::tt::{save_tt, tt_done, tt_today};
use crate::watch::classify_subject;
use crate::{Companion, Ev, read_json, unix_now};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::cell::RefCell;
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

/// what study_classify decided this second: ("s", subject), ("d", distraction site), ("u", window key, guess)
#[derive(Clone, Debug, PartialEq)]
pub enum What {
    S(String),
    D(String),
    U(String, String),
}

impl What {
    pub fn to_json(&self) -> Value {
        match self {
            What::S(s) => json!(["s", s]),
            What::D(s) => json!(["d", s]),
            What::U(k, g) => json!(["u", k, g]),
        }
    }
}

/// self.st
#[derive(Clone, Debug, Default)]
pub struct Study {
    pub buf: Vec<(What, i64)>,
    pub credit: Vec<(String, i64)>,
    pub asked: Vec<(String, f64)>,
    /// ("I'm studying bio"): (subject, until, unix time)
    pub over: Option<(String, f64)>,
    pub last_flush: f64,
    pub title: String,
    pub now: Option<What>,
    pub tidy: String,
    /// _last_fg: (title, proc, idle seconds, distraction site)
    pub fg: Option<(String, String, f64, Option<String>)>,
    /// study_slow_tick: 20 s after start, then every 30 s
    pub slow_next: f64,
}

impl Study {
    pub fn new(now: f64) -> Study {
        Study { last_flush: now, slow_next: now + 20.0, ..Default::default() }
    }

    fn asked_at(&self, k: &str) -> f64 {
        self.asked.iter().find(|(a, _)| a == k).map(|x| x.1).unwrap_or(-9999.0)
    }

    fn set_asked(&mut self, k: &str, t: f64) {
        match self.asked.iter_mut().find(|(a, _)| a == k) {
            Some(x) => x.1 = t,
            None => self.asked.push((k.into(), t)),
        }
    }

    pub fn credit_add(&mut self, sid: &str, n: i64) {
        match self.credit.iter_mut().find(|(a, _)| a == sid) {
            Some(x) => x.1 += n,
            None => self.credit.push((sid.into(), n)),
        }
    }
}

// ---- the commit queue ----------------------------------------------------------------------------------------------

struct Commit {
    today: String,
    hour: u32,
    buf: Vec<(What, i64)>,
    title: String,
}

thread_local! {
    static PENDING: RefCell<Vec<Commit>> = const { RefCell::new(Vec::new()) };
}

/// read_json("zenko_life.json") applies the commits not saved yet (zenko.py's in-memory life)
pub(crate) fn apply_pending(l: &mut Value) {
    PENDING.with(|p| {
        let p = p.borrow();
        if p.is_empty() {
            return;
        }
        if !l.is_object() {
            *l = json!({});
        }
        for c in p.iter() {
            apply_commit(l, c);
        }
    });
}

/// save_life wrote them
pub(crate) fn clear_pending() {
    PENDING.with(|p| p.borrow_mut().clear());
}

fn obj<'a>(v: &'a mut Value, k: &str) -> &'a mut Value {
    if !v.get(k).is_some_and(|x| x.is_object()) {
        v[k] = json!({});
    }
    &mut v[k]
}

/// a + n, an int while both are ints (Python's int + int)
fn add_num(a: Option<&Value>, n: f64) -> Value {
    match a {
        Some(Value::Number(x)) if x.is_i64() && n.fract() == 0.0 => json!(x.as_i64().unwrap() + n as i64),
        Some(v) => json!(num(v, 0.0) + n),
        None if n.fract() == 0.0 => json!(n as i64),
        None => json!(n),
    }
}

fn bump(m: &mut Value, k: &str, n: f64) {
    let v = add_num(m.get(k), n);
    m[k] = v;
}

fn apply_commit(l: &mut Value, c: &Commit) {
    let hk = c.hour.to_string();
    {
        let hd = obj(obj(l, "hourlog"), &c.today);
        if !hd.get(&hk).is_some_and(|x| x.is_array()) {
            hd[hk.as_str()] = json!([0, 0]);
        }
    }
    for (k, n) in &c.buf {
        let n = *n as f64;
        let slot = match k {
            What::S(sid) => {
                bump(obj(obj(l, "slog"), &c.today), sid, n);
                0
            }
            What::D(site) => {
                bump(obj(obj(l, "dlog"), &c.today), site, n);
                1
            }
            What::U(key, guess) => {
                let un = obj(l, "unsorted");
                if un.get(key).is_none() {
                    un[key.as_str()] = json!({"title": c.title.chars().take(90).collect::<String>(), "secs": 0,
                                              "date": c.today, "guess": guess});
                }
                let u = &mut un[key.as_str()];
                if u["date"].as_str() != Some(c.today.as_str()) {
                    u["secs"] = json!(0);
                    u["date"] = json!(c.today);
                }
                bump(u, "secs", n);
                if !guess.is_empty() {
                    u["guess"] = json!(guess);
                }
                0
            }
        };
        let hl = &mut l["hourlog"][c.today.as_str()][hk.as_str()];
        let v = add_num(hl.get(slot), n);
        hl[slot] = v;
    }
}

// ---- words and names -------------------------------------------------------------------------------------------------

fn strs(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn subject_ids() -> Vec<String> {
    crate::watch::data().subjects.iter().map(|s| s.sid.clone()).collect()
}

/// subject_name(sid, short=True)
pub fn sname(sid: &str) -> String {
    crate::watch::data().subjects.iter().find(|s| s.sid == sid).map(|s| s.short.clone())
        .unwrap_or_else(|| if sid.is_empty() { "?".into() } else { sid.into() })
}

/// clean_title(title): the window title without the browser / app name tail and "(3) " counters
pub fn clean_title(title: &str) -> String {
    let d = crate::watch::data();
    let mut t = title.trim().to_string();
    for _ in 0..2 {
        t = d.title_tails.replace(&t, "").to_string();
    }
    rx!(r"^\(\d+\)\s*").replace(&t, "").trim().to_string()
}

pub fn looks_like_study(title: &str, proc_: &str) -> bool {
    let low = title.to_lowercase();
    let p = proc_.to_lowercase();
    strs(&routes_data()["STUDY_APPS"]).contains(&p) || strs(&routes_data()["STUDY_HINTS"]).iter().any(|h| low.contains(h.as_str()))
}

/// title_words(title): distinctive words in a window title
pub fn title_words(title: &str) -> Vec<String> {
    let stop = strs(&routes_data()["SUBJ_STOP"]);
    let mut out: Vec<String> = vec![];
    let ct = clean_title(title).to_lowercase();
    for m in rx!(r"[a-zA-Zàâçéèêëîïôûùüÿœ][\w'\-]{3,}").find_iter(&ct) {
        let w = m.as_str().to_string();
        let digit = !w.is_empty() && w.chars().all(|c| c.is_numeric());
        if !stop.contains(&w) && !digit && !out.contains(&w) {
            out.push(w);
        }
    }
    out.truncate(4);
    out
}

/// when_text(date_iso, today)
pub fn when_text(date_iso: &str, today: i64) -> String {
    let Some(d) = parse_iso(date_iso) else { return String::new() };
    let n = d - today;
    match n {
        0 => "today".into(),
        1 => "tomorrow".into(),
        -1 => "yesterday".into(),
        n if n < 0 => format!("{} days ago", -n),
        n if n < 14 => format!("in {n} days"),
        n if n < 60 => format!("in {} weeks", n / 7),
        n => format!("in {} months", (n as f64 / 30.4).round_ties_even() as i64),
    }
}

/// day.strftime("%A %d %B")
fn long_date(day: i64) -> String {
    const WD: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    const MO: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September",
                            "October", "November", "December"];
    let (_, m, d) = crate::civil_from_days_pub(day);
    format!("{} {d:02} {}", WD[weekday(day) as usize], MO[m as usize - 1])
}

fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// int(x) of a JSON number (or numeric string)
fn int_of(v: &Value, d: i64) -> i64 {
    match v {
        Value::Null => d,
        v => num(v, d as f64).trunc() as i64,
    }
}

fn sum_vals(v: &Value) -> f64 {
    v.as_object().map(|o| o.values().map(|x| num(x, 0.0)).sum()).unwrap_or(0.0)
}

// ---- conjugate_fr ------------------------------------------------------------------------------------------------------

fn vowel_start(w: &str) -> bool {
    w.chars().next().is_none_or(|c| "aeiouéèêh".contains(c))
}

/// conjugate_fr(verb): présent, passé composé, imparfait and futur of a French verb (regular + common irregulars)
pub fn conjugate_fr(verb: &str) -> Option<String> {
    let d = routes_data();
    let mut v = verb.trim().to_lowercase();
    let reflexive = v.starts_with("se ") || v.starts_with("s'");
    v = rx!(r"^(?:se |s')").replace(&v, "").to_string();
    let (pres, pp, fut, imp, mut aux): (Vec<String>, String, String, String, String);
    if let Some(ir) = d["FR_IRREG"].get(&v) {
        pres = strs(&ir[0]);
        pp = ir[1].as_str().unwrap().into();
        fut = ir[2].as_str().unwrap().into();
        imp = ir[3].as_str().unwrap().into();
        aux = ir[4].as_str().unwrap().into();
    } else if v.ends_with("er") {
        let st = v[..v.len() - 2].to_string();
        let g = if st.ends_with('g') { "e" } else { "" };
        let st_n = if st.ends_with('c') { format!("{}ç", &st[..st.len() - 1]) } else { st.clone() };
        let (mut strong, mut fu) = (st.clone(), v.clone());
        let m = rx!(r"^(.*)([eé])([^aeiouéèêy]+)$").captures(&st);
        if st.ends_with('y') && !st.ends_with("ay") {
            strong = format!("{}i", &st[..st.len() - 1]);
            fu = format!("{strong}er");
        } else if ["appeler", "rappeler", "jeter", "rejeter", "épeler"].contains(&v.as_str()) {
            strong = format!("{st}{}", st.chars().last().unwrap());
            fu = format!("{strong}er");
        } else if let Some(m) = m.filter(|m| m[3].chars().count() <= 2) {
            strong = format!("{}è{}", &m[1], &m[3]);
            if &m[2] == "e" {
                fu = format!("{strong}er");
            }
        }
        pres = vec![format!("{strong}e"), format!("{strong}es"), format!("{strong}e"), format!("{st_n}{g}ons"),
                    format!("{st}ez"), format!("{strong}ent")];
        pp = format!("{st}é");
        imp = format!("{st_n}{g}");
        fut = fu;
        aux = "avoir".into();
    } else if v.ends_with("ir") {
        let st = &v[..v.len() - 2];
        pres = ["is", "is", "it", "issons", "issez", "issent"].iter().map(|e| format!("{st}{e}")).collect();
        pp = format!("{st}i");
        fut = v.clone();
        imp = format!("{st}iss");
        aux = "avoir".into();
    } else if v.ends_with("re") {
        let st = &v[..v.len() - 2];
        pres = ["s", "s", "", "ons", "ez", "ent"].iter().map(|e| format!("{st}{e}")).collect();
        pp = format!("{st}u");
        fut = v[..v.len() - 1].to_string();
        imp = st.to_string();
        aux = "avoir".into();
    } else {
        return None;
    }
    if strs(&d["FR_ETRE_VERBS"]).contains(&v) || reflexive {
        aux = "être".into();
    }
    let aux_f = strs(&d["FR_IRREG"][aux.as_str()][0]);
    let pron = strs(&d["FR_PRONOUNS"]);
    let ends_i = ["ais", "ais", "ait", "ions", "iez", "aient"];
    let ends_f = ["ai", "as", "a", "ons", "ez", "ont"];
    let refl = ["me", "te", "se", "nous", "vous", "se"];
    let subj = |i: usize, word: String| -> String {
        let mut word = word;
        if reflexive {
            let r = refl[i];
            word = if ["me", "te", "se"].contains(&r) && vowel_start(&word) {
                format!("{}'{word}", &r[..1])
            } else {
                format!("{r} {word}")
            };
        }
        if i == 0 && vowel_start(&word) {
            return format!("j'{word}");
        }
        format!("{} {word}", pron[i])
    };
    let etre = aux == "être";
    let agree: [&str; 6] = if etre { ["", "", "(e)", "(e)s", "(e)(s)", "(e)s"] } else { [""; 6] };
    let row = |f: &dyn Fn(usize) -> String| (0..6).map(|i| subj(i, f(i))).collect::<Vec<_>>().join(", ");
    let mut lines = vec![format!("{}{v} ({})", if reflexive { "se " } else { "" },
                                 if etre { "conjugated with être" } else { "conjugated with avoir" })];
    lines.push(format!("Présent: {}", row(&|i| pres[i].clone())));
    lines.push(format!("Passé composé: {}", row(&|i| format!("{} {pp}{}", aux_f[i], agree[i]))));
    lines.push(format!("Imparfait: {}", row(&|i| format!("{imp}{}", ends_i[i]))));
    lines.push(format!("Futur simple: {}", row(&|i| format!("{fut}{}", ends_f[i]))));
    lines.push(format!("Futur proche: je vais {}, nous allons {}", if reflexive { format!("me {v}") } else { v.clone() },
                       if reflexive { format!("nous {v}") } else { v.clone() }));
    Some(lines.join("\n"))
}

fn word_num(n: &str) -> f64 {
    if n.starts_with("half") {
        return 0.5;
    }
    match n {
        "a" | "an" | "one" => 1.0,
        "two" => 2.0,
        "three" => 3.0,
        "four" => 4.0,
        "five" => 5.0,
        n => n.parse().unwrap_or(0.0),
    }
}

// ---- the fox's side ----------------------------------------------------------------------------------------------------

impl Companion {
    pub(crate) fn life_now(&self) -> Value {
        read_json("zenko_life.json")
    }

    fn nl(&self) -> f64 {
        naive(unix_now())
    }

    /// tt_window(it, day) as naive seconds
    fn tt_win(it: &Value, day: i64) -> Option<(f64, f64)> {
        let st = (day * 86400 + start_min(it)? * 60) as f64;
        Some((st, st + num(&it["mins"], 0.0) * 60.0))
    }

    /// once a second (zenko.py tick -> study_tick, after _tick set _last_fg)
    pub fn study_tick(&mut self, now: f64) {
        let Some((title, proc_, idle, site)) = self.study.fg.clone() else { return };
        let tim = crate::plan::cfg_int(&self.cfg, "track_idle_minutes", 6);
        let limit = 60.max(tim * 60) as f64;
        if !self.cfg.b("auto_track", true) {
            return;
        }
        if self.brk.is_some() || idle >= limit {
            self.study.buf.clear(); // you stepped away: the last few minutes don't count (honest hours)
            self.study.now = None;
            return;
        }
        let what = self.study_classify(&title, &proc_, site.as_deref());
        self.study.now = what.clone();
        if let Some(w) = what {
            match self.study.buf.iter_mut().find(|(k, _)| *k == w) {
                Some(x) => x.1 += 1,
                None => self.study.buf.push((w, 1)),
            }
        }
        if idle < 3.0 && !self.study.buf.is_empty() {
            let buf = std::mem::take(&mut self.study.buf);
            self.study_commit(now, buf);
        }
        if now - self.study.last_flush >= 60.0 {
            self.study.last_flush = now;
            self.study_flush(now);
        }
    }

    /// study_context_subject: "I'm studying bio", your focus session, or the planned session now
    fn study_context_subject(&self, learned: &Value) -> Option<String> {
        if let Some((s, until)) = &self.study.over {
            if *until > unix_now() {
                return Some(s.clone());
            }
        }
        if let Some(s) = &self.session {
            let (sid, sc, _) = classify_subject(&s.task, learned);
            if sid.is_some() && sc >= 1 {
                return sid;
            }
        }
        let nl = self.nl();
        let today = day_n(nl);
        for i in tt_today(&self.tt, &iso(today)) {
            let it = &self.tt[i];
            let task = it["task"].as_str().unwrap_or("");
            if !truthy(&it["plan"]) && !rx!(r"(?i)\b(stud|revis|homework|hw)\w*").is_match(task) {
                continue;
            }
            let Some((st, en)) = Self::tt_win(it, today) else { continue };
            if st - 300.0 <= nl && nl <= en {
                let (sid, _, _) = classify_subject(task, learned);
                if sid.is_some() {
                    return sid;
                }
            }
        }
        None
    }

    fn study_classify(&mut self, title: &str, proc_: &str, site: Option<&str>) -> Option<What> {
        if title.is_empty() && proc_.is_empty() {
            return None;
        }
        let low = title.to_lowercase();
        let pl = proc_.to_lowercase();
        if proc_ == "zenko.exe" || pl == "zenkorust.exe" || low.starts_with("zenko") || (proc_.starts_with("python") && low.contains("zenko")) {
            // her own Notebook page counts as studying that subject
            let nb = self.life_now()["nb_last"].as_str().unwrap_or("overview").to_string();
            if self.hub_page.as_deref() == Some("notebook") && subject_ids().contains(&nb) {
                return Some(What::S(nb));
            }
            return None;
        }
        let ct = clean_title(title);
        self.study.title = ct.clone();
        if let Some((s, until)) = &self.study.over {
            if *until > unix_now() && site.is_none() {
                return Some(What::S(s.clone()));
            }
        }
        let key = crate::watch::window_key(&ct);
        let life = self.life_now();
        match life["win_subj"].get(&key) {
            Some(Value::String(ws)) if ws == "-" => return site.map(|s| What::D(s.into())),
            Some(ws) if truthy(ws) => return Some(What::S(ws.as_str().map(String::from).unwrap_or_else(|| ws.to_string()))),
            _ => {}
        }
        let learned = &life["subj_words"];
        let (sid, sc, sc2) = classify_subject(&ct, learned);
        if let Some(s) = &sid {
            if sc >= 3 && sc - sc2 >= 2 {
                return Some(What::S(s.clone()));
            }
        }
        if let Some(s) = site {
            return Some(What::D(s.into()));
        }
        let ctx = self.study_context_subject(learned);
        let studyish = looks_like_study(&ct, proc_) || self.session.is_some();
        if let Some(c) = &ctx {
            if studyish || sid.as_ref() == Some(c) {
                return Some(What::S(c.clone()));
            }
        }
        if studyish && !key.is_empty() {
            return Some(What::U(key, sid.or(ctx).unwrap_or_default()));
        }
        None
    }

    fn study_commit(&mut self, now: f64, buf: Vec<(What, i64)>) {
        let nl = self.nl();
        let c = Commit { today: iso(day_n(nl)), hour: (nl.rem_euclid(86400.0) / 3600.0) as u32, buf: buf.clone(),
                         title: self.study.title.clone() };
        PENDING.with(|p| p.borrow_mut().push(c));
        for (k, n) in buf {
            match k {
                What::S(sid) => self.study.credit_add(&sid, n),
                What::U(key, _) => self.study_maybe_ask(now, &key),
                What::D(_) => {}
            }
        }
    }

    fn study_flush(&mut self, now: f64) {
        for (sid, secs) in self.study.credit.clone() {
            if secs >= 60 {
                let m = secs / 60;
                if let Some(x) = self.study.credit.iter_mut().find(|(a, _)| *a == sid) {
                    x.1 = secs - m * 60;
                }
                self.credit_minutes(now, &sid, m);
            }
        }
        // study_tidy (once a day) + life_save
        let today = day_n(self.nl());
        let tidy = self.study.tidy != iso(today);
        self.study.tidy = iso(today);
        self.life_do(|l| {
            if !tidy {
                return;
            }
            for (k, keep) in [("slog", 60), ("dlog", 60), ("hourlog", 35)] {
                let cut = iso(today - keep);
                if let Some(o) = l.get_mut(k).and_then(|v| v.as_object_mut()) {
                    o.retain(|d, _| d.as_str() >= cut.as_str());
                }
            }
            let t = iso(today);
            if let Some(o) = l.get_mut("unsorted").and_then(|v| v.as_object_mut()) {
                o.retain(|_, v| v["date"].as_str() == Some(t.as_str()));
            }
        });
    }

    // ---- asking "studying Bio?" with one tap ---------------------------------------------------------------------------

    fn study_maybe_ask(&mut self, now: f64, key: &str) {
        let u = self.life_now()["unsorted"].get(key).cloned();
        let after = crate::plan::cfg_int(&self.cfg, "ask_after_seconds", 120);
        let Some(u) = u else { return };
        if (int_of(&u["secs"], 0) < after) || !self.cfg.b("ask_subject", true) {
            return;
        }
        if now - self.study.asked_at(key) < 600.0 || now - self.study.asked_at("*") < 90.0 {
            return;
        }
        if self.watch.alerted || self.current_mood(now) == "alert" || self.lock_eating || self.quiz_on() {
            return;
        }
        self.study.set_asked(key, now);
        self.study.set_asked("*", now);
        self.study_ask(key);
    }

    /// a quiz is running
    fn quiz_on(&self) -> bool {
        self.quiz.is_some()
    }

    pub fn study_ask(&mut self, key: &str) {
        let Some(u) = self.life_now()["unsorted"].get(key).cloned() else { return };
        let guess = u["guess"].as_str().unwrap_or("").to_string();
        let mut order: Vec<String> = if guess.is_empty() { vec![] } else { vec![guess.clone()] };
        for s in subject_ids() {
            if !order.contains(&s) {
                order.push(s);
            }
        }
        let mut choices: Vec<(String, String)> = order.iter()
            .map(|s| (format!("{}{}", if *s == guess { "✓ " } else { "" }, sname(s)), format!("study_sort:{s}:{key}")))
            .collect();
        choices.push(("Not study".into(), format!("study_sort::{key}")));
        let t = u["title"].as_str().unwrap_or("");
        let text = if !guess.is_empty() {
            format!("Kon? Studying {} right now? (\u{201c}{}\u{201d})", sname(&guess), clip(t, 34))
        } else {
            format!("Ne~ what are you studying here? (\u{201c}{}\u{201d}) Tap one and I'll remember~", clip(t, 34))
        };
        self.events.push(Ev::Say { text, secs: 45.0, choices });
    }

    pub fn study_sort(&mut self, now: f64, key: &str, sid: Option<&str>) {
        let today = iso(day_n(self.nl()));
        let aliases: Vec<String> = crate::watch::data().subjects.iter().flat_map(|s| s.aliases.clone()).collect();
        let k = key.to_string();
        let sid_s = sid.map(String::from);
        let secs_day = self.life_do(|l| {
            let u = obj(l, "unsorted").as_object_mut().unwrap().shift_remove(&k)
                .unwrap_or_else(|| json!({"title": "", "secs": 0, "date": today}));
            let ws = obj(l, "win_subj");
            ws[k.as_str()] = json!(sid_s.clone().unwrap_or_else(|| "-".into()));
            let o = ws.as_object_mut().unwrap();
            if o.len() > 600 {
                let first: Vec<String> = o.keys().take(100).cloned().collect();
                for f in first {
                    o.shift_remove(&f);
                }
            }
            let sid = sid_s.as_ref()?;
            let sw = obj(l, "subj_words");
            for w in title_words(u["title"].as_str().unwrap_or("")) {
                if w.chars().count() >= 5 && !aliases.contains(&w) {
                    sw[w.as_str()] = json!(sid);
                }
            }
            let o = sw.as_object_mut().unwrap();
            if o.len() > 400 {
                let first: Vec<String> = o.keys().take(80).cloned().collect();
                for f in first {
                    o.shift_remove(&f);
                }
            }
            let secs = int_of(&u["secs"], 0);
            let day = u["date"].as_str().filter(|d| !d.is_empty()).unwrap_or(today.as_str()).to_string();
            bump(obj(obj(l, "slog"), &day), sid, secs as f64);
            Some(secs)
        });
        let Some(sid) = sid else {
            self.say("Okay~ I won't count that window. Have fun~", 4.0);
            return;
        };
        let secs = secs_day.unwrap_or(0);
        self.study.credit_add(sid, secs);
        self.study_flush(now);
        let ex = self.exam_for(sid, false);
        self.set_mood(now, "happy", 3.0);
        let msg = format!("Yatta~ counted {} to {}{}. I'll remember this one!",
                          if secs >= 60 { fmt_hm(secs / 60) } else { "it".into() }, sname(sid),
                          ex.map(|e| format!(" (and toward \u{201c}{}\u{201d})", clip(e["title"].as_str().unwrap_or(""), 24)))
                              .unwrap_or_default());
        self.say(&msg, 7.0);
    }

    /// the study buttons (study_ask's subjects, the weak-subject / golden-hour session)
    pub fn study_button(&mut self, now: f64, cmd: &str) -> bool {
        if let Some(rest) = cmd.strip_prefix("study_sort:") {
            let (sid, key) = rest.split_once(':').unwrap_or((rest, ""));
            let sid = (!sid.is_empty()).then_some(sid);
            self.study_sort(now, key, sid);
            return true;
        }
        if let Some(sid) = cmd.strip_prefix("study_sess:") {
            let task = format!("{} revision", self.subject_full(sid));
            self.start_session(now, &task, 25.0);
            return true;
        }
        false
    }

    // ---- crediting time toward your exams ------------------------------------------------------------------------------

    /// exam_for(sid): the nearest upcoming exam/deadline this subject counts toward (a plain 'MYA' counts for all)
    pub fn exam_for(&mut self, sid: &str, specific_only: bool) -> Option<Value> {
        let life = self.life_now();
        let now = self.nl();
        let _ = self.plan_targets(&life, now); // makes sure every deadline has an id
        let mut cands: Vec<(i32, f64, Value)> = vec![];
        for d in load_deadlines() {
            let kind = match d.get("kind") {
                None => "Deadline",
                Some(Value::String(s)) => s.as_str(),
                Some(_) => "",
            };
            if truthy(&d["done"]) || !PLAN_KINDS.contains(&kind) || !truthy(&d["id"]) {
                continue;
            }
            let Some(due) = deadline_due(&d) else { continue };
            if due <= now || day_n(due) - day_n(now) > 60 {
                continue;
            }
            let (s2, sc, _) = classify_subject(d["title"].as_str().unwrap_or(""), &life["subj_words"]);
            if s2.as_deref() == Some(sid) && sc >= 1 {
                cands.push((0, due, d));
            } else if s2.is_none() && !specific_only {
                cands.push((1, due, d));
            }
        }
        cands.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.partial_cmp(&b.1).unwrap()));
        cands.into_iter().next().map(|c| c.2)
    }

    fn planned_now(&self, pid: &str) -> Option<usize> {
        let nl = self.nl();
        let today = day_n(nl);
        let ds = iso(today);
        tt_today(&self.tt, &ds).into_iter().find(|&i| {
            let it = &self.tt[i];
            it["plan"].as_str() == Some(pid) && !tt_done(it, &ds)
                && Self::tt_win(it, today).is_some_and(|(st, en)| st - 600.0 <= nl && nl <= en)
        })
    }

    /// credit_minutes: studied minutes go to that subject's exam; in a planned session they fill it, and at 70% she
    /// ticks it off herself
    pub fn credit_minutes(&mut self, now: f64, sid: &str, mins: i64) -> Option<Value> {
        let d = self.exam_for(sid, false)?;
        if mins <= 0 {
            return None;
        }
        let pid = d["id"].as_str().map(String::from).unwrap_or_else(|| d["id"].to_string());
        if let Some(i) = self.planned_now(&pid) {
            let a = int_of(&self.tt[i]["auto_s"], 0) + mins * 60;
            self.tt[i]["auto_s"] = json!(a);
            let need = 0.7 * int_of(&self.tt[i].get("mins").cloned().unwrap_or(json!(50)), 50) as f64 * 60.0;
            if a as f64 >= need {
                let task = self.tt[i]["task"].as_str().unwrap_or("").to_string();
                self.tt_mark(now, i, true);
                self.say(&format!("Kon! You studied {} through your planned session, so I ticked \u{201c}{}\u{201d} off for you~",
                                  sname(sid), clip(&task, 30)), 8.0);
                self.events.push(Ev::Beep { kind: "ding".into(), variety: false });
            } else {
                save_tt(&self.tt);
            }
            self.life_do(|_| ());
        } else {
            self.life_do(|l| bump(obj(l, "studied"), &pid, mins as f64));
        }
        Some(d)
    }

    /// log_study: "I studied bio for 2 hours"
    pub fn log_study(&mut self, now: f64, sid: &str, mins: f64, day: &str) -> Option<Value> {
        let s = sid.to_string();
        let dd = day.to_string();
        self.life_do(|l| bump(obj(obj(l, "slog"), &dd), &s, (mins * 60.0).trunc()));
        let d = self.credit_minutes(now, sid, mins.trunc() as i64);
        self.life_do(|_| ());
        d
    }

    // ---- numbers for reports -------------------------------------------------------------------------------------------

    /// subject_minutes(days, start): {subject: minutes}
    pub(crate) fn subject_minutes(&self, days: i64, start: Option<i64>) -> Vec<(String, f64)> {
        let today = day_n(self.nl());
        let first = start.unwrap_or(today - (days - 1));
        let mut out: Vec<(String, f64)> = vec![];
        if let Some(o) = self.life_now()["slog"].as_object() {
            for (d, v) in o {
                let Some(dn) = parse_iso(d) else { continue };
                if dn < first {
                    continue;
                }
                for (sid, secs) in v.as_object().into_iter().flatten() {
                    let m = num(secs, 0.0) / 60.0;
                    match out.iter_mut().find(|(a, _)| a == sid) {
                        Some(x) => x.1 += m,
                        None => out.push((sid.clone(), m)),
                    }
                }
            }
        }
        out
    }

    pub(crate) fn week_start(&self) -> i64 {
        let t = day_n(self.nl());
        t - weekday(t)
    }

    /// best_hours: [(hour, study minutes per active day, on-task fraction)] best first
    pub fn best_hours(&self) -> Vec<(i64, f64, f64)> {
        let mut agg: Vec<(i64, f64, f64)> = vec![];
        let mut days = 0;
        if let Some(o) = self.life_now()["hourlog"].as_object() {
            for hours in o.values() {
                days += 1;
                for (h, sx) in hours.as_object().into_iter().flatten() {
                    let h: i64 = h.trim().parse().unwrap_or(0);
                    let (s, x) = (num(&sx[0], 0.0), num(&sx[1], 0.0));
                    match agg.iter_mut().find(|a| a.0 == h) {
                        Some(a) => {
                            a.1 += s;
                            a.2 += x;
                        }
                        None => agg.push((h, s, x)),
                    }
                }
            }
        }
        let mut out: Vec<(i64, f64, f64)> = agg.into_iter().filter(|a| a.1 >= 1200.0)
            .map(|(h, s, x)| (h, s / 60.0 / (days.max(1) as f64), if s + x != 0.0 { s / (s + x) } else { 0.0 }))
            .collect();
        out.sort_by(|a, b| (-(a.1 * (0.5 + a.2))).partial_cmp(&-(b.1 * (0.5 + b.2))).unwrap());
        out
    }

    pub fn best_time_text(&self) -> String {
        let b = self.best_hours();
        if b.is_empty() {
            return "I'm still learning your focus rhythm~ Study a few days with me and I'll tell you your golden hours!".into();
        }
        let mut top: Vec<i64> = b.iter().take(2).map(|r| r.0).collect();
        top.sort();
        let span = if top.len() == 2 && top[1] - top[0] == 1 {
            format!("{:02}:00-{:02}:00", top[0], top[1] + 1)
        } else {
            top.iter().map(|h| format!("{h:02}:00")).collect::<Vec<_>>().join(" and ")
        };
        let n2 = b.len().min(2);
        let frac = b.iter().take(2).map(|r| r.2).sum::<f64>() / n2 as f64;
        let mut worst = &b[0];
        for r in &b {
            if r.2 < worst.2 {
                worst = r;
            }
        }
        let mut line = format!("Your golden focus time is {span} (on task {}% of the time)~", (frac * 100.0).round_ties_even() as i64);
        if b.len() >= 4 && worst.2 < 0.75 {
            line += &format!(" You drift the most around {:02}:00 ({}% on task).", worst.0, (worst.2 * 100.0).round_ties_even() as i64);
        }
        line
    }

    pub fn distraction_report(&self, days: i64) -> String {
        let today = day_n(self.nl());
        let (mut cur, mut prev): (Vec<(String, f64)>, Vec<(String, f64)>) = (vec![], vec![]);
        if let Some(o) = self.life_now()["dlog"].as_object() {
            for (d, v) in o {
                let Some(dn) = parse_iso(d) else { continue };
                let age = today - dn;
                let tgt = if age < days { &mut cur } else if age < 2 * days { &mut prev } else { continue };
                for (site, secs) in v.as_object().into_iter().flatten() {
                    let s = num(secs, 0.0);
                    match tgt.iter_mut().find(|(a, _)| a == site) {
                        Some(x) => x.1 += s,
                        None => tgt.push((site.clone(), s)),
                    }
                }
            }
        }
        if cur.is_empty() {
            return "Distraction report: nothing pulled you away this week~ Sugoi! (Or I haven't watched long enough yet.)".into();
        }
        let tot: f64 = cur.iter().map(|x| x.1).sum();
        let ptot: f64 = prev.iter().map(|x| x.1).sum();
        let fm = |s: f64| fmt_hm((s / 60.0).floor() as i64);
        let mut lines = vec![format!("Distraction report (last 7 days): {} in total{}", fm(tot),
            if ptot != 0.0 {
                format!(" ({} than the week before)", if ptot > tot { format!("{} less", fm(ptot - tot)) } else { format!("{} more", fm(tot - ptot)) })
            } else { String::new() })];
        let mut srt = cur.clone();
        srt.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (site, secs) in srt.iter().take(5) {
            lines.push(format!("• {site}: {}", if *secs >= 60.0 { fm(*secs) } else { "<1m".into() }));
        }
        let mut top = &cur[0];
        for c in &cur {
            if c.1 > top.1 {
                top = c;
            }
        }
        lines.push(format!("Tip: {} is your biggest time-eater. Try lock-in mode, or add it to 'Edit distracting sites'~", top.0));
        lines.join("\n")
    }

    pub fn week_report(&mut self) -> String {
        let mins = self.subject_minutes(7, Some(self.week_start()));
        let today = day_n(self.nl());
        let mut lines = vec!["This week (since Monday)~".to_string()];
        let mut any = false;
        let goals = self.life_now()["goals"].clone();
        for sid in subject_ids() {
            let m = mins.iter().find(|(a, _)| *a == sid).map(|x| x.1).unwrap_or(0.0);
            let goal = goals.get(&sid).map(|g| num(g, 0.0)).unwrap_or(0.0);
            let ex = self.exam_for(&sid, true);
            let mut bits = vec![];
            if goal != 0.0 {
                bits.push(format!("goal {}", fmt_hm((goal * 60.0).trunc() as i64)));
            }
            if let Some(ex) = ex {
                bits.push(format!("{} {}", clip(ex["title"].as_str().unwrap_or(""), 22), when_text(ex["date"].as_str().unwrap_or(""), today)));
            }
            if m >= 1.0 || !bits.is_empty() {
                any = true;
                lines.push(format!("• {}: {}{}", sname(&sid), fmt_hm(m.trunc() as i64),
                                   if bits.is_empty() { String::new() } else { format!("  ({})", bits.join(", ")) }));
            }
        }
        if !any {
            lines.push("No study tracked yet this week. Open your notes and I'll notice which subject it is~".into());
        }
        lines.join("\n")
    }

    /// exam_progress(d): (minutes studied, minutes needed)
    fn exam_progress(&self, d: &Value) -> (f64, f64) {
        let life = self.life_now();
        let id = d["id"].as_str().unwrap_or("");
        let hrs = life["plan_hours"].get(id).map(|v| num(v, 3.0))
            .unwrap_or_else(|| plan_hours_default(d["kind"].as_str().unwrap_or("")));
        (life["studied"].get(id).map(|v| num(v, 0.0)).unwrap_or(0.0), hrs * 60.0)
    }

    pub(crate) fn upcoming_deadlines(&self, n: usize) -> Vec<Value> {
        let today = iso(day_n(self.nl()));
        load_deadlines().into_iter()
            .filter(|d| !truthy(&d["done"]) && d["date"].as_str().unwrap_or("") >= today.as_str())
            .take(n).collect()
    }

    pub fn recap_text(&mut self, day: i64) -> String {
        let ds = iso(day);
        let real_today = day_n(self.nl());
        let l = self.life_now();
        let mut lines = vec![format!("Kon~ your recap for {}:", long_date(day))];
        let subj = l["slog"].get(&ds).and_then(|v| v.as_object()).cloned().unwrap_or_default();
        let tot: f64 = subj.values().map(|v| num(v, 0.0)).sum();
        if !subj.is_empty() {
            let mut srt: Vec<(String, f64)> = subj.iter().map(|(k, v)| (k.clone(), num(v, 0.0))).collect();
            srt.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            lines.push(format!("Studied {}: ", fmt_hm((tot / 60.0).floor() as i64))
                + &srt.iter().filter(|(_, v)| *v >= 60.0)
                    .map(|(s, v)| format!("{} {}", sname(s), fmt_hm((v / 60.0).floor() as i64))).collect::<Vec<_>>().join(", "));
        } else {
            lines.push("No subject time tracked today.".into());
        }
        let uns: f64 = l["unsorted"].as_object().into_iter().flatten()
            .filter(|(_, u)| u["date"].as_str() == Some(ds.as_str())).map(|(_, u)| num(&u["secs"], 0.0)).sum();
        if uns >= 300.0 {
            lines.push(format!("(+{} I wasn't sure about - sort it on the Notebook page)", fmt_hm((uns / 60.0).floor() as i64)));
        }
        let rows: Vec<usize> = tt_today(&self.tt, &ds).into_iter().filter(|&i| truthy(&self.tt[i]["plan"])).collect();
        if !rows.is_empty() {
            let done = rows.iter().filter(|&&i| tt_done(&self.tt[i], &ds)).count();
            let missed: Vec<usize> = rows.iter().copied().filter(|&i| truthy(&self.tt[i]["missed"]) && !tt_done(&self.tt[i], &ds)).collect();
            lines.push(format!("Planned sessions: {done} of {} done{}", rows.len(),
                if missed.is_empty() { String::new() } else {
                    format!("; missed: {} (I re-planned them)", missed.iter().take(3)
                        .map(|&i| clip(self.tt[i]["task"].as_str().unwrap_or(""), 26)).collect::<Vec<_>>().join(", "))
                }));
        }
        if let Some(o) = l["dlog"].get(&ds).and_then(|v| v.as_object()).filter(|o| !o.is_empty()) {
            let mut best: Option<(&String, f64)> = None;
            for (k, v) in o {
                let s = num(v, 0.0);
                if best.is_none_or(|b| s > b.1) {
                    best = Some((k, s));
                }
            }
            let (site, secs) = best.unwrap();
            if secs >= 300.0 {
                lines.push(format!("Biggest distraction: {site} ({})", fmt_hm((secs / 60.0).floor() as i64)));
            }
        }
        let nxt = tt_today(&self.tt, &iso(day + 1));
        if !nxt.is_empty() {
            lines.push(format!("Tomorrow: {}{}", nxt.iter().take(4).map(|&i| {
                let it = &self.tt[i];
                format!("{} {}", it["start"].as_str().unwrap_or(""), clip(it["task"].as_str().unwrap_or(""), 28))
            }).collect::<Vec<_>>().join("; "), if nxt.len() > 4 { format!(" ...+{} more", nxt.len() - 4) } else { String::new() }));
        }
        let ups = self.upcoming_deadlines(3);
        if let Some(d) = ups.first() {
            let (got, need) = if truthy(&d["id"]) { self.exam_progress(d) } else { (0.0, 0.0) };
            lines.push(format!("Next up: {}, {}{}", d["title"].as_str().unwrap_or(""), when_text(d["date"].as_str().unwrap_or(""), real_today),
                if need != 0.0 { format!(" - {} of {} studied", fmt_hm(got.trunc() as i64), fmt_hm(need.trunc() as i64)) } else { String::new() }));
        }
        if tot >= 1800.0 {
            const END: [&str; 3] = ["Otsukare~ rest well, you did good today!", "Every minute counts. Proud of you~kon!",
                                    "Tomorrow we go again, ne~? Sleep well!"];
            lines.push(self.brain.rng.choice(&END).to_string());
        } else {
            lines.push("Tomorrow's a fresh page. I'll help you start strong~".into());
        }
        lines.join("\n")
    }

    // ---- every 30 s: missed sessions, alerts, recap, golden hour ------------------------------------------------------

    /// study_slow_tick's timer (the window side calls it every second)
    pub fn study_timer(&mut self, now: f64) {
        if now >= self.study.slow_next {
            self.study.slow_next = now + 30.0;
            self.study_slow(now);
        }
    }

    pub fn study_slow(&mut self, now: f64) {
        self.study_missed(now);
        if self.brk.is_some() || self.lockin.is_some() || self.env.idle > 120.0 || self.quiz_on() {
            return;
        }
        let nl = self.nl();
        let today = day_n(nl);
        let ds = iso(today);
        let sod = nl.rem_euclid(86400.0) as i64;
        let (hour, minute) = (sod / 3600, sod / 60 % 60);
        let busy = self.session.is_some() || self.current_mood(now) == "alert";
        let rt = self.cfg.raw.get("recap_time").filter(|v| truthy(v)).map(|v| v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))
            .unwrap_or_else(|| "21:00".into());
        let (rh, rm) = rt.split_once(':').and_then(|(a, b)| Some((a.trim().parse::<i64>().ok()?, b.trim().parse::<i64>().ok()?)))
            .filter(|_| rt.matches(':').count() == 1).unwrap_or((21, 0));
        let life = self.life_now();
        if self.cfg.b("recap", true) && !busy && (hour, minute) >= (rh, rm) && hour < 24
            && (life["slog"].get(&ds).is_some_and(truthy) || tt_today(&self.tt, &ds).iter().any(|&i| truthy(&self.tt[i]["plan"])))
            && !self.said_today("recap") {
            let txt = self.recap_text(today);
            self.chat_line(now, "Fox", &txt);
            self.events.push(Ev::Say { text: format!("Your day recap is ready~ {}", txt.split('\n').nth(1).unwrap_or("")), secs: 20.0,
                                       choices: vec![("Show me".into(), "open_hub:chat".into()), ("Thanks!".into(), String::new())] });
            self.events.push(Ev::Beep { kind: "ding".into(), variety: false });
            return;
        }
        if busy {
            return;
        }
        if self.cfg.b("weak_alerts", true) && (15..21).contains(&hour) && life["said"]["weak"].as_str() != Some(ds.as_str()) {
            if let Some((text, sid)) = self.weak_subject(false) {
                self.said_today("weak");
                self.events.push(Ev::Say { text, secs: 30.0, choices: vec![(format!("25 min {}", sname(&sid)), format!("study_sess:{sid}")),
                                                                         ("Later".into(), String::new())] });
                return;
            }
        }
        if self.cfg.b("golden_hour", true) && minute < 10 && life["said"]["golden"].as_str() != Some(ds.as_str()) {
            let b = self.best_hours();
            if !b.is_empty() && b.iter().take(2).any(|r| r.0 == hour) {
                let today_min = sum_vals(&life["slog"][ds.as_str()]) / 60.0;
                if today_min < 30.0 {
                    self.said_today("golden");
                    let sid = self.weak_subject(true).map(|w| w.1).unwrap_or_else(|| subject_ids()[0].clone());
                    self.events.push(Ev::Say { text: format!("Kon! It's {hour:02}:00, usually your golden focus hour~ Shall we use it?"), secs: 30.0,
                                               choices: vec![(format!("25 min {}", sname(&sid)), format!("study_sess:{sid}")),
                                                             ("Not now".into(), String::new())] });
                }
            }
        }
    }

    /// weak_subject: (message, subject) for the subject that most needs time
    fn weak_subject(&mut self, any: bool) -> Option<(String, String)> {
        let week = self.subject_minutes(7, None);
        let wk_now = self.subject_minutes(7, Some(self.week_start()));
        let today = day_n(self.nl());
        let frac_week = (weekday(today) + 1) as f64 / 7.0;
        let goals = self.life_now()["goals"].clone();
        let get = |v: &Vec<(String, f64)>, s: &str| v.iter().find(|(a, _)| a == s).map(|x| x.1).unwrap_or(0.0);
        let mut worst: Option<(f64, String, String)> = None;
        for sid in subject_ids() {
            let ex = self.exam_for(&sid, true);
            let goal = goals.get(&sid).map(|g| num(g, 0.0)).unwrap_or(0.0);
            let days = ex.as_ref().and_then(|e| parse_iso(e["date"].as_str().unwrap_or(""))).map(|d| d - today);
            let m = get(&week, &sid);
            let wn = get(&wk_now, &sid);
            let (score, text);
            if let (Some(ex), Some(days)) = (ex.as_ref().filter(|_| days.is_some_and(|d| d <= 14) && m < 90.0), days) {
                let _ = days;
                score = m / 90.0;
                text = format!("Ne~ only {} of {} in the last 7 days, and \u{201c}{}\u{201d} is {}. Shall we do some {} now?",
                               fmt_hm(m.trunc() as i64), sname(&sid), clip(ex["title"].as_str().unwrap_or(""), 26),
                               when_text(ex["date"].as_str().unwrap_or(""), today), sname(&sid));
            } else if goal != 0.0 && weekday(today) >= 2 && wn < goal * 60.0 * frac_week * 0.5 {
                score = wn / (goal * 60.0 * frac_week).max(1.0);
                text = format!("Kon~ {} is behind: {} of your {} goal this week. A little session now?", sname(&sid),
                               fmt_hm(wn.trunc() as i64), fmt_hm((goal * 60.0).trunc() as i64));
            } else if any {
                score = 2.0 + m / 600.0;
                text = String::new();
            } else {
                continue;
            }
            if worst.as_ref().is_none_or(|w| score < w.0) {
                worst = Some((score, text, sid.clone()));
            }
        }
        let w = worst?;
        if w.1.is_empty() && !any {
            return None;
        }
        Some((w.1, w.2))
    }

    /// study_missed: planned sessions whose time passed: credit what you did, then re-plan the rest
    pub fn study_missed(&mut self, now: f64) {
        let nl = self.nl();
        let today = day_n(nl);
        let (mut fresh, mut changed) = (vec![], false);
        let mut add_studied: Vec<(String, i64)> = vec![];
        let mut i = 0;
        while i < self.tt.len() {
            let it = self.tt[i].clone();
            i += 1;
            if !truthy(&it["plan"]) || truthy(&it["missed"]) {
                continue;
            }
            let Some(day) = parse_iso(it["date"].as_str().unwrap_or("")) else { continue };
            let Some((_, en)) = Self::tt_win(&it, day) else { continue };
            if en > nl || tt_done(&it, &iso(day)) {
                continue;
            }
            let a = int_of(&it["auto_s"], 0);
            if a as f64 >= 0.7 * int_of(&it.get("mins").cloned().unwrap_or(json!(50)), 50) as f64 * 60.0 {
                self.tt_mark_day(now, i - 1, true, &iso(day));
                continue;
            }
            self.tt[i - 1]["missed"] = json!(true);
            changed = true;
            if a >= 60 {
                add_studied.push((it["plan"].as_str().map(String::from).unwrap_or_else(|| it["plan"].to_string()), a / 60));
            }
            if nl - en < 20.0 * 3600.0 {
                fresh.push(self.tt[i - 1].clone());
            }
        }
        if !changed {
            return;
        }
        save_tt(&self.tt);
        self.life_do(|l| {
            for (p, n) in &add_studied {
                bump(obj(l, "studied"), p, *n as f64);
            }
        });
        if !fresh.is_empty() && self.cfg.b("planner", true) {
            self.replan();
            let it = &fresh[0];
            let tds = iso(today);
            let mut nxt: Vec<Value> = self.tt.iter()
                .filter(|x| x["plan"] == it["plan"] && !truthy(&x["missed"]) && !tt_done(x, &tds)
                    && x["date"].as_str().unwrap_or("") >= tds.as_str())
                .cloned().collect();
            nxt.sort_by(|a, b| (a["date"].as_str(), a["start"].as_str()).cmp(&(b["date"].as_str(), b["start"].as_str())));
            nxt.retain(|x| parse_iso(x["date"].as_str().unwrap_or("")).and_then(|d| Self::tt_win(x, d)).is_some_and(|w| w.0 > nl));
            let place = nxt.first().map(|x| format!("{} {}", when_text(x["date"].as_str().unwrap_or(""), today), x["start"].as_str().unwrap_or("")))
                .unwrap_or_else(|| "your next free hour".into());
            let mut msg = format!("Uu~ you missed \u{201c}{}\u{201d} at {}. No worries, I moved it to {place}~",
                                  clip(it["task"].as_str().unwrap_or(""), 30), it["start"].as_str().unwrap_or(""));
            if fresh.len() > 1 {
                msg += &format!(" ({} missed sessions re-planned)", fresh.len());
            }
            self.later(now + 1.5, Later::SayLine(msg));
        }
    }

    // ---- chat --------------------------------------------------------------------------------------------------------

    /// nb_ask's part in chat: she says she's thinking; the answer itself comes from the brains (Ev::Command nb_ask)
    pub(crate) fn nb_ask_chat(&mut self, now: f64, sid: Option<String>, q: &str, mode: &str) -> String {
        let q = q.trim();
        if q.is_empty() {
            return "Ask me something first~".into();
        }
        if self.stat_add("homework", 1) >= 5 {
            self.award(now, "homework");
        }
        self.events.push(Ev::Command(format!("nb_ask:{mode}:{}:{q}", sid.clone().unwrap_or_default())));
        self.set_mood(now, "focus", 4.0);
        let nm = sid.map(|s| self.subject_full(&s)).unwrap_or_else(|| "your question".into());
        format!("Fumu fumu~ thinking about {nm}{}...", if mode == "eli12" { " (like you're 12)" } else { "" })
    }

    pub(crate) fn study_route(&mut self, now: f64, text: &str) -> Option<String> {
        let raw = text.trim();
        let low = raw.to_lowercase().trim().to_string();
        let lw = low.trim_end_matches([' ', '!', '.', '?', '~']).to_string();
        if rx!(r"^(?:open )?(?:my )?(?:notebook|subject notebook|study notebook)$").is_match(&lw) {
            self.later(now + 0.15, Later::Command("open_hub:notebook".into()));
            return Some("Opening your study books~ pick one!".into());
        }
        if let Some(m) = rx!(r"^conjugate\s+(?:the verb\s+)?([a-zàâçéèêëîïôûùüÿœ' ]{2,30})$").captures(&lw) {
            return Some(conjugate_fr(&m[1]).unwrap_or_else(|| "Hmm, I only know regular -er/-ir/-re verbs and the common irregular ones~".into()));
        }
        let dur = rx!(r"(\d+(?:\.\d+)?|an?|one|two|three|four|five|half an?)\s*(hours?|hrs?|h|minutes?|mins?|m)\b").captures(&low);
        let strip_dur = || rx!(r"(\d+(?:\.\d+)?)\s*(hours?|hrs?|h|minutes?|mins?|m)\b").replace_all(&low, " ").to_string();
        let future = rx!(r"^(?:i'?m|i am|i will|i'll|i'?m gonna|i'?m going to|gonna|going to|let'?s)\b").is_match(&lw)
            && !rx!(r"\b(past|last|already|been|was|did|studied|revised|yesterday)\b").is_match(&lw);
        if let Some(d) = &dur {
            let (v, unit) = (word_num(&d[1]), d[2].to_string());
            if future {
                if let Some(sid) = self.subject_in(&strip_dur()) {
                    let mins = (if unit.starts_with('h') { v * 60.0 } else { v }).min(480.0).max(10.0);
                    self.study.over = Some((sid.clone(), unix_now() + mins * 60.0));
                    return Some(format!("Kon! I'll count the next {} as {}~ Ganbatte! (Want a timer too? Say 'focus {} min on {}')",
                                        fmt_hm(mins.trunc() as i64), sname(&sid), mins.trunc() as i64, sname(&sid)));
                }
            }
            if rx!(r"\b(stud(?:y|ied|ying)|revis(?:e|ed|ing)|did|done|worked|working|log|add|spent)\b").is_match(&low)
                && !rx!(r"^(?:how|what|when|should|can|do)\b").is_match(&low) {
                if let Some(sid) = self.subject_in(&strip_dur()) {
                    let mins = if unit.starts_with('h') { v * 60.0 } else { v };
                    if 0.0 < mins && mins <= 960.0 {
                        let mut day = day_n(self.nl());
                        let today = day;
                        if rx!(r"\byesterday\b").is_match(&low) {
                            day -= 1;
                        }
                        let d = self.log_study(now, &sid, mins, &iso(day));
                        return Some(format!("Kon! Added {} of {}{}{}~ Yatta!", fmt_hm(mins.trunc() as i64), sname(&sid),
                            if day != today { " (yesterday)" } else { "" },
                            match d {
                                Some(d) => format!(" - it counts toward \u{201c}{}\u{201d} now", d["title"].as_str().unwrap_or("")),
                                None => " (tell me the exam, like 'my bio MYA is on Jan 12', and it'll count toward it too)".into(),
                            }));
                    }
                }
            }
        }
        let m = rx!(r"^i'?m (?:now )?(?:studying|revising|doing|working on)\s+(.+)$").captures(&lw)
            .or_else(|| rx!(r"^(?:switch|change) (?:subject )?to\s+(.+)$").captures(&lw));
        if let (Some(m), None) = (m, &dur) {
            if let Some(sid) = self.subject_in(&m[1]) {
                self.study.over = Some((sid.clone(), unix_now() + 7200.0));
                return Some(format!("Kon! I'll count your time as {} for the next 2 hours (say 'stop studying' to end it)~", sname(&sid)));
            }
        }
        if rx!(r"^(?:stop|done|finished) (?:studying|revising)$").is_match(&lw) {
            self.study.over = None;
            return Some("Otsukare~ I'll go back to guessing from your windows.".into());
        }
        if rx!(r"^what (?:am i|subject am i) (?:studying|doing)|^which subject").is_match(&lw) {
            return Some(match self.study.now.clone() {
                Some(What::S(s)) => format!("Right now I'm counting this as {}~ (\u{201c}{}\u{201d})", sname(&s), clip(&self.study.title, 40)),
                Some(What::U(k, _)) => {
                    self.later(now + 0.2, Later::StudyAsk(k));
                    "I'm not sure yet~ tell me with one tap!".into()
                }
                _ => "Hmm, this doesn't look like studying to me. Say 'I'm studying bio' and I'll count it~".into(),
            });
        }
        if rx!(r"^(?:(?:end of (?:the )?day|daily|day|today'?s?|my) )?recap$|^how did i do today|^(?:sum ?up|summary of) (?:my )?day").is_match(&lw) {
            let t = day_n(self.nl());
            return Some(self.recap_text(t));
        }
        if rx!(r"distraction report|what (?:distracts|distracted) me|my distractions?$|what wastes my time").is_match(&lw) {
            return Some(self.distraction_report(7));
        }
        if rx!(r"best (?:focus )?times?|when do i (?:focus|study) best|golden hour|focus rhythm").is_match(&lw) {
            return Some(self.best_time_text());
        }
        if rx!(r"(?:study|weekly|week'?s?) (?:report|hours|stats)|hours this week|how (?:much|long) (?:have|did) i stud").is_match(&lw) {
            return Some(self.week_report());
        }
        let m = rx!(r"^(?:set )?(?:my )?(.+?) goal (?:to |is |= |of )?(\d+(?:\.\d+)?)\s*(?:h|hrs?|hours?)(?: a week| per week| weekly)?$").captures(&lw)
            .or_else(|| rx!(r"^(?:i want to (?:do|study) )?(\d+(?:\.\d+)?)\s*(?:h|hrs?|hours?) (?:a|per) week (?:of|for|on) (.+)$").captures(&lw));
        if let Some(m) = m {
            let (a, b) = (m[1].to_string(), m[2].to_string());
            let (subj, hrs) = if !a.starts_with(|c: char| c.is_ascii_digit()) { (a, b) } else { (b, a) };
            if let Some(sid) = self.subject_in(&subj) {
                let h: f64 = hrs.parse().unwrap_or(0.0);
                let s2 = sid.clone();
                self.life_do(|l| obj(l, "goals")[s2.as_str()] = json!(h));
                return Some(format!("Goal set: {} of {} every week~ I'll cheer you on (and nudge you if it slips)!",
                                    fmt_hm((h * 60.0).trunc() as i64), sname(&sid)));
            }
        }
        if let Some(m) = rx!(r"(?is)^(?:eli12|eli5|explain (?:it |this )?(?:to me )?like i'?m (?:12|twelve|5|five|a kid|a child))\s*[:,\-]?\s*(.+)$").captures(raw) {
            let q = m[1].to_string();
            let sid = self.subject_in(&q);
            return Some(self.nb_ask_chat(now, sid, &q, "eli12"));
        }
        if let Some(m) = rx!(r"^(?:test me|quiz me|ask me questions)(?: on| about)?\s+(.+)$").captures(&lw) {
            if !rx!(r"\b(?:notes?|flashcards?|deck)\b").is_match(&lw) {
                let topic = m[1].to_string();
                if let Some(sid) = self.subject_in(&topic) {
                    for c in ["open_hub:notebook".to_string(), format!("nb_show:{sid}"), format!("nb_start_test:{sid}:{topic}")] {
                        self.later(now + 0.2, Later::Command(c));
                    }
                    return Some(format!("Kon! Opening your {} notebook for a test on {}~", sname(&sid), clip(&topic, 30)));
                }
            }
        }
        if rx!(r"flashcards? from (?:my )?(?:notes?|file|document|pdf)|make flashcards (?:out )?of (?:my )?notes").is_match(&lw) {
            let s = self.subject_in(&low).unwrap_or_default();
            self.later(now + 0.2, Later::Command(format!("nb_cards_from_file:{s}")));
            return Some("Pick your notes file (PDF, Word, PowerPoint or text) and I'll turn it into flashcards~".into());
        }
        if rx!(r"^(?:review|practise|practice|redo) (?:my )?mistakes$").is_match(&lw) {
            return Some(self.review_start());
        }
        None
    }

    /// the test's view of self.st
    pub fn study_state(&self) -> Value {
        let mut credit = Map::new();
        for (k, v) in &self.study.credit {
            credit.insert(k.clone(), json!(v));
        }
        json!({"now": self.study.now.as_ref().map(|w| w.to_json()), "override": self.study.over.as_ref().map(|(s, t)| json!([s, t])),
               "credit": credit})
    }
}
