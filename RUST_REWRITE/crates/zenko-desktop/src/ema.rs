//! zenko18's 絵馬 reminders: ema_parse / ema_when / ema_left / ema_words, the rack in zenko_life.json (ema_add,
//! ema_active, ema_find, ema_set_done, ema_snooze, ema_remove, ema_line), ema_tick + ema_ring (she says it out loud
//! when it's time), and z18_route (remind / plan / move / rename / done / delete / snooze by chat). tt_from_text
//! (the timetable's "add 4pm study 45") lives here too, since "plan ... at 5pm" goes through it.

use crate::rewards::Later;
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

const WD: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
const WD_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const PARTS: [(&str, (i64, i64)); 14] = [
    ("noon", (12, 0)), ("midday", (12, 0)), ("midnight", (23, 59)), ("tonight", (20, 0)), ("this evening", (18, 0)),
    ("in the evening", (18, 0)), ("this afternoon", (15, 0)), ("in the afternoon", (15, 0)), ("this morning", (9, 0)),
    ("in the morning", (7, 30)), ("before bed", (22, 0)), ("after school", (16, 0)), ("after dinner", (19, 30)),
    ("after lunch", (13, 30)),
];
const NUM: [(&str, f64); 25] = [
    ("a", 1.0), ("an", 1.0), ("one", 1.0), ("two", 2.0), ("three", 3.0), ("four", 4.0), ("five", 5.0), ("six", 6.0),
    ("ten", 10.0), ("fifteen", 15.0), ("twenty", 20.0), ("thirty", 30.0), ("forty", 40.0), ("forty-five", 45.0),
    ("fifty", 50.0), ("sixty", 60.0), ("ninety", 90.0), ("half a", 0.5), ("half an", 0.5), ("a couple of", 2.0),
    ("couple of", 2.0), ("a few", 3.0), ("few", 3.0), ("", 1.0), ("", 1.0),
];
const STOP: [&str; 22] = ["the", "a", "an", "my", "to", "of", "and", "at", "on", "for", "in", "it", "that", "this",
                          "reminder", "ema", "remind", "me", "about", "with", "one", "please"];

/// local wall-clock seconds since the epoch (datetime.now() as a number)
fn local(ts: f64) -> f64 {
    ts + crate::local_offset_pub() as f64
}

fn day_of(ts: f64) -> i64 {
    local(ts).div_euclid(86400.0) as i64
}

/// (hour, minute) of a moment, local
fn hm_of(ts: f64) -> (i64, i64) {
    let sod = local(ts).rem_euclid(86400.0) as i64;
    (sod / 3600, sod / 60 % 60)
}

/// datetime.weekday(): Monday = 0
fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// byte offset in `low` (the lower-cased copy) -> byte offset in `orig`, through the character count
fn map_off(orig: &str, low: &str, b: usize) -> usize {
    let n = low[..b].chars().count();
    orig.char_indices().nth(n).map(|(i, _)| i).unwrap_or(orig.len())
}

fn cut(st: &mut String, a: usize, b: usize) {
    let low = st.to_lowercase();
    let (a, b) = (map_off(st, &low, a), map_off(st, &low, b));
    *st = format!("{} {}", &st[..a], &st[b..]);
}

fn ci(p: &str) -> Regex {
    Regex::new(&format!("(?i){p}")).unwrap()
}

/// ema_parse(text): (clean text, epoch seconds or None, "daily" or None). Times with no am/pm from 1 to 7 mean the
/// afternoon.
pub fn ema_parse(text: &str, now: f64) -> (String, Option<f64>, Option<&'static str>) {
    let mut st = format!(" {} ", text.trim());
    let low = |s: &String| s.to_lowercase();
    let mut repeat = None;
    let l = low(&st);
    if let Some(m) = rx!(r"\b(?:every ?day|daily|each day|every morning|every night|every evening)\b").find(&l) {
        repeat = Some("daily");
        let word = m.as_str().to_string();
        cut(&mut st, m.start(), m.end());
        let has_d = low(&st).chars().any(|c| c.is_ascii_digit());
        if word.contains("morning") && !has_d {
            st.push_str(" at 7am");
        } else if (word.contains("night") || word.contains("evening")) && !has_d {
            st.push_str(" at 8pm");
        }
    }
    let mut at = None;
    let l = low(&st);
    let rel = rx!(r"\b(?:in|after)\s+(\d+(?:\.\d+)?|half an?|a couple of|couple of|a few|few|an?|one|two|three|four|five|six|ten|fifteen|twenty|thirty|forty-five|forty|fifty|sixty|ninety)\s*(seconds?|secs?|minutes?|mins?|m|hours?|hrs?|hr|h)\b");
    if let Some(c) = rel.captures(&l) {
        let n = &c[1];
        let v = NUM.iter().find(|(k, _)| !k.is_empty() && *k == n).map(|p| p.1)
            .unwrap_or_else(|| n.parse::<f64>().unwrap_or(1.0));
        let u = &c[2];
        let sec = v * if u.starts_with('h') { 3600.0 } else if u.starts_with('s') { 1.0 } else { 60.0 };
        at = Some(now + sec.max(5.0));
        let m = c.get(0).unwrap();
        cut(&mut st, m.start(), m.end());
    } else {
        let (mut day_off, mut day_set) = (0i64, false);
        let l = low(&st);
        if let Some(m) = rx!(r"\b(?:tomorrow|tmrw|tmr|tmrrw)\b").find(&l) {
            (day_off, day_set) = (1, true);
            cut(&mut st, m.start(), m.end());
        }
        let l = low(&st);
        let wdr = rx!(r"\b(?:on\s+|next\s+|this\s+)?(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tues|tue|wed|thurs|thu|fri|sat|sun)\b");
        if let Some(c) = wdr.captures(&l) {
            let all = c.get(0).unwrap();
            let g = &c[1];
            if ["on ", "next ", "this "].iter().any(|p| all.as_str().starts_with(p))
                || rx!(r"^(?:on|next|this)\s").is_match(all.as_str()) || g.chars().count() > 4
            {
                let wd = WD.iter().position(|w| *w == &g[..3]).unwrap() as i64;
                day_off = (wd - weekday(day_of(now))).rem_euclid(7);
                if day_off == 0 {
                    day_off = 7;
                }
                day_set = true;
                cut(&mut st, all.start(), all.end());
            }
        }
        let mut hm: Option<(i64, i64)> = None;
        let l = low(&st);
        let ampm = frx!(r"\b(?:at|@|by|around|for)?\s*(\d{1,2})(?:[:.](\d{2}))?\s*(a\.?m\.?|p\.?m\.?)(?!\w)");
        if let Ok(Some(c)) = ampm.captures(&l) {
            let h: i64 = c[1].parse().unwrap();
            let mi: i64 = c.get(2).map(|x| x.as_str().parse().unwrap()).unwrap_or(0);
            let ap = c[3].starts_with('p');
            if (1..=12).contains(&h) && mi < 60 {
                hm = Some((h % 12 + if ap { 12 } else { 0 }, mi));
                let m = c.get(0).unwrap();
                cut(&mut st, m.start(), m.end());
            }
        }
        if hm.is_none() {
            let l = low(&st);
            let c1 = frx!(r"\b(?:at|@|by|around)\s+(\d{1,2})(?:[:.](\d{2}))?\b(?!\s*(?:min|hour|h\b|m\b|%|kg|km))")
                .captures(&l).ok().flatten();
            let c = c1.map(|c| (c.get(0).unwrap().range(), c[1].to_string(), c.get(2).map(|x| x.as_str().to_string())))
                .or_else(|| rx!(r"\b(\d{1,2})[:.](\d{2})\b").captures(&l)
                    .map(|c| (c.get(0).unwrap().range(), c[1].to_string(), Some(c[2].to_string()))));
            if let Some((r, h, mi)) = c {
                let mut h: i64 = h.parse().unwrap();
                let mi: i64 = mi.map(|x| x.parse().unwrap()).unwrap_or(0);
                if h <= 23 && mi < 60 {
                    let cur = hm_of(now);
                    if (1..=7).contains(&h) {
                        h += 12;
                    } else if (8..=11).contains(&h) && !day_set && (h, mi) <= cur && cur < (h + 12, mi) {
                        h += 12;
                    }
                    hm = Some((h, mi));
                    cut(&mut st, r.start, r.end);
                }
            }
        }
        if hm.is_none() {
            let mut parts: Vec<_> = PARTS.to_vec();
            parts.sort_by_key(|p| -(p.0.len() as i64));
            let l = low(&st);
            for (k, v) in parts {
                let r = Regex::new(&format!(r"\b{}\b", regex::escape(k))).unwrap();
                if let Some(m) = r.find(&l) {
                    hm = Some(v);
                    cut(&mut st, m.start(), m.end());
                    break;
                }
            }
        }
        if hm.is_none() && day_set {
            hm = Some((8, 0));
        }
        if let Some((h, mi)) = hm {
            let off = crate::local_offset_pub() as f64;
            let mut base = ((day_of(now) + day_off) * 86400 + h * 3600 + mi * 60) as f64 - off;
            if base <= now && !day_set {
                base += 86400.0;
            }
            at = Some(base);
        }
    }
    let mut c = st;
    c = ci(r"^\s*(?:hey |hi |ok |okay )?(?:zenko[, ]+)?(?:please |pls |plz |can you |could you |would you )?").replace(&c, "").to_string();
    c = ci(r"^\s*(?:remind me|set (?:a |an )?reminder|make (?:a |an )?reminder|add (?:a |an )?reminder|reminder|hang (?:an |a )?ema|write (?:an |a )?ema|ema)\b\s*[:,-]?\s*").replace(&c, "").to_string();
    c = ci(r"^\s*(?:to|about|that|of|for)\s+").replace(&c, "").to_string();
    c = ci(r"\s+(?:at|on|by|around|to|for|please|pls)\s*$").replace(c.trim(), "").to_string();
    c = ci(r"^\s*(?:at|on|by)\s+").replace(&c, "").to_string();
    c = rx!(r"\s+").replace_all(&c, " ").trim_matches([' ', ',', '.', ';', ':', '-', '!', '?', '~']).to_string();
    let c2 = ci(r"\bi need to\b").replace_all(&c, "").trim().to_string();
    if !c2.is_empty() {
        c = c2;
    }
    let mut ch = c.chars();
    if let Some(f) = ch.next() {
        c = f.to_uppercase().collect::<String>() + ch.as_str();
    }
    (c.chars().take(160).collect(), at, repeat)
}

/// strftime("%I:%M %p").lstrip("0")
fn clock12(ts: f64) -> String {
    let (h, m) = hm_of(ts);
    let h12 = if h % 12 == 0 { 12 } else { h % 12 };
    format!("{h12}:{m:02} {}", if h < 12 { "AM" } else { "PM" })
}

/// ema_when(at): 'Today 5:00 PM', 'Tomorrow 7:30 AM', 'Mon 9:00 AM', '12 Dec 9:00 AM'
pub fn ema_when(at: Option<f64>, now: f64, short: bool) -> String {
    let Some(at) = at.filter(|a| *a != 0.0) else { return "Anytime".into() };
    let (d, n) = (day_of(at), day_of(now));
    let hm = clock12(at);
    if d == n {
        return format!("{}{hm}", if short { "" } else { "Today " });
    }
    if d == n + 1 {
        return format!("Tomorrow {hm}");
    }
    if 0 < d - n && d - n < 7 {
        return format!("{} {hm}", WD_NAMES[weekday(d) as usize]);
    }
    let (_, mo, dd) = crate::civil_from_days_pub(d);
    format!("{dd} {} {hm}", MONTHS[mo as usize - 1])
}

fn round_half_even(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// ema_left(at): 'in 25 min', 'in 2 h', 'now', '3 min ago'
pub fn ema_left(at: f64, now: f64) -> String {
    let s = at - now;
    if s.abs() < 45.0 {
        return "now".into();
    }
    let m = round_half_even(s.abs() / 60.0);
    let mut txt = if m < 60 {
        format!("{m} min")
    } else if m < 600 && m % 60 != 0 {
        format!("{} h {} min", m / 60, m % 60)
    } else {
        format!("{} h", round_half_even(m as f64 / 60.0))
    };
    if m >= 60 * 24 {
        let d = round_half_even(m as f64 / 1440.0);
        txt = format!("{d} day{}", if d == 1 { "" } else { "s" });
    }
    if s > 0.0 { format!("in {txt}") } else { format!("{txt} ago") }
}

pub fn ema_words(s: &str) -> Vec<String> {
    rx!(r"[a-z0-9']+").find_iter(&s.to_lowercase()).map(|m| m.as_str().to_string())
        .filter(|w| !STOP.contains(&w.as_str()) && w.chars().count() > 1).collect()
}

/// fmt_hm(minutes): '1h 05m', '2h', '45m'
pub fn fmt_hm(minutes: i64) -> String {
    let (h, m) = (minutes.div_euclid(60), minutes.rem_euclid(60));
    if h > 0 { if m > 0 { format!("{h}h {m:02}m") } else { format!("{h}h") } } else { format!("{m}m") }
}

fn at_of(e: &Value) -> Option<f64> {
    e["at"].as_f64().filter(|a| *a != 0.0)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn s_of(e: &Value, k: &str) -> String {
    match &e[k] {
        Value::String(s) => s.clone(),
        Value::Null => "None".into(),
        v => v.to_string(),
    }
}

impl Companion {
    pub fn ema_list(&self) -> Vec<Value> {
        read_json("zenko_life.json")["ema"].as_array().cloned().unwrap_or_default()
    }

    /// not yet fulfilled, soonest first, 'anytime' plaques after the timed ones
    pub fn ema_active(&self) -> Vec<Value> {
        let mut act: Vec<Value> = self.ema_list().into_iter().filter(|e| !truthy(&e["done"])).collect();
        act.sort_by(|a, b| {
            let k = |e: &Value| (at_of(e).is_none(), at_of(e).unwrap_or(0.0), e["made"].as_f64().unwrap_or(0.0));
            k(a).partial_cmp(&k(b)).unwrap_or(std::cmp::Ordering::Equal)
        });
        act
    }

    pub fn ema_done_list(&self) -> Vec<Value> {
        let mut d: Vec<Value> = self.ema_list().into_iter().filter(|e| truthy(&e["done"])).collect();
        d.sort_by(|a, b| {
            let k = |e: &Value| -e["done_at"].as_f64().unwrap_or(0.0);
            k(a).partial_cmp(&k(b)).unwrap_or(std::cmp::Ordering::Equal)
        });
        d
    }

    /// change one ema (by id) in zenko_life.json
    pub(crate) fn ema_edit(&mut self, id: &str, f: impl FnOnce(&mut Value)) {
        self.life_do(|l| {
            if let Some(e) = l["ema"].as_array_mut().and_then(|a| a.iter_mut().find(|e| e["id"] == id)) {
                f(e);
            }
        });
    }

    pub fn ema_add(&mut self, text: &str, at: Option<f64>, repeat: Option<&str>) -> Value {
        let t = unix_now();
        let id = format!("{:x}{:04x}", (t * 1000.0) as i64, self.brain.rng.randint(0, 0xffff));
        let txt: String = { let s = text.trim(); if s.is_empty() { "Reminder" } else { s } }.chars().take(160).collect();
        let e = json!({"id": id, "text": txt.trim(), "at": at, "repeat": repeat, "done": false, "rang": false,
                       "made": t, "seed": self.brain.rng.randint(1, 9999)});
        let e2 = e.clone();
        self.life_do(|l| {
            if !l["ema"].is_array() {
                l["ema"] = json!([]);
            }
            l["ema"].as_array_mut().unwrap().push(e2);
        });
        self.brain.ema_last = Some(id.clone());
        self.z23_after_add(&id);
        e
    }

    pub fn ema_mark(&mut self, id: &str, done: bool) {
        let t = unix_now();
        self.ema_edit(id, |e| {
            e["done"] = json!(done);
            e["done_at"] = if done { json!(t) } else { Value::Null };
            if !done {
                e["rang"] = json!(false);
            }
        });
        self.brain.ema_last = Some(id.into());
        self.z23_after_change();
    }

    pub fn ema_remove(&mut self, id: &str) {
        self.life_do(|l| {
            if let Some(a) = l["ema"].as_array_mut() {
                a.retain(|e| e["id"] != id);
            }
        });
        self.z23_after_change();
    }

    pub fn ema_snooze(&mut self, id: &str, minutes: i64) {
        let t = unix_now();
        self.ema_edit(id, |e| {
            if e["repeat"] == "daily" && at_of(e).is_some() && !truthy(&e["base_at"]) {
                e["base_at"] = e["at"].clone();
            }
            e["at"] = json!(t + minutes as f64 * 60.0);
            e["rang"] = json!(false);
            e["done"] = json!(false);
        });
        self.brain.ema_last = Some(id.into());
    }

    /// ema_find: '2', '#2', 'the bio one', 'call mum', 'it' -> an ema (active first)
    pub fn ema_find(&self, r: &str) -> Option<Value> {
        let r = r.trim().to_lowercase();
        let r = r.trim_matches([' ', '.', '!', '?', '~', '"', '\'']);
        let r = rx!(r"\b(?:reminders?|emas?|plaques?|notes?|one|the|my)\b").replace_all(r, " ").trim().to_string();
        let act = self.ema_active();
        if ["", "it", "that", "this", "last", "the last", "latest"].contains(&r.as_str()) {
            let e = self.brain.ema_last.as_ref().and_then(|id| self.ema_list().into_iter().find(|e| e["id"] == id.as_str()));
            return e.or(act.last().cloned());
        }
        if let Some(c) = rx!(r"^#?\s*(\d{1,2})$").captures(&r).or_else(|| rx!(r"^(?:number|no\.?)\s*(\d{1,2})$").captures(&r)) {
            let i: i64 = c[1].parse::<i64>().unwrap() - 1;
            return (0 <= i && (i as usize) < act.len()).then(|| act[i as usize].clone());
        }
        let mut want = ema_words(&r);
        want.sort();
        want.dedup();
        if want.is_empty() {
            return None;
        }
        let (mut best, mut score) = (None, 0.0);
        for e in act.iter().chain(self.ema_done_list().iter()) {
            let have = ema_words(&s_of(e, "text"));
            let hit = want.iter().filter(|w| {
                have.contains(w) || have.iter().any(|h| h.chars().count() > 3 && (h.starts_with(w.as_str()) || w.starts_with(h.as_str())))
            }).count();
            let sc = hit as f64 / want.len() as f64 + if truthy(&e["done"]) { 0.0 } else { 0.01 };
            if sc > score {
                (best, score) = (Some(e.clone()), sc);
            }
        }
        if score >= 0.5 { best } else { None }
    }

    pub fn ema_line(&self, e: &Value, n: Option<usize>, now: f64) -> String {
        let when = if at_of(e).is_some() { ema_when(at_of(e), now, false) } else { "anytime".into() };
        let rep = if e["repeat"] == "daily" { ", every day" } else { "" };
        format!("{}{}  ({when}{rep})", n.map(|n| format!("{n}. ")).unwrap_or_default(), s_of(e, "text"))
    }

    /// ema_tick (every 5 s): ring what's due; a daily one moves on to tomorrow
    pub fn ema_tick(&mut self, now: f64) {
        let t = unix_now();
        let mut ring = vec![];
        let mut changed = false;
        let list = self.ema_list();
        for e in &list {
            let Some(at) = at_of(e) else { continue };
            if truthy(&e["done"]) || truthy(&e["rang"]) || at > t {
                continue;
            }
            changed = true;
            if t - at > 6.0 * 3600.0 && e["repeat"] != "daily" {
                ring.push((s_of(e, "id"), None)); // missed while the PC was off: show, don't shout
                continue;
            }
            ring.push((s_of(e, "id"), Some(e.clone())));
        }
        if !changed {
            return;
        }
        for (id, e) in &ring {
            if let Some(e) = e {
                self.ema_ring(now, e);
            }
            let daily = e.as_ref().is_some_and(|e| e["repeat"] == "daily");
            self.ema_edit(id, |x| {
                if daily {
                    let mut nxt = x.as_object_mut().and_then(|o| o.shift_remove("base_at")).and_then(|v| v.as_f64())
                        .filter(|v| *v != 0.0).or(at_of(x)).unwrap_or(t);
                    while nxt <= t {
                        nxt += 86400.0;
                    }
                    x["at"] = json!(nxt);
                } else {
                    x["rang"] = json!(true);
                }
            });
        }
    }

    fn ema_ring(&mut self, now: f64, e: &Value) {
        let txt = match &e["text"] {
            Value::String(s) if !s.is_empty() => s.clone(),
            _ => "your reminder".into(),
        };
        self.events.push(Ev::Beep { kind: "ding".into(), variety: false });
        self.set_mood(now, "happy", 4.0);
        let msg = format!("Kon kon! Reminder: {txt}");
        let id = s_of(e, "id");
        self.events.push(Ev::Say { text: msg.clone(), secs: 30.0,
                                   choices: vec![("Done".into(), format!("ema_ring_done:{id}")),
                                                 ("Snooze 10 min".into(), format!("ema_snooze:{id}"))] });
        self.chat_line(now, "Fox", &msg);
        self.remind_aloud(&format!("Reminder. {txt}"));
    }

    /// the ringing bubble's buttons
    pub fn ema_ring_button(&mut self, cmd: &str) -> bool {
        if let Some(id) = cmd.strip_prefix("ema_ring_done:") {
            if self.ema_list().iter().any(|e| e["id"] == id) {
                self.ema_mark(id, true);
                self.say("Yatta~ I'll hang it with the fulfilled ones.", 5.0);
            }
            return true;
        }
        if let Some(id) = cmd.strip_prefix("ema_noplan:") {
            self.z23_noplan(id, true);
            return true;
        }
        if let Some(id) = cmd.strip_prefix("ema_snooze:") {
            if self.ema_list().iter().any(|e| e["id"] == id) {
                self.ema_snooze(id, 10);
                self.say("Okay, I'll ask again in 10 minutes~", 5.0);
            }
            return true;
        }
        false
    }

    /// tt_from_text("add 4pm study renal 45"): a timetable item
    pub fn tt_from_text(&mut self, text: &str) -> Option<String> {
        let low = text.to_lowercase();
        let tm = rx!(r"\b(\d{1,2})(?::(\d{2}))?\s*(am|pm)\b|\b(\d{1,2}):(\d{2})\b").find(&low)?;
        let start = crate::tt::parse_clock(tm.as_str())?;
        let mut rest = format!("{} {}", &low[..tm.start()], &low[tm.end()..]);
        let mut mm = rx!(r"\b(\d{1,3})\s*(?:min|mins|minutes|m)\b").captures(&rest)
            .or_else(|| rx!(r"\bfor\s+(\d{1,3})\b").captures(&rest)).map(|c| (c.get(0).unwrap().range(), c[1].to_string()));
        let mut length: i64 = 30;
        if mm.is_none() {
            let r2 = rest.trim().to_string();
            if let Some(c) = rx!(r"\b(\d{1,3})\s*$").captures(&r2) {
                mm = Some((c.get(0).unwrap().range(), c[1].to_string()));
                rest = r2;
            }
        }
        if let Some((r, n)) = mm {
            length = n.parse().unwrap();
            rest = format!("{} {}", &rest[..r.start], &rest[r.end..]);
        }
        let daily = rx!(r"\b(daily|every ?day)\b").is_match(&rest);
        let tomorrow = rest.contains("tomorrow");
        let task = rx!(r"\b(add|schedule|daily|every ?day|tomorrow|today|at|for|to my timetable|timetable)\b").replace_all(&rest, " ");
        let task = rx!(r"\s+").replace_all(&task, " ").trim_matches([' ', ',', '.', '-']).to_string();
        if task.is_empty() {
            return Some("What's the task? e.g. add 4pm study renal 45".into());
        }
        let cap = crate::chat::capitalize(&task);
        let day = day_of(unix_now()) + tomorrow as i64;
        let (y, m, d) = crate::civil_from_days_pub(day);
        self.tt.push(json!({"start": start, "mins": length.clamp(1, 480), "task": cap, "daily": daily,
                            "date": format!("{y:04}-{m:02}-{d:02}"), "done": []}));
        crate::tt::save_tt(&self.tt);
        Some(format!("Added {start} \u{2013} {cap} ({length} min){}.",
                     if daily { ", every day" } else if tomorrow { " for tomorrow" } else { "" }))
    }

    /// tt_text: today's timetable as lines
    pub fn tt_text(&self) -> String {
        let day = crate::today_iso();
        let rows = crate::tt::tt_today(&self.tt, &day);
        if rows.is_empty() {
            return "Nothing scheduled today. Try: add 4pm study 45".into();
        }
        rows.iter().map(|&i| {
            let it = &self.tt[i];
            format!("{}  {} ({} min){}", s_of(it, "start"), s_of(it, "task"), it["mins"].as_i64().unwrap_or(0),
                    if crate::tt::tt_done(it, &day) { "  [done]" } else { "" })
        }).collect::<Vec<_>>().join("\n")
    }

    /// zenko18 z18_route: the ema rack by chat
    pub(crate) fn z18_route(&mut self, now: f64, text: &str) -> Option<String> {
        let raw = text.trim();
        let lw0 = raw.to_lowercase();
        let lw = lw0.trim_matches([' ', '.', '!', '?', '~']);
        let lw = rx!(r"^(?:hey |hi |ok |okay )?(?:zenko[, ]+)?(?:please |pls |can you |could you |would you )?").replace(lw, "").to_string();
        if lw.is_empty() {
            return None;
        }
        let t = unix_now();
        if rx!(r"^(?:show |list |open |see |check )?(?:me )?(?:my |all (?:my )?|the )?(?:reminders?|ema(?: board| rack)?|emas|wish plaques?)(?: page| tab| list)?$|^what (?:are my reminders|do i need to remember|did i ask you to remind)|^any reminders").is_match(&lw) {
            self.later(now + 0.12, Later::Command("open_hub:reminders".into()));
            let act = self.ema_active();
            if act.is_empty() {
                return Some("Your ema rack is empty~ Say 'remind me to call mum at 6pm', or write one on the Reminders page.".into());
            }
            let lines: Vec<String> = act.iter().take(12).enumerate().map(|(i, e)| self.ema_line(e, Some(i + 1), t)).collect();
            return Some(format!("Your ema:\n{}", lines.join("\n")));
        }
        if rx!(r"^(?:clear|remove|delete|take down) (?:all )?(?:the )?(?:fulfilled|done|finished|old|completed) (?:reminders?|emas?|plaques?)$").is_match(&lw) {
            let n = self.ema_done_list().len();
            self.life_do(|l| {
                if let Some(a) = l["ema"].as_array_mut() {
                    a.retain(|e| !truthy(&e["done"]));
                }
            });
            return Some(format!("Took down {n} fulfilled ema~"));
        }
        if rx!(r"^(?:clear|remove|delete|cancel|take down) (?:all|every)(?: of)? (?:my |the )?(?:reminders?|emas?)$|^(?:clear|cancel|delete) (?:my )?reminders$").is_match(&lw) {
            let act = self.ema_active();
            let ids: Vec<String> = act.iter().map(|e| s_of(e, "id")).collect();
            self.life_do(|l| {
                if let Some(a) = l["ema"].as_array_mut() {
                    for e in a.iter_mut().filter(|e| ids.contains(&s_of(e, "id")) && !truthy(&e["done"])) {
                        e["done"] = json!(true);
                        e["done_at"] = json!(t);
                    }
                }
            });
            return Some(format!("Okay, I moved all {} reminders to the fulfilled row (tap one there to bring it back).", act.len()));
        }
        if !rx!(r"\bremind").is_match(&lw) {
            let m = rx!(r"^(?:let'?s |i want to |i'?d like to |help me )?(?:plan|schedule|block(?: out)?|book|pencil in|put)\s+(?:in\s+)?(.+)$").captures(&lw)
                .or_else(|| rx!(r"^(?:i (?:will|'ll|want to|need to|have to|wanna|gotta|am going to|'m going to|'m gonna) )((?:study|revise|do|practi[sc]e|work on)\b.+)$").captures(&lw));
            if let Some(m) = m {
                let g = m[1].to_string();
                if rx!(r"\b\d{1,2}(?::\d{2})?\s*(?:am|pm)\b|\b\d{1,2}:\d{2}\b|\bat \d{1,2}\b").is_match(&g) {
                    let mut body = rx!(r"\b(?:a |an |my )?(?:study )?session\b").replace_all(&g, "study").to_string();
                    body = rx!(r"\b(?:on|to|in|into) (?:my |the )?(?:timetable|schedule|planner|calendar)\b").replace_all(&body, " ").to_string();
                    body = rx!(r"\bfor (?:an|one) hour\b").replace_all(&body, "for 60 min").to_string();
                    body = rx!(r"\bfor (\d+(?:\.\d+)?) hours?\b").replace_all(&body, |c: &regex::Captures| {
                        format!("for {} min", (c[1].parse::<f64>().unwrap() * 60.0) as i64)
                    }).to_string();
                    body = rx!(r"\bfor half an hour\b").replace_all(&body, "for 30 min").to_string();
                    body = frx!(r"\bat (\d{1,2})\b(?!\s*(?::|am|pm))").replace_all(&body, |c: &fancy_regex::Captures| {
                        let h: i64 = c[1].parse().unwrap();
                        format!("at {}{}", &c[1], if (1..=7).contains(&h) { "pm" } else { ":00" })
                    }).to_string();
                    if let Some(r) = self.tt_from_text(&format!("add {body}")) {
                        return Some(format!("{r} It's on your timetable, and I'll say it out loud when it's time~"));
                    }
                }
            }
        }
        if rx!(r"^(?:remind me|set (?:a |an )?reminder|make (?:a |an )?reminder|add (?:a |an )?reminder|new reminder|reminder\s*[:-]|hang (?:an |a )?ema|write (?:an |a )?ema|ema\s*[:-]|don'?t let me forget)\b").is_match(&lw) {
            let body = rx!(r"^don'?t let me forget\s*(?:to )?").replace(&lw, "remind me to ").to_string();
            let (txt, at, rep) = ema_parse(&body, t);
            if txt.is_empty() || ["me", "reminder"].contains(&txt.to_lowercase().as_str()) {
                return Some("What should I remind you about? e.g. 'remind me to call mum at 6pm'".into());
            }
            let e = self.ema_add(&txt, at, rep);
            let et = s_of(&e, "text");
            return Some(match at {
                Some(a) => format!("Hung an ema: \u{201c}{et}\u{201d} \u{b7} {} ({}){}. I'll say it out loud then~",
                                   ema_when(Some(a), t, false), ema_left(a, t), if rep.is_some() { ", every day" } else { "" }),
                None => {
                    let w = ema_words(&et);
                    let two = if w.is_empty() { "it".to_string() } else { w[..w.len().min(2)].join(" ") };
                    format!("Hung an ema: \u{201c}{et}\u{201d} (no time yet). Want me to say it at a time? Say 'move {two} to 6pm'~")
                }
            });
        }
        if let Some(m) = rx!(r"^(?:move|change|push|reschedule|shift|set|make|put)\s+(.*?)\s*(?:reminder|ema)?\s+(?:to|for|until)\s+(.+)$").captures(&lw) {
            let tail = m[2].to_string();
            let (_, at, rep) = ema_parse(&format!("x {tail}"), t);
            let r = rx!(r"\b(?:reminder|ema)\b").replace_all(&m[1], "").trim().to_string();
            if let Some(e) = self.ema_find(&r) {
                let id = s_of(&e, "id");
                let et = s_of(&e, "text");
                if let Some(at) = at {
                    self.ema_edit(&id, |x| {
                        x["at"] = json!(at);
                        x["rang"] = json!(false);
                        x["done"] = json!(false);
                        if let Some(r) = rep {
                            x["repeat"] = json!(r);
                        }
                    });
                    self.brain.ema_last = Some(id);
                    return Some(format!("Moved \u{201c}{et}\u{201d} to {} ({})~", ema_when(Some(at), t, false), ema_left(at, t)));
                }
                if rx!(r"\b(?:daily|every ?day)\b").is_match(&tail) {
                    self.ema_edit(&id, |x| x["repeat"] = json!("daily"));
                    return Some(format!("\u{201c}{et}\u{201d} is now every day~"));
                }
                if rx!(r"\b(?:anytime|no time|untimed)\b").is_match(&tail) {
                    self.ema_edit(&id, |x| {
                        x["at"] = Value::Null;
                        x["rang"] = json!(false);
                    });
                    return Some(format!("\u{201c}{et}\u{201d} has no time now~"));
                }
            }
        }
        if let Some(m) = rx!(r"^(?:rename|reword|edit|change|rewrite)\s+(.*?)\s*(?:reminder|ema)?\s+(?:to say|to read|to|so it says|:)\s+(.+)$").captures(&lw) {
            if let Some(e) = self.ema_find(&m[1]) {
                let n = m[2].chars().count();
                let rc: Vec<char> = raw.chars().collect();
                let new: String = rc[rc.len().saturating_sub(n)..].iter().collect();
                let new = new.trim_matches([' ', '.', '"', '\'', '\u{201c}', '\u{201d}']).to_string();
                let mut ch = new.chars();
                let txt = match ch.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + &ch.as_str().chars().take(159).collect::<String>(),
                    None => String::new(),
                };
                let id = s_of(&e, "id");
                let t2 = txt.clone();
                self.ema_edit(&id, |x| x["text"] = json!(t2));
                self.brain.ema_last = Some(id);
                return Some(format!("Rewrote it: \u{201c}{txt}\u{201d}~"));
            }
        }
        let m = rx!(r"^(?:mark|tick off|check off|cross off|complete|finish(?:ed)?|done with|i did|i(?:'ve| have) done|fulfil+|fulfilled)\s+(.+?)(?:\s+as (?:done|finished|complete))?$").captures(&lw)
            .or_else(|| rx!(r"^(.+?) (?:reminder|ema) (?:is )?(?:done|finished|complete)$").captures(&lw));
        if let Some(m) = m {
            if let Some(e) = self.ema_find(&m[1]).filter(|e| !truthy(&e["done"])) {
                self.ema_mark(&s_of(&e, "id"), true);
                return Some(format!("Yatta! \u{201c}{}\u{201d} is fulfilled~ \u{2713}", s_of(&e, "text")));
            }
        }
        if let Some(m) = rx!(r"^(?:delete|remove|cancel|take down|unhang|drop|forget)\s+(.+?)$").captures(&lw) {
            if rx!(r"\b(?:reminders?|ema|#?\d{1,2}$)").is_match(&lw) {
                if let Some(e) = self.ema_find(&m[1]) {
                    self.ema_remove(&s_of(&e, "id"));
                    return Some(format!("Took down \u{201c}{}\u{201d}.", s_of(&e, "text")));
                }
            }
        }
        if let Some(m) = rx!(r"^snooze(?:\s+(.+?))?(?:\s+(?:for|by)\s+(\d+)\s*(min(?:ute)?s?|m|hours?|h))?$").captures(&lw) {
            let e = self.ema_find(m.get(1).map(|x| x.as_str()).unwrap_or(""))
                .or_else(|| self.ema_active().into_iter().find(|e| truthy(&e["rang"])));
            if let Some(e) = e {
                let n: i64 = m.get(2).map(|x| x.as_str().parse().unwrap()).unwrap_or(10)
                    * if m.get(3).map(|x| x.as_str()).unwrap_or("m").starts_with('h') { 60 } else { 1 };
                self.ema_snooze(&s_of(&e, "id"), n);
                return Some(format!("Snoozed \u{201c}{}\u{201d} for {}~", s_of(&e, "text"), fmt_hm(n)));
            }
        }
        None
    }
}
