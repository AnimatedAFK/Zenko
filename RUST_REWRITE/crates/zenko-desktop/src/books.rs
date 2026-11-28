//! ZenkoBooks.books_route: the 急 delay (push the rest of today back, undo, I'm back), grades by chat
//! (grade_from_text / grade_add), the study books, bedtime, the vision-model download lines, bio facts and English tips.

use crate::plan::{iso, naive};
use crate::rewards::{Later, local_days};
use crate::routes17::routes_data;
use crate::session::clip;
use crate::tt::{save_tt, tt_done, tt_today};
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

/// self._delay
#[derive(Clone, Debug)]
pub struct Delay {
    pub snap: Vec<Value>,
    pub paused: bool,
    pub job: Option<u64>,
    pub until: f64,
    pub mins: i64,
    pub why: String,
}

#[derive(Clone, Debug, Default)]
pub struct BooksState {
    pub delay: Option<Delay>,
    pub job_n: u64,
    /// the text behind the last "急? How long do you need?" bubble
    pub ask_raw: String,
}

/// Python's "%g"
pub fn pyg(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let e = x.abs().log10().floor() as i32;
    if !(-4..6).contains(&e) {
        let s = format!("{:.5e}", x);
        let (m, ex) = s.split_once('e').unwrap();
        let m = if m.contains('.') { m.trim_end_matches('0').trim_end_matches('.') } else { m };
        let ex: i32 = ex.parse().unwrap();
        return format!("{m}e{}{:02}", if ex < 0 { '-' } else { '+' }, ex.abs());
    }
    let prec = (5 - e).max(0) as usize;
    let s = format!("{x:.prec$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

fn round_even(x: f64) -> i64 {
    x.round_ties_even() as i64
}

impl Companion {
    pub fn subject_short(&self, sid: &str) -> String {
        crate::watch::data().subjects.iter().find(|s| s.sid == sid).map(|s| s.short.clone())
            .unwrap_or_else(|| if sid.is_empty() { "?".into() } else { sid.into() })
    }

    /// delay_schedule(mins, why): push today's remaining timetable back
    pub fn delay_schedule(&mut self, now: f64, mins: f64, why: &str) -> String {
        let mins = mins.clamp(1.0, 240.0) as i64;
        let nl = naive(unix_now());
        let today = nl.div_euclid(86400.0) as i64;
        let d = iso(today);
        let snap = self.tt.clone();
        let (mut moved, mut dropped, mut extra): (Vec<(String, String)>, Vec<String>, Vec<Value>) = (vec![], vec![], vec![]);
        for i in tt_today(&self.tt, &d) {
            let it = &self.tt[i];
            if tt_done(it, &d) {
                continue;
            }
            let Some((h, m)) = it["start"].as_str().and_then(|s| s.split_once(':')) else { continue };
            let (Ok(h), Ok(m)) = (h.parse::<i64>(), m.parse::<i64>()) else { continue };
            let st = (today * 86400 + h * 3600 + m * 60) as f64;
            if st < nl - 300.0 {
                continue;
            }
            let ns = st + (mins * 60) as f64;
            let task = match &it["task"] {
                Value::String(s) => s.clone(),
                v => v.to_string(),
            };
            if ns.div_euclid(86400.0) as i64 != today {
                dropped.push(task);
                continue;
            }
            let sod = ns.rem_euclid(86400.0) as i64;
            let hm = format!("{:02}:{:02}", sod / 3600, sod / 60 % 60);
            let it = &mut self.tt[i];
            if truthy(it.get("daily").unwrap_or(&Value::Null)) {
                if !it["skip"].is_array() {
                    it["skip"] = json!([]);
                }
                it["skip"].as_array_mut().unwrap().push(json!(d));
                let mut c = it.clone();
                if let Some(o) = c.as_object_mut() {
                    o.shift_remove("daily");
                }
                c["date"] = json!(d);
                c["start"] = json!(hm);
                c["done"] = json!([]);
                c["skip"] = json!([]);
                c["delayed"] = json!(true);
                extra.push(c);
            } else {
                it["start"] = json!(hm);
                it["delayed"] = json!(true);
            }
            moved.push((task, hm));
        }
        self.tt.extend(extra);
        save_tt(&self.tt);
        let mut paused = false;
        if let Some(s) = self.session.as_mut().filter(|s| !s.paused) {
            s.paused = true;
            self.watch.alerted = false;
            self.watch.dist_run = 0.0;
            paused = true;
        }
        self.books.job_n += 1;
        let job = self.books.job_n;
        self.later(now + (mins * 60) as f64, Later::DelayOver(job));
        let old = self.books.delay.take();
        self.books.delay = Some(Delay {
            snap: old.as_ref().map(|o| o.snap.clone()).unwrap_or(snap),
            paused: paused || old.as_ref().is_some_and(|o| o.paused),
            job: Some(job),
            until: unix_now() + (mins * 60) as f64,
            mins,
            why: why.into(),
        });
        let mut bits = vec![];
        if paused {
            bits.push("your focus timer is paused".to_string());
        }
        if let Some(first) = moved.first() {
            bits.push(format!("{} thing{} moved back {mins} min (next: \u{201c}{}\u{201d} at {})", moved.len(),
                              if moved.len() == 1 { "" } else { "s" }, clip(&first.0, 26), first.1));
        } else {
            bits.push("nothing else on today's timetable needed moving".into());
        }
        let mut msg = format!("\u{6025}! Go~ {}.", bits.join(", "));
        if !dropped.is_empty() {
            msg += &format!(" {} won't fit before midnight, so I left {}.",
                            dropped.iter().take(2).map(|x| clip(x, 20)).collect::<Vec<_>>().join(", "),
                            if dropped.len() == 1 { "it" } else { "them" });
        }
        msg + &format!(" I'll call you back in {mins} min.")
    }

    pub(crate) fn delay_over(&mut self, now: f64, job: u64) {
        let Some(d) = self.books.delay.as_mut() else { return };
        if d.job != Some(job) {
            return;
        }
        d.job = None;
        let (paused, mins) = (d.paused, d.mins);
        self.events.push(Ev::Beep { kind: "ding".into(), variety: false });
        self.set_mood(now, "happy", 3.0);
        let first = if self.session.as_ref().is_some_and(|s| s.paused) && paused { "Resume timer" } else { "I'm back" };
        self.events.push(Ev::Say {
            text: format!("Okaeri~ the {mins} min are up. Ready to carry on?"),
            secs: 30.0,
            choices: vec![(first.into(), "delay_resume".into()), ("+10 min".into(), "delay_more".into()),
                          ("Undo the move".into(), "delay_undo".into())],
        });
    }

    pub fn delay_resume(&mut self) {
        let d = self.books.delay.take();
        let resume = d.is_some_and(|d| d.paused) && self.session.as_ref().is_some_and(|s| s.paused);
        if resume {
            if let Some(s) = self.session.as_mut() {
                s.paused = false;
            }
            self.say("Back to work! Ganbatte~", 5.0);
        } else {
            self.say("Welcome back~ your timetable is already shifted.", 5.0);
        }
    }

    pub fn delay_undo(&mut self) -> String {
        let Some(d) = self.books.delay.take() else { return "There's no delay to undo~".into() };
        self.tt = d.snap;
        save_tt(&self.tt);
        if d.paused {
            if let Some(s) = self.session.as_mut().filter(|s| s.paused) {
                s.paused = false;
            }
        }
        self.say("Undone: your timetable is back how it was.", 6.0);
        "Undone: your timetable is back how it was.".into()
    }

    /// the delay bubbles' buttons
    pub fn books_button(&mut self, now: f64, cmd: &str) -> bool {
        match cmd {
            "delay_resume" => self.delay_resume(),
            "delay_more" => {
                let m = self.delay_schedule(now, 10.0, "");
                self.say(&m, 9.0);
            }
            "delay_undo" => {
                self.delay_undo();
            }
            c if c.starts_with("delay_ask:") => {
                let n: f64 = c[10..].parse().unwrap_or(10.0);
                let raw = self.books.ask_raw.clone();
                let m = self.delay_schedule(now, n, &raw);
                self.say(&m, 10.0);
            }
            _ => return false,
        }
        true
    }

    fn grade_add(&mut self, sid: &str, kind: &str, score: Option<f64>, out_of: Option<f64>, grade: Option<i64>) -> String {
        let id = format!("{:x}", (unix_now() * 1000.0) as i64);
        let e = json!({"id": id, "sid": sid, "kind": if kind.is_empty() { "Other" } else { kind }, "name": "",
                       "score": score, "out_of": out_of, "grade": grade, "date": iso(local_days())});
        let e2 = e.clone();
        self.life_do(|l| {
            if !l["grades"].is_array() {
                l["grades"] = json!([]);
            }
            l["grades"].as_array_mut().unwrap().push(e2);
        });
        let pct = match (score, out_of) {
            (Some(s), Some(o)) => format!(" ({}%)", round_even(100.0 * s / o)),
            _ => String::new(),
        };
        let what = format!("{} {}", self.subject_short(sid), e["kind"].as_str().unwrap());
        let mut prev: Vec<Value> = read_json("zenko_life.json")["grades"].as_array().cloned().unwrap_or_default()
            .into_iter().filter(|g| g["sid"] == sid && g["id"] != e["id"] && !g["score"].is_null()).collect();
        prev.sort_by(|a, b| (a["date"].as_str().unwrap_or(""), a["id"].as_str().unwrap_or(""))
            .cmp(&(b["date"].as_str().unwrap_or(""), b["id"].as_str().unwrap_or(""))));
        let mut tail = String::new();
        if let (Some(last), Some(s), Some(o)) = (prev.last(), score, out_of) {
            let d = 100.0 * s / o - 100.0 * last["score"].as_f64().unwrap_or(0.0) / last["out_of"].as_f64().unwrap_or(1.0);
            tail = format!(" That's {}{}% vs last time{}", if d >= 0.0 { "+" } else { "" }, round_even(d),
                           if d > 0.0 { " \u{2191} sugoi!" } else { "." });
        }
        let mut parts = vec![];
        if let (Some(s), Some(o)) = (score, out_of) {
            parts.push(format!("{}/{}{pct}", pyg(s), pyg(o)));
        }
        if let Some(g) = grade.filter(|&g| g != 0) {
            parts.push(format!("grade {g}"));
        }
        format!("Saved {what}: {}.{tail}", parts.join(", "))
    }

    fn grade_from_text(&mut self, raw: &str) -> Option<String> {
        let low = raw.to_lowercase();
        if !rx!(r"\b(got|scored|score|received|get|result|mark)\b").is_match(&low) {
            return None;
        }
        let sid = self.subject_in(&low)?;
        let mut kind = "Other".to_string();
        for k in routes_data()["GRADE_KINDS"].as_array().unwrap() {
            let k = k.as_str().unwrap();
            if low.contains(&k.to_lowercase()) {
                kind = k.into();
                break;
            }
        }
        if kind == "Other" {
            if rx!(r"\bmid[- ]?year\b").is_match(&low) {
                kind = "MYA".into();
            } else if rx!(r"\btest\b").is_match(&low) {
                kind = "Unit test".into();
            } else if let Some(m) = rx!(r"\bp(?:aper)? ?([123])\b").captures(&low) {
                kind = format!("Paper {}", &m[1]);
            }
        }
        let (mut sc, mut of) = (None, None);
        if let Some(m) = rx!(r"(\d+(?:\.\d+)?)\s*(?:/|out of)\s*(\d+(?:\.\d+)?)").captures(&low) {
            let (s, o): (f64, f64) = (m[1].parse().unwrap(), m[2].parse().unwrap());
            if o <= 0.0 || s > o {
                return None;
            }
            (sc, of) = (Some(s), Some(o));
        } else if let Some(m) = rx!(r"(\d+(?:\.\d+)?)\s*%").captures(&low) {
            (sc, of) = (Some(m[1].parse().unwrap()), Some(100.0));
        }
        let mg = frx!(r"\b(?:got|scored|received|get) (?:a |an )?([1-7])\b(?!\s*(?:/|out of|%))").captures(&low).ok().flatten()
            .map(|m| m.get(1).unwrap().as_str().to_string())
            .or_else(|| rx!(r"\bgrade (?:of )?([1-7])\b").captures(&low).map(|m| m[1].to_string()));
        let gr: Option<i64> = mg.map(|g| g.parse().unwrap());
        if sc.is_none() && gr.is_none() {
            return None;
        }
        if let (Some(s), Some(o)) = (sc, of) {
            if s > o {
                return None;
            }
        }
        let msg = self.grade_add(&sid, &kind, sc, of, gr);
        Some(msg + " It's in your \u{6210}\u{7e3e} book~")
    }

    pub(crate) fn books_route(&mut self, now: f64, text: &str) -> Option<String> {
        let raw = text.trim();
        let low = raw.to_lowercase();
        let lw = low.trim_end_matches([' ', '!', '.', '?', '~']).to_string();
        let cmd = |me: &mut Self, c: String| me.later(now + 0.15, Later::Command(c));
        if rx!(r"^(?:undo|cancel) (?:the |my )?(?:delay|push ?back|postpone)").is_match(&lw) {
            return Some(self.delay_undo());
        }
        if rx!(r"^(?:i'?m back|im back|back now|done,? i'?m back)$").is_match(&lw) && self.books.delay.is_some() {
            self.delay_resume();
            return Some("Okaeri~".into());
        }
        let dur = rx!(r"(\d+(?:\.\d+)?|an?|one|half an?)\s*(hours?|hrs?|h|minutes?|mins?|m)\b").captures(&low);
        let urgent = rx!(r"\b(delay|postpone|push (?:back|everything|my)|shift (?:my|the|everything)|something (?:came|has come) up|something urgent|need to (?:talk|speak|help|go|run|call|eat)|have to (?:talk|speak|help|go|call)|gotta (?:talk|go|help)|my (?:mum|mom|dad|parents) (?:need|want|called))\b").is_match(&low);
        if urgent && !rx!(r"\b(timer|remind me)\b.*\bin\b").is_match(&low) {
            if let Some(m) = dur {
                let n = match &m[1] {
                    "a" | "an" | "one" => 1.0,
                    "half a" | "half an" => 0.5,
                    s => s.parse().unwrap_or(1.0),
                };
                let mins = if m[2].starts_with('h') { n * 60.0 } else { n };
                return Some(self.delay_schedule(now, mins, raw));
            }
            if rx!(r"\b(delay|postpone|push|shift|urgent|came up|need to|have to|gotta)\b").is_match(&low) {
                self.books.ask_raw = raw.into();
                self.events.push(Ev::Say {
                    text: "\u{6025}? How long do you need?".into(),
                    secs: 25.0,
                    choices: [10, 20, 30, 60].iter().map(|n| (format!("{n} min"), format!("delay_ask:{n}"))).collect(),
                });
                return Some("How long do you need? Tap a time on my bubble, or say 'delay 20 min'.".into());
            }
        }
        if let Some(g) = self.grade_from_text(raw) {
            return Some(g);
        }
        if rx!(r"^(?:open |show )?(?:my )?(?:grades?|results|grade tracker|marks|seiseki|成績)(?: book)?$").is_match(&lw) {
            cmd(self, "book_open:grades".into());
            return Some("Here's your \u{6210}\u{7e3e} book~".into());
        }
        if rx!(r"^(?:open |start |do )?(?:my )?(?:a )?(?:french )?vocab(?:ulary)?(?: drill| practice| test| quiz)?$").is_match(&lw) {
            cmd(self, "book_open:french".into());
            return Some("Vocab drill in your \u{4ecf}\u{8a9e} book~ allons-y!".into());
        }
        if let Some(m) = rx!(r"^(?:open )?(?:my )?(\w+(?: \w+)?) (?:book|diary)$").captures(&lw) {
            let w = m[1].to_string();
            let k = match w.as_str() {
                "diary" | "study" | "study diary" => Some("diary".to_string()),
                "grade" | "grades" => Some("grades".to_string()),
                _ => self.subject_in(&w),
            };
            if let Some(k) = k {
                let kanji = routes_data()["BOOKS"].as_array().unwrap().iter().find(|b| b[0] == k.as_str())
                    .and_then(|b| b[1].as_str()).unwrap_or("").to_string();
                cmd(self, format!("book_open:{k}"));
                return Some(format!("Opening your {kanji} book~"));
            }
        }
        if rx!(r"^(?:open )?(?:my )?(?:book ?shelf|books|study books|hondana|本棚)$").is_match(&lw) {
            cmd(self, "book_open:shelf".into());
            return Some("Your bookshelf~".into());
        }
        if let Some(m) = rx!(r"^(?:set |change )?(?:my )?bed ?time(?: to| is|:| at)?\s*(\d{1,2})(?::|\.)?(\d{2})?\s*(am|pm)?$").captures(&lw) {
            let (mut h, mi): (i64, i64) = (m[1].parse().unwrap(), m.get(2).map(|x| x.as_str().parse().unwrap()).unwrap_or(0));
            let ap = m.get(3).map(|x| x.as_str());
            if ap == Some("pm") && h < 12 {
                h += 12;
            }
            if ap == Some("am") && h == 12 {
                h = 0;
            }
            if h > 23 || mi > 59 {
                return Some("That doesn't look like a time~ try 'bedtime 22:30'.".into());
            }
            let bt = format!("{h:02}:{mi:02}");
            crate::update_config(&mut self.cfg, |v| {
                v["bedtime"] = json!(bt);
                v["sleep_guard"] = json!(true);
            });
            return Some(format!("Bedtime set to {bt}. I'll start the wind-down 30 min before~ oyasumi practice!"));
        }
        if rx!(r"^(?:is it |how'?s (?:the |my )?|check (?:the )?)?(?:download(?:ing)?|model download|pull)(?: status| progress)?$").is_match(&lw)
            || rx!(r"^how do i know (?:it'?s|its|if it'?s) downloading$").is_match(&lw) {
            return Some("Nothing is downloading right now~ press 'Get ...' in a study book's Tutor brain card to start one.".into());
        }
        if rx!(r"^(?:get|download|install) (?:my |your |the )?(?:eyes|vision(?: model)?|screenshot reader)$").is_match(&lw) {
            let m = match self.cfg.raw.get("vision_model") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                _ => routes_data()["VISION_PULL"].as_str().unwrap().into(),
            };
            self.later(now, Later::Command(format!("brain_pull:{m}")));
            return Some(format!("Downloading my eyes ({m}, about 3 GB, free). I'll show the progress here and pop up when it's done~"));
        }
        if rx!(r"^(?:give me )?(?:a )?(?:bio(?:logy)? fact|syllabus fact)$").is_match(&lw) {
            let facts = routes_data()["BIO_FACTS"].as_array().unwrap();
            let f = self.brain.rng.choice(facts).clone();
            return Some(format!("{} {}: {}", f[0].as_str().unwrap(), f[1].as_str().unwrap(),
                                f[2].as_str().unwrap().replace(['[', ']'], "")));
        }
        if rx!(r"^(?:give me )?(?:an? )?(?:english|writing|essay|paper 1|paper 2) tip$").is_match(&lw) {
            let tips = routes_data()["ENGLISH_TIPS"].as_array().unwrap();
            let t = self.brain.rng.choice(tips).clone();
            return Some(format!("{} tip: {}", t[0].as_str().unwrap(), t[1].as_str().unwrap()));
        }
        None
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}
