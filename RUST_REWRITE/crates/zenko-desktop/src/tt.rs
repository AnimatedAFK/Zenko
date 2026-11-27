//! The timetable and pomodoro parts of a session (zenko.py load_tt/save_tt/tt_today/tt_done/tt_window, check_timetable
//! with the zenko18 read-aloud, focus_item, tt_mark + ZenkoLife/zenko23 life_tt_marked, pomo_start/session_done/
//! break_done). timetable.json is written like save_tt: json.dump(items, f, indent=2).

use crate::rewards::Later;
use crate::session::{clip, mins};
use crate::{Companion, Ev, data_dir, read_json, today_iso, unix_now};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
pub struct Pomo {
    pub id: u64,
    pub work: i64,
    pub brk: i64,
    pub rounds: i64,
    pub round: i64,
    pub waiting: bool,
}

/// parse_clock: '4pm', '16:30', '4.30 pm', '9' -> 'HH:MM'
pub fn parse_clock(text: &str) -> Option<String> {
    let t = text.to_lowercase();
    let t = t.trim();
    let b = t.as_bytes();
    let mut i = 0;
    while i < b.len() && i < 2 && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let h: i64 = t[..i].parse().ok()?;
    let mut mi = 0;
    let mut rest = &t[i..];
    if (rest.starts_with(':') || rest.starts_with('.')) && rest.len() >= 3 && rest.as_bytes()[1..3].iter().all(|c| c.is_ascii_digit()) {
        mi = rest[1..3].parse().ok()?;
        rest = &rest[3..];
    }
    let rest = rest.trim_start();
    let ap = match rest {
        "" => None,
        "am" | "a" => Some(false),
        "pm" | "p" => Some(true),
        _ => return None,
    };
    let mut h = h;
    if let Some(pm) = ap {
        if !(1..=12).contains(&h) {
            return None;
        }
        h = h % 12 + if pm { 12 } else { 0 };
    }
    if h > 23 || mi > 59 {
        return None;
    }
    Some(format!("{h:02}:{mi:02}"))
}

/// load_tt: the valid items, with start/mins/daily/date/done filled in
pub fn load_tt() -> Vec<Value> {
    let d = read_json("timetable.json");
    let mut out = vec![];
    for it in d.as_array().cloned().unwrap_or_default() {
        if !it.is_object() {
            continue;
        }
        let Some(start) = it.get("start").and_then(|v| v.as_str()).and_then(parse_clock) else { continue };
        let task_ok = match it.get("task") {
            Some(Value::String(s)) => !s.is_empty(),
            Some(Value::Null) | Some(Value::Bool(false)) | None => false,
            Some(Value::Number(n)) => n.as_f64() != Some(0.0),
            Some(Value::Array(a)) => !a.is_empty(),
            Some(Value::Object(o)) => !o.is_empty(),
            Some(_) => true,
        };
        if !task_ok {
            continue;
        }
        let mut it = it;
        it["start"] = json!(start);
        let m = match &it["mins"] {
            Value::Number(n) if n.as_f64() != Some(0.0) => n.as_f64().unwrap_or(30.0).trunc() as i64,
            Value::String(s) if !s.is_empty() => s.trim().parse().unwrap_or(30),
            _ => 30,
        };
        it["mins"] = json!(m);
        if it.get("daily").is_none() {
            it["daily"] = json!(false);
        }
        if it.get("date").is_none() {
            it["date"] = json!(today_iso());
        }
        if it.get("done").is_none() {
            it["done"] = json!([]);
        }
        out.push(it);
    }
    out
}

pub fn save_tt(items: &[Value]) {
    let _ = std::fs::create_dir_all(data_dir());
    let _ = std::fs::write(data_dir().join("timetable.json"), crate::py_json_ascii(&Value::Array(items.to_vec())));
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(_) => true,
    }
}

/// tt_today: today's items (daily ones not skipped today, or dated today), by start time; indexes into `items`
pub fn tt_today(items: &[Value], day: &str) -> Vec<usize> {
    let mut v: Vec<usize> = (0..items.len())
        .filter(|&i| {
            let it = &items[i];
            let skip = it.get("skip").and_then(|s| s.as_array()).is_some_and(|a| a.iter().any(|x| x == day));
            (truthy(it.get("daily")) && !skip) || it.get("date").and_then(|d| d.as_str()) == Some(day)
        })
        .collect();
    v.sort_by(|&a, &b| items[a]["start"].as_str().cmp(&items[b]["start"].as_str()));
    v
}

pub fn tt_done(it: &Value, day: &str) -> bool {
    it.get("done").and_then(|d| d.as_array()).is_some_and(|a| a.iter().any(|x| x == day))
}

/// tt_window(it)[0] - now, in seconds (local wall time)
fn secs_until_start(it: &Value) -> f64 {
    let (y, m, d, _, _, _) = crate::local_now();
    let st = it["start"].as_str().unwrap_or("00:00");
    let (h, mi) = st.split_once(':').unwrap_or(("0", "0"));
    let day0 = crate::rewards::days_from_civil(y as i64, m as i64, d as i64) as f64 * 86400.0;
    let start = day0 + h.parse::<f64>().unwrap_or(0.0) * 3600.0 + mi.parse::<f64>().unwrap_or(0.0) * 60.0;
    start - (unix_now() + crate::local_offset_pub() as f64)
}

/// md_plain-ish cleaning remind_aloud does before speaking
fn aloud_text(t: &str) -> String {
    let t = t.replace("Right-click me > Start timetable item!", "").replace("Right-click me > Start timetable item", "");
    let t = t.trim_end().to_string();
    let mut out = String::new();
    let mut rest = t.as_str();
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("Kon kon") {
            let r = r.strip_prefix('!').unwrap_or(r);
            rest = r.trim_start();
            continue;
        }
        let c = rest.chars().next().unwrap();
        if !matches!(c, '\u{201c}' | '\u{201d}' | '"' | '*' | '~' | '_' | '#') {
            out.push(c);
        }
        rest = &rest[c.len_utf8()..];
    }
    out.trim().to_string()
}

impl Companion {
    /// check_timetable(now): the 5-minute heads-up and the "time for" alert, each said once (and read aloud)
    pub fn check_timetable(&mut self, now: f64) {
        if now - self.tt_last < 10.0 {
            return;
        }
        self.tt_last = now;
        let day = today_iso();
        for i in tt_today(&self.tt, &day) {
            let it = self.tt[i].clone();
            if tt_done(&it, &day) {
                continue;
            }
            let diff = secs_until_start(&it);
            let start = it["start"].as_str().unwrap_or("").to_string();
            let task = it["task"].as_str().map(String::from).unwrap_or_else(|| it["task"].to_string());
            let key = format!("{day}|{start}|{task}");
            let m = it["mins"].as_i64().unwrap_or(30);
            if 0.0 < diff && diff <= 300.0 && self.announced.insert(format!("{key}|pre")) {
                let l = format!("Heads up: \u{201c}{}\u{201d} at {start} (in {} min).", clip(&task, 30), mins(diff));
                self.say(&l, 9.0);
                self.events.push(Ev::Beep { kind: "ding".into(), variety: false });
                self.remind_aloud(&l);
                return;
            }
            if (-600.0..=0.0).contains(&diff) && self.announced.insert(format!("{key}|now")) {
                self.set_mood(now, "happy", 3.0);
                self.events.push(Ev::Beep { kind: "happy".into(), variety: false });
                let l = format!("Timetable: time for \u{201c}{}\u{201d} ({m} min). Right-click me > Start timetable item!",
                                clip(&task, 30));
                self.say(&l, 12.0);
                self.remind_aloud(&l);
                return;
            }
        }
    }

    /// remind_aloud (zenko19): in her own voice by default. Her VOICEVOX line (translated to Japanese) isn't ported
    /// yet, so she does what zenko.py does when VOICEVOX is off: her little recorded "kon kon~". "english" = the
    /// Windows voice (zenko18), "off" = silent.
    pub fn remind_aloud(&mut self, text: &str) {
        if !self.cfg.b("remind_aloud", true) || !self.cfg.b("sound", true) {
            return;
        }
        match self.cfg.s("remind_voice", "zenko").as_str() {
            "english" => {
                let t = aloud_text(text);
                if !t.is_empty() {
                    self.events.push(Ev::Speak(t));
                }
            }
            "off" => {}
            _ => {
                if !aloud_text(text).is_empty() {
                    self.events.push(Ev::Beep { kind: "hello".into(), variety: false });
                }
            }
        }
    }

    /// focus_item: the item on now, or the next one today (index into self.tt)
    pub fn focus_item(&self) -> Option<usize> {
        let day = today_iso();
        tt_today(&self.tt, &day).into_iter().find(|&i| {
            let it = &self.tt[i];
            !tt_done(it, &day) && secs_until_start(it) + it["mins"].as_f64().unwrap_or(30.0) * 60.0 > 0.0
        })
    }

    /// tt_mark(it, done): tick a timetable item for today, save, and let the life side count it
    pub fn tt_mark(&mut self, now: f64, i: usize, done: bool) {
        self.tt_mark_day(now, i, done, &today_iso());
    }

    /// tt_mark(it, done, day)
    pub fn tt_mark_day(&mut self, now: f64, i: usize, done: bool, day: &str) {
        if i >= self.tt.len() {
            return;
        }
        let day = day.to_string();
        {
            let it = &mut self.tt[i];
            if !it.get("done").is_some_and(|d| d.is_array()) {
                it["done"] = json!([]);
            }
            let lst = it["done"].as_array_mut().unwrap();
            let has = lst.iter().any(|x| x == day.as_str());
            if done && !has {
                lst.push(json!(day));
            }
            if !done && has {
                let p = lst.iter().position(|x| x == day.as_str()).unwrap();
                lst.remove(p);
            }
        }
        save_tt(&self.tt);
        self.life_tt_marked(now, i, done, &day);
    }

    fn life_tt_marked(&mut self, now: f64, i: usize, done: bool, day: &str) {
        let it = self.tt[i].clone();
        if !truthy(it.get("plan")) {
            return;
        }
        let pid = match &it["plan"] {
            Value::String(s) => s.clone(),
            v => v.to_string(),
        };
        let start = match it.get("start") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => "None".into(),
        };
        let key = format!("{day}|{start}|{pid}");
        let m = it.get("mins").and_then(|v| v.as_f64()).map(|v| v.trunc() as i64).unwrap_or(50);
        let has_studied = self.life_do(|l| {
            let had = l["counted"].get(&key).is_some();
            if done && !had {
                l["counted"][key.as_str()] = json!(m);
                let s = l["studied"].get(&pid).and_then(|v| v.as_i64()).unwrap_or(0);
                l["studied"][pid.as_str()] = json!(s + m);
            } else if !done && had {
                let c = l["counted"].as_object_mut().unwrap().remove(&key).and_then(|v| v.as_i64()).unwrap_or(0);
                let s = l["studied"].get(&pid).and_then(|v| v.as_i64()).unwrap_or(0);
                l["studied"][pid.as_str()] = json!((s - c).max(0));
            }
            l["studied"].get(&pid).is_some()
        });
        if !done {
            return;
        }
        let left = self.tt.iter().any(|x| x.get("plan").is_some_and(|p| *p == it["plan"]) && !truthy(x.get("done")));
        if !left && has_studied {
            self.award(now, "prepared");
        }
        // zenko23: the last planned bit of an ema's study -> offer to take the ema down
        if let Some(eid) = pid.strip_prefix("ema:") {
            let ema = crate::read_json("zenko_life.json")["ema"].as_array().and_then(|a| a.iter().find(|e| e["id"] == eid).cloned());
            let left = self.tt.iter().enumerate().any(|(j, x)| {
                j != i && x.get("plan").and_then(|p| p.as_str()) == Some(pid.as_str()) && !truthy(x.get("done"))
                    && !tt_done(x, &today_iso()) && x.get("date").and_then(|d| d.as_str()).unwrap_or("") >= today_iso().as_str()
            });
            if let Some(e) = ema {
                if !truthy(e.get("done")) && !left {
                    let text = e.get("text").and_then(|t| t.as_str()).unwrap_or("None").to_string();
                    self.later(now + 0.5, Later::Say {
                        text: format!("That was the last planned bit of \u{201c}{}\u{201d}. Take its ema down?", clip(&text, 30)),
                        secs: 15.0,
                        choices: vec![("Yes, done!".into(), format!("ema_done:{eid}")), ("Not yet".into(), String::new())],
                        only_idle: false,
                        beep: None,
                    });
                }
            }
        }
    }

    /// ema_set_done(E, True) from the "Yes, done!" chip
    pub fn ema_set_done(&mut self, eid: &str) {
        self.life_do(|l| {
            if let Some(a) = l["ema"].as_array_mut() {
                if let Some(e) = a.iter_mut().find(|e| e["id"] == eid) {
                    e["done"] = json!(true);
                    e["done_at"] = json!(unix_now());
                }
            }
        });
        self.z23_after_change();
    }

    /// pomo_start(work, brk, rounds): from chat ("pomodoro 25 5 4")
    pub fn pomo_start(&mut self, now: f64, work: i64, brk: i64, rounds: i64) -> String {
        if self.session.is_some() {
            return "A session is already running. Say 'stop' first, then try again~".into();
        }
        self.pomo_id += 1;
        self.pomo = Some(Pomo { id: self.pomo_id, work, brk, rounds, round: 1, waiting: false });
        self.start_session(now, &format!("Pomodoro 1/{rounds}"), work as f64);
        format!("Pomodoro on! {rounds} rounds of {work} min focus + {brk} min break. Round 1, go~kon!")
    }

    /// pomo_session_done(completed): true when the pomodoro decides what happens next
    pub fn pomo_session_done(&mut self, now: f64, completed: bool) -> bool {
        let Some(p) = self.pomo.clone() else { return false };
        if !completed {
            self.pomo = None;
            return false;
        }
        if p.round >= p.rounds {
            self.pomo = None;
            self.later(now + 2.5, Later::Say {
                text: format!("All {} pomodoros done! Sugoi~! Take a long break, you earned it~kon!", p.rounds),
                secs: 12.0,
                choices: vec![],
                only_idle: false,
                beep: None,
            });
            return true;
        }
        if let Some(q) = &mut self.pomo {
            q.waiting = true;
        }
        self.later(now + 4.0, Later::Break(p.brk));
        true
    }

    /// pomo_break_done: after the break, the next round starts 2.5 s later
    pub fn pomo_break_done(&mut self, now: f64) {
        let Some(p) = &mut self.pomo else { return };
        if !p.waiting {
            return;
        }
        p.waiting = false;
        p.round += 1;
        let id = p.id;
        self.later(now + 2.5, Later::PomoGo(id));
    }

    pub(crate) fn pomo_go(&mut self, now: f64, id: u64) {
        let Some(p) = self.pomo.clone() else { return };
        if p.id == id && self.session.is_none() {
            self.start_session(now, &format!("Pomodoro {}/{}", p.round, p.rounds), p.work as f64);
        }
    }
}
