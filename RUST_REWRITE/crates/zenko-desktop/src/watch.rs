//! The watcher (zenko.py FoxBuddy._tick, is_distracting (+ zenko17 rules and study videos), pick_nag, nag,
//! maybe_motivate, maybe_close (zenko21.1), break_tick / start_break / end_break, the passive nudge,
//! learn_session / learn_distraction / learned_hint, study_video, study_video_subject, classify_subject, rule_verdict,
//! window_key). The word lists come from zenko.py itself (tools/gen_desktop_data.py -> data/desktop.json).

use crate::session::{clip, mins};
use crate::{Companion, Ev, read_json};
use regex::Regex;
use serde_json::{Value, json};
use std::sync::OnceLock;

pub struct Subject {
    pub sid: String,
    pub short: String,
    pub aliases: Vec<String>,
    pub topics: Vec<String>,
    pub sites: Vec<String>,
}

pub struct Data {
    pub raw: Value,
    pub subjects: Vec<Subject>,
    /// (subject, [(core pattern anchored at start, strong?)])
    pub sv_terms: Vec<(String, Vec<(Regex, bool)>)>,
    pub cues: Regex,
    pub nope: Regex,
    pub tail: Regex,
    pub title_tails: Regex,
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn py_re(p: &str, ci: bool) -> Regex {
    Regex::new(&format!("{}{}", if ci { "(?i)" } else { "" }, p)).expect("zenko.py pattern")
}

pub fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| {
        let raw: Value = serde_json::from_str(include_str!("../data/desktop.json")).expect("desktop.json");
        let subjects = raw["SUBJECTS"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| Subject {
                sid: s[0].as_str().unwrap().into(),
                short: s[2].as_str().unwrap().into(),
                aliases: strs(&s[3]),
                topics: strs(&s[4]),
                sites: strs(&s[5]),
            })
            .collect();
        let junk = py_re(raw["RE"]["phrase_junk"].as_str().unwrap(), false);
        let mut sv_terms = vec![];
        for (sid, lst) in raw["SV_TERMS"].as_object().unwrap() {
            let mut out = vec![];
            for t in lst.as_array().unwrap() {
                let w = t[0].as_str().unwrap();
                if junk.is_match(w) {
                    continue;
                }
                let core = w.split(' ').map(regex::escape).collect::<Vec<_>>().join(r"[\s\-]+");
                out.push((Regex::new(&format!("^(?:{core})")).unwrap(), t[1].as_bool().unwrap()));
            }
            sv_terms.push((sid.clone(), out));
        }
        Data {
            cues: py_re(raw["RE"]["cues"].as_str().unwrap(), true),
            nope: py_re(raw["RE"]["nope"].as_str().unwrap(), true),
            tail: py_re(raw["RE"]["browser_tail"].as_str().unwrap(), true),
            title_tails: py_re(raw["RE"]["title_tails"].as_str().unwrap(), true),
            subjects,
            sv_terms,
            raw,
        }
    })
}

fn alnum(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// re.search(r"(?<![a-z0-9])" + re.escape(word) + r"(?![a-z0-9])", low)
pub fn has_word(low: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let mut from = 0;
    while let Some(i) = low[from..].find(word) {
        let a = from + i;
        let b = a + word.len();
        if !alnum(low[..a].chars().next_back()) && !alnum(low[b..].chars().next()) {
            return true;
        }
        from = a + low[a..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }
    false
}

/// _has(low, word) from classify_subject
fn has(low: &str, word: &str) -> bool {
    if word.ends_with(' ') {
        return format!(" {low} ").contains(&format!(" {word}"));
    }
    has_word(low, word)
}

/// One study-video term: (?<![a-z0-9])core(?:s|es|'s)?(?![a-z0-9])
fn term_hit(low: &str, core: &Regex) -> bool {
    for (i, _) in low.char_indices() {
        if alnum(low[..i].chars().next_back()) {
            continue;
        }
        if let Some(m) = core.find(&low[i..]) {
            let rest = &low[i + m.end()..];
            for suf in ["s", "es", "'s", ""] {
                if let Some(after) = rest.strip_prefix(suf) {
                    if !alnum(after.chars().next()) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// study_video_subject(text, extra_words) -> the subject id (or "study") when a YouTube title is about your subjects.
pub fn study_video_subject(text: &str, extra: &[String]) -> Option<String> {
    let d = data();
    let low = text.to_lowercase();
    static P1: OnceLock<Regex> = OnceLock::new();
    static P2: OnceLock<Regex> = OnceLock::new();
    let p1 = P1.get_or_init(|| Regex::new(r#"[|:·•\[\]()#!?,."“”]+"#).unwrap());
    let p2 = P2.get_or_init(|| Regex::new(r"\s+").unwrap());
    let low = p1.replace_all(&low, " ");
    let low = p2.replace_all(&low, " ").trim().to_string();
    if low.is_empty() || d.nope.is_match(&text.to_lowercase()) {
        return None;
    }
    for w in extra {
        let w = w.trim().to_lowercase();
        if !w.is_empty() && has_word(&low, &w) {
            return Some("study".into());
        }
    }
    let cue = d.cues.is_match(&low);
    let (mut best, mut best_sc) = (None, 0.0);
    for (sid, terms) in &d.sv_terms {
        let (mut strong, mut weak) = (0, 0);
        for (core, st) in terms {
            if term_hit(&low, core) {
                if *st { strong += 1 } else { weak += 1 }
            }
        }
        let sc = 3.0 * strong as f64 + weak as f64;
        if (strong > 0 || weak >= 2 || (weak > 0 && cue)) && sc > best_sc {
            best = Some(sid.clone());
            best_sc = sc;
        }
    }
    best
}

/// classify_subject(text, learned) -> (subject, best score, runner-up score)
pub fn classify_subject(text: &str, learned: &Value) -> (Option<String>, i64, i64) {
    let low = text.to_lowercase();
    if low.trim().is_empty() {
        return (None, 0, 0);
    }
    let d = data();
    let mut scores: Vec<(String, i64)> = vec![];
    for s in &d.subjects {
        let mut sc = 0;
        if s.aliases.iter().any(|a| has(&low, a)) {
            sc += 4;
        }
        if s.sites.iter().any(|x| low.contains(x.as_str())) {
            sc += 4;
        }
        sc += 3.min(s.topics.iter().filter(|w| w.trim().chars().count() > 1 && has(&low, w.trim())).count() as i64);
        scores.push((s.sid.clone(), sc));
    }
    if let Some(m) = learned.as_object() {
        for (w, sid) in m {
            if let Some(e) = scores.iter_mut().find(|e| Some(e.0.as_str()) == sid.as_str()) {
                if has(&low, w) {
                    e.1 += 3;
                }
            }
        }
    }
    scores.sort_by_key(|e| -e.1);
    if scores.is_empty() || scores[0].1 <= 0 {
        return (None, 0, 0);
    }
    (Some(scores[0].0.clone()), scores[0].1, scores.get(1).map(|e| e.1).unwrap_or(0))
}

pub fn rule_words(s: &str) -> Vec<String> {
    s.split([',', ';', '\n']).map(|w| w.trim().to_lowercase()).filter(|w| !w.is_empty()).collect()
}

/// rule_verdict(rules, title, proc): ("keep" | "close", index of the rule) for the first matching rule.
pub fn rule_verdict(rules: &Value, title: &str, proc: &str) -> Option<(&'static str, usize)> {
    let t = title.to_lowercase();
    let p = proc.to_lowercase();
    let pn = p.strip_suffix(".exe").unwrap_or(&p).to_string();
    let aliases = &data().raw["RULE_ALIASES"];
    for (i, r) in rules.as_array()?.iter().enumerate() {
        if !r.is_object() || !r.get("on").map(|v| v.as_bool().unwrap_or(true)).unwrap_or(true) {
            continue;
        }
        let site = r.get("site").and_then(|v| v.as_str()).unwrap_or("").trim().to_lowercase();
        if !site.is_empty() && !["*", "any", "any site or app"].contains(&site.as_str()) {
            let pats = aliases.get(&site).map(strs).unwrap_or_else(|| vec![site.clone()]);
            if !pats.iter().any(|x| t.contains(x.as_str()) || *x == p || *x == pn) {
                continue;
            }
        }
        let words = rule_words(r.get("words").and_then(|v| v.as_str()).unwrap_or(""));
        if !words.is_empty() && !words.iter().any(|w| has_prefix_word(&t, w)) {
            continue;
        }
        if site.is_empty() && words.is_empty() {
            continue;
        }
        return Some((if r.get("do").and_then(|v| v.as_str()) == Some("keep") { "keep" } else { "close" }, i));
    }
    None
}

/// re.search(r"(?<![a-z0-9])" + re.escape(w), t)
fn has_prefix_word(t: &str, w: &str) -> bool {
    let mut from = 0;
    while let Some(i) = t[from..].find(w) {
        let a = from + i;
        if !alnum(t[..a].chars().next_back()) {
            return true;
        }
        from = a + t[a..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }
    false
}

/// window_key(title)
pub fn window_key(title: &str) -> String {
    let d = data();
    let mut t = title.trim().to_string();
    for _ in 0..2 {
        t = d.title_tails.replace(&t, "").to_string();
    }
    static C: OnceLock<Regex> = OnceLock::new();
    static N: OnceLock<Regex> = OnceLock::new();
    static S: OnceLock<Regex> = OnceLock::new();
    t = C.get_or_init(|| Regex::new(r"^\(\d+\)\s*").unwrap()).replace(&t, "").trim().to_lowercase();
    t = N.get_or_init(|| Regex::new(r"\d+").unwrap()).replace_all(&t, "#").to_string();
    t = S.get_or_init(|| Regex::new(r"\s+").unwrap()).replace_all(&t, " ").to_string();
    t.chars().take(70).collect()
}

pub(crate) fn pretty(k: &str) -> Option<String> {
    data().raw["PRETTY"].get(k).and_then(|v| v.as_str()).map(String::from)
}

fn title_case(s: &str) -> String {
    // str.title(): upper after any non-letter, lower otherwise
    let mut out = String::new();
    let mut prev_alpha = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            if prev_alpha { out.extend(c.to_lowercase()) } else { out.extend(c.to_uppercase()) }
            prev_alpha = true;
        } else {
            out.push(c);
            prev_alpha = false;
        }
    }
    out
}

// ---- learned.json (her habit notes) ------------------------------------------------------------------------------

pub(crate) fn load_learned() -> Value {
    let mut d = read_json("learned.json");
    if !d.is_object() {
        d = json!({});
    }
    // setdefault order: replies, sessions, distractions (the order they land in learned.json)
    for (k, v) in [("replies", json!({})), ("sessions", json!([])), ("distractions", json!({}))] {
        if d.get(k).is_none() {
            d[k] = v;
        }
    }
    d
}

pub(crate) fn save_learned(d: &Value) {
    let dir = crate::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let tmp = dir.join("learned.json.tmp");
    if std::fs::write(&tmp, crate::py_json_ascii_indent(d, b" ")).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join("learned.json"));
    }
}

pub fn learn_distraction(site: &str) {
    let mut d = load_learned();
    let k: String = site.chars().take(40).collect();
    let n = d["distractions"].get(&k).and_then(|v| v.as_i64()).unwrap_or(0);
    d["distractions"][k.as_str()] = json!(n + 1);
    save_learned(&d);
}

pub fn learn_session(planned_min: f64, focused_min: f64, completed: bool, dist_min: f64) {
    let mut d = load_learned();
    let (y, m, dd, h, _, _) = crate::local_now();
    if let Some(a) = d["sessions"].as_array_mut() {
        a.push(json!({"day": format!("{y:04}-{m:02}-{dd:02}"), "hour": h, "wd": crate::local_weekday(),
                      "planned": planned_min as i64, "focused": focused_min as i64, "done": completed,
                      "dist": dist_min as i64}));
        let k = a.len().saturating_sub(300);
        a.drain(..k);
    }
    save_learned(&d);
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

/// learned_hint(): "I've noticed you usually finish 25-min sessions and you focus best in the morning."
pub fn learned_hint() -> String {
    let d = load_learned();
    let all = d["sessions"].as_array().cloned().unwrap_or_default();
    let ss = &all[all.len().saturating_sub(80)..];
    let mut bits = vec![];
    if ss.len() >= 3 {
        let mut good_len = None;
        let mut done: Vec<i64> = ss.iter().filter(|s| s["done"].as_bool().unwrap_or(false))
            .map(|s| s["planned"].as_i64().unwrap_or(0)).collect();
        done.sort();
        if !done.is_empty() {
            good_len = Some(done[done.len() / 2]);
        }
        // best part of the day: max of (rate, n, name) over parts with n >= 2
        let mut parts: Vec<(&str, i64, i64)> = vec![];
        for s in ss {
            let p = part_of_day(s.get("hour").and_then(|v| v.as_i64()).unwrap_or(12));
            let planned = s.get("planned").and_then(|v| v.as_i64()).unwrap_or(25);
            let ok = s["done"].as_bool().unwrap_or(false)
                && s.get("dist").and_then(|v| v.as_i64()).unwrap_or(0) <= 2.max(planned.div_euclid(5));
            match parts.iter_mut().find(|e| e.0 == p) {
                Some(e) => {
                    e.1 += ok as i64;
                    e.2 += 1;
                }
                None => parts.push((p, ok as i64, 1)),
            }
        }
        let best = parts
            .iter()
            .filter(|e| e.2 >= 2)
            .map(|e| (e.1 as f64 / e.2 as f64, e.2, e.0))
            .max_by(|a, b| a.partial_cmp(b).unwrap());
        if let Some(g) = good_len {
            bits.push(format!("you usually finish {g}-min sessions"));
        }
        if let Some(b) = best {
            bits.push(format!("you focus best in the {}", b.2));
        }
    }
    if bits.is_empty() { String::new() } else { format!("I've noticed {}.", bits.join(" and ")) }
}

// ---- the watcher state ---------------------------------------------------------------------------------------------

#[derive(Default)]
pub struct Watch {
    pub alerted: bool,
    pub level: i64,
    pub last_nag: f64,
    pub dist_run: f64,
    pub warned: bool,
    pub passive_run: f64,
    pub last_suggest: f64,
    pub active_run: f64,
    pub last_break_nag: f64,
    pub yt_home_t0: f64,
    pub sv_seen: Vec<String>,
    pub auto_break: Option<(f64, i64)>,
}

pub struct Brk {
    pub planned: f64,
    pub elapsed: f64,
    pub resume: bool,
}

impl Companion {
    pub fn cfg_list(&self, k: &str) -> Vec<String> {
        match self.cfg.raw.get(k) {
            Some(v) if v.is_array() => strs(v),
            _ => strs(&data().raw["DEFAULTS"][k]),
        }
    }

    fn cfg_num(&self, k: &str) -> f64 {
        let dflt = data().raw["DEFAULTS"][k].as_f64().unwrap_or(0.0);
        self.cfg.f(k, dflt)
    }

    /// study_video(title): the subject id for a YouTube video about your subjects, "browsing" for a short while on
    /// YouTube's own pages, else None.
    pub fn study_video(&mut self, now: f64, title: &str) -> Option<String> {
        if !self.cfg.b("study_videos", true) {
            return None;
        }
        let raw = title.trim();
        static YT: OnceLock<Regex> = OnceLock::new();
        if !YT.get_or_init(|| Regex::new(r"(?i)\byoutube\b").unwrap()).is_match(raw) {
            self.watch.yt_home_t0 = 0.0;
            return None;
        }
        static R: OnceLock<[Regex; 4]> = OnceLock::new();
        let r = R.get_or_init(|| {
            [
                Regex::new(r"(?i)\s+and \d+ more pages?.*$").unwrap(),
                Regex::new(r"(?i)\s+[-\x{2013}\x{2014}|]\s+youtube\b.*$").unwrap(),
                Regex::new(r"(?i)^youtube\s+[-\x{2013}\x{2014}|:]\s+").unwrap(),
                Regex::new(r"^\(\d+\+?\)\s*").unwrap(),
            ]
        });
        let mut t = data().tail.replace(raw, "").to_string();
        for re in r.iter() {
            t = re.replace(&t, "").to_string();
        }
        let t = t.trim().to_string();
        let low = t.to_lowercase();
        if low.is_empty()
            || ["youtube", "home", "subscriptions", "history", "watch later", "library", "you", "playlists", "explore",
                "trending"].contains(&low.as_str())
        {
            if self.watch.yt_home_t0 == 0.0 {
                self.watch.yt_home_t0 = now;
            }
            return if now - self.watch.yt_home_t0 < 75.0 { Some("browsing".into()) } else { None };
        }
        self.watch.yt_home_t0 = 0.0;
        let extra = self.cfg_list("study_video_words");
        if let Some(s) = study_video_subject(&t, &extra) {
            return Some(s);
        }
        let learned = read_json("zenko_life.json").get("subj_words").cloned().unwrap_or(Value::Null);
        let (s2, sc, _) = classify_subject(&low, &learned);
        if s2.is_some() && sc >= 3 && !data().nope.is_match(&low) {
            return s2;
        }
        None
    }

    /// study_video_noted: once per video, while a session (or lock-in) is on, she says it's allowed.
    fn study_video_noted(&mut self, now: f64, title: &str) {
        if self.session.is_none() {
            return;
        }
        let key = window_key(title);
        if self.watch.sv_seen.contains(&key) {
            return;
        }
        self.watch.sv_seen.push(key);
        let k = self.watch.sv_seen.len().saturating_sub(40);
        self.watch.sv_seen.drain(..k);
        let sid = self.study_video(now, title);
        let Some(sid) = sid.filter(|s| s != "browsing") else {
            self.watch.sv_seen.pop();
            return;
        };
        let name = if sid == "study" {
            "study".to_string()
        } else {
            data().subjects.iter().find(|s| s.sid == sid).map(|s| s.short.clone()).unwrap_or(sid)
        };
        let lines = ["A %s video? That counts as studying. Allowed~", "%s video, I'll let it through. Take notes, ne~",
                     "Ooh, %s. Watching is fine, just don't wander off to the recommendations~"];
        let i = (self.rand() * 3.0) as usize;
        let l = lines[i.min(2)].replace("%s", &name);
        self.delayed.push((now + 0.01, Ev::Say { text: l, secs: 6.0, choices: vec![] }));
    }

    /// is_distracting(title, proc): the site's name when this window is a distraction (zenko17: your rules first,
    /// then the distraction list, and YouTube videos about your subjects are allowed).
    pub fn is_distracting(&mut self, now: f64, title: &str, proc: &str) -> Option<String> {
        let rules = self.cfg.raw.get("zk_rules").cloned().unwrap_or(Value::Null);
        if let Some((what, i)) = rule_verdict(&rules, title, proc) {
            if what == "keep" {
                return None;
            }
            let r = &rules[i];
            let site = r.get("site").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if site.is_empty() || site.to_lowercase().starts_with("any") {
                let w = rule_words(r.get("words").and_then(|v| v.as_str()).unwrap_or(""));
                return Some(title_case(w.first().map(|s| s.as_str()).unwrap_or("that")));
            }
            return Some(site);
        }
        let site = self.is_distracting_base(title, proc)?;
        if self.study_video(now, title).is_some() {
            self.study_video_noted(now, title);
            return None;
        }
        Some(site)
    }

    fn is_distracting_base(&self, title: &str, proc: &str) -> Option<String> {
        let t = title.to_lowercase();
        if t.is_empty() && proc.is_empty() {
            return None;
        }
        for a in self.cfg_list("allow") {
            if !a.is_empty() && t.contains(&a.to_lowercase()) {
                return None;
            }
        }
        for k in self.cfg_list("distractions") {
            if !k.is_empty() && t.contains(&k.to_lowercase()) {
                return Some(pretty(&k.to_lowercase()).unwrap_or_else(|| title_case(k.trim())));
            }
        }
        for p in self.cfg_list("distraction_apps") {
            if !p.is_empty() && p.to_lowercase() == proc {
                let stem = p.rsplit_once('.').map(|x| x.0.to_string()).unwrap_or(p.clone());
                return Some(title_case(&stem));
            }
        }
        None
    }

    pub(crate) fn pick_s(&mut self, v: &[String]) -> String {
        let i = (self.rand() * v.len() as f64) as usize;
        v[i.min(v.len().saturating_sub(1))].clone()
    }

    fn pick_nag(&mut self, ctx: &[(&str, String)]) -> String {
        let mode = self.cfg.s("mode", "mixed");
        let lvl = (self.watch.level.min(3) - 1) as usize;
        let nags = &data().raw["NAGS"];
        let g = |m: &str, i: usize| strs(&nags[m][i]);
        let pool = match mode.as_str() {
            "gentle" => g("gentle", lvl),
            "savage" => g("savage", lvl),
            _ => {
                if self.watch.level == 1 {
                    g("gentle", 0)
                } else if self.watch.level == 2 {
                    if self.rand() < 0.5 { g("gentle", 1) } else { g("savage", 1) }
                } else {
                    g("savage", lvl.min(2))
                }
            }
        };
        let mut msg = self.pick_s(&pool);
        for (k, v) in ctx {
            msg = msg.replace(&format!("{{{k}}}"), v);
        }
        let name = self.cfg.s("user_name", "").trim().split(' ').next().unwrap_or("").to_string();
        if !name.is_empty() && self.rand() < 0.35 {
            let mut c = msg.chars();
            let first: String = c.next().map(|f| f.to_lowercase().collect()).unwrap_or_default();
            msg = format!("{name}, {first}{}", c.as_str());
        }
        msg
    }

    fn nag(&mut self, now: f64, site: &str) {
        self.watch.level += 1;
        self.watch.last_nag = now;
        let s = self.session.as_ref().unwrap();
        let ctx = [
            ("task", clip(&s.task, 28)),
            ("site", site.to_string()),
            ("planned", mins(s.planned).to_string()),
            ("elapsed", mins(s.elapsed).to_string()),
            ("left", mins((s.planned - s.elapsed).max(0.0)).to_string()),
            ("dist", mins(s.dist).to_string()),
        ];
        self.set_mood(now, "alert", 3600.0);
        if self.cfg.b("learn_habits", true) {
            learn_distraction(site);
        }
        let l = self.pick_nag(&ctx);
        self.say(&l, 11.0);
        if self.cfg.b("sound", true) {
            self.events.push(Ev::Beep { kind: "nag".into(), variety: false });
        }
    }

    fn maybe_motivate(&mut self, now: f64) {
        if self.watch.alerted || now - self.watch.last_nag < 20.0 {
            return;
        }
        let every = self.cfg.f("motivate_every_minutes", 5.0).max(1.0) * 60.0;
        let s = self.session.as_mut().unwrap();
        let left = (s.planned - s.elapsed).max(0.0);
        let frac = s.elapsed / s.planned;
        let mut pool = None;
        if left <= 60.0 && !s.final_ && s.planned >= 180.0 {
            s.final_ = true;
            s.last_motiv = s.elapsed;
            pool = Some("FINAL");
        } else if frac >= 0.5 && !s.half && s.planned >= 300.0 {
            s.half = true;
            s.last_motiv = s.elapsed;
            pool = Some("HALFWAY");
        } else if s.elapsed - s.last_motiv >= every {
            s.last_motiv = s.elapsed;
            pool = Some("MOTIV");
        }
        let task = clip(&s.task, 24);
        if let Some(p) = pool {
            self.set_mood(now, "happy", 2.5);
            self.events.push(Ev::Beep { kind: "ding".into(), variety: false });
            let l = self.pick_s(&strs(&data().raw[p])).replace("{task}", &task).replace("{left}", &mins(left).to_string());
            self.say(&l, 8.0);
        }
    }

    /// maybe_close (zenko21.1): warn at the grace time, close the tab or app 10 seconds after the warning.
    fn maybe_close(&mut self, site: &str, proc: &str) {
        if !self.cfg.b("auto_close", true) {
            return;
        }
        let warn_at = { let v = self.cfg.f("grace_seconds", 10.0); if v == 0.0 { 10.0 } else { v } };
        let after = { let v = self.cfg.f("close_after_warning", 10.0); if v == 0.0 { 10.0 } else { v } };
        let limit = warn_at + after.max(3.0);
        let task = clip(&self.session.as_ref().unwrap().task, 24);
        if self.watch.dist_run < warn_at {
            self.watch.warned = false;
            return;
        }
        if self.watch.dist_run < limit {
            if !self.watch.warned {
                self.watch.warned = true;
                let l = format!("Closing {} in {} seconds unless you go back to \u{201c}{}\u{201d}!", site,
                                (limit - self.watch.dist_run).round_ties_even() as i64, task);
                self.say(&l, 8.0);
                self.events.push(Ev::Beep { kind: "nag".into(), variety: false });
            }
            return;
        }
        self.watch.dist_run = 0.0;
        self.watch.warned = false;
        let browsers = strs(&data().raw["BROWSERS"]);
        let apps: Vec<String> = self.cfg_list("distraction_apps").iter().map(|p| p.to_lowercase()).collect();
        let kind = if browsers.iter().any(|b| b == proc) {
            "tab"
        } else if apps.iter().any(|a| a == proc) {
            "window"
        } else {
            let l = format!("I can't close {} safely. Please close it yourself and get back to \u{201c}{}\u{201d}.", site, task);
            self.say(&l, 8.0);
            return;
        };
        // the window side closes it (Ctrl+W for a browser tab, WM_CLOSE for an app) and reports back
        self.events.push(Ev::Command(format!("close:{kind}:{site}")));
    }

    /// close_distraction finished (from the window side).
    pub fn closed_distraction(&mut self, now: f64, site: &str, ok: bool) {
        if ok {
            if let Some(s) = &mut self.session {
                s.closed += 1;
            }
            let task = self.session.as_ref().map(|s| clip(&s.task, 28)).unwrap_or_default();
            self.set_mood(now, "alert", 2.0);
            self.events.push(Ev::Beep { kind: "nag".into(), variety: false });
            self.say(&format!("Closed {site}. Back to \u{201c}{task}\u{201d}!"), 8.0);
        } else {
            self.say(&format!("I couldn't close {site}. Please close it yourself."), 8.0);
        }
    }

    // ---- breaks --------------------------------------------------------------------------------------------------

    fn break_tick(&mut self, now: f64, dt: f64, idle: f64) {
        if let Some(b) = &mut self.brk {
            b.elapsed += dt;
            if b.elapsed >= b.planned {
                self.end_break(now, true);
            }
            return;
        }
        if idle >= 180.0 {
            self.watch.active_run = 0.0; // you already stepped away long enough: that counts as a break
        } else if idle < 120.0 {
            self.watch.active_run += dt;
        }
        let limit = self.cfg.f("break_after_minutes", 50.0) * 60.0;
        if limit > 0.0 && self.watch.active_run >= limit && now - self.watch.last_break_nag > 900.0 {
            self.watch.last_break_nag = now;
            self.set_mood(now, "happy", 3.0);
            self.events.push(Ev::Beep { kind: "ding".into(), variety: false });
            let l = data().raw["BREAK_NUDGE"].as_str().unwrap_or("")
                .replacen("%d", &mins(self.watch.active_run).to_string(), 1)
                .replacen("%d", &(self.cfg.f("break_minutes", 5.0) as i64).to_string(), 1);
            self.say(&l, 14.0);
        }
    }

    pub fn start_break(&mut self, now: f64, minutes: Option<i64>) {
        let _ = now;
        let m = minutes.unwrap_or(self.cfg.f("break_minutes", 5.0) as i64);
        let resume = self.session.as_ref().is_some_and(|s| !s.paused);
        if resume {
            if let Some(s) = &mut self.session {
                s.paused = true;
            }
        }
        self.brk = Some(Brk { planned: m as f64 * 60.0, elapsed: 0.0, resume });
        self.watch.active_run = 0.0;
        self.watch.alerted = false;
        self.watch.dist_run = 0.0;
        self.mood_override = None;
        self.events.push(Ev::Beep { kind: "happy".into(), variety: false });
        self.say(&format!("Break time! {m} min. Stretch, drink water, look away from the screen."), 10.0);
    }

    pub fn end_break(&mut self, now: f64, done: bool) {
        let Some(b) = self.brk.take() else { return };
        let mut resumed = false;
        if let Some(s) = &mut self.session {
            if b.resume {
                s.paused = false;
                resumed = true;
            }
        }
        self.mood_override = None;
        self.set_mood(now, "happy", 3.0);
        self.events.push(Ev::Beep { kind: "happy".into(), variety: false });
        let l = format!("{}{}", if done { "Break over! Ready to go?" } else { "Break ended. Back to it!" },
                        if resumed { " Your session is running again." } else { "" });
        self.say(&l, 8.0);
        self.pomo_break_done(now);
    }

    /// zenko.py tick() -> _tick(), once a second. `title`/`proc` = the window you're using.
    pub fn tick_1s(&mut self, now: f64, dt: f64, idle: f64, title: &str, proc: &str) {
        if now >= self.brain.ema_next {
            self.brain.ema_next = now + 5.0;
            self.ema_tick(now);
        }
        if let Some((at, bm)) = self.watch.auto_break {
            if now >= at {
                self.watch.auto_break = None;
                if self.session.is_none() && self.brk.is_none() {
                    self.start_break(now, Some(bm));
                }
            }
        }
        let site = self.is_distracting(now, title, proc);
        self.study.fg = Some((title.to_string(), proc.to_string(), idle, site.clone()));
        let away = idle >= self.cfg_num("idle_minutes") * 60.0;
        self.check_timetable(now);
        self.break_tick(now, dt, idle);
        if self.brk.is_some() {
            return;
        }
        let has = self.session.is_some();
        if self.sleeping && !away && idle < 3.0 {
            self.sleeping = false;
            self.say(if has { "Welcome back! Let's keep going." } else { "Hi again!" }, 5.0);
        }
        if away && !self.sleeping {
            self.sleeping = true;
            if self.session.as_ref().is_some_and(|s| !s.paused) {
                self.say("Still there? I'll keep the timer running.", 8.0);
            }
        }
        if let Some(s) = &mut self.session {
            if s.paused {
                return;
            }
            if away && self.cfg.b("idle_pause", true) {
                s.idle_paused = true; // idle detection: the timer waits for you
                if self.watch.alerted {
                    self.watch.alerted = false;
                    self.watch.dist_run = 0.0;
                    self.mood_override = None;
                }
                return;
            }
            if s.idle_paused {
                s.idle_paused = false;
                self.say("Okaeri~ I paused your timer while you were away. Carrying on!", 5.0);
            }
            let s = self.session.as_mut().unwrap();
            s.elapsed += dt;
            if away {
                s.away += dt;
                if self.watch.alerted {
                    self.watch.alerted = false;
                    self.watch.dist_run = 0.0;
                    self.mood_override = None;
                }
            } else if let Some(site) = &site {
                s.dist += dt;
                self.watch.dist_run += dt;
                if self.watch.dist_run >= self.cfg_num("grace_seconds") {
                    if !self.watch.alerted {
                        self.watch.alerted = true;
                        self.watch.level = 0;
                        self.nag(now, site);
                    } else if now - self.watch.last_nag >= self.cfg_num("nag_every_seconds") {
                        self.nag(now, site);
                    }
                }
                self.maybe_close(site, proc);
            } else {
                if self.watch.alerted {
                    self.watch.alerted = false;
                    self.mood_override = None;
                    self.set_mood(now, "happy", 4.0);
                    self.events.push(Ev::Beep { kind: "happy".into(), variety: false });
                    let l = self.pick_s(&strs(&data().raw["PRAISE"]));
                    self.say(&l, 5.0);
                }
                self.watch.dist_run = 0.0;
                self.maybe_motivate(now);
            }
            if self.session.as_ref().is_some_and(|s| s.elapsed >= s.planned) {
                self.end_session(now, true);
            }
        } else if self.cfg.b("watch_without_session", true) {
            if let (Some(site), false) = (&site, away) {
                self.watch.passive_run += dt;
                if self.watch.passive_run >= self.cfg_num("passive_minutes") * 60.0 && now - self.watch.last_suggest > 900.0
                {
                    self.watch.last_suggest = now;
                    self.set_mood(now, "alert", 6.0);
                    let l = self.pick_s(&strs(&data().raw["SUGGEST"])).replace("{site}", site);
                    self.say(&l, 10.0);
                    if self.cfg.b("sound", true) {
                        self.events.push(Ev::Beep { kind: "nag".into(), variety: false });
                    }
                }
            } else {
                self.watch.passive_run = 0.0;
            }
        }
    }
}
