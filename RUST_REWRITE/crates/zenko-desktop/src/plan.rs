//! The study planner (zenko.py plan_targets / replan / plan_study_slots / plan_maybe) and zenko23's emas that plan
//! themselves (z23_ema_task, z23_plan_emas, z23_announce, z23_noplan, z23_ema_plan_reply) plus Zenko23's chat route.
//! Times are naive local seconds (datetime.now() as a number), like Python's naive datetimes.

use crate::ema::{ema_when, fmt_hm};
use crate::rewards::{Later, days_from_civil};
use crate::session::clip;
use crate::tt::{save_tt, tt_today};
use crate::{Companion, Ev, data_dir, read_json, unix_now};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

pub(crate) const PLAN_KINDS: [&str; 6] = ["Exam", "Quiz", "Assignment", "Project", "Presentation", "Deadline"];

pub(crate) fn plan_hours_default(kind: &str) -> f64 {
    match kind {
        "Exam" => 6.0,
        "Quiz" => 2.0,
        "Assignment" => 4.0,
        "Project" => 5.0,
        "Presentation" => 3.0,
        "Deadline" => 2.0,
        _ => 3.0,
    }
}

pub fn naive(ts: f64) -> f64 {
    ts + crate::local_offset_pub() as f64
}

pub(crate) fn day_n(nl: f64) -> i64 {
    nl.div_euclid(86400.0) as i64
}

pub fn iso(day: i64) -> String {
    let (y, m, d) = crate::civil_from_days_pub(day);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn parse_iso(s: &str) -> Option<i64> {
    let p: Vec<&str> = s.split('-').collect();
    if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 {
        return None;
    }
    Some(days_from_civil(p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?))
}

fn hhmm(nl: f64) -> String {
    let s = nl.rem_euclid(86400.0) as i64;
    format!("{:02}:{:02}", s / 3600, s / 60 % 60)
}

pub(crate) fn truthy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}

pub(crate) fn num(v: &Value, d: f64) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(d),
        Value::Bool(b) => *b as i64 as f64,
        Value::String(s) => s.trim().parse().unwrap_or(d),
        _ => d,
    }
}

pub(crate) fn start_min(it: &Value) -> Option<i64> {
    let s = it["start"].as_str()?;
    let (h, m) = s.split_once(':')?;
    Some(h.trim().parse::<i64>().ok()? * 60 + m.trim().parse::<i64>().ok()?)
}

/// the cfg value Python reads as int(cfg.get(k, d) or d)
pub(crate) fn cfg_int(c: &crate::Cfg, k: &str, d: i64) -> i64 {
    let v = num(c.raw.get(k).unwrap_or(&Value::Null), d as f64) as i64;
    if v == 0 { d } else { v }
}

#[derive(Clone, Debug)]
pub struct Target {
    pub id: String,
    pub title: String,
    pub due: f64,
    pub need: i64,
    /// the hours planned in all (plan_hours or the kind's default)
    pub hours: f64,
}

#[derive(Clone, Debug)]
pub struct Sess {
    pub date: String,
    pub start: String,
    pub mins: i64,
    pub id: String,
    pub title: String,
    pub review: bool,
}

/// plan_study_slots: half-hour free time -> study sessions before each target's due time. Returns (sessions, short)
pub fn plan_study_slots(targets: &[Target], free_slots: &dyn Fn(i64) -> Vec<f64>, busy: &dyn Fn(i64) -> Vec<(i64, i64)>,
                        now: f64, session_min: i64, day_cap: i64, horizon: i64) -> (Vec<Sess>, Vec<(String, i64)>) {
    let targets: Vec<Target> = targets.iter().filter(|t| t.need > 0 && t.due > now).cloned().collect();
    if targets.is_empty() {
        return (vec![], vec![]);
    }
    let session_min = 15.max(if session_min == 0 { 50 } else { session_min });
    let per = 1.max((session_min as f64 / 30.0).ceil() as usize);
    let short_min = session_min.min(25);
    let maxdue = targets.iter().map(|t| day_n(t.due)).max().unwrap();
    let last = maxdue.min(day_n(now) + horizon);
    let mut slots: Vec<(f64, i64)> = vec![];
    let mut day = day_n(now);
    while day <= last {
        let taken = busy(day);
        let mut vals: Vec<f64> = free_slots(day).iter().map(|v| (v * 2.0).round_ties_even() / 2.0).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        vals.dedup();
        let mut ok: Vec<f64> = vec![];
        for s in vals {
            let a = (s * 60.0) as i64;
            let st = (day * 86400) as f64 + (a * 60) as f64;
            if st < now + 300.0 {
                continue;
            }
            if taken.iter().any(|&(s_, e)| a < e && s_ < a + 30) {
                continue;
            }
            ok.push(s);
        }
        let mut i = 0;
        while i < ok.len() {
            let mut j = i;
            while j + 1 < ok.len() && (ok[j + 1] - ok[j] - 0.5).abs() < 1e-6 {
                j += 1;
            }
            let run = &ok[i..=j];
            let mut k = 0;
            while k < run.len() {
                let left = run.len() - k;
                let st = (day * 86400) as f64 + ((run[k] * 60.0) as i64 * 60) as f64;
                if left >= per {
                    slots.push((st, session_min));
                    k += per;
                } else {
                    slots.push((st, if (left as i64) * 30 < session_min { short_min } else { session_min }));
                    k += left;
                }
            }
            i = j + 1;
        }
        day += 1;
    }
    slots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut need: HashMap<String, i64> = targets.iter().map(|t| (t.id.clone(), t.need)).collect();
    let mut per_day: HashMap<(i64, String), i64> = HashMap::new();
    let mut out = vec![];
    let pad = 1800.0;
    for (k, &(st, mins)) in slots.iter().enumerate() {
        let mut best: Option<&Target> = None;
        let mut best_r = 0.0;
        for t in &targets {
            if need[&t.id] <= 0 || st + (mins * 60) as f64 > t.due - pad {
                continue;
            }
            let room: i64 = slots[k..].iter().filter(|(s2, m)| s2 + (m * 60) as f64 <= t.due - pad).map(|x| x.1).sum();
            let ratio = room as f64 / need[&t.id] as f64;
            if *per_day.get(&(day_n(st), t.id.clone())).unwrap_or(&0) >= day_cap * session_min && ratio > 1.0 {
                continue;
            }
            if best.is_none() || ratio < best_r || (ratio == best_r && t.due < best.unwrap().due) {
                best = Some(t);
                best_r = ratio;
            }
        }
        let Some(b) = best else { continue };
        let mut use_ = mins;
        let n = need[&b.id];
        if n < mins {
            use_ = 15.max((n as f64 / 5.0).ceil() as i64 * 5);
        }
        *need.get_mut(&b.id).unwrap() -= use_;
        *per_day.entry((day_n(st), b.id.clone())).or_default() += use_;
        out.push(Sess { date: iso(day_n(st)), start: hhmm(st), mins: use_, id: b.id.clone(), title: b.title.clone(),
                        review: need[&b.id] <= 0 });
    }
    let short = targets.iter().filter(|t| need[&t.id] > 0).map(|t| (t.id.clone(), need[&t.id])).collect();
    (out, short)
}

/// deadline_due: the date at its "HH:MM" time, else 08:00 (naive local seconds)
pub fn deadline_due(d: &Value) -> Option<f64> {
    let day = parse_iso(d["date"].as_str()?)?;
    let t = d["time"].as_str().unwrap_or("");
    let (h, m) = if rx!(r"^\d\d:\d\d$").is_match(t) { (t[..2].parse().unwrap(), t[3..].parse().unwrap()) } else { (8i64, 0i64) };
    Some((day * 86400 + h * 3600 + m * 60) as f64)
}

pub fn load_deadlines() -> Vec<Value> {
    match read_json("deadlines.json") {
        Value::Array(a) => a,
        _ => vec![],
    }
}

pub fn save_deadlines(items: &mut [Value]) {
    items.sort_by(|a, b| {
        let k = |d: &Value| (d["date"].as_str().unwrap_or("9999").to_string(),
                              d["time"].as_str().filter(|s| !s.is_empty()).unwrap_or("99:99").to_string());
        k(a).cmp(&k(b))
    });
    let _ = std::fs::create_dir_all(data_dir());
    let _ = std::fs::write(data_dir().join("deadlines.json"), crate::py_json_ascii_indent(&Value::Array(items.to_vec()), b" "));
}

pub(crate) fn getrandbits40(r: &mut zenko_sound::mt::PyRandom) -> u64 {
    let lo = r.genrand() as u64;
    let hi = (r.genrand() >> 24) as u64;
    lo | (hi << 32)
}

/// random.getrandbits(36) (card ids)
pub(crate) fn getrandbits36(r: &mut zenko_sound::mt::PyRandom) -> u64 {
    let lo = r.genrand() as u64;
    let hi = (r.genrand() >> 28) as u64;
    lo | (hi << 32)
}

// ---- zenko23: what the planner makes of an ema ----------------------------------------------------------------------
const Z23_TASK: &str = r"(?i)\b(?:stud(?:y|ying)|revis(?:e|ion|ing)|review|homework|hw|essay|ia|ee|tok|cas|assignment|project|read(?:ing)?|write|writing|finish|complete|prepare|prep|practi[cs]e|past ?papers?|worksheet|notes?|lab|report|presentation|slides|draft|learn|memori[sz]e|quiz|test|exam|flashcards?|vocab|questions?|exercises?|chapter|unit|topic|research|commentary|reflection|portfolio|outline|paper\s?\d|p\d|textbook|problems?|problem set|journal|bio|biology|chem|chemistry|maths?|french|psych|psychology|english|lang ?& ?lit)\b";

/// z23_ema_minutes: what the text says ('for 2 hours', '45 min'), else a guess from the kind of task
pub fn z23_ema_minutes(text: &str) -> i64 {
    let t = text.to_lowercase();
    if let Some(m) = rx!(r"(\d+(?:\.\d+)?|half an?|an?|one|two|three|four)\s*(hours?|hrs?|h|minutes?|mins?)\b").captures(&t) {
        let n: f64 = match &m[1] {
            "half a" | "half an" => 0.5,
            "a" | "an" | "one" => 1.0,
            "two" => 2.0,
            "three" => 3.0,
            "four" => 4.0,
            s => s.parse().unwrap_or(1.0),
        };
        let mins = if m[2].starts_with('h') { n * 60.0 } else { n };
        return mins.clamp(10.0, 600.0) as i64;
    }
    const LONG: [(&str, i64); 5] = [
        (r"\b(?:essay|ia|ee|project|report|presentation|portfolio|research|commentary|draft)\b", 120),
        (r"\bpast ?papers?\b|\bpaper\s?\d\b|\bp\d\b|\bmock\b", 90),
        (r"\b(?:homework|hw|worksheet|assignment|exercises?|problems?|problem set|lab)\b", 45),
        (r"\b(?:stud(?:y|ying)|revis(?:e|ion|ing)|review|practi[cs]e|prepare|prep|chapter|unit|topic|exam|test)\b", 60),
        (r"\b(?:read(?:ing)?|notes?|flashcards?|vocab|quiz|learn|memori[sz]e|questions?|outline)\b", 30),
    ];
    static RS: OnceLock<Vec<Regex>> = OnceLock::new();
    let rs = RS.get_or_init(|| LONG.iter().map(|(p, _)| Regex::new(p).unwrap()).collect());
    for (r, (_, mins)) in rs.iter().zip(LONG) {
        if r.is_match(&t) {
            return mins;
        }
    }
    30
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EmaTask {
    /// homework-like: find free time before it is due
    Task(i64, f64),
    /// at a set time: keep that time free
    Block(i64, f64),
}

/// z23_ema_task(E, now): `now` is naive local seconds
pub fn z23_ema_task(e: &Value, now: f64) -> Option<EmaTask> {
    if truthy(&e["done"]) || e["repeat"] == "daily" || truthy(&e["noplan"]) {
        return None;
    }
    let text = e["text"].as_str().unwrap_or("");
    let task = rx!(Z23_TASK).is_match(text) || rx!(r"(?i)\d+\s*(?:hours?|hrs?|minutes?|mins?)\b").is_match(text)
        || truthy(&e["mins"]);
    let mins = if truthy(&e["mins"]) { num(&e["mins"], 0.0) as i64 } else { z23_ema_minutes(text) };
    let dt = match &e["at"] {
        v if truthy(v) => Some(naive(num(v, 0.0))),
        _ => None,
    };
    if !task {
        return match dt {
            Some(d) if d > now => Some(EmaTask::Block(30, d)),
            _ => None,
        };
    }
    let Some(dt) = dt else {
        return Some(EmaTask::Task(mins, ((day_n(now) + 3) * 86400 + 23 * 3600 + 30 * 60) as f64));
    };
    if dt <= now {
        return Some(EmaTask::Task(mins, ((day_n(now) + 2) * 86400 + 23 * 3600) as f64));
    }
    let sod = dt.rem_euclid(86400.0) as i64;
    if dt - now < 20.0 * 3600.0 && !(sod / 3600 == 8 && sod / 60 % 60 == 0) {
        return Some(EmaTask::Block(mins, dt));
    }
    Some(EmaTask::Task(mins, dt))
}

fn z23_fest_today(day: i64) -> &'static str {
    let (y, mo, _) = crate::civil_from_days_pub(day);
    let y = y as i64;
    let d = |m: i64, dd: i64| days_from_civil(y, m, dd);
    let mut cal: Vec<(&str, i64, i64)> = vec![
        ("omisoka", d(12, 31), d(12, 31)), ("newyear", d(1, 1), d(1, 3)), ("christmas", d(12, 20), d(12, 26)),
        ("halloween", d(10, 25), d(10, 31)), ("setsubun", d(2, 2), d(2, 3)), ("valentine", d(2, 13), d(2, 14)),
        ("hina", d(3, 1), d(3, 3)), ("tanabata", d(7, 5), d(7, 7)), ("kodomo", d(5, 3), d(5, 5)),
        ("obon", d(8, 13), d(8, 16)), ("hanami", d(3, 25), d(4, 8)), ("summer", d(7, 20), d(8, 12)),
    ];
    let tabs: [(&str, [(i64, i64, i64); 5], i64, i64); 3] = [
        ("diwali", [(2026, 11, 8), (2027, 10, 29), (2028, 10, 17), (2029, 11, 5), (2030, 10, 26)], 1, 2),
        ("holi", [(2026, 3, 4), (2027, 3, 22), (2028, 3, 11), (2029, 3, 1), (2030, 3, 20)], 1, 0),
        ("tsukimi", [(2026, 9, 25), (2027, 9, 15), (2028, 10, 3), (2029, 9, 22), (2030, 9, 12)], 1, 1),
    ];
    for (key, tab, before, after) in tabs {
        if let Some(&(_, m, dd)) = tab.iter().find(|t| t.0 == y) {
            let x = d(m, dd);
            cal.insert(0, (key, x - before, x + after));
        }
    }
    for (key, a, b) in cal {
        if a <= day && day <= b {
            return key;
        }
    }
    match mo {
        3..=5 => "spring",
        6..=8 => "summer",
        9..=11 => "autumn",
        _ => "winter",
    }
}

fn z23_fest_names(key: &str) -> (&'static str, &'static str) {
    match key {
        "newyear" => ("お正月", "New Year at the shrine"),
        "setsubun" => ("節分", "Setsubun"),
        "valentine" => ("バレンタイン", "Valentine's Day"),
        "hina" => ("ひな祭り", "Hina-matsuri"),
        "holi" => ("Holi", "Holi, the festival of colours"),
        "hanami" => ("花見", "Hanami, cherry blossom night"),
        "kodomo" => ("こどもの日", "Children's Day"),
        "tanabata" => ("七夕", "Tanabata, the star festival"),
        "summer" => ("夏祭り", "Summer festival"),
        "obon" => ("お盆", "Obon, bon-odori night"),
        "tsukimi" => ("お月見", "Tsukimi, the moon festival"),
        "halloween" => ("ハロウィン", "Halloween night"),
        "diwali" => ("Diwali", "Diwali, the festival of lights"),
        "christmas" => ("クリスマス", "Christmas market"),
        "omisoka" => ("大晦日", "New Year's Eve"),
        "spring" => ("春祭り", "Spring festival"),
        "autumn" => ("秋祭り", "Autumn festival"),
        _ => ("冬祭り", "Winter lantern festival"),
    }
}

/// Python's json.dumps(x, sort_keys=True) is only compared with itself, so any stable text does
fn sorted_json(v: &Value) -> String {
    fn walk(v: &Value) -> Value {
        match v {
            Value::Object(o) => {
                let mut ks: Vec<&String> = o.keys().collect();
                ks.sort();
                let mut m = Map::new();
                for k in ks {
                    m.insert(k.clone(), walk(&o[k]));
                }
                Value::Object(m)
            }
            Value::Array(a) => Value::Array(a.iter().map(walk).collect()),
            x => x.clone(),
        }
    }
    walk(v).to_string()
}

#[derive(Clone, Debug, Default)]
pub struct Planner {
    pub sig: Option<String>,
    pub announced: std::collections::HashSet<String>,
    pub soon: bool,
    pub announce: Vec<String>,
    /// the app time of the chat line being answered (chat_send sets it)
    pub asked_at: Option<f64>,
}

impl Companion {
    pub(crate) fn now_naive(&self) -> f64 {
        naive(unix_now())
    }

    /// free_for(day): the hours (or half hours) you're free that day
    pub fn free_for(life: &Value, day: i64) -> Vec<f64> {
        let k = iso(day);
        let src = match life["free_dates"].get(&k) {
            Some(v) => v.clone(),
            None => {
                let wd = (day + 3).rem_euclid(7);
                life["free"].get(wd.to_string()).cloned().unwrap_or(json!([]))
            }
        };
        let mut v: Vec<f64> = src.as_array().map(|a| a.iter().map(|x| num(x, 0.0)).collect()).unwrap_or_default();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v
    }

    fn busy_for(&self, day: i64) -> Vec<(i64, i64)> {
        let ds = iso(day);
        tt_today(&self.tt, &ds)
            .into_iter()
            .map(|i| &self.tt[i])
            .filter(|it| !truthy(&it["plan"]))
            .filter_map(|it| start_min(it).map(|a| (a, a + num(&it["mins"], 0.0) as i64)))
            .collect()
    }

    pub(crate) fn plan_targets(&mut self, life: &Value, now: f64) -> Vec<Target> {
        let mut items = load_deadlines();
        let mut changed = false;
        let mut out = vec![];
        for d in items.iter_mut() {
            if !truthy(&d["id"]) {
                d["id"] = json!(format!("{:x}", getrandbits40(&mut self.brain.rng)));
                changed = true;
            }
            let kind = d["kind"].as_str().unwrap_or("Deadline").to_string();
            if truthy(&d["done"]) || !PLAN_KINDS.contains(&kind.as_str()) {
                continue;
            }
            let Some(due) = deadline_due(d) else { continue };
            if due <= now || day_n(due) - day_n(now) > 45 {
                continue;
            }
            let id = d["id"].as_str().unwrap_or("").to_string();
            let hrs = life["plan_hours"].get(&id).map(|v| num(v, 3.0)).unwrap_or(plan_hours_default(&kind));
            let studied = num(life["studied"].get(&id).unwrap_or(&Value::Null), 0.0);
            out.push(Target { id, title: d["title"].as_str().unwrap_or("").into(), due,
                              need: 0.max((hrs * 60.0 - studied) as i64), hours: hrs });
        }
        if changed {
            save_deadlines(&mut items);
        }
        out
    }

    pub fn plan_sig(&self) -> String {
        let life = read_json("zenko_life.json");
        let mt = std::fs::metadata(data_dir().join("deadlines.json"))
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0))
            .unwrap_or(0.0);
        let em: Vec<Value> = self.ema_list().into_iter().filter(|e| !truthy(&e["done"]))
            .map(|e| json!([e["id"], e["at"], e["done"], e["mins"], e["noplan"], e["text"]])).collect();
        format!("{}|{mt}|{}|{}|{}|{}|{}|{}|{}", iso(day_n(self.now_naive())), sorted_json(&life["free"]),
                sorted_json(&life["free_dates"]), sorted_json(&life["plan_hours"]),
                self.cfg.raw.get("plan_session").cloned().unwrap_or(json!(50)), self.cfg.b("planner", true),
                Value::Array(em), self.cfg.b("ema_plan", true))
    }

    pub fn plan_maybe(&mut self) {
        let sig = self.plan_sig();
        if self.planner.sig.as_deref() != Some(&sig) {
            self.planner.sig = Some(sig);
            if self.cfg.b("planner", true) {
                self.replan();
            }
        }
    }

    /// replan: re-make all future study sessions from your exams and free hours, then fit the emas in
    pub fn replan(&mut self) -> (Vec<Target>, Vec<Sess>) {
        let now = self.now_naive();
        let today = day_n(now);
        let old = iso(today - 14);
        let tds = iso(today);
        self.tt.retain(|it| {
            let plan = truthy(&it["plan"]);
            let date = it["date"].as_str().map(String::from).unwrap_or_else(|| tds.clone());
            if plan && date < old {
                return false;
            }
            if plan && !truthy(&it["done"]) {
                let st = parse_iso(&date).map(|d| (d * 86400 + start_min(it).unwrap_or(0) * 60) as f64).unwrap_or(0.0);
                if st >= now {
                    return false;
                }
            }
            true
        });
        let life = read_json("zenko_life.json");
        let targets = if self.cfg.b("planner", true) { self.plan_targets(&life, now) } else { vec![] };
        let (sessions, short) = {
            let free = |d: i64| Self::free_for(&life, d);
            let busy = |d: i64| self.busy_for(d);
            plan_study_slots(&targets, &free, &busy, now, cfg_int(&self.cfg, "plan_session", 50),
                             cfg_int(&self.cfg, "plan_day_cap", 4), 45)
        };
        for s in &sessions {
            self.tt.push(json!({"task": format!("{}{}", if s.review { "Review: " } else { "Study: " }, s.title),
                                "start": s.start, "mins": s.mins, "daily": false, "date": s.date, "done": [],
                                "plan": s.id}));
        }
        save_tt(&self.tt);
        let sm: Map<String, Value> = short.into_iter().map(|(k, v)| (k, json!(v))).collect();
        self.life_do(|l| l["plan_short"] = Value::Object(sm));
        self.planner.sig = Some(self.plan_sig());
        self.z23_plan_emas();
        (targets, sessions)
    }

    pub fn z23_plan_emas(&mut self) -> Vec<Sess> {
        if !self.cfg.b("ema_plan", true) || !self.cfg.b("planner", true) {
            let life = read_json("zenko_life.json");
            if life.get("ema_short").is_none_or(truthy) {
                self.life_do(|l| l["ema_short"] = json!({}));
            }
            return vec![];
        }
        let now = self.now_naive();
        let life = read_json("zenko_life.json");
        let mut targets = vec![];
        let mut blocks: HashMap<i64, Vec<(i64, i64)>> = HashMap::new();
        for e in self.ema_active() {
            match z23_ema_task(&e, now) {
                None => {}
                Some(EmaTask::Block(mins, when)) => {
                    let m = (when.rem_euclid(86400.0) as i64) / 60;
                    blocks.entry(day_n(when)).or_default().push((m, m + mins));
                }
                Some(EmaTask::Task(mins, when)) => {
                    let pid = format!("ema:{}", e["id"].as_str().unwrap_or(""));
                    let studied = num(life["studied"].get(&pid).unwrap_or(&Value::Null), 0.0) as i64;
                    let need = 0.max(mins - studied);
                    if need >= 10 {
                        let title = match &e["text"] {
                            Value::String(s) if !s.is_empty() => s.clone(),
                            _ => "Ema".into(),
                        };
                        targets.push(Target { id: pid, title, due: when, need, hours: need as f64 / 60.0 });
                    }
                }
            }
        }
        let (sess, short) = {
            let tt = &self.tt;
            let busy = |d: i64| -> Vec<(i64, i64)> {
                let mut out: Vec<(i64, i64)> = tt_today(tt, &iso(d)).into_iter()
                    .filter_map(|i| start_min(&tt[i]).map(|a| (a, a + num(tt[i].get("mins").unwrap_or(&json!(30)), 30.0) as i64)))
                    .collect();
                out.extend(blocks.get(&d).cloned().unwrap_or_default());
                out
            };
            let free = |d: i64| Self::free_for(&life, d);
            plan_study_slots(&targets, &free, &busy, now, 50.min(cfg_int(&self.cfg, "plan_session", 50)), 2, 21)
        };
        for s in &sess {
            self.tt.push(json!({"task": format!("Ema: {}", clip(&s.title, 48)), "start": s.start, "mins": s.mins,
                                "daily": false, "date": s.date, "done": [], "plan": s.id, "ema": &s.id[4..]}));
        }
        save_tt(&self.tt);
        let sm: Map<String, Value> = short.into_iter().map(|(k, v)| (k, json!(v))).collect();
        self.life_do(|l| l["ema_short"] = Value::Object(sm));
        sess
    }

    pub fn z23_ema_rows(&self, eid: &str) -> Vec<Value> {
        let today = iso(day_n(self.now_naive()));
        let pid = format!("ema:{eid}");
        let mut v: Vec<Value> = self.tt.iter()
            .filter(|i| i["plan"] == pid.as_str() && !truthy(&i["done"]) && i["date"].as_str().unwrap_or("") >= today.as_str())
            .cloned().collect();
        v.sort_by(|a, b| (a["date"].as_str(), a["start"].as_str()).cmp(&(b["date"].as_str(), b["start"].as_str())));
        v
    }

    pub fn z23_when(&self, it: &Value) -> String {
        let d = parse_iso(it["date"].as_str().unwrap_or("")).unwrap_or(0);
        let m = start_min(it).unwrap_or(0);
        let (h, mi) = (m / 60, m % 60);
        let h12 = if h % 12 == 0 { 12 } else { h % 12 };
        let t = format!("{h12}:{mi:02} {}", if h < 12 { "AM" } else { "PM" });
        let days = d - day_n(self.now_naive());
        const WD: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let pre = match days {
            0 => "today ".to_string(),
            1 => "tomorrow ".to_string(),
            _ => format!("{} ", WD[(d + 3).rem_euclid(7) as usize]),
        };
        pre + &t
    }

    fn short_has(&self, pid: &str) -> bool {
        read_json("zenko_life.json")["ema_short"].get(pid).is_some()
    }

    /// z23_announce: 900 ms after an ema is hung, say when she planned it
    pub fn z23_announce(&mut self, id: &str) {
        let Some(e) = self.ema_list().into_iter().find(|e| e["id"] == id) else { return };
        if self.planner.announced.contains(id) || z23_ema_task(&e, self.now_naive()).is_none() {
            return;
        }
        self.planner.announced.insert(id.into());
        self.planner.sig = None;
        self.plan_maybe();
        let rows = self.z23_ema_rows(id);
        let text = e["text"].as_str().unwrap_or("").to_string();
        if rows.is_empty() {
            if self.short_has(&format!("ema:{id}")) {
                self.events.push(Ev::Say {
                    text: format!("Uu~ I couldn't find free time for \u{201c}{}\u{201d} before it's due. Mark more free hours on \
                                   the Upcoming clock?", clip(&text, 30)),
                    secs: 12.0, choices: vec![] });
            }
            return;
        }
        let when = rows.iter().take(3).map(|i| self.z23_when(i)).collect::<Vec<_>>().join(", ")
            + if rows.len() > 3 { " and more" } else { "" };
        let total: i64 = rows.iter().map(|i| num(&i["mins"], 0.0) as i64).sum();
        self.events.push(Ev::Say {
            text: format!("I planned {} for \u{201c}{}\u{201d}: {when}~", fmt_hm(total), clip(&text, 32)),
            secs: 12.0,
            choices: vec![("Looks good".into(), String::new()), ("Don't plan this one".into(), format!("ema_noplan:{id}"))],
        });
    }

    pub fn z23_noplan(&mut self, id: &str, on: bool) {
        self.ema_edit(id, |e| e["noplan"] = json!(on));
        self.planner.sig = None;
        self.plan_maybe();
    }

    /// ema_add / ema_set_done / ema_remove hooks: zenko23 plans again a moment later
    /// (root.after(600, plan_maybe): pump turns it into a timer)
    pub(crate) fn z23_after_change(&mut self) {
        self.planner.sig = None;
        self.planner.soon = true;
    }

    /// (root.after(900, z23_announce))
    pub(crate) fn z23_after_add(&mut self, id: &str) {
        if self.cfg.b("ema_plan", true) {
            self.planner.announce.push(id.into());
        }
    }

    /// pump's start: the planner's after() timers
    pub(crate) fn planner_timers(&mut self, now: f64) {
        // asked for during a chat line: the after() counts from that line, not from this pump
        let now = self.planner.asked_at.take().map_or(now, |t| t.min(now));
        if std::mem::take(&mut self.planner.soon) {
            self.later(now + 0.6, Later::PlanMaybe);
        }
        for id in std::mem::take(&mut self.planner.announce) {
            self.later(now + 0.9, Later::Announce(id));
        }
    }

    pub fn z23_ema_plan_reply(&mut self) -> String {
        self.planner.sig = None;
        self.replan();
        let now = self.now_naive();
        let mut lines = vec![];
        for e in self.ema_active() {
            let Some(info) = z23_ema_task(&e, now) else { continue };
            let text = e["text"].as_str().unwrap_or("");
            let id = e["id"].as_str().unwrap_or("");
            if let EmaTask::Block(..) = info {
                lines.push(format!("\u{2022} {}: kept {} free for it", clip(text, 34),
                                   ema_when(e["at"].as_f64(), unix_now(), false)));
                continue;
            }
            let rows = self.z23_ema_rows(id);
            if !rows.is_empty() {
                let total: i64 = rows.iter().map(|i| num(&i["mins"], 0.0) as i64).sum();
                lines.push(format!("\u{2022} {} ({}): {}", clip(text, 34), fmt_hm(total),
                                   rows.iter().take(3).map(|i| self.z23_when(i)).collect::<Vec<_>>().join(", ")));
            } else if self.short_has(&format!("ema:{id}")) {
                lines.push(format!("\u{2022} {}: no free time left before it's due", clip(text, 34)));
            }
        }
        if lines.is_empty() {
            return "None of your emas look like homework to plan~ Hang one like 'finish bio IA draft by friday'.".into();
        }
        let tip = if truthy(&read_json("zenko_life.json")["free_set"]) {
            ""
        } else {
            "\n(I used my guess of your free time. Mark the real one on the Upcoming clock.)"
        };
        lines.truncate(10);
        format!("Kon! Here's when I fit your emas in:\n{}{tip}", lines.join("\n"))
    }

    /// Zenko23.gym_route's own commands
    pub(crate) fn z23_route(&mut self, now: f64, text: &str) -> Option<String> {
        let low = text.to_lowercase();
        let low = low.trim_matches([' ', '.', '!', '?', '~']);
        let lw = rx!(r"^(?:hey |hi |ok |okay )?(?:zenko[, ]+)?(?:please |pls |can you |could you )?").replace(low, "").into_owned();
        if rx!(r"^(?:auto ?)?plan (?:my |the )?(?:emas?|reminders?|tasks?|homework)$|^when (?:should|do) i do (?:my )?(?:emas?|homework|tasks)$").is_match(&lw) {
            return Some(self.z23_ema_plan_reply());
        }
        if let Some(m) = rx!(r"^(?:don'?t|do not|stop) (?:auto ?)?plan(?:ning)? (.+)$").captures(&lw) {
            if let Some(e) = self.ema_find(&m[1]) {
                let id = e["id"].as_str().unwrap_or("").to_string();
                self.z23_noplan(&id, true);
                return Some(format!("Okay, I won't plan time for \u{201c}{}\u{201d}~", clip(e["text"].as_str().unwrap_or(""), 40)));
            }
        }
        if let Some(m) = rx!(r"^(.+?) (?:takes|will take|needs|should take) (?:about |around )?(.+)$").captures(&lw) {
            if rx!(r"\d|an? hour|half").is_match(&m[2]) {
                if let Some(e) = self.ema_find(&m[1]) {
                    let id = e["id"].as_str().unwrap_or("").to_string();
                    let mins = z23_ema_minutes(&m[2]);
                    self.ema_edit(&id, |e| {
                        e["mins"] = json!(mins);
                        if let Some(o) = e.as_object_mut() {
                            o.shift_remove("noplan");
                        }
                    });
                    self.planner.sig = None;
                    self.plan_maybe();
                    let rows = self.z23_ema_rows(&id);
                    let planned = if rows.is_empty() {
                        String::new()
                    } else {
                        format!(" Planned: {}", rows.iter().take(3).map(|i| self.z23_when(i)).collect::<Vec<_>>().join(", "))
                    };
                    return Some(format!("Got it, {} for \u{201c}{}\u{201d}.{planned}", fmt_hm(mins),
                                        clip(e["text"].as_str().unwrap_or(""), 36)));
                }
            }
        }
        if rx!(r"^(?:open |go to )?(?:the )?(?:town square|square|festival|matsuri)$").is_match(&lw) {
            self.later(now, Later::Command("open_hub:square".into()));
            let (jp, en) = z23_fest_names(z23_fest_today(day_n(self.now_naive())));
            return Some(format!("To the {}~ ({jp})", en.to_lowercase()));
        }
        None
    }
}
