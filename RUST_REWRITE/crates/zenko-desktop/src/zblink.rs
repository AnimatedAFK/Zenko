//! The Zenko Browser link (zenko.py Zenko21 zb_inbox_tick / zb_inbox_item, Zenko22 z22_link_payload / z22_link_tick /
//! z22_reading_note / z22_in_*, Zenko23 z22_link_payload + z23_session_info / z23_session_watch) and the built-in
//! summariser it uses (summarise_offline, summarise_extract, _sentences, _terms, _stem).
//!
//! The browser drops what you send it into %APPDATA%\FoxBuddy\browser_inbox.json; Zenko takes the file every 5 s.
//! Every 15 s (and at once when a focus session starts or stops) she writes zenko_link.json, which the browser's new
//! tab reads: your deadlines for the next three weeks, today's plan, the ema rack and the running session.

use crate::plan::{day_n, getrandbits36, iso, load_deadlines, naive, parse_iso, truthy};
use crate::rewards::Later;
use crate::session::clip;
use crate::setpage::APP_VERSION;
use crate::{Companion, Ev, data_dir, read_json, unix_now};
use regex::Regex;
use serde_json::{Value, json};
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

/// self._zb_seen, _zb_was_linked, _z23_ses, and the summaries whose "Save to notes" button is still up
#[derive(Default)]
pub struct Zb {
    pub seen_title: Option<String>,
    pub seen_t: f64,
    pub noted: f64,
    pub was_linked: Option<bool>,
    pub ses: Option<String>,
    pub next_inbox: f64,
    pub next_link: f64,
    pub next_watch: f64,
    pub saves: Vec<(String, Value)>,
}

// ---- the built-in summariser ------------------------------------------------------------------------------------
const STOP: &str = "a an the of to in on at for and or is are was were be been being do does did done what who whom whose \
    when where why how which that this these those it its with by from as about into than then so if not no \
    can could would should will shall may might must i me my you your u ur we our they them their he she his \
    her him tell explain define meaning mean means please pls search look up find out know google wiki \
    something anything thing things really just like also very much many some any there here have has had \
    get got one two use used using way ways";

fn stop() -> &'static std::collections::HashSet<&'static str> {
    static S: OnceLock<std::collections::HashSet<&'static str>> = OnceLock::new();
    S.get_or_init(|| STOP.split_whitespace().collect())
}

/// str.isspace / re's \s on text
fn py_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

fn plen(s: &str) -> usize {
    s.chars().count()
}

fn py_strip(s: &str) -> &str {
    s.trim_matches(py_space)
}

/// _sentences(text): split after . ! ? before a capital, digit, quote or bracket; 35-420 characters, no cookie banners
pub fn sentences(text: &str) -> Vec<String> {
    let junk = rx!(r"(?i)cookie|subscribe|sign up|sign in|log in|javascript|newsletter|all rights reserved|©|click here|advertis|privacy policy|terms of (use|service)|your browser|enable js|accept all");
    let cs: Vec<char> = text.chars().collect();
    let mut parts = vec![];
    let (mut start, mut i) = (0usize, 0usize);
    while i < cs.len() {
        if py_space(cs[i]) && i > 0 && matches!(cs[i - 1], '.' | '!' | '?') {
            let mut j = i;
            while j < cs.len() && py_space(cs[j]) {
                j += 1;
            }
            if j < cs.len() && (cs[j].is_ascii_uppercase() || cs[j].is_ascii_digit() || matches!(cs[j], '"' | '\u{201c}' | '(')) {
                parts.push(cs[start..i].iter().collect::<String>());
                start = j;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    parts.push(cs[start..].iter().collect::<String>());
    parts.iter().map(|p| py_strip(p).to_string()).filter(|p| (35..=420).contains(&plen(p)) && !junk.is_match(p)).collect()
}

fn stem(w: &str) -> String {
    if w.len() > 4 {
        if let Some(b) = w.strip_suffix("ies") {
            return format!("{b}y");
        }
        if ["oes", "ses", "xes", "zes", "ches", "shes"].iter().any(|e| w.ends_with(e)) {
            return w[..w.len() - 2].to_string();
        }
        if w.ends_with('s') && !["ss", "us", "is"].iter().any(|e| w.ends_with(e)) {
            return w[..w.len() - 1].to_string();
        }
    }
    w.to_string()
}

/// _terms(text)
pub fn terms(text: &str) -> Vec<String> {
    rx!(r"[a-z0-9]+").find_iter(&text.to_lowercase()).map(|m| m.as_str()).filter(|w| !stop().contains(w) && w.len() >= 2)
        .map(stem).collect()
}

/// summarise_extract(question, docs, n, limit): the Luhn method (no AI). docs are (title, text).
pub fn summarise_extract(question: &str, docs: &[(String, String)], n: usize, limit: usize) -> String {
    use std::collections::{HashMap, HashSet};
    let qt: HashSet<String> = terms(question).into_iter().collect();
    let mut cands: Vec<(usize, usize, String, HashSet<String>)> = vec![];
    let mut freq: HashMap<String, i64> = HashMap::new();
    for (di, d) in docs.iter().enumerate() {
        for (si, s) in sentences(&d.1).into_iter().take(60).enumerate() {
            let ts = terms(&s);
            if ts.len() < 4 {
                continue;
            }
            let st: HashSet<String> = ts.into_iter().collect();
            for w in &st {
                *freq.entry(w.clone()).or_insert(0) += 1;
            }
            cands.push((di, si, s, st));
        }
    }
    if cands.is_empty() {
        return String::new();
    }
    let top = *freq.values().max().unwrap() as f64;
    let defn = rx!(r"\b(is|are|was|were|refers to|means)\b");
    let nd = docs.len().max(1) as f64;
    let score = |c: &(usize, usize, String, HashSet<String>)| -> f64 {
        let (di, si, s, st) = c;
        let cover = if qt.is_empty() { 0.0 } else { qt.intersection(st).count() as f64 / qt.len() as f64 };
        let central = st.iter().map(|w| freq[w]).sum::<i64>() as f64 / (top * st.len() as f64);
        let lead = if *si == 0 { 0.35 } else if *si < 3 { 0.15 } else { 0.0 };
        let head: String = s.chars().take(80).collect();
        let df = if defn.is_match(&head) && *si < 4 { 0.3 } else { 0.0 };
        2.2 * cover + central + lead + df - 0.15 * *di as f64 / nd
    };
    let mut order: Vec<(f64, usize)> = cands.iter().enumerate().map(|(i, c)| (score(c), i)).collect();
    order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let head3 = |s: &str| s.to_lowercase().split_whitespace().take(3).collect::<Vec<_>>().join(" ");
    let mut chosen: Vec<usize> = vec![];
    let mut per_doc: HashMap<usize, usize> = HashMap::new();
    for (_, ci) in order {
        let c = &cands[ci];
        if chosen.len() >= n || (!chosen.is_empty() && chosen.iter().map(|&x| plen(&cands[x].2)).sum::<usize>() + plen(&c.2) > limit + 120) {
            break;
        }
        if per_doc.get(&c.0).copied().unwrap_or(0) >= 2 {
            continue;
        }
        let h = head3(&c.2);
        if chosen.iter().any(|&x| {
            let o = &cands[x];
            c.3.intersection(&o.3).count() as f64 / c.3.union(&o.3).count() as f64 > 0.35 || head3(&o.2) == h
        }) {
            continue;
        }
        chosen.push(ci);
        *per_doc.entry(c.0).or_insert(0) += 1;
    }
    chosen.sort_by_key(|&x| (cands[x].0, cands[x].1));
    chosen.iter().map(|&x| cands[x].2.as_str()).collect::<Vec<_>>().join(" ")
}

/// summarise_offline(name, body)
pub fn summarise_offline(name: &str, body: &str) -> String {
    let b: String = body.chars().take(60000).collect();
    let text = rx!(r"[ \t]*\n[ \t]*\n\s*").replace_all(&b, ". ").to_string();
    let cs: Vec<char> = text.chars().collect();
    let mut t2 = String::with_capacity(text.len());
    for (i, &c) in cs.iter().enumerate() {
        if c == '\n' {
            let nx = cs.get(i + 1).copied();
            if nx.is_some_and(|n| n.is_ascii_uppercase() || n.is_ascii_digit() || matches!(n, '\u{2022}' | '-' | '*')) {
                t2.push_str(". ");
            } else {
                t2.push(' ');
            }
        } else {
            t2.push(c);
        }
    }
    let text = rx!(r"\.\s*\.").replace_all(&t2, ".").to_string();
    let sents = sentences(&text);
    let k = (sents.len() / 8 + 1).max(1);
    let docs: Vec<(String, String)> = sents.chunks(k).map(|c| (name.to_string(), c.join(" "))).collect();
    let pts = summarise_extract(name, &docs, 7, 1300);
    if pts.is_empty() {
        return format!("I read \u{201c}{name}\u{201d} but couldn't find much to summarise.");
    }
    let out: Vec<String> = sentences(&pts).into_iter().map(|x| py_strip(&x).to_string()).filter(|x| !x.is_empty()).collect();
    format!("Kon! The key points of \u{201c}{name}\u{201d}:\n") + &out.iter().take(7).map(|x| format!("\u{2022} {x}")).collect::<Vec<_>>().join("\n")
}

// ---- reading what the browser sent --------------------------------------------------------------------------------
fn falsy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => !b,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s.is_empty(),
        Some(Value::Array(a)) => a.is_empty(),
        Some(Value::Object(o)) => o.is_empty(),
    }
}

/// (it.get(k) or "") as a string; anything else truthy fails where Python's .strip() / + would
fn sget(it: &Value, k: &str) -> Result<String, String> {
    let v = it.get(k);
    if falsy(v) {
        return Ok(String::new());
    }
    match v {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("AttributeError: {k} is not text")),
    }
}

/// it.get(a) or it.get(b) or ""
fn sget2(it: &Value, a: &str, b: &str) -> Result<String, String> {
    if !falsy(it.get(a)) { sget(it, a) } else { sget(it, b) }
}

/// float(it.get("when") or 0)
fn fget(it: &Value, k: &str) -> Result<f64, String> {
    let v = it.get(k);
    if falsy(v) {
        return Ok(0.0);
    }
    match v.unwrap() {
        Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0)),
        Value::Bool(_) => Ok(1.0),
        Value::String(s) => {
            let t = py_strip(s).replace('_', "");
            match t.to_ascii_lowercase().as_str() {
                "inf" | "+inf" | "infinity" | "+infinity" => Ok(f64::INFINITY),
                "-inf" | "-infinity" => Ok(f64::NEG_INFINITY),
                "nan" | "+nan" | "-nan" => Ok(f64::NAN),
                _ => t.parse::<f64>().map_err(|_| format!("ValueError: could not convert string to float: {s:?}")),
            }
        }
        _ => Err("TypeError: float() argument".into()),
    }
}

fn first_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn squash(s: &str) -> String {
    rx!(r"\s+").replace_all(s, " ").to_string()
}

/// z22_split_card(text)
pub fn split_card(text: &str) -> Option<(String, String)> {
    let t = squash(text);
    let t = t.trim_matches([' ', '.']);
    for sep in [" = ", " - ", " \u{2013} ", " \u{2014} ", ": "] {
        if let Some((q, a)) = t.split_once(sep) {
            if !py_strip(q).is_empty() && !py_strip(a).is_empty() {
                return Some((py_strip(q).to_string(), py_strip(a).to_string()));
            }
        }
    }
    let m = rx!(r"(?i)^(.{3,80}?)\s+(?:is|are|was|were|means|refers to)\s+(.{3,})$").captures(t)?;
    Some((format!("What is {}?", py_strip(&m[1])), py_strip(&m[2]).to_string()))
}

const Z22_STOP: &str = "about above after again against because before being below between could doesn during further \
    having other their there these those through under until which while would should where whose \
    every another however therefore although";

/// z22_cloze(body, n=6): fill-in-the-blank cards from a page
pub fn cloze(body: &str, n: usize) -> Vec<(String, String)> {
    let stops: std::collections::HashSet<&str> = Z22_STOP.split_whitespace().collect();
    let b: String = body.chars().take(20000).collect();
    let mut out = vec![];
    let mut seen = std::collections::HashSet::new();
    for s in sentences(&squash(&b)) {
        let s = py_strip(&s).to_string();
        if !(50..=200).contains(&plen(&s)) || s.matches(' ').count() < 7 {
            continue;
        }
        let words: Vec<&str> = rx!(r"[A-Za-z][A-Za-z\-]{5,}").find_iter(&s).map(|m| m.as_str())
            .filter(|w| !stops.contains(w.to_lowercase().as_str())).collect();
        if words.is_empty() {
            continue;
        }
        let caps: Vec<&str> = words.iter().copied().filter(|w| w.starts_with(|c: char| c.is_ascii_uppercase()) && !s.starts_with(w)).collect();
        let pool = if caps.is_empty() { &words } else { &caps };
        let mut key = pool[0];
        for w in pool.iter() {
            if w.len() > key.len() {
                key = w;
            }
        }
        if !seen.insert(key.to_lowercase()) {
            continue;
        }
        let re = Regex::new(&format!(r"\b{}\b", regex::escape(key))).unwrap();
        let q = re.replacen(&s, 1, "_____").to_string();
        out.push((q, key.to_string()));
        if out.len() >= n {
            break;
        }
    }
    out
}

const SITES: [(&str, &str); 15] = [("youtube.com", "YouTube"), ("wikipedia.org", "Wikipedia"), ("khanacademy.org", "Khan Academy"),
    ("savemyexams.com", "Save My Exams"), ("revisionvillage.com", "Revision Village"), ("quizlet.com", "Quizlet"),
    ("ibo.org", "the IB site"), ("managebac.com", "ManageBac"), ("docs.google.com", "Google Docs"),
    ("classroom.google.com", "Google Classroom"), ("chemguide.co.uk", "Chemguide"), ("desmos.com", "Desmos"),
    ("wordreference.com", "WordReference"), ("bbc.co.uk", "BBC"), ("reddit.com", "Reddit")];

fn write_atomic(name: &str, body: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())?;
    let p = data_dir().join(name);
    let tmp = data_dir().join(format!("{name}.tmp"));
    std::fs::write(&tmp, body)?;
    std::fs::rename(tmp, p)
}

impl Companion {
    fn zb_log(&self, text: &str) {
        crate::update::log_error(text, APP_VERSION);
    }

    /// the clock for the link's timers starts with the app (zb_inbox_tick after 9 s, z22_link_tick after 7 s,
    /// z23_session_watch after 4 s); call once a frame. live(block) puts a note into the open Notes page's
    /// scratchpad and says whether it could; fishing is the garden's cast in progress; idle is idle_seconds().
    /// Returns true when the sidebar's link light changed.
    pub fn zb_tick(&mut self, now: f64, idle: f64, fishing: bool, live: &mut dyn FnMut(&str) -> bool) -> bool {
        if self.zb.next_inbox == 0.0 {
            self.zb.next_inbox = now + 9.0;
            self.zb.next_link = now + 7.0;
            self.zb.next_watch = now + 4.0;
        }
        let mut changed = false;
        if now >= self.zb.next_watch {
            self.zb.next_watch = now + 2.0;
            self.z23_session_watch();
        }
        if now >= self.zb.next_link {
            self.zb.next_link = now + 15.0;
            changed = self.z22_link_tick(now, idle, fishing);
        }
        if now >= self.zb.next_inbox {
            self.zb.next_inbox = now + 5.0;
            self.zb_inbox_tick(now, live);
        }
        changed
    }

    /// zb_inbox_tick: take browser_inbox.json and handle each thing in it
    pub fn zb_inbox_tick(&mut self, now: f64, live: &mut dyn FnMut(&str) -> bool) {
        let path = data_dir().join("browser_inbox.json");
        if !path.exists() {
            return;
        }
        let take = data_dir().join("browser_inbox.json.take");
        let res: Result<(), String> = (|| {
            std::fs::rename(&path, &take).map_err(|e| format!("{e}"))?;
            let raw = std::fs::read(&take).map_err(|e| format!("{e}"))?;
            let items: Value = serde_json::from_slice(&raw).map_err(|e| format!("JSONDecodeError({e})"))?;
            std::fs::remove_file(&take).map_err(|e| format!("{e}"))?;
            let mut said: Option<String> = None;
            if let Value::Array(a) = items {
                for it in a.iter().filter(|it| it.is_object()) {
                    if let Some(s) = self.zb_inbox_item(now, it, live)? {
                        said = Some(s);
                    }
                }
            }
            if let Some(s) = said {
                self.say(&s, 7.0);
            }
            Ok(())
        })();
        if let Err(e) = res {
            self.zb_log(&format!("browser inbox: {e}"));
        }
    }

    /// Zenko22.zb_inbox_item: deadlines, summaries, cards and quizzes; the rest as Zenko21 (notes, clips, reminders)
    pub fn zb_inbox_item(&mut self, now: f64, it: &Value, live: &mut dyn FnMut(&str) -> bool) -> Result<Option<String>, String> {
        let kind = sget(it, "kind").unwrap_or_else(|_| "?".into());
        let kind = if falsy(it.get("kind")) { "note".to_string() } else { kind };
        let r = match kind.as_str() {
            "deadline" => self.z22_in_deadline(now, it),
            "summary" => self.z22_in_summary(now, it),
            "card" => self.z22_in_card(it),
            "quiz" => self.z22_in_quiz(now, it),
            _ => return self.zb_inbox_note(it, live),
        };
        r.or_else(|e| {
            self.zb_log(&format!("browser inbox {kind}: {e}"));
            Ok(None)
        })
    }

    /// Zenko21.zb_inbox_item: a reminder on the ema rack, or a note / picture clip in the Notes scratchpad
    pub fn zb_inbox_note(&mut self, it: &Value, live: &mut dyn FnMut(&str) -> bool) -> Result<Option<String>, String> {
        let kind = if falsy(it.get("kind")) { "note".to_string() } else { sget(it, "kind").unwrap_or_default() };
        let title = first_chars(py_strip(&sget(it, "title")?), 140);
        let url = py_strip(&sget(it, "url")?).to_string();
        if kind == "reminder" {
            let when = fget(it, "when")?;
            let text = first_chars(py_strip(&squash(&sget(it, "text")?)), 150);
            if text.is_empty() {
                return Ok(None);
            }
            self.ema_add(&text, if when == 0.0 { None } else { Some(when) }, None);
            return Ok(Some("Kon! I hung that on the ema rack for you~".into()));
        }
        let mut text = py_strip(&sget(it, "text")?).to_string();
        if kind == "clip" {
            text = if text.is_empty() { String::new() } else { text + "\n" } + "[picture clip] " + &sget(it, "path")?;
        }
        if text.is_empty() {
            return Ok(None);
        }
        let subj = py_strip(&sget(it, "subject")?).to_string();
        let block = format!("\n{}{}{text}{}\n", if !subj.is_empty() && subj != "Core" { format!("{subj}\n") } else { String::new() },
                            if title.is_empty() { String::new() } else { format!("{title}\n") },
                            if url.is_empty() { String::new() } else { format!("\n(from {url})") });
        if !live(&block) {
            use std::io::Write;
            let r = std::fs::create_dir_all(data_dir()).and_then(|_| {
                let mut f = std::fs::OpenOptions::new().create(true).append(true).open(crate::notes::nb_scratch())?;
                f.write_all(block.as_bytes())
            });
            if let Err(e) = r {
                self.zb_log(&format!("browser inbox note: {e}"));
                return Ok(None);
            }
        }
        Ok(Some("Got it~ I put that in your Notes scratchpad. It files itself when you open Notes.".into()))
    }

    fn z22_in_deadline(&mut self, now: f64, it: &Value) -> Result<Option<String>, String> {
        let title = first_chars(py_strip(&squash(&sget2(it, "text", "title")?)), 120);
        let mut date_iso = py_strip(&sget(it, "date")?).to_string();
        let mut tm = py_strip(&sget(it, "time")?).to_string();
        if title.is_empty() {
            return Ok(None);
        }
        if !rx!(r"^\d{4}-\d{2}-\d{2}$").is_match(&date_iso) {
            let when = fget(it, "when")?;
            if when == 0.0 {
                return Ok(Some("I couldn't find a date in that. Try selecting the date too~".into()));
            }
            if !when.is_finite() {
                return Err("OverflowError: timestamp out of range".into());
            }
            let nl = naive(when);
            date_iso = iso(day_n(nl));
            let sod = nl.rem_euclid(86400.0) as i64;
            let (h, m) = (sod / 3600, sod / 60 % 60);
            if tm.is_empty() && (h, m) != (9, 0) {
                tm = format!("{h:02}:{m:02}");
            }
        }
        self.add_deadline(now, &title, &date_iso, &tm, None, "browser");
        let d = parse_iso(&date_iso).filter(|&d| iso(d) == date_iso).ok_or_else(|| format!("ValueError: Invalid isoformat string: {date_iso:?}"))?;
        Ok(Some(format!("Added to Upcoming: \u{201c}{}\u{201d} on {}~", clip(&title, 50), crate::lifechat::a_d_b(d).replace(" 0", " "))))
    }

    fn z22_in_summary(&mut self, now: f64, it: &Value) -> Result<Option<String>, String> {
        let t0 = sget(it, "title")?;
        let title = clip(if falsy(it.get("title")) { "this page" } else { py_strip(&t0) }, 80);
        let body = py_strip(&sget(it, "text")?).to_string();
        let url = sget(it, "url")?;
        if plen(&body) < 80 {
            return Ok(Some("There wasn't enough text there for me to summarise~".into()));
        }
        self.say(&format!("Reading \u{201c}{}\u{201d}... give me a moment~", clip(&title, 50)), 6.0);
        // work(): the online brain first when it's set up (stage 5b), else the built-in summariser
        let res = summarise_offline(&title, &body);
        if res.is_empty() {
            self.later(now + 0.3, Later::Say { text: "Hmm, I couldn't summarise that one~".into(), secs: 6.0, choices: vec![],
                                               only_idle: false, beep: None });
            return Ok(None);
        }
        self.chat_line(now, "Fox", &(res.clone() + &if url.is_empty() { String::new() } else { format!("\n(from {url})") }));
        let first = res.split('\n').find_map(|x| x.strip_prefix("\u{2022} ")).unwrap_or("");
        let id = format!("{:x}", getrandbits36(&mut self.brain.rng));
        self.zb.saves.push((id.clone(), json!({"kind": "note", "title": format!("Summary: {title}"), "text": res, "url": url})));
        if self.zb.saves.len() > 20 {
            self.zb.saves.remove(0);
        }
        self.later(now + 0.3, Later::Say { text: format!("Done~ {} (the rest is in Chat)", clip(first, 110)), secs: 22.0,
                                           choices: vec![("Save to notes".into(), format!("zb_save:{id}")),
                                                         ("Open chat".into(), "open_hub:chat".into())],
                                           only_idle: false, beep: None });
        Ok(None)
    }

    /// a summary's "Save to notes" button
    pub fn zb_save(&mut self, id: &str, live: &mut dyn FnMut(&str) -> bool) {
        let Some(k) = self.zb.saves.iter().position(|(i, _)| i == id) else { return };
        let it = self.zb.saves[k].1.clone();
        let _ = self.zb_inbox_note(&it, live);
    }

    fn z22_in_card(&mut self, it: &Value) -> Result<Option<String>, String> {
        let (mut q, mut a) = (py_strip(&sget(it, "q")?).to_string(), py_strip(&sget(it, "a")?).to_string());
        if q.is_empty() || a.is_empty() {
            match split_card(&sget(it, "text")?) {
                Some(qa) => (q, a) = qa,
                None => return Ok(Some("Select something like \u{201c}osmosis - water moving across a membrane\u{201d} and I'll make it a card~".into())),
            }
        }
        Ok(Some(self.card_add(&q, &a)))
    }

    fn z22_in_quiz(&mut self, _now: f64, it: &Value) -> Result<Option<String>, String> {
        let t0 = sget(it, "title")?;
        let title = clip(if falsy(it.get("title")) { "this page" } else { py_strip(&t0) }, 60);
        let cards = cloze(&sget(it, "text")?, 6);
        if cards.is_empty() {
            return Ok(Some("I couldn't make questions from that page. Try one with more sentences~".into()));
        }
        let today = crate::today_iso();
        let made: Vec<Value> = cards.iter().map(|(q, a)| {
            json!({"id": format!("{:x}", getrandbits36(&mut self.brain.rng)), "q": first_chars(q, 200), "a": first_chars(a, 200),
                   "box": 0, "wrong": 0, "right": 0, "added": today, "due": today, "from": "browser"})
        }).collect();
        self.life_do(|l| {
            if let Some(c) = l["cards"].as_array_mut() {
                c.extend(made);
            }
        });
        self.events.push(Ev::Say { text: format!("I made {} fill-in-the-blank cards from \u{201c}{title}\u{201d}~ Ready?", cards.len()),
                                   secs: 25.0, choices: vec![("Quiz me".into(), "hub_ask:review flashcards".into()), ("Later".into(), String::new())] });
        Ok(None)
    }

    // ---- what Zenko tells the browser -----------------------------------------------------------------------------
    /// z23_session_info
    pub fn z23_session_info(&self) -> Value {
        if let Some(l) = &self.lockin {
            return json!({"on": true, "kind": "lockin", "task": "Lock-in", "until": l.until, "started": format!("l{:x}", l.t0.to_bits() >> 12)});
        }
        if let Some(s) = &self.session
            && self.brk.is_none() {
            let left = (s.planned - s.elapsed).max(0.0);
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (s.task.as_str(), s.planned.to_bits(), s.item).hash(&mut h);
            return json!({"on": true, "kind": "focus", "task": clip(&s.task, 50), "until": unix_now() + left,
                          "started": format!("s{:x}", h.finish() >> 16)});
        }
        json!({"on": false})
    }

    /// Zenko23.z22_link_payload
    pub fn z22_link_payload(&self) -> Value {
        let today = crate::today_iso();
        let td = parse_iso(&today).unwrap_or(0);
        let end = iso(td + 21);
        let mut dl = vec![];
        for d in load_deadlines() {
            let ds = d["date"].as_str().unwrap_or("").to_string();
            if today <= ds && ds <= end && !truthy(&d["done"]) {
                dl.push(json!({"title": clip(d["title"].as_str().unwrap_or(""), 80), "date": ds,
                               "time": d["time"].as_str().unwrap_or(""), "kind": d["kind"].as_str().unwrap_or("")}));
            }
        }
        let plan: Vec<Value> = crate::tt::tt_today(&self.tt, &today).into_iter().map(|i| {
            let it = &self.tt[i];
            json!({"start": it.get("start").cloned().unwrap_or(Value::Null), "mins": it.get("mins").cloned().unwrap_or(Value::Null),
                   "task": clip(it["task"].as_str().unwrap_or(""), 60), "done": crate::tt::tt_done(it, &today)})
        }).collect();
        let ema: Vec<Value> = self.ema_active().iter().take(12).map(|e| {
            json!({"text": clip(e["text"].as_str().unwrap_or(""), 80), "at": e.get("at").cloned().unwrap_or(Value::Null)})
        }).collect();
        let mut d = json!({"app": "zenko", "version": APP_VERSION, "time": unix_now(), "pid": std::process::id(),
                           "name": self.fox_name(), "deadlines": dl.into_iter().take(30).collect::<Vec<_>>(),
                           "today": plan.into_iter().take(20).collect::<Vec<_>>(), "ema": ema});
        d["session"] = self.z23_session_info();
        d["ema_plan"] = self.tt.iter().filter(|i| i["plan"].as_str().is_some_and(|p| p.starts_with("ema:"))
            && i["date"].as_str().unwrap_or("") >= today.as_str()).take(12)
            .map(|i| json!({"task": clip(i["task"].as_str().unwrap_or(""), 60), "date": i.get("date").cloned().unwrap_or(Value::Null),
                            "start": i.get("start").cloned().unwrap_or(Value::Null), "mins": i.get("mins").cloned().unwrap_or(Value::Null)}))
            .collect::<Vec<_>>().into();
        d
    }

    fn write_link(&self) -> std::io::Result<()> {
        write_atomic("zenko_link.json", &crate::squarelink::py_dumps(&self.z22_link_payload()))
    }

    /// z22_link_tick: zenko_link.json, the reading note, and whether the sidebar's light changed
    pub fn z22_link_tick(&mut self, now: f64, idle: f64, fishing: bool) -> bool {
        if self.cfg.b("zb_link_on", true) {
            if let Err(e) = self.write_link() {
                self.zb_log(&format!("zb link: {e}"));
                return false;
            }
            self.z22_reading_note(now, idle, fishing);
        }
        let was = self.zb.was_linked;
        let linked = self.zb_linked();
        self.zb.was_linked = Some(linked);
        was.is_some_and(|w| w != linked)
    }

    /// z23_session_watch: the link file at once when a session or lock-in starts or stops
    pub fn z23_session_watch(&mut self) {
        let info = self.z23_session_info();
        let key = if info["on"] == true { info["started"].as_str().map(String::from) } else { None };
        if key != self.zb.ses {
            self.zb.ses = key;
            if self.cfg.b("zb_link_on", true)
                && let Err(e) = self.write_link() {
                self.zb_log(&format!("z23 session link: {e}"));
            }
        }
    }

    /// z22_reading_note: what you've settled on reading in Zenko Browser (at most every ten minutes)
    pub fn z22_reading_note(&mut self, _now: f64, idle: f64, fishing: bool) {
        if !self.cfg.b("zb_reading_note", true) || !self.zb_linked() {
            return;
        }
        let d = read_json("browser_now.json");
        let title = py_strip(d["title"].as_str().unwrap_or("")).to_string();
        let host = d["host"].as_str().unwrap_or("").to_string();
        if title.is_empty() || title == "New tab" || host.is_empty() || host == "newtab" {
            self.zb.seen_title = None;
            return;
        }
        let t = unix_now();
        if self.zb.seen_title.as_deref() != Some(title.as_str()) {
            self.zb.seen_title = Some(title);
            self.zb.seen_t = t;
            return;
        }
        if t - self.zb.seen_t < 100.0 || t - self.zb.noted < 600.0 {
            return;
        }
        if self.session.is_some() || self.lockin.is_some() || fishing || idle > 240.0 {
            return;
        }
        self.zb.noted = t;
        let host = host.to_lowercase();
        let site = SITES.iter().find(|(k, _)| host == *k || host.ends_with(&format!(".{k}"))).map(|(_, v)| v.to_string())
            .unwrap_or_else(|| host.replace("www.", ""));
        let learned = self.life_now()["subj_learned"].clone();
        let sid = crate::watch::classify_subject(&format!("{title} {host}"), &learned).0;
        let tc = clip(&title, 50);
        let line = match sid {
            Some(sid) => {
                let n = crate::study::sname(&sid);
                let opts = [format!("Ooh, {n} on {site}? \u{201c}{tc}\u{201d} looks useful~"),
                            format!("Studying {n}~ I'm counting it. (\u{201c}{tc}\u{201d})"),
                            format!("Kon~ reading \u{201c}{tc}\u{201d}? Right-click the page if you want me to quiz you.")];
                self.brain.rng.choice(&opts).clone()
            }
            None => {
                let opts = [format!("What'cha reading? \u{201c}{tc}\u{201d} on {site}~"),
                            format!("Mm, \u{201c}{tc}\u{201d}... right-click it in the browser and I'll summarise it.")];
                self.brain.rng.choice(&opts).clone()
            }
        };
        self.say(&line, 7.0);
    }
}
