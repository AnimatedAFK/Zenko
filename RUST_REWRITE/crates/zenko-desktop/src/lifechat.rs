//! zenko.py ZenkoLife.life_route after the breathing and check-in lines: homework, an exam told in words ("my
//! chemistry exam is in 2 days"), free time ("I'm free 4-7pm on weekdays", zenko21 half-hour slots), the study plan,
//! "I need 4 hours for my chemistry exam", the countdown, flashcards + the spaced-repetition review quiz, remembering
//! you (facts and notes), background music, desktop toggles, walks, streak and badges, how she is, haiku and visitors.
//! Also the date words every chat command shares: parse_rel_date, find_dates, find_time, guess_kind, parse_free_time,
//! hours_text, add_deadline.

use crate::ema::fmt_hm;
use crate::music::ldata;
use crate::plan::{day_n, getrandbits36, getrandbits40, iso, load_deadlines, naive, parse_iso, save_deadlines, truthy};
use crate::rewards::{Later, rdata, days_from_civil};
use crate::routes17::routes_data;
use crate::study::when_text;
use crate::{Companion, Ev, read_json, unix_now};
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

pub const DAY_NAMES: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const WD3: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MON3: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September",
                            "October", "November", "December"];
const WD_KEYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
const MON_DAYW: &str = r"(?:(?:mon|tue|tues|wed|thu|thur|thurs|fri|sat|sun)(?:day|nesday|sday|urday)?,?\s+)?";
const MON_RE: &str = r"(jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|june?|july?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)";

pub fn weekday(day: i64) -> usize {
    (day + 3).rem_euclid(7) as usize
}

pub fn ymd(day: i64) -> (i64, i64, i64) {
    let (y, m, d) = crate::civil_from_days_pub(day);
    (y as i64, m as i64, d as i64)
}

/// d.strftime("%a %d %b")
pub fn a_d_b(day: i64) -> String {
    let (_, m, d) = ymd(day);
    format!("{} {d:02} {}", WD3[weekday(day)], MON3[m as usize - 1])
}

/// _mk_date: a real calendar day, or None
fn mk_date(y: i64, m: i64, d: i64) -> Option<i64> {
    if !(1..=12).contains(&m) || d < 1 || !(1..=9999).contains(&y) {
        return None;
    }
    let n = days_from_civil(y, m, d);
    (ymd(n) == (y, m, d)).then_some(n)
}

fn future_year(m: i64, d: i64, today: i64) -> Option<i64> {
    let (y, _, _) = ymd(today);
    let dt = mk_date(y, m, d);
    if let Some(x) = dt {
        if x < today - 45 {
            return mk_date(y + 1, m, d);
        }
    }
    dt
}

fn month_no(s: &str) -> i64 {
    let k: String = s.chars().take(3).collect::<String>().to_lowercase();
    MON3.iter().position(|m| m.to_lowercase() == k).map(|i| i as i64 + 1).unwrap_or(1)
}

/// find_dates(line, today, day_first): [(day, start, end)] for every date in a line (byte offsets)
pub fn find_dates(line: &str, today: i64, day_first: bool) -> Vec<(i64, usize, usize)> {
    let mut out: Vec<(i64, usize, usize)> = vec![];
    for m in rx!(r"\b(20\d\d)[-/.](\d{1,2})[-/.](\d{1,2})\b").captures_iter(line) {
        if let Some(d) = mk_date(m[1].parse().unwrap(), m[2].parse().unwrap(), m[3].parse().unwrap()) {
            let g = m.get(0).unwrap();
            out.push((d, g.start(), g.end()));
        }
    }
    for m in rx!(r"\b(\d{1,2})[/.-](\d{1,2})(?:[/.-](\d{2,4}))?\b").captures_iter(line) {
        let g = m.get(0).unwrap();
        if out.iter().any(|(_, s, e)| *s <= g.start() && g.start() < *e) {
            continue;
        }
        let (a, b): (i64, i64) = (m[1].parse().unwrap(), m[2].parse().unwrap());
        let y = m.get(3).map(|x| x.as_str());
        if y.is_none() && (g.as_str().contains('.') || a > 31 || b > 31) {
            continue; // "3.5" is a number, not a date
        }
        let (mut day, mut mon) = if day_first { (a, b) } else { (b, a) };
        if mon > 12 && day <= 12 {
            std::mem::swap(&mut day, &mut mon);
        }
        let d = match y {
            Some(y) => mk_date(y.parse::<i64>().unwrap() + if y.len() == 2 { 2000 } else { 0 }, mon, day),
            None => future_year(mon, day, today),
        };
        if let Some(d) = d {
            out.push((d, g.start(), g.end()));
        }
    }
    static P: OnceLock<[Regex; 2]> = OnceLock::new();
    let pats = P.get_or_init(|| [
        Regex::new(&format!(r"(?i)\b{MON_DAYW}(\d{{1,2}})(?:st|nd|rd|th)?\s+(?:of\s+)?{MON_RE}\.?,?(?:\s+(20\d\d))?\b")).unwrap(),
        Regex::new(&format!(r"(?i)\b{MON_DAYW}{MON_RE}\.?\s+(\d{{1,2}})(?:st|nd|rd|th)?,?(?:\s+(20\d\d))?\b")).unwrap(),
    ]);
    for (k, pat) in pats.iter().enumerate() {
        for m in pat.captures_iter(line) {
            let g = m.get(0).unwrap();
            if out.iter().any(|(_, s, e)| (*s <= g.start() && g.start() < *e) || (*s < g.end() && g.end() <= *e)) {
                continue;
            }
            let (day, mon) = if k == 0 { (m[1].parse::<i64>().unwrap(), month_no(&m[2])) } else { (m[2].parse::<i64>().unwrap(), month_no(&m[1])) };
            let d = match m.get(3) {
                Some(y) => mk_date(y.as_str().parse().unwrap(), mon, day),
                None => future_year(mon, day, today),
            };
            if let Some(d) = d {
                out.push((d, g.start(), g.end()));
            }
        }
    }
    if out.is_empty() {
        if let Some(m) = rx!(r"(?i)\b(today|tonight|tomorrow|next (?:mon|tues|wednes|thurs|fri|satur|sun)day)\b").captures(line) {
            let w = m[1].to_lowercase();
            let d = if w == "today" || w == "tonight" {
                today
            } else if w == "tomorrow" {
                today + 1
            } else {
                let t = WD_KEYS.iter().position(|k| w.split(' ').nth(1).unwrap().starts_with(k)).unwrap() as i64;
                let a = (t - weekday(today) as i64).rem_euclid(7);
                today + if a == 0 { 7 } else { a }
            };
            let g = m.get(0).unwrap();
            out.push((d, g.start(), g.end()));
        }
    }
    out.sort_by_key(|x| x.1);
    out
}

/// find_time(line): ("HH:MM", its span) or ("", None)
pub fn find_time(line: &str) -> (String, Option<(usize, usize)>) {
    let Some(m) = rx!(r"(?i)\b(\d{1,2})(?::(\d{2}))?\s*(am|pm)\b|\b([01]?\d|2[0-3]):([0-5]\d)\b").captures(line) else {
        return (String::new(), None);
    };
    let g = m.get(0).unwrap();
    if let Some(ap) = m.get(3) {
        let h = m[1].parse::<i64>().unwrap() % 12 + if ap.as_str().to_lowercase() == "pm" { 12 } else { 0 };
        let mi: i64 = m.get(2).map(|x| x.as_str().parse().unwrap()).unwrap_or(0);
        return (format!("{h:02}:{mi:02}"), Some((g.start(), g.end())));
    }
    (format!("{:02}:{}", m[4].parse::<i64>().unwrap(), &m[5]), Some((g.start(), g.end())))
}

/// guess_kind(title)
pub fn guess_kind(title: &str) -> String {
    let low = title.to_lowercase();
    for (w, k) in routes_data()["DEADLINE_WORDS"].as_object().unwrap() {
        if Regex::new(&format!(r"\b{}", regex::escape(w))).unwrap().is_match(&low) {
            return k.as_str().unwrap().into();
        }
    }
    "Deadline".into()
}

fn word_num(s: &str) -> Option<i64> {
    Some(match s {
        "a" | "an" | "one" => 1,
        "two" | "couple of" | "a couple of" => 2,
        "three" | "few" | "a few" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        _ => return None,
    })
}

/// parse_rel_date(text, today)
pub fn parse_rel_date(text: &str, today: i64) -> Option<i64> {
    let low = text.to_lowercase();
    if let Some(m) = rx!(r"\bin\s+(\d{1,3}|a couple of|a few|couple of|few|an?|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s+(day|days|week|weeks)\b").captures(&low) {
        let n = m[1].parse::<i64>().ok().or_else(|| word_num(&m[1])).unwrap_or(1);
        return Some(today + n * if m[2].starts_with("week") { 7 } else { 1 });
    }
    if rx!(r"\b(?:the )?day after tomorrow\b").is_match(&low) {
        return Some(today + 2);
    }
    let m = rx!(r"\b(?:on |this |next |coming )?(mon|tues?|wed(?:nes)?|thu(?:rs?)?|fri|sat(?:ur)?|sun)(?:day)?\b").captures(&low);
    let ds = find_dates(text, today, true);
    if let Some(d) = ds.first() {
        return Some(d.0);
    }
    if let Some(m) = m {
        let k: String = m[1].chars().take(3).collect();
        let t = WD_KEYS.iter().position(|w| *w == k).unwrap() as i64;
        let a = (t - weekday(today) as i64).rem_euclid(7);
        return Some(today + if a == 0 { 7 } else { a });
    }
    if rx!(r"\btomorrow\b").is_match(&low) {
        return Some(today + 1);
    }
    if rx!(r"\btoday\b|\btonight\b").is_match(&low) {
        return Some(today);
    }
    None
}

/// parse_hour(s, pm_hint)
fn parse_hour(s: &str, pm_hint: bool) -> Option<i64> {
    let m = rx!(r"(?i)^\s*(\d{1,2})(?:[:.](\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)?\s*$").captures(s)?;
    let h: i64 = m[1].parse().unwrap();
    let ap = m.get(3).map(|a| a.as_str().to_lowercase().replace('.', "")).unwrap_or_default();
    if !ap.is_empty() {
        if !(1..=12).contains(&h) {
            return None;
        }
        return Some(h % 12 + if ap == "pm" { 12 } else { 0 });
    }
    if h > 24 {
        return None;
    }
    if pm_hint && (1..=11).contains(&h) {
        return Some(h + 12);
    }
    Some(h % 24)
}

/// parse_free_time(text, today): (day keys "0".."6" or a date, hours)
pub fn parse_free_time(text: &str, today: i64) -> Option<(Vec<String>, Vec<i64>)> {
    let low = text.to_lowercase();
    let low = low.trim_matches([' ', '.', '!', '~']);
    if !rx!(r"\b(?:free|available|can study|study time)\b").is_match(low) {
        return None;
    }
    let m = rx!(r"(?:from\s+|between\s+)?(\d{1,2}(?:[:.]\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)?)\s*(?:-|–|to|till|until|and)\s*(\d{1,2}(?:[:.]\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)?)").captures(low)?;
    let (a_raw, b_raw) = (m[1].to_string(), m[2].to_string());
    let g0 = m.get(0).unwrap();
    let mut pm = rx!(r"pm|p\.m\.").is_match(&b_raw) || rx!(r"\b(evening|afternoon|tonight|night)\b").is_match(low);
    if !rx!(r"am|pm|a\.m\.|p\.m\.|:").is_match(g0.as_str()) {
        if let Some(d) = rx!(r"^\s*(\d+)").captures(&a_raw) {
            if d[1].parse::<i64>().unwrap_or(99) < 8 {
                pm = true; // "free 3 to 6" means the afternoon, not 3 am
            }
        }
    }
    let b = parse_hour(&b_raw, pm);
    let a = parse_hour(&a_raw, pm && !rx!(r"am|a\.m\.").is_match(&a_raw));
    let (a, mut b) = (a?, b?);
    if b <= a && b + 12 > a && b < 12 {
        b += 12;
    }
    if b <= a {
        return None;
    }
    let hours: Vec<i64> = (a..24.min(b)).collect();
    let keys: Vec<String> = if rx!(r"\b(?:weekdays?|school days?|mon(?:day)?\s*(?:-|to)\s*fri(?:day)?)\b").is_match(low) {
        (0..5).map(|i| i.to_string()).collect()
    } else if rx!(r"\bweekends?\b").is_match(low) {
        vec!["5".into(), "6".into()]
    } else if rx!(r"\b(?:every ?day|daily|all week|each day)\b").is_match(low) {
        (0..7).map(|i| i.to_string()).collect()
    } else {
        let mut days = vec![];
        for (i, nm) in WD_KEYS.iter().enumerate() {
            if Regex::new(&format!(r"\b(?:every |on )?{nm}\w*days?\b")).unwrap().is_match(low) && rx!(r"\bevery\b|days\b").is_match(low) {
                days.push(i.to_string());
            }
        }
        if !days.is_empty() {
            days
        } else {
            let rest = format!("{} {}", &low[..g0.start()], &low[g0.end()..]);
            vec![iso(parse_rel_date(&rest, today).unwrap_or(today))]
        }
    };
    Some((keys, hours))
}

/// hours_text([16, 17, 18, 20]) -> '16:00-19:00, 20:00-21:00'
pub fn hours_text(hours: &[i64]) -> String {
    let mut hs: Vec<i64> = hours.to_vec();
    hs.sort();
    hs.dedup();
    if hs.is_empty() {
        return "none".into();
    }
    let mut runs = vec![];
    let (mut a, mut prev) = (hs[0], hs[0]);
    for h in hs[1..].iter().map(|h| Some(*h)).chain([None]) {
        if let Some(h) = h {
            if h == prev + 1 {
                prev = h;
                continue;
            }
        }
        runs.push(format!("{a:02}:00-{:02}:00", prev + 1));
        if let Some(h) = h {
            a = h;
            prev = h;
        }
    }
    runs.join(", ")
}

fn s_of(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Python's str.capitalize()
fn capitalize(s: &str) -> String {
    crate::chat::capitalize(s)
}

/// The review quiz (self.quiz)
#[derive(Clone, Debug)]
pub struct Quiz {
    pub cards: Vec<Value>,
    pub i: usize,
    pub score: i64,
    pub topic: String,
    pub t: f64,
    pub srs: Option<bool>,
    pub saved: Option<i64>,
}

/// answer_close(given, right)
pub fn answer_close(given: &str, right: &str) -> bool {
    let clean = |s: &str| -> String {
        let l = s.to_lowercase();
        rx!(r"[^a-z0-9 ]").replace_all(&l, "").trim().to_string()
    };
    let (g, r) = (clean(given), clean(right));
    if g.is_empty() {
        return false;
    }
    if g == r || (g.chars().count() >= 4 && (r.contains(&g) || g.contains(&r))) {
        return true;
    }
    crate::pydifflib::ratio(&g, &r) >= 0.78
}

const SRS_DAYS: [i64; 7] = [0, 1, 2, 4, 8, 16, 35];

impl Companion {
    fn today(&self) -> i64 {
        day_n(naive(unix_now()))
    }

    /// the quiz as tests see it (zenko.py's self.quiz keys)
    pub fn quiz_state(&self) -> Value {
        match &self.quiz {
            None => Value::Null,
            Some(q) => json!({"cards": q.cards, "i": q.i, "score": q.score, "topic": q.topic, "srs": q.srs, "saved": q.saved}),
        }
    }

    /// add_deadline(title, date, time, kind, source)
    pub fn add_deadline(&mut self, now: f64, title: &str, date_iso: &str, time_s: &str, kind: Option<&str>, source: &str) {
        let mut items = load_deadlines();
        let id = format!("{:x}", getrandbits40(&mut self.brain.rng));
        items.push(json!({"id": id, "title": title.trim().chars().take(90).collect::<String>(), "date": date_iso,
                          "time": time_s, "kind": kind.map(String::from).unwrap_or_else(|| guess_kind(title)),
                          "source": source, "done": false}));
        save_deadlines(&mut items);
        self.later(now + 0.2, Later::PlanMaybe);
    }

    /// plan_summary(only, n)
    pub fn plan_summary(&self, only: Option<&str>, n: usize) -> String {
        let today = self.today();
        let ts = iso(today);
        let mut rows: Vec<&Value> = self.tt.iter().filter(|i| truthy(&i["plan"]) && !truthy(&i["done"])
            && only.is_none_or(|o| i["plan"].as_str() == Some(o)) && i["date"].as_str().unwrap_or("") >= ts.as_str()).collect();
        rows.sort_by(|a, b| (s_of(&a["date"]), s_of(&a["start"])).cmp(&(s_of(&b["date"]), s_of(&b["start"]))));
        if rows.is_empty() {
            return String::new();
        }
        let mut out = vec![];
        for it in rows.iter().take(n) {
            let d = parse_iso(&s_of(&it["date"])).unwrap_or(today);
            let w = if d - today <= 1 {
                capitalize(&when_text(&s_of(&it["date"]), today))
            } else {
                let (_, _, dd) = ymd(d);
                format!("{} {dd:02}", WD3[weekday(d)])
            };
            out.push(format!("\u{2022} {w} {}  {} ({} min)", s_of(&it["start"]), s_of(&it["task"]), it["mins"].as_i64().unwrap_or(0)));
        }
        if rows.len() > n {
            out.push(format!("  ...and {} more on your timetable", rows.len() - n));
        }
        out.join("\n")
    }

    fn plan_short(&self) -> Value {
        read_json("zenko_life.json")["plan_short"].clone()
    }

    /// plan_reply()
    pub fn plan_reply(&mut self, now: f64) -> String {
        let (targets, _) = self.replan();
        if targets.is_empty() {
            return "Nothing to plan for~ Tell me about an exam, like 'my chemistry exam is in 2 days'.".into();
        }
        self.award(now, "planner");
        let short = self.plan_short();
        let life = read_json("zenko_life.json");
        let today = self.today();
        let ts = iso(today);
        let mut lines = vec![];
        for t in targets.iter().take(5) {
            let n = self.tt.iter().filter(|i| i["plan"].as_str() == Some(&t.id) && !truthy(&i["done"])
                && i["date"].as_str().unwrap_or("") >= ts.as_str()).count();
            let done = life["studied"].get(&t.id).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let date = iso(day_n(t.due));
            lines.push(format!("\u{2022} {}, {}: {n} session{} planned{}{}", t.title, when_text(&date, today),
                               if n == 1 { "" } else { "s" },
                               if done != 0.0 { format!(", {} already done", fmt_hm(done as i64)) } else { String::new() },
                               match short.get(&t.id) {
                                   Some(v) => format!("  (uu~ {} didn't fit in your free hours)", fmt_hm(v.as_f64().unwrap_or(0.0) as i64)),
                                   None => String::new(),
                               }));
        }
        let body = self.plan_summary(None, 8);
        let tip = if truthy(&life["free_set"]) { "" } else {
            "\n(I used my guess of your free hours. Mark your real ones on the Upcoming clock, or say 'I'm free 4-7pm on weekdays'.)"
        };
        self.later(now + 0.3, Later::Command("open_hub:deadlines".into()));
        format!("Kon! Here's your study plan~\n{}{}{tip}", lines.join("\n"),
                if body.is_empty() { String::new() } else { format!("\n\nNext up:\n{body}") })
    }

    /// set_free(keys, hours) (zenko21: hours become half-hour slots)
    pub fn set_free(&mut self, keys: &[String], hours: &[i64]) {
        let slots = crate::rewards::hours_to_slots(&json!(hours));
        let today = iso(self.today());
        self.life_do(|l| {
            for k in keys {
                let mut v = slots.clone();
                v.sort_by(|a, b| a.partial_cmp(b).unwrap());
                v.dedup();
                let arr = Value::Array(v.into_iter().map(|x| json!(x)).collect());
                if k.contains('-') {
                    l["free_dates"][k.as_str()] = arr;
                } else {
                    l["free"][k.as_str()] = arr;
                }
            }
            l["free_set"] = json!(true);
            if let Some(o) = l["free_dates"].as_object_mut() {
                o.retain(|k, _| k.as_str() >= today.as_str());
            }
        });
    }

    /// countdown_text()
    pub fn countdown_text(&self) -> String {
        let ups = self.upcoming_deadlines(8);
        if ups.is_empty() {
            return "No exams coming up! Tell me one: 'my maths exam is on Dec 10'.".into();
        }
        let today = self.today();
        let ts = iso(today);
        let mut out = vec!["Countdown~".to_string()];
        for d in ups.iter().take(6) {
            let days = parse_iso(&s_of(&d["date"])).unwrap_or(today) - today;
            let todays: i64 = self.tt.iter().filter(|i| i["plan"] == d["id"] && !d["id"].is_null() && i["date"].as_str() == Some(&ts))
                .map(|i| i["mins"].as_i64().unwrap_or(0)).sum();
            let goal = if todays != 0 { format!("  \u{b7}  today's goal: {}", fmt_hm(todays)) } else { String::new() };
            let when = match days {
                0 => "TODAY!".to_string(),
                1 => "tomorrow".to_string(),
                n => format!("{n} days"),
            };
            out.push(format!("\u{2022} {}: {when}{goal}", s_of(&d["title"])));
        }
        out.join("\n")
    }

    /// exam_from_text(raw, d)
    pub fn exam_from_text(&mut self, now: f64, raw: &str, d: i64) -> String {
        let low = raw.to_lowercase();
        let km = rx!(r"\b(exams?|tests?|quiz(?:zes)?|midterms?|finals?|assessments?|mocks?)\b").captures(&low);
        let kind_w = km.as_ref().map(|k| k[1].trim_end_matches('s').to_string()).unwrap_or("exam".into());
        let kind_w = match kind_w.as_str() {
            "quizze" | "quizz" => "quiz".to_string(),
            "final" => "final exam".into(),
            "mock" => "mock exam".into(),
            "midterm" => "midterm exam".into(),
            _ => kind_w,
        };
        let (ks, ke) = km.as_ref().map(|k| (k.get(0).unwrap().start(), k.get(0).unwrap().end())).unwrap_or((0, 0));
        // low and raw have the same byte offsets for ASCII; fall back to char positions otherwise
        let (before, after) = if km.is_some() && low.len() == raw.len() {
            (raw[..ks].to_string(), raw[ke..].to_string())
        } else if km.is_some() {
            let cs = low[..ks].chars().count();
            let ce = low[..ke].chars().count();
            (raw.chars().take(cs).collect(), raw.chars().skip(ce).collect())
        } else {
            (String::new(), String::new())
        };
        let subj = rx!(r"(?i)^(?:hey|hi|so|ok|okay|yo|um|uh|zenko|guess what)\b[\s,!]*").replace(before.trim(), "").into_owned();
        let subj = rx!(r"(?i)\b(?:i(?:'ve| have)?|have|got|there'?s|is|my|a|an|the|big|next|an?other|gonna have|will have|have got)\b").replace_all(&subj, " ").into_owned();
        let subj = rx!(r"[^\w &+\-]").replace_all(&subj, " ").into_owned();
        let mut subj = rx!(r"\s+").replace_all(&subj, " ").trim_matches([' ', '-']).to_string();
        if subj.is_empty() {
            if let Ok(Some(m)) = frx!(r"(?i)\b(?:for|on|in)\s+([a-z][a-z &+\-]{2,30}?)(?=\s+(?:is|on|in|at|this|next|tomorrow|today)\b|[,.!]|$)").captures(&after) {
                let g = m.get(1).unwrap().as_str();
                if !rx!(r"^(?:\d|a |an |one|two|three|monday|tuesday|wednesday|thursday|friday|saturday|sunday|jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec|the morning|the afternoon)").is_match(&g.to_lowercase()) {
                    subj = g.trim().to_string();
                }
            }
        }
        let w: Vec<&str> = subj.split_whitespace().collect();
        let subj = w[w.len().saturating_sub(4)..].join(" ");
        let title = upper_first(&format!("{}{kind_w}", if subj.is_empty() { String::new() } else { format!("{subj} ") }));
        let (t, _) = find_time(&after);
        let mut kind = guess_kind(&title);
        if kind == "Deadline" {
            kind = "Exam".into();
        }
        let di = iso(d);
        let exists = load_deadlines().iter().any(|it| it["date"].as_str() == Some(&di)
            && s_of(&it["title"]).to_lowercase() == title.to_lowercase() && !truthy(&it["done"]));
        if !exists {
            self.add_deadline(now, &title, &di, &t, Some(&kind), "chat");
        }
        let (_, sessions) = self.replan();
        self.award(now, "planner");
        let mut tid = None;
        for it in load_deadlines() {
            if it["date"].as_str() == Some(&di) && s_of(&it["title"]).to_lowercase() == title.to_lowercase() {
                tid = it["id"].as_str().map(String::from);
            }
        }
        let mine = sessions.iter().filter(|s| Some(&s.id) == tid.as_ref()).count();
        let today = self.today();
        let days = d - today;
        let head = format!("Kon! {title} {} ({}){}. ", if days == 0 { "is TODAY".to_string() } else { format!("on {}", a_d_b(d)) },
                           when_text(&di, today), if t.is_empty() { String::new() } else { format!(" at {t}") });
        let body = if mine > 0 {
            let mut b = format!("I planned {mine} study session{} for it in your free time:\n{}", if mine == 1 { "" } else { "s" },
                                self.plan_summary(tid.as_deref(), 6));
            if let Some(v) = tid.as_ref().and_then(|id| self.plan_short().get(id).cloned()) {
                b += &format!("\nUu~ {} more didn't fit. Mark more free hours on the Upcoming clock?", fmt_hm(v.as_f64().unwrap_or(0.0) as i64));
            }
            b
        } else {
            "But I couldn't find free time before it! Tell me when you're free ('I'm free 5-8pm today') or tap hours on the clock.".into()
        };
        let tip = if truthy(&read_json("zenko_life.json")["free_set"]) { "" } else {
            "\n(I guessed your free hours. Set yours on the Upcoming tab's clock, or say 'I'm free 4-7pm on weekdays'.)"
        };
        self.later(now + 0.4, Later::Command("open_hub:deadlines".into()));
        self.set_mood(now, "happy", 4.0);
        format!("{head}{body}{tip}")
    }

    fn cards(&self) -> Vec<Value> {
        read_json("zenko_life.json")["cards"].as_array().cloned().unwrap_or_default()
    }

    /// cards_due()
    pub fn cards_due(&self) -> Vec<Value> {
        let ts = iso(self.today());
        self.cards().into_iter().filter(|c| c["due"].as_str().unwrap_or("") <= ts.as_str()).collect()
    }

    /// review_start()
    pub fn review_start(&mut self) -> String {
        let mut due = self.cards_due();
        if due.is_empty() {
            let n = self.cards().len();
            return if n > 0 {
                format!("No cards due today~ Yatta! {n} card{} in your deck. Add one: 'add flashcard mitochondria = powerhouse of the cell'",
                        if n == 1 { "" } else { "s" })
            } else {
                "Your deck is empty! Add cards with 'add flashcard Q = A', or say 'quiz me on <topic>': I save the ones you miss.".into()
            };
        }
        let num = |v: &Value| v.as_f64().unwrap_or(0.0);
        due.sort_by(|a, b| (num(&a["box"]), -num(&a["wrong"])).partial_cmp(&(num(&b["box"]), -num(&b["wrong"]))).unwrap());
        let cards: Vec<Value> = due.iter().take(15).map(|c| json!({"q": c["q"], "a": c["a"], "id": c["id"]})).collect();
        let n = cards.len();
        let first = s_of(&cards[0]["q"]);
        self.quiz = Some(Quiz { cards, i: 0, score: 0, topic: "your review deck".into(), t: unix_now(), srs: Some(true), saved: None });
        format!("Kon! {n} card{} due. Type your answer, or 'hint', 'skip', 'stop quiz'.\n\nQ1/{n}: {first}", if n == 1 { "" } else { "s" })
    }

    /// card_add(q, a)
    pub fn card_add(&mut self, q: &str, a: &str) -> String {
        let q: String = q.trim_matches([' ', '.']).chars().take(200).collect();
        let a: String = a.trim_matches([' ', '.']).chars().take(200).collect();
        if q.is_empty() || a.is_empty() {
            return "Write it like: add flashcard capital of Japan = Tokyo".into();
        }
        let id = format!("{:x}", getrandbits36(&mut self.brain.rng));
        let today = iso(self.today());
        let (q2, a2) = (q.clone(), a.clone());
        self.life_do(|l| {
            if let Some(c) = l["cards"].as_array_mut() {
                c.push(json!({"id": id, "q": q2, "a": a2, "box": 0, "wrong": 0, "right": 0, "added": today, "due": today}));
            }
        });
        format!("Added to your deck: \u{201c}{}\u{201d} \u{2192} {}. Say 'review flashcards' when you're ready~",
                crate::extra::clip(&q, 60), crate::extra::clip(&a, 40))
    }

    /// srs_update(card, ok, from_deck)
    pub(crate) fn srs_update(&mut self, now: f64, card: &Value, ok: bool, topic: &str) {
        let today = self.today();
        let mut cards = self.cards();
        let pos = cards.iter().position(|c| (truthy(&card["id"]) && c["id"] == card["id"]) || (c["q"] == card["q"] && c["a"] == card["a"]));
        let k = match pos {
            Some(k) => k,
            None => {
                if ok {
                    return; // quiz cards you got right aren't saved; misses are
                }
                let id = format!("{:x}", getrandbits36(&mut self.brain.rng));
                cards.push(json!({"id": id, "q": card["q"], "a": card["a"], "box": 0, "wrong": 0, "right": 0,
                                  "added": iso(today), "topic": topic}));
                cards.len() - 1
            }
        };
        let c = &mut cards[k];
        let gi = |v: &Value| v.as_i64().unwrap_or(0);
        if ok {
            c["box"] = json!((SRS_DAYS.len() as i64 - 1).min(gi(&c["box"]) + 1));
            c["right"] = json!(gi(&c["right"]) + 1);
        } else {
            c["box"] = json!(0);
            c["wrong"] = json!(gi(&c["wrong"]) + 1);
        }
        let b = SRS_DAYS[gi(&c["box"]) as usize];
        c["due"] = json!(iso(today + if b == 0 { 1 } else { b }));
        self.life_do(|l| l["cards"] = Value::Array(cards));
        if self.stat_add("reviews", 1) >= 50 {
            self.award(now, "cards50");
        }
    }

    /// quiz_answer(text): ZenkoExtras' quiz plus ZenkoLife's spaced repetition
    pub fn quiz_answer(&mut self, now: f64, text: &str) -> Option<String> {
        let mut q = self.quiz.clone()?;
        let (i0, s0) = (q.i, q.score);
        let card = q.cards.get(i0).cloned();
        let low = text.to_lowercase();
        let low = low.trim_matches([' ', '!', '.', '?']);
        let r: String = 'base: {
            if rx!(r"^(stop|end|quit|exit|cancel)( the)? quiz|^no more quiz").is_match(low) {
                self.quiz = None;
                break 'base format!("Quiz over! You got {} of {}. Otsukare~kon!", q.score, q.i);
            }
            let card = &q.cards[q.i];
            let a = s_of(&card["a"]);
            if ["hint", "a hint", "give me a hint"].contains(&low) {
                break 'base format!("Hint: it starts with '{}' and is {} characters long~", a.chars().take(2).collect::<String>(),
                                    a.replace(' ', "").chars().count());
            }
            q.t = unix_now();
            let skip = ["skip", "next", "pass", "idk", "i don't know", "i dont know", "show answer", "answer", "?"].contains(&low);
            let ok = !skip && answer_close(text, &a);
            let out = if ok {
                q.score += 1;
                self.events.push(Ev::Beep { kind: "happy".into(), variety: false });
                format!("{} ({a})", self.brain.rng.choice(&["Correct! Yatta~!", "Sugoi! That's right!", "Kon kon~ perfect!"]))
            } else if skip {
                format!("The answer was: {a}")
            } else {
                format!("Ne~ not quite. It was: {a}")
            };
            q.i += 1;
            if q.i >= q.cards.len() {
                self.quiz = None;
                let (sc, n) = (q.score, q.cards.len() as i64);
                let verdict = if sc == n { "Perfect score!! Sugoi~!" } else if sc as f64 >= n as f64 / 2.0 { "Great job~kon!" } else {
                    "Let's review and try again, ne~"
                };
                break 'base format!("{out}\n\nDone! You got {sc} of {n}. {verdict}");
            }
            self.quiz = Some(q.clone());
            format!("{out}\n\nQ{}/{}: {}", q.i + 1, q.cards.len(), s_of(&q.cards[q.i]["q"]))
        };
        let mut r = r;
        if let Some(card) = card {
            if q.i == i0 + 1 {
                let ok = q.score > s0;
                let srs = q.srs.unwrap_or(false);
                let topic = self.quiz.as_ref().map(|x| x.topic.clone()).unwrap_or_default();
                self.srs_update(now, &card, ok, &topic);
                if !ok && !srs {
                    q.saved = Some(q.saved.unwrap_or(0) + 1);
                    if self.quiz.is_some() {
                        self.quiz = Some(q.clone());
                    }
                }
                if self.quiz.is_none() && q.saved.unwrap_or(0) != 0 {
                    r += &format!("\n(I saved the {} you missed to your flashcard deck. Say 'review flashcards' tomorrow~)", q.saved.unwrap());
                }
            }
        }
        Some(r)
    }

    /// fact_set(key, val)
    fn fact_set(&mut self, key: &str, val: &str) -> String {
        let key = rx!(r"\s+").replace_all(key.to_lowercase().trim_matches([' ', '.', '!']), " ").chars().take(40).collect::<String>();
        let key = key.replace("favorite", "favourite").replace("color", "colour");
        let val: String = val.trim_matches([' ', '.', '!', '~']).chars().take(120).collect();
        if key == "birthday" {
            let ds = find_dates(&val, self.today(), true);
            let Some(d) = ds.first() else { return "Ehh? When is it? Like 'my birthday is 14 May'.".into() };
            let (_, m, dd) = ymd(d.0);
            let v = format!("{m:02}-{dd:02}");
            self.life_do(|l| l["facts"]["birthday"] = json!(v));
            return format!("Kon! Your birthday is {dd:02} {}~ I'll remember, and I'll have a surprise ready!", MONTHS[m as usize - 1]);
        }
        let (k2, v2) = (key.clone(), val.clone());
        self.life_do(|l| l["facts"][k2.as_str()] = json!(v2));
        let f = *self.brain.rng.choice(&["Kon! I'll remember that your {k} is {v}~", "Fumu fumu~ your {k} is {v}. Noted in my fox memory!",
                                         "Ehehe~ got it: {k} = {v}. I won't forget!"]);
        f.replace("{k}", &key).replace("{v}", &val)
    }

    /// facts_text()
    fn facts_text(&self) -> String {
        let l = read_json("zenko_life.json");
        let f = l["facts"].as_object().cloned().unwrap_or_default();
        let notes: Vec<String> = l["notes"].as_array().map(|a| a.iter().map(s_of).collect()).unwrap_or_default();
        if f.is_empty() && notes.is_empty() {
            return "I don't know much about you yet~ Tell me! Like 'my favourite subject is biology' or 'remember that I play piano'.".into();
        }
        let mut out = vec!["Things I remember about you~".to_string()];
        for (k, v) in &f {
            let mut v = s_of(v);
            if k == "birthday" {
                let p: Vec<&str> = v.split('-').collect();
                if let (Some(m), Some(d)) = (p.first().and_then(|x| x.parse::<i64>().ok()), p.get(1).and_then(|x| x.get(..2)).and_then(|x| x.parse::<i64>().ok())) {
                    if mk_date(2000, m, d).is_some() {
                        v = format!("{d:02} {}", MONTHS[m as usize - 1]);
                    }
                }
            }
            out.push(format!("\u{2022} your {k}: {v}"));
        }
        for n in &notes[notes.len().saturating_sub(8)..] {
            out.push(format!("\u{2022} {n}"));
        }
        out.push("(Say 'forget my <thing>' to make me forget one.)".into());
        out.join("\n")
    }

    /// music_set(on, track)
    pub fn music_set(&mut self, on: Option<bool>, track: Option<&str>) {
        crate::update_config(&mut self.cfg, |c| {
            if let Some(o) = on {
                c["bgm"] = json!(o);
            }
            if let Some(t) = track {
                c["bgm_track"] = json!(t);
            }
        });
        self.music_apply();
    }

    /// mood_word()
    pub fn mood_word(&self) -> &'static str {
        let s = self.spirit();
        if s >= 75 { "Happy" } else if s >= 50 { "Content" } else if s >= 30 { "Sleepy" } else { "Pouty" }
    }

    /// needs_text()
    pub fn needs_text(&self) -> String {
        let h = self.hunger_now();
        let tummy = if h < 25.0 { "full" } else if h < 55.0 { "peckish" } else if h < 80.0 { "hungry" } else { "starving" };
        let st = crate::rewards::streak();
        format!("Mood: {} (foxfire {}%)  \u{b7}  Tummy: {tummy}  \u{b7}  Streak: {st} day{}", self.mood_word(), self.spirit(),
                if st == 1 { "" } else { "s" })
    }

    /// life_route's lines after the breathing and the check-in
    pub(crate) fn life_route_rest(&mut self, now: f64, raw: &str) -> Option<String> {
        let low = raw.to_lowercase();
        let low = low.trim();
        let lw = low.trim_end_matches([' ', '!', '.', '?', '~']);
        let today = self.today();
        let hm = rx!(r"(?is)^(?:homework|hw|hwk|home work)\s*(?:help)?\s*[:\-]\s*(.+)$").captures(raw)
            .or_else(|| rx!(r"(?is)^(?:help me (?:with|solve|understand)|how do (?:i|you) solve|solve|explain(?: to me)?)\s+(.+?)(?:\s+step by step)?\s*[?.]*$").captures(raw));
        if let Some(m) = hm {
            if !rx!(r"\b(?:that|the|this|my) (?:file|document|pdf|notes?)\b|\bnotes\b").is_match(low) {
                let q = m[1].trim().to_string();
                if q.chars().count() < 3 {
                    return Some("Paste the question after 'homework:' and I'll explain it step by step~".into());
                }
                let sid = self.subject_in(&q);
                return Some(self.nb_ask_chat(now, sid, &q, "explain"));
            }
        }
        // exam told in words: "my chemistry exam is in 2 days", "exam on friday at 9am"
        if rx!(r"\b(exams?|tests?|quiz(?:zes)?|midterms?|finals?|assessments?|mocks?)\b").is_match(low) && !raw.contains('?')
            && !rx!(r"^(when|what|how|which|is|are|do|does|show|list|open|delete|remove|quiz me)\b").is_match(lw) {
            if let Some(d) = parse_rel_date(low, today) {
                if d >= today {
                    return Some(self.exam_from_text(now, raw, d));
                }
            }
        }
        if let Some((keys, hours)) = parse_free_time(raw, today) {
            if rx!(r"\bi(?:'?m| am)?\b|^free\b|\bmy free\b|\bstudy time\b").is_match(low) {
                self.set_free(&keys, &hours);
                let wher = if !keys[0].contains('-') {
                    format!("on {}", keys.iter().map(|k| format!("{}s", DAY_NAMES[k.parse::<usize>().unwrap()])).collect::<Vec<_>>().join(", "))
                } else {
                    when_text(&keys[0], today)
                };
                self.replan();
                return Some(format!("Kon! Free {wher}: {}. I re-planned your study sessions to fit~ (You can also tap hours on the Upcoming clock.)",
                                    hours_text(&hours)));
            }
        }
        if rx!(r"\b(study plan|plan my (?:study|studies|studying|revision|exams?|week)|re-?plan|make (?:me )?a (?:study )?plan|schedule my (?:study|revision)|plan (?:my )?(?:study )?sessions)\b").is_match(low) {
            return Some(self.plan_reply(now));
        }
        if let Some(m) = rx!(r"\b(?:i need|i want|give me|plan)\s+(\d{1,2}(?:\.\d)?)\s*(?:h|hrs?|hours?)\s+(?:of (?:study|revision) )?(?:for|on)\s+(?:my |the )?(.+)$").captures(lw) {
            let want: f64 = m[1].parse().unwrap();
            let name = m[2].trim().to_string();
            let first = name.split_whitespace().next().unwrap_or("").to_string();
            let mut best: Option<Value> = None;
            let mut sc = 0.0;
            for d in self.upcoming_deadlines(20) {
                let tl = s_of(&d["title"]).to_lowercase();
                let s = crate::pydifflib::ratio(&name, &tl) + if tl.contains(&first) { 0.5 } else { 0.0 };
                if s > sc {
                    best = Some(d);
                    sc = s;
                }
            }
            if let Some(b) = best.filter(|b| sc >= 0.5 && truthy(&b["id"])) {
                let id = s_of(&b["id"]);
                let id2 = id.clone();
                self.life_do(|l| l["plan_hours"][id2.as_str()] = json!(want));
                self.replan();
                return Some(format!("Okay~ {} of study for {}. Plan updated!\n{}", fmt_hm((want * 60.0) as i64), s_of(&b["title"]),
                                    self.plan_summary(Some(&id), 5)));
            }
            return Some(format!("Which exam is that? I couldn't match '{name}'."));
        }
        if rx!(r"\bcountdown\b|how (?:many days|long) (?:until|till|before|left)|days (?:left|until|till)").is_match(low) {
            return Some(self.countdown_text());
        }
        // flashcards
        let fm = rx!(r"(?i)^(?:add |new |make a? ?)?(?:flash ?card|card)s?\s*[:\-]?\s*(.+?)\s*(?:=|->|=>|::| - )\s*(.+)$").captures(raw);
        if let Some(m) = &fm {
            if rx!(r"^(?:add |new |make)").is_match(lw) || lw.starts_with("flashcard") || lw.starts_with("flash card") {
                return Some(self.card_add(&m[1], &m[2]));
            }
        }
        if rx!(r"^(?:review|study|practi[cs]e|do)(?: my)? (?:flash ?cards?|cards|deck|review deck)$|^(?:flash ?cards|review)$").is_match(lw) {
            return Some(self.review_start());
        }
        if rx!(r"^(?:my|show(?: my)?|how many) (?:flash ?cards|cards|deck)").is_match(lw) {
            let cards = self.cards();
            let (n, due) = (cards.len(), self.cards_due().len());
            let mut hard = cards.clone();
            hard.sort_by_key(|c| -c["wrong"].as_i64().unwrap_or(0));
            hard.truncate(3);
            let any = hard.iter().any(|c| truthy(&c["wrong"]));
            let extra = if any {
                format!("\nTrickiest: {}", hard.iter().filter(|c| truthy(&c["wrong"]))
                    .map(|c| format!("{} (missed {}x)", crate::extra::clip(&s_of(&c["q"]), 40), c["wrong"].as_i64().unwrap_or(0)))
                    .collect::<Vec<_>>().join("; "))
            } else {
                String::new()
            };
            return Some(format!("Your deck: {n} card{}, {due} due today.{extra}", if n == 1 { "" } else { "s" }));
        }
        // remembering you
        if let Some(m) = rx!(r"(?i)^(?:remember(?: that)?|don'?t forget(?: that)?|note that)\s+(.+)$").captures(raw) {
            if !raw.contains('=') {
                let what = m[1].trim().to_string();
                if let Some(m2) = rx!(r"(?i)^my\s+([a-z' ]{2,30}?)\s+(?:is|are|was)\s+(.+)$").captures(&what) {
                    return Some(self.fact_set(&m2[1], &m2[2]));
                }
                let what = rx!(r"(?i)^i\b").replace(&what, "you").into_owned();
                let w2: String = what.chars().take(160).collect();
                self.life_do(|l| {
                    let mut n: Vec<Value> = l["notes"].as_array().cloned().unwrap_or_default();
                    n.push(json!(w2));
                    let k = n.len().saturating_sub(40);
                    l["notes"] = Value::Array(n[k..].to_vec());
                });
                return Some(format!("Kon! I'll remember: {what}~"));
            }
        }
        if let Some(m) = rx!(r"(?i)^my\s+([a-z' ]{2,30}?)\s+(?:is|are)\s+(.+)$").captures(raw.trim_matches([' ', '.', '!'])) {
            let k = m[1].to_lowercase();
            let keys = routes_data()["FACT_KEYS"].as_array().unwrap();
            if keys.iter().any(|f| k.contains(f.as_str().unwrap())) && !rx!(r"\b(exam|test|quiz|deadline|due)\b").is_match(low) {
                return Some(self.fact_set(&m[1], &m[2]));
            }
        }
        if rx!(r"what do you (?:remember|know) about me|what have i told you|things you remember").is_match(low) {
            return Some(self.facts_text());
        }
        if let Some(f) = self.life_forget(lw) {
            return Some(f);
        }
        // music
        if rx!(r"\b(?:background|bgm|japanese|shrine|zen|calm|relaxing|soothing|study) music\b|\bbgm\b").is_match(low) {
            if rx!(r"\b(off|stop|quiet|pause|no more|disable|turn off|mute)\b").is_match(low) {
                self.music_set(Some(false), None);
                return Some("Okay~ background music off. (Settings > Background music to turn it back on.)".into());
            }
            let tracks = ldata()["MUSIC_TRACKS"].as_array().unwrap().clone();
            for t in &tracks {
                let (key, name) = (s_of(&t[0]), s_of(&t[1]));
                if low.contains(&key) || low.contains(&name.to_lowercase()) {
                    self.music_set(Some(true), Some(&key));
                    return Some(format!("Playing \u{201c}{name}\u{201d} softly~"));
                }
            }
            self.music_set(Some(true), None);
            let cur = self.cfg.s("bgm_track", "garden");
            let name = tracks.iter().find(|t| t[0] == cur.as_str()).map(|t| s_of(&t[1])).unwrap_or_default();
            return Some(format!("Kon~ playing soft Japanese music (\u{201c}{name}\u{201d}). Change the track or volume in Settings."));
        }
        // desktop life toggles
        if let Some(m) = rx!(r"^(?:turn |switch )?(roam(?:ing)?|walking|seasons?|weather effects|snow|petals|night (?:mode|nags?)|day ?/? ?night)(?: mode)? (on|off)$|^(stop|start) (roaming|walking around|wandering)$").captures(lw) {
            let what = m.get(1).or(m.get(4)).map(|g| g.as_str()).unwrap_or("");
            let on = match m.get(2) { Some(g) => g.as_str() == "on", None => m.get(3).is_some_and(|g| g.as_str() == "start") };
            let key = if rx!(r"^(?:roam|walk|wander)").is_match(what) { "roam" } else if rx!(r"^(?:season|weather|snow|petal)").is_match(what) { "seasons" } else { "daynight" };
            self.cfg_set(key, json!(on));
            let label = match key { "roam" => "Roaming", "seasons" => "Seasons and weather", _ => "Day and night" };
            return Some(format!("{label} {}~", if on { "on" } else { "off" }));
        }
        if rx!(r"^(?:go for a walk|walk around|go explore|go roam)$").is_match(lw) {
            if self.session.is_some() || self.lockin.is_some() {
                return Some("Not during focus time~ I'm staying right here!".into());
            }
            self.life.roam_next = 0.0;
            self.life.roam_pause_until = 0.0;
            return Some("Yatta~ walkies! *trots off*".into());
        }
        // streaks, badges, her mood
        if rx!(r"\b(my )?streak\b|\bbadges?\b|\bachievements?\b|\bema\b").is_match(low) {
            let all = rdata()["BADGES"].as_array().unwrap();
            let l = read_json("zenko_life.json");
            let got: Vec<&Value> = l["badges"].as_object().map(|o| o.keys().filter_map(|b| all.iter().find(|x| x[0] == b.as_str())).collect())
                .unwrap_or_default();
            self.later(now + 0.2, Later::Command("open_hub:daily".into()));
            let st = crate::rewards::streak();
            let tail = if got.is_empty() { "\nFinish a focus session to earn your first one!".to_string() } else {
                format!("\n{}", got.iter().map(|b| format!("\u{3010}{}\u{3011}{}", s_of(&b[1]), s_of(&b[2]))).collect::<Vec<_>>().join("  "))
            };
            return Some(format!("Focus streak: {st} day{}~  Ema badges: {} of {}{tail}", if st == 1 { "" } else { "s" }, got.len(), all.len()));
        }
        if rx!(r"^(?:are you (?:hungry|ok|okay|happy|sad|sleepy)|how are you(?: feeling| doing)?(?: today)?|how'?s my fox|you hungry)$").is_match(lw) {
            let (sp, hu) = (self.spirit(), self.hunger_now());
            let ld = ldata();
            let pick = |s: &mut Self, k: &str| -> String {
                let v: Vec<String> = ld[k].as_array().unwrap().iter().map(s_of).collect();
                s.brain.rng.choice(&v).clone()
            };
            let mut feel = if sp >= 75 { pick(self, "HAPPY_LINES") } else if sp >= 50 { "I'm doing okay~ kon!".into() } else if sp >= 30 {
                "*yawn* A bit sleepy... a study session would wake me up~".into()
            } else {
                pick(self, "POUTY_LINES")
            };
            if hu >= 60.0 {
                feel += &format!(" {}", pick(self, "HUNGRY_LINES"));
            }
            return Some(format!("{feel}\n({})", self.needs_text()));
        }
        if rx!(r"^(?:write (?:me )?a |tell me a |)haiku$").is_match(lw) {
            return Some(format!("A haiku for you~\n\n{}", self.haiku()));
        }
        if rx!(r"^(?:invite|call) (?:a |your )?friend$|^any visitors$").is_match(lw) {
            self.visitor_start(now);
            return Some("Kon kon~ I'll call a friend!".into());
        }
        None
    }
}
