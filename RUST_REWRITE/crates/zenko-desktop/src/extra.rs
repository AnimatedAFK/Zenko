//! zenko.py ZenkoExtras.extra_route: lock-in, pomodoro, "focus 45 on essay", the old reminders, the weekly report,
//! "quiz me on", the calculator, define / translate (online lookups: they go to the host as commands), her quick notes,
//! to-dos, the omikuji, the Japanese word of the day, singing, outfits, coats, her name, treats and play, her treasure
//! box, your bond, sulking and sorry. Plus self.extra (zenko_extras.json with load_extras' defaults).

use crate::calc::{CalcErr, calc_eval, fmt_num};
use crate::plan::{day_n, iso, naive};
use crate::routes17::{py_title, routes_data};
use crate::play::Play;
use crate::{Companion, Ev, read_json, unix_now, update_json};
use fancy_regex::Regex as FRegex;
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
macro_rules! frx {
    ($p:expr) => {{
        static R: OnceLock<FRegex> = OnceLock::new();
        R.get_or_init(|| FRegex::new($p).unwrap())
    }};
}

/// load_extras(): zenko_extras.json with its defaults filled in
pub fn extras() -> Value {
    let mut d = read_json("zenko_extras.json");
    if !d.is_object() {
        d = json!({});
    }
    for (k, v) in [("notes", json!([])), ("todos", json!([])), ("reminders", json!([])), ("bond", json!(10)),
                   ("treasures", json!([])), ("eaten", json!({})), ("omikuji", json!({})), ("treats", json!(0))] {
        if d.get(k).is_none() {
            d[k] = v;
        }
    }
    d
}

/// change self.extra, then save_extra()
pub fn extras_do<T>(f: impl FnOnce(&mut Value) -> T) -> T {
    let mut d = extras();
    let r = f(&mut d);
    update_json("zenko_extras.json", |v| *v = d);
    r
}

/// clip(text, n)
pub fn clip(text: &str, n: usize) -> String {
    if text.chars().count() <= n { text.to_string() } else { text.chars().take(n - 1).collect::<String>() + "…" }
}

fn take(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn s_of(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

/// "%a" for a local naive day
pub fn wday(day: i64) -> &'static str {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][(day + 3).rem_euclid(7) as usize]
}

/// datetime.now().strftime("%Y-%m-%d %H:%M")
pub fn now_stamp() -> String {
    let n = naive(unix_now()).floor() as i64;
    let d = n.div_euclid(86400);
    let s = n.rem_euclid(86400);
    format!("{} {:02}:{:02}", iso(d), s / 3600, s % 3600 / 60)
}

/// notes_index(): {name: path} of the reMarkable notebooks already read (in listdir order, as Python sees it)
pub fn notes_index() -> Vec<(String, std::path::PathBuf)> {
    let dir = crate::data_dir().join("remarkable").join("notes");
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut names: Vec<String> = rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        for f in names {
            if let Some(n) = f.strip_suffix(".md") {
                out.push((n.to_string(), dir.join(&f)));
            }
        }
    }
    out
}

/// notes_find(query): the best-matching notebook, or None
pub fn notes_find(query: &str) -> Option<(String, std::path::PathBuf)> {
    let idx = notes_index();
    let q = query.to_lowercase();
    let q = q.trim();
    let mut best = None;
    let mut sc = 0.0;
    for (name, path) in idx {
        let nl = name.to_lowercase();
        let mut s = crate::pydifflib::ratio(q, &nl) + if !q.is_empty() && nl.contains(q) { 0.6 } else { 0.0 };
        let body = std::fs::read_to_string(&path).unwrap_or_default();
        if !q.is_empty() && body.to_lowercase().contains(q) {
            s += 0.3;
        }
        if s > sc {
            best = Some((name, path));
            sc = s;
        }
    }
    if sc > 0.45 { best } else { None }
}

impl Companion {
    /// cfg_set(k, v)
    pub fn cfg_set(&mut self, k: &str, v: Value) {
        crate::update_config(&mut self.cfg, |c| c[k] = v.clone());
    }

    /// apply_look(), as the chat commands call it (tests see it in `trace`)
    pub fn look_changed(&mut self) {
        self.trace.push("apply_look".into());
        self.apply_look();
    }

    /// set_outfit(name, on): toggles when on is None; returns whether it's on now
    pub fn set_outfit(&mut self, name: &str, on: Option<bool>) -> bool {
        let mut cur: Vec<Value> = self.cfg.raw.get("outfit").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let has = cur.iter().any(|v| v.as_str() == Some(name));
        let on = on.unwrap_or(!has);
        if on && !has {
            cur.push(json!(name));
        }
        if !on && has {
            let i = cur.iter().position(|v| v.as_str() == Some(name)).unwrap();
            cur.remove(i);
        }
        self.cfg_set("outfit", Value::Array(cur));
        self.look_changed();
        on
    }

    /// bond_text(): (hearts, name, bond)
    pub fn bond_text(&self) -> (String, &'static str, i64) {
        let b = extras()["bond"].as_f64().unwrap_or(10.0) as i64;
        let hearts = 5.min(b.div_euclid(20) + if b >= 5 { 1 } else { 0 });
        const NAMES: [&str; 6] = ["Strangers", "New friends", "Good friends", "Best friends", "Shrine partners", "Soul foxes"];
        let h = hearts.max(0) as usize;
        ("\u{2665}".repeat(h) + &"\u{2661}".repeat(5 - h), NAMES[h.min(5)], b)
    }

    /// web_job: online lookups go to the host (stage 5b); with lookups off she says so
    pub fn web_job(&mut self, now: f64, cmd: String, intro: &str) -> String {
        if !self.cfg.b("web_enabled", true) {
            return "Internet lookups are off. Turn them on in Settings (or right-click me) and ask again~".into();
        }
        self.later(now, crate::rewards::Later::Command(cmd));
        intro.to_string()
    }

    /// quiz_start(topic)
    pub fn quiz_start(&mut self, now: f64, topic: &str) -> String {
        if !self.cfg.b("web_enabled", true) {
            return "Internet lookups are off, so I can't fetch flashcards. Turn them on in Settings~".into();
        }
        self.later(now, crate::rewards::Later::Command(format!("quiz_start:{topic}")));
        format!("Kon kon~ making flashcards on '{topic}'... (one moment)")
    }

    /// The omikuji's own random.Random(): seeded from the OS, or fixed by tests
    fn omikuji_rng(&mut self) -> PyRandom {
        match self.omikuji_seed {
            Some(s) => PyRandom::new(s),
            None => {
                let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
                PyRandom::new((t as u64) ^ (std::process::id() as u64) << 32)
            }
        }
    }

    /// omikuji(): today's fortune (one a day)
    pub fn omikuji(&mut self, now: f64) -> String {
        let today = iso(day_n(naive(unix_now())));
        let mut o = extras()["omikuji"].clone();
        let again = o.get("date").and_then(|v| v.as_str()) == Some(today.as_str());
        if !again {
            let rd = routes_data();
            let mut rnd = self.omikuji_rng();
            let om = rd["OMIKUJI"].as_array().unwrap();
            let mut cum = vec![];
            let mut tot = 0.0;
            for x in om {
                tot += x[4].as_f64().unwrap();
                cum.push(tot);
            }
            let r = rnd.random() * tot;
            let k = cum[..om.len() - 1].partition_point(|c| *c <= r);
            let pick = &om[k];
            let lvl = pick[3].as_i64().unwrap();
            let mut lines = serde_json::Map::new();
            for (cat, v) in rd["OMI_LINES"].as_object().unwrap() {
                let pool = v[lvl.min(2) as usize].as_array().unwrap();
                lines.insert(cat.clone(), rnd.choice(pool).clone());
            }
            let item = rnd.choice(rd["OMI_LUCKY"].as_array().unwrap()).clone();
            let colour = rnd.choice(rd["OMI_COLOURS"].as_array().unwrap()).clone();
            o = json!({"date": today, "kanji": pick[0], "rank": pick[1], "gloss": pick[2], "lines": lines,
                       "item": item, "colour": colour});
            let o2 = o.clone();
            extras_do(|e| e["omikuji"] = o2);
            self.events.push(Ev::Beep { kind: "omikuji".into(), variety: false });
            let kind = if lvl == 0 { "boop" } else { "pet" };
            self.play = Some(Play { kind: kind.into(), t0: now, dur: 2.4, arg: None, found: false });
        }
        let mut out = vec![
            if again { "You already drew today's omikuji (one a day, ne~):" } else { "*shake shake* ...Your omikuji says:" }.to_string(),
            format!("  {}  {} - {}", s_of(&o["kanji"]), s_of(&o["rank"]), s_of(&o["gloss"])),
        ];
        if let Some(l) = o["lines"].as_object() {
            for (cat, line) in l {
                out.push(format!("  {cat}: {}", s_of(line)));
            }
        }
        out.push(format!("  Lucky item: {}  \u{b7}  Lucky colour: {}", s_of(&o["item"]), s_of(&o["colour"])));
        let rank = s_of(&o["rank"]);
        if ["Kyou", "Suekichi", "Hankichi"].contains(&rank.as_str()) && !crate::plan::truthy(&o["tied"]) {
            out.push("Don't worry~ say 'tie it' and I'll tie it to the shrine branch so the bad luck stays behind!".into());
        } else if rank == "Daikichi" || rank == "Kichi" {
            out.push("Sugoi~! Inari is smiling on you today!".into());
        }
        out.join("\n")
    }

    pub(crate) fn extra_route(&mut self, now: f64, text: &str) -> Option<String> {
        let raw = text.trim();
        let low = raw.to_lowercase();
        let low = low.trim();
        let lw = low.trim_end_matches([' ', '!', '.', '?', '~']);
        let nm = self.fox_name();
        // --- lock-in
        if let Some(m) = rx!(r"^(?:lock(?:ed)?[ -]?in|lockin|full focus(?: mode)?|focus mode)(?: mode)?(?: on)?(?: for)?\s*(\d{1,3})?\s*(?:m|min|mins|minutes|h|hr|hrs|hours?)?$").captures(lw) {
            if let Some(l) = &self.lockin {
                let left = ((l.until - unix_now()) / 60.0).floor() as i64;
                return Some(format!("We're already locked in! {} min to go. Say 'unlock' to stop.", left.max(0)));
            }
            let mut n: Option<i64> = m.get(1).map(|g| g.as_str().parse().unwrap());
            if let Some(v) = n {
                if v != 0 && rx!(r"\d\s*(h|hr|hrs|hours?)$").is_match(lw) {
                    n = Some(v * 60);
                }
            }
            return self.lockin_start(now, n, false);
        }
        if rx!(r"^(unlock|exit lock[ -]?in|stop lock[ -]?in|end lock[ -]?in|lock[ -]?in off|lock out|i'?m done locking in)$").is_match(lw) {
            return Some(self.lockin_stop(now, false, true));
        }
        // --- pomodoro and custom sessions
        if let Some(m) = rx!(r"^(?:pomodoro|pomo|tomato)\s*(?:(\d{1,3})\s*[/:x\- ]\s*(\d{1,3}))?(?:\s*(?:x|times|rounds?)?\s*(\d{1,2})\s*(?:rounds?|x)?)?$").captures(lw) {
            let g = |i: usize, d: i64| m.get(i).map(|x| x.as_str().parse::<i64>().unwrap()).unwrap_or(d);
            let (w, b, r) = (g(1, 25), g(2, 5), g(3, 4));
            return Some(self.pomo_start(now, w.clamp(1, 180), b.clamp(1, 60), r.clamp(1, 12)));
        }
        if rx!(r"^(stop|end|cancel) (the )?pomodoro").is_match(lw) {
            self.pomo = None;
            if self.session.is_some() {
                self.end_session(now, false);
            }
            return Some("Pomodoro stopped. Good effort, ne~".into());
        }
        if let Some(m) = rx!(r"^(?:focus|study|work)(?: for)?\s+(\d{1,3})\s*(m|min|mins|minutes|h|hr|hrs|hours?)?(?:\s+(?:on|for)\s+(.+))?$").captures(lw) {
            let n: i64 = m[1].parse::<i64>().unwrap() * if m.get(2).is_some_and(|u| u.as_str().starts_with('h')) { 60 } else { 1 };
            if self.session.is_some() {
                return Some("A session is already running. Say 'stop' first.".into());
            }
            let task = crate::chat::capitalize(m.get(3).map(|t| t.as_str()).unwrap_or("Focus").trim());
            self.start_session(now, &task, n.clamp(1, 600) as f64);
            return Some(format!("Focus session: {n} min on \u{201c}{task}\u{201d}. Ganbatte~kon!"));
        }
        // --- reminders (zenko18's ema rack catches most of these first)
        let r1 = rx!(r"^remind me (?:to )?(.+?) in (\d+(?:\.\d+)?)\s*(seconds?|secs?|s|minutes?|mins?|m|hours?|hrs?|hr|h)\b$").captures(lw);
        let r2 = if r1.is_none() {
            rx!(r"^remind me in (\d+(?:\.\d+)?)\s*(seconds?|secs?|s|minutes?|mins?|m|hours?|hrs?|hr|h)\b\s*(?:to |that |about )?(.+)$").captures(lw)
        } else {
            None
        };
        if r1.is_some() || r2.is_some() {
            let (what, n, unit) = match (&r1, &r2) {
                (Some(m), _) => (m[1].to_string(), m[2].parse::<f64>().unwrap(), m[3].to_string()),
                (_, Some(m)) => (m[3].to_string(), m[1].parse::<f64>().unwrap(), m[2].to_string()),
                _ => unreachable!(),
            };
            let sec = n * if unit.starts_with('h') { 3600.0 } else if unit.starts_with('s') { 1.0 } else { 60.0 };
            let rawc: Vec<char> = raw.chars().collect();
            let wn = what.chars().count();
            let what = if raw.to_lowercase().ends_with(&what) && rawc.len() >= wn {
                rawc[rawc.len() - wn..].iter().collect()
            } else {
                what
            };
            self.ema_add(&what, Some(unix_now() + sec), None);
            let mn = (sec / 60.0).round_ties_even() as i64;
            let nice = if sec < 60.0 {
                format!("{} sec", sec as i64)
            } else if mn < 60 {
                format!("{mn} min")
            } else {
                format!("{} h {:02} min", mn / 60, mn % 60)
            };
            return Some(format!("Okay~ I'll remind you to {what} in {nice}. Kon!"));
        }
        let a1 = rx!(r"^remind me (?:to )?(.+?) at ([\d:.]+\s*(?:am|pm|a|p)?)$").captures(lw);
        let a2 = if a1.is_none() { rx!(r"^remind me at ([\d:.]+\s*(?:am|pm|a|p)?)\s*(?:to |that |about )?(.+)$").captures(lw) } else { None };
        if a1.is_some() || a2.is_some() {
            let (what, clock) = match (&a1, &a2) {
                (Some(m), _) => (m[1].to_string(), m[2].to_string()),
                (_, Some(m)) => (m[2].to_string(), m[1].to_string()),
                _ => unreachable!(),
            };
            if let Some(hm) = crate::tt::parse_clock(&clock) {
                let nn = naive(unix_now());
                let today = day_n(nn);
                let h: i64 = hm[..2].parse().unwrap();
                let mi: i64 = hm[3..].parse().unwrap();
                let mut t = (today * 86400 + h * 3600 + mi * 60) as f64;
                if t <= nn {
                    t += 86400.0;
                }
                self.ema_add(&what, Some(unix_now() + (t - nn)), None);
                let tmr = if day_n(t) != today { " tomorrow" } else { "" };
                return Some(format!("Okay~ I'll remind you to {what} at {hm}{tmr}."));
            }
        }
        if rx!(r"^(my |show |list )?(my )?reminders?$|^what are my reminders").is_match(lw) {
            let rem = extras()["reminders"].as_array().cloned().unwrap_or_default();
            if rem.is_empty() {
                return Some("No reminders set. Try: remind me in 20 min to stretch".into());
            }
            let rows: Vec<String> = rem.iter().enumerate().map(|(i, r)| {
                let t = naive(r["at"].as_f64().unwrap_or(0.0)).floor() as i64;
                let (d, s) = (t.div_euclid(86400), t.rem_euclid(86400));
                format!("  {}. {} {:02}:{:02}  -  {}", i + 1, wday(d), s / 3600, s % 3600 / 60, s_of(&r["text"]))
            }).collect();
            return Some(format!("Your reminders:\n{}", rows.join("\n")));
        }
        if rx!(r"^(cancel|clear|delete|remove) (all )?(my )?reminders?$").is_match(lw) {
            let n = extras_do(|e| {
                let n = e["reminders"].as_array().map(|a| a.len()).unwrap_or(0);
                e["reminders"] = json!([]);
                n
            });
            return Some(format!("Cleared {n} reminder{}.", if n == 1 { "" } else { "s" }));
        }
        // --- weekly report
        if rx!(r"how (was|is|did) my week|weekly (report|stats|summary)|week(?:ly)? (?:focus )?report|this week'?s (stats|focus)|my week\b").is_match(lw) {
            return Some(self.week_report());
        }
        // --- quiz
        if rx!(r"^(stop|end|quit|cancel) (the )?quiz$").is_match(lw) {
            return Some("No quiz running. Say 'quiz me on <topic>' to start one~".into());
        }
        if let Some(m) = rx!(r"^(?:quiz|test) me(?: on| about)?\s+(.+)$|^flash ?cards?(?: on| about| for)?\s+(.+)$").captures(lw) {
            let topic = m.get(1).or(m.get(2)).map(|g| g.as_str()).unwrap_or("").trim().to_string();
            if rx!(r"\bnotes?\b").is_match(&topic) || notes_find(&topic).is_some() {
                return None; // her reMarkable notes quiz handles this
            }
            return Some(self.quiz_start(now, &topic));
        }
        // --- calculator
        let cm = rx!(r"^(?:calc|calculate|compute|maths?|=)\s*:?\s*(.+)$").captures(lw);
        let mut expr = cm.as_ref().map(|m| m[1].to_string());
        if expr.is_none() && rx!(r"^[\d\s.,+\-*/^()%x×÷=]+$").is_match(lw) && rx!(r"\d").is_match(lw)
            && rx!(r"[+\-*/^%x×÷]").is_match(lw) {
            expr = Some(lw.to_string());
        }
        if expr.is_none() {
            expr = rx!(r"^(?:what(?:'s| is)\s+)?(\d[\d.,]*\s*%\s*(?:of|off)\s*\d[\d.,]*|\d[\d.,]*\s*(?:plus|minus|times|divided by|x|\*|/|\+|\-|\^)\s*\d[\d.,]*(?:\s*(?:plus|minus|times|divided by|x|\*|/|\+|\-|\^)\s*\d[\d.,]*)*)\s*\??$")
                .captures(lw).map(|mm| mm[1].to_string());
        }
        if let Some(expr) = expr {
            match calc_eval(&expr) {
                Ok(v) => return Some(format!("{} = {}", expr.trim(), fmt_num(&v))),
                Err(CalcErr::ZeroDiv) => return Some("Uwaa~ dividing by zero breaks the universe! Let's not~".into()),
                Err(CalcErr::Other) => {
                    if cm.is_some() {
                        return Some("Hmm hmm~ I couldn't work that out. Try like: calc 15% of 240, or calc (3+4)*2^3".into());
                    }
                }
            }
        }
        // --- define
        if let Some(m) = rx!(r#"^(?:define|definition of|meaning of|what does)\s+["']?([a-z][a-z\- ']{0,40}?)["']?(?:\s+mean)?\s*$"#).captures(lw) {
            if lw.starts_with("define") || lw.starts_with("definition") || lw.starts_with("meaning") || lw.ends_with("mean") {
                let word = m[1].trim().to_string();
                return Some(self.web_job(now, format!("define:{word}"), &format!("Opening my dictionary for '{word}'~")));
            }
        }
        // --- translate
        let traw = raw.trim_matches([' ', '?', '!', '.']);
        let tm = rx!(r"(?i)^translate\s+(.+?)\s+(?:from\s+([a-z ]+?)\s+)?(?:to|into|in)\s+([a-z ]+?)\s*$").captures(traw);
        if tm.is_some() || rx!(r"^translate\b").is_match(lw) {
            let Some(m) = tm else {
                return Some("Tell me the language too~ e.g. translate good luck on your exam to Japanese".into());
            };
            let txt = m[1].trim_matches([' ', '"', '\'']).to_string();
            let src = m.get(2).map(|g| g.as_str().to_lowercase().trim().to_string()).unwrap_or_default();
            let dst = m[3].to_lowercase().trim().to_string();
            let langs = &routes_data()["LANGS"];
            let code = langs.get(&dst).and_then(|v| v.as_str()).map(String::from)
                .or_else(|| rx!(r"^[a-z]{2}(-[a-z]{2})?$").is_match(&dst).then(|| dst.clone()));
            let Some(code) = code else {
                return Some(format!("Ne~ I don't know the language '{dst}'. Try Japanese, Spanish, French, Hindi, Korean..."));
            };
            let scode = if src.is_empty() { "auto".to_string() } else {
                langs.get(&src).and_then(|v| v.as_str()).unwrap_or("auto").to_string()
            };
            let dt = py_title(&dst);
            return Some(self.web_job(now, format!("translate:{code}:{scode}:{dt}:{txt}"), &format!("Translating to {dt}~")));
        }
        // --- quick notes (her own notes; your reMarkable notes still work too)
        if let Ok(Some(m)) = frx!(r"(?i)^(?:note|take a note|add note|new note|jot(?: down)?)\s*[:\-]\s*(.+)$|^note (?!to self\b)(?!s\b)(.+)$|^note to self[:,]?\s*(.+)$").captures(raw) {
            let body = m.get(1).or(m.get(2)).or(m.get(3)).map(|g| g.as_str()).unwrap_or("").trim().to_string();
            let n = extras_do(|e| {
                let a = e["notes"].as_array_mut().unwrap();
                a.push(json!({"t": now_stamp(), "text": take(&body, 1000)}));
                a.len()
            });
            return Some(format!("Noted~ (note {n}). Say 'my notes' to see them all."));
        }
        if rx!(r"^(my |show (my )?|list (my )?|see (my )?|read (my )?)?(quick )?notes$|^what are my notes").is_match(lw) {
            let notes = extras()["notes"].as_array().cloned().unwrap_or_default();
            let mut idx: Vec<String> = notes_index().into_iter().map(|(n, _)| n).collect();
            idx.sort();
            let mut out = vec![];
            if !notes.is_empty() {
                let k = notes.len().saturating_sub(25);
                let rows: Vec<String> = notes[k..].iter().enumerate().map(|(i, n)| {
                    let t = s_of(&n["t"]);
                    format!("  {}. {}  ({})", i + 1, s_of(&n["text"]), t.chars().skip(5).collect::<String>())
                }).collect();
                out.push(format!("Your notes:\n{}", rows.join("\n")));
            }
            if !idx.is_empty() {
                out.push(format!("reMarkable notebooks I've read: {}", idx.iter().take(12).cloned().collect::<Vec<_>>().join(", ")));
            }
            return Some(if out.is_empty() { "No notes yet. Try: note: buy more aburaage".into() } else { out.join("\n\n") });
        }
        if let Some(m) = rx!(r"^(?:delete|remove|forget) note\s+#?(\d+)$").captures(lw) {
            let i = m[1].parse::<i64>().unwrap_or(0) - 1;
            let gone = extras_do(|e| {
                let a = e["notes"].as_array_mut().unwrap();
                if i >= 0 && (i as usize) < a.len() { Some(a.remove(i as usize)) } else { None }
            });
            return Some(match gone {
                Some(g) => format!("Poof~ deleted note: {}", clip(&s_of(&g["text"]), 60)),
                None => format!("I don't have a note {}.", &m[1]),
            });
        }
        if rx!(r"^(clear|delete|erase) (all )?(my )?(quick )?notes$").is_match(lw) {
            extras_do(|e| e["notes"] = json!([]));
            return Some("All your quick notes are cleared~".into());
        }
        // --- to-dos
        if let Some(m) = rx!(r"(?i)^(?:add )?(?:todo|to-do|to do|task)\s*[:\-]?\s+(.+)$").captures(raw) {
            if !rx!(r"^(?:todo|to-do|to do)s?\s*(list)?$").is_match(lw) {
                let item = m[1].trim().to_string();
                if !["list", "s"].contains(&item.to_lowercase().as_str()) {
                    let today = iso(day_n(naive(unix_now())));
                    let left = extras_do(|e| {
                        let a = e["todos"].as_array_mut().unwrap();
                        a.push(json!({"text": take(&item, 200), "done": null, "added": today}));
                        a.iter().filter(|t| !crate::plan::truthy(&t["done"])).count()
                    });
                    return Some(format!("Added to your to-dos: {item}  ({left} to go)"));
                }
            }
        }
        if rx!(r"^(my |show (my )?|list (my )?|see (my )?)?(to-?dos?|to do|todo list|to-do list|tasks)( list)?$|^what'?s on my (to-?do|to do)").is_match(lw) {
            let todos = extras()["todos"].as_array().cloned().unwrap_or_default();
            if todos.is_empty() {
                return Some("Your to-do list is empty. Add one: todo finish essay".into());
            }
            let open: Vec<&Value> = todos.iter().filter(|t| !crate::plan::truthy(&t["done"])).collect();
            let done: Vec<&Value> = todos.iter().filter(|t| crate::plan::truthy(&t["done"])).collect();
            let mut out = vec!["To-do:".to_string()];
            if open.is_empty() {
                out.push("  (nothing left, sugoi~!)".into());
            }
            for (i, t) in open.iter().enumerate() {
                out.push(format!("  {}. \u{2610} {}", i + 1, s_of(&t["text"])));
            }
            if !done.is_empty() {
                out.push("Done:".into());
                for t in &done[done.len().saturating_sub(6)..] {
                    out.push(format!("  \u{2611} {}", s_of(&t["text"])));
                }
            }
            out.push("(say 'done 1' or 'done essay' to tick one off)".into());
            return Some(out.join("\n"));
        }
        if let Some(m) = rx!(r"^(?:done|finished|completed|tick|check off|did)\s*:?\s+(.+)$").captures(lw) {
            let key = m[1].trim().to_string();
            let todos = extras()["todos"].as_array().cloned().unwrap_or_default();
            let open: Vec<usize> = (0..todos.len()).filter(|&i| !crate::plan::truthy(&todos[i]["done"])).collect();
            let texts: Vec<String> = open.iter().map(|&i| s_of(&todos[i]["text"]).to_lowercase()).collect();
            let mut pick = None;
            let num = (!key.is_empty() && key.chars().all(|c| c.is_ascii_digit())).then(|| key.parse::<usize>().ok()).flatten();
            if let Some(k) = num.filter(|k| *k >= 1 && *k <= open.len()) {
                pick = Some(open[k - 1]);
            } else {
                let best = crate::pydifflib::get_close_matches(&key, &texts, 1, 0.4);
                if let Some(b) = best.first() {
                    pick = open.iter().zip(&texts).find(|(_, t)| *t == b).map(|(i, _)| *i);
                }
                if pick.is_none() {
                    pick = open.iter().zip(&texts).find(|(_, t)| t.contains(&key)).map(|(i, _)| *i);
                }
            }
            if let Some(p) = pick {
                let today = iso(day_n(naive(unix_now())));
                extras_do(|e| e["todos"][p]["done"] = json!(today));
                self.events.push(Ev::Beep { kind: "happy".into(), variety: false });
                if self.brain.rng.random() < 0.3 {
                    self.interact(now, "praise", None);
                } else {
                    self.set_mood(now, "happy", 3.0);
                }
                let left = open.len() - 1;
                let tail = if left > 0 { format!("{left} left!") } else { "That's everything! Sugoi~!".into() };
                return Some(format!("Yatta~! Done: {}. {tail}", s_of(&todos[p]["text"])));
            }
            if !open.is_empty() {
                return None;
            }
        }
        if rx!(r"^(clear|delete|remove) (done|finished|completed) (to-?dos?|tasks)$|^clear (my )?(to-?dos?|to do list|todo list)$").is_match(lw) {
            let only_done = lw.contains("done") || lw.contains("finished") || lw.contains("completed");
            extras_do(|e| {
                if only_done {
                    let a: Vec<Value> = e["todos"].as_array().cloned().unwrap_or_default().into_iter()
                        .filter(|t| !crate::plan::truthy(&t["done"])).collect();
                    e["todos"] = Value::Array(a);
                } else {
                    e["todos"] = json!([]);
                }
            });
            return Some("To-do list tidied up~".into());
        }
        // --- omikuji
        if rx!(r"^(?:draw |get |give me |my |tell my |what'?s my )?(?:an? )?(?:omikuji|fortune|my fortune|today'?s fortune|fortune ?telling|luck)(?: today| please)?$").is_match(lw) {
            return Some(self.omikuji(now));
        }
        if rx!(r"^(tie it|tie the fortune|tie it to the branch|tie it to the tree)$").is_match(lw) {
            let o = extras()["omikuji"].clone();
            let rank = s_of(&o["rank"]);
            if ["Kyou", "Suekichi", "Hankichi"].contains(&rank.as_str()) && !crate::plan::truthy(&o["tied"]) {
                extras_do(|e| e["omikuji"]["tied"] = json!(true));
                self.events.push(Ev::Beep { kind: "ding".into(), variety: false });
                return Some("*ties the paper to the shrine's pine branch* There! The bad luck stays here, and you go on with good luck~kon!".into());
            }
            return Some("Ehehe~ that fortune is a good one! Keep it in your wallet for luck~".into());
        }
        // --- Japanese word of the day
        if rx!(r"word of the day|japanese word|teach me (a )?(japanese|nihongo)|nihongo|today'?s word").is_match(lw) {
            let words = routes_data()["JP_WORDS"].as_array().unwrap();
            let ord = day_n(naive(unix_now())) + 719163;
            let mut i = ord.rem_euclid(words.len() as i64) as usize;
            if lw.contains("another") || lw.contains("more") || lw.contains("random") {
                i = self.brain.rng.randbelow(words.len() as u64) as usize;
            }
            let w = &words[i];
            return Some(format!("Today's Japanese word~\n  {}  ({})  {}\n  = {}\n  e.g. {}", s_of(&w[0]), s_of(&w[1]), s_of(&w[2]),
                                s_of(&w[3]), s_of(&w[4])));
        }
        // --- sing
        if rx!(r"^(sing|sing (me )?a song|sing for me|sing something|play a tune|koto)$").is_match(lw) {
            return Some(self.interact(now, "sing", None));
        }
        // --- outfits and looks
        let rd = routes_data();
        if rx!(r"^(outfits?|accessor(y|ies)|what can you wear|wardrobe)$").is_match(lw) {
            let on: Vec<String> = self.cfg.raw.get("outfit").and_then(|v| v.as_array()).map(|a| a.iter().map(s_of).collect()).unwrap_or_default();
            let names: Vec<String> = rd["OUTFITS"].as_array().unwrap().iter().map(|o| s_of(&o[0])).filter(|n| n != "glasses_always").collect();
            return Some(format!("I can wear: {}, glasses (always).\nWearing now: {}.\nSay 'outfit crown' to put one on or take it off, \
                                 'outfit none' to clear, or open the Customize page.", names.join(", "),
                                if on.is_empty() { "nothing extra".to_string() } else { on.join(", ") }));
        }
        let om = rx!(r"^(?:outfit|wear|put on|equip|dress (?:up )?(?:in|as)?|try on)\s+(?:a |an |the |your |my )?(.+)$").captures(lw)
            .or_else(|| rx!(r"^(take off|remove|unequip)\s+(?:the |your |a )?(.+)$").captures(lw));
        if let Some(m) = om {
            let off = lw.starts_with("take off") || lw.starts_with("remove") || lw.starts_with("unequip");
            let what = m.get(m.len() - 1).unwrap().as_str().trim().to_string();
            if ["none", "nothing", "off", "all", "everything", "default"].contains(&what.as_str()) {
                self.cfg_set("outfit", json!([]));
                self.look_changed();
                return Some("All accessories off~ Just plain fluffy me!".into());
            }
            if let Some(mb) = rx!(r"^(?:bib|shrine bib|scarf)\s*(\w+)?").captures(&what) {
                let want = mb.get(1).map(|g| g.as_str().to_lowercase()).unwrap_or("red".into());
                let bibs = rd["BIB_COLOURS"].as_array().unwrap();
                let col = bibs.iter().find(|b| s_of(&b[0]).to_lowercase() == want).map(|b| s_of(&b[1]));
                let Some(col) = col else {
                    let names: Vec<String> = bibs[1..].iter().map(|b| s_of(&b[0]).to_lowercase()).collect();
                    return Some(format!("Bib colours: {}", names.join(", ")));
                };
                self.cfg_set("scarf", json!(if off { String::new() } else { col }));
                self.look_changed();
                return Some(format!("Bib {}~ How do I look?", if off { "off" } else { "on" }));
            }
            let ow = &rd["OUTFIT_WORDS"];
            let parts: Vec<&str> = what.split_whitespace().collect();
            let key = ow.get(&what).or_else(|| parts.first().and_then(|p| ow.get(*p)))
                .or_else(|| parts.last().and_then(|p| ow.get(*p))).and_then(|v| v.as_str()).map(String::from);
            if let Some(key) = key {
                let on = self.set_outfit(&key, if off { Some(false) } else if !lw.starts_with("outfit") { Some(true) } else { None });
                self.set_mood(now, "happy", 3.0);
                self.events.push(Ev::Beep { kind: "happy".into(), variety: false });
                let label = rd["OUTFITS"].as_array().unwrap().iter().find(|o| o[0] == key.as_str()).map(|o| s_of(&o[1])).unwrap_or_default();
                return Some(if on { format!("{label} on! Ehehe~ kawaii?") } else { format!("{label} off~") });
            }
            if lw.starts_with("outfit") {
                return Some(format!("Ne~ I don't have '{what}'. Say 'outfits' to see what I can wear."));
            }
        }
        if let Some(m) = rx!(r"^(?:coat|fur|change (?:your )?(?:coat|fur)(?: colou?r)?(?: to)?|be)\s+(\w+)(?: fox| coat| fur)?$").captures(lw) {
            let want = m[1].to_string();
            let coats = rd["COATS"].as_array().unwrap();
            let mut keys: Vec<(String, String)> = vec![];
            let mut put = |k: String, v: String| {
                if let Some(e) = keys.iter_mut().find(|(a, _)| *a == k) { e.1 = v } else { keys.push((k, v)) }
            };
            for c in coats.iter().filter(|c| !s_of(&c[0]).is_empty()) {
                put(s_of(&c[0]), s_of(&c[0]));
            }
            for c in coats.iter().filter(|c| !s_of(&c[0]).is_empty()) {
                put(s_of(&c[1]).to_lowercase(), s_of(&c[0]));
            }
            for (a, b) in [("gold", "golden"), ("red", "orange"), ("jade", "mint"), ("pink", "sakura"), ("desert", "fennec"),
                           ("dark", "black"), ("grey", "silver"), ("gray", "silver"), ("theme", ""), ("default", "")] {
                put(a.into(), b.into());
            }
            if let Some((_, k)) = keys.iter().find(|(a, _)| *a == want) {
                let k = k.clone();
                self.cfg_set("coat", json!(k));
                self.look_changed();
                self.events.push(Ev::Beep { kind: "happy".into(), variety: false });
                let label = coats.iter().find(|c| s_of(&c[0]) == k).map(|c| s_of(&c[1])).filter(|l| !l.is_empty())
                    .unwrap_or("matching the theme".into());
                return Some(format!("New coat: {label}! *twirls* Ehehe~"));
            }
            if lw.starts_with("coat") || lw.starts_with("fur") {
                let names: Vec<String> = coats.iter().filter(|c| !s_of(&c[0]).is_empty()).map(|c| s_of(&c[1]).to_lowercase()).collect();
                return Some(format!("Coats: {}", names.join(", ")));
            }
        }
        let rn = raw.trim_matches([' ', '.', '!']);
        if let Some(m) = rx!(r"(?i)^(?:your name is|i'?ll call you|call yourself|rename (?:you|yourself)(?: to)?)\s+(.{1,20})$").captures(rn) {
            self.cfg_set("fox_name", json!(py_title(m[1].trim())));
            return Some(format!("Kon! From now on I'm {}! Ehehe~ I like it.", self.fox_name()));
        }
        // --- play with her
        if let Some(m) = rx!(r"^(?:give (?:you |her )?|feed (?:you |her )?|here'?s (?:an? |some )?|have (?:an? |some )?|eat (?:an? |some )?)?(?:a |an |some )?(treat|snack|food|inari(?: sushi)?|sushi|aburaage|abura ?age|fried tofu|tofu|dango|onigiri|rice ?ball|mochi)s?\b").captures(lw) {
            if rx!(r"^(give|feed|here|have|eat|treat|snack|inari|sushi|aburaage|abura|fried|tofu|dango|onigiri|rice|mochi)").is_match(lw)
                && lw.split_whitespace().count() <= 6 {
                let word = m[1].to_string();
                let tw = rd["TREAT_WORDS"].as_object().unwrap();
                let item = tw.get(&word).and_then(|v| v.as_str()).map(String::from)
                    .or_else(|| tw.iter().find(|(k, _)| word.contains(k.as_str())).map(|(_, v)| s_of(v)));
                return Some(self.interact(now, "treat", item));
            }
        }
        let nml = regex::escape(&nm.to_lowercase());
        if Regex::new(&format!("^feed (you|her|zenko|{nml})$")).unwrap().is_match(lw) {
            return Some(self.interact(now, "treat", None));
        }
        let plays: [(&str, &str); 13] = [
            (r"^(?:\*?(?:head ?pats?|pat ?pat|pats?|pets?|pet you|pet her|headpat you)\*?)(?: (?:you|her|zenko|%s))?$", "pet"),
            (r"^(?:\*?belly ?rubs?\*?|rub (?:your|her) belly)$", "belly"),
            (r"^(?:\*?boop\*?|boop (?:you|your nose|her|the snoot)|\*boops?\*)$", "boop"),
            (r"^(?:\*?tickles?\*?|tickle (?:you|her))$", "tickle"),
            (r"^(?:play with (?:a |the )?(?:ball of )?yarn|yarn|ball of yarn|play ball|play with (?:a |the )?ball|throw (?:the |a )?ball|fetch)$", "yarn"),
            (r"^(?:(?:let'?s |play |wanna )?hide (?:and|n|&) ?seek|hide ?and ?seek)$", "hide"),
            (r"^(?:high ?five|hi ?five|gimme five|paw five|\*high fives?\*|up top)$", "highfive"),
            (r"^(?:give (?:you |her )?a (?:gift|present)|gift|present|here'?s a (?:gift|present)|open (?:a |your )?(?:gift|present))$", "gift"),
            (r"^(?:dance|let'?s dance|do a dance|dance for me|dance party)$", "dance"),
            (r"^(?:brush|brush (?:you|her|your fur|your tails)|groom (?:you|her))$", "brush"),
            (r"^(?:fluff (?:your |her )?tails?|pet (?:your |her )?tails?|touch (?:your |her )?tails?)$", "tail"),
            (r"^(?:good (?:fox|girl|kitsune|zenko)|you'?re (?:cute|adorable|the best|amazing)|i love you|cutie|kawaii)$", "praise"),
            (r"^(?:yawn|are you (?:sleepy|tired))$", "yawn"),
        ];
        for (pat, kind) in plays {
            let p = if pat.contains("%s") { pat.replace("%s", &nml) } else { pat.to_string() };
            if Regex::new(&p).unwrap().is_match(lw) {
                return Some(self.interact(now, kind, None));
            }
        }
        if rx!(r"^(?:your |my )?(?:treasures?|treasure box|gifts|presents)(?: box)?$|what (?:gifts|presents) do you have").is_match(lw) {
            let tr: Vec<String> = extras()["treasures"].as_array().cloned().unwrap_or_default().iter().map(s_of).collect();
            return Some(if tr.is_empty() {
                "My treasure box is empty... ne, a gift? *puppy eyes*".into()
            } else {
                format!("My treasure box: {}. Ehehe~ I love them all!", tr[tr.len().saturating_sub(15)..].join(", "))
            });
        }
        if rx!(r"^(?:our |my |your )?(?:bond|friendship|affection)(?: level)?$|how much do you like me|do you like me").is_match(lw) {
            let (h, name, b) = self.bond_text();
            return Some(format!("Our bond: {h}  ({name}, {b}/100). Pets, treats and focus sessions make it grow~kon!"));
        }
        if rx!(r"^(?:bad (?:fox|girl|zenko)|you'?re (?:annoying|useless|dumb|stupid|bad)|shut up|go away|i hate you|stupid fox|dumb fox|be quiet)$").is_match(lw) {
            return Some(self.start_sulk(now, ""));
        }
        if rx!(r"^(?:sorry|i'?m sorry|im sorry|apologi[sz]e|forgive me|sorry zenko|my bad|gomen(?:nasai)?)!*$").is_match(lw) {
            if self.sulk_until != 0.0 {
                return Some(self.forgive(now, false, "sorry", None));
            }
            return Some("Ehehe~ nothing to be sorry for! *tail wag*".into());
        }
        None
    }
}
