//! Her slow life (zenko.py ZenkoLife life_slow_tick/_life_slow every 30 s, random_event, haiku, the visiting fox,
//! the seasons/weather particles around her and weather_poll's URLs): birthday and good-morning lines, bedtime
//! nudges, flashcards due, her needs (hungry / pouty / happy), and little surprises (a haiku, something she found,
//! a visiting friend, a fact she remembers about you).

use crate::music::ldata;
use crate::play::{Play, play_dur};
use crate::rewards::Later;
use crate::tt::parse_clock;
use crate::{Companion, Ev, Frame, fill, local_now, read_json, stats, today_iso, unix_now};
use serde_json::{Value, json};
use zenko_art::palette::{Palette, hex_rgb};
use zenko_art::raster::Image;

pub const W_: i32 = 110;
pub const H_: i32 = 78;

#[derive(Clone, Debug)]
pub struct Visitor {
    pub name: String,
    pub coat: String,
    pub t0: f64,
    pub z: i32,
    pub imgs: Vec<Image>,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub ph: f64,
    pub born: f64,
    pub vy: f64,
    pub col: String,
    pub on: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Wx {
    pub kind: Option<String>,
    pub items: Vec<Particle>,
    /// open-meteo weather_code for the home city (weather_poll)
    pub code: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct Slow {
    pub started: f64,
    pub next_event: f64,
    pub next_mood: f64,
    pub night_t: f64,
    pub visitor: Option<Visitor>,
    pub wx: Wx,
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

/// season(): winter / spring / summer / autumn for a month
pub fn season(m: u32) -> &'static str {
    match m {
        12 | 1 | 2 => "winter",
        3..=5 => "spring",
        6..=8 => "summer",
        _ => "autumn",
    }
}

fn days_iso(days: i64) -> String {
    let (y, m, d) = crate::civil_from_days_pub(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn today_days() -> i64 {
    let (y, m, d, _, _, _) = local_now();
    crate::rewards::days_from_civil(y as i64, m as i64, d as i64)
}

/// days_missed(): whole days since the last day with 10+ minutes (0 = today or yesterday)
pub fn days_missed() -> i64 {
    let st = stats();
    let d = today_days();
    for i in 0..60 {
        if st.get(days_iso(d - i)).and_then(|v| v.as_f64()).unwrap_or(0.0) >= 600.0 {
            return 0.max(i - 1);
        }
    }
    if st.as_object().is_some_and(|o| !o.is_empty()) { 2 } else { 0 }
}

pub fn today_focus_minutes() -> i64 {
    (stats().get(today_iso()).and_then(|v| v.as_f64()).unwrap_or(0.0) / 60.0).floor() as i64
}

/// _q: urllib.parse.quote(s, safe="")
pub fn url_quote(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"_.-~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// geocode(place)'s search URL, and its pick from the results
pub fn geocode_url(place: &str) -> String {
    let parts: Vec<&str> = place.split(',').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    let name = parts.first().copied().unwrap_or(place.trim());
    format!("https://geocoding-api.open-meteo.com/v1/search?name={}&count=10&language=en&format=json", url_quote(name))
}

pub fn geocode_pick(place: &str, j: &Value) -> Option<Value> {
    let res = j.get("results").and_then(|r| r.as_array()).cloned().unwrap_or_default();
    if res.is_empty() {
        return None;
    }
    let parts: Vec<&str> = place.split(',').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    if parts.len() > 1 {
        let want = parts[1..].join(" ").to_lowercase();
        for r in &res {
            let blob = ["country", "country_code", "admin1", "admin2"]
                .iter()
                .map(|k| match r.get(*k) {
                    Some(Value::String(s)) => s.clone(),
                    Some(v) => v.to_string(),
                    None => String::new(),
                })
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            let cc = r.get("country_code").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            if blob.contains(&want) || (want.chars().count() == 2 && cc == want) {
                return Some(r.clone());
            }
        }
    }
    Some(res[0].clone())
}

pub fn forecast_url(geo: &Value) -> String {
    format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=weather_code&timezone=auto",
            geo["latitude"], geo["longitude"])
}

impl Companion {
    pub(crate) fn slow_init(&mut self, now: f64) {
        self.slow.started = now;
        self.slow.next_event = now + (35.0 + 35.0 * self.rand()) * 60.0;
        self.slow.next_mood = now + (12.0 + 13.0 * self.rand()) * 60.0;
    }

    /// said_today(key): true once a key has been used today (and marks it)
    pub fn said_today(&mut self, key: &str) -> bool {
        let day = today_iso();
        self.life_do(|l| {
            if l["said"].get(key).and_then(|v| v.as_str()) == Some(day.as_str()) {
                return true;
            }
            l["said"][key] = json!(day);
            false
        })
    }

    /// hunger(): 3 points an hour since it was last fed
    pub fn hunger_now(&self) -> f64 {
        let l = read_json("zenko_life.json");
        let now = unix_now();
        let h0 = l.get("hunger").and_then(|v| v.as_f64()).unwrap_or(30.0);
        let t = l.get("hunger_t").and_then(|v| v.as_f64()).unwrap_or(now);
        (h0 + (now - t) / 3600.0 * 3.0).clamp(0.0, 100.0)
    }

    /// spirit(): 0-100, how bright her foxfire is
    pub fn spirit(&self) -> i64 {
        let bond = read_json("zenko_extras.json").get("bond").and_then(|v| v.as_f64()).map(|b| b.trunc()).unwrap_or(10.0);
        let v = 52.0 + 6.0 * crate::rewards::streak().min(5) as f64 + 30.0f64.min(today_focus_minutes() as f64 / 2.0)
            - 16.0 * days_missed().min(3) as f64 - 0.0f64.max(self.hunger_now() - 60.0) / 2.0 + bond / 10.0;
        v.clamp(0.0, 100.0).trunc() as i64
    }

    /// is_night(): between bedtime and wake time
    pub fn is_night(&self) -> bool {
        let bed = parse_clock(&self.cfg.s("bedtime", "23:00")).unwrap_or("23:00".into());
        let wake = parse_clock(&self.cfg.s("wake_time", "06:00")).unwrap_or("06:00".into());
        let (_, _, _, h, mi, _) = local_now();
        let t = format!("{h:02}:{mi:02}");
        if bed > wake { t >= bed || t < wake } else { bed <= t && t < wake }
    }

    fn play_now(&mut self, now: f64, kind: &str, dur: f64) {
        self.play = Some(Play { kind: kind.into(), t0: now, dur, arg: None, found: false });
    }

    /// _life_slow, every 30 s
    pub fn life_slow(&mut self, now: f64) {
        if self.slow.started == 0.0 {
            self.slow_init(now - 6.0);
        }
        self.plan_maybe();
        self.music_apply();
        if self.env.idle > 120.0 || self.lockin.is_some() || self.brk.is_some() {
            return;
        }
        let (_, mo, d, hour, mi, _) = local_now();
        let busy = self.session.is_some() || self.current_mood(now) == "alert";
        let life = read_json("zenko_life.json");
        let bd = life["facts"].get("birthday").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let md = format!("{mo:02}-{d:02}");
        let bd5: String = bd.chars().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect();
        if !bd.is_empty() && md == bd5 && !self.said_today("birthday") {
            let nm = self.cfg.s("user_name", "").trim().split(' ').next().unwrap_or("").to_string();
            let nm = if nm.is_empty() { String::new() } else { format!(", {nm}") };
            self.say(&format!("Happy birthday{nm}!! *throws sakura petals* Otanjoubi omedetou~! I made you a wish at the \
                               shrine~kon!"), 15.0);
            self.events.push(Ev::Beep { kind: "gift".into(), variety: false });
            self.play_now(now, "dance", play_dur("dance"));
            return;
        }
        if self.cfg.b("daynight", true) {
            if (5..11).contains(&hour) && now - self.slow.started > 20.0 && !self.said_today("morning") {
                let l = self.pick_s(&["Ohayo~! Good morning! Let's make today a good one, ne~".to_string(),
                                      "Ohayou gozaimasu~ *stretches tails* Ready when you are!".to_string()]);
                self.say(&l, 8.0);
                return;
            }
            if self.is_night() && unix_now() - self.slow.night_t > if busy { 45.0 } else { 30.0 } * 60.0 {
                self.slow.night_t = unix_now();
                let l = self.pick_s(&strs(&ldata()["NIGHT_LINES"])).replace("{time}", &format!("{hour:02}:{mi:02}"));
                self.say(&l, 12.0);
                if busy {
                    self.play = None;
                } else {
                    self.play_now(now, "yawn", play_dur("yawn"));
                }
                self.events.push(Ev::Beep { kind: "yawn".into(), variety: false });
                return;
            }
        }
        if busy {
            return;
        }
        let today = today_iso();
        if self.cfg.b("daily_checkin", true) && hour >= 6 && now - self.slow.started > 90.0
            && life.get("last_checkin").and_then(|v| v.as_str()) != Some(today.as_str()) && !self.said_today("checkin_ask")
        {
            self.later(now + 0.5, crate::rewards::Later::Command("open_checkin".into()));
            return;
        }
        let due = life["cards"].as_array().map(|a| {
            a.iter().filter(|c| c.get("due").and_then(|v| v.as_str()).unwrap_or("") <= today.as_str()).count()
        }).unwrap_or(0);
        if due > 0 && hour >= 9 && !self.said_today("cards_due") {
            let s = if due == 1 { "" } else { "s" };
            self.say(&format!("Kon! {due} flashcard{s} due for review today. Say 'review flashcards' when you have 5 \
                               minutes~"), 10.0);
            return;
        }
        if self.cfg.b("pet_needs", true) && now >= self.slow.next_mood && self.play.is_none() {
            self.slow.next_mood = now + (25.0 + 20.0 * self.rand()) * 60.0;
            let (sp, hu) = (self.spirit(), self.hunger_now());
            if hu >= 75.0 {
                self.set_mood(now, "sad", 5.0);
                let l = self.pick_s(&strs(&ldata()["HUNGRY_LINES"]));
                self.say(&l, 9.0);
            } else if sp < 30 {
                self.set_mood(now, "sad", 5.0);
                let l = self.pick_s(&strs(&ldata()["POUTY_LINES"]));
                self.say(&l, 9.0);
            } else if sp < 50 {
                self.play_now(now, "yawn", play_dur("yawn"));
            } else if sp >= 75 && self.rand() < 0.5 {
                self.set_mood(now, "happy", 4.0);
                let l = self.pick_s(&strs(&ldata()["HAPPY_LINES"]));
                self.say(&l, 7.0);
            }
            return;
        }
        if self.cfg.b("random_events", true) && now >= self.slow.next_event && self.play.is_none() {
            self.slow.next_event = now + (50.0 + 60.0 * self.rand()) * 60.0;
            self.random_event(now, None);
        }
    }

    /// haiku(): one she hasn't said lately (she remembers the last 40)
    pub fn haiku(&mut self) -> String {
        let (_, m, _, _, _, _) = local_now();
        let h = &ldata()["HAIKU"];
        let mut pool = strs(&h[season(m)]);
        pool.extend(strs(&h["any"]));
        let seen: Vec<String> = strs(&read_json("zenko_life.json")["haiku"]);
        let key = |s: &str| s.chars().take(24).collect::<String>();
        let fresh: Vec<String> = pool.iter().filter(|p| !seen.contains(&key(p))).cloned().collect();
        let fresh = if fresh.is_empty() { pool } else { fresh };
        let pick = self.pick_s(&fresh);
        let k = key(&pick);
        self.life_do(|l| {
            if !l["haiku"].is_array() {
                l["haiku"] = json!([]);
            }
            let a = l["haiku"].as_array_mut().unwrap();
            a.push(json!(k));
            let n = a.len();
            if n > 40 {
                a.drain(..n - 40);
            }
        });
        pick
    }

    /// random_event(kind): a haiku, something she found, a visiting friend, or a fact she remembers
    pub fn random_event(&mut self, now: f64, kind: Option<&str>) {
        let facts = read_json("zenko_life.json")["facts"].as_object().cloned().unwrap_or_default();
        let mut kinds = vec!["haiku", "found", "visitor"];
        if !facts.is_empty() {
            kinds.push("fact");
        }
        let kind = kind.map(String::from).unwrap_or_else(|| {
            let i = ((self.rand() * kinds.len() as f64) as usize).min(kinds.len() - 1);
            kinds[i].to_string()
        });
        match kind.as_str() {
            "haiku" => {
                let h = self.haiku();
                self.say(&format!("I wrote you a haiku~\n{h}"), 16.0);
                self.events.push(Ev::Beep { kind: "hello".into(), variety: false });
                self.play_now(now, "sing", 2.4);
            }
            "found" => {
                let thing = self.pick_s(&strs(&ldata()["FOUND_THINGS"]));
                let day = today_iso();
                let th = thing.clone();
                self.life_do(|l| {
                    let mut f = l.get("found").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    f.push(json!({"what": th, "date": day}));
                    let n = f.len();
                    l["found"] = json!(f[n.saturating_sub(30)..].to_vec());
                });
                self.events.push(Ev::Beep { kind: "gift".into(), variety: false });
                self.play_now(now, "gift", play_dur("gift"));
                let line = format!("Look look! I found {thing} on my walk round the shrine. It's for you~! (I put it on the \
                                    Daily page)");
                self.later(now + 0.9, Later::Say { text: line, secs: 10.0, choices: vec![], only_idle: false, beep: None });
            }
            "visitor" => self.visitor_start(now),
            _ => {
                let ks: Vec<(String, Value)> = facts.into_iter().collect();
                let i = ((self.rand() * ks.len() as f64) as usize).min(ks.len() - 1);
                let (k, v) = ks[i].clone();
                if k == "birthday" {
                    return self.random_event(now, Some("haiku"));
                }
                let v = match v {
                    Value::String(s) => s,
                    v => v.to_string(),
                };
                let t = self.pick_s(&["Ne~ you told me your {k} is {v}. I still remember, ehehe~".to_string(),
                                      "Kon! I was thinking about your {k}, {v}. Tell me more sometime~".to_string()]);
                self.say(&t.replace("{k}", &k).replace("{v}", &v), 9.0);
            }
        }
    }

    /// visitor_start: a friend fox spirit in her own coat comes to sit beside her for a little while
    pub fn visitor_start(&mut self, now: f64) {
        if self.slow.visitor.is_some() {
            return;
        }
        let vs = ldata()["VISITORS"].as_array().unwrap().clone();
        let i = ((self.rand() * vs.len() as f64) as usize).min(vs.len() - 1);
        let (name, coat, place) = (vs[i][0].as_str().unwrap().to_string(), vs[i][1].as_str().unwrap().to_string(),
                                   vs[i][2].as_str().unwrap().to_string());
        let mut pal = self.pal.clone();
        let src = Palette::fur(&coat);
        for k in ldata()["COAT_KEYS"].as_str().unwrap().bytes() {
            if let Some(c) = src.get(k) {
                pal.map[k as usize] = Some(c);
            }
        }
        let z = 1.max(self.z - 1);
        let lk = self.look_ref().clone();
        let imgs = (0..8)
            .map(|f| {
                let t = zenko_art::core::build_tails(3, f, 8, false);
                let b = zenko_art::sit::build_frame(if f % 4 != 0 { "happy" } else { "blink" }, f % 2, None, &[], 0, 0, &lk);
                zenko_art::raster::sprite_photo(&[&t, &b], &pal, z as usize, self.hd)
            })
            .collect();
        self.slow.visitor = Some(Visitor { name: name.clone(), coat, t0: now, z, imgs });
        self.award(now, "friend");
        self.events.push(Ev::Beep { kind: "hello".into(), variety: false });
        self.later(now + 1.6, Later::Say { text: format!("Kon kon! My friend {name} {place} came to visit~! Say hi!"), secs: 9.0,
                                           choices: vec![], only_idle: false, beep: None });
        self.later(now + 7.5, Later::Say {
            text: format!("{name} says: \"Ganbatte with your studies!\" ...and now she has to go home~"),
            secs: 7.0, choices: vec![], only_idle: false, beep: None });
    }

    /// visitor_frame: where she is and which picture, or None when she has gone home
    pub fn visitor_draw(&mut self, out: &mut Frame, now: f64) {
        let Some(v) = &self.slow.visitor else { return };
        let e = now - v.t0;
        let dur = 13.0;
        if e > dur {
            self.slow.visitor = None;
            return;
        }
        let w = (W_ * v.z) as f64;
        let home = (self.colw as f64 - w + (30.0 * self.u).trunc()).trunc();
        let x = if e < 1.4 {
            -w + (home + w) * (1.0 - (1.0 - e / 1.4).powi(2))
        } else if e > dur - 1.4 {
            home - (home + w) * ((e - (dur - 1.4)) / 1.4).powi(2)
        } else {
            home
        };
        let y = self.h - H_ * v.z - (2.0 * self.u) as i32;
        let im = &v.imgs[(now / 0.15) as usize % 8];
        crate::blit(out, im, x as i32, y);
    }

    /// wx_kind: the real weather if known (snow / rain), else the season (fireflies on summer nights)
    pub fn wx_kind(&self) -> &'static str {
        if let Some(code) = self.slow.wx.code {
            if self.cfg.b("seasons_weather", true) {
                if [71, 73, 75, 77, 85, 86].contains(&code) {
                    return "snow";
                }
                if [51, 53, 55, 56, 57, 61, 63, 65, 66, 67, 80, 81, 82, 95, 96, 99].contains(&code) {
                    return "rain";
                }
            }
        }
        let (_, m, _, hr, _, _) = local_now();
        let s = season(m);
        if s == "summer" && (hr >= 19 || hr < 5) {
            return "fireflies";
        }
        match s {
            "winter" => "snow",
            "spring" => "petals",
            "summer" => "fireflies",
            _ => "leaves",
        }
    }

    /// wx_frame: move the particles (called from the frame, before drawing)
    pub fn wx_step(&mut self, now: f64) {
        if !self.cfg.b("seasons", true) {
            self.slow.wx.items.clear();
            return;
        }
        let kind = self.wx_kind();
        if self.slow.wx.kind.as_deref() != Some(kind) {
            self.slow.wx.items.clear();
            self.slow.wx.kind = Some(kind.to_string());
        }
        let z = self.z as f64;
        let (x0, x1) = (self.colw as f64, (self.w - self.z * 2) as f64);
        let want = match kind {
            "rain" => 9,
            "fireflies" => 5,
            "snow" => 8,
            "petals" => 6,
            _ => 5,
        };
        if self.slow.wx.items.len() < want && self.rand() < 0.08 {
            let look = &ldata()["WX_LOOK"][kind];
            let col = look[if self.rand() < 0.5 { 0 } else { 1 }].as_str().unwrap().to_string();
            let x = x0 + (x1 - x0) * self.rand();
            let y = if kind == "fireflies" {
                self.fox_y as f64 + ((self.h - 4 * self.z) - self.fox_y) as f64 * self.rand()
            } else {
                -3.0 * z
            };
            let ph = 6.28 * self.rand();
            let base = match kind {
                "rain" => 9.0,
                "snow" => 0.9,
                "petals" => 1.1,
                "leaves" => 1.4,
                _ => 0.0,
            };
            let vy = base * z * (0.7 + 0.5 * self.rand());
            self.slow.wx.items.push(Particle { x, y, ph, born: now, vy, col, on: true });
        }
        let h = self.h as f64;
        let mut keep = vec![];
        for mut p in std::mem::take(&mut self.slow.wx.items) {
            let dead;
            if kind == "fireflies" {
                p.x += (now * 0.9 + p.ph).sin() * z * 0.35;
                p.y += (now * 0.7 + p.ph).cos() * z * 0.25;
                p.on = ((now - p.born) / 0.8 + p.ph) as i64 % 3 != 0;
                dead = now - p.born > 9.0;
            } else {
                p.y += p.vy * 0.6;
                if kind != "rain" {
                    p.x += (now * 1.3 + p.ph).sin() * z * 0.4 - if kind == "leaves" { 0.25 * z } else { 0.0 };
                }
                dead = p.y > h;
            }
            if !dead {
                keep.push(p);
            }
        }
        self.slow.wx.items = keep;
    }

    /// the particles as Tk draws them (two rectangles for petals and leaves), under everything else
    pub fn wx_draw(&self, out: &mut Frame) {
        let Some(kind) = self.slow.wx.kind.as_deref() else { return };
        if !self.cfg.b("seasons", true) {
            return;
        }
        let z = self.z;
        let second = ldata()["WX_LOOK"][kind][1].as_str().unwrap_or("#ffffff");
        for p in &self.slow.wx.items {
            if !p.on {
                continue;
            }
            let (x, y) = (p.x as i32, p.y as i32);
            let c = hex_rgb(&p.col);
            match kind {
                "rain" => fill(out, x, y, 1.max(z / 2), z * 3, c),
                "petals" => {
                    fill(out, x, y, z * 2, z, c);
                    fill(out, x + z, y + z, z, z, hex_rgb(second));
                }
                "leaves" => {
                    fill(out, x, y, z * 2, z * 2, c);
                    fill(out, x + z, y + z * 2, z * 2, z, hex_rgb("#8a3a1c"));
                }
                _ => fill(out, x, y, z, z, c),
            }
        }
    }

    /// weather_poll is wanted (every 45 min) when seasons, real weather and internet lookups are on
    pub fn weather_wanted(&self) -> Option<String> {
        let city = self.cfg.s("home_city", "");
        (self.cfg.b("seasons", true) && self.cfg.b("seasons_weather", true) && self.cfg.b("web_enabled", true)
            && !city.is_empty())
            .then_some(city)
    }
}
