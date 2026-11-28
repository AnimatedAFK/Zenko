//! answer_local's routes after extra_route, in zenko.py's order: app_route (hub pages, then the open_app hook),
//! deadline_route (what's due, "add exam ... 12 Dec 9am", "summarise that file"), battery, screen_route ("what's on my
//! screen", the host answers), spotify_route (music commands; the Spotify player itself is the host's) and
//! notes_route (your reMarkable notes).

use crate::extra::{notes_find, notes_index};
use crate::lifechat::{a_d_b, find_dates, find_time, guess_kind, ymd};
use crate::plan::{day_n, iso, naive};
use crate::rewards::Later;
use crate::study::when_text;
use crate::{Companion, unix_now};
use regex::Regex;
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

const MON3: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// screen_route(text): the question to ask about the screen, or None
pub fn screen_route(text: &str) -> Option<String> {
    let low = text.to_lowercase();
    let low = low.trim();
    let m = rx!(r"^(?:look at|check|read|see|scan) (?:my |the |this )?screen[,:\s]*(.*)$|^(?:what'?s|whats|what is) on (?:my |the )?screen\??\s*(.*)$|^screen[,:]\s*(.+)$|^(?:explain|what is|what's) (?:this|that)(?: on (?:my|the) screen)?\??$|^where (?:do i|should i) click.*$|^point (?:at|to) (.+)$").captures(low)?;
    let q = (1..m.len()).filter_map(|i| m.get(i)).map(|g| g.as_str()).find(|g| !g.is_empty()).unwrap_or("");
    let q = q.trim();
    Some(if q.is_empty() { text.trim().to_string() } else { q.to_string() })
}

/// _spotify_route(t, low)
fn spotify_route_once(t: &str, low: &str) -> Option<(String, Option<String>)> {
    if rx!(r"^(?:what'?s|whats|what is|what|wat)(?: is)?(?: \w+)? (?:playing|this song|the song|on spotify)|^(?:which|what) song|^(?:now|currently) playing|^current song|^what music").is_match(low) {
        return Some(("now".into(), None));
    }
    if rx!(r"^(?:stop|pause) (?:playing|the music|music|the song|song|spotify)$").is_match(low) {
        return Some(("pause".into(), None));
    }
    if let Some(m) = rx!(r"^(?:play|paly|plya|ply|put on|spotify play|start music|play music)(?:\s+(.+))?$").captures(low) {
        if !rx!(r"^play\s*(time|date)\b").is_match(low) {
            let arg = match m.get(1) {
                Some(g) => {
                    let n = g.as_str().chars().count();
                    let tc: Vec<char> = t.chars().collect();
                    tc[tc.len().saturating_sub(n)..].iter().collect::<String>()
                }
                None => String::new(),
            };
            return Some(("play".into(), Some(arg.trim().to_string())));
        }
    }
    if rx!(r"^(?:pause|stop) (?:the )?(?:music|song|spotify)$|^(?:music|spotify) (?:pause|stop|off)$").is_match(low) {
        return Some(("pause".into(), None));
    }
    if rx!(r"^(?:resume|unpause) (?:the )?(?:music|song|spotify)$|^(?:music|spotify) (?:on|resume|play)$").is_match(low) {
        return Some(("play".into(), Some(String::new())));
    }
    if rx!(r"^(?:skip|next song|next track|skip (?:this )?(?:song|track)|skip it)$").is_match(low) {
        return Some(("next".into(), None));
    }
    if rx!(r"^(?:previous|last|prev|back) (?:song|track)$|^go back a song$").is_match(low) {
        return Some(("previous".into(), None));
    }
    if let Some(m) = rx!(r"^(?:set )?(?:the )?volume(?: to)? (\d{1,3})%?$|^(?:spotify|music) volume (\d{1,3})%?$").captures(low) {
        return Some(("volume".into(), Some(m.get(1).or(m.get(2)).unwrap().as_str().parse::<i64>().unwrap().to_string())));
    }
    if rx!(r"^(?:volume|music) (?:up|louder)$|^louder$").is_match(low) {
        return Some(("volume_rel".into(), Some("15".into())));
    }
    if rx!(r"^(?:volume|music) (?:down|quieter|softer)$|^quieter$").is_match(low) {
        return Some(("volume_rel".into(), Some("-15".into())));
    }
    if let Some(m) = rx!(r"^shuffle (on|off)$").captures(low) {
        return Some(("shuffle".into(), Some((&m[1] == "on").to_string())));
    }
    if rx!(r"^(?:like|save|heart) (?:this|the) (?:song|track)$|^i like this song$").is_match(low) {
        return Some(("like".into(), None));
    }
    None
}

/// spotify_route(text): (action, arg) for music commands; spelling mistakes are tolerated
pub fn spotify_route(text: &str) -> Option<(String, Option<String>)> {
    let t = text.trim();
    let low = t.to_lowercase();
    let low = low.trim_matches([' ', '.', '!', '?']);
    if let Some(r) = spotify_route_once(t, low) {
        return Some(r);
    }
    let fixed = crate::chat::fix_typos(t);
    if fixed.to_lowercase() == t.to_lowercase() {
        return None;
    }
    let fl = fixed.to_lowercase();
    let r = spotify_route_once(fixed.trim(), fl.trim_matches([' ', '.', '!', '?']))?;
    if r.0 == "play" && r.1.as_deref().is_some_and(|a| !a.is_empty()) {
        let parts: Vec<&str> = t.splitn(2, char::is_whitespace).collect();
        return Some(("play".into(), Some(parts.get(1).map(|p| p.trim_start().to_string()).unwrap_or_default())));
    }
    Some(r)
}

impl Companion {
    /// spotify.connected(): a saved refresh token
    pub fn spotify_connected(&self) -> bool {
        crate::read_json("spotify_token.json").get("refresh_token").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty())
    }

    /// music(action, arg): not connected -> she says so; otherwise the host's Spotify player does it (stage 5c)
    fn music(&mut self, now: f64, action: &str, arg: Option<&str>) -> Option<String> {
        if !self.spotify_connected() {
            let msg = "Spotify isn't connected yet. Settings > Spotify (one-time setup, about 3 minutes).".to_string();
            self.fox_reply(now, &msg);
            return Some(msg);
        }
        self.events.push(crate::Ev::Command(format!("spotify:{action}:{}", arg.unwrap_or(""))));
        None
    }

    /// fox_reply(msg): a chat line and its first line in a bubble
    pub fn fox_reply(&mut self, now: f64, msg: &str) {
        self.chat_line(now, "Fox", msg);
        let first = crate::mdtext::md_plain(msg).split('\n').next().unwrap_or("").to_string();
        self.say(&first, 12.0);
    }

    /// ask_screen(question, typed=True)
    fn ask_screen(&mut self, q: &str) -> String {
        if !cfg!(windows) {
            return "Looking at the screen works on Windows.".into();
        }
        self.say("Let me look...", 6.0);
        let now = unix_now();
        self.set_mood(now, "focus", 6.0);
        self.events.push(crate::Ev::Command(format!("ask_screen:{q}")));
        "Looking at your screen...".into()
    }

    /// open_app(name): the host opens it (Windows); elsewhere she says it only works on Windows
    fn open_app(&mut self, name: &str) -> String {
        let n = rx!(r"\b(the|app|application|program|please|for me|up)\b").replace_all(&name.to_lowercase(), " ").into_owned();
        let n = rx!(r"\s+").replace_all(&n, " ").trim_matches([' ', '.', '!', '?']).to_string();
        if n.is_empty() {
            return "Open what? e.g. 'open spotify'.".into();
        }
        match self.open_app_hook {
            Some(f) => f(&n, name.trim()),
            None => "Opening apps works on Windows.".into(),
        }
    }

    pub(crate) fn app_route(&mut self, now: f64, text: &str) -> Option<String> {
        let m = rx!(r"(?i)^(?:please )?(?:open|launch|run|start up|fire up)\s+(?:my |the )?(.+?)\s*[.!?]*$").captures(text.trim())?;
        let target = m[1].to_lowercase().trim().to_string();
        let page = match target.as_str() {
            "timetable" | "schedule" => "timetable",
            "reminders" | "ema" => "reminders",
            "chat" => "chat",
            "notes" => "notes",
            "settings" => "settings",
            "deadlines" | "calendar" | "exams" | "upcoming" | "study plan" | "planner" | "clock" => "deadlines",
            "home" | "dashboard" => "home",
            "daily" | "badges" | "journal" | "flashcards" => "daily",
            "profile" | "you" => "you",
            _ => "",
        };
        if !page.is_empty() && !(target == "calendar" && rx!(r"(?i)outlook|windows|google").is_match(text)) {
            self.later(now, Later::Command(format!("open_hub:{page}")));
            return Some("Here you go.".into());
        }
        if ["zenko", "window", "app"].contains(&target.as_str()) {
            self.later(now, Later::Command("open_hub:None".into()));
            return Some("Here I am.".into());
        }
        Some(self.open_app(&target))
    }

    pub(crate) fn deadline_route(&mut self, now: f64, text: &str) -> Option<String> {
        let low = text.to_lowercase();
        let low = low.trim_matches([' ', '?', '.', '!']);
        let today = day_n(naive(unix_now()));
        if rx!(r"^(?:my |show (?:my )?|list (?:my )?|any |upcoming |what'?s (?:coming )?(?:up|due)|whats due|what is due|when is my next)(?:deadlines?|exams?|tests?|due dates?|calendar|assignments?)?\b.*$").is_match(low)
            && rx!(r"deadline|exam|test|due|calendar|assignment|coming up").is_match(low) {
            let up = self.upcoming_deadlines(6);
            if up.is_empty() {
                return Some("Nothing coming up. Drop your exam schedule on me (or the Upcoming page) and I'll add the dates.".into());
            }
            self.later(now + 0.2, Later::Command("open_hub:deadlines".into()));
            let rows: Vec<String> = up.iter().map(|d| {
                let ds = d["date"].as_str().unwrap_or("");
                let day = crate::plan::parse_iso(ds).unwrap_or(today);
                format!("\u{2022} {} - {} ({})", d["title"].as_str().unwrap_or(""), a_d_b(day), when_text(ds, today))
            }).collect();
            return Some(format!("Coming up:\n{}", rows.join("\n")));
        }
        if let Some(m) = rx!(r"(?i)^add (?:a )?(?:deadline|exam|test|quiz|assignment)[:\s]+(.+)$").captures(text.trim()) {
            let rest = m[1].to_string();
            let ds = find_dates(&rest, today, self.cfg.b("date_day_first", true));
            let Some(&(d, s, e)) = ds.first() else {
                return Some("When is it? e.g. 'add exam renal block on 12 Dec 9am'.".into());
            };
            let (t, span) = find_time(&rest);
            let mut cuts = vec![(s, e)];
            if let Some(sp) = span {
                cuts.push(sp);
            }
            cuts.sort();
            cuts.reverse();
            let mut title = rest.clone();
            for (a, b) in cuts {
                title = format!("{} {}", &title[..a.min(title.len())], &title[b.min(title.len())..]);
            }
            let title = rx!(r"\s+").replace_all(&title, " ").trim().to_string();
            let title = rx!(r"(?i)\b(on|by|at|due)\s*$").replace(&title, "").trim_matches([' ', '-', ',']).to_string();
            let word = rx!(r"(?i)^add (?:a )?(\w+)").captures(text.trim()).unwrap()[1].to_lowercase();
            let mut title = title;
            if ["exam", "test", "quiz", "assignment"].contains(&word.as_str()) && guess_kind(&title) == "Deadline" {
                title = format!("{title} {word}").trim().to_string();
            }
            let title = {
                let mut c = title.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            };
            self.add_deadline(now, &title, &iso(d), &t, None, "typed");
            let (y, mo, dd) = ymd(d);
            return Some(format!("Added: {title} on {} {dd:02} {} {y} ({}).", ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][crate::lifechat::weekday(d)],
                                MON3[mo as usize - 1], when_text(&iso(d), today)));
        }
        if rx!(r"^(?:summari[sz]e|explain|what'?s in|read) (?:that|the|this|my) (?:file|document|pdf)").is_match(low) {
            let Some((name, _body)) = self.last_file.clone() else { return Some("Drop a file on me first.".into()) };
            self.events.push(crate::Ev::Command(format!("summarise_file:{name}")));
            return Some(format!("Reading \u{201c}{name}\u{201d}..."));
        }
        None
    }

    /// battery_text()
    pub fn battery_text(&self) -> String {
        match self.battery {
            None => "No battery (desktop PC).".into(),
            Some((p, c)) => format!("Battery: {p}%{}.", if c { ", charging" } else { "" }),
        }
    }

    /// notes_llm(task, name, question): without the brain running she shows the notes instead
    fn notes_llm(&mut self, now: f64, task: &str, name: &str, _question: &str) -> String {
        let mut hit = if name.is_empty() { None } else { notes_find(name) };
        if hit.is_none() {
            let idx = notes_index();
            if idx.is_empty() {
                return "No notes read yet. Open the Notes tab, pick a notebook and press 'Read'.".into();
            }
            if name.is_empty() && idx.len() == 1 {
                hit = idx.into_iter().next();
            } else {
                let mut names: Vec<String> = idx.into_iter().map(|(n, _)| n).collect();
                names.sort();
                return format!("Which notebook? I have: {}", names.into_iter().take(12).collect::<Vec<_>>().join(", "));
            }
        }
        let (nm, path) = hit.unwrap();
        let body = std::fs::read_to_string(&path).unwrap_or_default();
        if task == "show" {
            let n = body.chars().count();
            return format!("{nm}:\n{}{}", body.chars().take(1500).collect::<String>(), if n > 1500 { "..." } else { "" });
        }
        // the smart brain comes with stage 5b: until then, as zenko.py does when Ollama isn't running
        let msg = format!("Ollama isn't running, so I can't think about your notes. Here they are instead:\n{}",
                          body.chars().take(900).collect::<String>());
        self.later(now + 0.1, Later::FoxReply(msg));
        format!("Reading \u{201c}{nm}\u{201d}...")
    }

    pub(crate) fn notes_route(&mut self, now: f64, text: &str) -> Option<String> {
        let low = text.to_lowercase();
        let low = low.trim_matches([' ', '?', '.', '!']);
        if rx!(r"^(?:my |list (?:my )?|show (?:my )?)?(?:remarkable )?notes(?: list)?$|^(?:which|what) notes do (?:i|you) have").is_match(low) {
            let mut idx: Vec<String> = notes_index().into_iter().map(|(n, _)| n).collect();
            idx.sort();
            return Some(if idx.is_empty() { "No notes read yet. Open the Notes tab to connect your reMarkable.".into() } else {
                format!("Notes I've read: {}", idx.join(", "))
            });
        }
        if let Some(m) = rx!(r"^(?:summari[sz]e|summary of) (?:my )?(?:notes? )?(?:on |about |for )?(.*)$").captures(low) {
            if low.contains("note") || notes_find(&m[1]).is_some() {
                let n = rx!(r"\bnotes?\b").replace_all(&m[1], "").trim().to_string();
                return Some(self.notes_llm(now, "summary", &n, ""));
            }
        }
        if let Some(m) = rx!(r"^quiz me (?:on |about )?(?:my )?(?:notes? )?(?:on |about )?(.*)$").captures(low) {
            let n = rx!(r"\bnotes?\b").replace_all(&m[1], "").trim().to_string();
            return Some(self.notes_llm(now, "quiz", &n, ""));
        }
        if let Some(m) = rx!(r"^(?:read|show|open) (?:me )?(?:my )?notes? (?:on |about |for |called )?(.+)$").captures(low) {
            return Some(self.notes_llm(now, "show", &m[1], ""));
        }
        if let Some(m) = rx!(r"^ask (?:my )?notes?(?: on| about)? ?([^:]*):\s*(.+)$|^ask (?:my )?notes (.+)$").captures(low) {
            if let Some(q) = m.get(3) {
                return Some(self.notes_llm(now, "ask", "", q.as_str()));
            }
            return Some(self.notes_llm(now, "ask", &m[1], &m[2]));
        }
        None
    }

    /// answer_local's tail after extra_route
    pub(crate) fn misc_routes(&mut self, now: f64, text: &str, low: &str) -> Option<String> {
        if let Some(r) = self.app_route(now, text) {
            return Some(r);
        }
        if let Some(r) = self.deadline_route(now, text) {
            return Some(r);
        }
        if rx!(r"^(?:how(?:'s| is| much)? (?:my |the )?)?battery\b|^battery (?:level|left|status)").is_match(low) {
            return Some(self.battery_text());
        }
        if let Some(q) = screen_route(text) {
            return Some(self.ask_screen(&q));
        }
        if let Some((a, arg)) = spotify_route(text) {
            return Some(self.music(now, &a, arg.as_deref()).unwrap_or_else(|| "On it...".into()));
        }
        self.notes_route(now, text)
    }
}
