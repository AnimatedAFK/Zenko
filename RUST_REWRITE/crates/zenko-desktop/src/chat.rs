//! Her chat brain, the offline part: zenko.py FoxBuddy.chat_send + answer_local, with fix_word / fix_typos /
//! fix_first (typo tolerance), expand_shorthand + learn_aliases_from (your shorthand), fox_voice (her fox voice),
//! learn_reply / forget_reply / learned_reply (things you teach her), learned_summary, life_forget, the zenko19
//! poses and outfits, and life_route's breathing and check-in. Lines nothing here understands go to the smart
//! brain (chat_send returns them). Routes still to come: quiz, books, study, extras, apps, deadlines, battery,
//! screen, music, notes, web, timetable add/next and the later versions' chat commands.

use crate::chat_data::*;
use crate::rewards::Later;
use crate::session::mins;
use crate::{Companion, read_json, update_config};
use regex::Regex;
use serde_json::{Value, json};
use std::sync::OnceLock;
use zenko_sound::mt::PyRandom;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

/// Chat state that lives on the fox: the line she didn't understand (to learn your shorthand from the rephrase),
/// and Python's random (seedable, so tests replay zenko.py's picks).
pub struct Brain {
    pub miss: Option<(String, f64)>,
    pub rng: PyRandom,
    /// _ema_last: the ema she last touched ("move it to 6pm")
    pub ema_last: Option<String>,
    /// ema_tick runs every 5 s
    pub ema_next: f64,
}

impl Brain {
    pub fn new(seed: u64) -> Brain {
        Brain { miss: None, rng: PyRandom::new(seed), ema_last: None, ema_next: 0.0 }
    }
}

// ---- typo tolerance ------------------------------------------------------------------------------------------------

fn edit_distance(a: &[char], b: &[char]) -> usize {
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, r) in d.iter_mut().enumerate() {
        r[0] = i;
    }
    for j in 0..=b.len() {
        d[0][j] = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let v = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + (a[i - 1] != b[j - 1]) as usize);
            d[i][j] = v;
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

pub fn fix_word(w: &str, first: bool) -> String {
    let lw = w.to_lowercase();
    let lc: Vec<char> = lw.chars().collect();
    if VOCAB.contains(&lw.as_str()) || lc.len() < 3 || !lc.iter().all(|c| c.is_alphabetic()) {
        return w.to_string();
    }
    let mut limit = if lc.len() <= 6 { 1 } else { 2 };
    if first && lc.len() >= 5 {
        limit = limit.max(2);
    }
    let mut best: Option<(usize, usize)> = None;
    for (i, v) in VOCAB.iter().enumerate() {
        let vc: Vec<char> = v.chars().collect();
        let ok = vc[0] == lc[0] || {
            let (p, q) = (&lc[..2], &vc[..2.min(vc.len())]);
            let (mut sp, mut sq) = (p.to_vec(), q.to_vec());
            sp.sort();
            sq.sort();
            edit_distance(p, q) <= 1 && sp == sq
        };
        if ok {
            let c = (edit_distance(&lc, &vc), i);
            if best.is_none_or(|b| c < b) {
                best = Some(c);
            }
        }
    }
    match best {
        Some((d, i)) if d <= limit => VOCAB[i].to_string(),
        _ => w.to_string(),
    }
}

pub fn fix_typos(text: &str) -> String {
    rx!(r"[A-Za-z]+").replace_all(text, |m: &regex::Captures| fix_word(&m[0], false)).to_string()
}

/// str.split(None, 1) of an already stripped string
fn split1(t: &str) -> (&str, Option<&str>) {
    match t.find(char::is_whitespace) {
        Some(i) => (&t[..i], Some(t[i..].trim_start())),
        None => (t, None),
    }
}

pub fn fix_first(text: &str) -> String {
    let t = text.trim();
    if t.is_empty() {
        return text.to_string();
    }
    let (a, b) = split1(t);
    fix_word(a, true) + &b.map(|r| format!(" {r}")).unwrap_or_default()
}

// ---- learned.json --------------------------------------------------------------------------------------------------

pub fn norm_phrase(t: &str) -> String {
    rx!(r"[^a-z0-9 ]+").replace_all(&t.to_lowercase(), "").trim().to_string()
}

fn take(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

pub fn learn_reply(phrase: &str, answer: &str) {
    let mut d = crate::watch::load_learned();
    d["replies"][take(&norm_phrase(phrase), 80).as_str()] = json!(take(answer.trim(), 300));
    crate::watch::save_learned(&d);
}

fn olen(v: &Value) -> usize {
    v.as_object().map(|o| o.len()).or(v.as_array().map(|a| a.len())).unwrap_or(0)
}

pub fn forget_reply(phrase: &str) -> usize {
    let mut d = crate::watch::load_learned();
    let k = norm_phrase(phrase);
    if k == "everything" {
        let n = olen(&d["replies"]) + olen(&d["sessions"]) + olen(&d["style"]["aliases"]) + olen(&d["facts"]);
        crate::watch::save_learned(&json!({}));
        return n;
    }
    let hit = d["replies"].as_object_mut().and_then(|o| o.shift_remove(&k)).is_some();
    crate::watch::save_learned(&d);
    hit as usize
}

pub fn learned_reply(text: &str, fuzzy: bool) -> Option<String> {
    let d = crate::watch::load_learned();
    let reps = d["replies"].as_object()?;
    let k = norm_phrase(text);
    if let Some(v) = reps.get(&k) {
        return Some(v.as_str().unwrap_or("").to_string());
    }
    if fuzzy && !reps.is_empty() {
        let keys: Vec<String> = reps.keys().cloned().collect();
        if let Some(b) = crate::pydifflib::get_close_matches(&k, &keys, 1, 0.86).first() {
            return Some(reps[b].as_str().unwrap_or("").to_string());
        }
    }
    None
}

fn facts_recent(n: usize) -> Vec<String> {
    let d = crate::watch::load_learned();
    let Some(f) = d["facts"].as_object() else { return vec![] };
    let mut v: Vec<(&String, &Value)> = f.iter().collect();
    v.sort_by(|a, b| {
        let ta = a.1["t"].as_f64().unwrap_or(0.0);
        let tb = b.1["t"].as_f64().unwrap_or(0.0);
        tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
    });
    v.into_iter().take(n).map(|(k, x)| x["q"].as_str().map(String::from).unwrap_or(k.clone())).collect()
}

fn part_of_day(h: i64) -> &'static str {
    if (5..12).contains(&h) {
        "morning"
    } else if h < 17 {
        "afternoon"
    } else if h < 22 {
        "evening"
    } else {
        "late at night"
    }
}

fn shorthand_aliases() -> Vec<(String, String)> {
    let d = crate::watch::load_learned();
    d["style"]["aliases"].as_object().map(|o| {
        o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()
    }).unwrap_or_default()
}

fn py_int(v: &Value, d: i64) -> i64 {
    v.as_i64().or(v.as_f64().map(|f| f as i64)).unwrap_or(d)
}

pub fn learned_summary() -> String {
    let d = crate::watch::load_learned();
    let all = d["sessions"].as_array().cloned().unwrap_or_default();
    let ss = &all[all.len().saturating_sub(80)..];
    let mut lines: Vec<String> = vec![];
    let (mut best_time, mut good_len, mut finish) = (None, None, None);
    if ss.len() >= 3 {
        let mut parts: Vec<(&str, (i64, i64))> = vec![];
        for s in ss {
            let p = part_of_day(py_int(&s["hour"], 12));
            let planned = py_int(&s["planned"], 25);
            let ok = s["done"].as_bool().unwrap_or(false) && py_int(&s["dist"], 0) <= 2.max(planned.div_euclid(5));
            match parts.iter_mut().find(|e| e.0 == p) {
                Some(e) => e.1 = (e.1.0 + ok as i64, e.1.1 + 1),
                None => parts.push((p, (ok as i64, 1))),
            }
        }
        let good: Vec<(f64, i64, &str)> =
            parts.iter().filter(|e| e.1.1 >= 2).map(|e| (e.1.0 as f64 / e.1.1 as f64, e.1.1, e.0)).collect();
        best_time = good.iter().copied().reduce(|a, b| {
            if b.partial_cmp(&a).is_some_and(|o| o.is_gt()) { b } else { a }
        }).map(|g| g.2);
        let mut done: Vec<i64> = ss.iter().filter(|s| s["done"].as_bool().unwrap_or(false))
            .map(|s| py_int(&s["planned"], 0)).collect();
        done.sort();
        if !done.is_empty() {
            good_len = Some(done[done.len() / 2]);
        }
        finish = Some((100.0 * ss.iter().filter(|s| s["done"].as_bool().unwrap_or(false)).count() as f64
            / ss.len() as f64) as i64);
    }
    if let Some(b) = best_time {
        lines.push(format!("You focus best in the {b}."));
    }
    if let Some(g) = good_len.filter(|g| *g != 0) {
        lines.push(format!("You usually finish {g}-minute sessions."));
    }
    if let Some(f) = finish {
        lines.push(format!("You finish {f}% of the sessions you start."));
    }
    if let Some(o) = d["distractions"].as_object().filter(|o| !o.is_empty()) {
        // max(items, key=count): the first one with the biggest count
        let mut top: Option<(&String, i64)> = None;
        for (k, v) in o {
            let n = py_int(v, 0);
            if top.is_none_or(|t| n > t.1) {
                top = Some((k, n));
            }
        }
        let k = top.unwrap().0;
        lines.push(format!("Your biggest distraction is {}.", crate::watch::pretty(k).unwrap_or(k.clone())));
    }
    if let Some(reps) = d["replies"].as_object().filter(|o| !o.is_empty()) {
        let ks: Vec<String> = reps.keys().take(8).map(|k| format!("'{k}'")).collect();
        lines.push(format!("Things you taught me: {}{}", ks.join("; "), if reps.len() > 8 { "..." } else { "" }));
    }
    let al = shorthand_aliases();
    if !al.is_empty() {
        let v: Vec<String> = al[al.len().saturating_sub(8)..].iter().map(|(a, b)| format!("{a} = {b}")).collect();
        lines.push(format!("Your shorthand: {}", v.join(", ")));
    }
    let looked = facts_recent(5);
    if !looked.is_empty() {
        lines.push(format!("Things I looked up for you: {}", looked.join("; ")));
    }
    if lines.is_empty() {
        lines.push("Nothing yet. Do a few focus sessions and I'll start noticing your habits. \
                    You can also teach me: learn: good morning = Ohayo! Let's get to work.".into());
    }
    lines.join("\n")
}

// ---- her voice and your shorthand ----------------------------------------------------------------------------------

pub fn fox_voice(text: &str, mood: &str, tail: bool, lvl: i64, rng: &mut PyRandom) -> String {
    let mut t = text.trim().to_string();
    if t.is_empty() || lvl <= 0 {
        return t;
    }
    let tic = rx!(r"(?i)^(kon|ehehe|yatta|sugoi|waai|hmm|ne[,~]|fumu|uwaa|uu~|auu|gomen|hmph|mou~|kya|poof|okay~)");
    if !tic.is_match(&t) && (lvl >= 2 || rng.random() < 0.92) {
        let pool = match mood {
            "think" => TICS_THINK,
            "sad" => TICS_SAD,
            "cross" => TICS_CROSS,
            _ => TICS_HAPPY,
        };
        t = format!("{} {t}", rng.choice(pool));
    }
    if tail && !t.contains('\n') && t.chars().count() < 160 && t.ends_with(['.', '!'])
        && !["~kon!", "Kon!", "kon!", "..."].iter().any(|e| t.ends_with(e))
        && rng.random() < if lvl >= 2 { 0.85 } else { 0.45 }
    {
        t.pop();
        t.push_str("~kon!");
    }
    t
}

pub fn learn_alias(word: &str, meaning: &str) -> bool {
    let w = take(&word.trim().to_lowercase(), 30);
    let m = take(&meaning.trim().to_lowercase(), 60);
    if w.is_empty() || m.is_empty() || w == m {
        return false;
    }
    let mut d = crate::watch::load_learned();
    style_store(&mut d);
    let al = d["style"]["aliases"].as_object_mut().unwrap();
    al.insert(w, json!(m));
    if al.len() > 200 {
        if let Some(k0) = al.keys().next().cloned() {
            al.shift_remove(&k0);
        }
    }
    crate::watch::save_learned(&d);
    true
}

/// _style_store: learned.json's "style" holds only "aliases" now
fn style_store(d: &mut Value) {
    if !d["style"].is_object() {
        d["style"] = json!({});
    }
    let st = d["style"].as_object_mut().unwrap();
    st.retain(|k, _| k == "aliases");
    if !st.contains_key("aliases") {
        st.insert("aliases".into(), json!({}));
    }
}

pub fn learn_aliases_from(missed: &str, understood: &str) -> Vec<(String, String)> {
    let a: Vec<String> = missed.to_lowercase().split_whitespace().map(String::from).collect();
    let b: Vec<String> = understood.to_lowercase().split_whitespace().map(String::from).collect();
    if a.is_empty() || b.is_empty() || a.len() > 12 {
        return vec![];
    }
    let sm = crate::pydifflib::Matcher::new(&a, &b);
    if sm.ratio() < 0.34 {
        return vec![];
    }
    let p: &[char] = &['?', ',', '.', '!'];
    let mut out = vec![];
    for (op, i1, i2, j1, j2) in sm.opcodes() {
        if op == "replace" && i2 - i1 == 1 && (1..=2).contains(&(j2 - j1)) {
            let old = a[i1].trim_matches(p).to_string();
            let new = b[j1..j2].join(" ").trim_matches(p).to_string();
            let oc: Vec<char> = old.chars().collect();
            if !old.is_empty() && !new.is_empty() && old != new && !oc.iter().all(|c| c.is_ascii_digit())
                && !VOCAB.contains(&old.as_str()) && oc.len() <= 12
                && (new.starts_with(oc[0]) || oc.len() <= 3 || a.len() > 1) && !STOP.contains(&old.as_str())
                && learn_alias(&old, &new)
            {
                out.push((old, new));
            }
        }
    }
    out
}

pub fn expand_shorthand(text: &str) -> String {
    let al = shorthand_aliases();
    rx!(r"[A-Za-z0-9/']+").replace_all(text, |m: &regex::Captures| {
        let lw = m[0].to_lowercase();
        if let Some((_, v)) = al.iter().find(|(k, v)| *k == lw && !v.is_empty()) {
            return v.clone();
        }
        match SHORTHAND.binary_search_by(|(k, _)| k.cmp(&lw.as_str())) {
            Ok(i) => SHORTHAND[i].1.to_string(),
            Err(_) => m[0].to_string(),
        }
    }).to_string()
}

pub fn shorthand_summary() -> String {
    let al = shorthand_aliases();
    if al.is_empty() {
        return "Ne~ I haven't learned any of your shorthand yet! I already know the usual ones (tmrw, rn, pls, hw...). \
                Teach me more: when I say sched I mean timetable".into();
    }
    let v: Vec<String> = al[al.len().saturating_sub(12)..].iter().map(|(a, b)| format!("{a} = {b}")).collect();
    format!("Kon kon~ Your shorthand I've learned: {}", v.join(", "))
}

/// str.capitalize()
pub(crate) fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}

/// What chat_send did with your line.
#[derive(Debug, PartialEq)]
pub enum Sent {
    /// she answered (the line is in chat_log)
    Answered,
    /// nothing offline understood it: ask the smart brain
    Brain(String),
}

impl Companion {
    pub fn fox_name(&self) -> String {
        let n: String = self.cfg.s("fox_name", "").trim().chars().take(20).collect();
        if n.is_empty() { "Zenko".into() } else { n }
    }

    pub fn catchphrase(&self) -> String {
        let n: String = self.cfg.s("catchphrase", "").trim().chars().take(40).collect();
        if n.is_empty() { "Kon! Kon!".into() } else { n }
    }

    /// chat_send(q): `win` = the fox window's top-left (for her poses)
    pub fn chat_send(&mut self, now: f64, q: &str, win: (i32, i32)) -> Sent {
        self.chat_line(now, "You", q);
        let learning = self.cfg.b("learn_style", true);
        let mut ans = self.answer_local(now, q, win);
        if ans.is_none() {
            let alt = expand_shorthand(q);
            if alt.to_lowercase() != q.to_lowercase() {
                ans = self.answer_local(now, &alt, win);
            }
        }
        let Some(mut ans) = ans else {
            self.brain.miss = Some((q.to_string(), now));
            return Sent::Brain(q.to_string());
        };
        if !ans.contains('\n') && ans.chars().count() < 160 && !ans.ends_with("...") {
            let sad = rx!(r"(?i)^(no |nothing|i couldn|i can'?t|sorry)").is_match(&ans);
            let lvl = self.cfg.f("voice_level", 1.0) as i64;
            ans = fox_voice(&ans, if sad { "sad" } else { "happy" }, true, lvl, &mut self.brain.rng);
        }
        let miss = self.brain.miss.take();
        if let Some((mq, mt)) = miss {
            if learning && now - mt < 120.0
                && (mq.split_whitespace().count() as i64 - q.split_whitespace().count() as i64).abs() <= 1
            {
                let got = learn_aliases_from(&mq, q);
                if let Some((o, n)) = got.first() {
                    ans += &format!("\n(Kon! Noted~ when you say '{o}' you mean '{n}'.)");
                }
            }
        }
        self.chat_line(now, "Fox", &ans);
        let first = ans.split('\n').next().unwrap_or("").to_string();
        self.say(&first, 8.0);
        self.set_mood(now, "happy", 1.5);
        Sent::Answered
    }

    /// ask_llm without a smart brain (Ollama not running, or turned off): she says she doesn't know. (Web lookups and
    /// the brains come in a later stage.)
    pub fn brain_fallback(&mut self, now: f64) {
        const FALLBACK: [&str; 3] = ["Uu~ I only know focus things. Type 'help' to see what I can do~kon!",
                                     "Auu... I'm a small fox with a small brain. Try 'help'!",
                                     "Hmm hmm~ not sure about that one. Try 'help', or 'tip' for a focus tip, ne~?"];
        const NO_BRAIN: &str = "\n(Install the free offline smart brain and I'll understand anything: right-click me > Smart brain setup.)";
        let on = self.cfg.b("llm_enabled", true);
        if on {
            self.chat_line(now, "Fox", "(thinking...)");
            self.set_mood(now, "happy", 1.5);
        }
        let msg = format!("{}{NO_BRAIN}", self.brain.rng.choice(&FALLBACK));
        self.chat_line(now, "Fox", &msg);
        if on {
            let first = crate::mdtext::md_plain(&msg).split('\n').next().unwrap_or("").to_string();
            self.say(&first, 12.0);
            self.set_mood(now, "happy", 2.0);
            self.events.push(crate::Ev::Beep { kind: "hello".into(), variety: false });
        }
    }

    /// zenko19 z19_route: poses and outfits by chat
    fn z19_route(&mut self, now: f64, text: &str, win: (i32, i32)) -> Option<String> {
        let lw = text.to_lowercase();
        let lw = lw.trim_matches([' ', '.', '!', '?', '~']);
        let lw = rx!(r"^(?:hey |hi |ok |okay )?(?:zenko[, ]+)?(?:please |pls |can you |could you |will you )?")
            .replace(lw, "").to_string();
        let poses: [(&Regex, &str, &str); 5] = [
            (rx!(r"^(?:do (?:a |some )?)?(?:big )?stretch(?:es|ing)?(?: out)?$"), "stretch", "Nnnnh~ *stretches*"),
            (rx!(r"^(?:do (?:a )?)?play ?bow$|^wanna play\b|^let'?s play$"), "playbow", "*play bow* Kon kon~!"),
            (rx!(r"^(?:do (?:a )?)?pounce|^(?:mouse|mousing) ?jump|^hunt$"), "pounce", "*wiggles* ...Pounce!"),
            (rx!(r"^(?:lie|lay) down|^rest$|^take a rest$"), "rest", "*flops down* Ahh, comfy~"),
            (rx!(r"^sniff(?: around)?$"), "sniff", "*sniff sniff*"),
        ];
        for (pat, k, line) in poses {
            if pat.is_match(&lw) {
                if self.session.is_some() || self.lockin.is_some() {
                    return Some("After this focus session, ne~ I'm being a good study buddy right now.".into());
                }
                self.z19_pose_start(now, k, false, win);
                // her doze length comes from Python's random, before fox_voice's picks
                if let Some(r) = self.life.z19_seq.as_mut() {
                    let mut t = now;
                    for st in r.seq.iter_mut() {
                        let d = if st.0 == "doze" { self.brain.rng.uniform(10.0, 22.0) } else { st.2 - st.1 };
                        *st = (st.0, t, t + d);
                        t += d;
                    }
                }
                return Some(line.into());
            }
        }
        let m = rx!(r"^(?:wear|put on|change into|dress (?:up )?(?:in|for))\s+(?:a |an |the |your |my )?(.+)$").captures(&lw)?;
        let what = &m[1];
        let key = if what.contains("kimono") {
            "kimono"
        } else if rx!(r"yukata|festival").is_match(what) {
            "yukata"
        } else if rx!(r"uniform|sailor|seifuku").is_match(what) {
            "uniform"
        } else if rx!(r"season|auto").is_match(what) {
            "auto"
        } else if rx!(r"nothing|fur|no clothes|none").is_match(what) {
            "none"
        } else {
            return None;
        };
        let need = match key {
            "kimono" => 3,
            "auto" | "none" => 0,
            k => zenko_art::palette::ROBES.iter().find(|r| r.key == k).map(|r| r.hours as i64).unwrap_or(0),
        };
        let hrs = self.total_hours(now);
        if hrs < need as f64 {
            return Some(format!("Ehehe~ that one unlocks after {need} hours of study together. We're at {hrs:.1}!"));
        }
        update_config(&mut self.cfg, |v| v["robe"] = json!(key));
        Some(match key {
            "kimono" => "Kimono on~ it follows the seasons!",
            "yukata" => "Festival yukata! Kon kon~",
            "uniform" => "Sailor uniform for exam season! Ganbarou~",
            "auto" => "I'll dress for the season~",
            _ => "Just my fluffy fur, then~",
        }.into())
    }

    /// zenko16 z16_route: voice chat and the Zen garden breathing
    fn z16_route(&mut self, now: f64, text: &str) -> Option<String> {
        let lw = text.to_lowercase();
        let lw = lw.trim_matches([' ', '.', '!', '?', '~']);
        if lw.split_whitespace().count() > 9 {
            return None;
        }
        let cmd = |me: &mut Self, ms: f64, c: &str| me.later(now + ms / 1000.0, Later::Command(c.into()));
        if rx!(r"^(?:let'?s talk|talk to me|can we talk|voice (?:chat|mode|conversation)|talk out loud|(?:start|open) (?:a )?(?:voice|talking)(?: chat| mode)?)$").is_match(lw) {
            cmd(self, 200.0, "conv_start");
            return Some("Kon! Talk to me out loud~".into());
        }
        if rx!(r"\b(?:let'?s |can we |help me |i want to |start (?:a |the )?)?(?:do (?:some )?)?(?:box breath(?:ing|e)?)\b").is_match(lw) {
            cmd(self, 150.0, "breath:box");
            return Some("Box breathing~ Follow the circle in the Zen garden: in, hold, out, hold.".into());
        }
        if rx!(r"\b4[ -]?7[ -]?8\b|\bcan'?t (?:fall a)?sleep\b|\bhelp me (?:fall a)?sleep\b|\bbreath\w* (?:before|for) (?:bed|sleep)\b|\b(?:wind|calm) down for (?:bed|sleep)\b").is_match(lw) {
            cmd(self, 150.0, "breath:478");
            return Some("Let's do 4-7-8 together. Get cosy, I'll count for you~".into());
        }
        if rx!(r"^(?:a |do a |the )?(?:calming |physiological )?sigh$|\bquick (?:calm|reset)\b").is_match(lw) {
            cmd(self, 150.0, "breath:sigh");
            return Some("One minute of calming sighs. Two sips in, one long breath out~".into());
        }
        if rx!(r"^(?:let'?s |can we |help me |i need to |i want to )?(?:breathe|do (?:a |some )?breathing(?: exercises?)?|breathing(?: exercises?)?|calm me(?: down)?|help me calm down|i need to calm down|zen(?: garden)?|i'?m (?:so |really |super )?(?:nervous|anxious|stressed|panicking|scared)(?: (?:about|for|before) (?:my |the |an? )?\w+(?: \w+)?)?|exam (?:nerves|anxiety|stress))$").is_match(lw) {
            let h = crate::local_now().3;
            let sleepy = h >= 21 || h < 4;
            let k = if sleepy && !rx!(r"exam|test|nervous|panick|scared").is_match(lw) { "478" } else { "box" };
            if lw.starts_with("zen") {
                cmd(self, 150.0, "open_hub:garden");
                return Some("The Zen garden is open. Pick a breath and press Begin~".into());
            }
            cmd(self, 150.0, &format!("breath:{k}"));
            return Some(self.brain.rng.choice(&["It's okay. Breathe with me, I'm right here~",
                "Deep breath, ne~ Follow the circle with me.", "Shh, shh. Let's slow everything down together~"]).to_string());
        }
        if rx!(r"^(?:5 4 3 2 1|5-4-3-2-1|54321|grounding|ground me)$").is_match(lw) {
            cmd(self, 150.0, "grounding");
            return Some("Grounding, step one: name five things you can see~".into());
        }
        None
    }

    /// life_route: the parts ported so far (breathing, the daily check-in)
    fn life_route(&mut self, now: f64, text: &str) -> Option<String> {
        let low = text.trim().to_lowercase();
        let lw = low.trim_end_matches([' ', '!', '.', '?', '~']);
        if rx!(r"^(?:let'?s |can we |i want to |help me )?(?:do (?:a |some )?|start (?:a )?)?(?:breath(?:e|ing)(?: exercise)?|calm me down|i'?m (?:so )?(?:stressed|anxious|panicking|overwhelmed))").is_match(lw) {
            self.later(now + 0.2, Later::Command("open_breathing".into()));
            return Some("Okay~ breathe with my foxfire. In as it grows, out as it shrinks. I'm right here, ne~".into());
        }
        if rx!(r"^(?:daily )?check[ -]?in$|^mood (?:journal|check)$|^(?:open )?(?:my )?journal$|^how am i feeling").is_match(lw) {
            self.later(now + 0.2, Later::Command("open_checkin_force".into()));
            return Some("How are you feeling today? Pick one~".into());
        }
        None
    }

    /// life_forget: "forget my favourite colour" / a note she remembered
    fn life_forget(&mut self, text: &str) -> Option<String> {
        let low = text.to_lowercase();
        let m = rx!(r"^forget (?:my |about my |that my )?(.+?)[.!~]*$").captures(low.trim())?;
        let k = m[1].trim().replace("favorite", "favourite").replace("color", "colour");
        let l = read_json("zenko_life.json");
        if l["facts"].get(&k).is_some() {
            crate::rewards::update_life(&mut || self.rand(), |l| {
                l["facts"].as_object_mut().map(|o| o.shift_remove(&k));
            });
            return Some(format!("Poof~ I forgot your {k}."));
        }
        let notes: Vec<String> = l["notes"].as_array().map(|a| {
            a.iter().filter_map(|v| v.as_str().map(String::from)).collect()
        }).unwrap_or_default();
        for n in notes {
            if n.to_lowercase().contains(&k) && k.chars().count() >= 4 {
                crate::rewards::update_life(&mut || self.rand(), |l| {
                    if let Some(a) = l["notes"].as_array_mut() {
                        if let Some(i) = a.iter().position(|v| v.as_str() == Some(n.as_str())) {
                            a.remove(i);
                        }
                    }
                });
                return Some(format!("Poof~ I forgot that {n}."));
            }
        }
        None
    }

    pub fn answer_local(&mut self, now: f64, text: &str, win: (i32, i32)) -> Option<String> {
        let cmd = fix_first(text);
        let low = fix_typos(text).to_lowercase().trim().to_string();
        let ts = text.trim();
        if ["help", "?", "commands"].contains(&low.as_str()) {
            return Some(HELP.into());
        }
        let m = rx!(r"(?i)^(?:learn|teach(?: you| me)?|remember)\s*[:,]?\s*(.+?)\s*(?:=>|->|=)\s*(.+)$").captures(ts)
            .or_else(|| rx!(r#"(?i)^when i say\s+["']?(.+?)["']?\s*,?\s*(?:you )?(?:say|reply|answer|respond)(?: with)?\s+(.+)$"#).captures(ts));
        if let Some(m) = m {
            learn_reply(&m[1], &m[2]);
            let p = m[1].trim();
            return Some(format!("Kon! Kon! Yatta, learned it! When you say '{p}' I'll answer that. (Say 'forget {p}' to undo.)"));
        }
        if let Some(m) = rx!(r#"(?i)^(?:when|if) i (?:say|type|write)\s+["']?(.+?)["']?\s*,?\s*i (?:mean|meant)\s+["']?(.+?)["']?\s*\.?$"#).captures(ts) {
            if m[1].split_whitespace().count() <= 3 {
                learn_alias(&m[1], &m[2]);
                return Some(format!("Kon kon~ got it: '{}' means '{}'. I'll understand that from now on~kon!",
                                    m[1].trim(), m[2].trim()));
            }
        }
        if rx!(r"^forget (?:my (?:style|slang|shorthand)|how i (?:talk|type|speak|write))\b").is_match(&low) {
            let mut d = crate::watch::load_learned();
            d.as_object_mut().map(|o| o.shift_remove("style"));
            crate::watch::save_learned(&d);
            return Some("Okay~ I've forgotten your shorthand. Fresh start, kon!".into());
        }
        if rx!(r"my (?:slang|shorthand)\b|what shorthand|what(?:'s| is) my (?:style|slang)").is_match(&low) {
            return Some(shorthand_summary());
        }
        if rx!(r"what (?:did|have) you (?:look(?:ed)? up|search(?:ed)?|find|found)|what have you searched").is_match(&low) {
            let looked = facts_recent(8);
            return Some(if looked.is_empty() {
                "Ne~ I haven't looked anything up yet. Ask me anything!".into()
            } else {
                format!("Fumu fumu~ recently I looked up: {}. Ask again and I'll remember (for 12 hours)!", looked.join("; "))
            });
        }
        if let Some(r) = self.life_forget(text) {
            return Some(r);
        }
        if let Some(m) = rx!(r"(?i)^forget\s+(.+)$").captures(ts) {
            let what = m[1].trim();
            let all = ["everything", "everything you learned", "all"].contains(&what.to_lowercase().as_str());
            let n = forget_reply(if all { "everything" } else { what });
            return Some(if n > 0 { "Poof~ forgotten!".into() } else { format!("Ne~? I hadn't learned '{what}'.") });
        }
        if rx!(r"what (have|did) you learn|what do you know about me|my habits|what have you noticed").is_match(&low) {
            return Some(learned_summary());
        }
        if let Some(t) = learned_reply(text, false).filter(|t| !t.is_empty()) {
            return Some(t);
        }
        if let Some(m) = rx!(r"(?i)^(?:set |change )?(?:my )?home ?(?:city|town)(?: to| is|:)?\s+(.+)$").captures(ts) {
            let c = take(m[1].trim_matches([' ', '.']), 60);
            update_config(&mut self.cfg, |v| v["home_city"] = json!(c));
            return Some(format!("Home city set to {c}. Now 'weather' will use it."));
        }
        if let Some(m) = rx!(r"^(?:set |change )?(?:my )?(?:temp(?:erature)?)? ?unit(?: to| is|:)?\s*([cf])\w*\s*$").captures(&low) {
            let u = m[1].to_uppercase();
            update_config(&mut self.cfg, |v| v["temp_unit"] = json!(u));
            return Some(format!("Temperatures will be shown in \u{b0}{u}."));
        }
        // gym_route (zenko19's poses and outfits come first; the other versions' commands are not ported yet)
        if let Some(r) = self.z23_route(now, text) {
            return Some(r);
        }
        if let Some(r) = self.z19_route(now, text, win) {
            return Some(r);
        }
        if let Some(r) = self.z18_route(now, text) {
            return Some(r);
        }
        if let Some(r) = self.z16_route(now, text) {
            return Some(r);
        }
        if let Some(r) = self.life_route(now, text) {
            return Some(r);
        }
        if rx!(r"^(add|schedule)\b").is_match(&low) {
            if let Some(r) = self.tt_from_text(&cmd) {
                return Some(r);
            }
        }
        let s = self.session.as_ref();
        if rx!(r"time left|how long|how much time|remaining").is_match(&low) {
            return Some(match s {
                Some(s) => {
                    let left = (s.planned - s.elapsed).max(0.0) as i64;
                    format!("{} min {:02} s left on \u{201c}{}\u{201d}.", left / 60, left % 60, s.task)
                }
                None => "No session running. Say 'start reading 25' or double-click me.".into(),
            });
        }
        if rx!(r"time ?table|\bagenda\b|my day|today'?s plan|^(show |open |view |see |what'?s )?(my |the |today'?s )?schedule\s*\??$").is_match(&low) {
            self.later(now, Later::Command("open_hub:timetable".into()));
            return Some(format!("Today:\n{}", self.tt_text()));
        }
        if rx!(r"^(what'?s |whats )?next\b|what now|what should i (do|work)").is_match(&low) {
            return Some(match self.focus_item() {
                Some(i) => {
                    let it = &self.tt[i];
                    format!("Next: {} at {} ({} min).", it["task"].as_str().unwrap_or(""), it["start"].as_str().unwrap_or(""),
                            it["mins"].as_i64().unwrap_or(0))
                }
                None => "Nothing left on today's timetable. Add something: add 4pm study 45".into(),
            });
        }
        if let Some(m) = rx!(r"^start\b\s*(.*)").captures(cmd.to_lowercase().trim()) {
            if s.is_some() {
                return Some("A session is already running. Say 'stop' first.".into());
            }
            let rest = m[1].to_string();
            let mm = rx!(r"(\d{1,3})\s*(?:min|mins|minutes|m)?\s*$").captures(&rest);
            let length: i64 = match &mm {
                Some(c) => c[1].parse().unwrap(),
                None => self.cfg.f("last_minutes", 25.0) as i64,
            };
            let task = match &mm {
                Some(c) => &rest[..c.get(0).unwrap().start()],
                None => rest.as_str(),
            };
            let task = task.trim_matches([' ', ',', '.', '-']);
            let task = capitalize(if task.is_empty() { "Focus session" } else { task });
            self.start_session(now, &task, length.clamp(1, 240) as f64);
            return Some(format!("Started {length} min on \u{201c}{task}\u{201d}. Go!"));
        }
        if low == "pause" || low == "resume" {
            let Some(p) = s.map(|s| s.paused) else { return Some("No session running.".into()) };
            if (low == "pause") != p {
                self.toggle_pause();
            }
            let p = self.session.as_ref().is_some_and(|s| s.paused);
            return Some(if p { "Paused." } else { "Resumed." }.into());
        }
        if ["stop", "end", "quit session"].contains(&low.as_str()) {
            if s.is_none() {
                return Some("No session running.".into());
            }
            self.end_session(now, false);
            return Some("Session stopped.".into());
        }
        if rx!(r"how am i doing|stats|today'?s focus|how much have i").is_match(&low) {
            return Some(format!("Today you've focused for {} min.", crate::slow::today_focus_minutes()));
        }
        if rx!(r"motivat|inspire me|cheer me|encourage me|i can'?t do this|too hard|give up").is_match(&low) {
            let pool: Vec<&str> = MOTIV.iter().copied().filter(|m| s.is_some() || !m.contains("{left}")).collect();
            let (task, left) = match s {
                Some(s) => (s.task.clone(), mins((s.planned - s.elapsed).max(0.0))),
                None => ("your work".to_string(), 0),
            };
            let pick = *self.brain.rng.choice(&pool);
            return Some(pick.replace("{task}", &task).replace("{left}", &left.to_string()));
        }
        if rx!(r"^tips?\b|focus (tip|advice)|help me focus|procrastinat").is_match(&low) {
            return Some(self.brain.rng.choice(FOCUS_TIPS).to_string());
        }
        if rx!(r"\b(jokes?|funny|laugh)\b").is_match(&low) {
            let j = *self.brain.rng.choice(JOKES);
            let e = *self.brain.rng.choice(&["Ehehe~", "Kon kon~!", "Ehehe~ kon!"]);
            return Some(format!("{j} {e}"));
        }
        if rx!(r"(end|stop|finish|over).{0,6}\bbreak\b|back to work").is_match(&low) {
            if self.brk.is_none() {
                return Some("No break running.".into());
            }
            self.end_break(now, false);
            return Some("Break ended.".into());
        }
        if rx!(r"(take|start|begin|need|want|have|give me).{0,12}\bbreak\b|\bbreak (now|time)\b|^break$|^rest$").is_match(&low) {
            let m = self.cfg.f("break_minutes", 5.0) as i64;
            self.start_break(now, None);
            return Some(format!("Break started: {m} min. I'll wake you when it's over."));
        }
        if rx!(r"\bbreak\b|\btired\b|\brest\b").is_match(&low) {
            return Some(self.brain.rng.choice(BREAK).to_string());
        }
        if rx!(r"who are you|your name|what are you|what can you do").is_match(&low) {
            return Some(format!("{} I'm {}, a zenko: one of the good foxes who serve Inari, the kami of rice and good \
                                 fortune. I watch for distractions, run your timer, keep your timetable and cheer you on. \
                                 Type 'help' for commands.", self.catchphrase(), self.fox_name()));
        }
        if rx!(r"\b(zenko|inari|legend|foxfire|tails?)\b.{0,20}\?|tell me about (zenko|inari|the legend)|\blegend\b").is_match(&low) {
            return Some(self.brain.rng.choice(LORE).to_string());
        }
        if rx!(r"^(hi|hello|hey|heya|hiya|yo|sup|wassup|howdy|hola|ohayo|gm|good (?:morning|afternoon|evening))\b").is_match(&low) {
            let nm = self.user_short();
            let g = *self.brain.rng.choice(&["Kon kon~", "Konkon~", "Waai~", "Kon"]);
            let nm = if nm.is_empty() { String::new() } else { format!(" {nm}") };
            return Some(format!("{g}{nm}! Ready to focus? Type 'start studying 25' or open the timetable, ne~?"));
        }
        if rx!(r"^(thanks|thank you|thx|ty\b|tysm)").is_match(&low) {
            return Some(self.brain.rng.choice(&["Ehehe~ anytime! Now back to work~kon!", "Kon kon~ happy to help! Back to it, ne?",
                                                "Yatta~ anytime! Now let's focus, kon!"]).to_string());
        }
        learned_reply(text, true).filter(|t| !t.is_empty())
    }
}
