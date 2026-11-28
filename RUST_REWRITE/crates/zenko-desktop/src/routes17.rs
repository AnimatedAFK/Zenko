//! Zenko.gym_route's version routes after z18: z17_route (periodic table, calculator, French dictionary, Bible, close or
//! keep rules), z15_route (screen time, the world map, updates), ib_route (IA / EE / TOK words, limits, due dates,
//! exam countdown) and the garden / delay-picker lines.

use crate::plan::{iso, parse_iso};
use crate::rewards::{Later, garden_health, garden_stage, journey_state, local_days, rdata};
use crate::{Companion, read_json, unix_now};
use fancy_regex::Regex as FRegex;
use regex::Regex;
use serde_json::{Value, json};
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}
macro_rules! frx {
    ($p:expr) => {{
        static R: OnceLock<FRegex> = OnceLock::new();
        R.get_or_init(|| FRegex::new($p).unwrap())
    }};
}

pub fn routes_data() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/routes.json")).expect("routes.json"))
}

fn py_truthy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}

fn strip_low(text: &str) -> String {
    text.to_lowercase().trim_matches([' ', '.', '!', '?', '~']).to_string()
}

// ---- Bible references ----------------------------------------------------------------------------------------------
/// bible_book('Psa' / 'psalms' / '1 John' / '1jn') -> index into BIBLE_BOOKS
pub fn bible_book(name: &str) -> Option<usize> {
    let s = name.trim().to_lowercase().replace('.', "");
    if s.is_empty() {
        return None;
    }
    let s2 = match frx!(r"^(1|2|3|i{1,3}(?=\s))\s*").captures(&s).ok().flatten() {
        Some(m) => {
            let g = m.get(1).unwrap().as_str();
            let n = match g {
                "i" => "1",
                "ii" => "2",
                "iii" => "3",
                x => x,
            };
            format!("{n}{}", &s[m.get(0).unwrap().end()..])
        }
        None => s.clone(),
    };
    let s2n = s2.replace(' ', "");
    let books = routes_data()["BIBLE_BOOKS"].as_array().unwrap();
    for (i, b) in books.iter().enumerate() {
        let (c, n, slug) = (b[0].as_str().unwrap(), b[1].as_str().unwrap(), b[3].as_str().unwrap());
        if s2n == c.to_lowercase() || s2n == n.to_lowercase().replace(' ', "") || s2n == slug.replace('_', "") {
            return Some(i);
        }
    }
    let al = &routes_data()["BIBLE_ALIAS"];
    if let Some(a) = al.get(&s2).or_else(|| al.get(&s2n)).and_then(|v| v.as_str()).filter(|a| !a.is_empty()) {
        return books.iter().position(|b| b[0] == a);
    }
    for (i, b) in books.iter().enumerate() {
        let nm = b[1].as_str().unwrap().to_lowercase().replace(' ', "");
        if s2n.chars().count() >= 2 && nm.starts_with(&s2n) {
            return Some(i);
        }
    }
    None
}

/// bible_parse_ref('Psalm 119:105') -> (book, chapter, verse)
pub fn bible_parse_ref(text: &str) -> Option<(usize, i64, Option<i64>)> {
    let m = rx!(r"^\s*((?:[1-3]|i{1,3})?\s*[A-Za-z][A-Za-z .]*?)\s*(\d+)(?:\s*[:.]\s*(\d+))?").captures(text)?;
    let bi = bible_book(&m[1])?;
    let max = routes_data()["BIBLE_BOOKS"][bi][2].as_i64().unwrap();
    let ch = m[2].parse::<i64>().unwrap_or(i64::MAX).clamp(1, max);
    Some((bi, ch, m.get(3).map(|v| v.as_str().parse().unwrap_or(0))))
}

/// Python's str.title()
pub(crate) fn py_title(s: &str) -> String {
    let mut out = String::new();
    let mut prev = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            if prev { out.extend(c.to_lowercase()) } else { out.extend(c.to_uppercase()) }
            prev = true;
        } else {
            out.push(c);
            prev = false;
        }
    }
    out
}

// ---- close or keep rules -------------------------------------------------------------------------------------------
fn rule_words(s: &str) -> Vec<String> {
    rx!(r"[,;\n]+").split(s).map(|w| w.trim().to_lowercase()).filter(|w| !w.is_empty()).collect()
}

pub fn rule_text(r: &Value) -> String {
    let site = r["site"].as_str().filter(|s| !s.is_empty()).unwrap_or("Any site or app");
    let words = rule_words(r["words"].as_str().unwrap_or(""));
    let w = if words.is_empty() {
        String::new()
    } else {
        format!(" whose title has \u{201c}{}\u{201d}", words.join("\u{201d}, \u{201c}"))
    };
    let what = if site.to_lowercase().starts_with("any") {
        if w.is_empty() { "everything".to_string() } else { format!("anything{w}") }
    } else {
        format!("{site}{w}")
    };
    if r["do"] == "keep" { format!("Keep {what} open") } else { format!("Close {what}") }
}

// ---- screen time (read side) ---------------------------------------------------------------------------------------
fn round_even(x: f64) -> i64 {
    x.round_ties_even() as i64
}

pub fn screen_fmt(secs: f64) -> String {
    let m = round_even(secs / 60.0);
    if m < 1 {
        return if secs > 0.0 { "<1m".into() } else { "0m".into() };
    }
    let (h, m) = (m / 60, m % 60);
    if h > 0 { format!("{h}h {m:02}m") } else { format!("{m}m") }
}

pub fn screen_fmt_long(secs: f64) -> String {
    let m = round_even(secs / 60.0);
    let (h, m) = (m.div_euclid(60), m.rem_euclid(60));
    let hs = |h: i64| format!("{h} hour{}", if h == 1 { "" } else { "s" });
    if h != 0 && m != 0 {
        format!("{} {m} min", hs(h))
    } else if h != 0 {
        hs(h)
    } else {
        format!("{m} min")
    }
}

pub struct ScreenRange {
    pub apps: Vec<(String, f64)>,
    pub sites: Vec<(String, f64)>,
    pub total: f64,
    pub top: Vec<(String, f64, &'static str)>,
    /// seconds in each hour of the day (summed over the days)
    pub hours: [f64; 24],
    /// (iso date, its apps' total), oldest first
    pub days: Vec<(String, f64)>,
}

fn add_to(v: &mut Vec<(String, f64)>, k: &str, x: f64) {
    match v.iter_mut().find(|e| e.0 == k) {
        Some(e) => e.1 += x,
        None => v.push((k.into(), x)),
    }
}

pub fn screen_range(life: &Value, days: i64) -> ScreenRange {
    screen_range_at(life, days, local_days())
}

/// screen_range(days) with today's day number given
pub fn screen_range_at(life: &Value, days: i64, today: i64) -> ScreenRange {
    let s = &life["screen"]["days"];
    let (mut apps, mut sites) = (vec![], vec![]);
    let mut hours = [0.0f64; 24];
    let mut dl = vec![];
    for k in 0..days {
        let ds = iso(today - k);
        let d = &s[ds.as_str()];
        if let Some(h) = d["hours"].as_array().filter(|h| !h.is_empty()) {
            for (i, v) in h.iter().take(24).enumerate() {
                hours[i] += v.as_f64().unwrap_or(0.0);
            }
        }
        dl.push((ds, zenko_sound::music::py_sum(d["apps"].as_object().map(|o| o.values().map(|v| v.as_f64().unwrap_or(0.0))
            .collect::<Vec<_>>()).unwrap_or_default().into_iter())));
        if let Some(o) = d["apps"].as_object() {
            for (a, v) in o {
                add_to(&mut apps, a, v.as_f64().unwrap_or(0.0));
            }
        }
        if let Some(o) = d["sites"].as_object() {
            for (a, v) in o {
                add_to(&mut sites, a, v.as_f64().unwrap_or(0.0));
            }
        }
    }
    // sum() over the apps dict: CPython 3.12's float sum
    let total = zenko_sound::music::py_sum(apps.iter().map(|e| e.1));
    let names = &routes_data()["SCREEN_APP_NAMES"];
    let browsers: Vec<Value> = routes_data()["BROWSERS"].as_array().unwrap().iter()
        .map(|b| names.get(b.as_str().unwrap()).cloned().unwrap_or(Value::Null)).collect();
    let mut top: Vec<(String, f64, &'static str)> = sites.iter().filter(|e| e.0 != "Other websites" && e.0 != "New tab")
        .map(|e| (e.0.clone(), e.1, "site")).collect();
    top.extend(apps.iter().filter(|e| !browsers.contains(&json!(e.0))).map(|e| (e.0.clone(), e.1, "app")));
    top.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    dl.reverse();
    ScreenRange { apps, sites, total, top, hours, days: dl }
}

pub fn screen_summary(life: &Value) -> String {
    let r = screen_range(life, 1);
    if r.total < 60.0 {
        return "I haven't seen much screen time today yet (private windows and banking apps never count).".into();
    }
    let top: Vec<_> = r.top.iter().take(3).collect();
    if top.is_empty() {
        return format!("Today: {} of screen time so far.", screen_fmt_long(r.total));
    }
    let lead = format!("Longest today: {}, {}.", top[0].0, screen_fmt_long(top[0].1));
    let rest: Vec<String> = top[1..].iter().map(|t| format!("{} {}", t.0, screen_fmt(t.1))).collect();
    let rest = if rest.is_empty() { String::new() } else { format!(" Then {}.", rest.join(", ")) };
    format!("{lead}{rest} Total screen time: {}~", screen_fmt_long(r.total))
}

// ---- IB tasks ------------------------------------------------------------------------------------------------------
/// fmt_num for an int: "{:,}"
pub fn fmt_int(v: i64) -> String {
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 { format!("-{out}") } else { out }
}

fn ib_kind_of(what: &str) -> &'static str {
    if rx!(r"\bee\b|extended").is_match(what) {
        "ee"
    } else if what.contains("exhibition") {
        "tok_exh"
    } else if what.contains("tok") {
        "tok_essay"
    } else {
        "ia"
    }
}

const MONTHS: [&str; 12] = ["january", "february", "march", "april", "may", "june", "july", "august", "september",
                            "october", "november", "december"];

fn month_of(s: &str, full: bool) -> Option<i64> {
    let l = s.to_lowercase();
    MONTHS.iter().position(|m| if full { *m == l } else { m[..3] == l }).map(|i| i as i64 + 1)
}

fn valid(y: i64, m: i64, d: i64) -> Option<i64> {
    if !(1..=12).contains(&m) || d < 1 {
        return None;
    }
    let dim = [31, if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if d > dim[(m - 1) as usize] { None } else { Some(crate::rewards::days_from_civil(y, m, d)) }
}

/// the strptime formats ib_route tries, in order ("%d %B", "%B %d", "%d %b", "%b %d", "%d %B %Y", "%B %d %Y",
/// "%d/%m/%Y", "%d/%m"); a missing year is 1900 first (so 29 Feb fails), then this year
fn ib_strptime(raw: &str, year: i64) -> Option<i64> {
    let ws = r"\s+";
    let dd = r"(\d{1,2})";
    let mon = r"([A-Za-z]+)";
    let yy = r"(\d{4})";
    let pats: [(String, &str); 8] = [
        (format!("^{dd}{ws}{mon}$"), "dB"), (format!("^{mon}{ws}{dd}$"), "Bd"), (format!("^{dd}{ws}{mon}$"), "db"),
        (format!("^{mon}{ws}{dd}$"), "bd"), (format!("^{dd}{ws}{mon}{ws}{yy}$"), "dBY"),
        (format!("^{mon}{ws}{dd}{ws}{yy}$"), "BdY"), (format!(r"^{dd}/(\d{{1,2}})/{yy}$"), "dmY"),
        (format!(r"^{dd}/(\d{{1,2}})$"), "dm"),
    ];
    for (p, f) in pats {
        let Some(c) = Regex::new(&p).unwrap().captures(raw) else { continue };
        let g = |i: usize| c.get(i).unwrap().as_str();
        let (d, m, y) = match f {
            "dB" => (g(1).parse().ok(), month_of(g(2), true), None),
            "Bd" => (g(2).parse().ok(), month_of(g(1), true), None),
            "db" => (g(1).parse().ok(), month_of(g(2), false).filter(|_| g(2).len() == 3), None),
            "bd" => (g(2).parse().ok(), month_of(g(1), false).filter(|_| g(1).len() == 3), None),
            "dBY" => (g(1).parse().ok(), month_of(g(2), true), g(3).parse().ok()),
            "BdY" => (g(2).parse().ok(), month_of(g(1), true), g(3).parse().ok()),
            "dmY" => (g(1).parse().ok(), g(2).parse().ok(), g(3).parse().ok()),
            _ => (g(1).parse().ok(), g(2).parse().ok(), None),
        };
        let (Some(d), Some(m)) = (d, m) else { continue };
        match y {
            Some(y) => {
                if let Some(x) = valid(y, m, d) {
                    return Some(x);
                }
            }
            None => {
                if valid(1900, m, d).is_some() {
                    if let Some(x) = valid(year, m, d) {
                        return Some(x);
                    }
                    return None; // date.replace(year=...) raising ends the loop with d_ = None
                }
            }
        }
    }
    None
}

impl Companion {
    fn life(&self) -> Value {
        read_json("zenko_life.json")
    }

    /// sname(sid, short=False): the subject's full name (Notebook renames count)
    pub fn subject_full(&self, sid: &str) -> String {
        let life = self.life();
        if let Some(n) = life["subject_names"].get(sid).and_then(|v| v.as_str()) {
            return n.into();
        }
        crate::watch::data().raw["SUBJECTS"].as_array().unwrap().iter().find(|s| s[0] == sid)
            .and_then(|s| s[1].as_str()).map(String::from).unwrap_or_else(|| if sid.is_empty() { "?".into() } else { sid.into() })
    }

    pub fn subject_in(&self, text: &str) -> Option<String> {
        let (sid, sc, _) = crate::watch::classify_subject(text, &self.life()["subj_words"]);
        sid.filter(|_| sc >= 1)
    }

    fn ib_find(&self, text: &str) -> Option<usize> {
        let low = text.to_lowercase();
        let tasks = self.life()["ibtasks"].as_array().cloned().unwrap_or_default();
        let mut best = None;
        for (i, t) in tasks.iter().enumerate() {
            let kind = t["kind"].as_str().unwrap_or("");
            let title = t["title"].as_str().unwrap_or("").to_lowercase();
            if kind == "ee" && rx!(r"\b(?:ee|extended essay)\b").is_match(&low) {
                return Some(i);
            }
            if kind == "tok_essay" && frx!(r"\btok\b(?! exhibition)").is_match(&low).unwrap_or(false) {
                best = best.or(Some(i));
            }
            if kind == "tok_exh" && rx!(r"\btok exhibition\b|\bexhibition\b").is_match(&low) {
                return Some(i);
            }
            if kind == "ia" && rx!(r"\bia\b|internal assessment|exploration").is_match(&low) {
                let sid = self.subject_in(text);
                if sid.is_some() && sid.as_deref() == t["subject"].as_str() {
                    return Some(i);
                }
                if sid.is_none() {
                    best = best.or(Some(i));
                }
            }
            if low.contains(&title) {
                return Some(i);
            }
        }
        best
    }

    pub fn ib_add(&mut self, kind: &str, subject: &str) -> usize {
        let kinds = routes_data()["IB_TASK_KINDS"].as_array().unwrap();
        let k = kinds.iter().find(|x| x[0] == kind).unwrap_or(kinds.last().unwrap()).clone();
        let (u, tg) = match routes_data()["IB_IA_DEFAULTS"].get(subject).filter(|_| kind == "ia") {
            Some(d) => (d[0].clone(), d[1].clone()),
            None => (k[2].clone(), k[3].clone()),
        };
        let title = if kind == "ia" && !subject.is_empty() {
            format!("{} IA", self.subject_full(subject))
        } else {
            k[1].as_str().unwrap_or("").to_string()
        };
        let id = format!("{:x}", self.brain.rng.getrandbits(32));
        let t = json!({"id": id, "kind": kind, "subject": subject, "unit": u, "target": tg, "words": 0, "title": title,
                       "due": "", "stages": [], "updated": iso(local_days()), "hist": []});
        self.life_do(|l| {
            if !l["ibtasks"].is_array() {
                l["ibtasks"] = json!([]);
            }
            let a = l["ibtasks"].as_array_mut().unwrap();
            a.push(t);
            a.len() - 1
        })
    }

    fn ib_task(&self, i: usize) -> Value {
        self.life()["ibtasks"][i].clone()
    }

    pub fn ib_set_words(&mut self, i: usize, n: i64) -> String {
        let n = n.max(0);
        let day = iso(local_days());
        let old = self.ib_task(i)["words"].as_i64().unwrap_or(0);
        self.life_do(|l| {
            let t = &mut l["ibtasks"][i];
            t["words"] = json!(n);
            t["updated"] = json!(day);
            if !t["hist"].is_array() {
                t["hist"] = json!([]);
            }
            let h = t["hist"].as_array_mut().unwrap();
            h.push(json!([day, n]));
            let cut = h.len().saturating_sub(30);
            h.drain(..cut);
        });
        let t = self.ib_task(i);
        let title = t["title"].as_str().unwrap_or("");
        let tg = t["target"].as_i64().unwrap_or(0);
        if t["unit"] == "pages" {
            return format!("{title}: {n} page{} logged~", if n == 1 { "" } else { "s" });
        }
        let mut msg = format!("{title}: {} words{}.", fmt_int(n), if tg != 0 { format!(" of {}", fmt_int(tg)) } else { String::new() });
        if tg != 0 && n > tg {
            msg += &format!(" That's over the limit by {}: the examiner stops reading at {}, so trim it~", fmt_int(n - tg), fmt_int(tg));
        } else if n > old {
            msg += &format!(" +{} since last time. Yatta~", fmt_int(n - old));
        }
        msg
    }

    fn exam_day_n(&self) -> i64 {
        let def = routes_data()["EXAM_DATE_DEFAULT"].as_str().unwrap();
        let s = match self.cfg.raw.get("exam_date") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(v) if py_truthy(v) => v.to_string(),
            _ => def.into(),
        };
        parse_iso(&s).unwrap_or_else(|| parse_iso(def).unwrap())
    }

    pub(crate) fn ib_route(&mut self, now: f64, raw: &str) -> Option<String> {
        let lw = strip_low(raw);
        if let Some(m) = rx!(r"\b(?:my\s+)?(.{0,30}?\b(?:ia|ee|extended essay|tok(?: essay| exhibition)?|internal assessment|exploration))\b(?:\s+(?:draft|is|at|has|now))*\s+(?:is\s+|at\s+)?(\d[\d,]*)\s*(words?|pages?)\b").captures(&lw) {
            let what = m[1].to_string();
            let i = match self.ib_find(&what) {
                Some(i) => i,
                None => {
                    let kind = ib_kind_of(&what);
                    let subj = if kind == "ia" { self.subject_in(&what).unwrap_or_default() } else { String::new() };
                    self.ib_add(kind, &subj)
                }
            };
            let n: i64 = m[2].replace(',', "").parse().unwrap_or(0);
            return Some(self.ib_set_words(i, n));
        }
        if let Some(m) = rx!(r"\b(?:my\s+)?(.{0,30}?\b(?:ia|ee|extended essay|tok(?: essay| exhibition)?|internal assessment))\s+(?:limit|word limit|target)\s+(?:is\s+)?(\d[\d,]*)").captures(&lw) {
            if let Some(i) = self.ib_find(&m[1]) {
                let n: i64 = m[2].replace(',', "").parse().unwrap_or(0);
                self.life_do(|l| l["ibtasks"][i]["target"] = json!(n));
                return Some(format!("Okay~ {} limit set to {}.", self.ib_task(i)["title"].as_str().unwrap_or(""), fmt_int(n)));
            }
        }
        if let Some(m) = rx!(r"\b(?:my\s+)?(.{0,30}?\b(?:ia|ee|extended essay|tok(?: essay| exhibition)?|internal assessment))\s+(?:is\s+)?due\s+(?:on\s+|by\s+)?(.+)$").captures(&lw) {
            let what = m[1].to_string();
            let s = m[2].trim().trim_end_matches(['.', '!']);
            let raw_d = py_title(&rx!(r"(\d)(?:st|nd|rd|th)\b").replace_all(s, "$1"));
            let today = local_days();
            let year = crate::civil_from_days_pub(today).0 as i64;
            let mut d = parse_iso(&raw_d).filter(|_| rx!(r"^\d{4}-\d{2}-\d{2}$").is_match(&raw_d))
                .or_else(|| ib_strptime(&raw_d, year));
            if let Some(mut d_) = d.take() {
                if d_ < today {
                    let (y, mo, dd) = crate::civil_from_days_pub(d_);
                    d_ = valid(y as i64 + 1, mo as i64, dd as i64)?;
                }
                let i = match self.ib_find(&what) {
                    Some(i) => i,
                    None => {
                        let kind = ib_kind_of(&what);
                        let subj = if kind == "ia" { self.subject_in(&what).unwrap_or_default() } else { String::new() };
                        self.ib_add(kind, &subj)
                    }
                };
                let ds = iso(d_);
                self.life_do(|l| l["ibtasks"][i]["due"] = json!(ds));
                const WD: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
                const MO: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                let (_, mo, dd) = crate::civil_from_days_pub(d_);
                return Some(format!("Noted~ {} due {} {dd:02} {} ({} days). I'll nudge you if it goes quiet.",
                                    self.ib_task(i)["title"].as_str().unwrap_or(""), WD[(d_ + 3).rem_euclid(7) as usize],
                                    MO[mo as usize - 1], d_ - today));
            }
        }
        if rx!(r"^(?:open |show )?(?:my )?(?:ia|ee|tok)(?:\s*/\s*\w+)*\s*(?:tracker|progress)$|^(?:ia ee tok|ib tasks)$").is_match(&lw) {
            self.later(now + 0.15, Later::Command("open_hub:deadlines".into()));
            return Some("Here's your IA \u{b7} EE \u{b7} TOK tracker (bottom of Upcoming)~".into());
        }
        if rx!(r"^(?:how many days|days)\s+(?:until|till|to)\s+(?:my\s+)?(?:ib\s+)?exams?$|^exam countdown$").is_match(&lw) {
            let ed = self.exam_day_n();
            const MO: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
            let (y, mo, dd) = crate::civil_from_days_pub(ed);
            return Some(format!("{} days until your first exam ({dd:02} {} {y}). Ganbatte~", ed - local_days(), MO[mo as usize - 1]));
        }
        None
    }

    pub(crate) fn z17_route(&mut self, now: f64, text: &str) -> Option<String> {
        let lw = strip_low(text.trim());
        let cmd = |me: &mut Self, c: &str| me.later(now + 0.1, Later::Command(c.into()));
        if rx!(r"^(?:open |show )?(?:the |my )?(?:periodic table|ptable|元素)$").is_match(&lw) {
            cmd(self, "open_ptable");
            return Some("Here's the periodic table~ click an element for its details.".into());
        }
        if rx!(r"^(?:open |show )?(?:the |my )?(?:graphing )?(?:calculator|calc|gdc|cg ?50|fx-?cg ?50|電卓)$").is_match(&lw) {
            cmd(self, "open_calc");
            return Some("Calculator's open! MENU for Graph, Equation, Statistics and Distribution~".into());
        }
        if rx!(r"^(?:open |show )?(?:the |my )?(?:french )?(?:dictionary|dictionnaire|dico|translator)$").is_match(&lw) {
            cmd(self, "open_french_dict");
            return Some("Dictionnaire ouvert~ type a word in French or English.".into());
        }
        if let Some(m) = rx!(r"^(?:open |read |show )?(?:my |the )?(?:holy )?bible(?: (?:at|to))?(?: (.+))?$|^(?:read|open) ((?:[1-3] ?)?[a-z]+ \d+(?:[:.]\d+)?)$").captures(&lw) {
            let mut r = m.get(1).or(m.get(2)).map(|x| x.as_str()).unwrap_or("").trim().to_string();
            if !r.is_empty() && bible_parse_ref(&r).is_none() {
                if m.get(2).is_some() {
                    return None;
                }
                r.clear();
            }
            cmd(self, &format!("open_bible:{r}"));
            return Some(if r.is_empty() {
                "Your Bible is open~ click a verse for the Hebrew or Greek.".into()
            } else {
                format!("Opening {}~", py_title(&r))
            });
        }
        if let Some(m) = rx!(r"^(keep|allow|don'?t close|never close|always close|close|block)\s+(.+?)(?:\s+(?:if|when)\s+(?:it|the title)?\s*(?:has|contains|says|is about)\s+(.+))?$").captures(&lw) {
            if m.get(3).is_some() || &m[1] == "always close" || &m[1] == "never close" {
                let act = if ["always close", "close", "block"].contains(&&m[1]) { "close" } else { "keep" };
                let s = m[2].trim().to_string();
                let site = routes_data()["RULE_SITES"].as_array().unwrap().iter().find(|e| {
                    let n = e[0].as_str().unwrap();
                    s == n.to_lowercase() || e[1].as_array().unwrap().iter().any(|a| a.as_str().unwrap().trim_matches([' ', '/']) == s)
                }).map(|e| e[0].as_str().unwrap().to_string()).unwrap_or_else(|| py_title(&s));
                let rule = json!({"site": site, "words": rule_words(m.get(3).map(|x| x.as_str()).unwrap_or("")).join(", "),
                                  "do": act, "on": true});
                let txt = rule_text(&rule);
                crate::update_config(&mut self.cfg, |v| {
                    let mut rules = v.get("zk_rules").and_then(|r| r.as_array()).cloned().unwrap_or_default();
                    rules.push(rule.clone());
                    v["zk_rules"] = Value::Array(rules);
                });
                return Some(format!("Got it~ new rule: {txt}. (All rules: Settings > Close or keep.)"));
            }
        }
        if rx!(r"^(?:my |show (?:me )?my |what are my )?(?:close|keep|focus|lock-?in) rules$|^my rules$").is_match(&lw) {
            let rules = self.cfg.raw.get("zk_rules").and_then(|r| r.as_array()).cloned().unwrap_or_default();
            if rules.is_empty() {
                return Some("No rules yet~ say 'keep youtube if it has bio' or add them in Settings > Close or keep.".into());
            }
            let lines: Vec<String> = rules.iter().enumerate().map(|(i, r)| {
                let off = !r.get("on").map(py_truthy).unwrap_or(true);
                format!("{}. {}{}", i + 1, rule_text(r), if off { " (off)" } else { "" })
            }).collect();
            return Some(format!("Your rules (first match wins):\n{}", lines.join("\n")));
        }
        None
    }

    pub(crate) fn z15_route(&mut self, now: f64, text: &str) -> Option<String> {
        let lw = strip_low(text);
        if rx!(r"\bscreen ?time\b|what (?:site|website|app)s? (?:have i|did i|was i|am i)\b.*\blongest|(?:longest|most) (?:time )?(?:on )?(?:which|what) (?:site|app)|where did my (?:day|time) go").is_match(&lw) {
            if rx!(r"^(?:open |show )(?:me )?(?:my )?screen ?time").is_match(&lw) {
                self.later(now + 0.15, Later::Command("open_hub:screen".into()));
            }
            return Some(screen_summary(&self.life()));
        }
        if let Some(m) = rx!(r"^how long (?:have i been|was i|did i spend|have i spent) on ([\w .+&'-]+?)(?: today)?$").captures(&lw) {
            let want = m[1].trim().to_string();
            let r = screen_range(&self.life(), 1);
            let all = r.top.iter().map(|t| (t.0.clone(), t.1)).chain(r.sites.iter().cloned());
            for (name, v) in all {
                let nl = name.to_lowercase();
                if nl.contains(&want) || want.contains(&nl) {
                    return Some(format!("{name}: {} today~", screen_fmt_long(v)));
                }
            }
            return Some(format!("I haven't seen {want} today (and private windows never count)."));
        }
        if rx!(r"^(?:open |show )?(?:the |my |zenko'?s )?(?:world )?map$|^(?:where are we|where'?s zenko|our journey|journey)$").is_match(&lw) {
            self.later(now + 0.15, Later::Command("open_hub:map".into()));
            let (hrs, got) = journey_state(&self.life());
            let all = rdata()["JOURNEY_ALL"].as_array().unwrap();
            let place = |id: &str| all.iter().find(|p| p[0] == id).cloned();
            let cur = got.last().and_then(|id| place(id))
                .or_else(|| place(rdata()["JMAP_PLACES"][0].as_str().unwrap())).unwrap();
            let nxt = all.iter().find(|p| hrs < p[3].as_f64().unwrap_or(1e9));
            let tail = match nxt {
                Some(p) => format!("Next stop: {} in {:.1} focus hours.", p[2].as_str().unwrap(), p[3].as_f64().unwrap() - hrs),
                None => "We've visited every place!".into(),
            };
            return Some(format!("We're at {} {}~ {tail}", cur[1].as_str().unwrap(), cur[2].as_str().unwrap()));
        }
        if rx!(r"^(?:check for |any )?updates?\??$|^(?:check for|is there an?) (?:new )?(?:update|version)").is_match(&lw) {
            let src = match self.cfg.raw.get("update_source") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                _ => routes_data()["UPDATE_SOURCE_DEFAULT"].as_str().unwrap().into(),
            };
            if src.trim().is_empty() {
                return Some("Updates aren't set up yet: Settings > Updates, then add your GitHub name (like yourname/zenko).".into());
            }
            self.later(now, Later::Command("update_check_now".into()));
            return Some("Checking for a new version~".into());
        }
        if rx!(r"^(?:pause|stop) (?:tracking|screen ?time)").is_match(&lw) {
            let until = unix_now() + 3600.0;
            self.life_do(|l| {
                if !l["screen"].is_object() {
                    l["screen"] = json!({"days": {}});
                }
                l["screen"]["paused_until"] = json!(until);
            });
            self.say("Okay, I'm not looking for the next hour~ Your secrets are safe.", 6.0);
            return Some("Okay, screen time is paused for an hour~".into());
        }
        None
    }

    /// the end of Zenko.gym_route: the garden and the delay picker
    pub(crate) fn garden_route(&mut self, now: f64, text: &str) -> Option<String> {
        let lw = strip_low(text);
        if rx!(r"^(?:open |show )?(?:my )?(?:focus )?(?:garden|bonsai|niwa|庭)$|^how(?:'s| is) my (?:garden|tree|bonsai)$").is_match(&lw) {
            self.later(now + 0.15, Later::Command("open_hub:garden".into()));
            let g = self.life()["garden"].clone();
            let i = garden_stage(g["xp"].as_f64().unwrap_or(0.0));
            let st = &rdata()["GARDEN_STAGES"][i];
            return Some(format!("Your tree is a {} ({}) right now, vitality {}%~", st[3].as_str().unwrap().to_lowercase(),
                                st[1].as_str().unwrap(), (garden_health(&g) * 100.0) as i64));
        }
        if rx!(r"^(?:choose|pick|select) what to delay$|^delay (?:some|specific|certain) (?:things|items|tasks)$").is_match(&lw) {
            self.later(now + 0.1, Later::Command("delay_picker".into()));
            return Some("Tick what you want to push back~".into());
        }
        None
    }
}
